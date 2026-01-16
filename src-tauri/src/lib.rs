mod audio;
mod cleanup;
mod commands;
mod config;
mod input;
mod output;
mod transcribe;

use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use tauri::{
    image::Image,
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent},
    AppHandle, Emitter, Manager, RunEvent, WindowEvent,
};

use audio::AudioRecorder;
use cleanup::TextCleaner;
use commands::{get_config, list_audio_devices, save_config};
use config::Config;
use input::{InputEvent, InputMonitor};
use output::TextInserter;
use transcribe::ElevenLabsClient;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
pub enum AppStatus {
    Idle,
    Recording,
    Transcribing,
}

// Log entry for UI display
#[derive(Debug, Clone, serde::Serialize)]
pub struct LogEntry {
    pub timestamp: String,
    pub level: String,
    pub message: String,
}

// State shared with Tauri commands (must be Send + Sync)
pub struct AppState {
    pub config: Mutex<Config>,
    pub running: AtomicBool,
    pub logs: Mutex<VecDeque<LogEntry>>,
    pub status: Mutex<AppStatus>,
}

impl AppState {
    fn add_log(&self, level: &str, message: &str) {
        let mut logs = self.logs.lock().unwrap();
        let timestamp = chrono::Local::now().format("%H:%M:%S").to_string();
        logs.push_back(LogEntry {
            timestamp,
            level: level.to_string(),
            message: message.to_string(),
        });
        // Keep only last 100 logs
        while logs.len() > 100 {
            logs.pop_front();
        }
    }
}

// Create a colored dot icon
fn create_dot_icon(r: u8, g: u8, b: u8) -> Image<'static> {
    let size = 18;
    let mut data = vec![0u8; size * size * 4];

    let cx = size as f32 / 2.0;
    let cy = size as f32 / 2.0;
    let radius = 7.0;

    for y in 0..size {
        for x in 0..size {
            let idx = (y * size + x) * 4;
            let dx = x as f32 - cx;
            let dy = y as f32 - cy;
            let dist = (dx * dx + dy * dy).sqrt();

            if dist <= radius {
                data[idx] = r;
                data[idx + 1] = g;
                data[idx + 2] = b;
                data[idx + 3] = 255;
            }
        }
    }

    Image::new_owned(data, size as u32, size as u32)
}

// Green dot - idle
fn create_idle_icon() -> Image<'static> {
    create_dot_icon(34, 197, 94) // green-500
}

// Red dot - recording
fn create_recording_icon() -> Image<'static> {
    create_dot_icon(239, 68, 68) // red-500
}

// Yellow dot - transcribing
fn create_transcribing_icon() -> Image<'static> {
    create_dot_icon(234, 179, 8) // yellow-500
}

#[tauri::command]
fn get_logs(state: tauri::State<Arc<AppState>>) -> Vec<LogEntry> {
    state.logs.lock().unwrap().iter().cloned().collect()
}

#[tauri::command]
fn get_status(state: tauri::State<Arc<AppState>>) -> AppStatus {
    *state.status.lock().unwrap()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("ezwhisper=info".parse().unwrap()),
        )
        .init();

    tracing::info!("starting ezwhisper");

    let config = Config::load().unwrap_or_default();
    tracing::info!("config loaded");

    let state = Arc::new(AppState {
        config: Mutex::new(config),
        running: AtomicBool::new(true),
        logs: Mutex::new(VecDeque::new()),
        status: Mutex::new(AppStatus::Idle),
    });

    state.add_log("INFO", "ezwhisper started");

    let state_for_tauri = state.clone();

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .manage(state_for_tauri)
        .invoke_handler(tauri::generate_handler![
            get_config,
            save_config,
            list_audio_devices,
            get_logs,
            get_status,
        ])
        .setup(move |app| {
            let handle = app.handle().clone();
            let state_for_input = state.clone();

            // Build tray menu
            let settings_item = MenuItem::with_id(app, "settings", "Settings...", true, None::<&str>)?;
            let quit_item = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&settings_item, &quit_item])?;

            let tray = TrayIconBuilder::new()
                .icon(create_idle_icon())
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_menu_event(move |app, event| {
                    match event.id.as_ref() {
                        "settings" => {
                            if let Some(window) = app.get_webview_window("main") {
                                let _ = window.show();
                                let _ = window.set_focus();
                            }
                        }
                        "quit" => {
                            app.exit(0);
                        }
                        _ => {}
                    }
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        let app = tray.app_handle();
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                })
                .build(app)?;

            // Store tray icon ID for later updates
            let tray_id = tray.id().clone();

            // Start input monitoring in background thread
            std::thread::spawn(move || {
                run_input_loop(handle, state_for_input, tray_id);
            });

            Ok(())
        })
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                let _ = window.hide();
                api.prevent_close();
            }
        })
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|_app, event| {
            if let RunEvent::ExitRequested { api, .. } = event {
                api.prevent_exit();
            }
        });
}

