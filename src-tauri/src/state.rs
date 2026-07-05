use serde::{Serialize, Deserialize};
use tokio::task::JoinHandle;

#[derive(Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: String,
    pub content: String,
}

pub struct AppState {
    pub messages: Vec<Message>,
    pub abort_token: std::sync::Arc<tokio::sync::Notify>,
    pub listening: bool,
    // M2+: audio pipeline. Streams are Send but not Sync → Mutex.
    pub audio_task: tokio::sync::Mutex<Option<JoinHandle<()>>>,
    pub active_capture: tokio::sync::Mutex<Option<crate::audio::capture::ActiveCapture>>,
    // M4+: whisper engine (None in remote/disabled mode or if init fails)
    pub whisper: Option<std::sync::Arc<crate::stt::whisper::WhisperEngine>>,
    // M5: serializes whisper inference — single-threaded model.
    pub stt_busy: tokio::sync::Mutex<bool>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            messages: Vec::new(),
            abort_token: std::sync::Arc::new(tokio::sync::Notify::new()),
            listening: false,
            audio_task: tokio::sync::Mutex::new(None),
            active_capture: tokio::sync::Mutex::new(None),
            whisper: None,
            stt_busy: tokio::sync::Mutex::new(false),
        }
    }

    pub fn add_message(&mut self, role: &str, content: &str) {
        self.messages.push(Message {
            role: role.to_string(),
            content: content.to_string(),
        });
        if self.messages.len() > 20 {
            self.messages = self.messages.split_off(self.messages.len() - 20);
        }
    }

    pub fn get_context(&self, max: usize) -> Vec<Message> {
        let start = self.messages.len().saturating_sub(max);
        self.messages[start..].to_vec()
    }
}