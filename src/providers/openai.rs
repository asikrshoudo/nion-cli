use anyhow::Result;
use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};

use super::Provider;
use crate::session::Message;

pub struct OpenAIProvider {
    api_key: String,
    client: Client,
}

impl OpenAIProvider {
    pub fn new(api_key: &str) -> Self {
        Self {
            api_key: api_key.to_string(),
            client: Client::builder()
                .timeout(std::time::Duration::from_secs(180))
                .build()
                .unwrap(),
        }
    }
}

#[derive(Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<OAIMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
}

#[derive(Serialize, Deserialize)]
struct OAIMessage {
    role: String,
    content: String,
}

#[derive(Deserialize)]
struct ChatResponse {
    choices: Vec<Choice>,
}

#[derive(Deserialize)]
struct Choice {
    message: OAIMessage,
}

fn is_reasoning_model(model: &str) -> bool {
    matches!(model, "o1" | "o1-mini" | "o1-preview" | "o3-mini" | "o3")
}

#[async_trait]
impl Provider for OpenAIProvider {
    async fn complete(&self, messages: &[Message], model: &str) -> Result<String> {
        let oai_messages: Vec<OAIMessage> = messages
            .iter()
            .map(|m| OAIMessage {
                role: m.role.clone(),
                content: m.content.clone(),
            })
            .collect();

        let max_tokens = if is_reasoning_model(model) {
            None
        } else {
            Some(4096u32)
        };

        let req = ChatRequest {
            model: model.to_string(),
            messages: oai_messages,
            max_tokens,
        };

        let resp = self
            .client
            .post("https://api.openai.com/v1/chat/completions")
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&req)
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let err_text = resp.text().await?;
            anyhow::bail!("OpenAI API error ({}): {}", status, err_text);
        }

        let data: ChatResponse = resp.json().await?;
        Ok(data
            .choices
            .first()
            .map(|c| c.message.content.clone())
            .unwrap_or_default())
    }

    fn name(&self) -> &str {
        "openai"
    }

    fn default_model(&self) -> &str {
        "gpt-4o"
    }

    fn available_models(&self) -> Vec<String> {
        vec![
            "gpt-4o".into(),
            "gpt-4o-mini".into(),
            "gpt-4-turbo".into(),
            "gpt-3.5-turbo".into(),
            "o1".into(),
            "o1-mini".into(),
            "o3-mini".into(),
        ]
    }
}
