use bytes::Bytes;
use lunatic::net::{TcpListener, TcpStream};
use lunatic::process::Process;
use lunatic::Mailbox;
use std::io::{Read, Write};
use std::str;

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

pub(crate) trait Handler {
    fn handle<'request, 'response>(
        &self,
        request: &'request Request<'request>,
    ) -> Response<'response>;
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
    if response.headers.get("content-length").is_some() {
        eprintln!("unexpected header: content-length");
        panic!()
    }
    if response.headers.get("date").is_some() {
        eprintln!("unexpected header: date");
        panic!()
    }
    // Send the response back to the client.
    // TODO: investigate perf of multiple `write_all` vs single `write!`.
    println!("server writing response");
    match tcp_stream.write_all(b"HTTP/1.1 ") {
        Ok(()) => {}
        Err(e) => {
            eprintln!("write error: {}", e);
            panic!();
        }
    }
    match write!(tcp_stream, "{}", response.code) {
        Ok(()) => {}
        Err(e) => {
            eprintln!("write error: {}", e);
            panic!();
        }
    }
    match tcp_stream.write_all(b" ") {
        Ok(()) => {}
        Err(e) => {
            eprintln!("write error: {}", e);
            panic!();
        }
    }
    match tcp_stream.write_all(response.reason.as_bytes()) {
        Ok(()) => {}
        Err(e) => {
            eprintln!("write error: {}", e);
            panic!();
        }
    }
    match response.body.len() {
        BodyLength::Known(length) => {
            match tcp_stream.write_all(b"\r\ncontent-length: ") {
                Ok(()) => {}
                Err(e) => {
                    eprintln!("write error: {}", e);
                    panic!();
                }
            }
            match write!(tcp_stream, "{}", length) {
                Ok(()) => {}
                Err(e) => {
                    eprintln!("write error: {}", e);
                    panic!();
                }
            }
        }
        BodyLength::Unknown => {}
    }
    // FIXME: don't hardcode the timestamp here.
    match tcp_stream.write_all(b"\r\ndate: Fri, 14 Jan 2022 02:28:00 GMT") {
        Ok(()) => {}
        Err(e) => {
            eprintln!("write error: {}", e);
            panic!();
        }
    }
    match tcp_stream.write_all(b"\r\n\r\n") {
        Ok(()) => {}
        Err(e) => {
            eprintln!("write error: {}", e);
            panic!();
        }
    }
    match response.body.kind {
        BodyKind::Connection {
            connection: _,
            length: _,
        } => unimplemented!(),
        BodyKind::Bytes(ref bytes) => match tcp_stream.write_all(bytes) {
            Ok(()) => {}
            Err(e) => {
                eprintln!("write error: {}", e);
                panic!();
            }
        },
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

pub(crate) struct Request<'buf> {
    method: &'buf str,
    path: &'buf str,
    body: &'buf [u8],
}

#[cfg_attr(test, derive(Debug))]
pub(crate) struct Response<'body> {
    code: u16,
    reason: &'static str,
    // TODO: find a way to avoid copying the headers?
    headers: Headers,
    body: Body<'body>,
}

#[cfg_attr(test, derive(Debug))]
pub(crate) struct Headers {
    values: Vec<u8>,
    parts: Vec<HeaderPart>,
}

#[cfg_attr(test, derive(Debug))]
struct HeaderPart {
    name: &'static str,
    start: usize,
    end: usize,
}

#[cfg_attr(test, derive(Debug))]
pub(crate) struct Body<'a> {
    kind: BodyKind<'a>,
}

#[cfg_attr(test, derive(Debug))]
enum BodyKind<'a> {
    Connection {
        connection: &'a mut Connection,
        length: BodyLength,
    },
    Bytes(Bytes),
}

#[derive(Clone)]
#[cfg_attr(test, derive(Debug))]
enum BodyLength {
    Known(usize),
    Unknown,
}

impl Headers {
    #[inline]
    pub(crate) fn empty() -> Self {
        Self {
            values: vec![],
            parts: vec![],
        }
    }

    #[inline]
    fn len(&self) -> usize {
        self.parts.len()
    }

    #[inline]
    fn get(&self, name: &'static str) -> Option<&[u8]> {
        self.parts
            .iter()
            .find(|p| p.name == name)
            .map(|p| &self.values[p.start..p.end])
    }
}

