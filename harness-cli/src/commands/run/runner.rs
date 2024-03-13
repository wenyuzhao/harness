use std::{
    fs::OpenOptions,
    io::{self, Write},
    path::{Path, PathBuf},
    process::Command,
};

use cargo_metadata::MetadataCommand;
use colored::Colorize;

use crate::{
    configs::{harness::BuildConfig, run_info::RunInfo},
    print_md,
    utils::{
        self,
        bench_cmd::{get_bench_build_command, get_bench_run_command},
        lockfile::replay_lockfile,
    },
};

/// Benchmark running info
#[derive(Debug)]
pub struct BenchRunner<'a> {
    /// Names of the benches to run
    benches: Vec<String>,
    /// Sorted list of all build names
    build_names: Vec<String>,
    /// Benchmark profile
    run: &'a RunInfo,
    log_dir: Option<PathBuf>,
    scratch_dir: PathBuf,
    cache_dir: PathBuf,
}

impl<'a> BenchRunner<'a> {
    const BUILD_LABELS: &'static str = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ";
    pub const MAX_SUPPORTED_BUILDS: usize = Self::BUILD_LABELS.len();

    pub fn new(run: &'a RunInfo) -> Self {
        let mut build_names = run.profile.builds.keys().cloned().collect::<Vec<_>>();
        build_names.sort();
        Self {
            benches: Vec::new(),
            build_names,
            run,
            log_dir: None,
            scratch_dir: run.crate_info.target_dir.join("harness").join("scratch"),
            cache_dir: run.crate_info.target_dir.join("harness").join("cache"),
        }
    }

    fn get_log_file(&self, bench: &str, build: &str) -> PathBuf {
        self.log_dir
            .as_ref()
            .unwrap()
            .join(format!("{}.{}.log", bench, build))
    }

    fn setup_env_before_benchmarking(&self) -> anyhow::Result<()> {
        std::env::set_var("HARNESS_BENCH_CACHE_DIR", self.cache_dir.to_str().unwrap());
        std::env::set_var(
            "HARNESS_BENCH_SCRATCH_DIR",
            self.scratch_dir.to_str().unwrap(),
        );
        if let Some(log_dir) = &self.log_dir {
            std::env::set_var("HARNESS_BENCH_LOG_DIR", log_dir.to_str().unwrap());
        }
        std::env::set_var("HARNESS_BENCH_RUNID", self.run.runid.as_str());
        std::fs::create_dir_all(&self.scratch_dir)?;
        std::fs::create_dir_all(&self.cache_dir)?;
        Ok(())
    }

    fn setup_before_invocation(&self) -> anyhow::Result<()> {
        if self.scratch_dir.exists() {
            std::fs::remove_dir_all(&self.scratch_dir)?;
        }
        std::fs::create_dir_all(&self.scratch_dir)?;
        Ok(())
    }

    /// Collect all available benchmarks
    fn collect_benches(&mut self) -> anyhow::Result<()> {
        let meta = MetadataCommand::new()
            .manifest_path("./Cargo.toml")
            .exec()
            .unwrap();
        let Some(pkg) = meta.root_package() else {
            anyhow::bail!("No root package found");
        };
        for name in &self.run.crate_info.benches {
            let target = pkg.targets.iter().find(|t| &t.name == name && t.is_bench());
            if target.is_none() {
                anyhow::bail!("No bench target found for {}", name);
            }
            self.benches.push(name.clone());
        }
        Ok(())
    }

    /// Dump invocation-related metadata to the corresponding log file at the start of each invocation
    /// This include: env variables, command line args, cargo features, and git commit
    fn dump_metadata_for_single_invocation(
        &self,
        f: &mut impl Write,
        cmd: &Command,
        build: &BuildConfig,
    ) -> anyhow::Result<()> {
        writeln!(f, "---")?;
        // command line args
        let prog = cmd.get_program().to_string_lossy();
        let args = cmd
            .get_args()
            .map(|a| a.to_string_lossy())
            .collect::<Vec<_>>();
        writeln!(f, "command: {} {}", prog.as_ref(), args.join(" "))?;
        // env variable
        writeln!(f, "env:")?;
        let mut envs = self.run.profile.env.clone();
        for (k, v) in &build.env {
            envs.insert(k.clone(), v.clone());
        }
        for (k, v) in &envs {
            writeln!(f, "  {}: {}", k, v)?;
        }
        // cargo features
        writeln!(f, "features: {}", build.features.join(","))?;
        // git commit
        writeln!(
            f,
            "commit: {}",
            git_info2::get()
                .head
                .last_commit_hash
                .unwrap_or_else(|| "unknown".to_owned())
        )?;
        writeln!(f, "---")?;
        Ok(())
    }

