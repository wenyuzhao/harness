use polars::prelude::*;
use std::path::PathBuf;

pub fn get_data(csv: &PathBuf) -> anyhow::Result<DataFrame> {
    Ok(CsvReader::from_path(csv)?.finish()?)
}

pub fn mean_over_invocations(df: &DataFrame) -> anyhow::Result<DataFrame> {
    Ok(df
        .clone()
        .lazy()
        .group_by(["bench", "build"])
        .agg([len(), col("time").mean()])
        .rename(["len"], ["invocations"])
        .sort_by_exprs([col("bench"), col("build")], [false, false], false, true)
        .collect()?)
}

pub fn geomean_over_benchmarks(df: &DataFrame) -> anyhow::Result<DataFrame> {
    Ok(df
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
        .collect()?)
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
