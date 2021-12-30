#[cfg(not(feature = "bindgen"))]
fn main() {}

// Running `cargo build` generates bindings to `linux/io_uring.h` on the fly.
#[cfg(feature = "bindgen")]
fn main() {
    use std::env;
    use std::path::PathBuf;

    const INCLUDE: &str = r#"
#include <sys/syscall.h>
#include <linux/io_uring.h>
    "#;

    let out_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap()).join("src/sys");

    // `bindgen::Builder` is the main entry point to bindgen, and lets you
    // set options for the resulting bindings.
    bindgen::Builder::default()
        // The input header we would like to generate bindings for.
        //
        // Rather than creating an actual header file, we define an ephemeral
        // input header file that gets created when bindgen is executed.
        .header_contents("include-file.h", INCLUDE)
        // Use the given prefix for the raw types instead of
        // `::std::os::raw`. This is necessary in `#![no_std]` programs.
        .ctypes_prefix("libc")
        // Use core instead of libstd in the generated bindings.
        .use_core()
        // Allowlist the given types and variables so that they (and all the
        // types that they transitively refer to) appear in the generated
        // bindings.
        //
        // We use an allowlist to only bring in necessary types and
        // variables.
        .allowlist_type("io_uring_params")
        .allowlist_var("__NR_io_uring_setup")
        // Do not generate layout tests.
        //
        // These tests rely on UB (so they generate compiler warnings), and
        // it is [recommended to disable them].
        //
        // [recommended to disable them]:
        // https://github.com/rust-lang/rust-bindgen/issues/1651#issuecomment-971425905
        .layout_tests(false)
        // Finish the builder and generate the bindings.
        .generate()
        .expect("unable to generate bindings")
        // Write the bindings to the `src/sys/sys.rs` file.
        .write_to_file(out_dir.join("sys.rs"))
        .expect("unable to write bindings");
}
