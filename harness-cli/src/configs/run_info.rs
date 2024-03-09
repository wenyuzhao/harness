//! The evaluation summary specification.
//!
//! A evaluation summary contains the metadata of the evaluation run, including the crate info, the current system info at the time of the evaluation, and the enabled evaluation profile.
//!
//! Each `cargo harness run` will start an evaluation, and generate a unique `RUNID` for this evaluation.
//! The evaluation summary will be dumped to `target/harness/logs/<RUNID>/config.toml`.
//!
//! By having the repo and the evaluation summary, you can reproduce the evaluation by running:
//!
//! ```bash
//! cargo harness run --config <RUNID>
//! ```
//!
//! OR
//!
//! ```bash
//! cargo harness run --config /path/to/config.toml
//! ```

use std::{collections::HashMap, path::PathBuf};

use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};

use crate::utils;

use super::harness::Profile;

/// The evaluation run metadata. This will be colelcted before eaech evaluation and dumped to the log directory.
#[derive(Debug, Serialize, Deserialize)]
pub struct RunInfo {
    /// Benchmark run id
    pub runid: String,
    /// Benchmark start time
    #[serde(rename = "start-time-utc")]
    pub start_timestamp_utc: i64,
    /// Benchmark finish time
    #[serde(rename = "finish-time-utc")]
    pub finish_timestamp_utc: Option<i64>,
    /// The commit that the profile is loaded from. This is also used as the default build commit
    pub commit: String,
    /// The crate info
    #[serde(rename = "crate")]
    pub crate_info: CrateInfo,
    /// The enabled evaluation profile
    pub profile: Profile,
    /// Current system information
    pub system: SystemInfo,
}

impl RunInfo {
    pub(crate) fn new(
        crate_info: CrateInfo,
        profile: Profile,
        runid: String,
        start_time: DateTime<Local>,
    ) -> anyhow::Result<Self> {
        Ok(Self {
            crate_info,
            system: utils::sys::get_current_system_info(),
            profile,
            runid,
            commit: utils::git::get_git_hash()?,
            start_timestamp_utc: start_time.to_utc().timestamp(),
            finish_timestamp_utc: None,
        })
    }

    pub(crate) fn load(path: &PathBuf) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        Ok(toml::from_str(&content)?)
    }
}

/// Crate metadata
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CrateInfo {
    /// Crate name
    pub name: String,
    /// Path to the target directory
    pub target_dir: PathBuf,
    /// All benchmark names used in the evaluation
    pub benches: Vec<String>,
}

/// The system information, including the hareware specs, the OS info, and the environment variables.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SystemInfo {
    /// Host name
    pub host: String,
    /// Operating system name and version
    pub os: String,
    /// CPU architecture
    pub arch: String,
    /// Kernel version
    #[serde(rename = "kernel-version")]
    pub kernel: String,
    /// CPU model
    #[serde(rename = "cpu-model")]
    pub cpu_model: String,
    /// CPU frequency
    #[serde(rename = "cpu-frequency")]
    pub cpu_frequency: Vec<usize>,
    /// Total memory size in bytes
    pub memory_size: usize,
    /// Total swap size in bytes
    pub swap_size: usize,
    /// (*Linux only*) All logged in users
    #[cfg(target_os = "linux")]
    pub users: Vec<String>,
    /// Total number of running processes
    pub processes: usize,
    /// All current environment variables
    pub env: HashMap<String, String>,
    /// The PID of the current process
    pub pid: usize,
    /// The rustc version
    pub rustc: String,
    /// (*Linux only*) The scaling governor of each CPU core
    #[cfg(target_os = "linux")]
    #[serde(rename = "scaling-governor")]
    pub scaling_governor: Vec<String>,
}
