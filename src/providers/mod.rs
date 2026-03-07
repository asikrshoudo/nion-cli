use anyhow::Result;
use async_trait::async_trait;

use crate::config::Config;
use crate::session::Message;

mod anthropic;
mod cohere;
mod deepseek;
mod google;
mod grok;
mod groq;
mod mistral;
mod openai;
mod perplexity;
mod together;

#[async_trait]
pub trait Provider: Send + Sync {
    /// Standard completion — no system prompt
    async fn complete(&self, messages: &[Message], model: &str) -> Result<String>;

    /// Completion with system prompt.
    /// Default: prepends a system-role message (works for all OpenAI-compatible APIs).
    /// Anthropic, Google, Cohere override this with their native system field.
    async fn complete_with_system(
        &self,
        messages: &[Message],
        model: &str,
        system: &str,
    ) -> Result<String> {
        let mut all = Vec::with_capacity(messages.len() + 1);
        all.push(Message {
            role: "system".to_string(),
            content: system.to_string(),
        });
        all.extend_from_slice(messages);
        self.complete(&all, model).await
    }

    fn name(&self) -> &str;
    fn default_model(&self) -> &str;
    fn available_models(&self) -> Vec<String>;
}

pub fn get_provider(name: &str, cfg: &Config) -> Result<Box<dyn Provider>> {
    let key = cfg
        .get_api_key(name)
        .ok_or_else(|| anyhow::anyhow!(
            "No API key for '{}'. Run 'nion config setup' to add one.", name
        ))?;

    let provider: Box<dyn Provider> = match name {
        "openai"     => Box::new(openai::OpenAIProvider::new(key)),
        "anthropic"  => Box::new(anthropic::AnthropicProvider::new(key)),
        "google"     => Box::new(google::GoogleProvider::new(key)),
        "groq"       => Box::new(groq::GroqProvider::new(key)),
        "grok"       => Box::new(grok::GrokProvider::new(key)),
        "deepseek"   => Box::new(deepseek::DeepSeekProvider::new(key)),
        "mistral"    => Box::new(mistral::MistralProvider::new(key)),
        "perplexity" => Box::new(perplexity::PerplexityProvider::new(key)),
        "together"   => Box::new(together::TogetherProvider::new(key)),
        "cohere"     => Box::new(cohere::CohereProvider::new(key)),
        other => anyhow::bail!("Unknown provider: '{}'. Use: openai, anthropic, google, groq, grok, deepseek, mistral, perplexity, together, cohere", other),
    };
    Ok(provider)
}
