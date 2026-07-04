// M2+: rubato resampler wrapper
// Stub for M1

pub fn resample_to_16k(samples: &[f32], orig_rate: u32) -> Vec<f32> {
    if orig_rate == 16000 {
        return samples.to_vec();
    }
    let ratio = 16000.0 / orig_rate as f32;
    let out_len = (samples.len() as f32 * ratio) as usize;
    let mut out = Vec::with_capacity(out_len);
    for i in 0..out_len {
        let src_idx = (i as f32 / ratio) as usize;
        out.push(samples[src_idx.min(samples.len() - 1)]);
    }
    out
}