    fn test_build(&self) -> anyhow::Result<()> {
        for build_name in &self.build_names {
            let build = &self.run.profile.builds[build_name];
            let commit = build.commit.as_deref().unwrap_or(self.run.commit.as_str());
            let _git_guard = utils::git::checkout(commit)?;
            let _lock_guard = replay_lockfile(&self.run, commit)?;
            let mut cmd = get_bench_build_command(&self.run.profile, build_name);
            let out = cmd
                .output()
                .map_err(|e| anyhow::anyhow!("Failed to build `{}`: {}", build_name, e))?;
            if !out.status.success() {
                eprintln!("{}", String::from_utf8_lossy(&out.stderr));
                anyhow::bail!("Failed to build `{}`", build_name,);
            }
        }
        Ok(())
    }

    /// Run one benchmark with one build, for N iterations.
    pub fn test_run(&self, bench: &str, build_name: &str) -> anyhow::Result<()> {
        print_md!(
            "# Running bench *{}* with build *{}*\n\n",
            bench,
            build_name
        );
        self.setup_env_before_benchmarking()?;
        self.setup_before_invocation()?;
        let mut cmd = get_bench_run_command(&self.run, bench, build_name, 0, None);
        if cmd.status()?.success() {
            Ok(())
        } else {
            Err(anyhow::anyhow!(
                "Test run failed. bench={}, build={}",
                bench,
                build_name
            ))
        }
    }

    /// Run one benchmark with one build, for N iterations.
    fn run_one(
        &self,
        build_name: &str,
        build: &BuildConfig,
        bench: &str,
        log_dir: &Path,
        invocation: usize,
    ) -> anyhow::Result<()> {
        std::fs::create_dir_all(log_dir)?;
        self.setup_before_invocation()?;
        let log_file = self.get_log_file(bench, build_name);
        // Checkout the given commit if it's specified
        let commit = build.commit.as_deref().unwrap_or(self.run.commit.as_str());
        let _git_guard = utils::git::checkout(commit)?;
        let _lock_guard = replay_lockfile(&self.run, commit);
        let outputs = OpenOptions::new()
            .append(true)
            .create(true)
            .open(log_file)?;
        let errors = outputs.try_clone()?;
        let mut outputs2 = outputs.try_clone()?;
        let mut cmd =
            get_bench_run_command(&self.run, bench, build_name, invocation, Some(log_dir));
        cmd.stdout(outputs).stderr(errors);
        self.dump_metadata_for_single_invocation(&mut outputs2, &cmd, build)?;
        let out = cmd.status()?;
        writeln!(outputs2, "\n\n\n")?;
        if out.success() {
            Ok(())
        } else {
            Err(anyhow::anyhow!(
                "Failed to run bench `{}` with build {:?}",
                bench,
                build
            ))
        }
    }

    fn print_before_run(&self) {
        print_md!("# {}\n\n", self.run.runid);
        print_md!(
            "* logs: `{}`",
            self.log_dir.as_ref().unwrap().to_str().unwrap()
        );
        let probe_names = self.run.profile.probes.keys().cloned().collect::<Vec<_>>();
        print_md!("* probes: `{}`", probe_names.join(", "));
        print_md!("* iterations: `{}`", self.run.profile.iterations);
        let i = self.run.profile.invocations;
        let w = (i - 1).to_string().len();
        print_md!(
            "* invocations: `{}` {} {}{}{}",
            self.run.profile.invocations,
            "---".bright_black(),
            format!("#{}", "0".repeat(w)).bold().on_cyan(),
            " ~ ".bold().cyan(),
            format!("#{}", i - 1).to_string().bold().on_cyan()
        );
        // dump plain output
        print_md!(
            "* benchmarks: {}",
            self.benches
                .iter()
                .enumerate()
                .map(|(i, v)| format!(
                    "{}{}{}",
                    i.to_string().italic().bold().blue(),
                    "-".bright_black().italic(),
                    v.to_owned().italic().blue()
                ))
                .collect::<Vec<_>>()
                .join(", ")
        );
        print_md!(
            "* builds: {}",
            self.build_names
                .iter()
                .enumerate()
                .map(|(i, v)| format!(
                    "{}{}{}",
                    self.get_build_label(i).green(),
                    "-".bright_black(),
                    v.to_owned().green().italic()
                ))
                .collect::<Vec<_>>()
                .join(", ")
        );
        println!();
        println!("{}\n", "Running Benchmarks...".blue());
    }

    fn print_after_run(&self) {
        println!("\n{}\n", "✔ Benchmarking Finished.".green());
        let csv_path = self.log_dir.as_ref().unwrap().join("results.csv");
        print_md!("Raw benchmark results at:\n");
        print_md!("* `{}`\n\n", csv_path.display());
        print_md!("Please run `cargo harness report` to view results.\n");
    }

    fn get_inv_label(&self, index: usize, is_row_label: bool) -> String {
        let max = self.run.profile.invocations - 1;
        let max_w = max.to_string().len();
        let w = index.to_string().len();
        let label = if is_row_label {
            format!(" #{}{} ", "0".repeat(max_w - w), index)
        } else {
            format!("#{}{}", "0".repeat(max_w - w), index)
        };
        label.on_cyan().bold().to_string()
    }

