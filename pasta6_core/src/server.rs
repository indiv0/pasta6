use std::{convert::Infallible, env, net::{TcpListener, Ipv4Addr}};
use tracing_subscriber::fmt::format::FmtSpan;
use warp::{Reply, Filter, hyper};
use warp::{hyper::Server};
use listenfd::ListenFd;
use tracing::info;

pub fn bind() -> TcpListener {
    let mut listenfd = ListenFd::from_env();
    // if listenfd doesn't take a TcpListener (i.e. we're not running via the
    // command above), we fall back to explicitly binding to a given host:port.
    if let Some(l) = listenfd.take_tcp_listener(0).unwrap() {
        info!("initializing server with listenfd");
        l
    } else {
        let host: Ipv4Addr = env::var("PASTA6_HOST").expect("PASTA6_HOST unset").parse().unwrap();
        let address = format!("http://{}:{}", host, 3030);
        info!("initializing server on {}", address);
        TcpListener::bind(address).expect("failed to bind")
    }
}

pub async fn init_server<F>(routes: F)
    where F: Filter + Clone + Send + 'static,
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
        // the cline is there because not all warp filters impl Copy
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

pub async fn init_server2<F>(listener: TcpListener, routes: F) -> Result<(), hyper::error::Error>
    where F: Filter + Clone + Send + 'static,
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
        // the cline is there because not all warp filters impl Copy
        let svc = svc.clone();
        async move { Ok::<_, Infallible>(svc) }
    });

    let server = Server::from_tcp(listener).expect("failed to create the server");

    server.serve(make_svc).await
}

pub fn init_tracing(crate_name: &str) {
    // Filter traces based on the RUST_LOG env var, or, if it's not set,
    // default to show the output of the example.
    let env_filter = env::var("RUST_LOG").unwrap_or_else(|_| format!("{}=trace,tracing=info,warp=debug", crate_name));

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