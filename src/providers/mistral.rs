use anyhow::Result;
use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};

use super::Provider;
use crate::session::Message;

pub struct MistralProvider {
    api_key: String,
    client: Client,
}

impl MistralProvider {
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
struct ChatRequest {
    model: String,
    messages: Vec<MistralMessage>,
    max_tokens: u32,
}

#[derive(Serialize, Deserialize)]
struct MistralMessage {
    role: String,
    content: String,
}

#[derive(Deserialize)]
struct ChatResponse {
    choices: Vec<Choice>,
}

#[derive(Deserialize)]
struct Choice {
    message: MistralMessage,
}

#[async_trait]
impl Provider for MistralProvider {
    async fn complete(&self, messages: &[Message], model: &str) -> Result<String> {
        let m_messages: Vec<MistralMessage> = messages
            .iter()
            .map(|m| MistralMessage {
                role: m.role.clone(),
                content: m.content.clone(),
            })
            .collect();

        let req = ChatRequest {
            model: model.to_string(),
            messages: m_messages,
            max_tokens: 4096,
        };

        let resp = self
            .client
            .post("https://api.mistral.ai/v1/chat/completions")
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&req)
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let err_text = resp.text().await?;
            anyhow::bail!("Mistral API error ({}): {}", status, err_text);
        }

        let data: ChatResponse = resp.json().await?;
        Ok(data
            .choices
            .first()
            .map(|c| c.message.content.clone())
            .unwrap_or_default())
    }

    fn name(&self) -> &str {
        "mistral"
    }

    fn default_model(&self) -> &str {
        "mistral-large-latest"
    }

    fn available_models(&self) -> Vec<String> {
        vec![
            "mistral-large-latest".into(),
            "mistral-small-latest".into(),
            "codestral-latest".into(),
            "open-mistral-nemo".into(),
            "open-mixtral-8x22b".into(),
        ]
    }
}
