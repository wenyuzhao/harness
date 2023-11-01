# cargo-harness

**_Precise_ and _reproducible_ micro benchmarking.**

# Getting Started

1. Get the example crate: `git clone https://github.com/wenyuzhao/cargo-harness.git && cd cargo-harness/example-crate`.
2. The crate provides [a default queue type](example-crate/src/lib.rs#L28-32). Let's run a benchmark to find the best option between `LinkedList` and `VecDeque`.
3. Install cargo-harness CLI: `cargo install harness`.
4. Add `harness` to the project dependencies list: `cargo add harness`.
5. Add two micro-benchmarks: [benches/push.rs](example-crate/benches/push.rs) and [benches/pop.rs](example-crate/benches/pop.rs).
6. Register the benchmarks in Cargo.toml: [Cargo.toml#L21-27](example-crate/Cargo.toml#L21-27).
7. Add a default benchmarking configuration: [Cargo.toml#L29-35](example-crate/Cargo.toml#L29-35).
   - `invocations = 10`: Run each benchmark 10 times.
   - `iterations = 5`: In a single invocation, run the benchmarking code for 5 iterations. 1-4 are warmup iterations. Only results from the last iteration are reported.
   - `probes` A list of hooks/plugins for collecting extra data. We add `harness_perf` here to enable performance event counters (see `env.PERF_EVENTS` in the next line). Remember to add `harness-perf` to `[dev-dependencies]`.
   - `harness.profiles.default.build-variants`: Different builds to compare. You must have at least two variants to run the benchmark.
8. Run benchmarks: `cargo harness run`.
9. After the command above is finished, collect results and plot graphs: `cargo harness plot`.