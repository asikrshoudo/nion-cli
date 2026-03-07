use anyhow::Result;
use colored::*;
use crossterm::{
    cursor,
    execute,
    style::{Color, Print, SetForegroundColor, ResetColor},
    terminal,
};
use indicatif::{ProgressBar, ProgressStyle};
use std::io::{stdout, Write};
use std::time::Duration;

use crate::config::Config;

// Startup animation - renders the banner line by line with a delay
pub async fn startup_animation() {
    let lines = vec![
        "  ███╗   ██╗██╗ ██████╗ ███╗   ██╗",
        "  ████╗  ██║██║██╔═══██╗████╗  ██║",
        "  ██╔██╗ ██║██║██║   ██║██╔██╗ ██║",
        "  ██║╚██╗██║██║██║   ██║██║╚██╗██║",
        "  ██║ ╚████║██║╚██████╔╝██║ ╚████║",
        "  ╚═╝  ╚═══╝╚═╝ ╚═════╝ ╚═╝  ╚═══╝",
    ];

    println!();
    for line in &lines {
        execute!(
            stdout(),
            SetForegroundColor(Color::Cyan),
            Print(format!("{}\n", line)),
            ResetColor
        )
        .unwrap_or_else(|_| println!("{}", line));
        std::thread::sleep(Duration::from_millis(55));
    }

    // Tagline fade-in
    std::thread::sleep(Duration::from_millis(80));
    execute!(
        stdout(),
        SetForegroundColor(Color::DarkGrey),
        Print(format!("  The Universal AI CLI  v{}\n", env!("CARGO_PKG_VERSION"))),
        ResetColor
    )
    .unwrap_or_else(|_| {
        println!(
            "  The Universal AI CLI  v{}",
            env!("CARGO_PKG_VERSION")
        )
    });

    std::thread::sleep(Duration::from_millis(60));
    execute!(
        stdout(),
        SetForegroundColor(Color::DarkGrey),
        Print("  One tool. Every model. Every platform.\n\n"),
        ResetColor
    )
    .unwrap_or_else(|_| println!("  One tool. Every model. Every platform.\n"));
}

pub fn print_banner() {
    let lines = vec![
        "  ███╗   ██╗██╗ ██████╗ ███╗   ██╗",
        "  ████╗  ██║██║██╔═══██╗████╗  ██║",
        "  ██╔██╗ ██║██║██║   ██║██╔██╗ ██║",
        "  ██║╚██╗██║██║██║   ██║██║╚██╗██║",
        "  ██║ ╚████║██║╚██████╔╝██║ ╚████║",
        "  ╚═╝  ╚═══╝╚═╝ ╚═════╝ ╚═╝  ╚═══╝",
    ];
    println!();
    for line in &lines {
        println!("{}", line.bright_cyan().bold());
    }
    println!(
        "  {}",
        format!("The Universal AI CLI  v{}", env!("CARGO_PKG_VERSION")).bright_black()
    );
    println!("  {}\n", "One tool. Every model. Every platform.".bright_black());
}

pub fn print_response(text: &str) {
    println!("\n{}", "-".repeat(60).bright_black());

    let mut in_code_block = false;
    let mut lang = String::new();

    for line in text.lines() {
        if line.starts_with("```") {
            in_code_block = !in_code_block;
            if in_code_block {
                lang = line.trim_start_matches('`').to_string();
                if !lang.is_empty() {
                    println!("{}", format!("  [{}]", lang).bright_black());
                }
            } else {
                lang.clear();
            }
        } else if in_code_block {
            println!("{}", line.bright_green());
        } else if line.starts_with("# ") {
            println!("{}", line.bright_yellow().bold());
        } else if line.starts_with("## ") || line.starts_with("### ") {
            println!("{}", line.bright_cyan().bold());
        } else if line.starts_with("**") && line.ends_with("**") {
            println!("{}", line.replace("**", "").bold());
        } else if line.starts_with("- ") || line.starts_with("* ") {
            println!("{}", line.white());
        } else {
            println!("{}", line.white());
        }
    }

    println!("{}\n", "-".repeat(60).bright_black());
}

