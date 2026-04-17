use anyhow::Result;
use colored::*;

use crate::config::Config;
use crate::providers;
use crate::session::Message;
use crate::tools;
use crate::ui;

pub async fn run(provider_name: Option<&str>, model_override: Option<&str>) -> Result<()> {
    let cfg = Config::load()?;

    let provider_id = provider_name
        .map(String::from)
        .or_else(|| cfg.default_provider.clone())
        .unwrap_or_else(|| "groq".to_string());

    // Build provider once — not on every agentic iteration
    let provider = providers::get_provider(&provider_id, &cfg)?;

    let model = model_override
        .map(String::from)
        .or_else(|| cfg.default_model.clone())
        .unwrap_or_else(|| provider.default_model().to_string());

    let github_token = cfg.github_token.clone();
    let mut history: Vec<Message> = Vec::new();

    ui::print_agent_header(&cfg, &provider_id, &model);

    loop {
        let name = cfg.user_name.as_deref().unwrap_or("You");
        let input = match ui::read_user_input(name) {
            Ok(s) => s,
            Err(_) => break,
        };

        if input.trim().is_empty() {
            continue;
        }

        match input.trim().to_lowercase().as_str() {
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
                ui::print_agent_help();
                continue;
            }
            _ => {}
        }

        history.push(Message::user(&input));

        // Agentic loop
        'agent: loop {
            let spinner = ui::start_spinner("Thinking...");
            let result = provider.complete_with_system(&history, &model, tools::SYSTEM_PROMPT).await;
            spinner.finish_and_clear();

            match result {
                Err(e) => {
                    ui::print_error(&format!("{}", e));
                    history.pop();
                    break 'agent;
                }
                Ok(response) => {
                    if let Some((tool_name, tool_input)) = tools::parse_tool_call(&response) {
                        let before_tool = tools::text_before_tool(&response);
                        if !before_tool.is_empty() {
                            ui::print_response(before_tool);
                        }

                        let preview = tool_input.lines().next().unwrap_or("").trim();
                        println!(
                            "\n  {}  {}  {}",
                            "⚙".bright_yellow(),
                            format!("[{}]", tool_name).bright_cyan().bold(),
                            preview.bright_black()
                        );

                        let tool_result = tools::execute_tool(
                            &tool_name,
                            &tool_input,
                            github_token.as_deref(),
                        );

                        // Show preview
                        let preview_lines: Vec<&str> = tool_result.lines().take(6).collect();
                        for line in &preview_lines {
                            println!("  {}", line.bright_black());
                        }
                        if tool_result.lines().count() > 6 {
                            println!(
                                "  {}",
                                format!("... ({} more lines)", tool_result.lines().count() - 6)
                                    .bright_black()
                            );
                        }

                        history.push(Message::assistant(&response));
                        history.push(Message::user(&format!(
                            "[Tool: {}]\n[Result]\n{}",
                            tool_name, tool_result
                        )));

                        continue 'agent;
                    } else {
                        history.push(Message::assistant(&response));
                        ui::print_response(&response);
                        break 'agent;
                    }
                }
            }
        }
    }

    Ok(())
}
