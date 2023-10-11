use cargo_metadata::MetadataCommand;
use clap::Parser;

#[path = "../checks.rs"]
mod checks;
mod config;
mod harness;
mod meta;

#[derive(Parser, Debug)]
pub struct HarnessCmdArgs {
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

fn main() -> anyhow::Result<()> {
    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "info")
    }
    let mut args = std::env::args().collect::<Vec<_>>();
    if args.len() > 1 && args[1] == "harness" {
        args = args[1..].to_vec();
    }
    let args = HarnessCmdArgs::parse_from(args);
    let Ok(meta) = MetadataCommand::new().manifest_path("./Cargo.toml").exec() else {
        anyhow::bail!("Failed to get metadata from ./Cargo.toml");
    };
    let target_dir = meta.target_directory.as_std_path();
    let Some(pkg) = meta.root_package() else {
        anyhow::bail!("Could not find root package");
    };
    checks::pre_benchmarking_checks(args.allow_dirty)?;
    let config = config::load_from_cargo_toml()?;
    let Some(mut profile) = config.profiles.get(&args.profile).cloned() else {
        anyhow::bail!("Could not find harness profile `{}`", args.profile);
    };
    // Overwrite invocations and iterations
    if let Some(invocations) = args.invocations {
        profile.invocations = invocations;
    }
    if let Some(iterations) = args.iterations {
        profile.iterations = iterations;
    }
    crate::meta::dump_global_metadata(&mut std::io::stdout(), &profile)?;
    let time = chrono::Local::now().format("%Y%m%d-%H%M%S").to_string();
    let run_id = format!("{}-{}", args.profile, time);
    let mut harness = harness::Harness::new(run_id, pkg.name.clone(), profile);
    harness.run(target_dir, args.allow_dirty)?;
    Ok(())
}
