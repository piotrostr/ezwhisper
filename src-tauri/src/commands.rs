use crate::audio::list_input_devices;
use crate::config::Config;
use crate::AppState;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tauri::State;

#[derive(Debug, Serialize, Deserialize)]
pub struct AudioDevice {
    pub index: usize,
    pub name: String,
}

#[tauri::command]
pub fn get_config(state: State<'_, Arc<AppState>>) -> Config {
    state.config.lock().unwrap().clone()
}

#[tauri::command]
pub fn save_config(state: State<'_, Arc<AppState>>, config: Config) -> Result<(), String> {
    config.save().map_err(|e| e.to_string())?;
    *state.config.lock().unwrap() = config;
    Ok(())
}

#[tauri::command]
pub fn list_audio_devices() -> Vec<AudioDevice> {
    list_input_devices()
        .into_iter()
        .enumerate()
        .map(|(index, d)| AudioDevice {
            index,
            name: d.name,
        })
        .collect()
}
