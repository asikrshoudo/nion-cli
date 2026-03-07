use anyhow::Result;
use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};

use super::Provider;
use crate::session::Message;

pub struct DeepSeekProvider {
    api_key: String,
    client: Client,
}

impl DeepSeekProvider {
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
    messages: Vec<DSMessage>,
    max_tokens: u32,
    stream: bool,
}

#[derive(Serialize, Deserialize)]
struct DSMessage {
    role: String,
    content: String,
}

#[derive(Deserialize)]
struct ChatResponse {
    choices: Vec<Choice>,
}

#[derive(Deserialize)]
struct Choice {
    message: DSMessage,
}

#[async_trait]
impl Provider for DeepSeekProvider {
    async fn complete(&self, messages: &[Message], model: &str) -> Result<String> {
        let ds_messages: Vec<DSMessage> = messages
            .iter()
            .map(|m| DSMessage {
                role: m.role.clone(),
                content: m.content.clone(),
            })
            .collect();

        let req = ChatRequest {
            model: model.to_string(),
            messages: ds_messages,
            max_tokens: 4096,
            stream: false,
        };

        let resp = self
            .client
            .post("https://api.deepseek.com/chat/completions")
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&req)
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let err_text = resp.text().await?;
            anyhow::bail!("DeepSeek API error ({}): {}", status, err_text);
        }

        let data: ChatResponse = resp.json().await?;
        Ok(data
            .choices
            .first()
            .map(|c| c.message.content.clone())
            .unwrap_or_default())
    }

    fn name(&self) -> &str {
        "deepseek"
    }

    fn default_model(&self) -> &str {
        "deepseek-chat"
    }

    fn available_models(&self) -> Vec<String> {
        vec![
            "deepseek-chat".into(),
            "deepseek-reasoner".into(),
        ]
    }
}
