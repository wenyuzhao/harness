use std::io::Write;
use std::{collections::HashMap, fs::OpenOptions, path::PathBuf};

use clap::ValueEnum;

use crate::Value;

#[derive(ValueEnum, Debug, Clone, Copy, PartialEq, Eq)]
#[clap(rename_all = "kebab_case")]
pub(crate) enum StatPrintFormat {
    Table,
    Yaml,
}

pub(crate) struct Record<'a> {
    pub name: &'a str,
    pub csv: Option<&'a PathBuf>,
    pub invocation: Option<usize>,
    pub build: Option<&'a String>,
    pub format: StatPrintFormat,
    pub iteration: usize,
    pub is_timing_iteration: bool,
    pub stats: HashMap<String, Value>,
}

impl<'a> Record<'a> {
    fn dump_counters_stderr_table(&self, stats: &[(String, Value)]) {
        for (name, _) in stats {
            eprint!("{}\t", name);
        }
        eprintln!();
        for (_, value) in stats {
            eprint!("{}\t", value.into_string());
        }
        eprintln!();
    }

    fn dump_counters_stderr_yaml(&self, stats: &[(String, Value)]) {
        for (name, value) in stats {
            eprintln!("{}: {}", name, value.into_string());
        }
    }

    fn dump_counters_stderr(&self, stats: &[(String, Value)], format: StatPrintFormat) {
        let force_table = std::env::var("HARNESS_LOG_STAT_FORMAT") == Ok("table".to_owned());
        if force_table {
            return self.dump_counters_stderr_table(stats);
        }
        match format {
            StatPrintFormat::Table => self.dump_counters_stderr_table(stats),
            StatPrintFormat::Yaml => self.dump_counters_stderr_yaml(stats),
        }
    }

    fn dump_counters_csv(&self, stats: &[(String, Value)]) {
        if let Some(csv) = self.csv {
            if !csv.exists() {
                let mut headers = "bench,build,invocation,iteration".to_owned();
                for (name, _value) in stats {
                    headers += ",";
                    headers += name;
                }
                headers += "\n";
                std::fs::write(csv, headers).unwrap();
            }
            let mut record = format!(
                "{},{},{},{}",
                self.name,
                self.build.unwrap(),
                self.invocation.unwrap_or(0),
                self.iteration
            );
            for (_, value) in stats {
                record += &format!(",{}", value.into_string());
            }
            let mut csv = OpenOptions::new().append(true).open(csv).unwrap();
            writeln!(csv, "{record}").unwrap();
        }
    }

    pub fn dump_values(mut self) {
        let mut stats_map = std::mem::take(&mut self.stats);
        let time = stats_map.remove("time");
        let mut stats: Vec<(String, Value)> = vec![];
        for (name, value) in stats_map {
            stats.push((name.clone(), value));
        }
        stats.sort_by_key(|x| x.0.clone());
        if let Some(time) = time {
            stats.insert(0, ("time".to_owned(), time));
        }
        if self.is_timing_iteration {
            // Print to the log file
            let banner_start = std::env::var("HARNESS_LOG_STAT_BANNER_START").unwrap_or_else(|_| {
                "============================ Harness Statistics Totals ============================".to_string()
            });
            eprintln!("{banner_start}");
            self.dump_counters_stderr(&stats, self.format);
            let banner_end = std::env::var("HARNESS_LOG_STAT_BANNER_END").unwrap_or_else(|_| {
                "------------------------------ End Harness Statistics -----------------------------".to_string()
            });
            eprintln!("{banner_end}");
        }
        // Print to the CSV file
        self.dump_counters_csv(&stats);
    }
}
