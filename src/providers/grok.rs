use anyhow::Result;
use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};

use super::Provider;
use crate::session::Message;

pub struct GrokProvider {
    api_key: String,
    client: Client,
}

impl GrokProvider {
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

// xAI uses OpenAI-compatible API format
#[derive(Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<GrokMessage>,
    temperature: f32,
    max_tokens: u32,
    stream: bool,
}

#[derive(Serialize, Deserialize)]
struct GrokMessage {
    role: String,
    content: String,
}

#[derive(Deserialize)]
struct ChatResponse {
    choices: Vec<Choice>,
}

#[derive(Deserialize)]
struct Choice {
    message: GrokMessage,
}

#[async_trait]
impl Provider for GrokProvider {
    async fn complete(&self, messages: &[Message], model: &str) -> Result<String> {
        let grok_messages: Vec<GrokMessage> = messages
            .iter()
            .map(|m| GrokMessage {
                role: m.role.clone(),
                content: m.content.clone(),
            })
            .collect();

        let req = ChatRequest {
            model: model.to_string(),
            messages: grok_messages,
            temperature: 0.7,
            max_tokens: 4096,
            stream: false,
        };

        let resp = self
            .client
            .post("https://api.x.ai/v1/chat/completions")
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&req)
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let err_text = resp.text().await?;
            anyhow::bail!("xAI API error ({}): {}", status, err_text);
        }

        let data: ChatResponse = resp.json().await?;
        let text = data
            .choices
            .first()
            .map(|c| c.message.content.clone())
            .unwrap_or_default();

        Ok(text)
    }

    fn name(&self) -> &str {
        "grok"
    }

    fn default_model(&self) -> &str {
        "grok-2-latest"
    }

    fn available_models(&self) -> Vec<String> {
        vec![
            "grok-2-latest".to_string(),
            "grok-2-vision-latest".to_string(),
            "grok-beta".to_string(),
        ]
    }
}
