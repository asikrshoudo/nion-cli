use anyhow::Result;
use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};

use crate::session::Message;
use super::Provider;

pub struct PerplexityProvider {
    api_key: String,
    client: Client,
}

impl PerplexityProvider {
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
    messages: Vec<PplxMessage>,
    max_tokens: u32,
}

#[derive(Serialize, Deserialize)]
struct PplxMessage {
    role: String,
    content: String,
}

#[derive(Deserialize)]
struct ChatResponse {
    choices: Vec<Choice>,
}

#[derive(Deserialize)]
struct Choice {
    message: PplxMessage,
}

#[async_trait]
impl Provider for PerplexityProvider {
    async fn complete(&self, messages: &[Message], model: &str) -> Result<String> {
        let pplx_messages: Vec<PplxMessage> = messages
            .iter()
            .map(|m| PplxMessage {
                role: m.role.clone(),
                content: m.content.clone(),
            })
            .collect();

        let req = ChatRequest {
            model: model.to_string(),
            messages: pplx_messages,
            max_tokens: 4096,
        };

        let resp = self
            .client
            .post("https://api.perplexity.ai/chat/completions")
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&req)
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let err_text = resp.text().await?;
            anyhow::bail!("Perplexity API error ({}): {}", status, err_text);
        }

        let data: ChatResponse = resp.json().await?;
        Ok(data
            .choices
            .first()
            .map(|c| c.message.content.clone())
            .unwrap_or_default())
    }

    fn name(&self) -> &str { "perplexity" }

    fn default_model(&self) -> &str { "sonar-pro" }

    fn available_models(&self) -> Vec<String> {
        vec![
            "sonar-pro".into(),
            "sonar".into(),
            "sonar-reasoning-pro".into(),
            "sonar-reasoning".into(),
        ]
    }
}
