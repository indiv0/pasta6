use lunatic::{process::Process, Mailbox};

use crate::http::{Handler, Method, Request, Response};

struct App;

#[inline]
pub(crate) fn server(parent: Process<()>, mailbox: Mailbox<()>) {
    crate::http::server((parent, App), mailbox)
}

impl Handler for App {
    fn handle<'request, 'response>(
        &self,
        request: &'request Request<'request>,
    ) -> Response<'response> {
        println!("server handling request");
        assert_eq!(request.method(), Method::Get);
        assert_eq!(request.path(), "/");
        assert_eq!(request.body(), b"");
        Response::from_static(200, "hello, world!")
    }
}