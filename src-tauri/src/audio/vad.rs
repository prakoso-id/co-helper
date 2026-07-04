// M2+: Silero VAD via voice_activity_detector crate
// Stub for M1

pub struct VadConfig {
    pub positive_threshold: f32,
    pub negative_threshold: f32,
    pub chunk_size: usize,
    pub silence_ms: u64,
}

impl Default for VadConfig {
    fn default() -> Self {
        Self {
            positive_threshold: 0.8,
            negative_threshold: 0.35,
            chunk_size: 1024,
            silence_ms: 1000,
        }
    }
}
