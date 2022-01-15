use lunatic::{
    process::{self, Process},
    Mailbox,
};

use crate::http::{Handler, Method, Request, Response};

pub(crate) struct App;

impl Handler for App {
    #[inline]
    fn handle<'request, 'response>(request: &'request Request<'request>) -> Response<'response> {
        tracing::trace!("App server handling request");
        match (request.method(), request.path()) {
            (Method::Get, "/") => {
                assert_eq!(request.body(), b"");
                const BODY: &str = "<html>\
                  <head>\
                    <title>Home</title>\
                  </head>\
                  <body>\
                  <form method=\"post\" action\"/\" enctype=\"multipart/form-data\">\
                    <label for=\"content\">TODO:</label>\
                    <input type=\"text\" name=\"content\" id=\"content\" required>\
                  </form>\
                  </body>\
                  </html>";
                Response::from_static(200, BODY)
            }
            (Method::Post, "/") => {
                unimplemented!();
            }
            (_method, _path) => Response::from_static(404, ""),
        }
    }
}

#[cfg(target_arch = "wasm32")]
#[inline]
pub(crate) fn server(handler: for<'r, 's> fn(&'r Request<'s>) -> Response<'r>) {
    let handler_int = crate::http::handler_as_int(handler);
    fn server_((parent, handler_int): (Process<()>, usize), mailbox: Mailbox<()>) {
        let handler = crate::http::handler_from_int(handler_int);
        crate::http::server((parent, handler, ([0, 0, 0, 0], 3000)), mailbox)
    }
    tracing::info!("starting application");
    let mailbox = unsafe { Mailbox::new() };
    let this = process::this(&mailbox);
    // Run the entire application in a lunatic process because `println!`
    // doesn't work outside of one.
    tracing::info!("spawning server process");
    crate::spawn_with!((this, handler_int), server_).unwrap();
    // Wait for the server to initialize.
    mailbox.receive().unwrap();
}

#[cfg(all(test, not(target_arch = "wasm32")))]
#[inline]
pub(crate) fn server(
    handler: for<'r, 's> fn(&'r Request<'s>) -> Response<'r>,
    callback: fn(u16),
    port: u16,
) {
    use std::sync::Arc;

    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_io()
        .build()
        .unwrap();
    rt.block_on(async {
        async fn handle(
            handler: Arc<for<'r, 's> fn(&'r Request<'s>) -> Response<'r>>,
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
            let response = handler(&request);
            let hyper_response = response.into();
            Ok(hyper_response)
        }

        let addr = std::net::SocketAddr::from(([127, 0, 0, 1], port));

        let handler = Arc::new(handler);
        let make_svc = hyper::service::make_service_fn(move |_conn| {
            let handler = handler.clone();
            async move {
                Ok::<_, std::convert::Infallible>(hyper::service::service_fn(move |req| {
                    let handler = handler.clone();
                    async move { handle(handler, req).await }
                }))
            }
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

        let client_join_handle = tokio::task::spawn_blocking(move || callback(port));
        client_join_handle.await.unwrap();
        tx.send(()).unwrap();
        server_join_handle.await.unwrap();
    });
}

#[cfg(all(test, target_arch = "wasm32"))]
mod test {
    use crate::{
        app::App,
        http::{Client, Handler, Method},
    };

    #[test]
    fn test_get() {
        crate::app::server(App::handle);

        let tcp_stream = lunatic::net::TcpStream::connect("127.0.0.1:3000")
            .unwrap()
            .into();
        let mut client = Client::new(tcp_stream).unwrap();
        let response = client.request(Method::Get, "/").unwrap();
        assert_eq!(response.code(), 200);
        assert_eq!(response.reason(), "OK");
        assert_eq!(response.headers().len(), 2);
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod test {
    use crate::app::App;
    use crate::http::{Client, Handler, Method};
    #[test]
    fn test_get() {
        let callback = |port| {
            let tcp_stream = std::net::TcpStream::connect("127.0.0.1:3000")
                .unwrap()
                .into();
            let mut client = Client::new(tcp_stream).unwrap();
            let response = client.request(Method::Get, "/").unwrap();
            assert_eq!(response.code(), 200);
            assert_eq!(response.reason(), "OK");
            assert_eq!(response.headers().len(), 2);
        };
        crate::app::server(App::handle, callback, 3000);
    }
}
