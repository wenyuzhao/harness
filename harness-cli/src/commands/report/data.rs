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

pub fn geomean_over_benchmarks(df: &DataFrame) -> anyhow::Result<Vec<(String, DataFrame)>> {
    let mut metrics = Vec::new();
    for c in df.get_columns() {
        if c.dtype().is_numeric() && c.name() != "invocations" {
            let df_metric = df
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
            metrics.push((c.name().to_owned(), df_metric));
        }
    }
    Ok(metrics)
}

pub fn normalize(df: &DataFrame, baseline: &str) -> anyhow::Result<DataFrame> {
    let row_index = df
        .column("build")?
        .iter()
        .position(|x| x.get_str() == Some(baseline))
        .unwrap();
    Ok(df
        .clone()
        .lazy()
        .with_column(col("mean") / (col("mean").slice(row_index as i32, 1).first()))
        .with_column(col("geomean") / (col("geomean").slice(row_index as i32, 1).first()))
        .collect()?)
}
