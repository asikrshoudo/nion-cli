use anyhow::Result;
use clap::{Parser, Subcommand};

use crate::{config, providers, ui};

#[derive(Parser)]
#[command(name = "nion", about = "The Universal AI CLI", version)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Start an interactive chat session
    Chat {
        #[arg(short, long)]
        provider: Option<String>,
        #[arg(short, long)]
        model: Option<String>,
    },

    /// Start an agentic session — AI can read/write files and run commands
    Agent {
        #[arg(short, long)]
        provider: Option<String>,
        #[arg(short, long)]
        model: Option<String>,
    },

    /// Ask a single question and get a response
    Ask {
        question: Vec<String>,
        #[arg(short, long)]
        provider: Option<String>,
        #[arg(short, long)]
        model: Option<String>,
    },

    /// Start the Telegram bot (runs until stopped)
    Serve,

    /// Configuration commands
    Config {
        #[command(subcommand)]
        action: ConfigAction,
    },

    /// List all available models
    Models,

    /// Check for and apply updates
    Update,
}

#[derive(Subcommand)]
enum ConfigAction {
    /// Interactive setup wizard
    Setup,
    /// Set an API key: nion config set-key <provider> <key>
    SetKey {
        provider: String,
        key: String,
    },
    /// Set Telegram bot token: nion config set-telegram <token>
    SetTelegram {
        token: String,
    },
    /// Set GitHub token: nion config set-github <token>
    SetGithub {
        token: String,
    },
    /// Show current configuration
    Show,
}

pub async fn run() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        None | Some(Commands::Chat { provider: None, model: None }) => {
            run_chat(None, None).await?;
        }

        Some(Commands::Chat { provider, model }) => {
            run_chat(provider.as_deref(), model.as_deref()).await?;
        }

        Some(Commands::Agent { provider, model }) => {
            crate::agent::run(provider.as_deref(), model.as_deref()).await?;
        }

        Some(Commands::Ask { question, provider, model }) => {
            let q = question.join(" ");
            if q.trim().is_empty() {
                ui::print_error("Please provide a question. Example: nion ask \"Hello\"");
                return Ok(());
            }
            run_ask(&q, provider.as_deref(), model.as_deref()).await?;
        }

        Some(Commands::Serve) => {
            let cfg = config::Config::load()?;
            if cfg.telegram_bot_token.is_none() {
                ui::print_error("No Telegram bot token configured.");
                ui::print_info("Run 'nion config set-telegram <token>' or 'nion config setup'.");
                return Ok(());
            }
            ui::print_success("Starting Nion Telegram bot...");
            ui::print_info("Press Ctrl+C to stop.");
            crate::telegram::run_serve(cfg).await?;
        }

        Some(Commands::Config { action }) => match action {
            ConfigAction::Setup => {
                config::run_setup_wizard().await?;
            }
            ConfigAction::SetKey { provider, key } => {
                let mut cfg = config::Config::load()?;
                cfg.set_api_key(&provider, &key);
                if cfg.default_provider.is_none() {
                    cfg.default_provider = Some(provider.clone());
                }
                cfg.save()?;
                ui::print_success(&format!("{} API key saved.", provider));
            }
            ConfigAction::SetTelegram { token } => {
                let mut cfg = config::Config::load()?;
                cfg.telegram_bot_token = Some(token);
                cfg.save()?;
                ui::print_success("Telegram bot token saved. Run 'nion serve' to start.");
            }
            ConfigAction::SetGithub { token } => {
                let mut cfg = config::Config::load()?;
                cfg.github_token = Some(token);
                cfg.save()?;
                ui::print_success("GitHub token saved.");
            }
            ConfigAction::Show => {
                let cfg = config::Config::load()?;
                ui::print_config(&cfg);
            }
        },

        Some(Commands::Models) => {
            ui::print_models_list();
        }

        Some(Commands::Update) => {
            crate::updater::force_update().await?;
        }
    }

    Ok(())
}

