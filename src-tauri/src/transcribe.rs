use anyhow::{Context, Result};
use reqwest::multipart;
use serde::Deserialize;
use serde_json;
use std::time::Duration;

#[derive(Debug, Deserialize)]
struct TranscriptionResponse {
    text: String,
}

pub struct ElevenLabsClient {
    api_key: String,
    language: String,
    client: reqwest::Client,
}

impl ElevenLabsClient {
    pub fn new(api_key: String, language: String) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .expect("failed to build HTTP client");

        Self {
            api_key,
            language,
            client,
        }
    }

    pub async fn transcribe(&self, audio_data: Vec<u8>) -> Result<String> {
        if audio_data.is_empty() {
            return Ok(String::new());
        }

        tracing::info!(
            "sending {} bytes to ElevenLabs (language: {})",
            audio_data.len(),
            self.language
        );

        let start = std::time::Instant::now();
        let mut form = multipart::Form::new()
            .text("model_id", "scribe_v1")
            .part(
                "file",
                multipart::Part::bytes(audio_data)
                    .file_name("audio.wav")
                    .mime_str("audio/wav")?,
            );

        if self.language != "auto" {
            form = form.text("language_code", self.language.clone());
        }

        tracing::debug!("sending HTTP request...");
        let response = self
            .client
            .post("https://api.elevenlabs.io/v1/speech-to-text")
            .header("xi-api-key", &self.api_key)
            .multipart(form)
            .send()
            .await
            .context("failed to send request to ElevenLabs")?;
        tracing::debug!("HTTP response received");

        let status = response.status();
        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("ElevenLabs API error ({}): {}", status, error_text);
        }

        let body = response.text().await.context("failed to read response body")?;

        let result: TranscriptionResponse = serde_json::from_str(&body)
            .context("failed to parse ElevenLabs response")?;

        tracing::info!("transcription took {:?}", start.elapsed());
        tracing::info!("raw transcription: {}", result.text);

        Ok(result.text)
    }
}
