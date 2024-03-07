use clap::{Parser, Subcommand};
use colored::Colorize;
use git_info::types::GitInfo;
use once_cell::sync::Lazy;

#[macro_use]
mod utils;
mod commands;
mod config;
mod meta;

/// Benchmark harness CLI
#[derive(Parser)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    Run(commands::run::RunArgs),
    Report(commands::report::ReportArgs),
}

/// Plot benchmark results
#[derive(Parser)]
struct PlotArgs {}

static CMD_ARGS: Lazy<Cli> = Lazy::new(|| {
    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "info")
    }
    let mut args = std::env::args().collect::<Vec<_>>();
    if args.len() > 1 && args[1] == "harness" {
        args = args[1..].to_vec();
    }
    Cli::parse_from(args)
});

fn restore_git_state(prev: &GitInfo) {
    let curr = git_info::get();
    if prev.head.last_commit_hash != curr.head.last_commit_hash {
        let mut command = std::process::Command::new("git");
        command
            .arg("checkout")
            .arg(prev.head.last_commit_hash.as_ref().unwrap());
        if let Ok(status) = command.status() {
            if !status.success() {
                eprintln!(
                    "❌ {}: Failed to checkout to previous commit",
                    "ERROR".red().bold()
                );
                std::process::exit(1);
            }
        }
    }
}

#[doc(hidden)]
pub fn main() -> anyhow::Result<()> {
    let git = git_info::get();
    let result = match &CMD_ARGS.command {
        Commands::Run(cmd) => cmd.run(),
        Commands::Report(cmd) => cmd.run(),
    };
    if let Err(err) = result {
        eprintln!("❌ {}: {}", "ERROR".red().bold(), err.to_string().red());
        restore_git_state(&git);
        std::process::exit(1);
    }
    restore_git_state(&git);
    Ok(())
}
