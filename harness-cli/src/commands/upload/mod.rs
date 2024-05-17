use std::path::PathBuf;

use clap::Parser;
use reqwest::blocking::Client;
use serde_json::{Map, Value};

use crate::configs::run_info::{CrateInfo, RunInfo};

/// Upload benchmark results to https://r.harness.rs
#[derive(Parser)]
pub struct UploadResultsArgs {
    /// The run id to report. Default to the latest run.
    pub run_id: Option<String>,
    /// Host url of the harness server. Default to https://r.harness.rs
    #[clap(long)]
    pub remote: Option<String>,
}

impl UploadResultsArgs {
    fn find_log_dir(&self, target_dir: PathBuf) -> anyhow::Result<PathBuf> {
        let logs_dir = target_dir.join("harness").join("logs");
        let log_dir = if let Some(run_id) = &self.run_id {
            logs_dir.join(run_id)
        } else {
            logs_dir.join("latest")
        };
        if !log_dir.exists() {
            anyhow::bail!("Log dir not found: {}", log_dir.display());
        }
        Ok(log_dir)
    }

    pub fn run(&self) -> anyhow::Result<()> {
        let mut remote_url =
            url::Url::parse(self.remote.as_deref().unwrap_or("https://r.harness.rs"))?;
        if remote_url.scheme() != "https" && remote_url.scheme() != "http" {
            anyhow::bail!("Invalid URL: {}", remote_url);
        }
        let target_dir = CrateInfo::get_target_path()?;
        let log_dir = self.find_log_dir(target_dir)?;
        let results_csv = log_dir.join("results.csv");
        let config_toml = log_dir.join("config.toml");
        if !results_csv.exists() {
            anyhow::bail!("Benchmark results not found: {}", results_csv.display());
        }
        if !config_toml.exists() {
            anyhow::bail!("Config file not found: {}", config_toml.display());
        }
        let commit = RunInfo::load(&config_toml)?.commit;
        if commit.ends_with("-dirty") {
            anyhow::bail!("Cannot upload results with a dirty git worktree.");
        }

        let client = Client::new();
        let form = reqwest::blocking::multipart::Form::new()
            .file("files", results_csv)?
            .file("files", config_toml)?;
        let response = client
            .put(format!("{remote_url}api/v1/upload-results"))
            .multipart(form)
            .send()?;
        let status = response.status();
        let Ok(res) = response.json::<Map<String, Value>>() else {
            anyhow::bail!("Failed to parse response");
        };
        if !status.is_success() {
            let msg = res
                .get("error")
                .and_then(|e| e.as_str())
                .unwrap_or("Unknown error");
            anyhow::bail!("Failed to upload results: {} ({})", status, msg);
        }
        let Some(hash) = res.get("hash").and_then(|h| h.as_str()) else {
            anyhow::bail!("No upload hash returned");
        };
        remote_url.set_query(Some(format!("id={hash}").as_str()));
        if status.as_u16() != 201 {
            anyhow::bail!("Results already uploaded: {remote_url}");
        }
        println!("Results uploaded: {remote_url}");
        println!("Please claim the results by visiting the link above. Failure to claim the results within 7 days will result in automatic deletion.");
        Ok(())
    }
}
