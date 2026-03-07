use anyhow::Result;
use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};

use crate::session::Message;
use super::Provider;

pub struct GoogleProvider {
    api_key: String,
    client: Client,
}

impl GoogleProvider {
    pub fn new(api_key: &str) -> Self {
        Self {
            api_key: api_key.to_string(),
            client: Client::builder()
                .timeout(std::time::Duration::from_secs(120))
                .build()
                .unwrap(),
        }
    }
}

#[derive(Serialize)]
struct GeminiRequest {
    contents: Vec<GeminiContent>,
    #[serde(rename = "generationConfig")]
    generation_config: GenerationConfig,
}

#[derive(Serialize)]
struct GeminiContent {
    role: String,
    parts: Vec<Part>,
}

#[derive(Serialize)]
struct Part {
    text: String,
}

#[derive(Serialize)]
struct GenerationConfig {
    #[serde(rename = "maxOutputTokens")]
    max_output_tokens: u32,
    temperature: f32,
}

#[derive(Deserialize)]
struct GeminiResponse {
    candidates: Vec<Candidate>,
}

#[derive(Deserialize)]
struct Candidate {
    content: GeminiResponseContent,
}

#[derive(Deserialize)]
struct GeminiResponseContent {
    parts: Vec<ResponsePart>,
}

#[derive(Deserialize)]
struct ResponsePart {
    text: String,
}

fn to_gemini_role(role: &str) -> String {
    match role {
        "assistant" => "model".to_string(),
        _ => "user".to_string(),
    }
}

#[async_trait]
impl Provider for GoogleProvider {
    async fn complete(&self, messages: &[Message], model: &str) -> Result<String> {
        // Gemini requires alternating user/model roles — merge consecutive same-role messages
        let mut contents: Vec<GeminiContent> = Vec::new();
        for m in messages {
            let role = to_gemini_role(&m.role);
            if let Some(last) = contents.last_mut() {
                if last.role == role {
                    // Append to last message
                    last.parts.push(Part { text: m.content.clone() });
                    continue;
                }
            }
            contents.push(GeminiContent {
                role,
                parts: vec![Part { text: m.content.clone() }],
            });
        }

        let req = GeminiRequest {
            contents,
            generation_config: GenerationConfig {
                max_output_tokens: 4096,
                temperature: 0.7,
            },
        };

        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
            model, self.api_key
        );

        let resp = self
            .client
            .post(&url)
            .header("Content-Type", "application/json")
            .json(&req)
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let err_text = resp.text().await?;
            anyhow::bail!("Google API error ({}): {}", status, err_text);
        }

        let data: GeminiResponse = resp.json().await?;
        let text = data
            .candidates
            .first()
            .and_then(|c| c.content.parts.first())
            .map(|p| p.text.clone())
            .unwrap_or_default();

        Ok(text)
    }

    fn name(&self) -> &str {
        "google"
    }

    fn default_model(&self) -> &str {
        "gemini-1.5-pro"
    }

    fn available_models(&self) -> Vec<String> {
        vec![
            "gemini-1.5-pro".to_string(),
            "gemini-1.5-flash".to_string(),
            "gemini-2.0-flash".to_string(),
            "gemini-2.0-flash-thinking-exp".to_string(),
        ]
    }
}
