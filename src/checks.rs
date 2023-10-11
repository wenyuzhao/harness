use git2::StatusOptions;

fn check_git_worktree(allow_dirty: bool) -> anyhow::Result<()> {
    let Ok(repo) = git2::Repository::open("..") else {
        anyhow::bail!("No git repo found");
    };
    if !repo
        .statuses(Some(StatusOptions::new().include_untracked(true)))?
        .is_empty()
    {
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
