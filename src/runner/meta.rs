use std::{collections::HashMap, io::Write, path::PathBuf, process::Command};

use crate::{
    config::{self, Profile},
    platform_info::ProfileWithPlatformInfo,
};

/// Dump metadata before running all benchmarks
/// This include platform info, env variables, and current git commit that the profile is loaded from.
pub fn dump_global_metadata(
    f: &mut impl Write,
    runid: &str,
    profile: &Profile,
    log_dir: &PathBuf,
) -> anyhow::Result<()> {
    // dump to file
    std::fs::create_dir_all(log_dir)?;
    let profile_with_platform_info = ProfileWithPlatformInfo::new(profile, runid.to_owned());
    std::fs::write(
        log_dir.join("config.toml"),
        toml::to_string(&profile_with_platform_info)?,
    )?;
    // dump to terminal
    writeln!(f, "RUNID: {}", profile_with_platform_info.runid)?;
    Ok(())
}

/// Dump invocation-related metadata to the corresponding log file at the start of each invocation
/// This include: env variables, command line args, cargo features, and git commit
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
    writeln!(f, "commit: {}", ProfileWithPlatformInfo::get_git_hash())?;
    writeln!(f, "---")?;
    Ok(())
}
