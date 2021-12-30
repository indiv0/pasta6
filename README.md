# pasta6

## Crates

- [`pasta6`] - core library; provides I/O functionality
- [`pasta6-io-uring`]: userspace interface for [`io_uring`].
- [`pasta6-server`]: pasta6 API server implementation

[`pasta6`]: ./pasta6
[`pasta6-io-uring`]: ./io-uring
[`io_uring`]: https://kernel.dk/io_uring.pdf
[`pasta6-server`]: ./io-uring

## `#![no_std]`

The [`std`] crate is Rust's standard library. It assumes that the program
will be run an operating system rather than [*directly on the metal*].  The
[`core`] crate is a subset of the `std` crate that makes zero assumptions
about the system the program will run on. It lacks APIs for anything that
involves heap memory allocations and I/O.

For an application, `std` also takes care of (among other things) setting up
stack overflow protection, processing command line arguments, and spawning
the main thread before a program's `main` function is invoked.

A `#![no_std]` application lacks all that standard runtime, so it must
initialize its own runtime, if any is required.

This crate must make minimal assumptions about the system is will run on, so
we use the `#![no_std]` crate level attribute to indicate that the crate will
link to the `core` crate instead of the `std` crate.

[`std`]: https://doc.rust-lang.org/std/
[`core`]: https://doc.rust-lang.org/core/
[*directly on the metal*]: https://en.wikipedia.org/wiki/Bare_machine

## `"panic-strategy": "abort"`

If your configuration does not unconditionally abort on panic, which most
targets for full operating systems don't (or if your [`custom target`] does not
contain `"panic-strategy": "abort"`), then you must tell Cargo to do so or add
an `eh_personality` function, which requires a nightly compiler.  [`Here is
Rust's documentation about it`], and [`here is some discussion about it`].

For this program, we use `profile.dev.panic = "abort"` and
`profile.release.panic = "abort"`.

[`custom target`]: https://docs.rust-embedded.org/embedonomicon/custom-target.html
[`Here is Rust's documentation about it`]: https://doc.rust-lang.org/unstable-book/language-features/lang-items.html#more-about-the-language-items
[`here is some discussion about it`]: https://old.reddit.com/r/rust/comments/estvau/til_why_the_eh_personality_language_item_is/

## `libc`

[`libc`] provides all of the definitions necessary to easily interoperate
with C code (or "C-like" code) on each of the platforms that Rust supports.
This includes type definitions (e.g. `c_int`), constants (e.g. `EINVAL`) as
well as function headers (e.g. `malloc`).

The default features of the `libc` crate are disabled to be able to use
`libc` in `#![no_std]` crates.

[`libc`]: https://crates.io/crates/libc
