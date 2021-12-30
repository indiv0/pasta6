// The [`std`] crate is Rust's standard library. It assumes that the program
// will be run an operating system rather than [*directly on the metal*].
// The [`core`] crate is a subset of the `std` crate that makes zero
// assumptions about the system the program will run on. It lacks APIs for
// anything that involves heap memory allocations and I/O.
//
// For an application, `std` also takes care of (among other things) setting
// up stack overflow protection, processing command line arguments, and
// spawning the main thread before a program's `main` function is invoked.
//
// A `#![no_std]` application lacks all that standard runtime, so it must
// initialize its own runtime, if any is required.
//
// This crate must make minimal assumptions about the system is will run on,
// so we use the `#![no_std]` crate level attribute to indicate that the
// crate will link to the `core` crate instead of the `std` crate.
//
// [`std`]: https://doc.rust-lang.org/std/
// [`core`]: https://doc.rust-lang.org/core/
// [*directly on the metal*]: https://en.wikipedia.org/wiki/Bare_machine
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
