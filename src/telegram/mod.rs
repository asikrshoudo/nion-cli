use anyhow::Result;
use std::collections::HashMap;
use std::sync::Arc;
use teloxide::prelude::*;
use teloxide::utils::command::BotCommands;
use tokio::sync::Mutex;

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
                    move |bot: Bot, msg: Message, cmd: Command| {
                        handle_command(bot, msg, cmd, sessions.clone(), cfg.clone(), dp.clone(), dm.clone())
                    }
                }),
        )
        .branch(dptree::endpoint({
            let sessions = sessions.clone();
            let cfg = cfg.clone();
            let dp = default_provider_id.clone();
            let dm = default_model.clone();
            move |bot: Bot, msg: Message| {
                handle_message(bot, msg, sessions.clone(), cfg.clone(), dp.clone(), dm.clone())
            }
        }));

    Dispatcher::builder(bot, handler).build().dispatch().await;
    Ok(())
}

fn is_allowed(cfg: &Config, user_id: i64) -> bool {
    cfg.is_telegram_user_allowed(user_id)
}

fn friendly_error(e: &str) -> String {
    if e.contains("429") || e.contains("rate_limit") {
        return "Rate limit reached. Please wait and try again.".to_string();
    }
    if e.contains("401") || e.contains("Unauthorized") {
        return "Invalid API key. Run 'nion config setup' to fix.".to_string();
    }
    if e.contains("timeout") {
        return "Request timed out. Try again.".to_string();
    }
    format!("Error: {}", e)
}

