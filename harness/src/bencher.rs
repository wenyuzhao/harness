use std::{
    cell::RefCell,
    path::PathBuf,
    sync::Mutex,
    time::{Duration, Instant},
};

use clap::{Parser, ValueEnum};

use crate::probe::ProbeManager;

#[derive(Parser, Debug)]
pub struct BenchArgs {
    #[arg(long, default_value = "false")]
    pub bench: bool,
    #[arg(short = 'n', long, default_value = "1")]
    /// Number of iterations to run
    pub iterations: usize,
    /// Enabled probes and their configurations, as a json string.
    #[arg(long, default_value = "{}")]
    pub probes: String,
    #[arg(long)]
    #[doc(hidden)]
    /// Overwrite benchmark name
    pub overwrite_benchmark_name: Option<String>,
    #[arg(long)]
    #[doc(hidden)]
    /// Overwrite crate name
    pub overwrite_crate_name: Option<String>,
    #[arg(long)]
    #[doc(hidden)]
    /// Specify current invocation
    pub current_invocation: Option<usize>,
    #[arg(long)]
    #[doc(hidden)]
    /// Append counter values to csv
    pub output_csv: Option<PathBuf>,
    #[arg(long)]
    #[doc(hidden)]
    /// Specify current build name
    pub current_build: Option<String>,
}

#[derive(ValueEnum, Debug, Clone, Copy, PartialEq, Eq)]
#[clap(rename_all = "kebab_case")]
pub(crate) enum StatPrintFormat {
    Table,
    Yaml,
}

pub trait Value: ToString + 'static {}

impl Value for f64 {}
impl Value for f32 {}
impl Value for usize {}
impl Value for isize {}
impl Value for u128 {}
impl Value for i128 {}
impl Value for u64 {}
impl Value for i64 {}
impl Value for u32 {}
impl Value for i32 {}
impl Value for u16 {}
impl Value for i16 {}
impl Value for u8 {}
impl Value for i8 {}
impl Value for bool {}

pub struct BenchTimer<'a> {
    start_time: std::time::Instant,
    bencher: &'a Bencher,
}

impl<'a> Drop for BenchTimer<'a> {
    fn drop(&mut self) {
        {
            let mut state = self.bencher.state.lock().unwrap();
            assert_eq!(*state, BencherState::Timing);
            *state = BencherState::AfterTiming;
        }
        let elapsed = self.start_time.elapsed();
        self.bencher.timing_end(elapsed.clone());
        let mut lock = self.bencher.elapsed.lock().unwrap();
        assert!(lock.is_none(), "More than one benchmark timer detected");
        *lock = Some(elapsed);
    }
}

#[derive(Debug, PartialEq, Eq)]
enum BencherState {
    BeforeTiming,
    Timing,
    AfterTiming,
}

/// A handle to the benchmark runner
pub struct Bencher {
    bench: String,
    current_iteration: usize,
    max_iterations: usize,
    elapsed: Mutex<Option<Duration>>,
    probes: RefCell<ProbeManager>,
    extra_stats: Mutex<Vec<(String, Box<dyn Value>)>>,
    state: Mutex<BencherState>,
}

impl Bencher {
    fn new(bench: String, max_iterations: usize) -> Self {
        Self {
            bench,
            current_iteration: 0,
            max_iterations,
            elapsed: Mutex::new(None),
            probes: RefCell::new(ProbeManager::new()),
            extra_stats: Mutex::new(Vec::new()),
            state: Mutex::new(BencherState::BeforeTiming),
        }
    }

    fn iter_start(&mut self, iteration: usize) {
        self.current_iteration = iteration;
        self.extra_stats.lock().unwrap().clear();
        *self.state.lock().unwrap() = BencherState::BeforeTiming;
        // Erase scratch directory
        let scratch_dir = &*crate::utils::HARNESS_BENCH_SCRATCH_DIR;
        if scratch_dir.exists() {
            std::fs::remove_dir_all(scratch_dir).unwrap();
        }
        std::fs::create_dir_all(scratch_dir).unwrap();
    }

    fn iter_end(&mut self) {
        assert_eq!(*self.state.lock().unwrap(), BencherState::AfterTiming);
    }

    fn timing_begin(&self) {
        let mut probes = self.probes.borrow_mut();
        probes.begin(
            &self.bench,
            self.current_iteration,
            !self.is_timing_iteration(),
        )
    }

    fn timing_end(&self, walltime: Duration) {
        let mut probes = self.probes.borrow_mut();
        probes.end(
            &self.bench,
            self.current_iteration,
            !self.is_timing_iteration(),
            walltime,
        )
    }

    /// Returns true if this is the last iteration
    pub fn is_timing_iteration(&self) -> bool {
        self.current_iteration == self.max_iterations - 1
    }

    /// Indicates the start of the timing phase. Should not be called more than once, or used the same time as `time`.
    ///
    /// Returns a `BenchTimer` object that will automatically stop the timer when it goes out of scope.
    ///
    /// # Example
    ///
    /// ```rust
    /// use harness::{bench, Bencher, black_box};
    ///
    /// const LEN: usize = 10000000;
    ///
    /// #[bench]
    /// fn example(bencher: &Bencher) {
    ///     // Prepare the inputs
    ///     let mut list = black_box((0..LEN).collect::<Vec<_>>());
    ///     // Actual work. For the last timing iteration only this part will be measured.
    ///     let result = {
    ///         let _timer = bencher.start_timing();
    ///         // Do some work here
    ///         list.iter().sum::<usize>()
    ///     };
    ///     // Release the resources and check the result
    ///     assert_eq!(result, LEN * (LEN - 1) / 2)
    /// }
    /// ```
    pub fn start_timing(&self) -> BenchTimer {
        {
            let mut state = self.state.lock().unwrap();
            if *state != BencherState::BeforeTiming {
                panic!("More than one benchmark timing phase detected");
            }
            assert_eq!(*state, BencherState::BeforeTiming);
            *state = BencherState::Timing;
        }
        self.timing_begin();
        BenchTimer {
            start_time: Instant::now(),
            bencher: self,
        }
    }

