use std::convert::TryFrom;
use std::fmt;
use std::{
    cell::RefCell,
    path::PathBuf,
    sync::Mutex,
    time::{Duration, Instant},
};

use clap::Parser;

use crate::{
    probe::ProbeManager,
    record::{Record, StatPrintFormat},
};

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

#[derive(Debug, Clone, Copy)]
pub enum Value {
    F64(f64),
    F32(f32),
    Usize(usize),
    Isize(isize),
    U64(u64),
    I64(i64),
    U32(u32),
    I32(i32),
    U16(u16),
    I16(i16),
    U8(u8),
    I8(i8),
    Bool(bool),
}

impl Value {
    pub(crate) fn into_string(self) -> String {
        match self {
            Value::F64(v) => v.to_string(),
            Value::F32(v) => v.to_string(),
            Value::Usize(v) => v.to_string(),
            Value::Isize(v) => v.to_string(),
            Value::U64(v) => v.to_string(),
            Value::I64(v) => v.to_string(),
            Value::U32(v) => v.to_string(),
            Value::I32(v) => v.to_string(),
            Value::U16(v) => v.to_string(),
            Value::I16(v) => v.to_string(),
            Value::U8(v) => v.to_string(),
            Value::I8(v) => v.to_string(),
            Value::Bool(v) => v.to_string(),
        }
    }
}

macro_rules! impl_helper_traits {
    ($variant: ident, $t:ty) => {
        impl From<$t> for Value {
            fn from(v: $t) -> Self {
                Value::$variant(v)
            }
        }

        impl TryFrom<Value> for $t {
            type Error = ();

            fn try_from(v: Value) -> Result<Self, Self::Error> {
                match v {
                    Value::$variant(v) => Ok(v),
                    _ => Err(()),
                }
            }
        }
    };
}

impl_helper_traits!(F64, f64);
impl_helper_traits!(F32, f32);
impl_helper_traits!(Usize, usize);
impl_helper_traits!(Isize, isize);
impl_helper_traits!(U64, u64);
impl_helper_traits!(I64, i64);
impl_helper_traits!(U32, u32);
impl_helper_traits!(I32, i32);
impl_helper_traits!(U16, u16);
impl_helper_traits!(I16, i16);
impl_helper_traits!(U8, u8);
impl_helper_traits!(I8, i8);
impl_helper_traits!(Bool, bool);

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.into_string())
    }
}

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
        self.bencher.timing_end(elapsed);
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
    extra_stats: Mutex<Vec<(String, Value)>>,
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
    pub fn add_stat(&self, name: impl AsRef<str>, value: impl Into<Value>) {
        self.extra_stats
            .lock()
            .unwrap()
            .push((name.as_ref().to_owned(), value.into()));
    }

    /// Returns the wall-clock time of the last timing phase.
    /// Returns `None` if the timing phase has not finished yet.
    pub fn get_walltime(&self) -> Option<Duration> {
        *self.elapsed.lock().unwrap()
    }

    /// Returns the value of a counter.
    pub fn get_raw_counter_value(&self, name: impl AsRef<str>) -> Option<Value> {
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

    fn dump_counters(&self, iteration: usize, is_timing_iteration: bool) {
        let probe_stats = self
            .bencher
            .probes
            .borrow()
            .get_counter_values(std::mem::take(
                &mut *self.bencher.extra_stats.lock().unwrap(),
            ));
        let record = Record {
            name: &self.bench_name,
            csv: self.args.output_csv.as_ref(),
            invocation: self.args.current_invocation,
            build: self.args.current_build.as_ref(),
            format: StatPrintFormat::Yaml,
            iteration,
            is_timing_iteration,
            stats: probe_stats,
        };
        record.dump_values();
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
            self.dump_counters(i, is_timing_iteration);
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
        // Destroy probes
        self.bencher.probes.borrow_mut().deinit();
        Ok(())
    }
}
