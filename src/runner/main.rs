use clap::{Parser, Subcommand};
use once_cell::sync::Lazy;

mod commands;
mod config;
mod meta;
mod platform_info;

#[derive(Parser)]
pub struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    Run(commands::run::RunArgs),
    Plot(PlotArgs),
}

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
    match &CMD_ARGS.command {
        Commands::Run(cmd) => cmd.run(),
        Commands::Plot(_args) => unimplemented!(),
    }
}
