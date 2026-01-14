use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::time::Duration;

pub struct TextCleaner {
    api_key: String,
    client: reqwest::Client,
}

#[derive(Serialize)]
struct Message {
    role: &'static str,
    content: String,
}

#[derive(Serialize)]
struct ClaudeRequest {
    model: &'static str,
    max_tokens: u32,
    system: &'static str,
    messages: Vec<Message>,
}

#[derive(Deserialize)]
struct ClaudeResponse {
    content: Vec<ContentBlock>,
}

#[derive(Deserialize)]
struct ContentBlock {
    text: String,
}

impl TextCleaner {
    pub fn new(api_key: String) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .expect("failed to build HTTP client");

        Self { api_key, client }
    }

    pub async fn cleanup(&self, raw_text: &str) -> Result<String> {
        if raw_text.trim().is_empty() {
            return Ok(String::new());
        }

        let start = std::time::Instant::now();
        tracing::debug!("cleaning up transcription with Haiku");

        let request = ClaudeRequest {
            model: "claude-3-5-haiku-latest",
            max_tokens: 1024,
            system: "You are a text formatting tool. You receive raw speech-to-text output and return ONLY the cleaned version. Fix capitalization and punctuation. Never add commentary, notes, apologies, or explanations. Never say 'I', never ask questions, never add parenthetical remarks. Output the cleaned text and nothing else.",
            messages: vec![Message {
                role: "user",
                content: raw_text.to_string(),
            }],
        };

        let response = self
            .client
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&request)
            .send()
            .await
            .context("failed to send request to Claude")?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            tracing::warn!("Haiku cleanup failed ({}): {}, using raw text", status, error_text);
            return Ok(raw_text.to_string());
        }

        let result: ClaudeResponse = response
            .json()
            .await
            .context("failed to parse Claude response")?;

        let cleaned = result
            .content
            .first()
            .map(|c| c.text.clone())
            .unwrap_or_else(|| raw_text.to_string());

        tracing::info!("cleanup took {:?}", start.elapsed());

        Ok(cleaned)
    }
}
