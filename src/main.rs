mod audio;
mod cleanup;
mod config;
mod input;
mod menubar;
mod output;
mod transcribe;

use anyhow::Result;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc};

use audio::{list_input_devices, AudioRecorder};
use cleanup::TextCleaner;
use config::Config;
use input::{InputEvent, InputMonitor};
use menubar::{AppStatus, MenuBar, MenuCommand};
use output::TextInserter;
use transcribe::ElevenLabsClient;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AppState {
    Idle,
    Recording,
}

fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("ezwhisper=info".parse().unwrap()),
        )
        .init();

    tracing::info!("starting ezwhisper");

    let config = Config::from_env()?;
    tracing::info!("config loaded, language: {}", config.ezwhisper_language);

    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();

    ctrlc::set_handler(move || {
        tracing::info!("received ctrl-c, shutting down");
        r.store(false, Ordering::SeqCst);
    })
    .expect("failed to set ctrl-c handler");

    // Create tokio runtime for async operations
    let rt = tokio::runtime::Runtime::new()?;

    // Set up menu bar
    let (menu_tx, menu_rx) = mpsc::channel::<MenuCommand>();
    let menubar = MenuBar::new(menu_tx);

    // List and display audio devices
    let devices = list_input_devices();
    let device_names: Vec<String> = devices.iter().map(|d| d.name.clone()).collect();
    tracing::info!("available audio devices:");
    for (i, name) in device_names.iter().enumerate() {
        tracing::info!("  [{}] {}", i, name);
    }
    if let Some(default) = audio::get_default_device() {
        tracing::info!("default input: {}", default.name);
    }
    menubar.set_devices(device_names);
    menubar.set_status(AppStatus::Idle);

    let client = ElevenLabsClient::new(config.elevenlabs_api_key.clone(), config.ezwhisper_language.clone());
    let cleaner = config.anthropic_api_key.as_ref().map(|key| TextCleaner::new(key.clone()));
    let use_translate = config.ezwhisper_translate && cleaner.is_some();
    let use_cleanup = config.ezwhisper_cleanup && cleaner.is_some() && !use_translate;
    if use_translate {
        tracing::info!("Haiku translate-to-English enabled");
    } else if use_cleanup {
        tracing::info!("Haiku cleanup enabled");
    }
    let mut recorder = AudioRecorder::new()?;

    // Set device from config if specified
    if let Some(idx) = config.ezwhisper_device {
        if idx < devices.len() {
            recorder.set_device(devices[idx].device.clone());
            menubar.set_selected_device(idx);
            tracing::info!("selected device from config: {}", devices[idx].name);
        } else {
            tracing::warn!("EZWHISPER_DEVICE={} is out of range, using default", idx);
        }
    }

    let inserter = TextInserter::new(config.ezwhisper_enter);
    if config.ezwhisper_enter {
        tracing::info!("auto-Enter enabled");
    }
    let input_monitor = InputMonitor::new()?;

    let mut state = AppState::Idle;

    tracing::info!("ezwhisper ready - hold trigger button to record");
    tracing::info!("triggers: Logitech gesture button or Right Option key");
    tracing::info!("tip: in Logi Options+, map gesture button to Right Option");
    tracing::info!("tip: set EZWHISPER_DEVICE=N to select input device by index");
    tracing::info!("press Ctrl-C to quit");

    // Drop unused receiver
    drop(menu_rx);

    while running.load(Ordering::SeqCst) {
        // Handle input events
        if let Some(event) = input_monitor.try_recv() {
            match event {
                InputEvent::TriggerPressed => {
                    if state == AppState::Idle {
                        state = AppState::Recording;
                        menubar.set_status(AppStatus::Recording);
                        tracing::info!("recording...");
                        if let Err(e) = recorder.start() {
                            tracing::error!("failed to start recording: {}", e);
                            state = AppState::Idle;
                            menubar.set_status(AppStatus::Idle);
                        }
                    }
                }
                InputEvent::TriggerReleased => {
                    if state == AppState::Recording {
                        menubar.set_status(AppStatus::Transcribing);
                        menubar.pump(); // Force UI update before blocking
                        tracing::info!("transcribing...");

                        match recorder.stop() {
                            Ok(audio_data) => {
                                if !audio_data.is_empty() {
                                    let result =
                                        rt.block_on(async { client.transcribe(audio_data).await });

                                    match result {
                                        Ok(text) => {
                                            if !text.is_empty() {
                                                let final_text = if use_translate {
                                                    let translated = rt.block_on(async {
                                                        cleaner.as_ref().unwrap().translate(&text).await
                                                    });
                                                    match translated {
                                                        Ok(t) => t,
                                                        Err(e) => {
                                                            tracing::warn!("translate failed: {}, using raw", e);
                                                            text
                                                        }
                                                    }
                                                } else if use_cleanup {
                                                    let cleaned = rt.block_on(async {
                                                        cleaner.as_ref().unwrap().cleanup(&text).await
                                                    });
                                                    match cleaned {
                                                        Ok(t) => t,
                                                        Err(e) => {
                                                            tracing::warn!("cleanup failed: {}, using raw", e);
                                                            text
                                                        }
                                                    }
                                                } else {
                                                    text
                                                };
                                                tracing::info!("inserting: {}", final_text);
                                                if let Err(e) = inserter.insert(&final_text) {
                                                    tracing::error!(
                                                        "failed to insert text: {}",
                                                        e
                                                    );
                                                }
                                            } else {
                                                tracing::warn!("empty transcription");
                                            }
                                        }
                                        Err(e) => {
                                            tracing::error!("transcription failed: {}", e);
                                        }
                                    }
                                } else {
                                    tracing::warn!("no audio recorded");
                                }
                            }
                            Err(e) => {
                                tracing::error!("failed to stop recording: {}", e);
                            }
                        }

                        state = AppState::Idle;
                        menubar.set_status(AppStatus::Idle);
                        tracing::info!("ready");
                    }
                }
            }
        }

        // Pump the run loop for menu bar updates
        menubar.pump();

        // Small sleep to prevent busy-waiting
        std::thread::sleep(std::time::Duration::from_millis(1));
    }

    tracing::info!("ezwhisper stopped");
    Ok(())
}
