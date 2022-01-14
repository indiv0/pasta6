use lunatic::{
    process::{self, Process},
    Mailbox,
};

use crate::http::{Handler, Method, Request, Response};

struct App;

#[cfg(target_arch = "wasm32")]
#[inline]
pub(crate) fn server() {
    fn server_(parent: Process<()>, mailbox: Mailbox<()>) {
        crate::http::server((parent, App::handle, 3000), mailbox)
    }
    tracing::info!("starting application");
    let mailbox = unsafe { Mailbox::new() };
    let this = process::this(&mailbox);
    // Run the entire application in a lunatic process because `println!`
    // doesn't work outside of one.
    tracing::info!("spawning server process");
    crate::spawn_with!(this, server_).unwrap();
}

#[cfg(all(test, not(target_arch = "wasm32")))]
#[inline]
pub(crate) async fn server(
    port: u16,
) -> (
    tokio::sync::oneshot::Sender<()>,
    impl std::future::Future<Output = Result<(), tokio::task::JoinError>>,
) {
    async fn handle(
        hyper_request: hyper::Request<hyper::Body>,
    ) -> hyper::Result<hyper::Response<hyper::Body>> {
        let (parts, body) = hyper_request.into_parts();
        let body = match hyper::body::to_bytes(body).await {
            Ok(buf) => buf,
            Err(e) => {
                tracing::error!("aggregate error: {}", e);
                panic!();
            }
        };
        let uri = parts.uri.to_string();
        let request = crate::http::from_parts(&uri, parts.method.as_str(), &body);
        let handler = App::handle;
        let response = handler(&request);
        let hyper_response = response.into();
        Ok(hyper_response)
    }

    let addr = std::net::SocketAddr::from(([127, 0, 0, 1], port));

    let make_svc = hyper::service::make_service_fn(|_conn| async {
        Ok::<_, std::convert::Infallible>(hyper::service::service_fn(handle))
    });
    let server = hyper::Server::bind(&addr).serve(make_svc);
    tracing::info!("server listening on 127.0.0.1:{}", port);

    let (tx, rx) = tokio::sync::oneshot::channel::<()>();
    let graceful = server.with_graceful_shutdown(async {
        rx.await.unwrap();
    });

    let server_join_handle = tokio::spawn(async {
        if let Err(e) = graceful.await {
            panic!("server error: {}", e);
        }
    });

    (tx, server_join_handle)
}

impl Handler for App {
    #[inline]
    fn handle<'request, 'response>(request: &'request Request<'request>) -> Response<'response> {
        tracing::debug!("server handling request");
        match (request.method(), request.path()) {
            (Method::Get, "/") => {
                assert_eq!(request.body(), b"");
                Response::from_static(200, "hello, world!")
            }
            (Method::Get, _path) => Response::from_static(404, ""),
        }
    }
}

//#[cfg(all(test, target_arch = "wasm32"))]
//mod test {
//    #[test]
//    fn test_get() {
//        crate::app::server();
//    }
//}
//
//#[cfg(all(test, not(target_arch = "wasm32")))]
//mod test {
//    #[test]
//    fn test_get() {
//        let rt = tokio::runtime::Builder::new_current_thread()
//            .enable_io()
//            .build()
//            .unwrap();
//        rt.block_on(async {
//            let (tx, server) = crate::app::server(3000).await;
//
//            let client_join_handle = tokio::task::spawn_blocking(move || {
//                let tcp_stream = std::net::TcpStream::connect("127.0.0.1:3000")
//                    .unwrap()
//                    .into();
//                let client = crate::http::Client::new(tcp_stream).unwrap();
//            });
//            client_join_handle.await.unwrap();
//
//            tx.send(()).unwrap();
//            server.await.unwrap();
//        });
//    }
//}
