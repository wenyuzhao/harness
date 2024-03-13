use std::{collections::HashMap, path::Path};

use crate::configs::{
    harness::Profile,
    run_info::{CrateInfo, Lockfiles, RunInfo},
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
        let _git_guard = git::checkout(commit)?;
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
    }
    Ok(Lockfiles { lockfiles })
}

pub struct TempLockfileGuard {
    lockfile_path: std::path::PathBuf,
    original_lockfile: String,
}

impl Drop for TempLockfileGuard {
    fn drop(&mut self) {
        std::fs::write(&self.lockfile_path, &self.original_lockfile).unwrap();
    }
}

pub fn replay_lockfile(run_info: &RunInfo, hash: &str) -> anyhow::Result<TempLockfileGuard> {
    let lockfile = run_info
        .lockfiles
        .lockfiles
        .get(hash)
        .ok_or_else(|| anyhow::anyhow!("Lockfile for commit `{}` not found", hash))?;
    let lockfile_path = run_info.crate_info.workspace_root.join("Cargo.lock");
    let original_lockfile = std::fs::read_to_string(&lockfile_path)?;
    std::fs::write(&lockfile_path, toml::to_string(lockfile)?)?;
    Ok(TempLockfileGuard {
        lockfile_path,
        original_lockfile,
    })
}