    fn print_invoc_label(&self, i: usize, is_row_label: bool) {
        let label = self.get_inv_label(i, is_row_label);
        if is_row_label {
            print!("{} ", label);
        } else {
            print!("{}", label);
        }
        io::stdout().flush().unwrap();
    }

    fn get_bench_label(&self, index: usize, is_row_label: bool) -> String {
        if is_row_label {
            let max_w = self.benches.iter().map(|s| s.len()).max().unwrap();
            let w = self.benches[index].len();
            format!(
                "{}{} ",
                self.benches[index].bold().blue().italic(),
                " ".repeat(max_w - w)
            )
        } else {
            let max_w = (self.benches.len() - 1).to_string().len();
            let w = index.to_string().len();
            format!("{}{}", "0".repeat(max_w - w), index)
                .bold()
                .blue()
                .italic()
                .to_string()
        }
    }

    fn print_bench_label(&self, b: usize, is_row_label: bool) {
        let label = self.get_bench_label(b, is_row_label);
        print!("{}", label);
        io::stdout().flush().unwrap();
    }

    fn get_build_label(&self, index: usize) -> String {
        const KEYS: &str = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ";
        assert!(index < KEYS.len(), "Too many builds!");
        KEYS.chars().nth(index).unwrap().to_string()
    }

    fn print_build_label(&self, b: usize) {
        print!("{}", self.get_build_label(b).green());
        io::stdout().flush().unwrap();
    }

    fn run_inv_bench_build(&mut self, log_dir: &Path) -> anyhow::Result<()> {
        for i in 0..self.run.profile.invocations {
            // Start of an invocation
            self.print_invoc_label(i, true);
            for (bench_index, bench) in self.benches.iter().enumerate() {
                // Start of a benchmark
                self.print_bench_label(bench_index, false);
                // Run the benchmark for each build
                for (build_index, build_name) in self.build_names.iter().enumerate() {
                    // Start of a build
                    let build = &self.run.profile.builds[build_name];
                    match self.run_one(build_name, build, bench, log_dir, i) {
                        Ok(_) => self.print_build_label(build_index),
                        Err(e) => self.report_error_and_print_cross(bench, build_name, e)?,
                    }
                }
            }
            println!();
            io::stdout().flush()?;
        }
        Ok(())
    }

    fn run_bench_inv_build(&mut self, log_dir: &Path) -> anyhow::Result<()> {
        for (bench_index, bench) in self.benches.iter().enumerate() {
            self.print_bench_label(bench_index, true);
            for i in 0..self.run.profile.invocations {
                self.print_invoc_label(i, false);
                for (build_index, build_name) in self.build_names.iter().enumerate() {
                    // Start of a build
                    let build = &self.run.profile.builds[build_name];
                    match self.run_one(build_name, build, bench, log_dir, i) {
                        Ok(_) => self.print_build_label(build_index),
                        Err(e) => self.report_error_and_print_cross(bench, build_name, e)?,
                    }
                }
            }
            println!();
            io::stdout().flush()?;
        }
        Ok(())
    }

    fn run_bench_build_inv(&mut self, log_dir: &Path) -> anyhow::Result<()> {
        for (bench_index, bench) in self.benches.iter().enumerate() {
            self.print_bench_label(bench_index, true);
            for (build_index, build_name) in self.build_names.iter().enumerate() {
                self.print_build_label(build_index);
                for i in 0..self.run.profile.invocations {
                    let build = &self.run.profile.builds[build_name];
                    match self.run_one(build_name, build, bench, log_dir, i) {
                        Ok(_) => self.print_invoc_label(i, false),
                        Err(e) => self.report_error_and_print_cross(bench, build_name, e)?,
                    }
                }
            }
            println!();
            io::stdout().flush()?;
        }
        Ok(())
    }

    fn report_error_and_print_cross(
        &self,
        bench: &str,
        build: &str,
        e: anyhow::Error,
    ) -> anyhow::Result<()> {
        // Report error
        let log_file = self.get_log_file(bench, build);
        let mut outputs = OpenOptions::new()
            .append(true)
            .create(true)
            .open(log_file)?;
        writeln!(outputs, "\n\n\n")?;
        writeln!(outputs, "❌ ERROR: {}", e)?;
        // Print cross
        print!("{}", "✘".red());
        io::stdout().flush()?;
        Ok(())
    }

    /// Run all benchmarks with all builds.
    /// Benchmarks are invoked one by one.
    pub fn run(&mut self, log_dir: &Path) -> anyhow::Result<()> {
        self.log_dir = Some(log_dir.to_owned());
        self.collect_benches()?;
        self.print_before_run();
        self.setup_env_before_benchmarking()?;
        self.test_build()?;
        if cfg!(feature = "run_order_bench_inv_build") {
            self.run_bench_inv_build(log_dir)?;
        } else if cfg!(feature = "run_order_bench_build_inv") {
            self.run_bench_build_inv(log_dir)?;
        } else {
            self.run_inv_bench_build(log_dir)?;
        }
        self.print_after_run();
        Ok(())
    }
}
