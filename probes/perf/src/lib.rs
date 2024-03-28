#[cfg(target_os = "linux")]
use std::collections::HashMap;

use harness::probe::Probe;
#[cfg(target_os = "linux")]
use harness::probe::ProbeArgs;

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
    /// Initialize the probe before benchmarking.
    fn init(&mut self, args: ProbeArgs) {
        self.perfmon.initialize().expect("libpfm init failed.");
        let events = args.get::<String>("events").unwrap_or_default();
        let inherit = args.get::<bool>("inherit").unwrap_or_default();
        self.event_names = events
            .split(',')
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .map(|s| s.to_owned())
            .collect();
        self.events = self
            .event_names
            .iter()
            .map(|s| pfm::PerfEvent::new(s, inherit).unwrap())
            .collect();
        for e in &mut self.events {
            e.open(0, -1).unwrap();
        }
    }

    /// Prepare recording at the start of the timing iteration.
    fn begin(&mut self, _benchmark: &str, _iteration: usize, warmup: bool) {
        if !warmup {
            for e in &mut self.events {
                e.reset().expect("Failed to reset perf event");
                e.enable().expect("Failed to enable perf event");
            }
        }
    }

    /// Finish timing iteration. Disable recording.
    fn end(&mut self, _benchmark: &str, _iteration: usize, warmup: bool) {
        if !warmup {
            for e in &mut self.events {
                e.disable().expect("Failed to disable perf event");
            }
        }
    }

    /// Report data after the timing iteration.
    fn report(&mut self) -> HashMap<String, f32> {
        let mut values = HashMap::new();
        for (i, e) in self.events.iter().enumerate() {
            let v = e.read().unwrap().value as f32;
            values.insert(self.event_names[i].clone(), v);
        }
        values
    }
}