pub fn print_chat_header(cfg: &Config, provider: &str, model: &str) {
    let name = cfg.user_name.as_deref().unwrap_or("User");
    println!("\n{}", "-".repeat(60).bright_black());
    println!(
        "  {}    {}  {}   {}",
        format!("Hello, {}", name).bright_white().bold(),
        "Provider:".bright_black(),
        provider.bright_cyan().bold(),
        model.bright_black()
    );
    println!(
        "  {}",
        "Commands:  /exit  /clear  /help  /model <n>  /switch <provider>  /name <new_name>"
            .bright_black()
    );
    println!("{}\n", "-".repeat(60).bright_black());
}

pub fn print_chat_help() {
    println!("\n  {}", "Available commands:".bright_yellow().bold());
    println!("  {}        Exit the session", "/exit".cyan());
    println!("  {}       Clear chat history", "/clear".cyan());
    println!("  {}        Show this help", "/help".cyan());
    println!(
        "  {}  Switch model  e.g. /model gpt-4o",
        "/model <n>".cyan()
    );
    println!(
        "  {}  Switch provider  e.g. /switch groq",
        "/switch <p>".cyan()
    );
    println!(
        "  {}  Change your name  e.g. /name Alex",
        "/name <n>".cyan()
    );
    println!();
}

pub fn read_user_input(name: &str) -> Result<String> {
    use std::io::{self, Write};
    print!("\n  {} ", format!("{}  >", name).bright_green().bold());
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    Ok(input.trim().to_string())
}

pub fn start_spinner(msg: &str) -> ProgressBar {
    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::default_spinner()
            .tick_chars("--\\|/")
            .template("  {spinner:.cyan} {msg:.white}")
            .unwrap(),
    );
    pb.set_message(msg.to_string());
    pb.enable_steady_tick(Duration::from_millis(100));
    pb
}

pub fn print_success(msg: &str) {
    println!("  [OK]  {}", msg.white());
}

pub fn print_error(msg: &str) {
    eprintln!("  [ERR] {}", msg.red());
}

pub fn print_info(msg: &str) {
    println!("  [--]  {}", msg.bright_black());
}

pub fn print_goodbye(name: &str) {
    println!("\n  Goodbye, {}.\n", name.bright_white().bold());
}

pub fn print_config(cfg: &Config) {
    println!("\n  {}", "Nion Configuration".bright_yellow().bold());
    println!("{}", "-".repeat(40).bright_black());
    println!(
        "  {}: {}",
        "Name".bright_black(),
        cfg.user_name.as_deref().unwrap_or("not set").bright_white()
    );
    println!(
        "  {}: {}",
        "Default Provider".bright_black(),
        cfg.default_provider
            .as_deref()
            .unwrap_or("not set")
            .bright_cyan()
    );
    println!(
        "  {}: {}",
        "Default Model".bright_black(),
        cfg.default_model
            .as_deref()
            .unwrap_or("not set")
            .bright_cyan()
    );

    if cfg.api_keys.is_empty() {
        println!("\n  No API keys configured.");
        println!("  Run 'nion config setup' to add keys.");
    } else {
        println!("\n  {}", "Configured Providers:".bright_black());
        for (provider, _) in &cfg.api_keys {
            println!("  [+]  {}", provider.bright_cyan());
        }
    }

    println!(
        "\n  Config: {}",
        crate::config::Config::config_path()
            .display()
            .to_string()
            .bright_black()
    );
    println!();
}

