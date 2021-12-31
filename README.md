# pasta6

## Crates

- [`pasta6`] - core library; provides I/O functionality
- [`pasta6-benchmark`]: pasta6 API benchmark client
- [`pasta6-server`]: pasta6 API server implementation

[`pasta6`]: ./pasta6
[`pasta6-benchmark`]: ./pasta6-benchmark
[`pasta6-server`]: ./pasta6-server

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

| Commit ID | Configuration | Average QPS | Latency Average |
| --------- | ------------- | ----------- | --------------- |
| 21a59e54  | A             | 303468.969  | 783.796 us      |
| 21a59e54  | B             | 30059.957   | 1832.496 us     |
| 21a59e54  | C             | 15040.026   | 3164.967 us     |
| 21a59e54  | D             | 30167.947   | 2166.290 us     |
| 21a59e54  | E             | 60173.594   | 1248.999 us     |
