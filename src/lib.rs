mod bencher;
mod benchmark;
mod checks;
pub mod probe;

pub use benchmark::Benchmark;
pub use harness_macros::{bench, probe};
pub use std::hint::black_box;

#[doc(hidden)]
pub fn run(file_name: &str, benchmark: impl Benchmark) {
    let mut bencher = bencher::Bencher::new(file_name, benchmark);
    if let Err(e) = bencher.run() {
        panic!("{}", e.to_string())
    }
}
