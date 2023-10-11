fn check_git_worktree(allow_dirty: bool) -> anyhow::Result<()> {
    let git_info = git_info::get();
    let Some(dirty) = git_info.dirty else {
        anyhow::bail!("No git repo found");
    };
    if dirty {
        if !allow_dirty {
            anyhow::bail!("Current repository is dirty.");
        }
        eprintln!("🚨 WARNING: Git worktree is dirty.");
    }
    Ok(())
}

pub fn pre_benchmarking_checks(allow_dirty: bool) -> anyhow::Result<()> {
    check_git_worktree(allow_dirty)?;
    Ok(())
}
