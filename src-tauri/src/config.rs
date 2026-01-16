use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub elevenlabs_api_key: String,
    #[serde(default)]
    pub anthropic_api_key: String,
    #[serde(default = "default_language")]
    pub language: String,
    #[serde(default = "default_true")]
    pub auto_enter: bool,
    #[serde(default)]
    pub cleanup: bool,
    #[serde(default)]
    pub translate: bool,
    #[serde(default)]
    pub device_index: Option<usize>,
}

fn default_language() -> String {
    "auto".to_string()
}

fn default_true() -> bool {
    true
}

impl Default for Config {
    fn default() -> Self {
        Self {
            elevenlabs_api_key: std::env::var("ELEVENLABS_API_KEY").unwrap_or_default(),
            anthropic_api_key: std::env::var("ANTHROPIC_API_KEY").unwrap_or_default(),
            language: default_language(),
            auto_enter: true,
            cleanup: false,
            translate: false,
            device_index: None,
        }
    }
}

impl Config {
    fn config_path() -> Result<PathBuf> {
        let config_dir = dirs::config_dir()
            .context("could not find config directory")?
            .join("com.piotrostr.ezwhisper");

        if !config_dir.exists() {
            fs::create_dir_all(&config_dir)?;
        }

        Ok(config_dir.join("config.json"))
    }

    pub fn load() -> Result<Self> {
        let path = Self::config_path()?;

        if !path.exists() {
            return Ok(Self::default());
        }

        let contents = fs::read_to_string(&path)?;
        let mut config: Config = serde_json::from_str(&contents)?;

        // Fill in empty keys from environment
        if config.elevenlabs_api_key.is_empty() {
            config.elevenlabs_api_key = std::env::var("ELEVENLABS_API_KEY").unwrap_or_default();
        }
        if config.anthropic_api_key.is_empty() {
            config.anthropic_api_key = std::env::var("ANTHROPIC_API_KEY").unwrap_or_default();
        }

        Ok(config)
    }

    pub fn save(&self) -> Result<()> {
        let path = Self::config_path()?;
        let contents = serde_json::to_string_pretty(self)?;
        fs::write(path, contents)?;
        Ok(())
    }
}
