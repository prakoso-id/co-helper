use std::sync::Arc;
use tokio::sync::Mutex;
use tauri::{State, Emitter};
use crate::state::{AppState, Message};
use crate::llm::router_client;
use crate::config::Config;

#[tauri::command]
pub async fn start_capture(
    _sources: Vec<String>,
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<(), String> {
    {
        let mut s = state.lock().await;
        s.listening = true;
    }
    // M2+: Audio capture + VAD pipeline starts here
    Ok(())
}

#[tauri::command]
pub async fn stop_capture(
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<(), String> {
    {
        let mut s = state.lock().await;
        s.listening = false;
    }
    Ok(())
}

#[tauri::command]
pub async fn send_to_llm(
    messages: Vec<Message>,
    app: tauri::AppHandle,
    state: State<'_, Arc<Mutex<AppState>>>,
    config: State<'_, Config>,
) -> Result<(), String> {
    let segment_id = uuid::Uuid::new_v4().to_string();

    // Build full context: system prompt + history + new message
    let (full_messages, model, url) = {
        let mut s = state.lock().await;
        let mut ctx = s.get_context(config.llm.max_context_messages);
        ctx.insert(0, Message {
            role: "system".to_string(),
            content: config.llm.system_prompt.clone(),
        });
        ctx.push(Message {
            role: "user".to_string(),
            content: messages.last()
                .map(|m| m.content.clone())
                .unwrap_or_default(),
        });

        // Save user message
        s.add_message("user", &messages.last().map(|m| m.content.as_str()).unwrap_or(""));

        (ctx, config.llm.model.clone(), config.llm.url.clone())
    };

    let _ = app.emit("llm_start", serde_json::json!({ "segmentId": segment_id }));

    let abort = {
        let s = state.lock().await;
        s.abort_token.clone()
    };

    router_client::stream_chat(
        &url,
        &model,
        full_messages,
        &app,
        &segment_id,
        abort,
    ).await
}

#[tauri::command]
pub async fn abort_llm(
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<(), String> {
    let s = state.lock().await;
    s.abort_token.notify_waiters();
    Ok(())
}

#[derive(serde::Serialize)]
pub struct AudioDevice {
    pub name: String,
    pub kind: String,
}

#[tauri::command]
pub async fn list_audio_devices() -> Result<Vec<AudioDevice>, String> {
    // M2+: Enumerate via CPAL
    Ok(vec![])
}

#[tauri::command]
pub async fn set_config(
    config: Config,
) -> Result<(), String> {
    config.save().map_err(|e| e.to_string())
}