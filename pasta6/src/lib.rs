use lunatic::{process, Mailbox};

mod app;
mod http;

pub fn run() {
    tracing::info!("starting application");
    let mailbox = unsafe { Mailbox::new() };
    let this = process::this(&mailbox);
    // Run the entire application in a lunatic process because `println!`
    // doesn't work outside of one.
    tracing::info!("spawning server process");
    spawn_with!(this, crate::app::server).unwrap();
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
