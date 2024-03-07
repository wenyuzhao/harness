use std::{
    cell::RefCell,
    path::PathBuf,
    sync::Mutex,
    time::{Duration, Instant},
};

use clap::Parser;

use crate::probe::ProbeManager;

#[derive(Parser, Debug)]
pub struct BenchArgs {
    #[arg(long, default_value = "false")]
    pub bench: bool,
    #[arg(short = 'n', long, default_value = "1")]
    /// Number of iterations to run
    pub iterations: usize,
    /// Comma-separated probe names
    #[arg(long, default_value = "")]
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
        let elapsed = self.start_time.elapsed();
        self.bencher.harness_end();
        let mut lock = self.bencher.elapsed.lock().unwrap();
        assert!(lock.is_none(), "More than one benchmark timer detected");
        *lock = Some(elapsed);
    }
}

pub struct Bencher {
    elapsed: Mutex<Option<Duration>>,
    probes: RefCell<ProbeManager>,
    extra_stats: Mutex<Vec<(String, Box<dyn Value>)>>,
}

impl Bencher {
    fn new() -> Self {
        Self {
            elapsed: Mutex::new(None),
            probes: RefCell::new(ProbeManager::new()),
            extra_stats: Mutex::new(Vec::new()),
        }
    }

    fn harness_begin(&self) {
        let mut probes = self.probes.borrow_mut();
        probes.harness_begin();
    }

    fn harness_end(&self) {
        let mut probes = self.probes.borrow_mut();
        probes.harness_end();
    }

    pub fn start_timing(&self) -> BenchTimer {
        self.harness_begin();
        BenchTimer {
            start_time: Instant::now(),
            bencher: self,
        }
    }

    pub fn time(&self, mut f: impl FnMut()) {
        let _timer = self.start_timing();
        f();
    }

    pub fn add_stat(&self, name: impl AsRef<str>, value: impl Value) {
        self.extra_stats
            .lock()
            .unwrap()
            .push((name.as_ref().to_owned(), Box::new(value)));
    }
}

pub struct SingleBenchmarkRunner {
    args: BenchArgs,
    name: String,
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
        let name = if let Some(n) = args.overwrite_benchmark_name.as_ref() {
            n.clone()
        } else {
            name
        };
        let crate_name = if let Some(n) = args.overwrite_crate_name.as_ref() {
            n.clone()
        } else {
            "cargo-harness".to_owned()
        };
        Self {
            args: BenchArgs::parse(),
            name,
            crate_name,
            bencher: Bencher::new(),
            benchmark,
            is_single_shot,
        }
    }

    fn run_once_impl(&mut self) -> f32 {
        (self.benchmark)(&self.bencher);
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
                self.crate_name, self.name, start_label
            );
            let elapsed = self.run_once_impl();
            eprintln!(
                "===== {} {} {} in {:.1} msec =====",
                self.crate_name, self.name, end_label, elapsed
            );
        }
    }

    #[doc(hidden)]
    pub fn run(&mut self) -> anyhow::Result<()> {
        self.bencher.probes.borrow_mut().init(&self.args.probes);
        let iterations = if self.is_single_shot {
            eprintln!("WARNING: Force single-shot run.");
            1
        } else {
            self.args.iterations
        };
        self.run_iterative(iterations);
        self.bencher.probes.borrow().dump_counters(
            &self.name,
            self.args.output_csv.as_ref(),
            self.args.current_invocation,
            self.args.current_build.as_ref(),
            &self.bencher.extra_stats.lock().unwrap(),
        );
        Ok(())
    }
}
