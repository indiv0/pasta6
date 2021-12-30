use hyper::{
    rt::Executor, server::conn::Http, service, Body, Method, Request, Response, StatusCode,
};
use monoio::net::TcpListener;
use monoio_compat::TcpStreamCompat;

use std::{convert::Infallible, error::Error, future::Future, io, net::ToSocketAddrs};

#[derive(Clone)]
pub struct HyperExecutor;

impl<F> Executor<F> for HyperExecutor
where
    F: Future + 'static,
    F::Output: 'static,
{
    fn execute(&self, future: F) {
        monoio::spawn(future);
    }
}

pub async fn serve<A, S, F, R>(addr: A, service: S) -> io::Result<()>
where
    A: ToSocketAddrs,
    S: FnMut(Request<Body>) -> F + 'static + Copy,
    F: Future<Output = Result<Response<Body>, R>> + 'static,
    R: Error + 'static + Send + Sync,
{
    let listener = TcpListener::bind(addr)?;
    loop {
        let (stream, _) = listener.accept().await?;
        let stream: TcpStreamCompat = stream.into();
        monoio::spawn(
            Http::new()
                .with_executor(HyperExecutor)
                .serve_connection(stream, service::service_fn(service)),
        );
    }
}

pub async fn handler(req: Request<Body>) -> Result<Response<Body>, Infallible> {
    match (req.method(), req.uri().path()) {
        (&Method::GET, "/") => Ok(Response::new(Body::from("Hello, world!"))),
        _ => Ok(Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(Body::from("404 not found"))
            .unwrap()),
    }
}
