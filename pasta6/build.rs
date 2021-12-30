// Running `cargo build` generates bindings to `linux/io_uring.h` on the fly.
fn main() {
    use std::env;
    use std::path::PathBuf;

    const INCLUDE: &str = r#"
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
        // Allowlist the given types and variables so that they (and all the
        // types that they transitively refer to) appear in the generated
        // bindings.
        //
        // We use an allowlist to only bring in necessary types and
        // variables.
        .allowlist_type("")
        .allowlist_var("")
        // Finish the builder and generate the bindings.
        .generate()
        .expect("unable to generate bindings")
        // Write the bindings to the `src/sys/sys.rs` file.
        .write_to_file(out_dir.join("sys.rs"))
        .expect("unable to write bindings");
}
