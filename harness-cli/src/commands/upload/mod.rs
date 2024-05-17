use std::path::PathBuf;

use clap::Parser;
use reqwest::blocking::Client;
use serde_json::{Map, Value};

use crate::configs::run_info::CrateInfo;

/// Upload benchmark results to https://reports.harness.rs
#[derive(Parser)]
pub struct UploadResultsArgs {
    /// The run id to report. Default to the latest run.
    pub run_id: Option<String>,
}

const DOMAIN: &str = "http://localhost:8501";

impl UploadResultsArgs {
    fn find_log_dir(&self, crate_info: &CrateInfo) -> anyhow::Result<PathBuf> {
        let logs_dir = crate_info.target_dir.join("harness").join("logs");
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
        let crate_info = CrateInfo::load()?;
        let log_dir = self.find_log_dir(&crate_info)?;
        let results_csv = log_dir.join("results.csv");
        let config_toml = log_dir.join("config.toml");
        if !results_csv.exists() {
            anyhow::bail!("Benchmark results not found: {}", results_csv.display());
        }
        if !config_toml.exists() {
            anyhow::bail!("Config file not found: {}", config_toml.display());
        }

        let client = Client::new();
        let form = reqwest::blocking::multipart::Form::new()
            .file("files", results_csv)?
            .file("files", config_toml)?;
        let response = client
            .put(format!("{DOMAIN}/api/v1/upload-results"))
            .multipart(form)
            .send()?;
        let status = response.status();
        let Ok(res) = response.json::<Map<String, Value>>() else {
            anyhow::bail!("Failed to parse response");
        };
        if !status.is_success() {
            let msg = res
                .get("error")
                .map(|e| e.as_str())
                .flatten()
                .unwrap_or("Unknown error");
            anyhow::bail!("Failed to upload results: {} ({})", status, msg);
        }
        let Some(hash) = res.get("hash").map(|h| h.as_str()).flatten() else {
            anyhow::bail!("No upload hash returned");
        };
        if status.as_u16() != 201 {
            anyhow::bail!("Results already uploaded: {DOMAIN}/r/{hash}");
        }
        println!("Results uploaded: {DOMAIN}/r/{hash}");
        println!("Please claim the results by visiting the link above. Failure to claim the results within 7 days will result in automatic deletion.");
        Ok(())
    }
}
