# cargo-harness

**_<ins>Precise</ins>_** and **_<ins>reproducible</ins>_** benchmarking. Inspired by [running-ng](https://anupli.github.io/running-ng).

* [Getting Started](#getting-started)
* [**_<ins>Precise</ins>_** Measurement](#precise-measurement)
  * [Interleaved runs](#interleaved-runs)
  * [Warmup / timing phase separation](#warmup--timing-phase-separation)
  * [Statistical runs and analysis](#statistical-runs-and-analysis)
  * [Probes](#probes)
  * [System checks](#system-checks)
* [**_<ins>Reproducible</ins>_** Evaluation](#reproducible-evaluation)
  * [Git-tracked evaluation configs](#git-tracked-evaluation-configs)
  * [Tracked system environments](#tracked-system-environments)

# Getting Started

1. Install the harness CLI: `cargo install harness-cli`.
2. Get the example crate: `git clone https://github.com/wenyuzhao/cargo-harness.git && cd cargo-harness/examples/sort`.
3. Start an evaluation: `cargo harness run`.
4. View results: `cargo harness report`.

Please see more [examples](/examples) on how to configure and use `cargo-harness`. The evaluation configs can be found in _Cargo.toml_ of each example crate.

# _<ins>Precise</ins>_ Measurement

## Interleaved runs

For a evaluation, given benchmark programs $P_1..P_p$, builds $B_1..B_b$, and we run each $(P, B)$ pair for $I$ invocations, `cargo-harness` will use the following run order, row by row:

$$I_1\ :\ [P_1B_1,\ P_1B_2,\ ..,\ P_1B_b],\ \ \ [P_2B_1,\ P_2B_2,\ ..,\ P_2B_b]\ \ \ ...\ \ \ [P_pB_1,\ P_pB_2,\ ..,\ P_pB_b]$$

$$I_2\ :\ [P_1B_1,\ P_1B_2,\ ..,\ P_1B_b],\ \ \ [P_2B_1,\ P_2B_2,\ ..,\ P_2B_b]\ \ \ ...\ \ \ [P_pB_1,\ P_pB_2,\ ..,\ P_pB_b]$$

$$\dots$$

$$I_I\ :\ [P_1B_1,\ P_1B_2,\ ..,\ P_1B_b],\ \ \ [P_2B_1,\ P_2B_2,\ ..,\ P_2B_b]\ \ \ ...\ \ \ [P_pB_1,\ P_pB_2,\ ..,\ P_pB_b]$$

The meta-level idea is to **avoid running a single $(P,B)$ pair multiple times in a loop** (This is what most of the existing Rust bench tools would do!).

During benchmarking, the machine environment can have fluctuations, e.g. CPU frequency suddenly scaled down, or a background process waking up to do some task. Interleaved runs will make sure those fluctuations do not only affect one build or one benchmark, but all the benchmarks and builds in a relatively fair way.

When running benchmarks in a complex environment, rather than on a dedicated headless server, you are very likely to see a difference in the results between the two run orders.

**Note:** For the same reason, it's recommended to always have more than two different builds in each evaluation. Otherwise, there is no difference to running a single build in a loop.

## Warmup / timing phase separation

Instead of blindly iterating a single benchmark multiple times and reporting the per-iteration time distribution, `cargo-harness` has a clear notion of _warmup_ and _timing_ iterations. By default, each invocation of $(P,B)$ will repeat the workload for $5$ iterations. The first $4$ iterations are used for warmup. Only the results from the last _timing_ iteration are reported.

_Warmup_ / _timing_ separation can greatly reduce the noise due to the relatively unpredictable warmup oor boot phase. However, you can also choose to do single-iteration runs to cover the boot time and warmup cost.

## Statistical runs and analysis

Similar to other benchmarking tools, `cargo-harness` runs each $(P,B)$ pair multiple times (multiple invocations). However, we use a fixed number of invocations for all $(P,B)$ pairs for easier reasoning purposes. Unless specified differently, each $(P,B)$ is run for 10 invocations by default.

After all the $I$ invocations are finished, running `cargo harness report` will parse the results and report the min/max/mean/geomean for each performance value, as well as the 95% confidence interval per benchmark.

You can also use your own script to load the results and analyze them differently. The performance values are stored in `target/harness/logs/<RUNID>/results.csv`.

## Probes

In addition to report the running time, it's possible to optionally enable the following probes to collect extra performance data:

* **harness-perf**: Collect perf-event values for the timing iteration.
* **harness-ebpf (WIP)**: Extra performance data collcted by ebpf programs.

## System checks

`cargo-harness` performs a series of strict checks to minimize benchmarking noise and ensure correctness. It refuses to start benchmarking if any of the following checks fail:

* There are no uncommitted changes in the repo (mainly for correctness and reproducibility)
* (*Linux-only*) Only one user is logged in
* (*Linux-only*) All CPU scaling governors are set to `performance`

# _<ins>Reproducible</ins>_ Evaluation

## Git-tracked evaluation configs

Evaluation configs are forced to be tracked by Git alongside your Rust crate. `cargo-harness` enforces that all changes in the current git repo, including the evaluation config itself, must be committed prior to running the benchmark. Otherwise, it will refuse to run.

This ensures that each different evaluation alongside the benchmarked code is being tracked properly, without any accidental changes. Hence, it becomes possible to check the correctness or any details of any historical evaluations, by simply tracking back the git history.

## Tracked system environments

`cargo-harness` assigns each individual evaluation a unique `RUNID` and generates a evaluation summary at `target/harness/logs/<RUNID>/config.toml`. The following environmental or evaluation info is tracked in the summary config file:

* Git commit of the evaluation config
* Git commit, cargo features, and environment variables used for producing each evaluated build
* All global environment variables at the time of the run
* OS / CPU / Memory / Cache information used for the run

Reproducing a previous evaluation is as simple as running `cargo harness run --config <RUNID>`. `cargo-harness` automatically checks out the corresponding commits to ensure the codebase is exactly at the same state as the time `RUNID` was generated.

Any change to the system environments would affect reproducibility. So it's recommended to keep the same environment variables and the same OS / CPU / Memory / Cache config _as much as possible_. `cargo-harness` automatically compares the current system info against the recorded ones and warns the user for any differences.

# TODO:

- [x] Runner
- [x] Binary runner
- [x] Result reporting
- [x] Test runnner
- [x] Scratch folder
- [x] Default to compare HAED vs HEAD~1
- [x] Restore git states after benchmarking
- [x] Comments for public api
- [ ] Handle no result cases
- [ ] More examples
- [ ] Documentation
- [ ] Benchmark subsetting
- [ ] Plugin system
- [ ] Plugin: html or markdown report with graphs
- [ ] Plugin: Copy files
- [ ] Plugin: Rsync results