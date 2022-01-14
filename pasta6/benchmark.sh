#!/bin/sh

cargo build --release
cargo build --release --features logging
RUST_LOG=trace,regalloc=off,wasmtime_cranelift=off,cranelift_codegen=off,async_io=off,cranelift_wasm=off,wasi_common=off,polling=off,async_std=off,tracing=off cargo run --release --features logging &
server_pid="$!"
sleep 0.5

# Run a benchmark for 30 seconds, using 12 threads, and keeping 400
# HTTP connections open.
ADDR=127.0.0.1:3000
#ADDR=213.188.207.104
#ADDR=pasta6.fly.dev
#wrk -t12 -c400 -d10s -sbenchmark_post.lua http://${ADDR}/todo
#wrk -t12 -c400 -d10s http://${ADDR}
wrk -t12 -c400 -d10s http://${ADDR}
kill "${server_pid}"
