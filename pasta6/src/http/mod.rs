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
pub(crate) trait Handler {
    fn handle<'request, 'response>(
        &self,
        request: &'request Request<'request>,
    ) -> Response<'response>;
}

#[inline]
pub(crate) fn server<H>((parent, handler): (Process<()>, H), _mailbox: Mailbox<()>)
where
    H: Handler,
{
    println!("server binding to 127.0.0.1:3000");
    let listener = match TcpListener::bind("127.0.0.1:3000") {
        Ok(listener) => listener,
        Err(e) => {
            eprintln!("bind error: {}", e);
            panic!();
        }
    };
    parent.send(());
    println!("server accepting connections");
    loop {
        match listener.accept() {
            Ok((tcp_stream, peer)) => {
                println!("server accepted connection: {}", peer);
                handle_connection(tcp_stream, &handler);
            }
            Err(e) => {
                eprintln!("accept error: {}", e);
                panic!();
            }
        }
    }
}

#[inline]
fn handle_connection<H>(mut tcp_stream: TcpStream, handler: &H)
where
    H: Handler,
{
    println!("server handling connection");
    // Allocate a buffer to store request data.
    let mut buf = [0; 1024];
    // Read as much data as possible from the TCP stream into the buffer.
    println!("server reading stream");
    let bytes_read = match tcp_stream.read(&mut buf) {
        Ok(bytes) => bytes,
        Err(e) => {
            eprintln!("read error: {}", e);
            panic!();
        }
    };
    println!("server bytes read: {}", bytes_read);
    // Parse the data into an HTTP request.
    println!("server parsing request");
    let request = match Request::parse(&buf[..bytes_read]) {
        ParseResult::Ok(request) => request,
        ParseResult::Partial => unimplemented!(),
        ParseResult::Error(e) => {
            eprintln!("parse error: {}", e);
            panic!();
        }
    };
    // Invoke the provided handler function to process the request.
    let response = handler.handle(&request);
    // TODO: what's the proper behaviour if the handler defined these headers?
    if response.headers().get("content-length").is_some() {
        eprintln!("unexpected header: content-length");
        panic!()
    }
    if response.headers().get("date").is_some() {
        eprintln!("unexpected header: date");
        panic!()
    }
    // Send the response back to the client.
    // TODO: investigate perf of multiple `write_all` vs single `write!`.
    println!("server writing response");
    match connection::write_response(&mut tcp_stream, response) {
        Ok(()) => {}
        Err(e) => {
            eprintln!("write error: {}", e);
            panic!();
        }
    }
    println!("server flushing response");
    match tcp_stream.flush() {
        Ok(()) => {}
        Err(e) => {
            eprintln!("flush error: {}", e);
            panic!();
        }
    }
    println!("server closing connection");
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
        let mut headers = [httparse::EMPTY_HEADER; 0];
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
