# examples/sort

This contains a default profile (see `Cargo.toml`), which evaluates two builds:

* `stable_sort`: default cargo features
* `unstable_sort`: default cargo features with the extra feature `unstable`

Using four benchmarks: `zeros`, `sorted`, `reversed`, `random`,

Each $(Benchmark, Build)$ pair will be executed for $10$ invocations. In each invocation, within the same process, it will run the workload for $5$ iterations and only report the performance results for the last iteration.

It also enables the `harness-perf` probe to collect and report perf-event counter results.