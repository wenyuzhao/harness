use clap::{Parser, Subcommand};
use colored::Colorize;
use once_cell::sync::Lazy;

#[macro_use]
mod utils;
mod commands;
mod config;
mod meta;

/// Benchmark harness CLI
#[derive(Parser)]
pub struct Cli {
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
pub struct PlotArgs {}

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

fn main() -> anyhow::Result<()> {
    let result = match &CMD_ARGS.command {
        Commands::Run(cmd) => cmd.run(),
        Commands::Report(cmd) => cmd.run(),
    };
    if let Err(err) = result {
        eprintln!("❌ {}: {}", "ERROR".red().bold(), err.to_string().red());
        std::process::exit(1);
    }
    Ok(())
}
