#[cfg(target_os = "linux")]
use crate::platform_info::PLATFORM_INFO;

impl super::RunArgs {
    fn dirty_git_worktree_check(&self) -> anyhow::Result<()> {
        let git_info = git_info::get();
        let Some(dirty) = git_info.dirty else {
            anyhow::bail!("No git repo found");
        };
        if dirty {
            if !self.allow_dirty {
                anyhow::bail!("Git worktree is dirty.");
            }
            eprintln!("🚨 WARNING: Git worktree is dirty.");
        }
        Ok(())
    }

    #[cfg(target_os = "linux")]
    pub(super) fn pre_benchmarking_checks(&self) -> anyhow::Result<()> {
        // Check if the current git worktree is dirty
        self.dirty_git_worktree_check()?;
        // Check if the current user is the only one logged in
        if PLATFORM_INFO.users.len() > 1 {
            let msg = format!(
                "More than one user logged in: {}",
                PLATFORM_INFO.users.join(", ")
            );
            if self.allow_multi_user {
                eprintln!("🚨 WARNING: {}", msg);
            } else {
                anyhow::bail!("{}", msg);
            }
        }
        // Check if all the scaling governors are set to `performance`
        if !PLATFORM_INFO
            .scaling_governor
            .iter()
            .all(|g| g == "performance")
        {
            let msg = format!(
                "Not all scaling governors are set to `performance`: [{}]",
                PLATFORM_INFO.scaling_governor.join(", ")
            );
            if self.allow_any_scaling_governor {
                eprintln!("🚨 WARNING: {}", msg);
            } else {
                anyhow::bail!("{}", msg);
            }
        }
        Ok(())
    }

    #[cfg(not(target_os = "linux"))]
    pub(super) fn pre_benchmarking_checks(&self) -> anyhow::Result<()> {
        // Check if the current git worktree is dirty
        self.dirty_git_worktree_check()?;
        Ok(())
    }
}