async fn handle_command(
    bot: Bot,
    msg: Message,
    cmd: Command,
    sessions: Sessions,
    cfg: Arc<Config>,
    default_provider_id: String,
    default_model: String,
) -> Result<(), teloxide::RequestError> {
    let chat_id = msg.chat.id;
    let user_id = msg.from().map(|u| u.id.0 as i64).unwrap_or(0);

    if !is_allowed(&cfg, user_id) {
        bot.send_message(chat_id, "Not authorized.").await?;
        return Ok(());
    }

    match cmd {
        Command::Start | Command::Help => {
            bot.send_message(chat_id, "Nion AI Bot\nOne tool. Every model.\n\nSend any message for agent mode.\nCommands: /clear /status /switch <p> /model <m> /ask <q>").await?;
        }
        Command::Clear => {
            let mut s = sessions.lock().await;
            if let Some(session) = s.get_mut(&chat_id.0) { session.history.clear(); }
            bot.send_message(chat_id, "History cleared.").await?;
        }
        Command::Status => {
            let s = sessions.lock().await;
            let (p, m) = if let Some(session) = s.get(&chat_id.0) {
                (session.provider_id.clone(), session.model.clone())
            } else {
                (default_provider_id.clone(), default_model.clone())
            };
            bot.send_message(chat_id, format!("Provider: {}\nModel: {}", p, m)).await?;
        }
        Command::Switch(provider_id) => {
            let pid = provider_id.trim().to_string();
            if pid.is_empty() { bot.send_message(chat_id, "Usage: /switch groq").await?; return Ok(()); }
            match providers::get_provider(&pid, &cfg) {
                Ok(p) => {
                    let new_model = p.default_model().to_string();
                    let mut s = sessions.lock().await;
                    let session = s.entry(chat_id.0).or_insert_with(|| ChatSession::new(default_provider_id.clone(), default_model.clone()));
                    session.provider_id = pid.clone();
                    session.model = new_model.clone();
                    bot.send_message(chat_id, format!("Switched to {} ({})", pid, new_model)).await?;
                }
                Err(e) => { bot.send_message(chat_id, friendly_error(&e.to_string())).await?; }
            }
        }
        Command::Model(model_name) => {
            let m = model_name.trim().to_string();
            if m.is_empty() { bot.send_message(chat_id, "Usage: /model <name>").await?; return Ok(()); }
            let mut s = sessions.lock().await;
            let session = s.entry(chat_id.0).or_insert_with(|| ChatSession::new(default_provider_id.clone(), default_model.clone()));
            session.model = m.clone();
            bot.send_message(chat_id, format!("Model: {}", m)).await?;
        }
        Command::Ask(question) => {
            if question.trim().is_empty() { bot.send_message(chat_id, "Usage: /ask <question>").await?; return Ok(()); }
            let (pid, m) = {
                let s = sessions.lock().await;
                if let Some(session) = s.get(&chat_id.0) {
                    (session.provider_id.clone(), session.model.clone())
                } else {
                    (default_provider_id.clone(), default_model.clone())
                }
            };
            let thinking = bot.send_message(chat_id, "Thinking...").await?;
            let messages = vec![AiMessage::user(question.trim())];
            let response = match providers::get_provider(&pid, &cfg) {
                Ok(p) => p.complete(&messages, &m).await.unwrap_or_else(|e| friendly_error(&e.to_string())),
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
    msg: Message,
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
        bot.send_message(chat_id, "Not authorized.").await?;
        return Ok(());
    }
    if text.trim().is_empty() { return Ok(()); }

    let (provider_id, model) = {
        let s = sessions.lock().await;
        if let Some(session) = s.get(&chat_id.0) {
            (session.provider_id.clone(), session.model.clone())
        } else {
            (default_provider_id.clone(), default_model.clone())
        }
    };

    {
        let mut s = sessions.lock().await;
        let session = s.entry(chat_id.0)
            .or_insert_with(|| ChatSession::new(default_provider_id.clone(), default_model.clone()));
        session.history.push(AiMessage::user(&text));
    }

    let provider = match providers::get_provider(&provider_id, &cfg) {
        Ok(p) => p,
        Err(e) => { bot.send_message(chat_id, friendly_error(&e.to_string())).await?; return Ok(()); }
    };

    let github_token = cfg.github_token.clone();
    let thinking = bot.send_message(chat_id, "Thinking...").await?;

    let mut tool_count = 0u32;
    const MAX_TOOL_CALLS: u32 = 20;

    loop {
        let history = {
            let s = sessions.lock().await;
            s.get(&chat_id.0).map(|s| s.history.clone()).unwrap_or_default()
        };

        let response = match provider.complete_with_system(&history, &model, tools::SYSTEM_PROMPT).await {
            Ok(r) => r,
            Err(e) => {
                bot.delete_message(chat_id, thinking.id).await.ok();
                let mut s = sessions.lock().await;
                if let Some(session) = s.get_mut(&chat_id.0) { session.history.pop(); }
                bot.send_message(chat_id, friendly_error(&e.to_string())).await?;
                return Ok(());
            }
        };

        if let Some((tool_name, tool_input)) = tools::parse_tool_call(&response) {
            tool_count += 1;

            let before = tools::text_before_tool(&response);
            if !before.is_empty() {
                bot.delete_message(chat_id, thinking.id).await.ok();
                send_long_message(&bot, chat_id, before).await?;
            }

            let tool_result = tools::execute_tool(&tool_name, &tool_input, github_token.as_deref());

            {
                let mut s = sessions.lock().await;
                if let Some(session) = s.get_mut(&chat_id.0) {
                    session.history.push(AiMessage::assistant(&response));
                    session.history.push(AiMessage::user(&format!("[Tool: {}]\n[Result]\n{}", tool_name, tool_result)));
                }
            }

            if tool_count >= MAX_TOOL_CALLS {
                bot.delete_message(chat_id, thinking.id).await.ok();
                bot.send_message(chat_id, "Too many steps. Task stopped.").await?;
                break;
            }

            bot.delete_message(chat_id, thinking.id).await.ok();
            let _ = bot.send_message(chat_id, "Thinking...").await;
            continue;
        }

        // Final answer
        bot.delete_message(chat_id, thinking.id).await.ok();
        {
            let mut s = sessions.lock().await;
            if let Some(session) = s.get_mut(&chat_id.0) {
                session.history.push(AiMessage::assistant(&response));
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