use std::{collections::HashMap, path::PathBuf};

use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use sysinfo::{CpuExt, System, SystemExt};

use crate::config::Profile;

#[derive(Debug, Serialize, Deserialize)]
pub struct ProfileWithPlatformInfo {
    pub platform: PlatformInfo,
    pub profile: Profile,
    pub runid: String,
    #[serde(rename = "profile-commit")]
    pub profile_commit: String,
}

impl ProfileWithPlatformInfo {
    pub fn get_git_hash() -> String {
        let git_info = git_info::get();
        let mut hash = git_info
            .head
            .last_commit_hash
            .unwrap_or("unknown".to_owned());
        if git_info.dirty.unwrap_or_default() {
            hash += "-dirty";
        }
        hash
    }

    pub fn new(profile: &Profile, runid: String) -> Self {
        Self {
            platform: PLATFORM_INFO.clone(),
            profile: profile.clone(),
            runid,
            profile_commit: Self::get_git_hash(),
        }
    }

    pub fn load(path: &PathBuf) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        Ok(toml::from_str(&content)?)
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PlatformInfo {
    pub host: String,
    pub os: String,
    pub arch: String,
    #[serde(rename = "kernel-version")]
    pub kernel_version: String,
    #[serde(rename = "cpu-model")]
    pub cpu_model: String,
    #[serde(rename = "cpu-frequency")]
    pub cpu_frequency: Vec<usize>,
    pub memory: usize,
    pub swap: usize,
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

fn get_rustc_version() -> Option<String> {
    let vmeta = rustc_version::version_meta().ok()?;
    Some(format!(
        "{} ({})",
        vmeta.semver,
        format!("{:?}", vmeta.channel).to_lowercase()
    ))
}

#[cfg(target_os = "linux")]
fn get_logged_in_users() -> anyhow::Result<Vec<String>> {
    std::process::Command::new("users")
        .output()
        .map(|o| {
            String::from_utf8_lossy(&o.stdout)
                .split_whitespace()
                .map(|s| s.to_owned())
                .collect()
        })
        .map_err(|e| e.into())
}

#[cfg(target_os = "linux")]
fn get_scaling_governor() -> anyhow::Result<Vec<String>> {
    let mut governors = Vec::new();
    let mut sys = System::new_all();
    sys.refresh_all();
    for path in (std::fs::read_dir("/sys/devices/system/cpu/")?).flatten() {
        let path = path.path();
        if path.is_dir() {
            let path = path.join("cpufreq/scaling_governor");
            if path.exists() {
                if let Ok(governor) = std::fs::read_to_string(path) {
                    governors.push(governor.trim().to_owned());
                }
            }
        }
    }
    Ok(governors)
}

pub static PLATFORM_INFO: Lazy<PlatformInfo> = Lazy::new(|| {
    let mut sys = System::new_all();
    sys.refresh_all();
    const UNKNOWN: &str = "<unknown>";
    PlatformInfo {
        host: sys.host_name().unwrap_or(UNKNOWN.to_string()),
        os: sys.long_os_version().unwrap_or(UNKNOWN.to_string()),
        arch: std::env::consts::ARCH.to_string(),
        kernel_version: sys.kernel_version().unwrap_or(UNKNOWN.to_string()),
        cpu_model: sys.global_cpu_info().brand().to_owned(),
        cpu_frequency: sys.cpus().iter().map(|c| c.frequency() as usize).collect(),
        memory: sys.total_memory() as usize,
        swap: sys.total_swap() as usize,
        processes: sys.processes().len(),
        env: std::env::vars().collect(),
        pid: std::process::id() as usize,
        rustc: get_rustc_version().unwrap_or_else(|| UNKNOWN.to_string()),
        #[cfg(target_os = "linux")]
        users: get_logged_in_users().unwrap_or_default(),
        #[cfg(target_os = "linux")]
        scaling_governor: get_scaling_governor().unwrap_or_default(),
    }
});
