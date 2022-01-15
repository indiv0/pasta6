// FIXME: remove this attr
#![allow(dead_code)]

use lunatic::process;

mod app;
mod http;
mod net;

#[cfg(target_arch = "wasm32")]
pub fn run() {
    use crate::http::Handler;

    crate::app::server(crate::app::App::handle);
    loop {
        process::sleep(u64::MAX);
    }
}

/// Define a wrapper macro for `process::spawn` that initializes our
/// logger when a process is spawned. Unlike normal Rust applications, the
/// logger must be re-initialized for every process.
#[macro_export]
macro_rules! spawn {
    ( $function:expr ) => {
        lunatic::process::spawn(|mailbox| {
            #[cfg(feature = "logging")]
            tracing_subscriber::fmt::init();
            $function(mailbox)
        })
    };
}

/// Define a wrapper macro for `process::spawn_with` that initializes our
/// logger when a process is spawned. Unlike normal Rust applications, the
/// logger must be re-initialized for every process.
#[macro_export]
macro_rules! spawn_with {
    ( $context:expr, $function:expr ) => {
        lunatic::process::spawn_with($context, |context, mailbox| {
            #[cfg(feature = "logging")]
            tracing_subscriber::fmt::init();
            $function(context, mailbox)
        })
    };
}
