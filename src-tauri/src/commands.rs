use std::sync::Arc;
use tokio::sync::Mutex;
use tauri::{Manager, State, Emitter, AppHandle};
use crate::state::{AppState, Message};
use crate::llm::router_client;
use crate::config::Config;
use crate::audio::{capture, resample, vad};

#[tauri::command]
pub async fn start_capture(
    sources: Vec<String>,
    app: AppHandle,
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<(), String> {
    // stop any prior
    stop_capture_inner(&app, &state).await;

    let (active, rx) = capture::start(&sources).map_err(|e| {
        let _ = app.emit("error", serde_json::json!({"source":"audio","message":e}));
        e
    })?;
    {
        let s = &*state; let mut s = s.lock().await;
        s.listening = true;
        s.active_capture.lock().await.replace(active);
    }
    let _ = app.emit("vad_status", "listening");

    let app2 = app.clone();
    let state2 = state.inner().clone();
    let task = tokio::spawn(async move {
        run_pipeline(app2, state2, rx).await;
    });
    (*state).clone().lock().await.audio_task.lock().await.replace(task);
    Ok(())
}

async fn run_pipeline(app: AppHandle, state: Arc<Mutex<AppState>>, mut rx: tokio::sync::mpsc::UnboundedReceiver<capture::AudioFrame>) {
    let cfg = vad::VadConfig::default();
    let mut vad_mic = vad::Vad::new(&cfg).ok();
    let mut vad_sys = vad::Vad::new(&cfg).ok();

    while let Some(frame) = rx.recv().await {
        let mono = resample::to_mono(&frame.samples, frame.channels);
        let resampled = resample::resample_to_16k(&mono, frame.sample_rate);
        let vad = match frame.source {
            capture::AudioSource::Mic => &mut vad_mic,
            capture::AudioSource::System => &mut vad_sys,
        };
        let Some(vad) = vad.as_mut() else { continue };
        let segments = vad.push(&resampled);
        for seg in segments {
            let label = match frame.source {
                capture::AudioSource::Mic => "mic",
                capture::AudioSource::System => "system",
            };
            let _ = app.emit("transcript", serde_json::json!({
                "source": label,
                "samples": seg.len(),
                // M5 wires actual STT; emit raw count for now
            }));
            let _ = app.emit("vad_status", "processing");
            let _ = app.emit("vad_status", "listening");
        }
    }
    // drain: flush VADs
    for (src, v) in [("mic", vad_mic.as_mut()), ("system", vad_sys.as_mut())] {
        if let Some(v) = v {
            for _seg in v.flush() {
                let _ = app.emit("transcript", serde_json::json!({"source":src,"samples":0}));
            }
        }
    }
    let _ = app.emit("vad_status", "idle");
    let s = &*state; let mut s = s.lock().await;
    s.listening = false;
}

#[tauri::command]
pub async fn stop_capture(
    app: AppHandle,
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<(), String> {
    stop_capture_inner(&app, &state).await;
    Ok(())
}

async fn stop_capture_inner(app: &AppHandle, state: &State<'_, Arc<Mutex<AppState>>>) {
    // drop streams
    let s = &*state; let active = s.lock().await.active_capture.lock().await.take();
    drop(active);
    // abort task
    let s = &*state; let task = s.lock().await.audio_task.lock().await.take();
    if let Some(t) = task {
        t.abort();
    }
    let s = &*state; s.lock().await.listening = false;
    let _ = app.emit("vad_status", "idle");
}

#[tauri::command]
pub async fn send_to_llm(
    messages: Vec<Message>,
    app: AppHandle,
    state: State<'_, Arc<Mutex<AppState>>>,
    config: State<'_, Config>,
) -> Result<(), String> {
    let segment_id = uuid::Uuid::new_v4().to_string();
    let (full_messages, model, url) = {
        let s = &*state; let mut s = s.lock().await;
        let mut ctx = s.get_context(config.llm.max_context_messages);
        ctx.insert(0, Message {
            role: "system".to_string(),
            content: config.llm.system_prompt.clone(),
        });
        for m in &messages {
            s.add_message(&m.role, &m.content);
        }
        (ctx, config.llm.model.clone(), config.llm.url.clone())
    };
    let s = &*state; let abort_token = s.lock().await.abort_token.clone();
    let _ = app.emit("llm_start", serde_json::json!({"segmentId": segment_id}));
    tokio::spawn(async move {
        if let Err(e) = router_client::stream_chat(
            &url, &model, full_messages, &app, &segment_id, abort_token,
        ).await {
            let _ = app.emit("error", serde_json::json!({"source": "llm", "message": e}));
        } else {
            // stream_chat emits llm_end on success; save assistant message
            let st = app.state::<std::sync::Arc<Mutex<AppState>>>();
            let last = st.lock().await.messages.last().map(|m| m.content.clone()).unwrap_or_default();
            st.lock().await.add_message("assistant", &last);
        }
    });
    Ok(())
}

#[tauri::command]
pub async fn abort_llm(
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<(), String> {
    let s = &*state; let s = s.lock().await;
    s.abort_token.notify_waiters();
    Ok(())
}

pub use capture::AudioDevice;

#[tauri::command]
pub async fn list_audio_devices() -> Result<Vec<AudioDevice>, String> {
    Ok(capture::list_devices())
}

#[tauri::command]
pub async fn set_config(
    config: Config,
) -> Result<(), String> {
    config.save().map_err(|e| e.to_string())
}