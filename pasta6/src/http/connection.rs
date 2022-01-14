use crate::http::Headers;
use bytes::Bytes;
#[cfg(target_arch = "wasm32")]
use lunatic::net::TcpStream;
#[cfg(test)]
use std::fmt::{self, Formatter};
#[cfg(test)]
use std::io::Read;
use std::io::{self, Write};
#[cfg(not(target_arch = "wasm32"))]
use std::net::TcpStream;
use std::str;
#[cfg(test)]
use std::str::Utf8Error;

// Maximum number of headers allowed in an HTTP response.
#[cfg(test)]
const MAX_RESPONSE_HEADERS: usize = 16;

#[cfg_attr(test, derive(Debug))]
pub(super) struct Connection {
    #[cfg_attr(not(test), allow(dead_code))]
    head_length: usize,
    #[cfg_attr(not(test), allow(dead_code))]
    buf: [u8; 1024],
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

#[cfg_attr(test, derive(Debug))]
pub(super) struct Body<'a> {
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
}

#[cfg(test)]
#[cfg_attr(test, derive(Debug))]
pub(super) struct ResponseError {
    _kind: ResponseErrorKind,
    _source: Option<httparse::Error>,
}

#[cfg(test)]
#[cfg_attr(test, derive(Debug))]
enum ResponseErrorKind {
    ParseHead,
    ParseInt,
    InvalidMessageFraming,
}

#[cfg(test)]
#[cfg_attr(test, derive(Debug))]
struct ParseIntError;

impl Connection {
    #[inline]
    #[cfg(test)]
    pub(super) fn new(tcp_stream: TcpStream) -> Self {
        Self {
            head_length: 0,
            buf: [0; 1024],
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
    #[cfg(test)]
    pub(super) fn next_response(&mut self) -> Result<Response, ResponseError>
where {
        tracing::debug!("connection reading response");
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
            tracing::debug!("connection bytes read: {}", bytes_read);
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
                                    continue;
                                } else {
                                    return Err(ResponseError {
                                        _kind: ResponseErrorKind::InvalidMessageFraming,
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
                    let body_length =
                        match content_length {
                            // RFC 7230 section 3.3.3 point 5:
                            // > If a valid Content-Length header field is present
                            // > without Transfer-Encoding, its decimal value defines the
                            // > expected message body length in octets.
                            Some(content_length) => BodyLength::Known(
                                usize_from_bytes(content_length).map_err(|_e| ResponseError {
                                    _kind: ResponseErrorKind::ParseInt,
                                    _source: None,
                                })?,
                            ),
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
                        _kind: ResponseErrorKind::ParseHead,
                        _source: Some(source),
                    })
                }
            };
        }
    }
}

impl<'body> Response<'body> {
    #[inline]
    pub(crate) fn from_static(code: u16, body: &'static str) -> Response<'body> {
        Self::new(code, Headers::empty(), body.as_bytes().into())
    }

    #[inline]
    pub(super) fn reason(&self) -> &'static str {
        self.reason
    }

    #[inline]
    pub(super) fn headers(&self) -> &Headers {
        &self.headers
    }

