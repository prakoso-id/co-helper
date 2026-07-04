// M4+: whisper-rs integration
// Stub for M1

pub struct WhisperEngine {
    // M4+: WhisperContext singleton
}

impl WhisperEngine {
    pub fn new() -> Self {
        Self {}
    }

    pub fn transcribe(&self, _samples: &[f32]) -> Result<String, String> {
        Err("STT not implemented yet (M4)".to_string())
    }
}
