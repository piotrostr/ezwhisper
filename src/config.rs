use anyhow::{Context, Result};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub elevenlabs_api_key: String,
    #[serde(default)]
    pub anthropic_api_key: Option<String>,
    #[serde(default = "default_language")]
    pub ezwhisper_language: String,
    #[serde(default)]
    pub ezwhisper_device: Option<usize>,
    #[serde(default)]
    pub ezwhisper_cleanup: bool,
    #[serde(default)]
    pub ezwhisper_enter: bool,
}

fn default_language() -> String {
    "en".to_string()
}

impl Config {
    pub fn from_env() -> Result<Self> {
        envy::from_env::<Config>().context("failed to parse config from environment")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_language() {
        assert_eq!(default_language(), "en");
    }
}
