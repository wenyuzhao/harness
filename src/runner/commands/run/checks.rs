#[cfg(target_os = "linux")]
use crate::platform_info::PLATFORM_INFO;
use colored::Colorize;

struct PreBenchmarkingChecker {
    warnings: Vec<String>,
    allow_dirty: bool,
    #[allow(unused)]
    allow_multi_user: bool,
}

impl PreBenchmarkingChecker {
    fn new(allow_dirty: bool, allow_multi_user: bool) -> Self {
        Self {
            warnings: Vec::new(),
            allow_dirty,
            allow_multi_user,
        }
    }

    fn warn(&mut self, msg: impl AsRef<str>) {
        self.warnings.push(msg.as_ref().to_owned());
    }

    fn dirty_git_worktree_check(&mut self) -> anyhow::Result<()> {
        let git_info = git_info::get();
        let Some(dirty) = git_info.dirty else {
            anyhow::bail!("No git repo found");
        };
        if dirty {
            if !self.allow_dirty {
                anyhow::bail!("Git worktree is dirty.");
            }
            self.warn("Git worktree is dirty.".to_string());
        }
        Ok(())
    }

    #[cfg(target_os = "linux")]
    fn check(&mut self) -> anyhow::Result<()> {
        // Check if the current git worktree is dirty
        self.dirty_git_worktree_check()?;
        // Check if the current user is the only one logged in
        if PLATFORM_INFO.users.len() > 1 {
            let msg = format!(
                "More than one user logged in: {}",
                PLATFORM_INFO.users.join(", ")
            );
            if self.allow_multi_user {
                self.warn(msg);
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
                self.warn(msg);
            } else {
                anyhow::bail!("{}", msg);
            }
        }
        Ok(())
    }

    #[cfg(not(target_os = "linux"))]
    fn check(&mut self) -> anyhow::Result<()> {
        // Check if the current git worktree is dirty
        self.dirty_git_worktree_check()?;
        Ok(())
    }

    fn dump_warnings(&self) {
        eprintln!("{}\n", "WARNING".bold().black().on_red());
        for msg in &self.warnings {
            eprintln!("{} {}", "•".bright_red(), msg.red());
        }
        eprintln!("");
    }
}

impl super::RunArgs {
    #[cfg(target_os = "linux")]
    pub(super) fn pre_benchmarking_checks(&self) -> anyhow::Result<()> {
        let mut errors = Vec::new();
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
        let mut checker = PreBenchmarkingChecker::new(self.allow_dirty, self.allow_multi_user);
        checker.check()?;
        checker.dump_warnings();
        Ok(())
    }
}
