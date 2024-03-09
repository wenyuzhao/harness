# examples/simple

The most simple benchmark configuration. Only two benchmarks (see `benches/` and `Cargo.toml`). No special profile configuration.

It will compare two builds:

* `HEAD`: produced using the latest commit
* `HEAD~1` produced using the second last commit

Using two benchmarks: `foo` and `bar`.