use clap::{Parser, Subcommand};
use once_cell::sync::Lazy;

mod config;
mod harness;
mod meta;
mod platform_info;
mod plot;

#[derive(Parser)]
pub struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    Run(RunArgs),
    Plot(PlotArgs),
}

#[derive(Parser)]
pub struct RunArgs {
    #[arg(short = 'n', long)]
    /// Number of iterations
    pub iterations: Option<usize>,
    #[arg(short = 'i', long)]
    /// Number of invocations
    pub invocations: Option<usize>,
    #[arg(long, default_value = "default")]
    /// Benchmarking profile
    pub profile: String,
    #[arg(long, default_value = "false")]
    /// Allow dirty working directories
    pub allow_dirty: bool,
    #[arg(long, default_value = "false")]
    /// (Linux only) Allow benchmarking even when multiple users are logged in
    pub allow_multi_user: bool,
    /// (Linux only) Allow any scaling governor value, instead of only `performance`
    #[arg(long, default_value = "false")]
    pub allow_any_scaling_governor: bool,
}

#[derive(Parser)]
pub struct PlotArgs {
    pub y: String,
    #[arg(short = 'b', long)]
    pub baseline: Option<String>,
    #[arg(short = 'o', long)]
    pub output: Option<String>,
}

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

static RUN_ARGS: Lazy<&'static RunArgs> = Lazy::new(|| match &CMD_ARGS.command {
    Commands::Run(args) => args,
    _ => unreachable!(),
});

fn main() -> anyhow::Result<()> {
    match &CMD_ARGS.command {
        Commands::Run(args) => harness::harness_run(&args),
        Commands::Plot(args) => plot::harness_plot(&args),
    }
}
