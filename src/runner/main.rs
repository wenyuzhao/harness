use clap::{Parser, Subcommand};

#[path = "../checks.rs"]
mod checks;
mod config;
mod harness;
mod meta;
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
}

#[derive(Parser)]
pub struct PlotArgs {
    pub y: String,
    #[arg(short = 'b', long)]
    pub baseline: Option<String>,
    #[arg(short = 'o', long)]
    pub output: Option<String>,
}

fn main() -> anyhow::Result<()> {
    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "info")
    }
    let mut args = std::env::args().collect::<Vec<_>>();
    if args.len() > 1 && args[1] == "harness" {
        args = args[1..].to_vec();
    }
    let cli = Cli::parse_from(args);
    match cli.command {
        Commands::Run(args) => harness::harness_run(&args),
        Commands::Plot(args) => plot::harness_plot(&args),
    }
}
