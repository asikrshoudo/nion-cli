use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

use anyhow::Result;
use teloxide::prelude::*;
use teloxide::utils::command::BotCommands;

use crate::config::Config;
use crate::providers;
use crate::session::Message as AiMessage;
use crate::tools;

#[derive(Clone)]
struct ChatSession {
    history: Vec<AiMessage>,
    provider_id: String,
    model: String,
}

impl ChatSession {
    fn new(provider_id: String, model: String) -> Self {
        Self { history: Vec::new(), provider_id, model }
    }
}

type Sessions = Arc<Mutex<HashMap<i64, ChatSession>>>;

#[derive(BotCommands, Clone)]
#[command(rename_rule = "lowercase", description = "Nion Bot commands:")]
enum Command {
    #[command(description = "Show help")]
    Help,
    #[command(description = "Clear conversation history")]
    Clear,
    #[command(description = "Switch provider: /switch groq")]
    Switch(String),
    #[command(description = "Switch model: /model llama-3.3-70b-versatile")]
    Model(String),
    #[command(description = "Ask without history: /ask question")]
    Ask(String),
    #[command(description = "Show current provider and model")]
    Status,
    #[command(description = "Start")]
    Start,
}

pub async fn run_serve(cfg: Config) -> Result<()> {
    let token = cfg.telegram_bot_token.as_ref().expect("telegram_bot_token must be set");
    let bot = Bot::new(token);

    let default_provider_id = cfg.default_provider.clone().unwrap_or_else(|| "groq".to_string());
    let default_model = match providers::get_provider(&default_provider_id, &cfg) {
        Ok(p) => cfg.default_model.clone().unwrap_or_else(|| p.default_model().to_string()),
        Err(_) => "llama-3.3-70b-versatile".to_string(),
    };

    let sessions: Sessions = Arc::new(Mutex::new(HashMap::new()));
    let cfg = Arc::new(cfg);

    println!("  Bot running. Waiting for messages...");

    let handler = Update::filter_message()
        .branch(
            dptree::entry()
                .filter_command::<Command>()
                .endpoint({
                    let sessions = sessions.clone();
                    let cfg = cfg.clone();
                    let dp = default_provider_id.clone();
                    let dm = default_model.clone();
                    move |bot: Bot, msg: teloxide::types::Message, cmd: Command| {
                        handle_command(bot, msg, cmd, sessions.clone(), cfg.clone(), dp.clone(), dm.clone())
                    }
                }),
        )
        .branch(dptree::endpoint({
            let sessions = sessions.clone();
            let cfg = cfg.clone();
            let dp = default_provider_id.clone();
            let dm = default_model.clone();
            move |bot: Bot, msg: teloxide::types::Message| {
                handle_message(bot, msg, sessions.clone(), cfg.clone(), dp.clone(), dm.clone())
            }
        }));

    Dispatcher::builder(bot, handler).build().dispatch().await;
    Ok(())
}

fn is_allowed(cfg: &Config, user_id: i64) -> bool {
    cfg.is_telegram_user_allowed(user_id)
}

/// Extract a friendly error message — especially for rate limits
fn friendly_error(e: &str) -> String {
    if e.contains("429") || e.contains("rate_limit") || e.contains("Too Many Requests") {
        // Try to extract retry time
        if let Some(pos) = e.find("Please try again in ") {
            let rest = &e[pos + 20..];
            let secs = rest.split('s').next().unwrap_or("a few seconds");
            return format!("⏳ Rate limit reached. Try again in {}.", secs);
        }
        return "⏳ Rate limit reached. Please wait a moment and try again.".to_string();
    }
    if e.contains("401") || e.contains("Unauthorized") {
        return "❌ Invalid API key. Run 'nion config setup' to fix.".to_string();
    }
    if e.contains("timeout") || e.contains("Timeout") {
        return "⏱ Request timed out. Try again.".to_string();
    }
    format!("❌ Error: {}", e)
}

