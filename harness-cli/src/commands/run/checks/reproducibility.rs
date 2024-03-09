use colored::{Colorize, CustomColor};
use once_cell::sync::Lazy;

use crate::{commands::run::RunArgs, configs::run_info::RunInfo};

use super::super::runner::BenchRunner;

static BG: Lazy<CustomColor> = Lazy::new(|| CustomColor::new(0x23, 0x23, 0x23));

struct PreBenchmarkingChecker<'a> {
    warnings: Vec<String>,
    allow_dirty: bool,
    #[allow(unused)]
    allow_multi_user: bool,
    #[allow(unused)]
    allow_any_scaling_governor: bool,
    run: &'a RunInfo,
}

impl<'a> PreBenchmarkingChecker<'a> {
    fn new(
        run: &'a RunInfo,
        allow_dirty: bool,
        allow_multi_user: bool,
        allow_any_scaling_governor: bool,
    ) -> Self {
        Self {
            warnings: Vec::new(),
            allow_dirty,
            allow_multi_user,
            allow_any_scaling_governor,
            run,
        }
    }

    fn warn(&mut self, msg: impl AsRef<str>) {
        self.warnings.push(msg.as_ref().to_owned());
    }

    fn check_bench_configs(&mut self) -> anyhow::Result<()> {
        let benches = self.run.crate_info.benches.len();
        if benches == 0 {
            anyhow::bail!("No benches found.");
        }
        if benches == 1 {
            self.warn("Only one benchmark is probably not enough.");
        }
        Ok(())
    }

    fn check_build_configs(&mut self) -> anyhow::Result<()> {
        // No builds or only one build?
        let builds = self.run.profile.builds.len();
        if builds == 0 {
            anyhow::bail!("No builds found in the profile.");
        }
        if builds == 1 {
            self.warn("It's recommended to always have more than one builds.");
        }
        if builds >= BenchRunner::MAX_SUPPORTED_BUILDS {
            anyhow::bail!(
                "Too many builds. Maximum supported builds is {}.",
                BenchRunner::MAX_SUPPORTED_BUILDS
            );
        }
        // Identical builds?
        let names = self.run.profile.builds.keys().cloned().collect::<Vec<_>>();
        for i in 0..names.len() {
            for j in i + 1..names.len() {
                let (n1, n2) = (&names[i], &names[j]);
                if self.run.profile.builds[n1] == self.run.profile.builds[n2] {
                    self.warn(format!(
                        "Builds {} and {} are identical.",
                        n1.italic(),
                        n2.italic(),
                    ));
                }
            }
        }
        // git commit exists?
        for (name, build) in &self.run.profile.builds {
            if let Some(mut commit) = build.commit.clone() {
                if commit.ends_with("-dirty") {
                    commit = commit.trim_end_matches("-dirty").to_owned();
                }
                let verified = std::process::Command::new("git")
                    .args(["cat-file", "-e", &commit])
                    .current_dir(&self.run.crate_info.target_dir)
                    .output()
                    .map(|o| o.status.success())
                    .unwrap_or(false);
                if !verified {
                    anyhow::bail!(
                        "Git commit for build `{}` does not exist: {}.",
                        name.italic(),
                        commit.italic().on_custom_color(*BG),
                    );
                }
            }
        }
        // baseline correct?
        if let Some(baseline) = &self.run.profile.baseline {
            if !self.run.profile.builds.contains_key(baseline) {
                anyhow::bail!(
                    "Baseline `{}` is not an existing build name.",
                    baseline.italic(),
                );
            }
        }
        Ok(())
    }

    #[cfg(target_os = "linux")]
    fn check_perf_event(&mut self) -> anyhow::Result<()> {
        let perf_event_paranoid = std::fs::read_to_string("/proc/sys/kernel/perf_event_paranoid")?;
        let perf_event_paranoid = perf_event_paranoid.trim().parse::<i32>()?;
        if perf_event_paranoid != -1 {
            self.warn(format!(
                "/proc/sys/kernel/perf_event_paranoid is {}. This may cause permission issues when reading performance counters.",
                perf_event_paranoid
            ));
        }
        Ok(())
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
            self.warn("Git worktree is dirty.");
        }
        Ok(())
    }

    fn check_common(&mut self) -> anyhow::Result<()> {
        self.check_dirty_git_worktree()?;
        self.check_bench_configs()?;
        self.check_build_configs()?;
        Ok(())
    }

    #[cfg(target_os = "linux")]
    fn check(&mut self) -> anyhow::Result<()> {
        self.check_common()?;
        self.check_perf_event()?;
        // Check if the current user is the only one logged in
        let sys = &self.run.system;
        if sys.users.len() > 1 {
            let msg = format!(
                "More than one user logged in: {}",
                sys.users
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
        if !sys.scaling_governor.iter().all(|g| g == "performance") {
            let sg = sys.scaling_governor.clone();
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
        self.check_common()?;
        Ok(())
    }
}

pub fn check(args: &RunArgs, run: &RunInfo) -> anyhow::Result<()> {
    let mut checker = PreBenchmarkingChecker::new(
        run,
        args.allow_dirty,
        args.allow_multiple_users,
        args.allow_any_scaling_governor,
    );
    checker.check()?;
    super::dump_warnings("WARNINGS", &checker.warnings);
    Ok(())
}
