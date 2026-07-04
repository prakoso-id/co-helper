// M2+: CPAL mic + loopback capture
// Stub for M1

#[derive(Clone, Debug)]
pub enum AudioSource {
    Mic,
    System,
}

#[derive(Clone, Debug)]
pub struct AudioFrame {
    pub source: AudioSource,
    pub samples: Vec<f32>,
    pub timestamp: std::time::Instant,
}
