use std::cell::RefCell;

use colored::{Colorize, CustomColor};
use once_cell::sync::Lazy;

use crate::meta::RunInfo;
#[cfg(target_os = "linux")]
use crate::meta::PLATFORM_INFO;

use super::runner::BenchRunner;

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
        self.check_common()?;
        Ok(())
    }
}

struct ReproducibilityChecker<'a, 'b> {
    warnings: RefCell<Vec<String>>,
    old: &'a RunInfo,
    new: &'b RunInfo,
}

impl<'a, 'b> ReproducibilityChecker<'a, 'b> {
    fn new(old: &'a RunInfo, new: &'b RunInfo) -> Self {
        Self {
            warnings: RefCell::new(Vec::new()),
            old,
            new,
        }
    }

    fn warn(&self, msg: impl AsRef<str>) {
        self.warnings.borrow_mut().push(msg.as_ref().to_owned());
    }

    fn warn_changed(&self, name: impl AsRef<str>, old: impl AsRef<str>, new: impl AsRef<str>) {
        self.warn(format!(
            "{}: {} ➔ {}",
            name.as_ref().bold(),
            old.as_ref().italic().on_custom_color(*BG),
            new.as_ref().italic().on_custom_color(*BG)
        ));
    }

    fn check_changed(&self, name: impl AsRef<str>, old: impl AsRef<str>, new: impl AsRef<str>) {
        if old.as_ref() != new.as_ref() {
            self.warn_changed(name, old, new);
        }
    }

    fn check_changed_mem(&self, name: impl AsRef<str>, old: usize, new: usize) {
        let to_gb = |x: usize| format!("{:.1}GB", x as f64 / 1024.0 / 1024.0);
        if old != new {
            self.warn_changed(name, to_gb(old), to_gb(new));
        }
    }

    fn check_changed_int(&self, name: impl AsRef<str>, old: usize, new: usize) {
        if old != new {
            self.warn_changed(name, format!("{}", old), format!("{}", new));
        }
    }

    fn check(&mut self) -> anyhow::Result<()> {
        let old = &self.old;
        let new = &self.new;
        self.check_changed("OS", &old.platform.os, &new.platform.os);
        self.check_changed("Arch", &old.platform.arch, &new.platform.arch);
        self.check_changed("Kernel", &old.platform.kernel, &new.platform.kernel);
        self.check_changed("CPU", &old.platform.cpu_model, &new.platform.cpu_model);
        self.check_changed_mem("Memory", old.platform.memory_size, new.platform.memory_size);
        self.check_changed_mem("Swap", old.platform.swap_size, new.platform.swap_size);
        self.check_changed("Rust Version", &old.platform.rustc, &new.platform.rustc);
        if old.platform.env != new.platform.env {
            let mut s = "Environment Variables Changed:\n".to_owned();
            let mut list_env = |name: &str, old: &str, new: &str| {
                s += &format!(
                    "   {} {}: {} {} {}\n",
                    "•".bright_red(),
                    name,
                    old.italic(),
                    "➔".bold(),
                    new.italic(),
                );
            };
            for (k, v) in &new.platform.env {
                if old.platform.env.get(k) != Some(v) {
                    list_env(k, old.platform.env.get(k).unwrap_or(&"".to_owned()), v);
                }
            }
            for (k, v) in &old.platform.env {
                if !new.platform.env.contains_key(k) {
                    list_env(k, v, "");
                }
            }
            self.warn(s.trim_end());
        }
        #[cfg(target_os = "linux")]
        if old.platform.scaling_governor != new.platform.scaling_governor {
            let sg_summary = |sg: &[String]| {
                let mut dedup = sg.to_vec();
                dedup.dedup();
                dedup
                    .iter()
                    .map(|x| (x, sg.iter().filter(|y| x == *y).count()))
                    .map(|(x, c)| format!("{} × {}", x, c))
                    .collect::<Vec<_>>()
                    .join(",")
            };
            self.warn_changed(
                "Scaling Governor",
                sg_summary(&old.platform.scaling_governor),
                sg_summary(&new.platform.scaling_governor),
            );
        }
        if old.profile.invocations != new.profile.invocations {
            self.check_changed_int(
                "Invocations",
                old.profile.invocations,
                new.profile.invocations,
            );
        }
        if old.profile.iterations != new.profile.iterations {
            self.check_changed_int("Iterations", old.profile.iterations, new.profile.iterations);
        }
        if old.commit.ends_with("-dirty") {
            self.warn(format!(
                "Profile commit {} is dirty. Uncommited changes may affect reproducibility.",
                old.commit.italic().on_custom_color(*BG)
            ));
        }
        Ok(())
    }
}

fn dump_warnings(title: &str, warnings: &[String]) {
    if warnings.is_empty() {
        return;
    }
    eprintln!("{}\n", title.bold().black().on_red());
    for msg in warnings {
        eprintln!("{} {}", "•".bright_red(), msg.red());
    }
    eprintln!();
}

impl super::RunArgs {
    pub(super) fn reproducibility_checks(
        &self,
        old: &RunInfo,
        new: &RunInfo,
    ) -> anyhow::Result<()> {
        let mut checker = ReproducibilityChecker::new(old, new);
        checker.check()?;
        dump_warnings(
            "Reproducibility: Unmatched Environment",
            &checker.warnings.borrow(),
        );
        Ok(())
    }

    pub(super) fn pre_benchmarking_checks(&self, run: &RunInfo) -> anyhow::Result<()> {
        let mut checker = PreBenchmarkingChecker::new(
            run,
            self.allow_dirty,
            self.allow_multiple_users,
            self.allow_any_scaling_governor,
        );
        checker.check()?;
        dump_warnings("WARNINGS", &checker.warnings);
        Ok(())
    }
}
