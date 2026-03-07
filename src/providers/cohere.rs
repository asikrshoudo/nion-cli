use anyhow::Result;
use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};

use super::Provider;
use crate::session::Message;

pub struct CohereProvider {
    api_key: String,
    client: Client,
}

impl CohereProvider {
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
        #[derive(Serialize)]
        struct Req {
            model: String,
            messages: Vec<CohereMessage>,
            max_tokens: u32,
            #[serde(skip_serializing_if = "Option::is_none")]
            system: Option<String>,
        }

        let co_messages: Vec<CohereMessage> = messages
            .iter()
            .filter(|m| m.role != "system")
            .map(|m| CohereMessage {
                role: m.role.clone(),
                content: m.content.clone(),
            })
            .collect();

        let req = Req {
            model: model.to_string(),
            messages: co_messages,
            max_tokens: 4096,
            system: system.map(String::from),
        };

        let resp = self
            .client
            .post("https://api.cohere.com/v2/chat")
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&req)
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let err_text = resp.text().await?;
            anyhow::bail!("Cohere API error ({}): {}", status, err_text);
        }

        let data: ChatResponse = resp.json().await?;
        let text = data
            .message
            .content
            .iter()
            .filter_map(|b| b.text.clone())
            .collect::<Vec<_>>()
            .join("");
        Ok(text)
    }
}

#[derive(Serialize, Deserialize)]
struct CohereMessage {
    role: String,
    content: String,
}

#[derive(Deserialize)]
struct ChatResponse {
    message: CohereResponseMessage,
}

#[derive(Deserialize)]
struct CohereResponseMessage {
    content: Vec<ContentBlock>,
}

#[derive(Deserialize)]
struct ContentBlock {
    text: Option<String>,
}

#[async_trait]
impl Provider for CohereProvider {
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

    fn name(&self) -> &str { "cohere" }
    fn default_model(&self) -> &str { "command-r-plus-08-2024" }
    fn available_models(&self) -> Vec<String> {
        vec![
            "command-r-plus-08-2024".into(),
            "command-r-08-2024".into(),
            "command-light".into(),
        ]
    }
}
