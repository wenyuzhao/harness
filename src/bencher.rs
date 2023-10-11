use clap::Parser;

use crate::{benchmark::Benchmark, probe::ProbeManager};

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
    #[arg(long, default_value = "false")]
    /// Allow dirty working directories
    pub allow_dirty: bool,
}

pub struct Bencher<B> {
    name: String,
    benchmark: B,
    probes: ProbeManager,
}

impl<B: Benchmark> Bencher<B> {
    #[doc(hidden)]
    pub fn new(fname: &str, benchmark: B) -> Self {
        let fname = std::path::PathBuf::from(fname);
        let name = fname.file_stem().unwrap().to_str().unwrap().to_owned();
        Self {
            name,
            benchmark,
            probes: ProbeManager::new(),
        }
    }

    #[doc(hidden)]
    pub fn run(&mut self) -> anyhow::Result<()> {
        let args = BenchArgs::parse();
        crate::checks::pre_benchmarking_checks(args.allow_dirty)?;
        self.probes.init(&args.probes);
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
            self.benchmark.prologue();
            let time = std::time::Instant::now();
            if is_timing_iteration {
                self.probes.harness_begin();
            }
            self.benchmark.iter();
            if is_timing_iteration {
                self.probes.harness_end();
            }
            let elapsed = time.elapsed().as_micros() as f32 / 1000.0;
            self.benchmark.epilogue();
            eprintln!(
                "===== {} {} {} in {:.1} msec =====",
                crate_name, name, end_label, elapsed
            );
        }
        self.probes.dump_counters();
        Ok(())
    }
}
