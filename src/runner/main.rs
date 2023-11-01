use cargo_metadata::MetadataCommand;
use clap::Parser;
use once_cell::sync::Lazy;

mod config;
mod harness;
mod meta;
mod platform_info;

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
    #[arg(long, default_value = "false")]
    /// (Linux only) Allow benchmarking even when multiple users are logged in
    pub allow_multi_user: bool,
    /// (Linux only) Allow any scaling governor value, instead of only `performance`
    #[arg(long, default_value = "false")]
    pub allow_any_scaling_governor: bool,
}

fn generate_runid(profile_name: &str) -> String {
    let time = chrono::Local::now()
        .format("%Y-%m-%d-%a-%H%M%S")
        .to_string();
    let host = crate::platform_info::PLATFORM_INFO.host.clone();
    let run_id = format!("{}-{}-{}", profile_name, host, time);
    run_id
}

static CMD_ARGS: Lazy<HarnessCmdArgs> = Lazy::new(|| {
    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "info")
    }
    let mut args = std::env::args().collect::<Vec<_>>();
    if args.len() > 1 && args[1] == "harness" {
        args = args[1..].to_vec();
    }
    let args = HarnessCmdArgs::parse_from(args);
    args
});

fn main() -> anyhow::Result<()> {
    let Ok(meta) = MetadataCommand::new().manifest_path("./Cargo.toml").exec() else {
        anyhow::bail!("Failed to get metadata from ./Cargo.toml");
    };
    let target_dir = meta.target_directory.as_std_path();
    let Some(pkg) = meta.root_package() else {
        anyhow::bail!("Could not find root package");
    };
    crate::platform_info::PLATFORM_INFO.pre_benchmarking_checks()?;
    let config = config::load_from_cargo_toml()?;
    let Some(mut profile) = config.profiles.get(&CMD_ARGS.profile).cloned() else {
        anyhow::bail!("Could not find harness profile `{}`", CMD_ARGS.profile);
    };
    // Overwrite invocations and iterations
    if let Some(invocations) = CMD_ARGS.invocations {
        profile.invocations = invocations;
    }
    if let Some(iterations) = CMD_ARGS.iterations {
        profile.iterations = iterations;
    }
    let run_id = generate_runid(&CMD_ARGS.profile);
    let log_dir = target_dir.join("harness").join("logs").join(&run_id);
    crate::meta::dump_global_metadata(&mut std::io::stdout(), &run_id, &profile, &log_dir)?;
    let mut harness = harness::Harness::new(pkg.name.clone(), profile);
    harness.run(&log_dir)?;
    Ok(())
}
