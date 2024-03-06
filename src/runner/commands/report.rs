use std::path::PathBuf;

use cargo_metadata::MetadataCommand;
use clap::Parser;
use polars::prelude::*;

/// Analyze and report benchmark results summary
#[derive(Parser)]
pub struct ReportArgs {
    /// The run id to report. Default to the latest run.
    pub run_id: Option<String>,
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
        let mut skin = termimad::MadSkin::default();
        for i in 0..8 {
            skin.headers[i].align = termimad::Alignment::Left;
            skin.headers[i].add_attr(termimad::crossterm::style::Attribute::Bold);
            skin.headers[i].set_fg(termimad::crossterm::style::Color::Blue);
        }
        skin.headers[0].set_bg(termimad::crossterm::style::Color::Blue);
        // Pre-benchmarking checks
        let crate_info = self.load_crate_info()?;
        let log_dir = self.find_log_dir(&crate_info)?;
        // Load benchmark result
        let results_csv = log_dir.join("results.csv");
        if !results_csv.exists() {
            anyhow::bail!("Benchmark results not found: {}", results_csv.display());
        }
        let raw_df = CsvReader::from_path(results_csv).unwrap().finish().unwrap();
        let bm_df = raw_df
            .clone()
            .lazy()
            .group_by(["bench", "build"])
            .agg([len(), col("time").mean()])
            .rename(["len"], ["invocations"])
            .sort_by_exprs([col("bench"), col("build")], [false, false], false, true)
            .collect()?;
        skin.print_text(&format!(
            "# [{}] Benchmark results summary",
            crate_info.name
        ));
        skin.print_text("\n## Mean over all invocations");
        println!("{}", bm_df);
        let overall_df = bm_df
            .clone()
            .lazy()
            .group_by(["build"])
            .agg([
                len().alias("benchmarks"),
                col("time").mean().alias("mean"),
                col("time")
                    .product()
                    .pow(lit(1.0f64) / len())
                    .alias("geomean"),
            ])
            .sort_by_exprs([col("build")], [false, false], false, true)
            .collect()?;
        skin.print_text("\n## Overall");
        println!("{}", overall_df);
        Ok(())
    }
}
