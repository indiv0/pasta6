// Because `linux/io_uring`'s symbols do not follow Rust's style conventions,
// we suppress warnings with `#![allow(...)]` pragmas.
#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]

use libc::{c_int, c_long, c_uint, syscall};

// Include the generated bindings directly in this module.
include!("sys.rs");

pub unsafe fn _io_uring_setup(entries: c_uint, p: *mut io_uring_params) -> c_int {
    syscall(
        __NR_io_uring_setup as c_long,
        entries as c_long,
        p as c_long,
    ) as _
}
