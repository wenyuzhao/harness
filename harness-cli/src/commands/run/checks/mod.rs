use colored::Colorize;

use crate::configs::run_info::RunInfo;

use super::RunArgs;

mod pre_bench;
mod reproducibility;

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

pub fn run_all_checks(args: &RunArgs, run: &RunInfo, old: Option<&RunInfo>) -> anyhow::Result<()> {
    if let Some(old) = old {
        reproducibility::check(old, run)?;
    }
    pre_bench::check(args, run)?;
    Ok(())
}
