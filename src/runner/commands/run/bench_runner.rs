use std::{collections::HashMap, fs::OpenOptions, io, io::Write, path::Path, process::Command};

use cargo_metadata::MetadataCommand;

use crate::{config, platform_info::ProfileWithPlatformInfo};

/// Benchmark running info
#[derive(Debug)]
pub struct BenchRunner {
    /// Name of the current crate
    crate_name: String,
    /// Names of the benches to run
    benches: Vec<String>,
    /// Benchmark profile
    profile: config::Profile,
}

impl BenchRunner {
    pub fn new(crate_name: String, profile: config::Profile) -> Self {
        Self {
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

    /// Dump invocation-related metadata to the corresponding log file at the start of each invocation
    /// This include: env variables, command line args, cargo features, and git commit
    fn dump_metadata_for_single_invocation(
        &self,
        f: &mut impl Write,
        cmd: &Command,
        variant: &config::BuildVariant,
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
        writeln!(f, "features: {}", variant.features.join(","))?;
        // git commit
        writeln!(f, "commit: {}", ProfileWithPlatformInfo::get_git_hash())?;
        writeln!(f, "---")?;
        Ok(())
    }

    /// Run one benchmark with a build variant, for N iterations.
    fn run_one(
        &self,
        profile: &config::Profile,
        varient_name: &str,
        variant: &config::BuildVariant,
        bench: &str,
        log_dir: &Path,
        invocation: usize,
    ) -> anyhow::Result<()> {
        std::fs::create_dir_all(log_dir)?;
        let log_file = log_dir.join(format!("{}.{}.log", bench, varient_name));
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
            .arg("--current-invocation")
            .arg(format!("{invocation}"))
            .arg("--output-csv")
            .arg(log_dir.join("results.csv"))
            .arg("--current-build-variant")
            .arg(varient_name);
        if !profile.probes.is_empty() {
            cmd.args(["--probes".to_owned(), profile.probes.join(",")]);
        }
        let mut envs = profile.env.clone();
        for (k, v) in &variant.env {
            envs.insert(k.clone(), v.clone());
        }
        cmd.envs(&envs);
        self.dump_metadata_for_single_invocation(&mut outputs2, &cmd, variant, &envs)?;
        let out = cmd.status()?;
        writeln!(outputs2, "\n\n\n")?;
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
    pub fn run(&mut self, log_dir: &Path) -> anyhow::Result<()> {
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
                    let result =
                        self.run_one(&self.profile, variant_name, variant, bench, log_dir, i);
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