impl From<&mut [httparse::Header<'_>]> for Headers {
    #[inline]
    fn from(httparse_headers: &mut [httparse::Header<'_>]) -> Self {
        let values_len = httparse_headers.iter().map(|h| h.value.len()).sum();
        let mut headers = Headers {
            values: Vec::with_capacity(values_len),
            parts: Vec::with_capacity(httparse_headers.len()),
        };
        let mut start = 0;
        httparse_headers
            .iter()
            .map(|h| (h.name, h.value))
            .for_each(|(n, v)| {
                let name = match n {
                    "content-length" => "content-length",
                    "date" => "date",
                    n => {
                        eprintln!("unsupported header: {}", n);
                        unimplemented!()
                    }
                };
                headers.parts.push(HeaderPart {
                    name,
                    start,
                    end: start + v.len(),
                });
                start += v.len();
                headers.values.extend_from_slice(v);
            });
        headers
    }
}

impl Body<'_> {
    #[inline]
    fn len(&self) -> BodyLength {
        match &self.kind {
            BodyKind::Connection {
                connection: _,
                length,
            } => length.clone(),
            BodyKind::Bytes(bytes) => BodyLength::Known(bytes.len()),
        }
    }
}

impl From<&'static [u8]> for Body<'_> {
    #[inline]
    fn from(bytes: &'static [u8]) -> Self {
        Body {
            kind: BodyKind::Bytes(Bytes::from_static(bytes)),
        }
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
impl Into<hyper::Body> for Body<'static> {
    #[inline]
    fn into(self) -> hyper::Body {
        match self.kind {
            BodyKind::Connection {
                connection: _,
                length: _,
            } => unimplemented!(),
            BodyKind::Bytes(bytes) => bytes.into(),
        }
    }
}

#[cfg_attr(test, derive(Debug))]
enum ParseResult<T> {
    Ok(T),
    Partial,
    Error(httparse::Error),
}

#[cfg_attr(test, derive(Debug))]
struct ParseError {
    source: httparse::Error,
}

#[cfg_attr(test, derive(Debug))]
struct ResponseError {
    kind: ResponseErrorKind,
    source: Option<httparse::Error>,
}

#[cfg_attr(test, derive(Debug))]
struct ParseIntError;

#[cfg_attr(test, derive(Debug))]
enum ResponseErrorKind {
    ParseHead,
    ParseInt,
    InvalidMessageFraming,
}

impl Request<'_> {
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
            method: request.method.unwrap(),
            body,
        };
        ParseResult::Ok(request)
    }
}

