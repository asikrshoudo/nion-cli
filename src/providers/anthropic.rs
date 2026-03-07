use anyhow::Result;
use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};

use super::Provider;
use crate::session::Message;

pub struct AnthropicProvider {
    api_key: String,
    client: Client,
}

impl AnthropicProvider {
    pub fn new(api_key: &str) -> Self {
        Self {
            api_key: api_key.to_string(),
            client: Client::builder()
                .timeout(std::time::Duration::from_secs(120))
                .build()
                .unwrap(),
        }
    }

    async fn complete_inner(
        &self,
        messages: &[Message],
        model: &str,
        system: Option<&str>,
    ) -> Result<String> {
        let anthropic_messages: Vec<AnthropicMessage> = messages
            .iter()
            .filter(|m| m.role != "system") // strip any system messages
            .map(|m| AnthropicMessage {
                role: m.role.clone(),
                content: m.content.clone(),
            })
            .collect();

        #[derive(Serialize)]
        struct Req {
            model: String,
            max_tokens: u32,
            messages: Vec<AnthropicMessage>,
            #[serde(skip_serializing_if = "Option::is_none")]
            system: Option<String>,
        }

        let req = Req {
            model: model.to_string(),
            max_tokens: 8192,
            messages: anthropic_messages,
            system: system.map(String::from),
        };

        let resp = self
            .client
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("Content-Type", "application/json")
            .json(&req)
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let err_text = resp.text().await?;
            anyhow::bail!("Anthropic API error ({}): {}", status, err_text);
        }

        let data: AnthropicResponse = resp.json().await?;
        let text = data
            .content
            .iter()
            .filter(|b| b.block_type == "text")
            .filter_map(|b| b.text.clone())
            .collect::<Vec<_>>()
            .join("");
        Ok(text)
    }
}

#[derive(Serialize, Deserialize, Clone)]
struct AnthropicMessage {
    role: String,
    content: String,
}

#[derive(Deserialize)]
struct AnthropicResponse {
    content: Vec<ContentBlock>,
}

#[derive(Deserialize)]
struct ContentBlock {
    #[serde(rename = "type")]
    block_type: String,
    text: Option<String>,
}

#[async_trait]
impl Provider for AnthropicProvider {
    async fn complete(&self, messages: &[Message], model: &str) -> Result<String> {
        self.complete_inner(messages, model, None).await
    }

    async fn complete_with_system(
        &self,
        messages: &[Message],
        model: &str,
        system: &str,
    ) -> Result<String> {
        self.complete_inner(messages, model, Some(system)).await
    }

    fn name(&self) -> &str {
        "anthropic"
    }

    fn default_model(&self) -> &str {
        "claude-3-5-sonnet-20241022"
    }

    fn available_models(&self) -> Vec<String> {
        vec![
            "claude-3-5-sonnet-20241022".to_string(),
            "claude-3-5-haiku-20241022".to_string(),
            "claude-3-opus-20240229".to_string(),
            "claude-3-haiku-20240307".to_string(),
        ]
    }
}
