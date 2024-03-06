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

pub struct Bencher {
    elapsed: Mutex<Option<Duration>>,
    probes: RefCell<ProbeManager>,
}

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

impl Bencher {
    fn new() -> Self {
        Self {
            elapsed: Mutex::new(None),
            probes: RefCell::new(ProbeManager::new()),
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
}

pub struct SingleBenchmarkRunner {
    name: String,
    bencher: Bencher,
    benchmark: fn(&Bencher),
}

impl SingleBenchmarkRunner {
    #[doc(hidden)]
    pub fn new(fname: &str, benchmark: fn(&Bencher)) -> Self {
        let fname = std::path::PathBuf::from(fname);
        let name = fname.file_stem().unwrap().to_str().unwrap().to_owned();
        Self {
            name,
            bencher: Bencher::new(),
            benchmark,
        }
    }

    fn run_once(&mut self) -> f32 {
        (self.benchmark)(&self.bencher);
        // Return execution time
        let elapsed = self.bencher.elapsed.lock().unwrap().take();
        assert!(elapsed.is_some(), "No benchmark timer detected");
        let elapsed = elapsed.unwrap();
        elapsed.as_micros() as f32 / 1000.0
    }

    #[doc(hidden)]
    pub fn run(&mut self) -> anyhow::Result<()> {
        let args = BenchArgs::parse();
        self.bencher.probes.borrow_mut().init(&args.probes);
        let name = if let Some(n) = args.overwrite_benchmark_name.as_ref() {
            n.clone()
        } else {
            self.name.clone()
        };
        let crate_name = if let Some(n) = args.overwrite_crate_name.as_ref() {
            n.clone()
        } else {
            "cargo-harness".to_owned()
        };
        for i in 0..args.iterations {
            let is_timing_iteration = i == args.iterations - 1;
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
                crate_name, name, start_label
            );
            let elapsed = self.run_once();
            eprintln!(
                "===== {} {} {} in {:.1} msec =====",
                crate_name, name, end_label, elapsed
            );
        }
        self.bencher.probes.borrow().dump_counters(
            &name,
            args.output_csv.as_ref(),
            args.current_invocation,
            args.current_build.as_ref(),
        );
        Ok(())
    }
}
