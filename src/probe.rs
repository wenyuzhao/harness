use std::{collections::HashMap, time::Instant};

use libloading::{Library, Symbol};

#[derive(Default)]
pub struct Counters {
    counters: HashMap<&'static str, f32>,
}

impl Counters {
    pub fn report(&mut self, name: &'static str, value: f32) {
        self.counters.insert(name, value);
    }
}

pub trait Probe {
    fn init(&mut self) {}

    fn harness_begin(&mut self) {}

    fn harness_end(&mut self, _counters: &mut Counters) {}
}

#[derive(Default)]
struct BaseProbe {
    start: Option<std::time::Instant>,
}

impl Probe for BaseProbe {
    fn harness_begin(&mut self) {
        self.start = Some(Instant::now());
    }

    fn harness_end(&mut self, counters: &mut Counters) {
        let elapsed = self.start.unwrap().elapsed().as_millis() as f32;
        counters.report("time", elapsed);
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
        let probes = probes
            .split(',')
            .map(|s| s.trim())
            .filter(|s| !s.is_empty());
        for probe in probes {
            unsafe {
                let filename = if cfg!(target_os = "macos") {
                    format!("lib{probe}.dylib")
                } else if cfg!(target_os = "linux") {
                    format!("lib{probe}.so")
                } else {
                    unimplemented!()
                };
                let lib = Library::new(filename).unwrap();
                let register_probe_fn: Symbol<extern "C" fn(probes: &mut ProbeManager)> =
                    lib.get(b"register_probe").unwrap();
                register_probe_fn(self);
                self.libraries.push(lib);
            }
        }
        self.base_probe.init();
        for probe in self.probes.iter_mut() {
            probe.init();
        }
    }

    pub(crate) fn harness_begin(&mut self) {
        self.base_probe.harness_begin();
        for probe in self.probes.iter_mut() {
            probe.harness_begin();
        }
    }

    pub(crate) fn harness_end(&mut self) {
        let mut counters = Counters::default();
        self.base_probe.harness_end(&mut counters);
        for probe in self.probes.iter_mut() {
            probe.harness_end(&mut counters);
        }
        self.counters = counters;
    }

    pub(crate) fn dump_counters(&mut self) {
        eprintln!(
            "============================ Harness Statistics Totals ============================"
        );
        for (name, _value) in &self.counters.counters {
            eprint!("{}\t", name);
        }
        eprintln!();
        for (_name, value) in &self.counters.counters {
            eprint!("{}\t", value);
        }
        eprintln!();
        eprintln!(
            "------------------------------ End Harness Statistics -----------------------------"
        );
    }
}
