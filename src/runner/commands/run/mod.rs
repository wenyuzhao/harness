use std::path::PathBuf;

use cargo_metadata::MetadataCommand;
use chrono::{DateTime, Local};
use clap::Parser;

use crate::{
    config::{self, Profile},
    platform_info::{RunInfo, PLATFORM_INFO},
};

mod bench_runner;
mod checks;

/// Run all the benchmarks
#[derive(Parser)]
pub struct RunArgs {
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
    #[arg(long, default_value = "false")]
    /// (Linux only) Allow benchmarking even when multiple users are logged in
    pub allow_multi_user: bool,
    /// (Linux only) Allow any scaling governor value, instead of only `performance`
    #[arg(long, default_value = "false")]
    pub allow_any_scaling_governor: bool,
    /// Specify a path to the config file, or the run id to reproduce a previous run.
    #[arg(long)]
    pub config: Option<String>,
}

struct CrateInfo {
    name: String,
    target_dir: PathBuf,
}

impl RunArgs {
    fn generate_runid(&self) -> (String, DateTime<chrono::Local>) {
        let t = chrono::Local::now();
        let time = t.format("%Y-%m-%d-%a-%H%M%S").to_string();
        let host = crate::platform_info::PLATFORM_INFO.host.clone();
        let run_id = format!("{}-{}-{}", self.profile, host, time);
        (run_id, t)
    }

    fn load_crate_info(&self) -> anyhow::Result<CrateInfo> {
        let Ok(meta) = MetadataCommand::new().manifest_path("./Cargo.toml").exec() else {
            anyhow::bail!("Failed to get metadata from ./Cargo.toml");
        };
        let target_dir = meta.target_directory.as_std_path();
        let Some(pkg) = meta.root_package() else {
            anyhow::bail!("No root package found");
        };
        Ok(CrateInfo {
            name: pkg.name.clone(),
            target_dir: target_dir.to_owned(),
        })
    }

    fn prepare_logs_dir(&self, crate_info: &CrateInfo, run_id: &str) -> anyhow::Result<PathBuf> {
        let logs_dir = crate_info.target_dir.join("harness").join("logs");
        let log_dir = logs_dir.join(run_id);
        let latest_log_dir = logs_dir.join("latest");
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
        Ok(log_dir)
    }

    /// Dump metadata before running all benchmarks
    /// This include platform info, env variables, and current git commit that the profile is loaded from.
    fn dump_metadata(
        &self,
        runid: &str,
        profile: &Profile,
        log_dir: &PathBuf,
        start_time: DateTime<Local>,
    ) -> anyhow::Result<RunInfo> {
        // dump to file
        std::fs::create_dir_all(log_dir)?;
        let profile_with_platform_info = RunInfo::new(profile, runid.to_owned(), start_time);
        std::fs::write(
            log_dir.join("config.toml"),
            toml::to_string(&profile_with_platform_info)?,
        )?;
        // dump to terminal
        println!("RUNID: {}", profile_with_platform_info.runid);
        println!("LOGS: {}", log_dir.to_str().unwrap());
        Ok(profile_with_platform_info)
    }

    fn update_metadata_on_finish(
        &self,
        log_dir: &PathBuf,
        mut meta: RunInfo,
    ) -> anyhow::Result<()> {
        assert!(log_dir.exists());
        assert!(meta.finish_timestamp_utc.is_none());
        meta.finish_timestamp_utc = Some(Local::now().to_utc().timestamp());
        std::fs::write(log_dir.join("config.toml"), toml::to_string(&meta)?)?;
        Ok(())
    }

    fn run_benchmarks(&self, crate_info: &CrateInfo, mut profile: Profile) -> anyhow::Result<()> {
        // Overwrite invocations and iterations
        if let Some(invocations) = self.invocations {
            profile.invocations = invocations;
        }
        if let Some(iterations) = self.iterations {
            profile.iterations = iterations;
        }
        // Prepare logs dir and runid
        let (run_id, start_time) = self.generate_runid();
        let log_dir = self.prepare_logs_dir(&crate_info, &run_id)?;
        let meta = self.dump_metadata(&run_id, &profile, &log_dir, start_time)?;
        // Run benchmarks
        let mut runner = bench_runner::BenchRunner::new(crate_info.name.clone(), profile);
        runner.run(&log_dir)?;
        self.update_metadata_on_finish(&log_dir, meta)?;
        Ok(())
    }

