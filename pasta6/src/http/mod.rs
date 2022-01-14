#[cfg(test)]
use crate::http::connection::Connection;
use lunatic::net::{TcpListener, TcpStream};
use lunatic::process::Process;
use lunatic::Mailbox;
use std::io::{Read, Write};
use std::str;

mod connection;
mod header;

pub(super) use crate::http::connection::Response;
pub(super) use crate::http::header::Headers;

/// Maximum number of headers allowed in an HTTP request.
const MAX_REQUEST_HEADERS: usize = 100;
/// Initial buffer size allocated for an HTTP request.
const INIT_REQUEST_BUFFER_SIZE: usize = 1024;

pub(crate) struct Request<'buf> {
    method: Method,
    path: &'buf str,
    body: &'buf [u8],
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Method {
    Get,
}

#[cfg_attr(test, derive(Debug))]
enum ParseResult<T> {
    Ok(T),
    Partial,
    Error(httparse::Error),
}

#[derive(Debug)]
struct ConnectionError {
    _kind: ConnectionErrorKind,
}

#[derive(Debug)]
enum ConnectionErrorKind {
    UnexpectedEof,
}

pub(crate) trait Handler {
    fn handle<'request, 'response>(
        &self,
        request: &'request Request<'request>,
    ) -> Response<'response>;
}

#[inline]
pub(crate) fn server<H>((parent, handler, port): (Process<()>, H, u16), _mailbox: Mailbox<()>)
where
    H: Handler,
{
    tracing::info!("server binding to 127.0.0.1:{}", port);
    let listener = match TcpListener::bind(format!("127.0.0.1:{}", port)) {
        Ok(listener) => listener,
        Err(e) => {
            tracing::error!("bind error: {}", e);
            panic!();
        }
    };
    parent.send(());
    tracing::info!("server accepting connections");
    loop {
        match listener.accept() {
            Ok((tcp_stream, peer)) => {
                tracing::trace!("server accepted connection: {}", peer);
                match handle_connection(tcp_stream, &handler) {
                    Ok(()) => {
                        tracing::trace!("closed connection: {}", peer);
                    }
                    Err(e) => {
                        tracing::error!("connection error: {:?}", e);
                    }
                }
                continue;
            }
            Err(e) => {
                tracing::error!("accept error: {}", e);
                panic!();
            }
        }
    }
}