fn run_input_loop(app: AppHandle, state: Arc<AppState>, tray_id: tauri::tray::TrayIconId) {
    let input_monitor = match InputMonitor::new() {
        Ok(m) => m,
        Err(e) => {
            tracing::error!("failed to start input monitor: {}", e);
            state.add_log("ERROR", &format!("failed to start input monitor: {}", e));
            return;
        }
    };

    let mut recorder = match AudioRecorder::new() {
        Ok(r) => Some(r),
        Err(e) => {
            tracing::error!("failed to create audio recorder: {}", e);
            state.add_log("ERROR", &format!("failed to create audio recorder: {}", e));
            None
        }
    };

    let rt = tokio::runtime::Runtime::new().unwrap();
    let mut status = AppStatus::Idle;

    tracing::info!("input monitoring started");
    state.add_log("INFO", "input monitoring started - click trigger to record");

    // Helper to update tray icon based on status
    let update_icon = |app: &AppHandle, status: AppStatus| {
        if let Some(tray) = app.tray_by_id(&tray_id) {
            let icon = match status {
                AppStatus::Idle => create_idle_icon(),
                AppStatus::Recording => create_recording_icon(),
                AppStatus::Transcribing => create_transcribing_icon(),
            };
            let _ = tray.set_icon(Some(icon));
        }
    };

    while state.running.load(Ordering::SeqCst) {
        if let Some(event) = input_monitor.try_recv() {
            if matches!(event, InputEvent::TriggerPressed) {
                match status {
                    AppStatus::Idle => {
                        if let Some(ref mut rec) = recorder {
                            if let Err(e) = rec.start() {
                                tracing::error!("failed to start recording: {}", e);
                                state.add_log("ERROR", &format!("failed to start recording: {}", e));
                            } else {
                                status = AppStatus::Recording;
                                *state.status.lock().unwrap() = status;
                                update_icon(&app, status);
                                tracing::info!("recording...");
                                state.add_log("INFO", "recording...");
                                let _ = app.emit("status-changed", status);
                            }
                        }
                    }
                    AppStatus::Recording => {
                        status = AppStatus::Transcribing;
                        *state.status.lock().unwrap() = status;
                        update_icon(&app, status);
                        tracing::info!("transcribing...");
                        state.add_log("INFO", "transcribing...");
                        let _ = app.emit("status-changed", status);

                        let config = state.config.lock().unwrap().clone();
                        let audio_data = recorder.as_mut().and_then(|rec| rec.stop().ok());

                        if let Some(audio_data) = audio_data {
                            if !audio_data.is_empty() {
                                let client = ElevenLabsClient::new(
                                    config.elevenlabs_api_key.clone(),
                                    config.language.clone(),
                                );

                                let cleaner = if config.anthropic_api_key.is_empty() {
                                    None
                                } else {
                                    Some(TextCleaner::new(config.anthropic_api_key.clone()))
                                };

                                let result = rt.block_on(async {
                                    client.transcribe(audio_data).await
                                });

                                match result {
                                    Ok(text) if !text.is_empty() => {
                                        let final_text = if config.translate && cleaner.is_some() {
                                            rt.block_on(async {
                                                cleaner.as_ref().unwrap().translate(&text).await
                                            }).unwrap_or(text)
                                        } else if config.cleanup && cleaner.is_some() {
                                            rt.block_on(async {
                                                cleaner.as_ref().unwrap().cleanup(&text).await
                                            }).unwrap_or(text)
                                        } else {
                                            text
                                        };

                                        tracing::info!("inserting: {}", final_text);
                                        state.add_log("INFO", &format!("inserting: {}", final_text));
                                        let inserter = TextInserter::new(config.auto_enter);
                                        if let Err(e) = inserter.insert(&final_text) {
                                            tracing::error!("failed to insert text: {}", e);
                                            state.add_log("ERROR", &format!("failed to insert text: {}", e));
                                        }
                                    }
                                    Ok(_) => {
                                        tracing::warn!("empty transcription");
                                        state.add_log("WARN", "empty transcription");
                                    }
                                    Err(e) => {
                                        tracing::error!("transcription failed: {}", e);
                                        state.add_log("ERROR", &format!("transcription failed: {}", e));
                                    }
                                }
                            }
                        }

                        status = AppStatus::Idle;
                        *state.status.lock().unwrap() = status;
                        update_icon(&app, status);
                        tracing::info!("ready");
                        state.add_log("INFO", "ready");
                        let _ = app.emit("status-changed", status);
                    }
                    AppStatus::Transcribing => {
                        // Ignore clicks while transcribing
                    }
                }
            }
        }

        std::thread::sleep(std::time::Duration::from_millis(1));
    }
}
