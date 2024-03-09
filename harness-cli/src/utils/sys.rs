use sysinfo::{CpuExt, System, SystemExt};

use crate::meta::SystemInfo;

#[cfg(target_os = "linux")]
fn get_logged_in_users() -> anyhow::Result<Vec<String>> {
    use std::process::Command;

    Command::new("users")
        .output()
        .map(|o| {
            let mut users = String::from_utf8_lossy(&o.stdout)
                .split_whitespace()
                .map(|s| s.to_owned())
                .collect::<Vec<_>>();
            users.dedup();
            users
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

fn get_rustc_version() -> Option<String> {
    let vmeta = rustc_version::version_meta().ok()?;
    Some(format!(
        "{} ({})",
        vmeta.semver,
        format!("{:?}", vmeta.channel).to_lowercase()
    ))
}

pub fn get_current_system_info() -> SystemInfo {
    let mut sys = System::new_all();
    sys.refresh_all();
    const UNKNOWN: &str = "<unknown>";
    SystemInfo {
        host: sys.host_name().unwrap_or(UNKNOWN.to_string()),
        os: sys.long_os_version().unwrap_or(UNKNOWN.to_string()),
        arch: std::env::consts::ARCH.to_string(),
        kernel: sys.kernel_version().unwrap_or(UNKNOWN.to_string()),
        cpu_model: sys.global_cpu_info().brand().to_owned(),
        cpu_frequency: sys.cpus().iter().map(|c| c.frequency() as usize).collect(),
        memory_size: sys.total_memory() as usize,
        swap_size: sys.total_swap() as usize,
        processes: sys.processes().len(),
        env: std::env::vars().collect(),
        pid: std::process::id() as usize,
        rustc: get_rustc_version().unwrap_or_else(|| UNKNOWN.to_string()),
        #[cfg(target_os = "linux")]
        users: get_logged_in_users().unwrap_or_default(),
        #[cfg(target_os = "linux")]
        scaling_governor: get_scaling_governor().unwrap_or_default(),
    }
}
