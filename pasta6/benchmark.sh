#!/bin/sh

cargo build --release
cargo run --release &
server_pid="$!"
sleep 0.1

# Run a benchmark for 30 seconds, using 12 threads, and keeping 400
# HTTP connections open.
ADDR=127.0.0.1:9090
#ADDR=213.188.207.104
#ADDR=pasta6.fly.dev
#wrk -t12 -c400 -d10s -sbenchmark_post.lua http://${ADDR}/todo
wrk -t12 -c400 -d10s http://${ADDR}
kill "${server_pid}"