    /// Marks the whole timing phase. Should not be called more than once, or used the same time as `start_timing`.
    ///
    /// Returns the result of the closure.
    ///
    /// # Example
    ///
    /// ```rust
    /// use harness::{bench, Bencher, black_box};
    ///
    /// const LEN: usize = 10000000;
    ///
    /// #[bench]
    /// fn example(bencher: &Bencher) {
    ///     // Prepare the inputs
    ///     let mut list = black_box((0..LEN).collect::<Vec<_>>());
    ///     // Actual work. For the last timing iteration only this part will be measured.
    ///     let result = bencher.time(|| {
    ///         // Do some work here
    ///         list.iter().sum::<usize>()
    ///     });
    ///     // Release the resources and check the result
    ///     assert_eq!(result, LEN * (LEN - 1) / 2)
    /// }
    pub fn time<R, F: FnOnce() -> R>(&self, f: F) -> R {
        let _timer = self.start_timing();
        f()
    }

    /// Adds a custom statistic to the benchmark results
    ///
    /// Please ensure you are collecting the statistics in a cheap way during the timing phase,
    /// and call this function only after the timing phase.
    pub fn add_stat(&self, name: impl AsRef<str>, value: impl Value) {
        self.extra_stats
            .lock()
            .unwrap()
            .push((name.as_ref().to_owned(), Box::new(value)));
    }

    /// Returns the wall-clock time of the last timing phase.
    /// Returns `None` if the timing phase has not finished yet.
    pub fn get_walltime(&self) -> Option<Duration> {
        self.elapsed.lock().unwrap().clone()
    }

    /// Returns the value of a counter as a floating point number.
    pub fn get_raw_counter_value(&self, name: impl AsRef<str>) -> Option<f32> {
        self.probes.borrow().get_value(name.as_ref())
    }
}

pub struct SingleBenchmarkRunner {
    args: BenchArgs,
    bench_name: String,
    crate_name: String,
    bencher: Bencher,
    benchmark: fn(&Bencher),
    is_single_shot: bool,
}

impl SingleBenchmarkRunner {
    #[doc(hidden)]
    pub fn new(fname: &str, benchmark: fn(&Bencher), is_single_shot: bool) -> Self {
        let args = BenchArgs::parse();
        let fname = std::path::PathBuf::from(fname);
        let name = fname.file_stem().unwrap().to_str().unwrap().to_owned();
        let bench_name = if let Some(n) = args.overwrite_benchmark_name.as_ref() {
            n.clone()
        } else {
            name
        };
        let crate_name = if let Some(n) = args.overwrite_crate_name.as_ref() {
            n.clone()
        } else {
            "harness".to_owned()
        };
        Self {
            args: BenchArgs::parse(),
            bench_name: bench_name.clone(),
            crate_name,
            bencher: Bencher::new(bench_name, if is_single_shot { 1 } else { args.iterations }),
            benchmark,
            is_single_shot,
        }
    }

    fn run_once_impl(&mut self, iteration: usize) -> f32 {
        self.bencher.iter_start(iteration);
        (self.benchmark)(&self.bencher);
        self.bencher.iter_end();
        // Return execution time
        let elapsed = self.bencher.elapsed.lock().unwrap().take();
        assert!(elapsed.is_some(), "No benchmark timer detected");
        let elapsed = elapsed.unwrap();
        elapsed.as_micros() as f32 / 1000.0
    }

    fn run_iterative(&mut self, iterations: usize) {
        for i in 0..iterations {
            let is_timing_iteration = i == iterations - 1;
            let (start_label, end_label) = if !is_timing_iteration {
                (
                    format!("warmup {} ", i + 1),
                    format!("completed warmup {}", i + 1),
                )
            } else {
                ("".to_owned(), "PASSED".to_owned())
            };
            eprintln!(
                "===== {} {} starting {}=====",
                self.crate_name, self.bench_name, start_label
            );
            let elapsed = self.run_once_impl(i);
            eprintln!(
                "===== {} {} {} in {:.1} msec =====",
                self.crate_name, self.bench_name, end_label, elapsed
            );
        }
    }

    #[doc(hidden)]
    pub fn run(&mut self) -> anyhow::Result<()> {
        // Initialize probes
        self.bencher.probes.borrow_mut().init(&self.args.probes);
        // Run the benchmark
        let iterations = if self.is_single_shot {
            eprintln!("Harness: Single-shot run.");
            1
        } else {
            self.args.iterations
        };
        self.run_iterative(iterations);
        // Dump counters
        self.bencher.probes.borrow().dump_counters(
            &self.bench_name,
            self.args.output_csv.as_ref(),
            self.args.current_invocation,
            self.args.current_build.as_ref(),
            std::mem::take(&mut *self.bencher.extra_stats.lock().unwrap()),
            StatPrintFormat::Yaml,
        );
        // Destroy probes
        self.bencher.probes.borrow_mut().deinit();
        Ok(())
    }
}
