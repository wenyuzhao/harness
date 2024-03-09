use harness::{bench, black_box, Bencher};

const LEN: usize = 10000000;

#[bench]
fn bench(bencher: &Bencher) {
    // prepare the inputs
    let mut list = black_box((0..LEN).collect::<Vec<_>>());
    // timing
    bencher.time(|| {
        sort::sort(&mut list);
    });
    // check the result
    assert!(sort::is_sorted(&list))
}