    fn reproduce_run(&self) -> anyhow::Result<()> {
        // Load config and previous machine info
        let crate_info = self.load_crate_info()?;
        let config_path_or_runid = self.config.as_ref().unwrap();
        let config_path = if config_path_or_runid.ends_with(".toml") {
            PathBuf::from(config_path_or_runid)
        } else {
            crate_info
                .target_dir
                .join("harness")
                .join("logs")
                .join(config_path_or_runid)
                .join("config.toml")
        };
        let run_info = RunInfo::load(&config_path)?;
        let profile = run_info.profile.clone();
        let platform = run_info.platform.clone();
        println!("Reproducing run: {}", run_info.runid);
        // Check current environment, and warn if it's different from the previous run
        if PLATFORM_INFO.os != platform.os {
            println!(
                "WARNING: OS changed: {} -> {}",
                platform.os, PLATFORM_INFO.os
            );
        }
        if PLATFORM_INFO.arch != platform.arch {
            println!(
                "WARNING: Architecture changed: {} -> {}",
                platform.arch, PLATFORM_INFO.arch
            );
        }
        if PLATFORM_INFO.kernel_version != platform.kernel_version {
            println!(
                "WARNING: Kernel changed: {} -> {}",
                platform.kernel_version, PLATFORM_INFO.kernel_version
            );
        }
        if PLATFORM_INFO.cpu_model != platform.cpu_model {
            println!(
                "WARNING: CPU changed: {} -> {}",
                platform.cpu_model, PLATFORM_INFO.cpu_model
            );
        }
        if PLATFORM_INFO.memory != platform.memory {
            println!(
                "WARNING: Memory changed: {} -> {}",
                platform.memory, PLATFORM_INFO.memory
            );
        }
        if PLATFORM_INFO.swap != platform.swap {
            println!(
                "WARNING: Swap changed: {} -> {}",
                platform.swap, PLATFORM_INFO.swap
            );
        }
        if PLATFORM_INFO.rustc != platform.rustc {
            println!(
                "WARNING: Rustc version changed: {} -> {}",
                platform.rustc, PLATFORM_INFO.rustc
            );
        }
        if PLATFORM_INFO.rustc != platform.rustc {
            println!(
                "WARNING: Rustc version changed: {} -> {}",
                platform.rustc, PLATFORM_INFO.rustc
            );
        }
        if PLATFORM_INFO.env != platform.env {
            println!("WARNING: Environment variable changed");
            for (k, v) in &platform.env {
                if PLATFORM_INFO.env.get(k) != Some(v) {
                    println!("  - {}: {:?} -> {:?}", k, v, PLATFORM_INFO.env.get(k));
                }
            }
            for (k, v) in &PLATFORM_INFO.env {
                if !platform.env.contains_key(k) {
                    println!("  - {}: {:?} -> {:?}", k, platform.env.get(k), v);
                }
            }
        }
        #[cfg(target_os = "linux")]
        if PLATFORM_INFO.scaling_governor != platform.scaling_governor {
            println!(
                "WARNING: Scaling governor changed: {:?} -> {:?}",
                platform.scaling_governor, PLATFORM_INFO.scaling_governor
            );
        }
        if self.invocations.is_some() && self.invocations != Some(profile.invocations) {
            println!(
                "WARNING: Invocations changed: {} -> {}",
                profile.invocations,
                self.invocations.unwrap()
            );
        }
        if self.iterations.is_some() && self.iterations != Some(profile.iterations) {
            println!(
                "WARNING: Iterations changed: {} -> {}",
                profile.iterations,
                self.iterations.unwrap()
            );
        }
        // Run benchmarks
        self.run_benchmarks(&crate_info, profile)?;
        Ok(())
    }

    pub fn run(&self) -> anyhow::Result<()> {
        // Pre-benchmarking checks
        let crate_info = self.load_crate_info()?;
        self.pre_benchmarking_checks()?;
        // Reproduce a previous run?
        if self.config.is_some() {
            return self.reproduce_run();
        }
        // Load benchmark profile
        let config = config::load_from_cargo_toml()?;
        let Some(profile) = config.profiles.get(&self.profile).cloned() else {
            anyhow::bail!("Could not find harness profile `{}`", self.profile);
        };
        // Run benchmarks
        self.run_benchmarks(&crate_info, profile)?;
        Ok(())
    }
}
