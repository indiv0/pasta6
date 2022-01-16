//! HTTP client/server library.
//!
//! # RFCs
//!
//! - [RFC 2616 (Hypertext Transfer Protocol -- HTTP/1.1)][rfc2616]
//! - [RFC 6585 (Additional HTTP Status Codes)][rfc6585]
//! - [RFC 7230 (Hypertext Transfer Protocol (HTTP/1.1): Message Syntax and Routing)][rfc7230]
//! - [RFC 7231 (Hypertext Transfer Protocol (HTTP/1.1): Semantics and Content)][rfc7231]
//!
//! [rfc2616]: https://datatracker.ietf.org/doc/html/rfc2616 "Hypertext Transfer Protocol -- HTTP/1.1"
//! [rfc6585]: https://datatracker.ietf.org/doc/html/rfc6585 "Additional HTTP Status Codes"
//! [rfc7230]: https://datatracker.ietf.org/doc/html/rfc7231 "Hypertext Transfer Protocol (HTTP/1.1): Message Syntax and Routing"
//! [rfc7231]: https://datatracker.ietf.org/doc/html/rfc7231 "Hypertext Transfer Protocol (HTTP/1.1): Semantics and Content"
use crate::http::connection::Connection;
use crate::net::TcpStream;
use lunatic::net::TcpListener;
use lunatic::process::Process;
use lunatic::Mailbox;
use std::io::Write;
use std::mem;
use std::net::SocketAddr;

mod client;
mod connection;
mod header;

pub(super) use crate::http::client::Client;
#[cfg(all(test, not(target_arch = "wasm32")))]
pub(super) use crate::http::connection::from_parts;
pub(super) use crate::http::connection::{Method, Request, Response};
pub(super) use crate::http::header::Headers;

#[cfg_attr(test, derive(Debug))]
enum ParseResult<T> {
    Ok(T),
    Partial,
    Error(httparse::Error),
}

#[derive(Debug)]
struct ConnectionError {
    kind: ConnectionErrorKind,
}

#[derive(Debug)]
enum ConnectionErrorKind {
    UnexpectedEof,
    RequestError,
    Closed,
}

pub(crate) trait Handler {
    fn handle<'request, 'response>(request: &'request Request<'request>) -> Response<'response>;
}

#[inline]
pub(crate) fn handler_as_int(handler: for<'r, 's> fn(&'r Request<'s>) -> Response<'r>) -> usize {
    handler as *const () as usize
}

#[inline]
pub(crate) fn handler_from_int(
    handler_int: usize,
) -> for<'r, 's> fn(&'r Request<'s>) -> Response<'r> {
    unsafe {
        let pointer = handler_int as *const ();
        mem::transmute::<*const (), for<'r> fn(&'r Request) -> Response<'r>>(pointer)
    }
}

#[cfg(target_arch = "wasm32")]
#[inline]
pub(crate) fn server(
    (parent, handler, (ip, port)): (
        Process<()>,
        for<'r, 's> fn(&'r Request<'s>) -> Response<'r>,
        ([u8; 4], u16),
    ),
    _mailbox: Mailbox<()>,
) {
    let addr = SocketAddr::from((ip, port));
    tracing::info!("server binding to {:?}", addr);
    let listener = match TcpListener::bind(addr) {
        Ok(listener) => listener,
        Err(e) => {
            tracing::error!("bind error: {}", e);
            panic!();
        }
    };
    parent.send(());
    tracing::info!("server accepting connections");
    let handler_int = handler_as_int(handler);
    loop {
        match listener.accept() {
            Ok((tcp_stream, peer)) => {
                tracing::debug!("server accepted connection: {}", peer);
                match crate::spawn_with!(
                    (tcp_stream, peer, handler_int),
                    |(tcp_stream, peer, handler_int): (lunatic::net::TcpStream, _, _),
                     _mailbox: Mailbox::<()>| {
                        let handler = handler_from_int(handler_int);
                        match handle_connection(tcp_stream.into(), &handler) {
                            Ok(()) => {
                                tracing::debug!("closed connection: {}", peer);
                            }
                            Err(e) if e.is_closed() => {
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
    // FIXME: make this agnostic over both stream types.
    mut tcp_stream: TcpStream,
    handler: &for<'r, 's> fn(&'r Request<'s>) -> Response<'r>,
) -> Result<(), ConnectionError> {
    tracing::trace!("server handling connection");
    // FIXME: keep the connection around.
    // FIXME: wrap the error, don't unwrap
    let mut connection = Connection::new(tcp_stream.try_clone().unwrap());
    loop {
        // Invoke the provided handler function to process the request.
        let request = connection.next_request()?;
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
}

impl ConnectionError {
    #[inline]
    fn is_closed(&self) -> bool {
        matches!(self.kind, ConnectionErrorKind::Closed)
    }

    #[inline]
    fn unexpected_eof() -> Self {
        Self {
            kind: ConnectionErrorKind::UnexpectedEof,
        }
    }

    #[inline]
    fn request_error() -> Self {
        Self {
            kind: ConnectionErrorKind::RequestError,
        }
    }

    #[inline]
    fn closed() -> Self {
        Self {
            kind: ConnectionErrorKind::Closed,
        }
    }
}
