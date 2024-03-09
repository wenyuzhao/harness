use std::{collections::HashMap, path::PathBuf};

use chrono::{DateTime, Local};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};

use crate::{config::Profile, utils};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CrateInfo {
    /// Crate name
    pub name: String,
    /// Path to the target directory
    pub target_dir: PathBuf,
    /// Benchmark names
    pub benches: Vec<String>,
}

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

    #[serde(rename = "crate")]
    pub crate_info: CrateInfo,

    pub profile: Profile,

    pub system: SystemInfo,
}

impl RunInfo {
    pub fn new(
        crate_info: CrateInfo,
        profile: Profile,
        runid: String,
        start_time: DateTime<Local>,
    ) -> anyhow::Result<Self> {
        Ok(Self {
            crate_info,
            system: SYSTEM_INFO.clone(),
            profile,
            runid,
            commit: utils::git::get_git_hash()?,
            start_timestamp_utc: start_time.to_utc().timestamp(),
            finish_timestamp_utc: None,
        })
    }

    pub fn load(path: &PathBuf) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        Ok(toml::from_str(&content)?)
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SystemInfo {
    pub host: String,
    pub os: String,
    pub arch: String,
    #[serde(rename = "kernel-version")]
    pub kernel: String,
    #[serde(rename = "cpu-model")]
    pub cpu_model: String,
    #[serde(rename = "cpu-frequency")]
    pub cpu_frequency: Vec<usize>,
    pub memory_size: usize,
    pub swap_size: usize,
    #[cfg(target_os = "linux")]
    pub users: Vec<String>,
    pub processes: usize,
    pub env: HashMap<String, String>,
    pub pid: usize,
    pub rustc: String,
    #[cfg(target_os = "linux")]
    #[serde(rename = "scaling-governor")]
    pub scaling_governor: Vec<String>,
}

pub static SYSTEM_INFO: Lazy<SystemInfo> = Lazy::new(|| utils::sys::get_current_system_info());
