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

pub use anthropic::AnthropicProvider;
pub use cohere::CohereProvider;
pub use deepseek::DeepSeekProvider;
pub use google::GoogleProvider;
pub use grok::GrokProvider;
pub use groq::GroqProvider;
pub use mistral::MistralProvider;
pub use openai::OpenAIProvider;
pub use perplexity::PerplexityProvider;
pub use together::TogetherProvider;

#[async_trait]
pub trait Provider: Send + Sync {
    async fn complete(&self, messages: &[Message], model: &str) -> Result<String>;
    fn name(&self) -> &str;
    fn default_model(&self) -> &str;
    #[allow(dead_code)]
    fn available_models(&self) -> Vec<String>;
}

pub fn get_provider(name: &str, cfg: &Config) -> Result<Box<dyn Provider>> {
    let lower = name.to_lowercase();
    let lower = lower.as_str();

    match lower {
        "openai" | "gpt" | "chatgpt" | "codex" => {
            let key = require_key(cfg, "openai", "https://platform.openai.com/api-keys")?;
            Ok(Box::new(OpenAIProvider::new(key)))
        }
        "anthropic" | "claude" => {
            let key = require_key(cfg, "anthropic", "https://console.anthropic.com")?;
            Ok(Box::new(AnthropicProvider::new(key)))
        }
        "google" | "gemini" => {
            let key = require_key(cfg, "google", "https://aistudio.google.com/app/apikey")?;
            Ok(Box::new(GoogleProvider::new(key)))
        }
        "groq" => {
            let key = require_key(cfg, "groq", "https://console.groq.com  [free]")?;
            Ok(Box::new(GroqProvider::new(key)))
        }
        "grok" | "xai" => {
            let key = require_key(cfg, "grok", "https://console.x.ai")?;
            Ok(Box::new(GrokProvider::new(key)))
        }
        "deepseek" => {
            let key = require_key(cfg, "deepseek", "https://platform.deepseek.com")?;
            Ok(Box::new(DeepSeekProvider::new(key)))
        }
        "mistral" | "codestral" => {
            let key = require_key(cfg, "mistral", "https://console.mistral.ai")?;
            Ok(Box::new(MistralProvider::new(key)))
        }
        "perplexity" | "sonar" | "pplx" => {
            let key = require_key(cfg, "perplexity", "https://www.perplexity.ai/settings/api")?;
            Ok(Box::new(PerplexityProvider::new(key)))
        }
        "together" | "togetherai" => {
            let key = require_key(cfg, "together", "https://api.together.ai")?;
            Ok(Box::new(TogetherProvider::new(key)))
        }
        "cohere" => {
            let key = require_key(cfg, "cohere", "https://dashboard.cohere.com/api-keys")?;
            Ok(Box::new(CohereProvider::new(key)))
        }
        _ => anyhow::bail!(
            "Unknown provider: '{}'\n  Available: openai, anthropic, google, groq, grok, deepseek, mistral, perplexity, together, cohere",
            name
        ),
    }
}

fn require_key<'a>(cfg: &'a Config, provider: &str, url: &str) -> Result<&'a str> {
    cfg.get_api_key(provider).ok_or_else(|| {
        anyhow::anyhow!(
            "{} API key not set.\n  Run: nion config set-key {} YOUR_KEY\n  Get key: {}",
            provider,
            provider,
            url
        )
    })
}
