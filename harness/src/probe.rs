use std::io::Write;
use std::time::Duration;
use std::{collections::HashMap, fs::OpenOptions, path::PathBuf, time::Instant};

use libloading::{Library, Symbol};
use serde::{de::DeserializeOwned, Deserialize, Serialize};

use crate::bencher::{StatPrintFormat, Value};

#[derive(Default)]
struct Counters {
    counters: Vec<(String, f32)>,
}

impl Counters {
    fn merge(&mut self, values: HashMap<String, f32>) {
        let mut values = values.iter().collect::<Vec<_>>();
        values.sort_by_key(|x| x.0.as_str());
        for (k, v) in values {
            self.counters.push((k.clone(), *v));
        }
    }
}

#[derive(Default, Clone, Debug, Serialize, Deserialize)]
pub struct ProbeArgs {
    #[serde(flatten)]
    raw: HashMap<String, serde_json::Value>,
}

impl ProbeArgs {
    pub fn get<T: DeserializeOwned>(&self, key: &str) -> anyhow::Result<T> {
        let value = self
            .raw
            .get(key)
            .ok_or_else(|| anyhow::anyhow!("Key not found: {}", key))?;
        Ok(serde_json::from_value(value.clone())?)
    }
}

#[allow(unused)]
pub trait Probe {
    fn init(&mut self, args: ProbeArgs) {}

    fn begin(&mut self, benchmark: &str, iteration: usize, warmup: bool) {}

    fn end(&mut self, benchmark: &str, iteration: usize, warmup: bool) {}

    fn report(&mut self) -> HashMap<String, f32> {
        HashMap::new()
    }

    fn deinit(&mut self) {}
}

#[derive(Default)]
struct BaseProbe {
    start: Option<std::time::Instant>,
    elapsed: Duration,
}

impl Probe for BaseProbe {
    fn begin(&mut self, _benchmark: &str, _iteration: usize, _warmup: bool) {
        self.start = Some(Instant::now());
    }

    fn end(&mut self, _benchmark: &str, _iteration: usize, _warmup: bool) {
        self.elapsed = self.start.unwrap().elapsed();
    }

    fn report(&mut self) -> HashMap<String, f32> {
        let mut values = HashMap::new();
        values.insert("time".to_owned(), self.elapsed.as_micros() as f32 / 1000.0);
        values
    }
}

pub struct ProbeManager {
    base_probe: BaseProbe,
    probes: Vec<Box<dyn Probe>>,
    counters: Counters,
    libraries: Vec<Library>,
}

impl ProbeManager {
    pub(crate) fn new() -> Self {
        Self {
            base_probe: BaseProbe::default(),
            probes: vec![],
            counters: Counters::default(),
            libraries: vec![],
        }
    }

    pub fn register(&mut self, probe: Box<dyn Probe>) {
        self.probes.push(probe);
    }

    pub(crate) fn init(&mut self, probes: &str) {
        let probes = if probes.trim().is_empty() {
            HashMap::new()
        } else {
            serde_json::from_str::<HashMap<String, ProbeArgs>>(probes).unwrap()
        };
        let mut probe_args = vec![];
        for (probe, args) in probes {
            let dylib_name = probe.replace('-', "_");
            let dylib_filename = if cfg!(target_os = "macos") {
                format!("lib{dylib_name}.dylib")
            } else if cfg!(target_os = "linux") {
                format!("lib{dylib_name}.so")
            } else {
                unimplemented!()
            };
            unsafe {
                let lib = Library::new(dylib_filename).unwrap();
                // This will call `ProbeManager::register` to add the probe to the list of probes
                let register_probe_fn: Symbol<extern "C" fn(probes: &mut ProbeManager)> =
                    lib.get(b"harness_register_probe").unwrap();
                register_probe_fn(self);
                self.libraries.push(lib);
            }
            probe_args.push(Some(args));
        }
        self.base_probe.init(ProbeArgs::default());
        for (i, probe) in self.probes.iter_mut().enumerate() {
            let args = probe_args[i].take().unwrap();
            probe.init(args);
        }
    }

