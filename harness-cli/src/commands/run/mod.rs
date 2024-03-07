use std::path::PathBuf;

use cargo_metadata::MetadataCommand;
use chrono::{DateTime, Local};
use clap::Parser;
use termimad::crossterm::style::Stylize;

use crate::{
    config::{self, Profile},
    meta::{CrateInfo, RunInfo},
};

use super::report::ReportArgs;

mod checks;
mod runner;

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
    /// Report the benchmark results after running
    #[arg(long, default_value = "false")]
    pub report: bool,
}

impl RunArgs {
    fn generate_runid(&self) -> (String, DateTime<chrono::Local>) {
        let t = chrono::Local::now();
        let time = t.format("%Y-%m-%d-%a-%H%M%S").to_string();
        let host = crate::meta::PLATFORM_INFO.host.clone();
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
    fn dump_metadata(&self, log_dir: &PathBuf, run_info: &RunInfo) -> anyhow::Result<()> {
        // dump to file
        std::fs::create_dir_all(log_dir)?;
        std::fs::write(log_dir.join("config.toml"), toml::to_string(&run_info)?)?;
        Ok(())
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

    fn run_benchmarks(
        &self,
        crate_info: CrateInfo,
        mut profile: Profile,
        old_run: Option<&RunInfo>,
    ) -> anyhow::Result<()> {
        // Overwrite invocations and iterations
        if let Some(invocations) = self.invocations {
            profile.invocations = invocations;
        }
        if let Some(iterations) = self.iterations {
            profile.iterations = iterations;
        }
        // Checks
        let (runid, start_time) = self.generate_runid();
        let run_info = RunInfo::new(crate_info, profile, runid.clone(), start_time);
        if let Some(old) = old_run {
            self.reproducibility_checks(old, &run_info)?;
        }
        self.pre_benchmarking_checks(&run_info)?;
        // Initialize logs dir
        let log_dir = self.prepare_logs_dir(&run_info.crate_info, &runid)?;
        // Run benchmarks
        self.dump_metadata(&log_dir, &run_info)?;
        let mut runner = runner::BenchRunner::new(&run_info);
        runner.run(&log_dir)?;
        self.update_metadata_on_finish(&log_dir, run_info)?;
        Ok(())
    }

    fn prepare_reproduced_run(&self, crate_info: &CrateInfo) -> anyhow::Result<RunInfo> {
        // Load config and previous machine info
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
        println!(
            "{}",
            format!("Reproduce Run: {}\n", run_info.runid.clone().italic())
                .on_magenta()
                .bold()
        );
        let mut commit = run_info.commit.clone();
        if commit.ends_with("-dirty") {
            commit = commit.trim_end_matches("-dirty").to_owned();
        }
        println!("{}", format!("Checkout git commit: {}\n", commit).magenta());
        if RunInfo::get_git_hash() != run_info.commit {
            let output = std::process::Command::new("git")
                .args(&["checkout", &commit])
                .output()?;
            if !output.status.success() {
                anyhow::bail!(
                    "Failed to checkout git commit: {}: {}",
                    commit,
                    String::from_utf8_lossy(&output.stderr)
                );
            }
        }
        Ok(run_info)
    }

    pub fn run(&self) -> anyhow::Result<()> {
        let crate_info = self.load_crate_info()?;
        let (profile, old_run) = if self.config.is_some() {
            // Reproduce a previous run
            let old_run = self.prepare_reproduced_run(&crate_info)?;
            let profile = old_run.profile.clone();
            (profile, Some(old_run))
        } else {
            // A new run
            let config = config::load_from_cargo_toml()?;
            let Some(profile) = config.profiles.get(&self.profile).cloned() else {
                anyhow::bail!("Could not find harness profile `{}`", self.profile);
            };
            (profile, None)
        };
        let baseline = profile.baseline.clone();
        self.run_benchmarks(crate_info, profile, old_run.as_ref())?;
        // Report
        if self.report {
            let report = ReportArgs {
                run_id: None,
                norm: baseline.is_some(),
                baseline,
            };
            report.run()?;
        }
        Ok(())
    }
}
