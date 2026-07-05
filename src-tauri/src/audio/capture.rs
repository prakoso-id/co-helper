// M2+: CPAL mic + system-loopback capture → MPSC channel of AudioFrame
use std::time::Instant;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{SampleFormat, Stream, StreamConfig};
use tokio::sync::mpsc;

/// Safe wrapper: cpal::Stream is !Send on some platforms (contains *mut ()),
/// but we only ever drop it on the thread that created it. The wrapper
/// asserts Send so it can live in Arc<Mutex<...>>.
pub(crate) struct SendStream(cpal::Stream);
unsafe impl Send for SendStream {}
impl SendStream {
    fn new(s: cpal::Stream) -> Self { Self(s) }
    fn into_inner(self) -> cpal::Stream { self.0 }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AudioSource {
    Mic,
    System,
}

#[derive(Clone, Debug)]
pub struct AudioFrame {
    pub source: AudioSource,
    pub samples: Vec<f32>,
    pub sample_rate: u32,
    pub channels: u16,
    pub timestamp: Instant,
}

fn mic_stream(src: AudioSource, tx: mpsc::UnboundedSender<AudioFrame>) -> Result<(Stream, StreamConfig), String> {
    let host = cpal::default_host();
    let dev = host.default_input_device().ok_or("no default input device")?;
    let cfg = dev.default_input_config().map_err(|e| e.to_string())?;
    if cfg.sample_format() != SampleFormat::F32 {
        return Err(format!("mic format {} != F32", cfg.sample_format()));
    }
    let scfg: StreamConfig = cfg.into();
    let sr = scfg.sample_rate.0;
    let ch = scfg.channels;
    let stream = dev
        .build_input_stream(
            &scfg,
            move |data: &[f32], _| {
                let _ = tx.send(AudioFrame {
                    source: src,
                    samples: data.to_vec(),
                    sample_rate: sr,
                    channels: ch,
                    timestamp: Instant::now(),
                });
            },
            |e| eprintln!("mic stream err: {e}"), None,
        )
        .map_err(|e| e.to_string())?;
    stream.play().map_err(|e| e.to_string())?;
    Ok((stream, scfg))
}

/// Linux: find input device whose name contains 'monitor' for loopback.
/// Windows: default_output_device + build_input_stream (WASAPI loopback pair).
/// ponytail: ASIO not hooked.
#[cfg(target_os = "linux")]
fn sys_stream(src: AudioSource, tx: mpsc::UnboundedSender<AudioFrame>) -> Result<(Stream, StreamConfig), String> {
    let host = cpal::default_host();
    let mut found = None;
    for dev in host.input_devices().ok().into_iter().flatten().filter_map(|d| {
        let n = d.name().ok()?;
        Some((d, n))
    }) {
        if dev.1.to_lowercase().contains("monitor") {
            found = Some(dev.0);
            break;
        }
    }
    let dev = found.ok_or("no Linux 'monitor' loopback input device")?;
    let cfg = dev.default_input_config().map_err(|e| e.to_string())?;
    if cfg.sample_format() != SampleFormat::F32 {
        return Err(format!("sys format {} != F32", cfg.sample_format()));
    }
    let scfg: StreamConfig = cfg.into();
    let sr = scfg.sample_rate.0;
    let ch = scfg.channels;
    let stream = dev
        .build_input_stream(
            &scfg,
            move |data: &[f32], _| {
                let _ = tx.send(AudioFrame {
                    source: src,
                    samples: data.to_vec(),
                    sample_rate: sr,
                    channels: ch,
                    timestamp: Instant::now(),
                });
            },
            |e| eprintln!("sys stream err: {e}"), None,
        )
        .map_err(|e| e.to_string())?;
    stream.play().map_err(|e| e.to_string())?;
    Ok((stream, scfg))
}

#[cfg(not(target_os = "linux"))]
fn sys_stream(src: AudioSource, tx: mpsc::UnboundedSender<AudioFrame>) -> Result<(Stream, StreamConfig), String> {
    let host = cpal::default_host();
    let dev = host.default_output_device().ok_or("no default output device")?;
    let cfg = dev.default_output_config().map_err(|e| e.to_string())?;
    if cfg.sample_format() != SampleFormat::F32 {
        return Err(format!("out format {} != F32", cfg.sample_format()));
    }
    let scfg: StreamConfig = cfg.into();
    let sr = scfg.sample_rate.0;
    let ch = scfg.channels;
    let stream = dev
        .build_input_stream(
            &scfg,
            move |data: &[f32], _| {
                let _ = tx.send(AudioFrame {
                    source: src,
                    samples: data.to_vec(),
                    sample_rate: sr,
                    channels: ch,
                    timestamp: Instant::now(),
                });
            },
            |e| eprintln!("sys stream err: {e}"), None,
        )
        .map_err(|e| e.to_string())?;
    stream.play().map_err(|e| e.to_string())?;
    Ok((stream, scfg))
}

/// Owned active capture. Drop = streams drop = halt.
pub struct ActiveCapture {
    pub streams: Vec<SendStream>,
}

pub fn start(sources: &[String]) -> Result<(ActiveCapture, mpsc::UnboundedReceiver<AudioFrame>), String> {
    let (tx, rx) = mpsc::unbounded_channel();
    let mut streams = Vec::new();
    let want_mic = sources.iter().any(|s| s.eq_ignore_ascii_case("mic"));
    let want_sys = sources.iter().any(|s| s.eq_ignore_ascii_case("system"));
    let mut errs = Vec::new();
    if want_mic {
        match mic_stream(AudioSource::Mic, tx.clone()) {
            Ok((s, _)) => streams.push(SendStream::new(s)),
            Err(e) => errs.push(format!("mic: {e}")),
        }
    }
    if want_sys {
        match sys_stream(AudioSource::System, tx) {
            Ok((s, _)) => streams.push(SendStream::new(s)),
            Err(e) => errs.push(format!("system: {e}")),
        }
    }
    if streams.is_empty() {
        return Err(errs.join("; "));
    }
    Ok((ActiveCapture { streams }, rx))
}

#[derive(Clone, Debug, serde::Serialize)]
pub struct AudioDevice {
    pub name: String,
    pub kind: String,
}

pub fn list_devices() -> Vec<AudioDevice> {
    let host = cpal::default_host();
    let mut out = Vec::new();
    for d in host.input_devices().ok().into_iter().flatten().filter_map(|d| {
        let n = d.name().ok()?;
        Some((d, n))
    }) {
        out.push(AudioDevice { name: d.1, kind: "input".into() });
    }
    for d in host.output_devices().ok().into_iter().flatten().filter_map(|d| {
        let n = d.name().ok()?;
        Some((d, n))
    }) {
        out.push(AudioDevice { name: d.1, kind: "output".into() });
    }
    out
}