impl<'body> Response<'body> {
    // FIXME: don't leak `httparse::Header` type.
    #[inline]
    pub(crate) fn new(code: u16, headers: Headers, body: Body<'body>) -> Response<'body> {
        let reason = match code {
            200 => "OK",
            _ => unimplemented!(),
        };
        Self {
            code,
            reason,
            headers,
            body,
        }
    }
}

// Maximum number of headers allowed in an HTTP response.
// TODO: write tests to verify behaviour when too many request or response
//   headers are provided.
const MAX_RESPONSE_HEADERS: usize = 16;

#[cfg_attr(test, derive(Debug))]
struct Connection {
    buf: [u8; 1024],
    #[cfg(target_arch = "wasm32")]
    tcp_stream: lunatic::net::TcpStream,
    #[cfg(not(target_arch = "wasm32"))]
    tcp_stream: std::net::TcpStream,
}

impl Connection {
    /// Parses a byte slice into an HTTP response.
    ///
    /// # Limitations
    ///
    /// `Response` can parse up to 16 headers. Attempting to parse a response
    /// with more will result in an error.
    #[inline]
    fn next_response(&mut self) -> Result<Response, ResponseError>
where {
        println!("connection reading response");
        // TODO: read until `\r\n\r\n` as that indicates the end of the
        //   response head. Currently we blindly loop and read as much as
        //   possible, then try to parse the response even if we can't
        //   possibly have enough data yet, leading to potentially wasted
        //   CPU cycles on parsing.
        let mut bytes_read = 0;
        loop {
            bytes_read += match self.tcp_stream.read(&mut self.buf[bytes_read..]) {
                Ok(bytes_read) => bytes_read,
                Err(e) => {
                    eprintln!("read error: {}", e);
                    panic!();
                }
            };
            println!("connection bytes read: {}", bytes_read);
            let response_bytes = &mut self.buf[..bytes_read];
            let lossy_response_str = String::from_utf8_lossy(response_bytes);
            println!("connection parsing response: {}", lossy_response_str);

            // FIXME: start reading the body after the head of the request.
            let mut httparse_headers = [httparse::EMPTY_HEADER; MAX_RESPONSE_HEADERS];
            let mut httparse_response = httparse::Response::new(&mut httparse_headers);
            match httparse_response.parse(response_bytes) {
                Ok(httparse::Status::Complete(_head_length)) => {
                    debug_assert!(httparse_response.code.is_some(), "missing code");
                    debug_assert!(httparse_response.reason.is_some(), "missing reason");
                    debug_assert!(httparse_response.version.is_some(), "missing version");
                    let response_str = match str::from_utf8(response_bytes) {
                        Ok(response_str) => response_str,
                        Err(e) => {
                            eprintln!("utf-8 error: {}", e);
                            panic!();
                        }
                    };
                    println!("client received response: {}", response_str);

                    // RFC 7230 section 3.3.3 point 4:
                    // > If a message is received without Transfer-Encoding and with
                    // > either multiple Content-Length header fields having
                    // > differing field-values or a single Content-Length header
                    // > field having an invalid value, then the message framing is
                    // > invalid and the recipient MUST treat it as an unrecoverable
                    // > error. ... If this is a response message received by a user
                    // > agent, the user agent MUST close the connection to the
                    // > server and discard the received response.
                    // TODO: verify that this works case-insensitively.
                    let mut content_length = None;
                    let mut has_transfer_encoding = false;
                    for (name, value) in httparse_response.headers.iter().map(|h| (h.name, h.value))
                    {
                        if name == "transfer-encoding" {
                            has_transfer_encoding = true;
                            continue;
                        }

                        if name == "content-length" {
                            if let Some(content_length) = content_length {
                                if content_length == value {
                                    continue;
                                } else {
                                    return Err(ResponseError {
                                        kind: ResponseErrorKind::InvalidMessageFraming,
                                        source: None,
                                    });
                                }
                            }
                            content_length = Some(value);
                        }
                    }
                    if has_transfer_encoding {
                        unimplemented!()
                    }
                    let body_length =
                        match content_length {
                            // RFC 7230 section 3.3.3 point 5:
                            // > If a valid Content-Length header field is present
                            // > without Transfer-Encoding, its decimal value defines the
                            // > expected message body length in octets.
                            Some(content_length) => BodyLength::Known(
                                usize_from_bytes(content_length).map_err(|_e| ResponseError {
                                    kind: ResponseErrorKind::ParseInt,
                                    source: None,
                                })?,
                            ),
                            // RFC 7230 section 3.3.3 point 7:
                            // > Otherwise, this is a response message without a declared
                            // > message body length, so the message body length is
                            // > determined by the number of octets received prior to the
                            // > server closing the connection.
                            None => BodyLength::Unknown,
                        };

                    let code = httparse_response.code.unwrap();
                    let headers = httparse_response.headers.into();
                    let body = Body {
                        kind: BodyKind::Connection {
                            connection: self,
                            length: body_length,
                        },
                    };
                    let response = Response::new(code, headers, body);

                    return Ok(response);
                }
                Ok(httparse::Status::Partial) => continue,
                Err(source) => {
                    return Err(ResponseError {
                        kind: ResponseErrorKind::ParseHead,
                        source: Some(source),
                    })
                }
            };
        }
    }
}

// TODO: use a faster integer parsing method here.
#[inline]
fn usize_from_bytes(bytes: &[u8]) -> Result<usize, ParseIntError> {
    if bytes.is_empty() {
        return Err(ParseIntError);
    }

    let mut value = 0usize;
    for byte in bytes.iter().copied() {
        if (b'0'..=b'9').contains(&byte) {
            match value.checked_mul(10) {
                Some(v) => value = v,
                None => return Err(ParseIntError),
            }
            match value.checked_add((byte - b'0') as usize) {
                Some(v) => value = v,
                None => return Err(ParseIntError),
            }
        } else {
            return Err(ParseIntError);
        }
    }
    Ok(value)
}

#[cfg(test)]
mod test {
    use crate::http::{Headers, Response};
    use regex::Regex;

