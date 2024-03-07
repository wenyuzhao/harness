use polars::prelude::*;
use std::path::PathBuf;

pub fn get_data(csv: &PathBuf) -> anyhow::Result<DataFrame> {
    Ok(CsvReader::from_path(csv)?.finish()?)
}

pub fn mean_over_invocations(df: &DataFrame) -> anyhow::Result<(DataFrame, DataFrame)> {
    let vals = || col("*").exclude(["bench", "build", "invocation"]);
    let avg = df
        .clone()
        .lazy()
        .group_by(["bench", "build"])
        .agg([len().alias("invocations"), vals().mean()])
        .sort_by_exprs([col("bench"), col("build")], [false, false], false, true)
        .collect()?;
    let ci = df
        .clone()
        .lazy()
        .group_by(["bench", "build"])
        .agg([(vals().std(0) / len().sqrt()) * lit(0.95)])
        .sort_by_exprs([col("bench"), col("build")], [false, false], false, true)
        .collect()?;
    Ok((avg, ci))
}

#[derive(Default)]
pub(crate) struct PerMetricSummary {
    pub name: String,
    pub unnormed: DataFrame,
    pub normed: Option<DataFrame>,
    pub min_names: Vec<String>,
    pub max_names: Vec<String>,
}

pub fn per_metric_summary(
    df: &DataFrame,
    baseline: Option<&str>,
) -> anyhow::Result<Vec<PerMetricSummary>> {
    let mut metrics = Vec::new();
    let norm_index = if let Some(baseline) = baseline {
        df.column("build")?
            .iter()
            .position(|x| x.get_str() == Some(baseline))
            .unwrap()
    } else {
        0
    };
    let df_build_col = df.column("build").unwrap();
    for c in df.get_columns() {
        if c.dtype().is_numeric() && c.name() != "invocations" {
            // min/max/mean/geomean
            let df_metric_unnormed = df
                .clone()
                .lazy()
                .group_by(["build"])
                .agg([
                    len().alias("benchmarks"),
                    col(c.name()).min().alias("min"),
                    col(c.name()).max().alias("max"),
                    col(c.name()).mean().alias("mean"),
                    col(c.name())
                        .product()
                        .pow(lit(1.0f64) / len())
                        .alias("geomean"),
                ])
                .sort_by_exprs([col("build")], [false, false], false, true)
                .collect()?;
            let mut summary = PerMetricSummary {
                name: c.name().to_owned(),
                unnormed: df_metric_unnormed.clone(),
                normed: None,
                min_names: vec![],
                max_names: vec![],
            };
            // min bench names
            let build_col = df_metric_unnormed.column("build")?;
            let df_metric_col = df.column(c.name()).unwrap();
            let find_min_max_bench_name = |row: usize, build: &str, label: &str| {
                let min_or_max = df_metric_unnormed.column(label).unwrap().get(row).unwrap();
                // Get bench name where [bench.min, bench.build] == [min, build]
                let bench_name = df
                    .column("bench")
                    .unwrap()
                    .iter()
                    .enumerate()
                    .find_map(|(i, bench)| {
                        let build_name_at_i =
                            df_build_col.get(i).unwrap().get_str().unwrap().to_owned();
                        let value_at_i = df_metric_col.get(i).unwrap();
                        if build_name_at_i == build && value_at_i == min_or_max {
                            Some(bench.get_str().unwrap().to_owned())
                        } else {
                            None
                        }
                    })
                    .unwrap();
                bench_name
            };
            for (i, b) in build_col.iter().enumerate() {
                let build = b.get_str().unwrap();
                let name = find_min_max_bench_name(i, build, "min");
                summary.min_names.push(name);
                let name = find_min_max_bench_name(i, build, "max");
                summary.max_names.push(name);
            }
            // Normalize to baseline
            if baseline.is_some() {
                let vals = || col("*").exclude(["build", "benchmarks"]);
                let df_metric_normed = df_metric_unnormed
                    .clone()
                    .lazy()
                    .with_column(vals() / (vals().slice(norm_index as i32, 1).first()))
                    .collect()?;
                summary.normed = Some(df_metric_normed);
            }
            metrics.push(summary);
        }
    }
    Ok(metrics)
}
