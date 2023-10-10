use std::hint::black_box;

#[harness::entry]
fn main(bencher: &mut harness::Bencher) {
    bencher.iter(|| example_crate::fib(black_box(40)));
}
