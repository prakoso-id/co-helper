mod audio;
mod stt;
mod llm;
mod commands;
mod state;
mod config;

use std::sync::Arc;
use tokio::sync::Mutex;
use tauri::Manager;
use state::AppState;

pub fn run() {
    let state = Arc::new(Mutex::new(AppState::new()));

    tauri::Builder::default()
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .manage(state)
        .invoke_handler(tauri::generate_handler![
            commands::start_capture,
            commands::stop_capture,
            commands::send_to_llm,
            commands::abort_llm,
            commands::list_audio_devices,
            commands::set_config,
        ])
        .setup(|app| {
            // Load config on startup
            let config = config::Config::load_or_default();
            app.manage(config);
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running co-helper");
}
