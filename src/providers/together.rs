use anyhow::Result;
use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};

use crate::session::Message;
use super::Provider;

pub struct TogetherProvider {
    api_key: String,
    client: Client,
}

impl TogetherProvider {
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
    messages: Vec<TgtMessage>,
    max_tokens: u32,
    temperature: f32,
}

#[derive(Serialize, Deserialize)]
struct TgtMessage {
    role: String,
    content: String,
}

#[derive(Deserialize)]
struct ChatResponse {
    choices: Vec<Choice>,
}

#[derive(Deserialize)]
struct Choice {
    message: TgtMessage,
}

#[async_trait]
impl Provider for TogetherProvider {
    async fn complete(&self, messages: &[Message], model: &str) -> Result<String> {
        let tgt_messages: Vec<TgtMessage> = messages
            .iter()
            .map(|m| TgtMessage {
                role: m.role.clone(),
                content: m.content.clone(),
            })
            .collect();

        let req = ChatRequest {
            model: model.to_string(),
            messages: tgt_messages,
            max_tokens: 4096,
            temperature: 0.7,
        };

        let resp = self
            .client
            .post("https://api.together.xyz/v1/chat/completions")
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&req)
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let err_text = resp.text().await?;
            anyhow::bail!("Together AI API error ({}): {}", status, err_text);
        }

        let data: ChatResponse = resp.json().await?;
        Ok(data
            .choices
            .first()
            .map(|c| c.message.content.clone())
            .unwrap_or_default())
    }

    fn name(&self) -> &str { "together" }

    fn default_model(&self) -> &str { "meta-llama/Llama-3.3-70B-Instruct-Turbo" }

    fn available_models(&self) -> Vec<String> {
        vec![
            "meta-llama/Llama-3.3-70B-Instruct-Turbo".into(),
            "meta-llama/Meta-Llama-3.1-405B-Instruct-Turbo".into(),
            "deepseek-ai/DeepSeek-V3".into(),
            "Qwen/Qwen2.5-72B-Instruct-Turbo".into(),
            "mistralai/Mixtral-8x22B-Instruct-v0.1".into(),
            "google/gemma-2-27b-it".into(),
        ]
    }
}
