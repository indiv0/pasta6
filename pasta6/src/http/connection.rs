use crate::http::{ConnectionError, Headers};
use crate::net::TcpStream;
use bytes::Bytes;
use std::fmt::{self, Formatter};
use std::io::Read;
use std::io::{self, Write};
use std::str;
use std::str::Utf8Error;

// Maximum number of headers allowed in an HTTP response.
const MAX_RESPONSE_HEADERS: usize = 16;
/// Maximum number of headers allowed in an HTTP request.
const MAX_REQUEST_HEADERS: usize = 100;
/// Initial buffer size allocated for an HTTP request.
const INIT_REQUEST_BUFFER_SIZE: usize = 1024;

#[cfg_attr(test, derive(Debug))]
pub(super) struct Connection {
    #[cfg_attr(not(test), allow(dead_code))]
    head_length: usize,
    #[cfg_attr(not(test), allow(dead_code))]
    buf: Vec<u8>,
    #[cfg_attr(not(test), allow(dead_code))]
    tcp_stream: TcpStream,
}

#[cfg_attr(test, derive(Debug))]
pub(crate) struct Response<'body> {
    code: u16,
    reason: &'static str,
    // TODO: find a way to avoid copying the headers?
    headers: Headers,
    body: Body<'body>,
}

pub(crate) struct Request<'body> {
    method: Method,
    // TODO: find a way to avoid copying the path
    path: String,
    // TODO: find a way to avoid copying the headers?
    headers: Headers,
    body: Body<'body>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Method {
    Get,
    Post,
}

#[cfg_attr(test, derive(Debug))]
pub(crate) struct Body<'a> {
    kind: BodyKind<'a>,
}

enum BodyKind<'a> {
    #[cfg_attr(not(test), allow(dead_code))]
    Connection {
        connection: &'a mut Connection,
        length: BodyLength,
    },
    Bytes(Bytes),
}

#[derive(Copy, Clone)]
#[cfg_attr(test, derive(Debug))]
enum BodyLength {
    Known(usize),
    Unknown,
    Empty,
}

#[derive(Debug)]
pub(super) struct ResponseError {
    kind: ResponseErrorKind,
    _source: Option<httparse::Error>,
}

#[derive(Debug)]
enum ResponseErrorKind {
    ParseHead,
    ParseInt,
    InvalidMessageFraming,
}

#[cfg_attr(test, derive(Debug))]
struct ParseIntError;

impl Connection {
    #[inline]
    pub(super) fn new(tcp_stream: TcpStream) -> Self {
        Self {
            head_length: 0,
            // FIXME: rename the constant? This is for both req/recv bufs.
            buf: vec![0; INIT_REQUEST_BUFFER_SIZE],
            tcp_stream,
        }
    }

