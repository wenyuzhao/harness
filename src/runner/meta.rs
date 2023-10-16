use std::{collections::HashMap, io::Write, path::PathBuf, process::Command};

use sysinfo::SystemExt;

use crate::config::{self, Profile};

fn get_git_hash() -> String {
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

pub fn get_hostname() -> String {
    let mut sys = sysinfo::System::new_all();
    sys.refresh_all();
    sys.host_name().unwrap_or("unknown".to_owned())
}

pub fn dump_global_metadata(
    f: &mut impl Write,
    runid: &str,
    profile: &Profile,
    log_dir: &PathBuf,
) -> anyhow::Result<()> {
    // dump to file
    std::fs::create_dir_all(log_dir)?;
    #[derive(serde::Serialize)]
    struct ProfileWithExtraMeta<'a> {
        #[serde(flatten)]
        profile: &'a Profile,
        runid: &'a str,
        #[serde(rename = "profile-commit")]
        profile_commit: String,
        os: String,
        #[serde(rename = "kernel-version")]
        kernel_version: String,
        #[serde(rename = "memory-size")]
        memory_size: String,
        host: String,
    }
    let mut sys = sysinfo::System::new_all();
    sys.refresh_all();
    let meta = ProfileWithExtraMeta {
        profile,
        runid,
        profile_commit: get_git_hash(),
        os: sys.long_os_version().unwrap_or_default(),
        kernel_version: sys.kernel_version().unwrap_or("unknown".to_owned()),
        memory_size: format!("{:.1} GB", sys.total_memory() as f32 / (1 << 30) as f32),
        host: sys.host_name().unwrap_or("unknown".to_owned()),
    };
    std::fs::write(log_dir.join("config.toml"), toml::to_string(&meta)?)?;
    // dump to terminal
    writeln!(f, "---")?;
    // runid and log dir
    writeln!(f, "runid: {}", meta.runid)?;
    writeln!(
        f,
        "log-dir: {}",
        log_dir.canonicalize()?.to_string_lossy().as_ref()
    )?;
    // machine and system info
    writeln!(f, "os: {}", meta.os)?;
    writeln!(f, "kernel-version: {}", meta.kernel_version)?;
    writeln!(f, "host: {}", meta.host)?;
    writeln!(f, "memory-size: {}", meta.memory_size)?;
    // env variable
    writeln!(f, "env:")?;
    for (k, v) in profile.env.iter() {
        writeln!(f, "  {}: {}", k, v)?;
    }
    // git commit
    writeln!(f, "profile-commit: {}", meta.profile_commit)?;
    writeln!(f, "---")?;
    Ok(())
}

pub fn dump_metadata_for_single_invocation(
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
    writeln!(f, "commit: {}", get_git_hash())?;
    writeln!(f, "---")?;
    Ok(())
}
