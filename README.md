# pasta6

## Crates

- [`pasta6`] - core library; provides I/O functionality
- [`pasta6-benchmark`]: pasta6 API benchmark client
- [`pasta6-server`]: pasta6 API server implementation

[`pasta6`]: ./pasta6
[`pasta6-benchmark`]: ./pasta6-benchmark
[`pasta6-server`]: ./pasta6-server

## Running

To run the development server, enter the `pasta6-server` directory and run:

```shell
cargo watch -w .. -i database -s "cargo run --release"
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

## Benchmarks

**Configuration A:**

- Server cores: 0, 1, 2
- Client cores: 3, 4, 5
- Connection count per core: 80
- Global connection count: 240
- QPS limit per core: 0
- Global QPS limit: 0

**Configuration B:**

- Server cores: 0, 1, 2
- Client cores: 3, 4, 5
- Connection count per core: 80
- Global connection count: 240
- QPS limit per core: 10000
- Global QPS limit: 30000

**Configuration C:**

- Server cores: 0
- Client cores: 3, 4, 5
- Connection count per core: 80
- Global connection count: 240
- QPS limit per core: 5000
- Global QPS limit: 15000

**Configuration D:**

- Server cores: 0
- Client cores: 3, 4, 5
- Connection count per core: 80
- Global connection count: 240
- QPS limit per core: 10000
- Global QPS limit: 30000

**Configuration E:**

- Server cores: 0
- Client cores: 3, 4, 5
- Connection count per core: 80
- Global connection count: 240
- QPS limit per core: 20000
- Global QPS limit: 60000

| Commit ID | Description       | Configuration | Average QPS | Latency Average |
| --------- | ----------------- | ------------- | ----------- | --------------- |
| 21a59e54  | GET baseline      | A             | 303468.969  | 783.796 us      |
| 21a59e54  | GET baseline      | B             | 30059.957   | 1832.496 us     |
| 21a59e54  | GET baseline      | C             | 15040.026   | 3164.967 us     |
| 21a59e54  | GET baseline      | D             | 30167.947   | 2166.290 us     |
| 21a59e54  | GET baseline      | E             | 60173.594   | 1248.999 us     |
| 501820cf  | w/ perf overhead  | A             | 88464.500   | 2651.111 us     |
| af6e08ac  | POST baseline     | A             | 136.678     | 1474505.000 us  |
