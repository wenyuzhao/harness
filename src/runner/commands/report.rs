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

    fn fmt_value(&self, v: &AnyValue) -> (String, bool) {
        match v {
            AnyValue::Float32(v) => (format!("{:.3}", v), true),
            AnyValue::Float64(v) => (format!("{:.3}", v), true),
            AnyValue::UInt8(v) => (format!("{}", v), true),
            AnyValue::UInt16(v) => (format!("{}", v), true),
            AnyValue::UInt32(v) => (format!("{}", v), true),
            AnyValue::UInt64(v) => (format!("{}", v), true),
            AnyValue::Int8(v) => (format!("{}", v), true),
            AnyValue::Int16(v) => (format!("{}", v), true),
            AnyValue::Int32(v) => (format!("{}", v), true),
            AnyValue::Int64(v) => (format!("{}", v), true),
            AnyValue::Boolean(v) => (format!("{}", v), true),
            _ => {
                if let Some(v) = v.get_str() {
                    (v.to_string(), false)
                } else {
                    (format!("{:?}", v), true)
                }
            }
        }
    }

    fn df_to_markdown(&self, df: &DataFrame) -> String {
        // Collect cell strings by columns
        let mut cols = vec![];
        let mut col_align_r = vec![];
        for col in df.get_columns() {
            let mut c = vec![col.name().to_owned()];
            for i in 0..col.len() {
                let (v, align_right) = self.fmt_value(&col.get(i).unwrap());
                c.push(v);
                if i == 0 {
                    col_align_r.push(align_right);
                }
            }
            cols.push(c);
        }
        // Get each column's max width
        let mut col_widths = vec![];
        for col in &cols {
            col_widths.push(col.iter().map(|s| s.len()).max().unwrap());
        }
        // Update cols with padded strings
        let pad = |c: &str, n: usize| (0..n).map(|_| c).collect::<Vec<_>>().join("");
        for (j, col) in cols.iter_mut().enumerate() {
            for i in 0..col.len() {
                let s = col[i].clone();
                col[i] += &pad(" ", col_widths[j] - s.len());
            }
        }
        // Construct markdown table string, row by row
        let build_row = |values: Option<Vec<&str>>, align: bool| {
            let mid = if let Some(values) = values {
                values.join(" | ")
            } else if !align {
                (0..cols.len())
                    .map(|i| pad("-", col_widths[i]))
                    .collect::<Vec<_>>()
                    .join(" | ")
            } else {
                let mut s = "|".to_string();
                for (i, w) in col_widths.iter().enumerate() {
                    if !col_align_r[i] {
                        s += ":";
                    } else {
                        s += " ";
                    }
                    s += &pad("-", *w);
                    if col_align_r[i] {
                        s += ":";
                    } else {
                        s += " ";
                    }
                    s += "|";
                }
                return s + "\n";
            };
            "| ".to_string() + mid.as_str() + " |\n"
        };
        let rows = cols[0].len();
        let mut md = "".to_string();
        md += &build_row(None, false);
        for i in 0..rows {
            md += &build_row(Some(cols.iter().map(|c| c[i].as_str()).collect()), false);
            if i == 0 || i == rows - 1 {
                md += &build_row(None, i == 0);
            }
        }
        return md;
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
        // Mean over all invocations, group by [bench, build]
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
        skin.print_text("\n## Mean over all invocations\n\n");
        skin.print_text(&self.df_to_markdown(&bm_df));
        // Mean and geomean over all benchmarks, group by builds
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
        skin.print_text("\n## Overall\n\n");
        skin.print_text(&self.df_to_markdown(&overall_df));
        Ok(())
    }
}