pub fn print_models_list() {
    println!("\n  {}", "All Available Models".bright_yellow().bold());
    println!("{}", "-".repeat(60).bright_black());

    let providers: Vec<(&str, Vec<(&str, &str)>)> = vec![
        (
            "OpenAI",
            vec![
                ("gpt-4o",                      "Latest flagship, vision support"),
                ("gpt-4o-mini",                 "Fast and affordable"),
                ("gpt-4-turbo",                 "128k context"),
                ("gpt-3.5-turbo",               "Budget option"),
                ("o1",                          "Advanced reasoning"),
                ("o1-mini",                     "Fast reasoning"),
                ("o3-mini",                     "Latest reasoning model"),
            ],
        ),
        (
            "OpenAI Codex / Instruct",
            vec![
                ("gpt-4o",                      "Best for coding tasks"),
                ("o3-mini",                     "Reasoning + code"),
            ],
        ),
        (
            "Anthropic",
            vec![
                ("claude-3-5-sonnet-20241022",  "Best overall"),
                ("claude-3-5-haiku-20241022",   "Fast and affordable"),
                ("claude-3-opus-20240229",       "Most powerful Claude"),
                ("claude-3-haiku-20240307",      "Budget option"),
            ],
        ),
        (
            "Google",
            vec![
                ("gemini-1.5-pro",              "1M context, multimodal"),
                ("gemini-1.5-flash",            "Fast and efficient"),
                ("gemini-2.0-flash",            "Latest Gemini"),
                ("gemini-2.0-flash-thinking-exp", "Thinking / reasoning"),
            ],
        ),
        (
            "Groq  [free tier available]",
            vec![
                ("llama-3.3-70b-versatile",     "Best Llama, very fast"),
                ("llama-3.1-8b-instant",        "Ultra fast"),
                ("llama3-70b-8192",             "Stable Llama 3"),
                ("mixtral-8x7b-32768",          "Strong reasoning"),
                ("gemma2-9b-it",                "Google Gemma via Groq"),
                ("qwen-2.5-72b",                "Alibaba Qwen 72B"),
            ],
        ),
        (
            "xAI",
            vec![
                ("grok-2-latest",               "Latest Grok"),
                ("grok-2-vision-latest",        "Vision support"),
                ("grok-beta",                   "Stable Grok"),
            ],
        ),
        (
            "DeepSeek",
            vec![
                ("deepseek-chat",               "DeepSeek V3"),
                ("deepseek-reasoner",           "DeepSeek R1 reasoning"),
            ],
        ),
        (
            "Mistral",
            vec![
                ("mistral-large-latest",        "Best Mistral"),
                ("mistral-small-latest",        "Fast and cheap"),
                ("codestral-latest",            "Best for code"),
                ("open-mistral-nemo",           "Open source"),
            ],
        ),
        (
            "Perplexity",
            vec![
                ("sonar-pro",                   "Web search built-in"),
                ("sonar",                       "Fast web search"),
                ("sonar-reasoning-pro",         "Reasoning + web"),
            ],
        ),
        (
            "Together AI",
            vec![
                ("meta-llama/Llama-3.3-70B-Instruct-Turbo", "Llama 3.3 70B"),
                ("deepseek-ai/DeepSeek-V3",     "DeepSeek V3"),
                ("Qwen/Qwen2.5-72B-Instruct-Turbo", "Qwen 72B"),
                ("mistralai/Mixtral-8x22B-Instruct-v0.1", "Mixtral 8x22B"),
            ],
        ),
        (
            "Cohere",
            vec![
                ("command-r-plus-08-2024",      "Best Cohere model"),
                ("command-r-08-2024",           "Fast Cohere"),
                ("command-light",               "Lightweight"),
            ],
        ),
    ];

    for (name, models) in providers {
        println!("\n  {}", name.bright_cyan().bold());
        for (model, desc) in models {
            println!(
                "    {:45} {}",
                model.white().bold(),
                desc.bright_black()
            );
        }
    }
    println!();
}

pub async fn show_update_prompt(new_version: &str) -> Result<()> {
    use std::io::{self, Write};

    println!();
    println!("{}", "-".repeat(60).bright_yellow());
    println!(
        "  Nion v{} is available.",
        new_version.bright_yellow().bold()
    );
    print!("  Would you like to update? [Y/n] ");
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    let response = input.trim().to_lowercase();

    if response.is_empty() || response == "y" || response == "yes" {
        crate::updater::download_and_replace(new_version).await?;
    } else {
        print_info("Keeping current version. Run 'nion update' anytime.");
    }
    println!("{}", "-".repeat(60).bright_yellow());

    Ok(())
}