#[inline]
fn handle_connection<H>(mut tcp_stream: TcpStream, handler: &H) -> Result<(), ConnectionError>
where
    H: Handler,
{
    tracing::trace!("server handling connection");
    'outer: loop {
        // Allocate a buffer to store request data.
        // TODO: re-use this buffer between requests.
        // TODO: allow non-contiguous buffers to allow re-allocation.
        // TODO: add a limit to the size of a single HTTP header.
        // TODO: add a limit to the total size of the request head.
        // TODO: shrink this buffer after the request is processed?
        let mut buf = vec![0; INIT_REQUEST_BUFFER_SIZE];
        // Read as much data as possible from the TCP stream into the buffer.
        tracing::trace!("server reading stream");
        let mut bytes_read = 0;
        let mut reached_eof = false;
        let request = loop {
            // If there is no remaining space in the buffer to read into, then
            // we need to grow the buffer.
            // TODO: grow the buffer in powers of two to perform O(log n)
            //   allocations rather than O(n) allocations.
            if bytes_read >= buf.len() {
                debug_assert!(bytes_read == buf.len());
                const EMPTY_BUFFER: [u8; INIT_REQUEST_BUFFER_SIZE] = [0; INIT_REQUEST_BUFFER_SIZE];
                buf.extend_from_slice(&EMPTY_BUFFER);
            }

            bytes_read += match tcp_stream.read(&mut buf[bytes_read..]) {
                Ok(0) => {
                    tracing::trace!("reached EOF");
                    // If we've reached EOF and there are no unparsed bytes,
                    // then the client has closed the connection.
                    if bytes_read == 0 {
                        return Ok(());
                    }
                    reached_eof = true;
                    bytes_read
                }
                Ok(bytes_read) => {
                    tracing::trace!("server bytes read: {}", bytes_read);
                    bytes_read
                }
                Err(e) => {
                    tracing::error!("read error: {}", e);
                    panic!();
                }
            };
            // Parse the data into an HTTP request.
            tracing::trace!(
                "server parsing request: {}",
                String::from_utf8_lossy(&buf[..bytes_read])
            );
            match Request::parse(&buf[..bytes_read]) {
                ParseResult::Ok(request) => break request,
                ParseResult::Partial => {
                    if !reached_eof {
                        continue;
                    } else {
                        // TODO: is just returning in the event of an unparsable HTTP
                        //   request head and an unexpected EOF the correct thing to
                        //   do?
                        return Err(ConnectionError {
                            _kind: ConnectionErrorKind::UnexpectedEof,
                        });
                    }
                }
                ParseResult::Error(e) => match e {
                    // RFC 6585 section 5:
                    // > The 431 status code indicates that the server is
                    // > unwilling to process the request because its header
                    // > fields are too large. The request MAY be resubmitted
                    // > after reducing the size of the request header fields.
                    // >
                    // > It can be used both when the set of request header
                    // > fields in total is too large, and when a single header
                    // > field is at fault. In the latter case, the response
                    // > representation SHOULD specify which header field was too
                    // > large.
                    httparse::Error::TooManyHeaders => {
                        let response = Response::from_static(431, "");
                        tracing::trace!("server writing response");
                        match connection::write_response(&mut tcp_stream, response) {
                            Ok(()) => {}
                            Err(e) => {
                                tracing::error!("write error: {}", e);
                                panic!()
                            }
                        }
                        tracing::trace!("server flushing response");
                        match tcp_stream.flush() {
                            Ok(()) => {}
                            Err(e) => {
                                tracing::error!("flush error: {}", e);
                                panic!();
                            }
                        }
                        break 'outer;
                    }
                    _ => {
                        tracing::error!("parse error: {}", e);
                        panic!()
                    }
                },
            };
        };
        tracing::trace!("server total bytes read: {}", bytes_read);
        // Invoke the provided handler function to process the request.
        let response = handler.handle(&request);
        // TODO: what's the proper behaviour if the handler defined these headers?
        if response.headers().get("content-length").is_some() {
            tracing::error!("unexpected header: content-length");
            panic!()
        }
        if response.headers().get("date").is_some() {
            tracing::error!("unexpected header: date");
            panic!()
        }
        // Send the response back to the client.
        // TODO: investigate perf of multiple `write_all` vs single `write!`.
        tracing::trace!("server writing response");
        match connection::write_response(&mut tcp_stream, response) {
            Ok(()) => {}
            Err(e) => {
                tracing::error!("write error: {}", e);
                return Ok(());
            }
        }
        tracing::trace!("server flushing response");
        match tcp_stream.flush() {
            Ok(()) => {}
            Err(e) => {
                tracing::error!("flush error: {}", e);
                panic!();
            }
        }
    }
    tracing::trace!("server closing connection");
    Ok(())
}

impl Request<'_> {
    pub(super) fn path(&self) -> &str {
        self.path
    }

    pub(super) fn body(&self) -> &[u8] {
        self.body
    }
}

// FIXME: replace this with `TryFrom`.
impl From<&str> for Method {
    fn from(string: &str) -> Self {
        match string {
            "GET" => Method::Get,
            _ => unimplemented!(),
        }
    }
}

impl Request<'_> {
    #[inline]
    pub(crate) fn method(&self) -> Method {
        self.method
    }

    #[inline]
    fn parse(buf: &[u8]) -> ParseResult<Request> {
        let mut headers = [httparse::EMPTY_HEADER; MAX_REQUEST_HEADERS];
        let mut request = httparse::Request::new(&mut headers);
        let idx = match request.parse(buf) {
            Ok(httparse::Status::Complete(idx)) => idx,
            Ok(httparse::Status::Partial) => return ParseResult::Partial,
            Err(source) => return ParseResult::Error(source),
        };
        let body = match buf.get(idx..) {
            Some(body) => body,
            None => return ParseResult::Partial,
        };
        debug_assert!(request.path.is_some(), "missing path");
        debug_assert!(request.method.is_some(), "missing method");
        debug_assert!(request.version.is_some(), "missing version");
        let request = Request {
            path: request.path.unwrap(),
            method: request.method.unwrap().into(),
            body,
        };
        ParseResult::Ok(request)
    }
}
