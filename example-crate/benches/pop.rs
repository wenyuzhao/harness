use example_crate::DefaultQueue;
use std::hint::black_box;

#[harness::bench]
#[derive(Default)]
struct Pop {
    input_range: (usize, usize),
    queue: DefaultQueue<usize>,
    sum: usize,
}

impl harness::Benchmark for Pop {
    /// Prepare the benchmark before each iteration.
    fn prologue(&mut self) {
        self.input_range = (0, 10000000);
        let mut queue = DefaultQueue::default();
        for i in self.input_range.0..self.input_range.1 {
            queue.push_back(i);
        }
        self.queue = black_box(queue);
        self.sum = 0;
    }

    /// Run one benchmark iteration.
    fn iter(&mut self) {
        let queue = std::mem::take(&mut self.queue);
        for v in queue.into_iter() {
            self.sum += v;
        }
    }

    /// Clean up the benchmark after each iteration.
    fn epilogue(&mut self) {
        let expected = (self.input_range.0 + self.input_range.1 - 1)
            * (self.input_range.1 - self.input_range.0)
            / 2;
        println!("checksum: {}", self.sum);
        assert_eq!(expected, self.sum);
    }
}