    #[test]
    fn test() {
        use crate::http::Connection;
        use std::str;

        crate::request!(
            {
                struct App;

                impl crate::http::Handler for App {
                    fn handle<'request, 'response>(
                        &self,
                        request: &'request crate::http::Request<'request>,
                    ) -> crate::http::Response<'response> {
                        println!("server handling request");
                        assert_eq!(request.method, "GET");
                        assert_eq!(request.path, "/");
                        assert_eq!(request.body, b"");
                        Response::new(200, Headers::empty(), b"hello, world!"[..].into())
                    }
                }

                App
            },
            {
                use std::io::Write;

                println!("client connecting to 127.0.0.1:3000");
                let mut tcp_stream = match TcpStream::connect("127.0.0.1:3000") {
                    Ok(tcp_stream) => tcp_stream,
                    Err(e) => {
                        eprintln!("connect error: {}", e);
                        panic!();
                    }
                };
                const REQUEST: &str = "GET / HTTP/1.1\r\n\r\n";
                println!("client writing request: {}", REQUEST);
                match tcp_stream.write_all(REQUEST.as_bytes()) {
                    Ok(()) => {}
                    Err(e) => {
                        eprintln!("request error: {}", e);
                        panic!();
                    }
                }
                println!("client reading response");
                let mut connection = Connection {
                    tcp_stream,
                    buf: [0; 1024],
                };
                println!("client reading response");
                let response = match connection.next_response() {
                    Ok(response) => response,
                    Err(e) => {
                        eprintln!("response error: {:?}", e);
                        panic!();
                    }
                };
                println!("client received response: {:?}", response);
                assert_eq!(response.code, 200);
                assert_eq!(response.reason, "OK");
                assert_eq!(response.headers.len(), 2);
                assert_eq!(response.headers.get("content-length"), Some(&b"13"[..]));
                const DATE_REGEX: &str = r"^Fri, \d{2} Jan \d{4} \d{2}:\d{2}:\d{2} GMT$";
                let re = match Regex::new(DATE_REGEX) {
                    Ok(re) => re,
                    Err(e) => {
                        eprintln!("regex error: {}", e);
                        panic!();
                    }
                };
                match response.headers.get("date") {
                    Some(date) => match str::from_utf8(date) {
                        Ok(date) => assert!(re.is_match(date)),
                        Err(e) => {
                            eprintln!("utf-8 error: {}", e);
                            panic!();
                        }
                    },
                    None => {
                        eprintln!("missing header: daate");
                        panic!();
                    }
                }
                let response_bytes = &connection.buf[..89];
                let response_str = match str::from_utf8(response_bytes) {
                    Ok(response_str) => response_str,
                    Err(e) => {
                        eprintln!("utf-8 error: {}", e);
                        panic!();
                    }
                };
                println!("client received response: {}", response_str);
                let re = match Regex::new(
                    r"^HTTP/1.1 200 OK\r\ncontent-length: 13\r\ndate: Fri, \d{2} Jan \d{4} \d{2}:\d{2}:\d{2} GMT\r\n\r\nhello, world!$",
                ) {
                    Ok(re) => re,
                    Err(e) => {
                        eprintln!("regex error: {}", e);
                        panic!();
                    }
                };
                assert!(re.is_match(response_str));

                //assert_eq!(response.body, b"hello, world!");

                //let response = ureq::get("http://127.0.0.1:3000").call().unwrap();
                //assert_eq!(response.get_url(), "http://127.0.0.1:3000/");
                //assert_eq!(response.http_version(), "HTTP/1.1");
                //assert_eq!(response.status(), 200);
                //assert_eq!(response.status_text(), "OK");
                //assert_eq!(response.headers_names(), vec!["content-length", "date"]);
                //assert_eq!(response.header("content-length").unwrap(), "13");
                //let re = Regex::new(r"^Fri, \d{2} Jan \d{4} \d{2}:\d{2}:\d{2} GMT").unwrap();
                //assert!(re.is_match(response.header("date").unwrap()));
                //assert_eq!(response.content_type(), "text/plain");
                //assert_eq!(response.charset(), "utf-8");
                //assert_eq!(response.into_string().unwrap(), "hello, world!");
            }
        );
    }

