use harness::probe::{Counters, Probe, ProbeManager};
use pfm::{PerfEvent, Perfmon};

#[derive(Default)]
pub struct PerfEventProbe {
    perfmon: Perfmon,
    events: Vec<PerfEvent>,
    event_names: Vec<String>,
}

impl Probe for PerfEventProbe {
    fn init(&mut self) {
        self.perfmon
            .initialize()
            .expect("Perfmon failed to initialize");
        let events = std::env::var("PERF_EVENTS").unwrap_or_default();
        self.event_names = events
            .split(',')
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .map(|s| s.to_owned())
            .collect();
        self.events = self
            .event_names
            .iter()
            .map(|s| PerfEvent::new(s, true).unwrap())
            .collect();
        for e in &mut self.events {
            e.open(0, -1).unwrap();
        }
    }

    fn harness_begin(&mut self) {
        for e in &mut self.events {
            e.reset().expect("Failed to reset perf evet");
            e.enable().expect("Failed to enable perf evet");
        }
    }

    fn harness_end(&mut self, counters: &mut Counters) {
        for e in &mut self.events {
            e.disable().expect("Failed to disable perf evet");
        }
        for (i, e) in self.events.iter().enumerate() {
            let v = e.read().unwrap().value as f32;
            counters.report(&self.event_names[i], v)
        }
    }
}

#[no_mangle]
pub extern "C" fn register_probe(probes: &mut ProbeManager) {
    probes.register(Box::new(PerfEventProbe::default()));
}
