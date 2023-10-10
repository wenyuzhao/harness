mod bencher;
mod benchmark;

pub use benchmark::Benchmark;
pub use harness_macros::bench;
pub use std::hint::black_box;

#[doc(hidden)]
pub fn run(file_name: &str, benchmark: impl Benchmark) {
    let mut bencher = bencher::Bencher::new(file_name, benchmark);
    bencher.run();
}
