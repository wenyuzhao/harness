# cargo-harness

**_Precise_** and **_reproducible_** benchmarking.

* [Getting Started](#getting-started)
* Precise Measurement
  * Interleaved runs
  * Warmup / timing phase separation
  * Statistical runs and analysis
* Reproducible Evaluation
  * Git-tracked configs
  * Tracked machine environments
  * Automatic reprodicibility checks

# Getting Started

1. Install the harness CLI: `cargo install harness-cli`.
2. Get the example crate: `git clone https://github.com/wenyuzhao/cargo-harness.git && cd cargo-harness/examples/sort`.
3. Start benchmarking: `cargo harness run`.
4. View results: `cargo harness report`.

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