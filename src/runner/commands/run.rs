use cargo_metadata::MetadataCommand;
use clap::Parser;

use crate::config;

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

impl RunArgs {
    fn generate_runid(&self) -> String {
        let time = chrono::Local::now()
            .format("%Y-%m-%d-%a-%H%M%S")
            .to_string();
        let host = crate::platform_info::PLATFORM_INFO.host.clone();
        let run_id = format!("{}-{}-{}", self.profile, host, time);
        run_id
    }

    pub fn run(&self) -> anyhow::Result<()> {
        let Ok(meta) = MetadataCommand::new().manifest_path("./Cargo.toml").exec() else {
            anyhow::bail!("Failed to get metadata from ./Cargo.toml");
        };
        let target_dir = meta.target_directory.as_std_path();
        let Some(pkg) = meta.root_package() else {
            anyhow::bail!("Could not find root package");
        };
        crate::platform_info::PLATFORM_INFO.pre_benchmarking_checks(self)?;
        let config = config::load_from_cargo_toml()?;
        let Some(mut profile) = config.profiles.get(&self.profile).cloned() else {
            anyhow::bail!("Could not find harness profile `{}`", self.profile);
        };
        // Overwrite invocations and iterations
        if let Some(invocations) = self.invocations {
            profile.invocations = invocations;
        }
        if let Some(iterations) = self.iterations {
            profile.iterations = iterations;
        }
        let run_id = self.generate_runid();
        let log_dir = target_dir.join("harness").join("logs").join(&run_id);
        let latest_log_dir = target_dir.join("harness").join("logs").join("latest");
        std::fs::create_dir_all(&log_dir)?;
        if latest_log_dir.exists() {
            if latest_log_dir.is_dir() && !latest_log_dir.is_symlink() {
                std::fs::remove_dir(&latest_log_dir)?;
            } else {
                std::fs::remove_file(&latest_log_dir)?;
            }
        }
        #[cfg(target_os = "windows")]
        std::os::windows::fs::symlink_dir(&log_dir, latest_log_dir)?;
        #[cfg(not(target_os = "windows"))]
        std::os::unix::fs::symlink(&log_dir, latest_log_dir)?;
        crate::meta::dump_global_metadata(&mut std::io::stdout(), &run_id, &profile, &log_dir)?;
        let mut harness = crate::harness::Harness::new(pkg.name.clone(), profile);
        harness.run(&log_dir)?;
        Ok(())
    }
}
