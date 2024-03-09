use harness::{bench, black_box, Bencher};

const LEN: usize = 10000000;

#[bench]
fn bar(bencher: &Bencher) {
    // prepare the inputs
    let mut list = black_box((0..LEN).map(|x| x * x).collect::<Vec<_>>());
    // timing
    let result = bencher.time(|| {
        // timing
        list.reverse();
        list.iter().sum::<usize>()
    });
    // check the result
    assert_eq!(result, 1291890006563070912)
}
