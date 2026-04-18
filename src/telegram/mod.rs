use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

use anyhow::Result;
use teloxide::prelude::*;
use teloxide::types::ParseMode;
use teloxide::utils::command::BotCommands;

use crate::config::Config;
use crate::providers;
use crate::session::Message;
use crate::tools;

// ── Per-chat session state ────────────────────────────────────────────────

#[derive(Clone)]
struct ChatSession {
    history: Vec<Message>,
    provider_id: String,
    model: String,
}

impl ChatSession {
    fn new(provider_id: String, model: String) -> Self {
        Self {
            history: Vec::new(),
            provider_id,
            model,
        }
    }
}

type Sessions = Arc<Mutex<HashMap<i64, ChatSession>>>;

// ── Bot commands ──────────────────────────────────────────────────────────

#[derive(BotCommands, Clone)]
#[command(rename_rule = "lowercase", description = "Nion Bot commands:")]
enum Command {
    #[command(description = "Show this help message")]
    Help,
    #[command(description = "Clear your conversation history")]
    Clear,
    #[command(description = "Switch provider: /switch groq")]
    Switch(String),
    #[command(description = "Switch model: /model llama-3.3-70b-versatile")]
    Model(String),
    #[command(description = "Ask a single question without history")]
    Ask(String),
    #[command(description = "Show current provider and model")]
    Status,
    #[command(description = "Start / restart the bot")]
    Start,
}

// ── Entry point ───────────────────────────────────────────────────────────

pub async fn run_serve(cfg: Config) -> Result<()> {
    let token = cfg
        .telegram_bot_token
        .as_ref()
        .expect("telegram_bot_token must be set before calling run_serve");

    let bot = Bot::new(token);

    // Determine default provider and model
    let default_provider_id = cfg
        .default_provider
        .clone()
        .unwrap_or_else(|| "groq".to_string());

    let default_model = {
        match providers::get_provider(&default_provider_id, &cfg) {
            Ok(p) => cfg
                .default_model
                .clone()
                .unwrap_or_else(|| p.default_model().to_string()),
            Err(_) => "llama-3.3-70b-versatile".to_string(),
        }
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
                    let default_provider_id = default_provider_id.clone();
                    let default_model = default_model.clone();
                    move |bot: Bot, msg: Message, cmd: Command| {
                        handle_command(
                            bot,
                            msg,
                            cmd,
                            sessions.clone(),
                            cfg.clone(),
                            default_provider_id.clone(),
                            default_model.clone(),
                        )
                    }
                }),
        )
        .branch(dptree::endpoint({
            let sessions = sessions.clone();
            let cfg = cfg.clone();
            let default_provider_id = default_provider_id.clone();
            let default_model = default_model.clone();
            move |bot: Bot, msg: Message| {
                handle_message(
                    bot,
                    msg,
                    sessions.clone(),
                    cfg.clone(),
                    default_provider_id.clone(),
                    default_model.clone(),
                )
            }
        }));

    Dispatcher::builder(bot, handler)
        .enable_ctrlc_handler()
        .build()
        .dispatch()
        .await;

    Ok(())
}

// ── Security check ────────────────────────────────────────────────────────

fn is_allowed(cfg: &Config, user_id: i64) -> bool {
    cfg.is_telegram_user_allowed(user_id)
}

