#[cfg(test)]
use crate::http::connection::Connection;
use lunatic::net::{TcpListener, TcpStream};
use lunatic::process::Process;
use lunatic::Mailbox;
use std::io::{Read, Write};
use std::{mem, str};

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
    fn handle<'request, 'response>(request: &'request Request<'request>) -> Response<'response>;
}

#[inline]
pub(crate) fn server(
    (parent, handler, port): (
        Process<()>,
        for<'r, 's> fn(&'r Request<'s>) -> Response<'r>,
        u16,
    ),
    _mailbox: Mailbox<()>,
) {
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
    let handler_int = handler as *const () as usize;
    loop {
        match listener.accept() {
            Ok((tcp_stream, peer)) => {
                tracing::debug!("server accepted connection: {}", peer);
                match crate::spawn_with!(
                    (tcp_stream, peer, handler_int),
                    |(tcp_stream, peer, handler_int), _mailbox: Mailbox::<()>| {
                        let handler = unsafe {
                            let pointer = handler_int as *const ();
                            mem::transmute::<*const (), for<'r> fn(&'r Request) -> Response<'r>>(
                                pointer,
                            )
                        };
                        match handle_connection(tcp_stream, &handler) {
                            Ok(()) => {
                                tracing::debug!("closed connection: {}", peer);
                            }
                            Err(e) => {
                                tracing::error!("connection error: {:?}", e);
                            }
                        }
                    }
                ) {
                    Ok(_proc) => {}
                    Err(e) => {
                        tracing::error!("process error: {}", e);
                        panic!();
                    }
                };
            }
            Err(e) => {
                tracing::error!("accept error: {}", e);
                panic!();
            }
        }
    }
}

#[inline]
fn handle_connection(
    mut tcp_stream: TcpStream,
    handler: &for<'r, 's> fn(&'r Request<'s>) -> Response<'r>,
) -> Result<(), ConnectionError> {
    tracing::debug!("server handling connection");
    'outer: loop {
        // Allocate a buffer to store request data.
        // TODO: re-use this buffer between requests.
        // TODO: allow non-contiguous buffers to allow re-allocation.
        // TODO: add a limit to the size of a single HTTP header.
        // TODO: add a limit to the total size of the request head.
        // TODO: shrink this buffer after the request is processed?
        let mut buf = vec![0; INIT_REQUEST_BUFFER_SIZE];
        // Read as much data as possible from the TCP stream into the buffer.
        tracing::debug!("server reading stream");
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
                    tracing::debug!("reached EOF");
                    // If we've reached EOF and there are no unparsed bytes,
                    // then the client has closed the connection.
                    if bytes_read == 0 {
                        return Ok(());
                    }
                    reached_eof = true;
                    bytes_read
                }
                Ok(bytes_read) => {
                    tracing::debug!("server bytes read: {}", bytes_read);
                    bytes_read
                }
                Err(e) => {
                    tracing::debug!("read error: {}", e);
                    // If the client dropped the socket without properly
                    // shutting down the TCP connection, then we stop
                    // processing.
                    // TODO: should we finish processing the currect request
                    //   before exiting?
                    debug_assert!(bytes_read == 0);
                    return Err(ConnectionError::unexpected_eof());
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
                        debug_assert!(bytes_read > 0);
                        // TODO: is just returning in the event of an unparsable HTTP
                        //   request head and an unexpected EOF the correct thing to
                        //   do?
                        return Err(ConnectionError::unexpected_eof());
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
                        tracing::debug!("server writing response");
                        match connection::write_response(&mut tcp_stream, response) {
                            Ok(()) => {}
                            Err(e) => {
                                tracing::error!("write error: {}", e);
                                panic!()
                            }
                        }
                        tracing::debug!("server flushing response");
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
        tracing::debug!("server total bytes read: {}", bytes_read);
        // Invoke the provided handler function to process the request.
        let response = handler(&request);
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
        tracing::debug!("server writing response");
        match connection::write_response(&mut tcp_stream, response) {
            Ok(()) => {}
            Err(e) => {
                tracing::error!("write error: {}", e);
                return Ok(());
            }
        }
        tracing::debug!("server flushing response");
        match tcp_stream.flush() {
            Ok(()) => {}
            Err(e) => {
                tracing::error!("flush error: {}", e);
                panic!();
            }
        }
    }
    tracing::debug!("server closing connection");
    Ok(())
}

impl Request<'_> {
    #[inline]
    pub(super) fn path(&self) -> &str {
        self.path
    }

    #[inline]
    pub(super) fn body(&self) -> &[u8] {
        self.body
    }
}

// FIXME: replace this with `TryFrom`.
impl From<&str> for Method {
    #[inline]
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

impl ConnectionError {
    #[inline]
    fn unexpected_eof() -> Self {
        Self {
            _kind: ConnectionErrorKind::UnexpectedEof,
        }
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
#[inline]
pub(crate) fn from_parts<'a>(path: &'a str, method: &'a str, body: &'a [u8]) -> Request<'a> {
    Request {
        path,
        method: method.into(),
        body,
    }
}
