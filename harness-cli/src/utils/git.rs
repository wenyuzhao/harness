use std::process::Command;

use git_info::types::GitInfo;

pub fn get_git_hash() -> anyhow::Result<String> {
    let git_info = git_info::get();
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
    let curr = git_info::get();
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
        checkout(checkout_target)?;
    }
    Ok(())
}

pub fn checkout(commit: &str) -> anyhow::Result<()> {
    let output = Command::new("git").args(["checkout", commit]).output()?;
    if !output.status.success() {
        anyhow::bail!(
            "Failed to checkout git commit: {}: {}",
            commit,
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(())
}
