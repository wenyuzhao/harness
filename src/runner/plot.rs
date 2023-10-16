use cargo_metadata::MetadataCommand;
use polars::prelude::*;
use std::io;
use tui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    widgets::{Block, Borders, Widget},
    Terminal,
};

fn load_raw_data(_args: &crate::PlotArgs) -> anyhow::Result<DataFrame> {
    let Ok(meta) = MetadataCommand::new().manifest_path("./Cargo.toml").exec() else {
        anyhow::bail!("Failed to get metadata from ./Cargo.toml");
    };
    let target_dir = meta.target_directory.as_std_path();
    let latest_log_dir = target_dir.join("harness").join("logs").join("latest");
    let csv_file = latest_log_dir.join("results.csv");
    // let csv =
    let raw_data = CsvReader::from_path(csv_file)?.has_header(true).finish()?;
    Ok(raw_data)
}

fn get_aggregated_and_normalized_data(
    raw_data: DataFrame,
    args: &crate::PlotArgs,
) -> anyhow::Result<DataFrame> {
    let mean_over_invocation = raw_data
        .lazy()
        .select([col("*").exclude(["invocation"])])
        .group_by(["bench", "build"])
        .agg([col("*").mean()])
        .collect()?;
    let schema = Arc::new(mean_over_invocation.schema());
    println!("{}", mean_over_invocation);
    let mut data = mean_over_invocation;
    if let Some(baseline) = args.baseline.as_ref() {
        let b = baseline.clone();
        data = data
            .lazy()
            .group_by(["bench"])
            .apply(
                move |df| {
                    let index = df
                        .column("build")
                        .unwrap()
                        .iter()
                        .position(|v| v.get_str() == Some(&b))
                        .unwrap();
                    let df = df
                        .lazy()
                        .select([
                            col("bench"),
                            col("build"),
                            col("*").exclude(["bench", "build"])
                                / col("*")
                                    .exclude(["bench", "build"])
                                    .slice(lit(index as i64), 1),
                        ])
                        .collect()
                        .unwrap();
                    Ok(df)
                },
                schema,
            )
            .collect()?;
    }
    Ok(data)
}

pub fn harness_plot(args: &crate::PlotArgs) -> anyhow::Result<()> {
    let raw_data = load_raw_data(args)?;
    let data = get_aggregated_and_normalized_data(raw_data, args)?;
    println!("{}", data);
    let stdout = io::stdout();
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.draw(|f| {
        let size = f.size();
        let block = Block::default().title("Block").borders(Borders::ALL);
        f.render_widget(
            block,
            Rect {
                x: 32,
                y: 6,
                width: 100,
                height: 32,
            },
        );
    })?;
    Ok(())
}
