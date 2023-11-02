use example_crate::DefaultQueue;
use std::hint::black_box;

#[harness::bench]
#[derive(Default)]
struct Push {
    queue: DefaultQueue<usize>,
    input_range: (usize, usize),
}

impl harness::Benchmark for Push {
    /// Prepare the benchmark before each iteration.
    fn prologue(&mut self) {
        self.input_range = black_box((0, 10000000));
    }

    /// Run one benchmark iteration.
    fn iter(&mut self) {
        for i in self.input_range.0..self.input_range.1 {
            self.queue.push_back(i);
        }
    }

    /// Clean up the benchmark after each iteration.
    fn epilogue(&mut self) {
        let queue = std::mem::take(&mut self.queue);
        let sum = queue.into_iter().sum::<usize>();
        let expected = (self.input_range.0 + self.input_range.1 - 1)
            * (self.input_range.1 - self.input_range.0)
            / 2;
        println!("checksum: {}", sum);
        assert_eq!(sum, expected);
    }
}
