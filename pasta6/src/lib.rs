use lunatic::{process, Mailbox};

mod app;
mod http;

pub fn run() {
    let mailbox = unsafe { Mailbox::new() };
    let this = process::this(&mailbox);
    // Run the entire application in a lunatic process because `println!`
    // doesn't work outside of one.
    process::spawn_with(this, crate::app::server).unwrap();
    loop {}
}
