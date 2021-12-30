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