async fn run_chat(provider_name: Option<&str>, model_override: Option<&str>) -> Result<()> {
    use crate::session::Message;

    let cfg = config::Config::load()?;

    let mut current_provider_id = provider_name
        .map(String::from)
        .or_else(|| cfg.default_provider.clone())
        .unwrap_or_else(|| "groq".to_string());

    // Build provider once — not on every message
    let mut provider = providers::get_provider(&current_provider_id, &cfg)?;

    let mut current_model = model_override
        .map(String::from)
        .or_else(|| cfg.default_model.clone())
        .unwrap_or_else(|| provider.default_model().to_string());

    let mut history: Vec<Message> = Vec::new();

    ui::print_chat_header(&cfg, &current_provider_id, &current_model);

    loop {
        let name = cfg.user_name.as_deref().unwrap_or("You");
        let input = match ui::read_user_input(name) {
            Ok(s) => s,
            Err(_) => break,
        };

        if input.trim().is_empty() {
            continue;
        }

        if input.starts_with('/') {
            let parts: Vec<&str> = input.splitn(2, ' ').collect();
            match parts[0] {
                "/exit" | "/quit" => {
                    ui::print_goodbye(cfg.user_name.as_deref().unwrap_or("User"));
                    break;
                }
                "/clear" => {
                    history.clear();
                    ui::print_info("History cleared.");
                    continue;
                }
                "/help" => {
                    ui::print_chat_help();
                    continue;
                }
                "/model" => {
                    if let Some(m) = parts.get(1) {
                        current_model = m.trim().to_string();
                        ui::print_info(&format!("Model switched to: {}", current_model));
                    } else {
                        ui::print_error("Usage: /model <model-name>");
                    }
                    continue;
                }
                "/switch" => {
                    if let Some(p) = parts.get(1) {
                        let pid = p.trim().to_string();
                        match providers::get_provider(&pid, &cfg) {
                            Ok(new_p) => {
                                current_model = new_p.default_model().to_string();
                                current_provider_id = pid;
                                provider = new_p;
                                ui::print_info(&format!(
                                    "Switched to {} ({})",
                                    current_provider_id, current_model
                                ));
                            }
                            Err(e) => ui::print_error(&format!("{}", e)),
                        }
                    } else {
                        ui::print_error("Usage: /switch <provider>");
                    }
                    continue;
                }
                "/name" => {
                    if let Some(new_name) = parts.get(1) {
                        let mut c = config::Config::load()?;
                        c.user_name = Some(new_name.trim().to_string());
                        c.save()?;
                        ui::print_info(&format!("Name updated to: {}", new_name.trim()));
                    } else {
                        ui::print_error("Usage: /name <new_name>");
                    }
                    continue;
                }
                _ => {
                    ui::print_error(&format!("Unknown command: {}. Type /help for help.", parts[0]));
                    continue;
                }
            }
        }

        history.push(Message::user(&input));

        let spinner = ui::start_spinner("Thinking...");
        let result = provider.complete(&history, &current_model).await;
        spinner.finish_and_clear();

        match result {
            Ok(response) => {
                history.push(Message::assistant(&response));
                ui::print_response(&response);
            }
            Err(e) => {
                ui::print_error(&format!("{}", e));
                history.pop();
            }
        }
    }

    Ok(())
}

async fn run_ask(question: &str, provider_name: Option<&str>, model_override: Option<&str>) -> Result<()> {
    let cfg = config::Config::load()?;

    let provider_id = provider_name
        .map(String::from)
        .or_else(|| cfg.default_provider.clone())
        .unwrap_or_else(|| "groq".to_string());

    let provider = providers::get_provider(&provider_id, &cfg)?;

    let model = model_override
        .map(String::from)
        .or_else(|| cfg.default_model.clone())
        .unwrap_or_else(|| provider.default_model().to_string());

    let messages = vec![crate::session::Message::user(question)];

    let spinner = ui::start_spinner("Thinking...");
    let result = provider.complete(&messages, &model).await;
    spinner.finish_and_clear();

    match result {
        Ok(response) => ui::print_response(&response),
        Err(e) => ui::print_error(&format!("{}", e)),
    }

    Ok(())
}
