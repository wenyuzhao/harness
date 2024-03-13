use clap::{Parser, Subcommand};
use colored::Colorize;
use once_cell::sync::Lazy;

mod commands;
mod utils;

pub mod configs;

/// The Precise and Reproducible Benchmarking Harness CLI
#[derive(Parser)]
pub struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Run(commands::run::RunArgs),
    Report(commands::report::ReportArgs),
}

/// Plot benchmark results
#[derive(Parser)]
struct PlotArgs {}

static CMD_ARGS: Lazy<Cli> = Lazy::new(|| {
    let mut args = std::env::args().collect::<Vec<_>>();
    if args.len() > 1 && args[1] == "harness" {
        args = args[1..].to_vec();
    }
    Cli::parse_from(args)
});

pub fn dump_backtrace(e: &anyhow::Error) {
    let env = std::env::var("RUST_BACKTRACE");
    if env.is_ok() && env != Ok("0".to_string()) {
        eprintln!("BACKTRACE:");
        eprintln!("{}", e.backtrace());
    }
}

#[doc(hidden)]
pub fn main() -> anyhow::Result<()> {
    let args = &*CMD_ARGS;
    let result = entey(args);
    if result.is_err() {
        std::process::exit(1);
    }
    Ok(())
}

#[doc(hidden)]
pub fn entey(args: &Cli) -> anyhow::Result<()> {
    let git = git_info2::get();
    let run_result = match &args.command {
        Commands::Run(cmd) => cmd.run(),
        Commands::Report(cmd) => cmd.run(),
    };
    if let Err(err) = run_result.as_ref() {
        eprintln!("❌ {}: {}", "ERROR".red().bold(), err.to_string().red());
        dump_backtrace(err);
    }
    let restore_result = utils::git::restore_git_state(&git);
    if let Err(err) = restore_result.as_ref() {
        eprintln!("❌ {}: {}", "ERROR".red().bold(), err.to_string().red());
        dump_backtrace(err);
    }
    Ok(())
}
