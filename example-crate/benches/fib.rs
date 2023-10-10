use std::hint::black_box;

#[harness::bench]
#[derive(Default)]
struct Fib;

impl harness::Benchmark for Fib {
    fn iter(&mut self) {
        let v = black_box(40);
        example_crate::fib(v);
    }
}
