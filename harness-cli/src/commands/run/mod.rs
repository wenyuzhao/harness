use std::path::{Path, PathBuf};

use chrono::{DateTime, Local};
use clap::Parser;
use termimad::crossterm::style::Stylize;

use crate::{
    configs::{
        harness::{BuildConfig, HarnessConfig, Profile},
        run_info::{CrateInfo, RunInfo},
    },
    utils,
};

use super::report::ReportArgs;

mod checks;
mod runner;

/// Start a benchmarking run
#[derive(Parser)]
pub struct RunArgs {
    /// Number of iterations. Default is 5, or the value specified in the profile.
    #[arg(short = 'n', long)]
    pub iterations: Option<usize>,
    /// Number of invocations. Default is 10, or the value specified in the profile.
    #[arg(short = 'i', long)]
    pub invocations: Option<usize>,
    /// Benchmarking profile
    #[arg(short, long, default_value = "default")]
    pub profile: String,
    /// Allow dirty working directories
    #[arg(long, default_value = "false")]
    pub allow_dirty: bool,
    /// (Linux only) Allow benchmarking even when multiple users are logged in
    #[arg(long, default_value = "false")]
    pub allow_multiple_users: bool,
    /// (Linux only) Allow any scaling governor value, instead of only `performance`
    #[arg(long, default_value = "false")]
    pub allow_any_scaling_governor: bool,
    /// Specify a path to the config file, or the run id to reproduce a previous run.
    #[arg(long)]
    pub config: Option<String>,
    /// Do an one-shot test run on a single benchmark.
    #[arg(long)]
    pub bench: Option<String>,
    /// The build used for the one-shot test run.
    /// If not specified, a temporary default build config will be created and used.
    #[arg(long)]
    pub build: Option<String>,
    /// Report the benchmark results after running
    #[arg(long, default_value = "false")]
    pub report: bool,
}

impl RunArgs {
    fn generate_runid(&self) -> (String, DateTime<chrono::Local>) {
        let t = chrono::Local::now();
        let time = t.format("%Y-%m-%d-%a-%H%M%S").to_string();
        let host = utils::sys::get_current_host();
        let run_id = format!("{}-{}-{}", self.profile, host, time);
        (run_id, t)
    }

    fn prepare_logs_dir(&self, crate_info: &CrateInfo, run_id: &str) -> anyhow::Result<PathBuf> {
        let logs_dir = crate_info.target_dir.join("harness").join("logs");
        let log_dir = logs_dir.join(run_id);
        let latest_log_dir = logs_dir.join("latest");
        std::fs::create_dir_all(&log_dir)?;
        if latest_log_dir.exists() || latest_log_dir.is_symlink() {
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

    fn update_metadata_on_finish(&self, log_dir: &Path, mut meta: RunInfo) -> anyhow::Result<()> {
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
        // Default build configs
        if profile.builds.is_empty() {
            let head = BuildConfig {
                commit: Some(utils::git::get_git_hash()?),
                ..Default::default()
            };
            profile.builds.insert("HEAD".to_owned(), head);
            let head_1 = BuildConfig {
                commit: Some(utils::git::get_second_last_git_hash()?),
                ..Default::default()
            };
            profile.builds.insert("HEAD~1".to_owned(), head_1);
        }
        // If this is a reproduced run, use the old crate info
        let crate_info = if let Some(old) = old_run {
            old.crate_info.clone()
        } else {
            crate_info
        };
        // Create a new run
        let (runid, start_time) = self.generate_runid();
        let run_info = RunInfo::new(crate_info, profile, runid.clone(), start_time)?;
        // Run checks
        checks::run_all_checks(self, &run_info, old_run)?;
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
        if utils::git::get_git_hash()? != run_info.commit {
            utils::git::checkout(&commit)?;
        }
        Ok(run_info)
    }

    pub fn test_run(&self, crate_info: &CrateInfo) -> anyhow::Result<()> {
        if self.invocations.is_some() {
            anyhow::bail!("Cannot specify invocations for a single-shot test run");
        }
        if self.config.is_some() {
            anyhow::bail!("Cannot specify config for a single-shot test run");
        }
        let bench = self.bench.as_ref().unwrap();
        let config = HarnessConfig::load_from_cargo_toml()?;
        let Some(mut profile) = config.profiles.get(&self.profile).cloned() else {
            anyhow::bail!("Could not find harness profile `{}`", self.profile);
        };
        if let Some(iterations) = self.iterations {
            profile.iterations = iterations;
        }
        let build = if self.build.is_none() {
            let test_build_name = "@test";
            profile
                .builds
                .insert(test_build_name.to_owned(), BuildConfig::default());
            test_build_name
        } else {
            self.build.as_ref().unwrap()
        };
        let (runid, start_time) = self.generate_runid();
        let run_info = RunInfo::new(crate_info.clone(), profile, runid.clone(), start_time)?;
        let runner = runner::BenchRunner::new(&run_info);
        runner.test_run(bench, build)?;
        Ok(())
    }

    pub fn run(&self) -> anyhow::Result<()> {
        let crate_info = CrateInfo::load()?;
        if self.bench.is_some() {
            return self.test_run(&crate_info);
        }
        let (profile, old_run) = if self.config.is_some() {
            // Reproduce a previous run
            let old_run = self.prepare_reproduced_run(&crate_info)?;
            let profile = old_run.profile.clone();
            (profile, Some(old_run))
        } else {
            // A new run
            let config = HarnessConfig::load_from_cargo_toml()?;
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
            println!();
            report.run()?;
        }
        Ok(())
    }
}
