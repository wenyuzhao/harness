use std::collections::HashMap;

use harness::probe::{Probe, ProbeManager};

#[harness::probe]
#[derive(Default)]
pub struct PerfEventProbe {
    #[cfg(target_os = "linux")]
    perfmon: pfm::Perfmon,
    #[cfg(target_os = "linux")]
    events: Vec<pfm::PerfEvent>,
    #[cfg(target_os = "linux")]
    event_names: Vec<String>,
}

#[cfg(not(target_os = "linux"))]
impl Probe for PerfEventProbe {}

#[cfg(target_os = "linux")]
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
            .map(|s| pfm::PerfEvent::new(s, true).unwrap())
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

    fn harness_end(&mut self) {
        for e in &mut self.events {
            e.disable().expect("Failed to disable perf evet");
        }
    }

    fn report_values(&mut self) -> HashMap<String, f32> {
        let mut values = HashMap::new();
        for (i, e) in self.events.iter().enumerate() {
            let v = e.read().unwrap().value as f32;
            values.insert(self.event_names[i].clone(), v);
        }
        values
    }
}
