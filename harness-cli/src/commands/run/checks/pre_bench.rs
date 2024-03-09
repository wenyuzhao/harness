use std::cell::RefCell;

use colored::{Colorize, CustomColor};
use once_cell::sync::Lazy;

use crate::configs::run_info::RunInfo;

static BG: Lazy<CustomColor> = Lazy::new(|| CustomColor::new(0x23, 0x23, 0x23));

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
        self.check_changed("OS", &old.system.os, &new.system.os);
        self.check_changed("Arch", &old.system.arch, &new.system.arch);
        self.check_changed("Kernel", &old.system.kernel, &new.system.kernel);
        self.check_changed("CPU", &old.system.cpu_model, &new.system.cpu_model);
        self.check_changed_mem("Memory", old.system.memory_size, new.system.memory_size);
        self.check_changed_mem("Swap", old.system.swap_size, new.system.swap_size);
        self.check_changed("Rust Version", &old.system.rustc, &new.system.rustc);
        if old.system.env != new.system.env {
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
            for (k, v) in &new.system.env {
                if old.system.env.get(k) != Some(v) {
                    list_env(k, old.system.env.get(k).unwrap_or(&"".to_owned()), v);
                }
            }
            for (k, v) in &old.system.env {
                if !new.system.env.contains_key(k) {
                    list_env(k, v, "");
                }
            }
            self.warn(s.trim_end());
        }
        #[cfg(target_os = "linux")]
        if old.system.scaling_governor != new.system.scaling_governor {
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
                sg_summary(&old.system.scaling_governor),
                sg_summary(&new.system.scaling_governor),
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

pub fn check(old: &RunInfo, new: &RunInfo) -> anyhow::Result<()> {
    let mut checker = ReproducibilityChecker::new(old, new);
    checker.check()?;
    super::dump_warnings(
        "Reproducibility: Unmatched Environment",
        &checker.warnings.borrow(),
    );
    Ok(())
}
