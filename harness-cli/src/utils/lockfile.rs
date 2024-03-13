use std::{collections::HashMap, path::Path};

use crate::configs::{
    harness::Profile,
    run_info::{CrateInfo, Lockfiles},
};

use super::{bench_cmd, git};

fn load_current_lockfile(ws: &Path) -> anyhow::Result<toml::Value> {
    let lockfile = ws.join("Cargo.lock");
    let lockfile = std::fs::read_to_string(lockfile)?;
    let lockfile = toml::from_str(&lockfile)?;
    Ok(lockfile)
}

pub fn load_lockfiles(crate_info: &CrateInfo, profile: &Profile) -> anyhow::Result<Lockfiles> {
    // Get lockfile for each build
    let mut lockfiles = HashMap::new();
    let lockfile_path = crate_info.workspace_root.join("Cargo.lock");
    let profile_commit = super::git::get_git_hash()?;
    for (build_name, build) in &profile.builds {
        // Switch to the build commit
        let commit = build.commit.as_deref().unwrap_or(profile_commit.as_str());
        if commit != profile_commit {
            git::checkout(commit)?;
        }
        // Run cargo build once to generate the lockfile
        if !lockfile_path.exists() {
            let mut cmd = bench_cmd::get_bench_build_command(profile, build_name);
            let out = cmd
                .output()
                .map_err(|e| anyhow::anyhow!("Failed to build `{}`: {}", build_name, e))?;
            if !out.status.success() {
                eprintln!("{}", String::from_utf8_lossy(&out.stderr));
                anyhow::bail!("Failed to build `{}`", build_name,);
            }
        }
        // Get the lock file
        let lockfile = load_current_lockfile(&crate_info.workspace_root)?;
        let mut commit_hash = commit.to_owned();
        if commit_hash.ends_with("-dirty") {
            commit_hash = commit_hash.trim_end_matches("-dirty").to_owned();
        }
        lockfiles.insert(commit_hash, lockfile);
        // Switch back to the original commit
        if commit != profile_commit {
            git::checkout(&profile_commit)?;
        }
    }
    Ok(Lockfiles { lockfiles })
}
