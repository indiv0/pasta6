use lunatic::{process::Process, Mailbox};

use crate::http::{Handler, Headers, Request, Response};

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
        //assert_eq!(request.method, "GET");
        //assert_eq!(request.path, "/");
        //assert_eq!(request.body, b"");
        Response::new(200, Headers::empty(), b"hello, world!"[..].into())
    }
}
