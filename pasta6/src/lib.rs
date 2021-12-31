use hyper::{
    body, rt::Executor, server::conn::Http, service, Body, Method, Request, Response, StatusCode,
};
use monoio::net::TcpListener;
use monoio_compat::TcpStreamCompat;
use sled::Db;

use std::{convert::Infallible, error::Error, future::Future, io, net::ToSocketAddrs, str};

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
    S: FnMut(Request<Body>) -> F + 'static + Clone,
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
                .serve_connection(stream, service::service_fn(service.clone())),
        );
    }
}

pub async fn handler(db: &Db, req: Request<Body>) -> Result<Response<Body>, Infallible> {
    match (req.method(), req.uri().path()) {
        (&Method::GET, "/") => Ok(Response::new(Body::from("Hello, world!"))),
        (&Method::GET, "/todo") => get_todo(req),
        (&Method::POST, "/todo") => post_todo(db, req).await,
        _ => Ok(Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(Body::from("404 not found"))
            .unwrap()),
    }
}

pub fn get_todo(req: Request<Body>) -> Result<Response<Body>, Infallible> {
    let response = Response::new(Body::from(
        "\
<html>\
<head></head>\
<body>\
  <form method=\"post\">\
    <label for=\"content\">TODO:</label>\
    <input type=\"text\" name=\"content\" id=\"content\" required>\
  </form>\
</body>\
</html>\
",
    ));
    Ok(response)
}

pub async fn post_todo(db: &Db, req: Request<Body>) -> Result<Response<Body>, Infallible> {
    use rand::Rng;

    let body = match body::to_bytes(req.into_body()).await {
        Ok(b) => b,
        Err(_e) => todo!(),
    };
    let string = match str::from_utf8(&body) {
        Ok(s) => s,
        Err(_e) => todo!(),
    };
    let mut pair = string.split('=');
    // FIXME: handle missing parts.
    let (key, value) = (pair.next().unwrap(), pair.next().unwrap());
    // FIXME: don't assert here?
    assert_eq!(key, "content");

    // TODO: use snowflakes instead of random keys & remove the rand dep.
    let key: u64 = rand::thread_rng().gen();

    // TODO: don't unwrap here
    db.insert(key.to_be_bytes(), value).unwrap();

    // TODO: what's the right way to iterate over all values?
    let todos = db.range(0u64.to_be_bytes()..).map(|e| e.map(|(_, v)| v));
    // TODO: do this more efficiently with templates.
    let todos: String = todos
        .map(|v| {
            format!("<li>{}</li>", unsafe {
                str::from_utf8_unchecked(&v.unwrap())
            })
        })
        .collect();

    let response = Response::new(Body::from(format!(
        "\
<html>\
<head></head>\
<body>\
  <form method=\"post\">\
    <label for=\"content\">TODO:</label>\
    <input type=\"text\" name=\"content\" id=\"content\" required>\
  </form>\
  <ul>{}<ul>\
</body>\
</html>\
",
        todos
    )));
    Ok(response)
}