    #[inline]
    fn new(code: u16, headers: Headers, body: Body<'body>) -> Response<'body> {
        let reason = match code {
            200 => "OK",
            431 => "Request Header Fields Too Large",
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

#[cfg(all(test, not(target_arch = "wasm32")))]
impl From<Response<'_>> for hyper::Response<hyper::Body> {
    fn from(response: Response) -> Self {
        hyper::Response::new(response.body.into())
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

    #[inline]
    #[cfg(test)]
    fn to_string(&self) -> Result<String, Utf8Error> {
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
            },
            BodyKind::Bytes(bytes) => write!(f, "{}", String::from_utf8_lossy(bytes)),
        }
    }
}

// TODO: would it be better to `io::copy` the response into the `tcp_stream`?
#[inline]
pub(super) fn write_response(
    tcp_stream: &mut lunatic::net::TcpStream,
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
#[cfg(test)]
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
    use crate::http::{
        Connection, Handler, Headers, Method, Request, Response, MAX_REQUEST_HEADERS,
    };
    #[cfg(target_arch = "wasm32")]
    use lunatic::net::TcpStream;
    use regex::Regex;
    use std::io::Write;
    #[cfg(not(target_arch = "wasm32"))]
    use std::net::TcpStream;
    use std::str;

    struct HelloWorld;

    impl Handler for HelloWorld {
        fn handle<'request, 'response>(
            request: &'request Request<'request>,
        ) -> Response<'response> {
            tracing::debug!("server handling request");
            assert_eq!(request.method, Method::Get);
            assert_eq!(request.path, "/");
            assert_eq!(request.body, b"");
            Response::new(200, Headers::empty(), b"hello, world!"[..].into())
        }
    }

    #[test]
    fn test_hello_world() {
        #[cfg(feature = "logging")]
        let _ = tracing_subscriber::fmt::try_init();
        let port = random_port();
        crate::request!(HelloWorld::handle, port, |port: u16| {
            let mut tcp_stream = TcpStream::connect(format!("127.0.0.1:{}", port)).unwrap();
            tcp_stream
                .write_all(b"GET / HTTP/1.1\r\nUser-Agent: curl/7.76.1\r\nAccept: */*\r\n\r\n")
                .unwrap();
            let mut connection = Connection::new(tcp_stream);
            let response = connection.next_response().unwrap();
            assert_eq!(response.code, 200);
            assert_eq!(response.reason, "OK");
            assert_eq!(response.headers.len(), 2);
            assert_eq!(response.headers.get("content-length"), Some(&b"13"[..]));
            let date = str::from_utf8(response.headers.get("date").unwrap()).unwrap();
            const DATE_REGEX: &str = r"^Fri, \d{2} Jan \d{4} \d{2}:\d{2}:\d{2} GMT$";
            assert!(Regex::new(DATE_REGEX).unwrap().is_match(date));
            assert_eq!(
                response.body.to_string().as_ref().map(String::as_str),
                Ok("hello, world!")
            );
        });
    }

    #[test]
    fn test_too_many_headers() {
        #[cfg(feature = "logging")]
        let _ = tracing_subscriber::fmt::try_init();
        let port = random_port();
        crate::request!(HelloWorld::handle, port, |port: u16| {
            let mut tcp_stream = TcpStream::connect(format!("127.0.0.1:{}", port)).unwrap();
            tcp_stream.write_all(b"GET / HTTP/1.1\r\n").unwrap();
            for _ in 0..(MAX_REQUEST_HEADERS + 1) {
                tcp_stream.write_all(b"foo: bar\r\n").unwrap();
            }
            tcp_stream.write_all(b"\r\n").unwrap();
            let mut connection = Connection::new(tcp_stream);
            let response = connection.next_response().unwrap();
            assert_eq!(response.code, 431);
            assert_eq!(response.reason, "Request Header Fields Too Large");
            assert_eq!(response.headers.len(), 2);
            assert_eq!(response.headers.get("content-length"), Some(&b"0"[..]));
            let date = str::from_utf8(response.headers.get("date").unwrap()).unwrap();
            const DATE_REGEX: &str = r"^Fri, \d{2} Jan \d{4} \d{2}:\d{2}:\d{2} GMT$";
            assert!(Regex::new(DATE_REGEX).unwrap().is_match(date));
            assert_eq!(
                response.body.to_string().as_ref().map(String::as_str),
                Ok("")
            );
        });
    }

    fn clone_tcp_stream(tcp_stream: &TcpStream) -> TcpStream {
        #[cfg(target_arch = "wasm32")]
        return tcp_stream.clone();
        #[cfg(not(target_arch = "wasm32"))]
        return tcp_stream.try_clone().unwrap();
    }

    #[test]
    fn test_multiple_requests() {
        #[cfg(feature = "logging")]
        let _ = tracing_subscriber::fmt::try_init();
        let port = random_port();
        crate::request!(HelloWorld::handle, port, |port: u16| {
            let mut tcp_stream = TcpStream::connect(format!("127.0.0.1:{}", port)).unwrap();
            tcp_stream.write_all(b"GET / HTTP/1.1\r\n\r\n").unwrap();
            let mut connection = Connection::new(clone_tcp_stream(&tcp_stream));
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
                let _server_proc =
                    match crate::spawn_with!((this.clone(), $port), |(parent, port), mailbox| {
                        let handler = $handler;
                        crate::http::server((parent, handler, port), mailbox)
                    }) {
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
                let rt = tokio::runtime::Builder::new_current_thread()
                    .enable_io()
                    .build()
                    .unwrap();
                rt.block_on(async {
                    let (tx, server) = crate::app::server($port).await;
                    let client_join_handle = tokio::task::spawn_blocking(move || $test($port));
                    client_join_handle.await.unwrap();

                    tx.send(()).unwrap();
                    server.await.unwrap();
                });
            }
        };
    }

    /// Returns a random non-privileged port.
    fn random_port() -> u16 {
        use rand::Rng;
        rand::thread_rng().gen_range(1025..=65535)
    }
}
