use lunatic::net::TcpStream;
use std::io::{self, Read, Write};

fn handle_connection<H>(mut tcp_stream: TcpStream, handle: H)
where
    H: for<'buf> Fn(&'buf Request<'buf>) -> Response<'buf>,
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
    let response = handle(&request);
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
    match tcp_stream.write_all(b"\r\ncontent-length: ") {
        Ok(()) => {}
        Err(e) => {
            eprintln!("write error: {}", e);
            panic!();
        }
    }
    match write!(tcp_stream, "{}", response.body.len()) {
        Ok(()) => {}
        Err(e) => {
            eprintln!("write error: {}", e);
            panic!();
        }
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
    match tcp_stream.write_all(response.body) {
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

struct Request<'buf> {
    method: &'buf str,
    path: &'buf str,
    body: &'buf [u8],
}

#[cfg_attr(test, derive(Debug))]
struct Response<'buf> {
    code: u16,
    reason: &'buf str,
    body: &'buf [u8],
}

#[cfg_attr(test, derive(Debug))]
enum ParseResult<T> {
    Ok(T),
    Partial,
    Error(httparse::Error),
}

impl Request<'_> {
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

impl<'reason, 'body, 'response> Response<'response>
where
    'reason: 'response,
    'body: 'response,
{
    fn new(code: u16, reason: &'reason str, body: &'body [u8]) -> Response<'response> {
        Self { code, reason, body }
    }
}

// Maximum number of headers allowed in an HTTP response.
const MAX_RESPONSE_HEADERS: usize = 16;

impl Response<'_> {
    // Parses a byte slice into an HTTP response.
    //
    // # Limitations
    //
    // `Response` can parse up to 16 headers. Attempting to parse a response
    // with more will result in an error.
    fn parse(buf: &[u8]) -> ParseResult<Response> {
        let mut headers = [httparse::EMPTY_HEADER; MAX_RESPONSE_HEADERS];
        let mut response = httparse::Response::new(&mut headers);
        let idx = match response.parse(buf) {
            Ok(httparse::Status::Complete(idx)) => idx,
            Ok(httparse::Status::Partial) => return ParseResult::Partial,
            Err(source) => return ParseResult::Error(source),
        };
        let body = match buf.get(idx..) {
            Some(body) => body,
            None => return ParseResult::Partial,
        };
        debug_assert!(response.code.is_some(), "missing code");
        debug_assert!(response.reason.is_some(), "missing reason");
        debug_assert!(response.version.is_some(), "missing version");
        let response = Response {
            code: response.code.unwrap(),
            reason: response.reason.unwrap(),
            body,
        };
        ParseResult::Ok(response)
    }
}

#[cfg(test)]
mod test {
    use crate::http::{ParseResult, Response, MAX_RESPONSE_HEADERS};
    use regex::Regex;
    use std::iter;

    #[test]
    fn response_with_too_many_headers() {
        let headers = iter::repeat("foo: bar")
            .take(MAX_RESPONSE_HEADERS + 1)
            .collect::<Vec<_>>()
            .join("\r\n");
        let response = format!("HTTP/1.1 200 OK\r\n{}", headers);
        let result = Response::parse(response.as_bytes());
        match result {
            ParseResult::Error(httparse::Error::TooManyHeaders) => {}
            other => panic!(
                "assertion failed; expected `TooManyHeaders`, got `{:?}",
                other
            ),
        }
    }

    #[test]
    fn test() {
        use crate::http::ParseResult;
        use std::io::{Read, Write};
        use std::str;

        crate::request!(
            {
                fn handle<'r>(request: &'r crate::http::Request<'r>) -> crate::http::Response<'r> {
                    println!("server handling request");
                    assert_eq!(request.method, "GET");
                    assert_eq!(request.path, "/");
                    assert_eq!(request.body, b"");
                    Response::new(200, "OK", b"hello, world!")
                }
                handle
            },
            {
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
                let mut buffer = [0; 1024];
                println!("client reading response");
                let mut bytes_read = 0;
                let response = loop {
                    bytes_read += match tcp_stream.read(&mut buffer[bytes_read..]) {
                        Ok(bytes_read) => bytes_read,
                        Err(e) => {
                            eprintln!("read error: {}", e);
                            panic!();
                        }
                    };
                    println!("client bytes read: {}", bytes_read);
                    let response_bytes = &buffer[..bytes_read];
                    let lossy_response_str = String::from_utf8_lossy(response_bytes);
                    println!("client parsing response: {}", lossy_response_str);
                    let response = match Response::parse(response_bytes) {
                        ParseResult::Ok(response) => response,
                        ParseResult::Partial => continue,
                        ParseResult::Error(e) => {
                            eprintln!("parse error: {}", e);
                            panic!();
                        }
                    };
                    let response_str = match str::from_utf8(response_bytes) {
                        Ok(response_str) => response_str,
                        Err(e) => {
                            eprintln!("utf-8 error: {}", e);
                            panic!();
                        }
                    };
                    println!("client received response: {}", response_str);
                    assert_eq!(bytes_read, 89);
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
                    break response;
                };
                assert_eq!(response.code, 200);
                assert_eq!(response.reason, "OK");
                assert_eq!(response.body, b"hello, world!");
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
        ( $handle:expr, $test:expr ) => {
            #[cfg(target_arch = "wasm32")]
            {
                use lunatic::net::TcpStream;

                fn server(parent: lunatic::process::Process<()>, _mailbox: lunatic::Mailbox<()>) {
                    use lunatic::net::TcpListener;

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
                                let handle = $handle;
                                crate::http::handle_connection(tcp_stream, handle);
                            }
                            Err(e) => {
                                eprintln!("accept error: {}", e);
                                panic!();
                            }
                        }
                    }
                }

                let mailbox = unsafe { lunatic::Mailbox::new() };
                let this = lunatic::process::this(&mailbox);
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
                        let handle = $handle;
                        let response = handle(&request);
                        let hyper_response = hyper::Response::new(response.body.to_vec().into());
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
