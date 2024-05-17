use std::process::{Command, Stdio};

use clap::Parser;

use crate::configs::run_info::CrateInfo;

/// Start local data visualization server.
#[derive(Parser)]
pub struct VizArgs {}

impl VizArgs {
    fn vizkit_exists() -> bool {
        let status = Command::new("vizkit")
            .arg("--help")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
        if let Ok(status) = status {
            status.success()
        } else {
            false
        }
    }
    pub fn run(&self) -> anyhow::Result<()> {
        if !Self::vizkit_exists() {
            anyhow::bail!(
                "vizkit is not installed. Please install it by running: pipx install vizkit"
            );
        }
        let crate_info = CrateInfo::load()?;
        Command::new("vizkit")
            .arg(crate_info.workspace_root)
            .spawn()?
            .wait()?;
        Ok(())
    }
}
