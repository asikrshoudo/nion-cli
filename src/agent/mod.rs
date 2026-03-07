use anyhow::Result;
use colored::*;

use crate::config::Config;
use crate::providers;
use crate::session::Message;
use crate::ui;

const SYSTEM_PROMPT: &str = r#"You are Nion Agent — an agentic AI coding assistant that can read and write files, list directories, and run shell commands to complete tasks autonomously.

## How to use tools

When you need to use a tool, output EXACTLY this format — nothing extra on those lines:

<tool>TOOL_NAME</tool>
<input>INPUT_HERE</input>

## Available tools

### read_file
Read a file's contents.
Input: the file path
Example:
<tool>read_file</tool>
<input>src/main.rs</input>

### write_file
Write content to a file. Creates the file and any parent directories if needed.
Input: First line is the path. Second line is three dashes (---). Remaining lines are the file content.
Example:
<tool>write_file</tool>
<input>hello.py
---
print("Hello, world!")
</input>

### list_dir
List all files and folders in a directory.
Input: the directory path. Use "." for current directory.
Example:
<tool>list_dir</tool>
<input>.</input>

### run_command
Run a shell command and get the output.
Input: the shell command to run.
Example:
<tool>run_command</tool>
<input>python hello.py</input>

## Rules
- Think step by step before acting
- Use tools one at a time and wait for the result
- After receiving a tool result, continue working until the task is fully complete
- When done, write a short summary of what you did — with NO more tool tags
- Never run destructive commands like `rm -rf /` or `format`
"#;

