use std::time::Instant;

use harness::probe::{Counters, Probe, ProbeManager};

#[derive(Default)]
pub struct StatProbe {
    start: Option<Instant>,
}

impl Probe for StatProbe {
    fn harness_begin(&mut self) {
        self.start = Some(Instant::now());
    }

    fn harness_end(&mut self, counters: &mut Counters) {
        let elapsed = self.start.unwrap().elapsed().as_millis() as f32;
        counters.report("timex", elapsed);
    }
}

#[no_mangle]
pub extern "C" fn register_probe(probes: &mut ProbeManager) {
    probes.register(Box::new(StatProbe::default()));
}
