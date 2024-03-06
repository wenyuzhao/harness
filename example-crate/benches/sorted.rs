use harness::{bench, black_box, Bencher};

const LEN: usize = 10000000;

#[bench]
fn bench(bencher: &Bencher) {
    let mut list = black_box((0..LEN).collect::<Vec<_>>());
    bencher.time(|| {
        example_crate::sort(&mut list);
    });
    assert!(example_crate::is_sorted(&list))
}
