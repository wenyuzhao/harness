use std::ops::Range;

use harness::{bench, black_box, Bencher};
use rand::prelude::*;
use rand::rngs::SmallRng;

const LEN: usize = 10000000;
const RANGE: Range<usize> = 0..10000;

#[bench]
fn bench(bencher: &Bencher) {
    let mut rng = SmallRng::seed_from_u64(42);
    let mut list = black_box((0..LEN).map(|_| rng.gen_range(RANGE)).collect::<Vec<_>>());
    bencher.time(|| {
        example_crate::sort(&mut list);
    });
    assert!(example_crate::is_sorted(&list));
    let sum = list.iter().sum::<usize>();
    println!("checksum: {}", sum);
    assert_eq!(sum, 50005793915);
}