// ── Command handler ───────────────────────────────────────────────────────

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
        bot.send_message(chat_id, "⛔ You are not authorized to use this bot.")
            .await?;
        return Ok(());
    }

    match cmd {
        Command::Start | Command::Help => {
            let text = format!(
                "*Nion AI Bot* 🤖\n\
                One tool. Every model. Every platform.\n\n\
                Just send any message and I'll respond using AI.\n\
                I can also run commands, read/write files, and push to GitHub.\n\n\
                *Commands:*\n\
                /help — show this message\n\
                /clear — clear conversation history\n\
                /switch <provider> — change AI provider\n\
                /model <model> — change model\n\
                /ask <question> — single question (no history)\n\
                /status — current provider & model\n\n\
                *Available providers:*\n\
                groq, openai, anthropic, google, grok,\n\
                deepseek, mistral, perplexity, together, cohere"
            );
            bot.send_message(chat_id, text)
                .parse_mode(ParseMode::Markdown)
                .await?;
        }

        Command::Clear => {
            let mut sessions = sessions.lock().await;
            if let Some(s) = sessions.get_mut(&chat_id.0) {
                s.history.clear();
            }
            bot.send_message(chat_id, "🗑 History cleared.").await?;
        }

        Command::Status => {
            let sessions = sessions.lock().await;
            let (provider, model) = if let Some(s) = sessions.get(&chat_id.0) {
                (s.provider_id.clone(), s.model.clone())
            } else {
                (default_provider_id.clone(), default_model.clone())
            };
            bot.send_message(
                chat_id,
                format!("*Provider:* {}\n*Model:* {}", provider, model),
            )
            .parse_mode(ParseMode::Markdown)
            .await?;
        }

        Command::Switch(provider_id) => {
            let provider_id = provider_id.trim().to_string();
            if provider_id.is_empty() {
                bot.send_message(chat_id, "Usage: /switch <provider>\nExample: /switch groq")
                    .await?;
                return Ok(());
            }

            match providers::get_provider(&provider_id, &cfg) {
                Ok(p) => {
                    let new_model = p.default_model().to_string();
                    let mut sessions = sessions.lock().await;
                    let session = sessions.entry(chat_id.0).or_insert_with(|| {
                        ChatSession::new(default_provider_id.clone(), default_model.clone())
                    });
                    session.provider_id = provider_id.clone();
                    session.model = new_model.clone();
                    bot.send_message(
                        chat_id,
                        format!("✅ Switched to *{}* ({})", provider_id, new_model),
                    )
                    .parse_mode(ParseMode::Markdown)
                    .await?;
                }
                Err(e) => {
                    bot.send_message(chat_id, format!("❌ {}", e)).await?;
                }
            }
        }

        Command::Model(model_name) => {
            let model_name = model_name.trim().to_string();
            if model_name.is_empty() {
                bot.send_message(chat_id, "Usage: /model <model-name>")
                    .await?;
                return Ok(());
            }
            let mut sessions = sessions.lock().await;
            let session = sessions.entry(chat_id.0).or_insert_with(|| {
                ChatSession::new(default_provider_id.clone(), default_model.clone())
            });
            session.model = model_name.clone();
            bot.send_message(chat_id, format!("✅ Model set to: {}", model_name))
                .await?;
        }

        Command::Ask(question) => {
            if question.trim().is_empty() {
                bot.send_message(chat_id, "Usage: /ask <your question>")
                    .await?;
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

            let thinking = bot
                .send_message(chat_id, "⏳ Thinking...")
                .await?;

            let messages = vec![Message::user(question.trim())];
            let response = match providers::get_provider(&provider_id, &cfg) {
                Ok(p) => match p.complete(&messages, &model).await {
                    Ok(r) => r,
                    Err(e) => format!("❌ Error: {}", e),
                },
                Err(e) => format!("❌ {}", e),
            };

            bot.delete_message(chat_id, thinking.id).await.ok();
            send_long_message(&bot, chat_id, &response).await?;
        }
    }

    Ok(())
}

