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

    // Telegram integration (optional)
    pub telegram_bot_token: Option<String>,
    #[serde(default)]
    pub telegram_allowed_users: Vec<i64>, // Telegram user IDs — empty = allow all

    // GitHub integration (optional)
    pub github_token: Option<String>,
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

    pub fn is_telegram_user_allowed(&self, user_id: i64) -> bool {
        if self.telegram_allowed_users.is_empty() {
            return true; // no restriction
        }
        self.telegram_allowed_users.contains(&user_id)
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
    ui::print_success(&format!("Hello, {}. Your name has been saved.", display_name));
    println!();
    println!("  Run 'nion config setup' to add your API keys.");
    println!();

    Ok(())
}

pub async fn run_setup_wizard() -> Result<()> {
    use crate::ui;
    use std::io::{self, Write};

    let providers: Vec<(&str, &str, &str)> = vec![
        ("openai",     "OpenAI",      "https://platform.openai.com/api-keys"),
        ("anthropic",  "Anthropic",   "https://console.anthropic.com"),
        ("google",     "Google",      "https://aistudio.google.com/app/apikey"),
        ("groq",       "Groq",        "https://console.groq.com  [free tier available]"),
        ("grok",       "xAI Grok",    "https://console.x.ai"),
        ("deepseek",   "DeepSeek",    "https://platform.deepseek.com"),
        ("mistral",    "Mistral",     "https://console.mistral.ai"),
        ("perplexity", "Perplexity",  "https://www.perplexity.ai/settings/api"),
        ("together",   "Together AI", "https://api.together.ai"),
        ("cohere",     "Cohere",      "https://dashboard.cohere.com/api-keys"),
    ];

    let mut cfg = Config::load()?;

    println!();
    println!("  Nion Setup");
    println!("  Select a provider to configure, then enter its API key.");
    println!();

    loop {
        let labels: Vec<String> = providers
            .iter()
            .map(|(id, name, _)| {
                if cfg.get_api_key(id).is_some() {
                    format!("{} [configured]", name)
                } else {
                    name.to_string()
                }
            })
            .collect();

        let mut menu_items = labels.clone();
        menu_items.push("Done — finish setup".to_string());

        let default_idx = providers
            .iter()
            .position(|(id, _, _)| cfg.default_provider.as_deref() == Some(id))
            .unwrap_or(3);

        let selected = ui::select_menu(&menu_items, default_idx)?;

        if selected == providers.len() {
            break;
        }

        let (id, name, url) = providers[selected];

        println!();
        println!("  {} -- {}", name, url);
        print!("  API Key: ");
        io::stdout().flush()?;

        let mut key = String::new();
        io::stdin().read_line(&mut key)?;
        let key = key.trim();

        if !key.is_empty() {
            cfg.set_api_key(id, key);
            cfg.default_provider = Some(id.to_string());
            cfg.save()?;
            println!();
            ui::print_success(&format!("{} key saved. Set as default provider.", name));
        } else {
            println!();
            ui::print_info("Skipped.");
        }

        println!();
    }

    // Telegram setup
    println!();
    println!("  --- Telegram Bot (optional) ---");
    println!("  Get a token from @BotFather on Telegram.");
    print!("  Telegram Bot Token (leave blank to skip): ");
    io::stdout().flush()?;

    let mut tg_token = String::new();
    io::stdin().read_line(&mut tg_token)?;
    let tg_token = tg_token.trim();

    if !tg_token.is_empty() {
        cfg.telegram_bot_token = Some(tg_token.to_string());

        print!("  Your Telegram User ID (for security, leave blank to allow all): ");
        io::stdout().flush()?;
        let mut tg_id = String::new();
        io::stdin().read_line(&mut tg_id)?;
        let tg_id = tg_id.trim();
        if !tg_id.is_empty() {
            if let Ok(id) = tg_id.parse::<i64>() {
                cfg.telegram_allowed_users = vec![id];
                ui::print_info(&format!("Only user ID {} can use the bot.", id));
            }
        }
        ui::print_success("Telegram bot token saved. Run 'nion serve' to start the bot.");
    }

    // GitHub setup
    println!();
    println!("  --- GitHub Token (optional) ---");
    println!("  Allows the agent to push to GitHub repos.");
    println!("  Generate at: https://github.com/settings/tokens");
    print!("  GitHub Token (leave blank to skip): ");
    io::stdout().flush()?;

    let mut gh_token = String::new();
    io::stdin().read_line(&mut gh_token)?;
    let gh_token = gh_token.trim();

    if !gh_token.is_empty() {
        cfg.github_token = Some(gh_token.to_string());
        ui::print_success("GitHub token saved.");
    }

    cfg.save()?;

    println!();
    ui::print_success("Setup complete.");
    println!("  Run 'nion chat' to start chatting.");
    println!("  Run 'nion serve' to start the Telegram bot.");
    println!();

    Ok(())
}
