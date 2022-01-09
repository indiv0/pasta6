# pasta6

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
