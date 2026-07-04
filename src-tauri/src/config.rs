use serde::{Serialize, Deserialize};
use std::path::PathBuf;

#[derive(Clone, Serialize, Deserialize)]
pub struct Config {
    pub llm: LlmConfig,
    pub audio: AudioConfig,
    pub vad: VadConfig,
    pub ui: UiConfig,
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

impl Default for Config {
    fn default() -> Self {
        Self {
            llm: LlmConfig {
                url: "http://localhost:20128".to_string(),
                model: "ocg/deepseek-v4".to_string(),
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
