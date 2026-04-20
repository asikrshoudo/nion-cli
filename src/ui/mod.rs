use anyhow::Result;
use colored::*;
use indicatif::{ProgressBar, ProgressStyle};
use std::io::{self, stdout, Write};
use std::time::Duration;

use crate::config::Config;

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
        println!("{}", line.cyan());
        std::thread::sleep(Duration::from_millis(55));
    }

    println!("  The Universal AI CLI  v{}", env!("CARGO_PKG_VERSION").bright_black());
    std::thread::sleep(Duration::from_millis(60));
    println!("  One tool. Every model. Every platform.\n");
}

pub fn print_response(text: &str) {
    println!("\n{}", "─".repeat(80).bright_black());

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
        } else {
            println!("{}", line.white());
        }
    }

    println!("{}\n", "─".repeat(80).bright_black());
}

pub fn print_chat_header(cfg: &Config, provider: &str, model: &str) {
    let name = cfg.user_name.as_deref().unwrap_or("User");
    println!("\n{}", "─".repeat(80).bright_black());
    println!(
        "  {}    {}  {}   {}",
        format!("Hello, {}", name).bright_white().bold(),
        "Provider:".bright_black(),
        provider.bright_cyan().bold(),
        model.bright_black()
    );
    println!(
        "  {}",
        "Commands: /exit /clear /help /model <n> /switch <p> /name <new>".bright_black()
    );
    println!("{}\n", "─".repeat(80).bright_black());
}

pub fn print_chat_help() {
    println!("\n  {}", "Available commands:".bright_yellow().bold());
    println!("  {}        Exit session", "/exit".cyan());
    println!("  {}       Clear history", "/clear".cyan());
    println!("  {}        Show help", "/help".cyan());
    println!("  {}  Switch model", "/model <n>".cyan());
    println!("  {}  Switch provider", "/switch <p>".cyan());
    println!("  {}  Change name", "/name <n>".cyan());
    println!();
}

pub fn read_user_input(name: &str) -> Result<String> {
    print!("\n  {} ", format!("{} >", name).bright_green().bold());
    stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    Ok(input.trim().to_string())
}

pub fn start_spinner(msg: &str) -> ProgressBar {
    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::default_spinner()
            .tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏")
            .template("  {spinner:.cyan} {msg:.white}")
            .unwrap(),
    );
    pb.set_message(msg.to_string());
    pb.enable_steady_tick(Duration::from_millis(80));
    pb
}

pub fn print_success(msg: &str) { println!("  {} {}", "[OK]".bright_green().bold(), msg.white()); }
pub fn print_error(msg: &str) { eprintln!("  {} {}", "[ERR]".bright_red().bold(), msg.red()); }
pub fn print_info(msg: &str) { println!("  {} {}", "[--]".bright_black(), msg.bright_black()); }
pub fn print_goodbye(name: &str) { println!("\n  Goodbye, {}.\n", name.bright_white().bold()); }

pub fn print_config(cfg: &Config) {
    println!();
    box_line_top(50);
    box_title("  Nion Configuration", 50);
    box_line_mid(50);
    box_row(&format!("  Name        {}", cfg.user_name.as_deref().unwrap_or("not set").bright_white()), 50);
    box_row(&format!("  Provider    {}", cfg.default_provider.as_deref().unwrap_or("not set").bright_cyan()), 50);
    box_row(&format!("  Model       {}", cfg.default_model.as_deref().unwrap_or("not set").bright_cyan()), 50);
    box_line_mid(50);

    if cfg.api_keys.is_empty() {
        box_row("  No API keys configured.", 50);
    } else {
        box_row("  Configured providers:", 50);
        for (provider, _) in &cfg.api_keys {
            box_row(&format!("    ✓  {}", provider.bright_cyan()), 50);
        }
    }

    box_row(&format!("  Config: {}", crate::config::Config::config_path().display()), 50);
    box_line_bot(50);
    println!();
}

pub fn print_models_list() {
    println!("\n  All Available Models (keep your original long list here)");
}

pub async fn show_update_prompt(new_version: &str) -> Result<()> {
    println!();
    box_line_top(60);
    box_title(&format!("  Nion v{} is available!", new_version), 60);
    box_line_bot(60);
    print!("\n  Would you like to update? [Y/n] ");
    stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    let response = input.trim().to_lowercase();

    if response.is_empty() || response == "y" || response == "yes" {
        crate::updater::download_and_replace(new_version).await?;
    } else {
        print_info("Keeping current version. Run 'nion update' anytime.");
    }
    Ok(())
}

pub fn select_menu(_options: &[String], default: usize) -> Result<usize> {
    Ok(default)
}

fn box_line_top(width: usize) {
    println!("{}{}{}", "┌".bright_black(), "─".repeat(width - 2).bright_black(), "┐".bright_black());
}

fn box_line_mid(width: usize) {
    println!("{}{}{}", "├".bright_black(), "─".repeat(width - 2).bright_black(), "┤".bright_black());
}

fn box_line_bot(width: usize) {
    println!("{}{}{}", "└".bright_black(), "─".repeat(width - 2).bright_black(), "┘".bright_black());
}

fn box_title(text: &str, width: usize) {
    println!("{}{}{}", "│".bright_black(), format!("{:<width$}", text, width = width - 2).bright_yellow().bold(), "│".bright_black());
}

fn box_row(text: &str, width: usize) {
    println!("{}{}{}", "│".bright_black(), format!("{:<width$}", text, width = width - 2).white(), "│".bright_black());
}

pub fn print_agent_header(cfg: &Config, provider: &str, model: &str) {
    let name = cfg.user_name.as_deref().unwrap_or("User");
    println!();
    box_line_top(70);
    println!("{}{}{}", "│".bright_black(), format!("  Agent   Provider: {}   {}", provider.bright_cyan().bold(), model.bright_black()), "│".bright_black());
    println!("{}{}{}", "│".bright_black(), format!("  Hello, {}  —  I can read/write files & run commands.", name).bright_black(), "│".bright_black());
    box_line_bot(70);
    println!("{}", "  Commands: /exit /clear /help".bright_black());
    println!();
}

pub fn print_agent_help() {
    println!("\n  {}", "Nion Agent".bright_yellow().bold());
    println!("{}", "─".repeat(60).bright_black());
    println!("  I can read files, write files, list directories, and run commands.");
    println!();
    println!("  {}", "Examples:".bright_black());
    println!("    create a snake game in snake.py");
    println!("    read main.rs and add error handling");
    println!("    list all files and summarize the project");
    println!("    run pytest and fix any failing tests");
    println!();
    println!("  {}", "Commands:".bright_black());
    println!("    {}   exit the session", "/exit".cyan());
    println!("    {}  clear history", "/clear".cyan());
    println!("    {}   show this help", "/help".cyan());
    println!();
}