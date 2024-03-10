# examples/binary

This example shows how to use `harness` to benchmark any binary executables, not limited to rust binaries.

It compares two zip implementations: `zip` and `7z`, and use them to compress three different types of datasets: `xml`, `jpg` and `wav`.

## One-shot runs

Since we're benchmarking third-party binaries, it's not easy to do warmup iterations before the timing iteration. So we only do oneshot runs, by using the `#[bench(oneshot)]` annotation. For your customized binaries, it's also possible to do some IPC to synchronize with the benchmarked binaries, launch them ahead of time, and tell them to repeat some workloads for multiple iterations.

Note that by doing oneshot runs, the result also covers all the binary startup cost.

## Execute binaries

We're benchmarking third-party binaries. This rust crate is only served as both a wrapper to run the binaries (see `src/lib.rs`), and a benchmark suite (see `benches` folder).

The only difference between the two builds configured in `Cargo.toml` is the `BIN` env variable. The wrapper (`src/lib.rs`) will check this variable to decide which binary to run.

## Additional performance metrics

This is a data compression benchmark. In addition to reporting the total running time, we also added additional custom performance metrics to evaluate and report the data compression ratio. Please refer to the `bencher.add_stat(...)` calls in the benchmark files.