async fn handle_command(
    bot: Bot,
    msg: teloxide::types::Message,
    cmd: Command,
    sessions: Sessions,
    cfg: Arc<Config>,
    default_provider_id: String,
    default_model: String,
) -> Result<(), teloxide::RequestError> {
    let chat_id = msg.chat.id;
    let user_id = msg.from().map(|u| u.id.0 as i64).unwrap_or(0);

    if !is_allowed(&cfg, user_id) {
        bot.send_message(chat_id, "⛔ Not authorized.").await?;
        return Ok(());
    }

    match cmd {
        Command::Start | Command::Help => {
            bot.send_message(chat_id,
                "Nion AI Bot\n\
                One tool. Every model. Every platform.\n\n\
                Send any message — I'll respond and can run commands,\n\
                read/write files, and push to GitHub.\n\n\
                /clear — clear history\n\
                /status — current provider & model\n\
                /switch groq — change provider\n\
                /model llama-3.3-70b-versatile — change model\n\
                /ask question — single question (no history)\n\n\
                Providers: groq, openai, anthropic, google, grok,\n\
                deepseek, mistral, perplexity, together, cohere"
            ).await?;
        }

        Command::Clear => {
            let mut sessions = sessions.lock().await;
            if let Some(s) = sessions.get_mut(&chat_id.0) { s.history.clear(); }
            bot.send_message(chat_id, "🗑 History cleared.").await?;
        }

        Command::Status => {
            let sessions = sessions.lock().await;
            let (provider, model) = if let Some(s) = sessions.get(&chat_id.0) {
                (s.provider_id.clone(), s.model.clone())
            } else {
                (default_provider_id.clone(), default_model.clone())
            };
            bot.send_message(chat_id, format!("Provider: {}\nModel: {}", provider, model)).await?;
        }

        Command::Switch(provider_id) => {
            let provider_id = provider_id.trim().to_string();
            if provider_id.is_empty() {
                bot.send_message(chat_id, "Usage: /switch groq").await?;
                return Ok(());
            }
            match providers::get_provider(&provider_id, &cfg) {
                Ok(p) => {
                    let new_model = p.default_model().to_string();
                    let mut sessions = sessions.lock().await;
                    let s = sessions.entry(chat_id.0)
                        .or_insert_with(|| ChatSession::new(default_provider_id.clone(), default_model.clone()));
                    s.provider_id = provider_id.clone();
                    s.model = new_model.clone();
                    bot.send_message(chat_id, format!("✅ Switched to {} ({})", provider_id, new_model)).await?;
                }
                Err(e) => { bot.send_message(chat_id, friendly_error(&e.to_string())).await?; }
            }
        }

        Command::Model(model_name) => {
            let model_name = model_name.trim().to_string();
            if model_name.is_empty() {
                bot.send_message(chat_id, "Usage: /model llama-3.3-70b-versatile").await?;
                return Ok(());
            }
            let mut sessions = sessions.lock().await;
            let s = sessions.entry(chat_id.0)
                .or_insert_with(|| ChatSession::new(default_provider_id.clone(), default_model.clone()));
            s.model = model_name.clone();
            bot.send_message(chat_id, format!("✅ Model: {}", model_name)).await?;
        }

        Command::Ask(question) => {
            if question.trim().is_empty() {
                bot.send_message(chat_id, "Usage: /ask your question here").await?;
                return Ok(());
            }
            let (provider_id, model) = {
                let sessions = sessions.lock().await;
                if let Some(s) = sessions.get(&chat_id.0) {
                    (s.provider_id.clone(), s.model.clone())
                } else {
                    (default_provider_id.clone(), default_model.clone())
                }
            };
            let thinking = bot.send_message(chat_id, "⏳").await?;
            let messages = vec![AiMessage::user(question.trim())];
            let response = match providers::get_provider(&provider_id, &cfg) {
                Ok(p) => match p.complete(&messages, &model).await {
                    Ok(r) => r,
                    Err(e) => friendly_error(&e.to_string()),
                },
                Err(e) => friendly_error(&e.to_string()),
            };
            bot.delete_message(chat_id, thinking.id).await.ok();
            send_long_message(&bot, chat_id, &response).await?;
        }
    }

    Ok(())
}

