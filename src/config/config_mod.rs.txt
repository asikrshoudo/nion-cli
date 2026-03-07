use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct Config {
    pub default_provider: Option<String>,
    pub default_model: Option<String>,
    pub user_name: Option<String>,
    #[serde(default)]
    pub api_keys: HashMap<String, String>,
}

impl Config {
    pub fn config_path() -> PathBuf {
        let home = dirs::home_dir().expect("Cannot find home directory");
        home.join(".nion").join("config.toml")
    }

    pub fn load() -> Result<Self> {
        let path = Self::config_path();
        if !path.exists() {
            return Ok(Config::default());
        }
        let content = std::fs::read_to_string(&path)?;
        let config: Config = toml::from_str(&content)?;
        Ok(config)
    }

    pub fn save(&self) -> Result<()> {
        let path = Self::config_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let content = toml::to_string_pretty(self)?;
        std::fs::write(&path, content)?;
        Ok(())
    }

    pub fn get_api_key(&self, provider: &str) -> Option<&str> {
        self.api_keys.get(provider).map(String::as_str)
    }

    pub fn set_api_key(&mut self, provider: &str, key: &str) {
        self.api_keys.insert(provider.to_string(), key.to_string());
    }

    pub fn is_first_run(&self) -> bool {
        self.user_name.is_none()
    }
}

pub async fn run_first_time_setup() -> Result<()> {
    use crate::ui;
    use std::io::{self, Write};

    let mut cfg = Config::load()?;

    if !cfg.is_first_run() {
        return Ok(());
    }

    println!();
    println!("  Welcome to Nion CLI.");
    println!("  Before we begin, I would like to know your name.");
    println!();
    print!("  What would you like me to call you? ");
    io::stdout().flush()?;

    let mut name = String::new();
    io::stdin().read_line(&mut name)?;
    let name = name.trim().to_string();

    cfg.user_name = Some(if name.is_empty() {
        "User".to_string()
    } else {
        name
    });

    cfg.save()?;

    let display_name = cfg.user_name.as_deref().unwrap_or("User");
    println!();
    ui::print_success(&format!(
        "Hello, {}. Your name has been saved.",
        display_name
    ));
    println!();

    // FIX: removed duplicate wizard call here.
    // Setup wizard only runs when user explicitly calls `nion config setup`.
    println!("  Run 'nion config setup' to add your API keys.");
    println!();

    Ok(())
}

pub async fn run_setup_wizard() -> Result<()> {
    use crate::ui;
    use std::io::{self, Write};

    println!();
    println!("  Nion Setup");
    println!("  Press Enter to skip any provider.");
    println!();

    let mut cfg = Config::load()?;

    let providers: Vec<(&str, &str, &str)> = vec![
        ("openai", "OpenAI", "https://platform.openai.com/api-keys"),
        ("anthropic", "Anthropic", "https://console.anthropic.com"),
        ("google", "Google", "https://aistudio.google.com/app/apikey"),
        (
            "groq",
            "Groq",
            "https://console.groq.com  [free tier available]",
        ),
        ("grok", "xAI Grok", "https://console.x.ai"),
        ("deepseek", "DeepSeek", "https://platform.deepseek.com"),
        ("mistral", "Mistral", "https://console.mistral.ai"),
        (
            "perplexity",
            "Perplexity",
            "https://www.perplexity.ai/settings/api",
        ),
        ("together", "Together AI", "https://api.together.ai"),
        ("cohere", "Cohere", "https://dashboard.cohere.com/api-keys"),
    ];

    for (id, name, url) in &providers {
        println!("  {} -- {}", name, url);
        print!("  API Key: ");
        io::stdout().flush()?;

        let mut key = String::new();
        io::stdin().read_line(&mut key)?;
        let key = key.trim();

        if !key.is_empty() {
            cfg.set_api_key(id, key);
            cfg.save()?;
            ui::print_success(&format!("{} key saved.", name));
        }
        println!();
    }

    print!("  Default provider [groq]: ");
    io::stdout().flush()?;
    let mut provider = String::new();
    io::stdin().read_line(&mut provider)?;
    let provider = provider.trim();

    cfg.default_provider = Some(if provider.is_empty() {
        "groq".to_string()
    } else {
        provider.to_string()
    });

    cfg.save()?;

    println!();
    ui::print_success("Setup complete.");
    println!("  Run 'nion chat' to start.");
    println!();

    Ok(())
}