    pub(crate) fn deinit(&mut self) {
        self.base_probe.deinit();
        for probe in self.probes.iter_mut() {
            probe.deinit();
        }
    }

    pub(crate) fn begin(&mut self, benchmark: &str, iteration: usize, warmup: bool) {
        self.base_probe.begin(benchmark, iteration, warmup);
        for probe in self.probes.iter_mut() {
            probe.begin(benchmark, iteration, warmup)
        }
    }

    pub(crate) fn end(&mut self, benchmark: &str, iteration: usize, warmup: bool) {
        // harness_end
        self.base_probe.end(benchmark, iteration, warmup);
        for probe in self.probes.iter_mut() {
            probe.end(benchmark, iteration, warmup)
        }
        // report values
        let mut counters = Counters::default();
        counters.merge(self.base_probe.report());
        for probe in self.probes.iter_mut() {
            counters.merge(probe.report());
        }
        self.counters = counters;
    }

    fn dump_counters_stderr_table(&self, stats: &[(String, Box<dyn Value>)]) {
        for (name, _) in stats {
            eprint!("{}\t", name);
        }
        eprintln!();
        for (_, value) in stats {
            eprint!("{}\t", value.to_string());
        }
        eprintln!();
    }

    fn dump_counters_stderr_yaml(&self, stats: &[(String, Box<dyn Value>)]) {
        for (name, value) in stats {
            eprintln!("{}: {}", name, value.to_string());
        }
    }

    fn dump_counters_stderr(&self, stats: &[(String, Box<dyn Value>)], format: StatPrintFormat) {
        let force_table = std::env::var("HARNESS_LOG_STAT_FORMAT") == Ok("table".to_owned());
        if force_table {
            return self.dump_counters_stderr_table(stats);
        }
        match format {
            StatPrintFormat::Table => self.dump_counters_stderr_table(stats),
            StatPrintFormat::Yaml => self.dump_counters_stderr_yaml(stats),
        }
    }

    fn dump_counters_csv(
        &self,
        name: &str,
        csv: Option<&PathBuf>,
        invocation: Option<usize>,
        build: Option<&String>,
        stats: &[(String, Box<dyn Value>)],
    ) {
        if let Some(csv) = csv {
            if !csv.exists() {
                let mut headers = "bench,build,invocation".to_owned();
                for (name, _value) in stats {
                    headers += ",";
                    headers += name;
                }
                headers += "\n";
                std::fs::write(csv, headers).unwrap();
            }
            let mut record = format!("{},{},{}", name, build.unwrap(), invocation.unwrap_or(0));
            for (_, value) in stats {
                record += &format!(",{}", value.to_string());
            }
            let mut csv = OpenOptions::new().append(true).open(csv).unwrap();
            writeln!(csv, "{record}").unwrap();
        }
    }

    pub(crate) fn dump_counters(
        &self,
        name: &str,
        csv: Option<&PathBuf>,
        invocation: Option<usize>,
        build: Option<&String>,
        extra_stats: Vec<(String, Box<dyn Value>)>,
        format: StatPrintFormat,
    ) {
        // Collect all stats
        let mut stats: Vec<(String, Box<dyn Value>)> = vec![];
        for (name, value) in &self.counters.counters {
            stats.push((name.clone(), Box::new(*value)));
        }
        for (name, value) in extra_stats {
            stats.push((name.clone(), value));
        }
        // Print to the log file
        let banner_start = std::env::var("HARNESS_LOG_STAT_BANNER_START").unwrap_or_else(|_| {
            "============================ Harness Statistics Totals ============================".to_string()
        });
        eprintln!("{banner_start}");
        self.dump_counters_stderr(&stats, format);
        let banner_end = std::env::var("HARNESS_LOG_STAT_BANNER_END").unwrap_or_else(|_| {
            "------------------------------ End Harness Statistics -----------------------------".to_string()
        });
        eprintln!("{banner_end}");
        // Print to the CSV file
        self.dump_counters_csv(name, csv, invocation, build, &stats);
    }
}
