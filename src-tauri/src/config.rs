use serde::{Serialize, Deserialize};
use std::path::PathBuf;

#[derive(Clone, Serialize, Deserialize)]
pub struct Config {
    pub llm: LlmConfig,
    pub audio: AudioConfig,
    pub vad: VadConfig,
    pub ui: UiConfig,
    #[serde(default)]
    pub hotkeys: HotkeysConfig,
    #[serde(default)]
    pub stt: SttConfig,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct LlmConfig {
    pub url: String,
    pub model: String,
    pub max_context_messages: usize,
    pub system_prompt: String,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct AudioConfig {
    pub mic_device: String,
    pub loopback_device: String,
    pub capture_mic: bool,
    pub capture_system: bool,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct VadConfig {
    pub positive_threshold: f32,
    pub negative_threshold: f32,
    pub chunk_size: usize,
    pub silence_ms: u64,
    pub max_segment_ms: u64,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct UiConfig {
    pub always_on_top: bool,
    pub hide_from_taskbar: bool,
    pub window_position: String,
    pub show_recording_indicator: bool,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct HotkeysConfig {
    pub toggle_window: String,
    pub toggle_listening: String,
    pub panic_hide: String,
}

impl Default for HotkeysConfig {
    fn default() -> Self {
        Self {
            toggle_window: "Ctrl+Shift+Space".to_string(),
            toggle_listening: "Ctrl+Shift+L".to_string(),
            panic_hide: "Escape".to_string(),
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct SttConfig {
    /// "in_process" (whisper.cpp) | "remote" (HTTP POST to remote_url) | "disabled"
    pub mode: String,
    /// Path to ggml .bin model, used when mode == "in_process"
    pub model_path: String,
    /// Base URL of remote whisper server, used when mode == "remote"
    pub remote_url: String,
    /// "auto" for ID+EN mix, or a code like "en"/"id"
    pub language: String,
    /// Hint to enable GPU backend; ignored if unsupported by build features
    pub gpu: bool,
}

impl Default for SttConfig {
    fn default() -> Self {
        Self {
            mode: "in_process".to_string(),
            model_path: String::new(),
            remote_url: String::new(),
            language: "auto".to_string(),
            gpu: true,
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            llm: LlmConfig {
                url: "http://localhost:20128".to_string(),
                model: "combos1".to_string(),
                max_context_messages: 10,
                system_prompt: "You are a helpful assistant listening in on a meeting. Keep answers concise. The meeting is in Indonesian and English.".to_string(),
            },
            audio: AudioConfig {
                mic_device: "default".to_string(),
                loopback_device: "default".to_string(),
                capture_mic: true,
                capture_system: true,
            },
            vad: VadConfig {
                positive_threshold: 0.8,
                negative_threshold: 0.35,
                chunk_size: 1024,
                silence_ms: 1000,
                max_segment_ms: 30000,
            },
            ui: UiConfig {
                always_on_top: false,
                hide_from_taskbar: true,
                window_position: "top-right".to_string(),
                show_recording_indicator: false,
            },
            hotkeys: HotkeysConfig::default(),
            stt: SttConfig::default(),
        }
    }
}

impl Config {
    pub fn load_or_default() -> Self {
        let path = Self::config_path();
        if path.exists() {
            match std::fs::read_to_string(&path) {
                Ok(content) => toml::from_str(&content).unwrap_or_default(),
                Err(_) => Self::default(),
            }
        } else {
            let config = Self::default();
            let _ = config.save();
            config
        }
    }

    pub fn save(&self) -> std::io::Result<()> {
        let path = Self::config_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let content = toml::to_string_pretty(self).unwrap();
        std::fs::write(path, content)
    }

    fn config_path() -> PathBuf {
        let dirs = directories::ProjectDirs::from("com", "prakosodev", "co-helper")
            .unwrap_or_else(|| {
                directories::ProjectDirs::from("com", "prakosodev", "co-helper")
                    .unwrap_or_else(|| panic!("Cannot determine config directory"))
            });
        dirs.config_dir().join("config.toml")
    }
}