use anyhow::Result;
use clap::{Parser, Subcommand};

use crate::{config, providers, session, ui};

#[derive(Parser)]
#[command(
    name = "nion",
    about = "Nion -- The Universal AI CLI\nOne tool. Every model. Every platform.",
    version,
    propagate_version = true,
    after_help = "Examples:\n  nion ask \"What is Rust?\"\n  nion ask -p gemini \"Explain transformers\"\n  nion chat\n  nion chat -p groq -m llama-3.3-70b-versatile\n  nion config setup\n\nProviders:\n  openai  anthropic  google  groq  grok  deepseek  mistral  perplexity  together  cohere"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Ask a single question
    Ask {
        #[arg(required = true, num_args = 1..)]
        question: Vec<String>,
        /// Provider  (openai, anthropic, google, groq, grok, deepseek, mistral, perplexity, together, cohere)
        #[arg(short, long)]
        provider: Option<String>,
        /// Model name
        #[arg(short, long)]
        model: Option<String>,
    },

    /// Start an interactive multi-turn chat session
    Chat {
        #[arg(short, long)]
        provider: Option<String>,
        #[arg(short, long)]
        model: Option<String>,
    },

    /// Manage configuration and API keys
    Config {
        #[command(subcommand)]
        action: ConfigAction,
    },

    /// Check for and install updates
    Update,
}

#[derive(Subcommand)]
enum ConfigAction {
    /// Interactive setup wizard
    Setup,
    /// Set an API key  e.g. nion config set-key groq YOUR_KEY
    SetKey { provider: String, key: String },
    /// Set default provider
    SetProvider { provider: String },
    /// Set default model
    SetModel { model: String },
    /// Set or change your name
    SetName { name: String },
    /// Show current config
    Show,
    /// List all available models
    ListModels,
}

pub async fn run() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Ask { question, provider, model } => {
            ask_command(&question.join(" "), provider.as_deref(), model.as_deref()).await
        }
        Commands::Chat { provider, model } => {
            chat_command(provider.as_deref(), model.as_deref()).await
        }
        Commands::Config { action } => config_command(action).await,
        Commands::Update => crate::updater::force_update().await,
    }
}

async fn ask_command(question: &str, provider_name: Option<&str>, model: Option<&str>) -> Result<()> {
    let cfg = config::Config::load()?;

    let provider_name = provider_name
        .map(String::from)
        .or_else(|| cfg.default_provider.clone())
        .unwrap_or_else(|| "groq".to_string());

    let provider = providers::get_provider(&provider_name, &cfg)?;

    let model = model
        .map(String::from)
        .or_else(|| cfg.default_model.clone())
        .unwrap_or_else(|| provider.default_model().to_string());

    let messages = vec![session::Message::user(question)];

    let spinner = ui::start_spinner(&format!("Thinking  {}  {}", provider.name(), model));
    let response = provider.complete(&messages, &model).await;
    spinner.finish_and_clear();

    match response {
        Ok(text) => { ui::print_response(&text); Ok(()) }
        Err(e) => Err(e),
    }
}

async fn chat_command(provider_name: Option<&str>, model: Option<&str>) -> Result<()> {
    let cfg = config::Config::load()?;

    let provider_name = provider_name
        .map(String::from)
        .or_else(|| cfg.default_provider.clone())
        .unwrap_or_else(|| "groq".to_string());

    let provider = providers::get_provider(&provider_name, &cfg)?;

    let model = model
        .map(String::from)
        .or_else(|| cfg.default_model.clone())
        .unwrap_or_else(|| provider.default_model().to_string());

    let mut current_provider_name = provider_name;
    let mut current_model = model;
    let mut history: Vec<session::Message> = Vec::new();
    let mut cfg = cfg;

    ui::print_chat_header(&cfg, &current_provider_name, &current_model);

    loop {
        let name = cfg.user_name.as_deref().unwrap_or("You");
        let input = match ui::read_user_input(name) {
            Ok(i) => i,
            Err(_) => break,
        };

        if input.trim().is_empty() {
            continue;
        }

        let lower = input.trim().to_lowercase();

        // -- Slash commands --
        if lower == "/exit" || lower == "/quit" {
            let n = cfg.user_name.as_deref().unwrap_or("User");
            ui::print_goodbye(n);
            break;
        }

        if lower == "/clear" {
            history.clear();
            ui::print_info("History cleared.");
            continue;
        }

        if lower == "/help" {
            ui::print_chat_help();
            continue;
        }

        if lower.starts_with("/model ") {
            let new_model = input.trim()[7..].trim().to_string();
            if new_model.is_empty() {
                ui::print_error("Usage: /model <model-name>");
            } else {
                current_model = new_model;
                ui::print_info(&format!("Model: {}", current_model));
            }
            continue;
        }

        if lower.starts_with("/switch ") {
            let new_provider = input.trim()[8..].trim().to_string();
            match providers::get_provider(&new_provider, &cfg) {
                Ok(p) => {
                    current_model = p.default_model().to_string();
                    current_provider_name = new_provider;
                    history.clear();
                    ui::print_info(&format!("Switched to {} ({}). History cleared.", current_provider_name, current_model));
                }
                Err(e) => ui::print_error(&format!("{}", e)),
            }
            continue;
        }

        if lower.starts_with("/name ") {
            let new_name = input.trim()[6..].trim().to_string();
            if new_name.is_empty() {
                ui::print_error("Usage: /name <your-name>");
            } else {
                cfg.user_name = Some(new_name.clone());
                cfg.save()?;
                ui::print_success(&format!("Name updated to: {}", new_name));
            }
            continue;
        }

        // -- Normal message --
        history.push(session::Message::user(&input));

        let provider = match providers::get_provider(&current_provider_name, &cfg) {
            Ok(p) => p,
            Err(e) => {
                ui::print_error(&format!("{}", e));
                history.pop();
                continue;
            }
        };

        let spinner = ui::start_spinner("Thinking...");
        let response = provider.complete(&history, &current_model).await;
        spinner.finish_and_clear();

        match response {
            Ok(text) => {
                history.push(session::Message::assistant(&text));
                ui::print_response(&text);
            }
            Err(e) => {
                history.pop();
                ui::print_error(&format!("{}", e));
            }
        }
    }

    Ok(())
}

async fn config_command(action: ConfigAction) -> Result<()> {
    match action {
        ConfigAction::Setup => {
            config::run_setup_wizard().await?;
        }
        ConfigAction::SetKey { provider, key } => {
            let mut cfg = config::Config::load()?;
            cfg.set_api_key(&provider, &key);
            cfg.save()?;
            ui::print_success(&format!("Key for '{}' saved.", provider));
        }
        ConfigAction::SetProvider { provider } => {
            let mut cfg = config::Config::load()?;
            cfg.default_provider = Some(provider.clone());
            cfg.save()?;
            ui::print_success(&format!("Default provider: {}", provider));
        }
        ConfigAction::SetModel { model } => {
            let mut cfg = config::Config::load()?;
            cfg.default_model = Some(model.clone());
            cfg.save()?;
            ui::print_success(&format!("Default model: {}", model));
        }
        ConfigAction::SetName { name } => {
            let mut cfg = config::Config::load()?;
            cfg.user_name = Some(name.clone());
            cfg.save()?;
            ui::print_success(&format!("Name updated to: {}", name));
        }
        ConfigAction::Show => {
            let cfg = config::Config::load()?;
            ui::print_config(&cfg);
        }
        ConfigAction::ListModels => {
            ui::print_models_list();
        }
    }
    Ok(())
}
