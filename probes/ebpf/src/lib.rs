use harness::probe::Probe;

#[harness::probe]
#[derive(Default)]
pub struct EBPFProbe {}

impl Probe for EBPFProbe {}
