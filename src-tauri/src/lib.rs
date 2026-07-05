mod audio;
mod stt;
mod llm;
mod commands;
mod state;
mod config;

use std::sync::Arc;
use tokio::sync::Mutex;
use tauri::{
    menu::{MenuBuilder},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Emitter, Manager, WebviewWindow,
};
use tauri_plugin_global_shortcut::{
    Builder as ShortcutBuilder, Shortcut, ShortcutEvent,
};
use state::AppState;

/// Parsed hotkey ids used to dispatch the global shortcut handler.
struct HotkeyIds {
    toggle_window: u32,
    toggle_listening: u32,
    panic_hide: u32,
}

fn parse_shortcut(s: &str) -> Result<Shortcut, String> {
    s.parse::<Shortcut>()
        .map_err(|e| format!("invalid hotkey '{s}': {e}"))
}

fn toggle_window(window: &WebviewWindow) {
    match window.is_visible() {
        Ok(true) => {
            let _ = window.hide();
        }
        Ok(false) => {
            let _ = window.show();
            let _ = window.set_focus();
        }
        Err(_) => {}
    }
}

async fn panic_hide(app: &AppHandle, window: &WebviewWindow, state: &Arc<Mutex<AppState>>) {
    // Stop capture immediately
    {
        let mut s = state.lock().await;
        s.listening = false;
    }
    let _ = window.hide();
    let _ = app.emit("panic", ());
    let _ = app.emit("vad_status", "idle");
}

fn build_tray(app: &AppHandle) -> tauri::Result<()> {
    let menu = MenuBuilder::new(app)
        .text("toggle", "Show/Hide")
        .text("listen", "Start/Stop Listening")
        .separator()
        .text("quit", "Quit")
        .build()?;

    let icon = app.default_window_icon().cloned().unwrap_or_else(|| {
        // ponytail: Tauri 2 removed Image::from_path; embed icon at compile time.
        // ponytail: Tauri 2 removed Image::from_path; use default icon or empty
        app.default_window_icon().cloned().unwrap_or_else(|| {
            // 1x1 transparent pixel as fallback
            tauri::image::Image::new_owned(vec![0u8; 4], 1, 1)
        })
    });

    TrayIconBuilder::with_id("main")
        .tooltip("AudioSvc ¤ idle")
        .icon(icon)
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| {
            let id = event.id().as_ref();
            match id {
                "toggle" => {
                    if let Some(w) = app.get_webview_window("main") {
                        toggle_window(&w);
                    }
                }
                "listen" => {
                    let app = app.clone();
                    tauri::async_runtime::spawn(async move {
                        let state = app.state::<Arc<Mutex<AppState>>>().inner().clone();
                        let listening = {
                            let mut s = state.lock().await;
                            let was = s.listening;
                            s.listening = !was;
                            !was
                        };
                        let _ = app.emit("vad_status", if listening { "listening" } else { "idle" });
                    });
                }
                "quit" => {
                    app.exit(0);
                }
                _ => {}
            }
        })
        .on_tray_icon_event(|tray, event| {
            // ponytail: Linux tray click events unsupported in Tauri 2; left-click toggle won't fire on X11.
            // Menu items above remain the reliable path on Linux.
            if let TrayIconEvent::Click { button, button_state, .. } = event {
                if button == MouseButton::Left && button_state == MouseButtonState::Up {
                    let app = tray.app_handle().clone();
                    if let Some(w) = app.get_webview_window("main") {
                        toggle_window(&w);
                    }
                }
            }
        })
        .build(app)?;
    Ok(())
}

pub fn run() {
    let state = Arc::new(Mutex::new(AppState::new()));

    // Load config + parse hotkeys before building app (needs to live for handler closure).
    let config = config::Config::load_or_default();
    let ui_config = config.ui.clone();
    let hotkey_config = config.hotkeys.clone();

    let toggle_sc = parse_shortcut(&hotkey_config.toggle_window).expect("toggle_window hotkey invalid");
    let listen_sc = parse_shortcut(&hotkey_config.toggle_listening).expect("toggle_listening hotkey invalid");
    let panic_sc = parse_shortcut(&hotkey_config.panic_hide).expect("panic_hide hotkey invalid");

    let hk_ids = Arc::new(HotkeyIds {
        toggle_window: toggle_sc.id(),
        toggle_listening: listen_sc.id(),
        panic_hide: panic_sc.id(),
    });

    let state_for_hotkeys = state.clone();

    tauri::Builder::default()
        .plugin(
            ShortcutBuilder::new()
                .with_handler(move |_app: &AppHandle, shortcut: &Shortcut, event: ShortcutEvent| {
                    if event.state != tauri_plugin_global_shortcut::ShortcutState::Pressed {
                        return;
                    }
                    let id = shortcut.id();
                    let app = _app.clone();
                    let state = state_for_hotkeys.clone();
                    if id == hk_ids.toggle_window {
                        if let Some(w) = app.get_webview_window("main") {
                            toggle_window(&w);
                        }
                    } else if id == hk_ids.toggle_listening {
                        tauri::async_runtime::spawn(async move {
                            let listening = {
                                let mut s = state.lock().await;
                                let was = s.listening;
                                s.listening = !was;
                                !was
                            };
                            let _ = app.emit("vad_status", if listening { "listening" } else { "idle" });
                        });
                    } else if id == hk_ids.panic_hide {
                        let window = app.get_webview_window("main");
                        if let Some(w) = window {
                            tauri::async_runtime::spawn(async move {
                                panic_hide(&app, &w, &state).await;
                            });
                        }
                    }
                })
                .with_shortcut(toggle_sc.clone())
                .expect("register toggle_window")
                .with_shortcut(listen_sc.clone())
                .expect("register toggle_listening")
                .with_shortcut(panic_sc.clone())
                .expect("register panic_hide")
                .build(),
        )
        .manage(state)
        .manage(config)
        .setup(move |app| {
            // Apply stealth window flags from config.
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.set_skip_taskbar(ui_config.hide_from_taskbar);
                let _ = window.set_always_on_top(ui_config.always_on_top);
            }
            build_tray(app.handle())?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::start_capture,
            commands::stop_capture,
            commands::send_to_llm,
            commands::abort_llm,
            commands::list_audio_devices,
            commands::set_config,
        ])
        .run(tauri::generate_context!())
        .expect("error while running co-helper");
}