use std::{
    future::Future,
    io, mem,
    pin::Pin,
    task::{Context, Poll},
};

use hyper::{
    body,
    client::connect::{Connected, Connection},
    rt::Executor,
    service::Service,
    Body, Client, Method, Request, Uri,
};
use monoio::net::TcpStream;
use monoio_compat::{AsyncRead, AsyncWrite, TcpStreamCompat};
use tokio::io::ReadBuf;

#[derive(Clone)]
pub(crate) struct HyperConnector;

impl Service<Uri> for HyperConnector {
    type Response = HyperConnection;
    type Error = io::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, _: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, uri: Uri) -> Self::Future {
        let host = uri.host().unwrap();
        let port = uri.port().unwrap();
        let address = format!("{}:{}", host, port);

        let future: Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>>>> =
            Box::pin(async move {
                let connection = TcpStream::connect(address).await?;
                let hyper_connection = HyperConnection(connection.into());
                Ok(hyper_connection)
            });
        unsafe { mem::transmute(future) }
    }
}

#[derive(Clone)]
struct HyperExecutor;

impl<F> Executor<F> for HyperExecutor
where
    F: Future + 'static,
    F::Output: 'static,
{
    fn execute(&self, future: F) {
        monoio::spawn(future);
    }
}

pub(crate) struct HyperConnection(TcpStreamCompat);

impl AsyncRead for HyperConnection {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        Pin::new(&mut self.0).poll_read(cx, buf)
    }
}

impl AsyncWrite for HyperConnection {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<Result<usize, io::Error>> {
        Pin::new(&mut self.0).poll_write(cx, buf)
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), io::Error>> {
        Pin::new(&mut self.0).poll_flush(cx)
    }

    fn poll_shutdown(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Result<(), io::Error>> {
        Pin::new(&mut self.0).poll_shutdown(cx)
    }
}

impl Connection for HyperConnection {
    fn connected(&self) -> Connected {
        Connected::new()
    }
}

unsafe impl Send for HyperConnection {}

pub(crate) fn build() -> Client<HyperConnector> {
    let connector = HyperConnector;
    Client::builder()
        .executor(HyperExecutor)
        .build::<HyperConnector, Body>(connector)
}

pub(crate) async fn get(client: &Client<HyperConnector>, addr: Uri) -> Result<(), hyper::Error> {
    client.get(addr).await.map(|_| ())
}

pub(crate) async fn post(client: &Client<HyperConnector>, addr: Uri) -> Result<(), hyper::Error> {
    use rand::Rng;

    // TODO: generate random bodies more efficiently?.
    let value: u8 = rand::thread_rng().gen();
    let body = format!("content={}", value);

    let req = Request::builder()
        .uri(addr)
        .method(Method::POST)
        .body(Body::from(body))
        .unwrap();
    let response = client.request(req).await?;
    let _body = body::to_bytes(response.into_body()).await?;
    Ok(())
}
