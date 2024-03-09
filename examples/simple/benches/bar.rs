use harness::{bench, black_box, Bencher};

const LEN: usize = 10000000;

#[bench]
fn bar(bencher: &Bencher) {
    // prepare the inputs
    let list = black_box((0..LEN).collect::<Vec<_>>());
    // timing
    let result = bencher.time(|| {
        // timing
        list.iter().sum::<usize>()
    });
    // check the result
    assert_eq!(result, LEN * (LEN - 1) / 2)
}
