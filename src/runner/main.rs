use std::{fs::OpenOptions, io, io::Write, path::Path, process::Command};

use cargo_metadata::MetadataCommand;
use clap::Parser;

#[path = "../checks.rs"]
mod checks;
mod config;

/// Benchmark running info
#[derive(Debug)]
struct Harness {
    /// Unique ID of the run
    run_id: String,
    /// Name of the current crate
    crate_name: String,
    /// Names of the benches to run
    benches: Vec<String>,
    /// Benchmark profile
    profile: config::Profile,
}

impl Harness {
    fn new(run_id: String, crate_name: String, profile: config::Profile) -> Self {
        Self {
            run_id,
            crate_name,
            benches: Vec::new(),
            profile,
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

    /// Run one benchmark with a build variant, for N iterations.
    fn run_one(
        &self,
        profile: &config::Profile,
        varient_name: &str,
        variant: &config::BuildVariant,
        bench: &str,
        target_dir: &Path,
        allow_dirty: bool,
    ) -> anyhow::Result<()> {
        let dir = target_dir.join("harness").join("logs").join(&self.run_id);
        std::fs::create_dir_all(&dir)?;
        let log_file = dir.join(format!("{}.{}.log", bench, varient_name));
        let outputs = OpenOptions::new()
            .write(true)
            .append(true)
            .create(true)
            .open(log_file)?;
        let errors = outputs.try_clone()?;
        let out = Command::new("cargo")
            .stdout(outputs)
            .stderr(errors)
            .args(["bench", "--bench", bench])
            .arg("--features")
            .arg(variant.features.join(" "))
            .args(if !variant.default_features {
                &["--no-default-features"] as &[&str]
            } else {
                &[] as &[&str]
            })
            .args(["--", "-n"])
            .arg(format!("{}", self.profile.iterations))
            .arg("--overwrite-crate-name")
            .arg(&self.crate_name)
            .arg("--overwrite-benchmark-name")
            .arg(bench)
            .args(if !profile.probes.is_empty() {
                vec!["--probes".to_owned(), profile.probes.join(",")]
            } else {
                vec![]
            })
            .args(if allow_dirty {
                vec!["--allow-dirty"]
            } else {
                vec![]
            })
            .status()?;
        if out.success() {
            Ok(())
        } else {
            Err(anyhow::anyhow!(
                "Failed to run bench `{}` with variant {:?}",
                bench,
                variant
            ))
        }
    }

    /// Run all benchmarks with all build variants.
    /// Benchmarks are invoked one by one.
    fn run(&mut self, target_dir: &Path, allow_dirty: bool) -> anyhow::Result<()> {
        self.collect_benches()?;
        for bench in &self.benches {
            print!("[{}] ", bench);
            io::stdout().flush()?;
            for i in 0..self.profile.invocations {
                print!("{}", i);
                io::stdout().flush()?;
                for (index, (variant_name, variant)) in
                    self.profile.build_variants.iter().enumerate()
                {
                    assert!(index < 26);
                    const KEYS: &str = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ";
                    let key = KEYS.chars().nth(index).unwrap();
                    let result = self.run_one(
                        &self.profile,
                        variant_name,
                        variant,
                        bench,
                        target_dir,
                        allow_dirty,
                    );
                    match result {
                        Ok(_) => {
                            print!("{}", key)
                        }
                        Err(_) => {
                            print!(".")
                        }
                    }
                    io::stdout().flush()?;
                }
            }
            println!();
            io::stdout().flush()?;
        }
        Ok(())
    }
}

#[derive(Parser, Debug)]
pub struct HarnessCmdArgs {
    #[arg(short = 'n', long)]
    /// Number of iterations
    pub iterations: Option<usize>,
    #[arg(short = 'i', long)]
    /// Number of invocations
    pub invocations: Option<usize>,
    #[arg(long, default_value = "default")]
    /// Benchmarking profile
    pub profile: String,
    #[arg(long, default_value = "false")]
    /// Allow dirty working directories
    pub allow_dirty: bool,
}

fn main() -> anyhow::Result<()> {
    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "info")
    }
    let mut args = std::env::args().collect::<Vec<_>>();
    if args.len() > 1 && args[1] == "harness" {
        args = args[1..].to_vec();
    }
    let args = HarnessCmdArgs::parse_from(args);
    let Ok(meta) = MetadataCommand::new().manifest_path("./Cargo.toml").exec() else {
        anyhow::bail!("Failed to get metadata from ./Cargo.toml");
    };
    let target_dir = meta.target_directory.as_std_path();
    let Some(pkg) = meta.root_package() else {
        anyhow::bail!("Could not find root package");
    };
    checks::pre_benchmarking_checks(args.allow_dirty)?;
    let config = config::load_from_cargo_toml()?;
    let Some(mut profile) = config.profiles.get(&args.profile).cloned() else {
        anyhow::bail!("Could not find harness profile `{}`", args.profile);
    };
    // Overwrite invocations and iterations
    if let Some(invocations) = args.invocations {
        profile.invocations = invocations;
    }
    if let Some(iterations) = args.iterations {
        profile.iterations = iterations;
    }
    let time = chrono::Local::now().format("%Y%m%d-%H%M%S").to_string();
    let run_id = format!("{}-{}", args.profile, time);
    let mut harness = Harness::new(run_id, pkg.name.clone(), profile);
    harness.run(target_dir, args.allow_dirty)?;
    Ok(())
}
