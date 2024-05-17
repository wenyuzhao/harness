mod bencher;
pub mod probe;
mod record;
pub mod utils;

pub use bencher::{BenchTimer, Bencher, Value};
pub use harness_macros::{bench, probe};
pub use std::hint::black_box;

#[doc(hidden)]
pub fn run(file_name: &str, bench_fn: fn(&Bencher), single_shot: bool) {
    let mut bencher = bencher::SingleBenchmarkRunner::new(file_name, bench_fn, single_shot);
    if let Err(e) = bencher.run() {
        panic!("{}", e.to_string())
    }
}