    /// Parses a byte slice into an HTTP response.
    ///
    /// # Limitations
    ///
    /// `Response` can parse up to 16 headers. Attempting to parse a response
    /// with more will result in an error.
    // TODO: should we zero out `buf` every time this method is called?
    #[inline]
    pub(super) fn next_response(&mut self) -> Result<Response, ResponseError>
where {
        tracing::trace!("connection reading response");
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
                    tracing::error!("read error: {}", e);
                    panic!();
                }
            };
            tracing::trace!("connection bytes read: {}", bytes_read);
            let response_bytes = &mut self.buf[..bytes_read];
            let lossy_response_str = String::from_utf8_lossy(response_bytes);
            tracing::trace!("connection parsing response: {}", lossy_response_str);

            // FIXME: start reading the body after the head of the request.
            let mut httparse_headers = [httparse::EMPTY_HEADER; MAX_RESPONSE_HEADERS];
            let mut httparse_response = httparse::Response::new(&mut httparse_headers);
            match httparse_response.parse(response_bytes) {
                Ok(httparse::Status::Complete(head_length)) => {
                    self.head_length = head_length;
                    debug_assert!(httparse_response.code.is_some(), "missing code");
                    debug_assert!(httparse_response.reason.is_some(), "missing reason");
                    debug_assert!(httparse_response.version.is_some(), "missing version");
                    let response_str = match str::from_utf8(response_bytes) {
                        Ok(response_str) => response_str,
                        Err(e) => {
                            tracing::error!("utf-8 error: {}", e);
                            panic!();
                        }
                    };
                    tracing::trace!("client received response: {}", response_str);

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
                                    tracing::warn!("duplicate content-length header");
                                    continue;
                                } else {
                                    return Err(ResponseError {
                                        kind: ResponseErrorKind::InvalidMessageFraming,
                                        _source: None,
                                    });
                                }
                            }
                            content_length = Some(value);
                        }
                    }
                    if has_transfer_encoding {
                        unimplemented!()
                    }
                    let body_length = match content_length {
                        // RFC 7230 section 3.3.3 point 5:
                        // > If a valid Content-Length header field is present
                        // > without Transfer-Encoding, its decimal value defines the
                        // > expected message body length in octets.
                        Some(content_length) => {
                            let content_length =
                                usize_from_bytes(content_length).map_err(|_e| ResponseError {
                                    kind: ResponseErrorKind::ParseInt,
                                    _source: None,
                                })?;
                            tracing::trace!("response content-length: {}", content_length);
                            BodyLength::Known(content_length)
                        }
                        // RFC 7230 section 3.3.3 point 7:
                        // > Otherwise, this is a response message without a declared
                        // > message body length, so the message body length is
                        // > determined by the number of octets received prior to the
                        // > server closing the connection.
                        None => unimplemented!(),
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
                        _source: Some(source),
                    })
                }
            };
        }
    }

    #[inline]
    pub(super) fn next_request(&mut self) -> Result<Request, ConnectionError> {
        tracing::trace!("server handling connection");
        loop {
            // TODO: we probably shouldn't reuse the same buf on a single
            //   connection (although it is uni-directional...)
            // TODO: re-use this buffer between requests.
            // TODO: allow non-contiguous buffers to allow re-allocation.
            // TODO: add a limit to the size of a single HTTP header.
            // TODO: add a limit to the total size of the request head.
            // TODO: shrink this buffer after the request is processed?
            // Read as much data as possible from the TCP stream into the buffer.
            tracing::trace!("server reading stream");
            let mut bytes_read = 0;
            let mut reached_eof = false;
            loop {
                // FIXME: make resizing work again
                //// If there is no remaining space in the buffer to read into, then
                //// we need to grow the buffer.
                //// TODO: grow the buffer in powers of two to perform O(log n)
                ////   allocations rather than O(n) allocations.
                //if bytes_read >= self.buf.len() {
                //    debug_assert!(bytes_read == self.buf.len());
                //    const EMPTY_BUFFER: [u8; INIT_REQUEST_BUFFER_SIZE] =
                //        [0; INIT_REQUEST_BUFFER_SIZE];
                //    self.buf.extend_from_slice(&EMPTY_BUFFER);
                //}

                bytes_read += match self.tcp_stream.read(&mut self.buf[bytes_read..]) {
                    Ok(0) => {
                        tracing::trace!("reached EOF");
                        // If we've reached EOF and there are no unparsed bytes,
                        // then the client has closed the connection.
                        if bytes_read == 0 {
                            // FIXME: what do we return here?
                            return Err(ConnectionError::closed());
                        }
                        reached_eof = true;
                        bytes_read
                    }
                    Ok(bytes_read) => {
                        tracing::trace!("server bytes read: {}", bytes_read);
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
                // FIXME: remove these logs
                //tracing::trace!(
                //    "server parsing request: {}",
                //    String::from_utf8_lossy(&self.buf[..bytes_read])
                //);

                // Parse the request into an `httparse::Request`.
                let head_buf = &mut self.buf[..bytes_read];
                let mut headers = [httparse::EMPTY_HEADER; MAX_REQUEST_HEADERS];
                let mut request = httparse::Request::new(&mut headers);
                match request.parse(head_buf) {
                    Ok(httparse::Status::Complete(head_length)) => {
                        self.head_length = head_length;
                        debug_assert!(request.path.is_some(), "missing path");
                        debug_assert!(request.method.is_some(), "missing method");
                        debug_assert!(request.version.is_some(), "missing version");
                        // TODO: avoid re-parsing the request head once we're on the body.
                        // TODO: content-length may be omitted for some kinds of requests.
                        let body_length = match request
                            .headers
                            .iter()
                            .find(|h| h.name == "content-length")
                            .map(|h| h.value)
                        {
                            // FIXME: don't discard the source error
                            Some(content_length) => {
                                let content_length =
                                    crate::http::connection::usize_from_bytes(content_length)
                                        .map_err(|_source| ConnectionError::request_error())?;
                                BodyLength::Known(content_length)
                            }
                            None => {
                                // FIXME: we're assuming no content-length
                                //   means no message-body but this is
                                //   incorrect.
                                BodyLength::Empty
                            }
                        };
                        // FIXME: include the headers of the request.
                        let path = request.path.unwrap().to_string();
                        let method = request.method.unwrap().into();
                        let headers = request.headers.into();
                        let body = Body {
                            kind: BodyKind::Connection {
                                connection: self,
                                length: body_length,
                            },
                        };
                        let request = Request {
                            path,
                            method,
                            headers,
                            body,
                        };
                        return Ok(request);
                    }
                    Ok(httparse::Status::Partial) => {
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
                    Err(source) => match source {
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
                        // FIXME: return an error here
                        httparse::Error::TooManyHeaders => unimplemented!(),
                        e => {
                            tracing::error!("parse error: {}", e);
                            panic!()
                        }
                    },
                };
            }
        }
    }
}

impl<'body> Response<'body> {
    #[inline]
    pub(crate) fn from_static(code: u16, body: &'static str) -> Response<'body> {
        Self::new(code, Headers::empty(), body.as_bytes().into())
    }

    #[inline]
    pub(crate) fn code(&self) -> u16 {
        self.code
    }

    #[inline]
    pub(crate) fn reason(&self) -> &'static str {
        self.reason
    }

    #[inline]
    pub(crate) fn headers(&self) -> &Headers {
        &self.headers
    }

    #[inline]
    fn new(code: u16, headers: Headers, body: Body<'body>) -> Response<'body> {
        let reason = match code {
            200 => "OK",
            431 => "Request Header Fields Too Large",
            code => {
                tracing::error!("unknown status code: {}", code);
                unimplemented!()
            }
        };
        Self {
            code,
            reason,
            headers,
            body,
        }
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
impl From<Response<'_>> for hyper::Response<hyper::Body> {
    #[inline]
    fn from(response: Response) -> Self {
        hyper::Response::new(response.body.into())
    }
}

impl AsRef<str> for Method {
    #[inline]
    fn as_ref(&self) -> &str {
        match self {
            Method::Get => "GET",
            Method::Post => "POST",
        }
    }
}

// FIXME: replace this with `TryFrom`.
impl From<&str> for Method {
    #[inline]
    fn from(string: &str) -> Self {
        match string {
            "GET" => Method::Get,
            "POST" => Method::Post,
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
    pub(crate) fn path(&self) -> &str {
        &self.path
    }

    #[inline]
    pub(crate) fn headers(&self) -> &Headers {
        &self.headers
    }

    // TODO: have this consume the request since it involves reading from the
    //   stream.
    #[inline]
    pub(crate) fn body(&self) -> &Body {
        &self.body
    }
}

impl Body<'_> {
    #[inline]
    fn len(&self) -> BodyLength {
        match &self.kind {
            BodyKind::Connection {
                connection: _,
                length,
            } => *length,
            BodyKind::Bytes(bytes) => BodyLength::Known(bytes.len()),
        }
    }

    // FIXME: have this consume request since it involves reading from the
    //   stream.
    #[inline]
    pub(crate) fn to_string(&self) -> Result<String, Utf8Error> {
        match &self.kind {
            BodyKind::Connection { connection, length } => match length {
                BodyLength::Known(length) => {
                    // FIXME: should this be less than or leq?
                    assert!(connection.head_length + length <= connection.buf.len());
                    let buf = connection
                        .buf
                        .get(connection.head_length..connection.head_length + length);
                    assert!(buf.is_some());
                    let buf = buf.unwrap();
                    let string = str::from_utf8(buf)?.to_string();
                    Ok(string)
                }
                BodyLength::Unknown => unimplemented!(),
                // FIXME: avoid allocating a string here
                BodyLength::Empty => Ok(String::new()),
            },
            BodyKind::Bytes(bytes) => Ok(str::from_utf8(&bytes[..])?.to_string()),
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
impl From<Body<'_>> for hyper::Body {
    fn from(body: Body<'_>) -> Self {
        match body.kind {
            BodyKind::Connection {
                connection: _,
                length: _,
            } => unimplemented!(),
            BodyKind::Bytes(bytes) => bytes.into(),
        }
    }
}

#[cfg(test)]
impl fmt::Debug for BodyKind<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            BodyKind::Connection { connection, length } => match length {
                BodyLength::Known(length) => {
                    let buf =
                        &connection.buf[connection.head_length..connection.head_length + length];
                    let string = String::from_utf8_lossy(buf);
                    write!(f, "{}", string)
                }
                BodyLength::Unknown => {
                    // FIXME: not guaranteed to have read all the way to the
                    //   end here. Also not guaranteed to not be reading the
                    //   next request in the pipelin.
                    let buf = &connection.buf[connection.head_length..];
                    let string = String::from_utf8_lossy(buf);
                    write!(f, "{}", string)
                }
                BodyLength::Empty => Ok(()),
            },
            BodyKind::Bytes(bytes) => write!(f, "{}", String::from_utf8_lossy(bytes)),
        }
    }
}

impl fmt::Display for ResponseError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self.kind {
            ResponseErrorKind::ParseInt => write!(f, "parse integer error"),
            ResponseErrorKind::ParseHead => write!(f, "parse head error"),
            ResponseErrorKind::InvalidMessageFraming => write!(f, "invalid message framing"),
        }
    }
}

impl std::error::Error for ResponseError {}

// TODO: would it be better to `io::copy` the response into the `tcp_stream`?
#[inline]
pub(super) fn write_response(
    tcp_stream: &mut TcpStream,
    response: Response,
) -> Result<(), io::Error> {
    tcp_stream.write_all(b"HTTP/1.1 ")?;
    write!(tcp_stream, "{}", response.code)?;
    tcp_stream.write_all(b" ")?;
    tcp_stream.write_all(response.reason().as_bytes())?;
    match response.body.len() {
        BodyLength::Known(length) => {
            tcp_stream.write_all(b"\r\ncontent-length: ")?;
            write!(tcp_stream, "{}", length)?;
        }
        BodyLength::Unknown => unimplemented!(),
        BodyLength::Empty => unimplemented!(),
    }
    // FIXME: don't hardcode the timestamp here.
    tcp_stream.write_all(b"\r\ndate: Fri, 14 Jan 2022 02:28:00 GMT")?;
    tcp_stream.write_all(b"\r\n\r\n")?;
    match response.body.kind {
        BodyKind::Connection {
            connection: _,
            length: _,
        } => unimplemented!(),
        BodyKind::Bytes(ref bytes) => tcp_stream.write_all(bytes)?,
    }
    Ok(())
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

#[cfg(all(test, not(target_arch = "wasm32")))]
#[inline]
pub(crate) fn from_parts<'a>(
    path: String,
    method: &'a str,
    headers: Headers,
    body: &'a [u8],
) -> Request<'a> {
    Request {
        path,
        method: method.into(),
        headers,
        body: Body {
            // TODO: avoid this copy
            kind: BodyKind::Bytes(Bytes::copy_from_slice(body)),
        },
    }
}

#[cfg(test)]
mod test {
    use crate::http::{Connection, Handler, Headers, Method, Request, Response};
    #[cfg(target_arch = "wasm32")]
    use lunatic::net::TcpStream;
    use regex::Regex;
    use std::io::Write;
    #[cfg(not(target_arch = "wasm32"))]
    use std::net::TcpStream;
    use std::str;

    use super::MAX_REQUEST_HEADERS;

    struct HelloWorld;

    impl Handler for HelloWorld {
        fn handle<'request, 'response>(
            request: &'request Request<'request>,
        ) -> Response<'response> {
            tracing::debug!("HelloWorld server handling request");
            assert_eq!(request.method, Method::Get);
            assert_eq!(request.path, "/");
            assert_eq!(request.body.to_string().unwrap(), "");
            Response::new(200, Headers::empty(), b"hello, world!"[..].into())
        }
    }

    #[test]
    fn test_hello_world() {
        #[cfg(feature = "logging")]
        let _ = tracing_subscriber::fmt::try_init();
        let port = random_port();
        crate::request!(HelloWorld::handle, port, |port: u16| {
            tracing::debug!("connecting to 127.0.0.1:{}", port);
            let tcp_stream = TcpStream::connect(format!("127.0.0.1:{}", port)).unwrap();
            let mut tcp_stream = crate::net::TcpStream::from(tcp_stream);
            tcp_stream
                .write_all(b"GET / HTTP/1.1\r\nUser-Agent: curl/7.76.1\r\nAccept: */*\r\n\r\n")
                .unwrap();
            let mut connection = Connection::new(tcp_stream);
            let response = connection.next_response().unwrap();
            assert_eq!(response.code, 200);
            assert_eq!(response.reason, "OK");
            assert_eq!(response.headers.len(), 2);
            assert_eq!(
                str::from_utf8(response.headers.get("content-length").unwrap()).unwrap(),
                "13"
            );
            let date = str::from_utf8(response.headers.get("date").unwrap()).unwrap();
            const DATE_REGEX: &str =
                r"^(Mon|Tue|Wed|Thu|Fri|Sat|Sun), \d{2} Jan \d{4} \d{2}:\d{2}:\d{2} GMT$";
            assert!(Regex::new(DATE_REGEX).unwrap().is_match(date));
            assert_eq!(
                response.body.to_string().as_ref().map(String::as_str),
                Ok("hello, world!")
            );
        });
    }

    // FIXME: uncomment this once fixed
    //#[test]
    //fn test_too_many_headers() {
    //    #[cfg(feature = "logging")]
    //    let _ = tracing_subscriber::fmt::try_init();
    //    let port = random_port();
    //    crate::request!(HelloWorld::handle, port, |port: u16| {
    //        let mut tcp_stream = TcpStream::connect(format!("127.0.0.1:{}", port)).unwrap();
    //        let mut tcp_stream = crate::net::TcpStream::from(tcp_stream);
    //        tcp_stream.write_all(b"GET / HTTP/1.1\r\n").unwrap();
    //        for _ in 0..(MAX_REQUEST_HEADERS + 1) {
    //            tcp_stream.write_all(b"foo: bar\r\n").unwrap();
    //        }
    //        tcp_stream.write_all(b"\r\n").unwrap();
    //        let mut connection = Connection::new(tcp_stream);
    //        let response = connection.next_response().unwrap();
    //        assert_eq!(response.code, 431);
    //        assert_eq!(response.reason, "Request Header Fields Too Large");
    //        assert_eq!(response.headers.len(), 2);
    //        assert_eq!(response.headers.get("content-length"), Some(&b"0"[..]));
    //        let date = str::from_utf8(response.headers.get("date").unwrap()).unwrap();
    //        const DATE_REGEX: &str =
    //            r"^(Mon|Tue|Wed|Thu|Fri|Sat|Sun), \d{2} Jan \d{4} \d{2}:\d{2}:\d{2} GMT$";
    //        assert!(Regex::new(DATE_REGEX).unwrap().is_match(date));
    //        assert_eq!(
    //            response.body.to_string().as_ref().map(String::as_str),
    //            Ok("")
    //        );
    //    });
    //}

    #[test]
    fn test_multiple_requests() {
        #[cfg(feature = "logging")]
        let _ = tracing_subscriber::fmt::try_init();
        let port = random_port();
        crate::request!(HelloWorld::handle, port, |port: u16| {
            let mut tcp_stream = TcpStream::connect(format!("127.0.0.1:{}", port)).unwrap();
            let mut tcp_stream = crate::net::TcpStream::from(tcp_stream);
            tcp_stream.write_all(b"GET / HTTP/1.1\r\n\r\n").unwrap();
            let mut connection = Connection::new(tcp_stream.try_clone().unwrap());
            let response = connection.next_response().unwrap();
            assert_eq!(response.code, 200);
            tcp_stream.write_all(b"GET / HTTP/1.1\r\n\r\n").unwrap();
            let response = connection.next_response().unwrap();
            assert_eq!(response.code, 200);
        });
    }

    #[macro_export]
    macro_rules! request {
        ( $handler:expr, $port:expr, $test:expr ) => {
            #[cfg(target_arch = "wasm32")]
            {
                let mailbox = unsafe { lunatic::Mailbox::new() };
                let this = lunatic::process::this(&mailbox);
                let _server_proc = match crate::spawn_with!(
                    (this.clone(), ([127, 0, 0, 1], $port)),
                    |(parent, addr), mailbox| {
                        let handler = $handler;
                        crate::http::server((parent, handler, addr), mailbox)
                    }
                ) {
                    Ok(proc) => proc,
                    Err(e) => {
                        tracing::error!("process error: {}", e);
                        panic!();
                    }
                };
                match mailbox.receive() {
                    Ok(()) => {}
                    Err(e) => {
                        tracing::error!("receive error: {}", e);
                        panic!();
                    }
                }

                fn client(
                    (parent, port): (lunatic::process::Process<()>, u16),
                    _mailbox: lunatic::Mailbox<()>,
                ) {
                    $test(port);
                    parent.send(());
                }

                // Run the entire test in a lunatic process because
                // `println!` doesn't work outside of one.
                let _client_proc = match crate::spawn_with!((this, $port), client) {
                    Ok(proc) => proc,
                    Err(e) => {
                        tracing::error!("process error: {}", e);
                        panic!();
                    }
                };
                match mailbox.receive() {
                    Ok(()) => {}
                    Err(e) => {
                        tracing::error!("receive error: {}", e);
                        panic!();
                    }
                }
            }

            #[cfg(not(target_arch = "wasm32"))]
            {
                let callback = move |port| $test(port);
                crate::app::server($handler, callback, $port);
            }
        };
    }

    /// Returns a random non-privileged port.
    fn random_port() -> u16 {
        use rand::Rng;
        rand::thread_rng().gen_range(1025..=65535)
    }
}
