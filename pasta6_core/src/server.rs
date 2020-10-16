use futures_util::future::FutureExt;
use listenfd::ListenFd;
use std::{convert::Infallible, env, net::{Ipv4Addr, TcpListener}, panic::AssertUnwindSafe};
use tracing::{error, info};
use tracing_subscriber::fmt::format::FmtSpan;
use warp::{Future, hyper::{Body, Response, Server, service::Service, StatusCode, service::service_fn}};
use warp::{hyper, Filter, Reply};

use crate::Config;

type HttpResult = Result<Response<Body>, Infallible>;

pub fn bind() -> TcpListener {
    let mut listenfd = ListenFd::from_env();
    // if listenfd doesn't take a TcpListener (i.e. we're not running via the
    // command above), we fall back to explicitly binding to a given host:port.
    if let Some(l) = listenfd.take_tcp_listener(0).unwrap() {
        info!("initializing server with listenfd");
        l
    } else {
        let host: Ipv4Addr = env::var("PASTA6_HOST")
            .expect("PASTA6_HOST unset")
            .parse()
            .unwrap();
        let port: u16 = env::var("PASTA6_PORT")
            .expect("PASTA6_PORT unset")
            .parse()
            .unwrap();
        let address = format!("{}:{}", host, port);
        info!("initializing server on {}", address);
        TcpListener::bind(address).expect("failed to bind")
    }
}

pub async fn init_server<F>(routes: F)
where
    F: Filter + Clone + Send + 'static,
    F::Extract: Reply,
{
    // Wrap all the routes with a filter that creates a `tracing` span for
    // each request we receive, including data about the request.
    let routes = routes.with(warp::trace::request());

    // hyper lets us build a server from a TcpListener. Thus, we'll need to
    // convert our `warp::Filter` into a `hyper::service::MakeService` for use
    // with a `hyper::server::Server`.
    let svc = warp::service(routes);

    let make_svc = hyper::service::make_service_fn(|_: _| {
        // the clone is there because not all warp filters impl Copy
        let svc = svc.clone();
        async move { Ok::<_, Infallible>(svc) }
    });

    let mut listenfd = ListenFd::from_env();
    // if listenfd doesn't take a TcpListener (i.e. we're not running via the
    // command above), we fall back to explicitly binding to a given host:port.
    let server = if let Some(l) = listenfd.take_tcp_listener(0).unwrap() {
        info!("initializing server with listenfd");
        Server::from_tcp(l).unwrap()
    } else {
        let host: Ipv4Addr = env::var("PASTA6_HOST")
            .expect("PASTA6_HOST unset")
            .parse()
            .unwrap();
        info!("initializing server on {}:{}", host, 3030);
        Server::bind(&(host, 3030).into())
    };

    server.serve(make_svc).await.unwrap();
}

// We require that the config is passed in to ensure that is loaded before the server starts,
// as our config is lazy-loaded and would cause errors at request time if it was malformed.
pub async fn init_server2<F>(
    _config: &Config,
    listener: TcpListener,
    routes: F,
) -> Result<(), hyper::error::Error>
where
    F: Filter + Clone + Send + 'static,
    F::Extract: Reply,
{
    // Wrap all the routes with a filter that creates a `tracing` span for
    // each request we receive, including data about the request.
    let routes = routes.with(warp::trace::request());

    // hyper lets us build a server from a TcpListener. Thus, we'll need to
    // convert our `warp::Filter` into a `hyper::service::MakeService` for use
    // with a `hyper::server::Server`.
    let svc = warp::service(routes);

    let make_svc = hyper::service::make_service_fn(move |_: _| {
        // the clone is there because not all warp filters impl Copy
        let mut svc = svc.clone();
        // Run `svc.call(req)` immediately, which produces a `Future`. Feed that future to `handle_panics` to
        // wrap it up and return a panic-handling future. This is done so that a panic in the handler doesn't
        // terminate the request, but instead returns a properly-formed 500 response.
        async move { Ok::<_, Infallible>(service_fn(move |req| handle_panics(svc.call(req)))) }
    });

    let server = Server::from_tcp(listener).expect("failed to create the server");

    server.serve(make_svc).await
}

/// Wrapper function for hyper services
async fn handle_panics(fut: impl Future<Output = HttpResult>) -> HttpResult {
    // Turn panics falling out of the `poll` into errors.
    let wrapped = AssertUnwindSafe(fut).catch_unwind();
    match wrapped.await {
        Ok(response) => response,
        Err(_panic) => {
            error!("A panic occurred while handling a request");
            // TODO: ideally we'd display a themed/templated 500 error page here, instead of a minimal, non-HTML
            // response.
            let error = Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body("Internal server error".into())
                .expect("Failed to construct response");
            Ok(error)
        }
    }
}

pub fn init_tracing(crate_name: &str) {
    // Filter traces based on the RUST_LOG env var, or, if it's not set,
    // default to show the output of the example.
    let env_filter = env::var("RUST_LOG").unwrap_or_else(|_| {
        format!(
            "pasta6_core=trace,{}=trace,tracing=info,warp=debug",
            crate_name
        )
    });

    // Configure the default `tracing` subscriber.
    // The `fmt` subscriber from the `tracing-subscriber` crate logs `tracing`
    // events to stdout. Other subscribers are available for integrating with
    // distributed tracing systems such as OpenTelemetry.
    tracing_subscriber::fmt()
        // Use the filter we built above to determine which traces to record.
        .with_env_filter(env_filter)
        // Record an event when each span closes. This can be used to time our
        // routes' durations!
        .with_span_events(FmtSpan::CLOSE)
        .init();
}
