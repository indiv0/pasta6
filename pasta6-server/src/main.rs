#![no_std]
// The `#![no_main]` attribute means that the program won't use the standard
// `main` function as its entry point. At the time of writing, Rust's `main`
// interface makes some assumptions about the environment the program
// executes in. For example, it assumes the existence of command line
// arguments so in general it's not appropriate for `#![no_std]` programs.
#![no_main]

// `#![no_std]` executables without a `libc` crate are not well defined in
// Rust and result in [cryptic errors]. Including the `libc` crate provides
// the necessary symbols.
//
// [cryptic errors]: https://github.com/rust-lang/rust/issues/17346
extern crate libc;

use core::panic::PanicInfo;

// A function marked with the`#[panic_handler]` attribute defines the
// behaviour of panics, both library level panics (`core::panic!`) and
// language level panics (out of bounds indexing).
#[panic_handler]
fn panic(_panic: &PanicInfo<'_>) -> ! {
    loop {}
}

#[no_mangle]
pub extern "C" fn main(_argc: i32, _argv: *const *const u8) -> i32 {
    0
}
