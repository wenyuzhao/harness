use harness::{bench, black_box, Bencher};

const LEN: usize = 10000000;

#[bench]
fn bench(bencher: &Bencher) {
    let mut list = black_box(vec![0; LEN]);
    bencher.time(|| {
        sort::sort(&mut list);
    });
    assert!(sort::is_sorted(&list))
}
