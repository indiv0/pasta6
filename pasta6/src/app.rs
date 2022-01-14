use lunatic::{
    process::{self, Process},
    Mailbox,
};

use crate::http::{Handler, Method, Request, Response};

struct App;

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

impl Handler for App {
    fn handle<'request, 'response>(request: &'request Request<'request>) -> Response<'response> {
        tracing::debug!("server handling request");
        assert_eq!(request.method(), Method::Get);
        assert_eq!(request.path(), "/");
        assert_eq!(request.body(), b"");
        Response::from_static(200, "hello, world!")
    }
}
