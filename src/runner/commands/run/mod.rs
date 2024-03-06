use std::path::PathBuf;

use cargo_metadata::MetadataCommand;
use chrono::{DateTime, Local};
use clap::Parser;

use crate::{
    config::{self, Profile},
    platform_info::ProfileWithPlatformInfo,
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
    ) -> anyhow::Result<ProfileWithPlatformInfo> {
        // dump to file
        std::fs::create_dir_all(log_dir)?;
        let profile_with_platform_info =
            ProfileWithPlatformInfo::new(profile, runid.to_owned(), start_time);
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
        mut meta: ProfileWithPlatformInfo,
    ) -> anyhow::Result<()> {
        assert!(log_dir.exists());
        assert!(meta.finish_timestamp_utc.is_none());
        meta.finish_timestamp_utc = Some(Local::now().to_utc().timestamp());
        std::fs::write(log_dir.join("config.toml"), toml::to_string(&meta)?)?;
        Ok(())
    }

    pub fn run(&self) -> anyhow::Result<()> {
        // Pre-benchmarking checks
        let crate_info = self.load_crate_info()?;
        self.pre_benchmarking_checks()?;
        // Load benchmark profile
        let config = config::load_from_cargo_toml()?;
        let Some(mut profile) = config.profiles.get(&self.profile).cloned() else {
            anyhow::bail!("Could not find harness profile `{}`", self.profile);
        };
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
        let mut runner = bench_runner::BenchRunner::new(crate_info.name, profile);
        runner.run(&log_dir)?;
        self.update_metadata_on_finish(&log_dir, meta)?;
        Ok(())
    }
}
