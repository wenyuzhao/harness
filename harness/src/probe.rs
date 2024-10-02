use std::collections::HashMap;
use std::time::Duration;

use libloading::{Library, Symbol};
use serde::{de::DeserializeOwned, Deserialize, Serialize};

use crate::bencher::Value;

struct Counters {
    counters: Vec<(String, Value)>,
}

impl Counters {
    pub(crate) fn new(walltime: Duration) -> Self {
        Self {
            counters: vec![(
                "time".to_owned(),
                (walltime.as_micros() as f32 / 1000.0).into(),
            )],
        }
    }

    fn merge(&mut self, values: HashMap<String, Value>) {
        let mut values = values.iter().collect::<Vec<_>>();
        values.sort_by_key(|x| x.0.as_str());
        for (k, v) in values {
            self.counters.push((k.clone(), *v));
        }
    }

    fn get_value(&self, name: &str) -> Option<Value> {
        self.counters
            .iter()
            .find(|(k, _)| k == name)
            .map(|(_, v)| *v)
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

    fn report(&mut self) -> HashMap<String, Value> {
        HashMap::new()
    }

    fn deinit(&mut self) {}
}

pub struct ProbeManager {
    probes: Vec<Box<dyn Probe>>,
    counters: Counters,
    libraries: Vec<Library>,
}

impl ProbeManager {
    pub(crate) fn new() -> Self {
        Self {
            probes: vec![],
            counters: Counters::new(Duration::ZERO),
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
        for (i, probe) in self.probes.iter_mut().enumerate() {
            let args = probe_args[i].take().unwrap();
            probe.init(args);
        }
    }

    pub(crate) fn deinit(&mut self) {
        for probe in self.probes.iter_mut() {
            probe.deinit();
        }
    }

    pub(crate) fn begin(&mut self, benchmark: &str, iteration: usize, warmup: bool) {
        for probe in self.probes.iter_mut() {
            probe.begin(benchmark, iteration, warmup)
        }
    }

    pub(crate) fn end(
        &mut self,
        benchmark: &str,
        iteration: usize,
        warmup: bool,
        walltime: Duration,
    ) {
        // harness_end
        for probe in self.probes.iter_mut() {
            probe.end(benchmark, iteration, warmup)
        }
        // report values
        let mut counters = Counters::new(walltime);
        for probe in self.probes.iter_mut() {
            counters.merge(probe.report());
        }
        self.counters = counters;
    }

    pub(crate) fn get_value(&self, name: &str) -> Option<Value> {
        self.counters.get_value(name)
    }

    pub(crate) fn get_counter_values(&self, extra: Vec<(String, Value)>) -> HashMap<String, Value> {
        // Collect all stats
        let mut stats_map: HashMap<String, Value> = HashMap::new();
        for (name, value) in &self.counters.counters {
            stats_map.insert(name.clone(), *value);
        }
        for (name, value) in extra {
            stats_map.insert(name.clone(), value);
        }
        stats_map
    }
}