// ── Message handler (agentic loop) ────────────────────────────────────────

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
        None => return Ok(()), // ignore non-text
    };

    let chat_id = msg.chat.id;
    let user_id = msg.from().map(|u| u.id.0 as i64).unwrap_or(0);

    if !is_allowed(&cfg, user_id) {
        bot.send_message(chat_id, "⛔ You are not authorized to use this bot.")
            .await?;
        return Ok(());
    }

    if text.trim().is_empty() {
        return Ok(());
    }

    // Get or create session
    let (provider_id, model) = {
        let sessions = sessions.lock().await;
        if let Some(s) = sessions.get(&chat_id.0) {
            (s.provider_id.clone(), s.model.clone())
        } else {
            (default_provider_id.clone(), default_model.clone())
        }
    };

    // Add user message to history
    {
        let mut sessions = sessions.lock().await;
        let session = sessions.entry(chat_id.0).or_insert_with(|| {
            ChatSession::new(default_provider_id.clone(), default_model.clone())
        });
        session.history.push(Message::user(&text));
    }

    let provider = match providers::get_provider(&provider_id, &cfg) {
        Ok(p) => p,
        Err(e) => {
            bot.send_message(chat_id, format!("❌ {}", e)).await?;
            return Ok(());
        }
    };

    let github_token = cfg.github_token.clone();

    // Send "thinking" indicator
    let thinking_msg = bot.send_message(chat_id, "⏳ Thinking...").await?;

    // Agentic loop
    let mut tool_count = 0;
    const MAX_TOOL_CALLS: u32 = 20; // safety limit

    loop {
        let history = {
            let sessions = sessions.lock().await;
            sessions
                .get(&chat_id.0)
                .map(|s| s.history.clone())
                .unwrap_or_default()
        };

        let result = provider
            .complete_with_system(&history, &model, tools::SYSTEM_PROMPT)
            .await;

        let response = match result {
            Ok(r) => r,
            Err(e) => {
                // Remove thinking message
                bot.delete_message(chat_id, thinking_msg.id).await.ok();
                // Remove last user message from history on error
                {
                    let mut sessions = sessions.lock().await;
                    if let Some(s) = sessions.get_mut(&chat_id.0) {
                        s.history.pop();
                    }
                }
                bot.send_message(chat_id, format!("❌ AI error: {}", e))
                    .await?;
                return Ok(());
            }
        };

        if let Some((tool_name, tool_input)) = tools::parse_tool_call(&response) {
            tool_count += 1;

            // Show any thinking text before the tool call
            let before = tools::text_before_tool(&response);
            if !before.is_empty() {
                bot.delete_message(chat_id, thinking_msg.id).await.ok();
                send_long_message(&bot, chat_id, before).await?;
            }

            // Show tool invocation
            let preview = tool_input.lines().next().unwrap_or("").trim();
            bot.delete_message(chat_id, thinking_msg.id).await.ok();
            let tool_msg = bot
                .send_message(
                    chat_id,
                    format!("⚙️ `[{}]` {}", tool_name, preview),
                )
                .parse_mode(ParseMode::Markdown)
                .await?;

            // Execute tool
            let tool_result = tools::execute_tool(
                &tool_name,
                &tool_input,
                github_token.as_deref(),
            );

            // Show tool result (trimmed for Telegram)
            let result_preview = truncate_for_telegram(&tool_result, 800);
            bot.send_message(chat_id, format!("```\n{}\n```", result_preview))
                .parse_mode(ParseMode::Markdown)
                .await
                .ok(); // ok() — don't fail if markdown parse error

            // Feed tool result into history
            {
                let mut sessions = sessions.lock().await;
                if let Some(s) = sessions.get_mut(&chat_id.0) {
                    s.history.push(Message::assistant(&response));
                    s.history.push(Message::user(&format!(
                        "[Tool: {}]\n[Result]\n{}",
                        tool_name, tool_result
                    )));
                }
            }

            // Safety limit
            if tool_count >= MAX_TOOL_CALLS {
                bot.delete_message(chat_id, tool_msg.id).await.ok();
                bot.send_message(chat_id, "⚠️ Reached tool call limit. Stopping agent loop.")
                    .await?;
                break;
            }

            // Continue agent loop — show new thinking indicator
            let _ = bot
                .send_message(chat_id, "⏳ Continuing...")
                .await;

            continue;
        }

        // No tool call — final answer
        bot.delete_message(chat_id, thinking_msg.id).await.ok();

        {
            let mut sessions = sessions.lock().await;
            if let Some(s) = sessions.get_mut(&chat_id.0) {
                s.history.push(Message::assistant(&response));
            }
        }

        send_long_message(&bot, chat_id, &response).await?;
        break;
    }

    Ok(())
}

// ── Helpers ───────────────────────────────────────────────────────────────

/// Telegram messages have a 4096 char limit. Split long messages.
async fn send_long_message(
    bot: &Bot,
    chat_id: ChatId,
    text: &str,
) -> Result<(), teloxide::RequestError> {
    const LIMIT: usize = 4000;

    if text.len() <= LIMIT {
        bot.send_message(chat_id, text).await?;
        return Ok(());
    }

    // Split at newlines
    let mut chunk = String::new();
    for line in text.lines() {
        if chunk.len() + line.len() + 1 > LIMIT {
            if !chunk.is_empty() {
                bot.send_message(chat_id, &chunk).await?;
                chunk.clear();
            }
        }
        if !chunk.is_empty() {
            chunk.push('\n');
        }
        chunk.push_str(line);
    }
    if !chunk.is_empty() {
        bot.send_message(chat_id, &chunk).await?;
    }

    Ok(())
}

/// Truncate text to max_chars, appending "... (truncated)" if needed.
fn truncate_for_telegram(text: &str, max_chars: usize) -> String {
    if text.len() <= max_chars {
        return text.to_string();
    }
    format!("{}...\n(truncated — {} total chars)", &text[..max_chars], text.len())
}