    #[macro_export]
    macro_rules! request {
        ( $handler:expr, $test:expr ) => {
            #[cfg(target_arch = "wasm32")]
            {
                use lunatic::net::TcpStream;

                let mailbox = unsafe { lunatic::Mailbox::new() };
                let this = lunatic::process::this(&mailbox);
                let server = |parent, mailbox| {
                    let handler = $handler;
                    crate::http::server((parent, handler), mailbox)
                };
                let _server_proc = match lunatic::process::spawn_with(this.clone(), server) {
                    Ok(proc) => proc,
                    Err(e) => {
                        eprintln!("process error: {}", e);
                        panic!();
                    }
                };
                match mailbox.receive() {
                    Ok(()) => {}
                    Err(e) => {
                        eprintln!("receive error: {}", e);
                        panic!();
                    }
                }

                // Run the entire test in a lunatic process because
                // `println!` doesn't work outside of one.
                let _client_proc =
                    match lunatic::process::spawn_with(this, |parent, _mailbox: lunatic::Mailbox<()>| {
                        $test
                        parent.send(());
                    }) {
                        Ok(proc) => proc,
                        Err(e) => {
                            eprintln!("process error: {}", e);
                            panic!();
                        }
                    };
                match mailbox.receive() {
                    Ok(()) => {}
                    Err(e) => {
                        eprintln!("receive error: {}", e);
                        panic!();
                    }
                }
            }

            #[cfg(not(target_arch = "wasm32"))]
            {
                use std::net::TcpStream;

                let rt = tokio::runtime::Builder::new_current_thread().enable_io().build().unwrap();
                rt.block_on(async {
                    async fn hello_world(hyper_request: hyper::Request<hyper::Body>) -> hyper::Result<hyper::Response<hyper::Body>> {
                        let (parts, body) = hyper_request.into_parts();
                        let body = match hyper::body::to_bytes(body).await {
                            Ok(buf) => buf,
                            Err(e) => {
                                eprintln!("aggregate error: {}", e);
                                panic!();
                            },
                        };
                        let request = crate::http::Request {
                            path: &parts.uri.to_string(),
                            method: parts.method.as_str(),
                            body: &body[..],
                        };
                        let handler = $handler;
                        let response = crate::http::Handler::handle(&handler, &request);
                        let hyper_response = hyper::Response::new(response.body.into());
                        Ok(hyper_response)
                    }

                    let addr = std::net::SocketAddr::from(([127, 0, 0, 1], 3000));

                    let make_svc = hyper::service::make_service_fn(|_conn| async {
                        Ok::<_, std::convert::Infallible>(hyper::service::service_fn(hello_world))
                    });
                    let server = hyper::Server::bind(&addr).serve(make_svc);
                    println!("server listening on 127.0.0.1:3000");

                    let (tx, rx) = tokio::sync::oneshot::channel::<()>();
                    let graceful = server.with_graceful_shutdown(async {
                        rx.await.unwrap();
                    });

                    let server_join_handle = tokio::spawn(async {
                        if let Err(e) = graceful.await {
                            panic!("server error: {}", e);
                        }
                    });

                    let client_join_handle = tokio::task::spawn_blocking(|| $test);
                    client_join_handle.await.unwrap();

                    tx.send(()).unwrap();
                    server_join_handle.await.unwrap();
                });
            }
        };
    }
}

//#[cfg(all(target_arch = "aarch64", test))]
//mod test_aarch64 {
//    use hyper::service::{make_service_fn, service_fn};
//    use hyper::{Body, Request, Response, Result, Server};
//    use regex::Regex;
//    use std::convert::Infallible;
//    use std::net::SocketAddr;
//    use tokio::runtime::Builder;
//    use tokio::sync::oneshot;
//
//    #[test]
//    fn test() {
//        let rt = Builder::new_current_thread().enable_io().build().unwrap();
//        rt.block_on(async {
//            async fn hello_world(_req: Request<Body>) -> Result<Response<Body>> {
//                Ok(Response::new("hello, world!".into()))
//            }
//
//            let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
//
//            let make_svc =
//                make_service_fn(|_conn| async { Ok::<_, Infallible>(service_fn(hello_world)) });
//            let server = Server::bind(&addr).serve(make_svc);
//            println!("server listening on http://127.0.0.1:3000");
//
//            let (tx, rx) = oneshot::channel::<()>();
//            let graceful = server.with_graceful_shutdown(async {
//                rx.await.unwrap();
//            });
//
//            let server_join_handle = tokio::spawn(async {
//                if let Err(e) = graceful.await {
//                    panic!("server error: {}", e);
//                }
//            });
//
//            let client_join_handle = tokio::task::spawn_blocking(|| {
//                println!("client requesting GET http://127.0.0.1:3000");
//                let response = ureq::get("http://127.0.0.1:3000").call().unwrap();
//                assert_eq!(response.get_url(), "http://127.0.0.1:3000/");
//                assert_eq!(response.http_version(), "HTTP/1.1");
//                assert_eq!(response.status(), 200);
//                assert_eq!(response.status_text(), "OK");
//                assert_eq!(response.headers_names(), vec!["content-length", "date"]);
//                assert_eq!(response.header("content-length").unwrap(), "13");
//                let re = Regex::new(r"^Fri, \d{2} Jan \d{4} \d{2}:\d{2}:\d{2} GMT").unwrap();
//                assert!(re.is_match(response.header("date").unwrap()));
//                assert_eq!(response.content_type(), "text/plain");
//                assert_eq!(response.charset(), "utf-8");
//                assert_eq!(response.into_string().unwrap(), "hello, world!");
//            });
//            client_join_handle.await.unwrap();
//
//            tx.send(()).unwrap();
//            server_join_handle.await.unwrap();
//        });
//    }
//}
