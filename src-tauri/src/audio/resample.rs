// M2+: rubato resampler wrapper — FftFixedIn for quality
use rubato::{FftFixedIn, Resampler};

pub fn resample_to_16k(samples: &[f32], orig_rate: u32) -> Vec<f32> {
    if orig_rate == 16000 || samples.is_empty() {
        return samples.to_vec();
    }
    // ponytail: if multi-channel arrive interleaved, caller downsamples to mono before here.
    let in_chunks = vec![samples.to_vec()];
    let mut r = match FftFixedIn::<f32>::new(orig_rate as usize, 16000, 512, 1, 256) {
        Ok(r) => r,
        Err(_) => return samples.to_vec(), // fallback — keep audio rather than drop
    };
    match r.process(&in_chunks, None) {
        Ok(out) => out.into_iter().flatten().collect(),
        Err(_) => samples.to_vec(),
    }
}

/// Average stereo (interleaved) → mono. No-op for mono.
pub fn to_mono(samples: &[f32], channels: u16) -> Vec<f32> {
    let ch = channels as usize;
    if ch <= 1 {
        return samples.to_vec();
    }
    samples
        .chunks_exact(ch)
        .map(|frame| frame.iter().sum::<f32>() / ch as f32)
        .collect()
}