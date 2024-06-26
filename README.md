# Harness

**_<ins>Precise</ins>_** and **_<ins>reproducible</ins>_** benchmarking. Inspired by [running-ng](https://anupli.github.io/running-ng).

* [Getting Started](#getting-started)
* [**_<ins>Precise</ins>_** Measurement](#precise-measurement)
  * [Interleaved runs](#interleaved-runs)
  * [Warmup / timing phase separation](#warmup--timing-phase-separation)
  * [Statistical runs and analysis](#statistical-runs-and-analysis)
  * [Probes](#probes)
  * [System checks](#system-checks)
* [**_<ins>Reproducible</ins>_** Evaluation](#reproducible-evaluation)
  * [Tracked evaluation configs](#tracked-evaluation-configs)
  * [Deterministic builds](#deterministic-builds)
  * [System environment verification](#system-environment-verification)
* [SIGPLAN Empirical Evaluation Checklist](https://github.com/SIGPLAN/empirical-evaluation/raw/master/checklist/checklist.pdf)

[![crates.io](https://img.shields.io/crates/v/harness?style=flat-square&logo=rust)](https://crates.io/crates/harness)
[![docs](https://img.shields.io/docsrs/harness/latest?style=flat-square&logo=docs.rs)](https://docs.rs/harness)
[![workflow-status](https://img.shields.io/github/actions/workflow/status/wenyuzhao/harness/rust.yml?style=flat-square&logo=github&label=checks)](https://github.com/wenyuzhao/harness/actions/workflows/rust.yml)

# Getting Started

1. Install the harness CLI: `cargo install harness-cli`.
2. Get the example crate: `git clone https://github.com/wenyuzhao/harness.git && cd harness/examples/sort`.
3. Start an evaluation: `cargo harness run`.
4. View results: `cargo harness report`.

Please see more [examples](/examples) on how to configure and use `harness`. The evaluation configs can be found in _Cargo.toml_ of each example crate.

# _<ins>Precise</ins>_ Measurement

## Interleaved runs

**`harness` avoids running the same benchmark multiple times in a loop**, unlike what most existing Rust benchmarking tools would do.

For an evaluation, given benchmark programs $P_1..P_p$, builds $B_1..B_b$, and we run each $(P, B)$ pair for $I$ invocations, `harness` will use the following run order, row by row:

$$I_1\ :\ [P_1B_1,\ P_1B_2,\ ..,\ P_1B_b],\ \ \ [P_2B_1,\ P_2B_2,\ ..,\ P_2B_b]\ \ \ ...\ \ \ [P_pB_1,\ P_pB_2,\ ..,\ P_pB_b]$$

$$I_2\ :\ [P_1B_1,\ P_1B_2,\ ..,\ P_1B_b],\ \ \ [P_2B_1,\ P_2B_2,\ ..,\ P_2B_b]\ \ \ ...\ \ \ [P_pB_1,\ P_pB_2,\ ..,\ P_pB_b]$$

$$\dots$$

$$I_I\ :\ [P_1B_1,\ P_1B_2,\ ..,\ P_1B_b],\ \ \ [P_2B_1,\ P_2B_2,\ ..,\ P_2B_b]\ \ \ ...\ \ \ [P_pB_1,\ P_pB_2,\ ..,\ P_pB_b]$$

Any machine can have performance fluctuations, e.g. CPU frequency suddenly scaled down, or a background process waking up to do some task. Interleaved runs will make sure fluctuations do not affect only one build or one benchmark, but all the benchmarks and builds in a relatively fair way.

When running in a complex environment, you are very likely to see a difference in the results between the two run orders.

**Note:** For the same reason, it's recommended to always have more than two different builds in each evaluation. Otherwise, there is no difference to running a single build in a loop.

## Warmup / timing phase separation

**`harness` has a clear notion of _warmup_ and _timing_ iterations**, instead of blindly iterating a single benchmark multiple times and reporting the per-iteration time distribution. By default, each invocation of $(P,B)$ will repeat the workload for $5$ iterations. The first $4$ iterations are used for warmup. Only the results from the last _timing_ iteration are reported. This can greatly reduce the noise due to program warmup and precisely measure the peak performance. However, you can also choose to do single-iteration runs to cover the boot time and warmup cost.

## Statistical runs and analysis

Similar to other bench tools, `harness` runs each $(P,B)$ pair multiple times (multiple invocations). However, we **use a fixed number of invocations for all $(P,B)$ pairs for easier reasoning**. Unless specified differently, each $(P,B)$ is run for 10 invocations by default.

After all the $I$ invocations are finished, running `cargo harness report` will parse the results and report the min/max/mean/geomean for each performance value, as well as the 95% confidence interval per benchmark. You can also use your own script to load the results and analyze them differently. The performance values are stored in `target/harness/logs/<RUNID>/results.csv`.

## Probes

**`harness` supports collecting and reporting extra performance data other than execution time**, by enabling the following probes:

* `harness-probe-perf`: Collect perf-event values for the timing iteration.
* `harness-probe-ebpf (WIP)`: Extra performance data collected by eBPF programs.

## System checks

**`harness` performs a series of strict checks to minimize system noise.** It refuses to start benchmarking if any of the following checks fail:

* (*Linux-only*) Only one user is logged in
* (*Linux-only*) All CPU scaling governors are set to `performance`

# _<ins>Reproducible</ins>_ Evaluation

## Tracked evaluation configs

**`harness` refuses to support casual benchmarking.** Each evaluation is enforced to be properly tracked by Git, including all the benchmark configurations and the revisions of all the benchmarks and benchmarked programs. Verifying the correctness of any evaluation, or re-running an evaluation from years ago, can be done by simply tracking back the git history.

## Deterministic builds

`harness` assigns each individual evaluation a unique `RUNID` and generates an evaluation summary at `target/harness/logs/<RUNID>/config.toml`. `harness` uses this file to record the evaluation info for the current benchmark run, including:

* Git commit of the evaluation config
* Git commit, cargo features, and environment variables used for producing each evaluated build
* The `Cargo.lock` file used for producing each evaluated builds

Reproducing a previous evaluation is as simple as running `cargo harness run --config <RUNID>`. `harness` automatically checks out the corresponding commits, sets up the recorded cargo features or environment variables, and replays the pre-recorded `Cargo.lock` file, to ensure the codebase and builds are exactly at the same state as when `RUNID` was generated.

_Note: `harness` cannot check local dependencies right now. For completely deterministic builds, don't use local dependencies._

## System environment verification

In the same `<RUNID>/config.toml` file, `harness` also records all the environmental info for every benchmark run, including but not limited to:

* All global system environment variables at the time of the run
* OS / CPU / Memory / Swap information used for the run

Any change to the system environments would affect reproducibility. So it's recommended to keep the same environment variables and the same OS / CPU / Memory / Swap config _as much as possible_. `harness` automatically verifies the current system info against the recorded ones and warns for any differences.

# TODO:

- [x] Runner
- [x] Binary runner
- [x] Result reporting
- [x] Test runner
- [x] Scratch folder
- [x] Default to compare HEAD vs HEAD~1
- [x] Restore git states after benchmarking
- [x] Comments for public api
- [x] Documentation
- [x] Benchmark subsetting
- [x] Handle no result cases
- [x] More examples
- [ ] Add tests
- [ ] Plugin system
- [ ] Plugin: html or markdown report with graphs
- [ ] Plugin: Copy files
- [ ] Plugin: Rsync results
- [ ] Performance evaluation guide / tutorial
