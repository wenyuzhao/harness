mod bencher;
mod benchmark;
pub mod probe;

pub use bencher::{BenchTimer, Bencher};
pub use benchmark::Benchmark;
pub use harness_macros::{bench, probe};
pub use std::hint::black_box;

#[doc(hidden)]
pub fn run(file_name: &str, bench_fn: fn(&Bencher)) {
    let mut bencher = bencher::SingleBenchmarkRunner::new(file_name, bench_fn);
    if let Err(e) = bencher.run() {
        panic!("{}", e.to_string())
    }
}