async fn handle_message(
    bot: Bot,
    msg: teloxide::types::Message,
    sessions: Sessions,
    cfg: Arc<Config>,
    default_provider_id: String,
    default_model: String,
) -> Result<(), teloxide::RequestError> {
    let text = match msg.text() {
        Some(t) => t.to_string(),
        None => return Ok(()),
    };

    let chat_id = msg.chat.id;
    let user_id = msg.from().map(|u| u.id.0 as i64).unwrap_or(0);

    if !is_allowed(&cfg, user_id) {
        bot.send_message(chat_id, "⛔ Not authorized.").await?;
        return Ok(());
    }

    if text.trim().is_empty() { return Ok(()); }

    let (provider_id, model) = {
        let sessions = sessions.lock().await;
        if let Some(s) = sessions.get(&chat_id.0) {
            (s.provider_id.clone(), s.model.clone())
        } else {
            (default_provider_id.clone(), default_model.clone())
        }
    };

    {
        let mut sessions = sessions.lock().await;
        let s = sessions.entry(chat_id.0)
            .or_insert_with(|| ChatSession::new(default_provider_id.clone(), default_model.clone()));
        s.history.push(AiMessage::user(&text));
    }

    let provider = match providers::get_provider(&provider_id, &cfg) {
        Ok(p) => p,
        Err(e) => {
            bot.send_message(chat_id, friendly_error(&e.to_string())).await?;
            return Ok(());
        }
    };

    let github_token = cfg.github_token.clone();
    let thinking_msg = bot.send_message(chat_id, "⏳").await?;

    let mut tool_count = 0u32;
    const MAX_TOOL_CALLS: u32 = 20;

    loop {
        let history = {
            let sessions = sessions.lock().await;
            sessions.get(&chat_id.0).map(|s| s.history.clone()).unwrap_or_default()
        };

        let response = match provider.complete_with_system(&history, &model, tools::SYSTEM_PROMPT).await {
            Ok(r) => r,
            Err(e) => {
                bot.delete_message(chat_id, thinking_msg.id).await.ok();
                {
                    let mut sessions = sessions.lock().await;
                    if let Some(s) = sessions.get_mut(&chat_id.0) { s.history.pop(); }
                }
                bot.send_message(chat_id, friendly_error(&e.to_string())).await?;
                return Ok(());
            }
        };

        if let Some((tool_name, tool_input)) = tools::parse_tool_call(&response) {
            tool_count += 1;

            // Show thinking text before tool if any
            let before = tools::text_before_tool(&response);
            if !before.is_empty() {
                bot.delete_message(chat_id, thinking_msg.id).await.ok();
                send_long_message(&bot, chat_id, before).await?;
            }

            // Execute tool silently
            let tool_result = tools::execute_tool(&tool_name, &tool_input, github_token.as_deref());

            {
                let mut sessions = sessions.lock().await;
                if let Some(s) = sessions.get_mut(&chat_id.0) {
                    s.history.push(AiMessage::assistant(&response));
                    s.history.push(AiMessage::user(&format!(
                        "[Tool: {}]\n[Result]\n{}", tool_name, tool_result
                    )));
                }
            }

            if tool_count >= MAX_TOOL_CALLS {
                bot.delete_message(chat_id, thinking_msg.id).await.ok();
                bot.send_message(chat_id, "⚠️ Too many steps. Task stopped.").await?;
                break;
            }

            // Update thinking indicator
            bot.delete_message(chat_id, thinking_msg.id).await.ok();
            let _ = bot.send_message(chat_id, "⏳").await;
            continue;
        }

        // Final answer
        bot.delete_message(chat_id, thinking_msg.id).await.ok();
        {
            let mut sessions = sessions.lock().await;
            if let Some(s) = sessions.get_mut(&chat_id.0) {
                s.history.push(AiMessage::assistant(&response));
            }
        }
        send_long_message(&bot, chat_id, &response).await?;
        break;
    }

    Ok(())
}

async fn send_long_message(bot: &Bot, chat_id: ChatId, text: &str) -> Result<(), teloxide::RequestError> {
    const LIMIT: usize = 4000;
    if text.len() <= LIMIT {
        bot.send_message(chat_id, text).await?;
        return Ok(());
    }
    let mut chunk = String::new();
    for line in text.lines() {
        if chunk.len() + line.len() + 1 > LIMIT {
            if !chunk.is_empty() {
                bot.send_message(chat_id, &chunk).await?;
                chunk.clear();
            }
        }
        if !chunk.is_empty() { chunk.push('\n'); }
        chunk.push_str(line);
    }
    if !chunk.is_empty() {
        bot.send_message(chat_id, &chunk).await?;
    }
    Ok(())
}
