# pasta6

## Install the `lunatic` runtime

```
# Switch to stable toolchain since nightly ICEs when compiling cranelift.
# See: https://github.com/rust-lang/rust/issues/92757
rustup default stable
# Add wasm32 compilation target to be able to compile your lunatic program.
rustup target add wasm32-wasi
# Clone the lunatic repository.
git clone https://github.com/lunatic-solutions/lunatic.git
# Enter the cloned folder.
cd lunatic
# Build and install the Lunatic runtime.
cargo install --path .
```

## Compile your program and run it on the `lunatic` runtime

```
# Compile your program (for the wasm32-wasi target).
cargo build
# Run your program (on the lunatic runtime).
cargo run
```

## Run `wasm32-wasi` tests

```
# Install the `cargo wasi` subcommand.
cargo install cargo-wasi
# Run the tests in `wasm32-wasi`.
cargo wasi test
```

## Profiling

To execute the server for profiling with `cargo-flamegraph`, first enter a
Nix shell with `perf`, then run `cargo-flamegraph`, after enabling the
necessary kernel features:

```shell
nix-shell -p linuxPackages.perf
echo 0 | sudo tee /proc/sys/kernel/kptr_restrict
echo -1 | sudo tee /proc/sys/kernel/perf_event_paranoid
CARGO_PROFILE_RELEASE_DEBUG=true cargo flamegraph -c "record -c 100 -F 99 --call-graph dwarf -g"
```
