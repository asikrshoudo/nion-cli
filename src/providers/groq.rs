use anyhow::Result;
use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};

use crate::session::Message;
use super::Provider;

pub struct GroqProvider {
    api_key: String,
    client: Client,
}

impl GroqProvider {
    pub fn new(api_key: &str) -> Self {
        Self {
            api_key: api_key.to_string(),
            client: Client::builder()
                .timeout(std::time::Duration::from_secs(60))
                .build()
                .unwrap(),
        }
    }
}

#[derive(Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<GroqMessage>,
    temperature: f32,
    max_tokens: u32,
}

#[derive(Serialize, Deserialize)]
struct GroqMessage {
    role: String,
    content: String,
}

#[derive(Deserialize)]
struct ChatResponse {
    choices: Vec<Choice>,
}

#[derive(Deserialize)]
struct Choice {
    message: GroqMessage,
}

#[async_trait]
impl Provider for GroqProvider {
    async fn complete(&self, messages: &[Message], model: &str) -> Result<String> {
        let groq_messages: Vec<GroqMessage> = messages
            .iter()
            .map(|m| GroqMessage {
                role: m.role.clone(),
                content: m.content.clone(),
            })
            .collect();

        let req = ChatRequest {
            model: model.to_string(),
            messages: groq_messages,
            temperature: 0.7,
            max_tokens: 4096,
        };

        let resp = self
            .client
            .post("https://api.groq.com/openai/v1/chat/completions")
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&req)
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let err_text = resp.text().await?;
            anyhow::bail!("Groq API error ({}): {}", status, err_text);
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
        "groq"
    }

    fn default_model(&self) -> &str {
        "llama-3.3-70b-versatile"
    }

    fn available_models(&self) -> Vec<String> {
        vec![
            "llama-3.3-70b-versatile".to_string(),
            "llama-3.1-8b-instant".to_string(),
            "llama3-70b-8192".to_string(),
            "mixtral-8x7b-32768".to_string(),
            "gemma2-9b-it".to_string(),
        ]
    }
}
