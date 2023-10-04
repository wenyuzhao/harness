// #![feature(test)]
// extern crate test;

// use test::Bencher;

// fn add() {}

#[harness::entry]
fn main(bencher: &mut harness::Bencher) {
    bencher.iter(|| example_crate::fib(40));
}
