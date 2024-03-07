#[cfg(target_os = "linux")]
use crate::platform_info::PLATFORM_INFO;
use colored::{Colorize, CustomColor};
use once_cell::sync::Lazy;

static BG: Lazy<CustomColor> = Lazy::new(|| CustomColor::new(0x23, 0x23, 0x23));

struct PreBenchmarkingChecker {
    warnings: Vec<String>,
    allow_dirty: bool,
    #[allow(unused)]
    allow_multi_user: bool,
    #[allow(unused)]
    allow_any_scaling_governor: bool,
}

impl PreBenchmarkingChecker {
    fn new(allow_dirty: bool, allow_multi_user: bool, allow_any_scaling_governor: bool) -> Self {
        Self {
            warnings: Vec::new(),
            allow_dirty,
            allow_multi_user,
            allow_any_scaling_governor,
        }
    }

    fn warn(&mut self, msg: impl AsRef<str>) {
        self.warnings.push(msg.as_ref().to_owned());
    }

    fn check_dirty_git_worktree(&mut self) -> anyhow::Result<()> {
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
        self.check_dirty_git_worktree()?;
        // Check if the current user is the only one logged in
        if PLATFORM_INFO.users.len() > 1 {
            let msg = format!(
                "More than one user logged in: {}",
                PLATFORM_INFO
                    .users
                    .iter()
                    .map(|u| u.on_custom_color(*BG).to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
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
            let sg = PLATFORM_INFO.scaling_governor.clone();
            let mut sg_dedup = sg.clone();
            sg_dedup.dedup();
            let sg_info = sg_dedup
                .iter()
                .map(|x| (x, sg.iter().filter(|y| x == *y).count()))
                .map(|(x, c)| format!("{} × {}", x, c).on_custom_color(*BG).to_string())
                .collect::<Vec<_>>()
                .join(", ");

            let msg =
                format!(
                "Not all scaling governors are set to performance: {}. See {} for more details.",
                sg_info.italic(),
                "https://wiki.archlinux.org/title/CPU_frequency_scaling".italic().underline()
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
    pub(super) fn pre_benchmarking_checks(&self) -> anyhow::Result<()> {
        let mut checker = PreBenchmarkingChecker::new(
            self.allow_dirty,
            self.allow_multi_user,
            self.allow_any_scaling_governor,
        );
        checker.check()?;
        checker.dump_warnings();
        Ok(())
    }
}