pub async fn run(provider_name: Option<&str>, model_override: Option<&str>) -> Result<()> {
    let cfg = Config::load()?;

    let provider_id = provider_name
        .map(String::from)
        .or_else(|| cfg.default_provider.clone())
        .unwrap_or_else(|| "groq".to_string());

    let provider = providers::get_provider(&provider_id, &cfg)?;

    let model = model_override
        .map(String::from)
        .or_else(|| cfg.default_model.clone())
        .unwrap_or_else(|| provider.default_model().to_string());

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

        // Agentic loop: keep going until AI gives no more tool calls
        'agent: loop {
            let p = providers::get_provider(&provider_id, &cfg)?;
            let spinner = ui::start_spinner("Thinking...");
            let result = p.complete_with_system(&history, &model, SYSTEM_PROMPT).await;
            spinner.finish_and_clear();

            match result {
                Err(e) => {
                    ui::print_error(&format!("{}", e));
                    history.pop(); // remove user message on error
                    break 'agent;
                }
                Ok(response) => {
                    if let Some((tool_name, tool_input)) = parse_tool_call(&response) {
                        // Print any thinking text before the tool tag
                        let before_tool = text_before_tool(&response);
                        if !before_tool.is_empty() {
                            ui::print_response(before_tool);
                        }

                        // Show tool invocation
                        let preview = tool_input.lines().next().unwrap_or("").trim();
                        println!(
                            "\n  {}  {}  {}",
                            "⚙".bright_yellow(),
                            format!("[{}]", tool_name).bright_cyan().bold(),
                            preview.bright_black()
                        );

                        // Execute tool
                        let tool_result = execute_tool(&tool_name, &tool_input);

                        // Show short result preview
                        let preview_lines: Vec<&str> = tool_result.lines().take(6).collect();
                        for line in &preview_lines {
                            println!("  {}", line.bright_black());
                        }
                        if tool_result.lines().count() > 6 {
                            println!("  {}", format!("... ({} more lines)", tool_result.lines().count() - 6).bright_black());
                        }

                        // Feed result back into history
                        history.push(Message::assistant(&response));
                        history.push(Message::user(&format!(
                            "[Tool: {}]\n[Result]\n{}",
                            tool_name, tool_result
                        )));

                        // Continue agent loop
                        continue 'agent;
                    } else {
                        // No tool call — final answer
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

// ── Tool parsing ──────────────────────────────────────────────────────────

fn parse_tool_call(text: &str) -> Option<(String, String)> {
    let t_start = text.find("<tool>")?;
    let t_end   = text.find("</tool>")?;
    let i_start = text.find("<input>")?;
    let i_end   = text.find("</input>")?;

    if t_end <= t_start || i_end <= i_start || i_start < t_end {
        return None;
    }

    let name  = text[t_start + 6..t_end].trim().to_string();
    let input = text[i_start + 7..i_end].trim_start_matches('\n').trim_end().to_string();

    if name.is_empty() { return None; }
    Some((name, input))
}

fn text_before_tool(text: &str) -> &str {
    match text.find("<tool>") {
        Some(pos) => text[..pos].trim(),
        None => text.trim(),
    }
}

// ── Tool execution ────────────────────────────────────────────────────────

fn execute_tool(name: &str, input: &str) -> String {
    match name {
        "read_file"   => tool_read_file(input.trim()),
        "write_file"  => tool_write_file(input),
        "list_dir"    => tool_list_dir(input.trim()),
        "run_command" => tool_run_command(input.trim()),
        other         => format!("Unknown tool: '{}'. Available: read_file, write_file, list_dir, run_command", other),
    }
}

fn tool_read_file(path: &str) -> String {
    match std::fs::read_to_string(path) {
        Ok(c) if c.is_empty() => "(empty file)".to_string(),
        Ok(c) => c,
        Err(e) => format!("Error: {}", e),
    }
}

fn tool_write_file(input: &str) -> String {
    // First line = path, then "---", then content
    let mut lines = input.lines();
    let path = match lines.next() {
        Some(p) => p.trim(),
        None => return "Error: missing file path".to_string(),
    };

    // skip the separator line ("---")
    let _ = lines.next();

    let content: String = lines.collect::<Vec<_>>().join("\n");

    if let Some(parent) = std::path::Path::new(path).parent() {
        if !parent.as_os_str().is_empty() {
            if let Err(e) = std::fs::create_dir_all(parent) {
                return format!("Error creating dirs: {}", e);
            }
        }
    }

    match std::fs::write(path, &content) {
        Ok(_) => format!("Written {} bytes → '{}'", content.len(), path),
        Err(e) => format!("Error: {}", e),
    }
}

fn tool_list_dir(path: &str) -> String {
    match std::fs::read_dir(path) {
        Ok(entries) => {
            let mut items: Vec<String> = entries
                .filter_map(|e| e.ok())
                .map(|e| {
                    let name = e.file_name().to_string_lossy().to_string();
                    if e.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                        format!("{}/", name)
                    } else {
                        name
                    }
                })
                .collect();
            items.sort();
            if items.is_empty() { "(empty)".to_string() } else { items.join("\n") }
        }
        Err(e) => format!("Error: {}", e),
    }
}

fn tool_run_command(cmd: &str) -> String {
    // Block dangerous patterns
    let blocked = [
        "rm -rf /", "rm -rf ~", "mkfs", "dd if=",
        ":(){ :|:& };:", "> /dev/sda", "format c:",
    ];
    for b in &blocked {
        if cmd.contains(b) {
            return format!("Blocked: '{}' is a dangerous command.", cmd);
        }
    }

    let output = std::process::Command::new("sh")
        .args(["-c", cmd])
        .output();

    match output {
        Ok(out) => {
            let stdout = String::from_utf8_lossy(&out.stdout);
            let stderr = String::from_utf8_lossy(&out.stderr);
            let exit   = out.status.code().unwrap_or(-1);

            let mut result = String::new();
            if !stdout.is_empty() {
                result.push_str(stdout.trim_end());
            }
            if !stderr.is_empty() {
                if !result.is_empty() { result.push('\n'); }
                result.push_str(&format!("[stderr] {}", stderr.trim_end()));
            }
            if result.is_empty() {
                format!("(exit {})", exit)
            } else if exit != 0 {
                format!("{}\n[exit {}]", result, exit)
            } else {
                result
            }
        }
        Err(e) => format!("Error: {}", e),
    }
}
