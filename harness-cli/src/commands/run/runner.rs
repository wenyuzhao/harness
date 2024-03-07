use std::{
    collections::HashMap,
    fs::OpenOptions,
    io::{self, Write},
    path::{Path, PathBuf},
    process::Command,
};

use cargo_metadata::MetadataCommand;
use colored::Colorize;

use crate::{config, meta::RunInfo, print_md};

/// Benchmark running info
#[derive(Debug)]
pub struct BenchRunner<'a> {
    /// Names of the benches to run
    benches: Vec<String>,
    /// Benchmark profile
    run: &'a RunInfo,
    logdir: Option<PathBuf>,
}

impl<'a> BenchRunner<'a> {
    pub fn new(run: &'a RunInfo) -> Self {
        Self {
            benches: Vec::new(),
            run,
            logdir: None,
        }
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
        for target in &pkg.targets {
            if target.is_bench() {
                self.benches.push(target.name.clone());
            }
        }
        Ok(())
    }

    /// Dump invocation-related metadata to the corresponding log file at the start of each invocation
    /// This include: env variables, command line args, cargo features, and git commit
    fn dump_metadata_for_single_invocation(
        &self,
        f: &mut impl Write,
        cmd: &Command,
        build: &config::BuildConfig,
        envs: &HashMap<String, String>,
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
        for (k, v) in envs.iter() {
            writeln!(f, "  {}: {}", k, v)?;
        }
        // cargo features
        writeln!(f, "features: {}", build.features.join(","))?;
        // git commit
        writeln!(f, "commit: {}", RunInfo::get_git_hash())?;
        writeln!(f, "---")?;
        Ok(())
    }

    /// Run one benchmark with one build, for N iterations.
    fn run_one(
        &self,
        profile: &config::Profile,
        build_name: &str,
        build: &config::BuildConfig,
        bench: &str,
        log_dir: &Path,
        invocation: usize,
    ) -> anyhow::Result<()> {
        std::fs::create_dir_all(log_dir)?;
        let log_file = log_dir.join(format!("{}.{}.log", bench, build_name));
        // Checkout branch
        if let Some(commit) = &build.commit {
            let out = Command::new("git")
                .args(["checkout", commit])
                .current_dir(&self.run.crate_info.target_dir)
                .output()?;
            if !out.status.success() {
                return Err(anyhow::anyhow!(
                    "Failed to checkout commit `{}` for build `{}`",
                    commit,
                    build_name
                ));
            }
        }
        let outputs = OpenOptions::new()
            .write(true)
            .append(true)
            .create(true)
            .open(&log_file)?;
        let errors = outputs.try_clone()?;
        let mut outputs2 = outputs.try_clone()?;
        let mut cmd = Command::new("cargo");
        cmd.stdout(outputs)
            .stderr(errors)
            .args(["bench", "--bench", bench])
            .arg("--features")
            .arg(build.features.join(" "))
            .args(if !build.default_features {
                &["--no-default-features"] as &[&str]
            } else {
                &[] as &[&str]
            })
            .args(["--", "-n"])
            .arg(format!("{}", self.run.profile.iterations))
            .arg("--overwrite-crate-name")
            .arg(&self.run.crate_info.name)
            .arg("--overwrite-benchmark-name")
            .arg(bench)
            .arg("--current-invocation")
            .arg(format!("{invocation}"))
            .arg("--output-csv")
            .arg(log_dir.join("results.csv"))
            .arg("--current-build")
            .arg(build_name);
        if !profile.probes.is_empty() {
            cmd.args(["--probes".to_owned(), profile.probes.join(",")]);
        }
        let mut envs = profile.env.clone();
        for (k, v) in &build.env {
            envs.insert(k.clone(), v.clone());
        }
        cmd.envs(&envs);
        self.dump_metadata_for_single_invocation(&mut outputs2, &cmd, build, &envs)?;
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
            self.logdir.as_ref().unwrap().to_str().unwrap()
        );
        print_md!("* benchmarks: `{}`", self.benches.len());
        print_md!("* builds: `{}`", self.run.profile.builds.len());
        print_md!("* invocations: `{}`", self.run.profile.invocations);
        print_md!("* iterations: `{}`", self.run.profile.iterations);
        println!("");
        println!("{}\n", "Running Benchmarks...".blue());
    }

    fn print_after_run(&self) {
        println!("\n{}\n", "✔ Benchmarking Finished.".green());
        let csv_path = self.logdir.as_ref().unwrap().join("results.csv");
        print_md!("Raw benchmark results at:\n");
        print_md!("* `{}`\n\n", csv_path.display());
        print_md!("Please run `cargo harness report` to view results.\n");
    }

    fn max_bench_name_len(&self) -> usize {
        self.benches.iter().map(|b| b.len()).max().unwrap()
    }

    /// Run all benchmarks with all builds.
    /// Benchmarks are invoked one by one.
    pub fn run(&mut self, log_dir: &Path) -> anyhow::Result<()> {
        self.logdir = Some(log_dir.to_owned());
        self.collect_benches()?;
        self.print_before_run();
        let name_len = self.max_bench_name_len() + 3;
        for bench in &self.benches {
            print!("{}", bench.blue().bold());
            (0..name_len - bench.len()).for_each(|_| print!(" "));
            io::stdout().flush()?;
            for i in 0..self.run.profile.invocations {
                print!("{}", format!("{}", i).bold().blue().italic());
                io::stdout().flush()?;
                for (index, (build_name, build)) in self.run.profile.builds.iter().enumerate() {
                    const KEYS: &str = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ";
                    assert!(index < KEYS.len(), "Too many builds!");
                    let key = KEYS.chars().nth(index).unwrap().to_string();
                    let result =
                        self.run_one(&self.run.profile, build_name, build, bench, log_dir, i);
                    match result {
                        Ok(_) => {
                            print!("{}", key.green())
                        }
                        Err(_) => {
                            print!("{}", "✘".red())
                        }
                    }
                    io::stdout().flush()?;
                }
            }
            println!();
            io::stdout().flush()?;
        }
        self.print_after_run();
        Ok(())
    }
}
