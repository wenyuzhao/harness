use std::path::PathBuf;

use cargo_metadata::MetadataCommand;
use chrono::{DateTime, Utc};
use clap::Parser;

use crate::config::RunInfo;

pub(crate) mod data;

/// Analyze and report benchmark results
#[derive(Parser)]
pub struct ReportArgs {
    /// The run id to report. Default to the latest run.
    pub run_id: Option<String>,
    /// Normalize the results to a baseline build.
    #[clap(long, default_value = "false")]
    pub norm: bool,
    /// The baseline build name to normalize to.
    /// If not specified, the one specified in the profile will be used.
    #[clap(long)]
    pub baseline: Option<String>,
}

struct CrateInfo {
    name: String,
    target_dir: PathBuf,
}

impl ReportArgs {
    fn load_crate_info(&self) -> anyhow::Result<CrateInfo> {
        let Ok(meta) = MetadataCommand::new().manifest_path("./Cargo.toml").exec() else {
            anyhow::bail!("Failed to get metadata from ./Cargo.toml");
        };
        let target_dir = meta.target_directory.as_std_path();
        let Some(pkg) = meta.root_package() else {
            anyhow::bail!("No root package found");
        };
        Ok(CrateInfo {
            name: pkg.name.clone(),
            target_dir: target_dir.to_owned(),
        })
    }

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
        // Collect crate info and other metadata
        let crate_info = self.load_crate_info()?;
        let log_dir = self.find_log_dir(&crate_info)?;
        let config = RunInfo::load(&log_dir.join("config.toml"))?;
        let baseline = if self.norm {
            let b = self.baseline.clone().or(config.profile.baseline);
            if b.is_none() {
                anyhow::bail!("No baseline specified");
            }
            b
        } else {
            None
        };
        // Load benchmark result
        let results_csv = log_dir.join("results.csv");
        if !results_csv.exists() {
            anyhow::bail!("Benchmark results not found: {}", results_csv.display());
        }
        let raw_df = data::get_data(&results_csv)?;
        // Mean over all invocations, group by [bench, build]
        let (avg_df, ci_df) = data::mean_over_invocations(&raw_df)?;
        // Mean and geomean over all benchmarks, group by builds
        let summaries = data::per_metric_summary(&avg_df, baseline.as_deref())?;
        // let normed_summary_df = if let Some(baseline) = &self.baseline {
        //     Some(data::normalize(&summary_df, baseline)?)
        // } else {
        //     None
        // };
        // Print results
        let mut printer = crate::utils::md::MarkdownPrinter::new();
        printer.add(format!(
            "# [{}] Benchmark Results Summary\n\n",
            crate_info.name
        ));
        printer.add(format!("* Run ID: `{}`\n", config.runid));
        printer.add(format!(
            "* Start Time (UTC): `{}`\n",
            DateTime::<Utc>::from_timestamp(config.start_timestamp_utc, 0)
                .unwrap()
                .format("%Y-%m-%d %H:%M:%S")
        ));
        if let Some(t) = config.finish_timestamp_utc {
            printer.add(format!(
                "* Finish Time (UTC): `{}`\n",
                DateTime::<Utc>::from_timestamp(t, 0)
                    .unwrap()
                    .format("%Y-%m-%d %H:%M:%S")
            ));
        } else {
            printer.add("* Finish Time (UTC): `N/A`\n");
        }
        printer.add(format!("* OS: `{}`\n", config.system.os));
        printer.add(format!("* CPU: `{}`\n", config.system.cpu_model));
        printer.add(format!(
            "* Memory: `{} GB`\n",
            config.system.memory_size >> 30
        ));
        printer.add("\n## Mean Over All Invocations\n\n");
        printer.add_dataframe_with_ci(&avg_df, &ci_df);
        printer.add("\n## Summary\n");
        for summary in summaries {
            printer.add_metric_summary(&summary);
        }
        printer.dump();
        Ok(())
    }
}
