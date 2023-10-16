use std::{fs::OpenOptions, io, io::Write, path::Path, process::Command};

use cargo_metadata::MetadataCommand;

use crate::config;

/// Benchmark running info
#[derive(Debug)]
struct Harness {
    /// Name of the current crate
    crate_name: String,
    /// Names of the benches to run
    benches: Vec<String>,
    /// Benchmark profile
    profile: config::Profile,
}

impl Harness {
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

    /// Run one benchmark with a build variant, for N iterations.
    fn run_one(
        &self,
        profile: &config::Profile,
        varient_name: &str,
        variant: &config::BuildVariant,
        bench: &str,
        log_dir: &Path,
        allow_dirty: bool,
        invocation: usize,
    ) -> anyhow::Result<()> {
        std::fs::create_dir_all(&log_dir)?;
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
        if allow_dirty {
            cmd.arg("--allow-dirty");
        }
        let mut envs = profile.env.clone();
        for (k, v) in &variant.env {
            envs.insert(k.clone(), v.clone());
        }
        cmd.envs(&envs);
        crate::meta::dump_metadata_for_single_invocation(&mut outputs2, &cmd, variant, &envs)?;
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
    pub fn run(&mut self, log_dir: &Path, allow_dirty: bool) -> anyhow::Result<()> {
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
                        log_dir,
                        allow_dirty,
                        i,
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

fn generate_runid(profile_name: &str) -> String {
    let time = chrono::Local::now()
        .format("%Y-%m-%d-%a-%H%M%S")
        .to_string();
    let host = crate::meta::get_hostname();
    let run_id = format!("{}-{}-{}", profile_name, host, time);
    run_id
}

pub fn harness_run(args: &crate::RunArgs) -> anyhow::Result<()> {
    let Ok(meta) = MetadataCommand::new().manifest_path("./Cargo.toml").exec() else {
        anyhow::bail!("Failed to get metadata from ./Cargo.toml");
    };
    let target_dir = meta.target_directory.as_std_path();
    let Some(pkg) = meta.root_package() else {
        anyhow::bail!("Could not find root package");
    };
    crate::checks::pre_benchmarking_checks(args.allow_dirty)?;
    let config = crate::config::load_from_cargo_toml()?;
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
    let run_id = generate_runid(&args.profile);
    let log_dir = target_dir.join("harness").join("logs").join(&run_id);
    let latest_log_dir = target_dir.join("harness").join("logs").join("latest");
    std::fs::create_dir_all(&log_dir)?;
    if latest_log_dir.exists() {
        if latest_log_dir.is_dir() && !latest_log_dir.is_symlink() {
            std::fs::remove_dir(&latest_log_dir)?;
        } else {
            std::fs::remove_file(&latest_log_dir)?;
        }
    }
    #[cfg(target_os = "windows")]
    std::os::windows::fs::symlink_dir(&log_dir, latest_log_dir)?;
    #[cfg(not(target_os = "windows"))]
    std::os::unix::fs::symlink(&log_dir, latest_log_dir)?;
    crate::meta::dump_global_metadata(&mut std::io::stdout(), &run_id, &profile, &log_dir)?;
    let mut harness = Harness::new(pkg.name.clone(), profile);
    harness.run(&log_dir, args.allow_dirty)?;
    Ok(())
}
