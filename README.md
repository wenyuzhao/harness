# cargo-harness

**_<ins>Precise</ins>_** and **_<ins>reproducible</ins>_** benchmarking.

* [Getting Started](#getting-started)
* [**Precise** Measurement](#precise-measurement)
  * [Interleaved runs](#interleaved-runs)
  * [Warmup / timing phase separation](#warmup--timing-phase-separation)
  * Statistical runs and analysis
  * [Probes](#probes)
* **Reproducible** Evaluation
  * Git-tracked configs
  * Tracked machine environments
  * Automatic reprodicibility checks

# Getting Started

1. Install the harness CLI: `cargo install harness-cli`.
2. Get the example crate: `git clone https://github.com/wenyuzhao/cargo-harness.git && cd cargo-harness/examples/sort`.
3. Start benchmarking: `cargo harness run`.
4. View results: `cargo harness report`.

# Precise Measurement

## Interleaved runs

For a evaluation, given benchmark programs $P_1..P_p$, builds $B_1..B_b$, and we run each $(P, B)$ pair for $I$ invocations, `cargo-harness` will use the following run order, row by row:

$$I_1\ :\ [P_1B_1,\ P_1B_2,\ ..,\ P_1B_b],\ \ \ [P_2B_1,\ P_2B_2,\ ..,\ P_2B_b]\ \ \ ...\ \ \ [P_pB_1,\ P_pB_2,\ ..,\ P_pB_b]$$

$$I_2\ :\ [P_1B_1,\ P_1B_2,\ ..,\ P_1B_b],\ \ \ [P_2B_1,\ P_2B_2,\ ..,\ P_2B_b]\ \ \ ...\ \ \ [P_pB_1,\ P_pB_2,\ ..,\ P_pB_b]$$

$$\dots$$

$$I_I\ :\ [P_1B_1,\ P_1B_2,\ ..,\ P_1B_b],\ \ \ [P_2B_1,\ P_2B_2,\ ..,\ P_2B_b]\ \ \ ...\ \ \ [P_pB_1,\ P_pB_2,\ ..,\ P_pB_b]$$

The meta-level idea is to **avoid running a single $(P,B)$ pair multiple times in a loop** (This is what most of the existing Rust bench tools would do!).

During benchmarking, the machine environment can have fluctuations, e.g. CPU frequency suddenly scaled down, or a background process waking up to do some task. Interleaved runs will make sure those fluctuations do not only affect one build or one benchmark, but all the benchmarks and builds in a relatively fair way.

## Warmup / timing phase separation

Instead of blindly iterating a single benchmark multiple times and report the per-iteration time distribution, `cargo-haress` have a clear notion of _warmup_ and _timing_ iterations. By default, each _invocation_ of $(P,B)$ will repeat the workload for $5$ iterations. The first $4$ iterations are used for warmup. Only the results from the last _timing_ iteration are reported.

Unless explicitly set iterations to $1$, warmup / timing separation can greatly reduce the noise due to the relatively unpredictable warmups.

Instead of blindly iterating a single benchmark multiple times and reporting the per-iteration time distribution, `cargo-harness` has a clear notion of _warmup_ and _timing_ iterations. By default, each invocation of $(P,B)$ will repeat the workload for $5$ iterations. The first $4$ iterations are used for warmup. Only the results from the last _timing_ iteration are reported.

_Warmup_ / _timing_ separation can greatly reduce the noise due to the relatively unpredictable warmup phase. However, you can also choose to do single-iteration runs to cover the boot time and warmup cost.

## Statistical runs and analysis

## Probes

In addition to report the running time, it's possible to optionally enable the following probes to collect extra performance data:

* **harness-perf**: Collect perf-event values for the timing iteration.
* **harness-ebpf (WIP)**: Extra performance data collcted by ebpf programs.

# TODO:

- [x] Runner
- [x] Binary runner
- [x] Result reporting
- [x] Test runnner
- [x] Scratch folder
- [x] Default to compare HAED vs HEAD~1
- [x] Restore git states after benchmarking
- [ ] Handle no result cases
- [ ] More examples
- [ ] Documentation
- [ ] Benchmark subsetting
- [ ] Comments for public api
- [ ] Post invocation: Copy files
- [ ] Post invocation: Rsync results
- [ ] Post invocation: Other customized hooks / plugins