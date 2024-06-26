use std::process::Command;

use git_info2::types::GitInfo;

pub fn get_git_hash() -> anyhow::Result<String> {
    let git_info = git_info2::get();
    let mut hash = git_info
        .head
        .last_commit_hash
        .ok_or_else(|| anyhow::anyhow!("Failed to get the current commit hash"))?;
    if git_info.dirty.unwrap_or_default() {
        hash += "-dirty";
    }
    Ok(hash)
}

pub fn get_second_last_git_hash() -> anyhow::Result<String> {
    Command::new("git")
        .args(["rev-parse", "@~"])
        .output()
        .map_err::<anyhow::Error, _>(|e| e.into())
        .and_then(|o| String::from_utf8(o.stdout).map_err(|e| e.into()))
        .map(|s| s.trim().to_owned())
}

pub fn get_branch_last_git_hash(branch: &str) -> anyhow::Result<String> {
    Command::new("git")
        .args(["rev-parse", branch])
        .output()
        .map_err::<anyhow::Error, _>(|e| e.into())
        .and_then(|o| String::from_utf8(o.stdout).map_err(|e| e.into()))
        .map(|s| s.trim().to_owned())
}

pub fn restore_git_state(prev: &GitInfo) -> anyhow::Result<()> {
    let curr = git_info2::get();
    if prev.head.last_commit_hash != curr.head.last_commit_hash {
        let checkout_target = if let Some(branch) = prev.current_branch.as_ref() {
            let hash = get_branch_last_git_hash(branch)?;
            if Some(hash) == prev.head.last_commit_hash {
                branch
            } else {
                prev.head.last_commit_hash.as_ref().unwrap()
            }
        } else {
            prev.head.last_commit_hash.as_ref().unwrap()
        };
        checkout_no_guard(checkout_target)?;
    }
    Ok(())
}

fn checkout_no_guard(mut commit: &str) -> anyhow::Result<bool> {
    let current_commit = git_info2::get().head.last_commit_hash.unwrap();
    if commit.ends_with("-dirty") {
        commit = commit.trim_end_matches("-dirty");
    }
    if current_commit == commit {
        return Ok(false);
    }
    let output = Command::new("git").args(["checkout", commit]).output()?;
    if !output.status.success() {
        anyhow::bail!(
            "Failed to checkout git commit: {}: {}",
            commit,
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(true)
}

pub struct TempGitCommitGuard {
    prev: GitInfo,
}

impl Drop for TempGitCommitGuard {
    fn drop(&mut self) {
        restore_git_state(&self.prev).unwrap();
    }
}

pub fn checkout(commit: &str) -> anyhow::Result<TempGitCommitGuard> {
    let prev = git_info2::get();
    checkout_no_guard(commit)?;
    Ok(TempGitCommitGuard { prev })
}
