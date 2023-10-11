use std::{collections::HashMap, fs::File, io::Write, path::PathBuf, process::Command};

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
    writeln!(f, "---")?;
    // runid and log dir
    writeln!(f, "runid: {}", runid)?;
    std::fs::create_dir_all(log_dir)?;
    writeln!(
        f,
        "log-dir: {}",
        log_dir.canonicalize()?.to_string_lossy().as_ref()
    )?;
    // machine and system info
    let mut sys = sysinfo::System::new_all();
    sys.refresh_all();
    writeln!(f, "os: {}", sys.long_os_version().unwrap_or_default())?;
    writeln!(
        f,
        "kernel-version: {}",
        sys.kernel_version().unwrap_or("unknown".to_owned())
    )?;
    writeln!(
        f,
        "host: {}",
        sys.host_name().unwrap_or("unknown".to_owned())
    )?;
    writeln!(
        f,
        "memory-size: {:.1} GB",
        sys.total_memory() as f32 / (1 << 30) as f32
    )?;
    // env variable
    writeln!(f, "env:")?;
    for (k, v) in profile.env.iter() {
        writeln!(f, "  {}: {}", k, v)?;
    }
    // git commit
    writeln!(f, "profile-commit: {}", get_git_hash())?;
    writeln!(f, "---")?;
    Ok(())
}

pub fn dump_metadata_for_single_invocation(
    log: &PathBuf,
    cmd: &Command,
    variant: &config::BuildVariant,
    envs: &HashMap<String, String>,
) -> anyhow::Result<()> {
    let mut f = File::create(log)?;
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
