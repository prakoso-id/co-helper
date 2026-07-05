// M2+: Silero VAD via voice_activity_detector crate (v0.2 bundles silero_vad.onnx)
use voice_activity_detector::{IteratorExt, LabeledAudio, VoiceActivityDetector};

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
            // v0.2 forces 512 for 16k; kept for API compat
            chunk_size: 512,
            silence_ms: 1000,
        }
    }
}

/// Per-source VAD. Uses crate `label()` iterator for hysteresis + padding.
pub struct Vad {
    detector: VoiceActivityDetector,
    threshold: f32,
    padding_chunks: usize,
    buf: Vec<f32>,
    chunk_size: usize,
}

impl Vad {
    pub fn new(cfg: &VadConfig) -> Result<Self, String> {
        let detector = VoiceActivityDetector::builder()
            .sample_rate(16000)
            .chunk_size(cfg.chunk_size)
            .build()
            .map_err(|e| format!("vad init: {e}"))?;
        let padding_chunks = (cfg.silence_ms as usize * 16 / cfg.chunk_size).max(1);
        Ok(Self {
            detector,
            threshold: cfg.positive_threshold,
            padding_chunks,
            buf: Vec::with_capacity(cfg.chunk_size),
            chunk_size: cfg.chunk_size,
        })
    }

    /// Push 16k mono samples. Returns Speech segments completed in this call.
    pub fn push(&mut self, samples: &[f32]) -> Vec<Vec<f32>> {
        self.buf.extend_from_slice(samples);
        let mut segments = Vec::new();
        while self.buf.len() >= self.chunk_size {
            let chunk: Vec<f32> = self.buf.drain(..self.chunk_size).collect();
            let labeled = chunk
                .into_iter()
                .label(&mut self.detector, self.threshold, self.padding_chunks);
            for lab in labeled {
                if let LabeledAudio::Speech(s) = lab {
                    segments.push(s);
                }
            }
        }
        segments
    }

    /// Flush residual buffered audio at stop.
    pub fn flush(&mut self) -> Vec<Vec<f32>> {
        if self.buf.is_empty() {
            return Vec::new();
        }
        let remainder = std::mem::take(&mut self.buf);
        let labeled = remainder
            .into_iter()
            .label(&mut self.detector, self.threshold, self.padding_chunks);
        labeled
            .filter_map(|lab| match lab {
                LabeledAudio::Speech(s) => Some(s),
                _ => None,
            })
            .collect()
    }
}