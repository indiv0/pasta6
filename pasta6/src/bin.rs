fn main() {
    tracing_subscriber::fmt::init();
    #[cfg(all(not(test), target_arch = "wasm32"))]
    pasta6::run();
}
