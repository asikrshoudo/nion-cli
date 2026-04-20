use serde_json::Value;

pub const SYSTEM_PROMPT: &str = r#"You are Nion Agent — autonomous AI like OpenClaw/Claude Code. Read/write files, run commands, use GitHub.

Respond ONLY in this exact JSON format (no extra text outside JSON):

{
  "thinking": "step-by-step reasoning",
  "tool": "TOOL_NAME",
  "input": "INPUT_HERE"
}

Or when task is fully complete:
{
  "thinking": "final summary",
  "final_answer": "clear message to user"
}

## Tools
- read_file: file path
- write_file: path\n---\ncontent
- list_dir: dir path (use ".")
- run_command: shell command
- http_get: url
- github_clone: repo url
- github_push: commit message
- github_status: ""

## Rules
- Think step by step before acting
- Use tools one at a time and wait for the result
- After receiving a tool result, continue working until the task is fully complete
- When done, write a clear summary of what you did — with NO more tool tags
- Never run destructive commands like `rm -rf /` or `format`
- For dangerous commands (deleting files, formatting), always warn the user first"#;

pub fn parse_tool_call(text: &str) -> Option<(String, String)> {
    let cleaned = text.trim().replace("```json", "").replace("```", "").trim().to_string();
    let start = cleaned.find('{')?;
    let end = cleaned.rfind('}')? + 1;
    let json_str = &cleaned[start..end];

    let value: Value = serde_json::from_str(json_str).ok()?;
    let tool = value.get("tool")?.as_str()?.to_string();
    let input = value.get("input").and_then(|v| v.as_str()).unwrap_or("").to_string();
    Some((tool, input))
}

pub fn text_before_tool(text: &str) -> &str {
    match text.find('{') {
        Some(pos) => text[..pos].trim(),
        None => text.trim(),
    }
}

pub fn execute_tool(name: &str, input: &str, github_token: Option<&str>) -> String {
    match name {
        "read_file"     => tool_read_file(input.trim()),
        "write_file"    => tool_write_file(input),
        "list_dir"      => tool_list_dir(input.trim()),
        "run_command"   => tool_run_command(input.trim()),
        "http_get"      => tool_http_get(input.trim()),
        "github_clone"  => tool_github_clone(input.trim(), github_token),
        "github_push"   => tool_github_push(input.trim(), github_token),
        "github_status" => tool_run_command("git status"),
        other => format!("Unknown tool: '{}'", other),
    }
}

fn tool_read_file(path: &str) -> String {
    match std::fs::read_to_string(path) {
        Ok(c) if c.is_empty() => "(empty file)".to_string(),
        Ok(c) => c,
        Err(e) => format!("Error reading '{}': {}", path, e),
    }
}

fn tool_write_file(input: &str) -> String {
    let mut lines = input.lines();
    let path = match lines.next() {
        Some(p) => p.trim(),
        None => return "Error: missing file path".to_string(),
    };
    let _ = lines.next();
    let content: String = lines.collect::<Vec<_>>().join("\n");

    if let Some(parent) = std::path::Path::new(path).parent() {
        if !parent.as_os_str().is_empty() {
            let _ = std::fs::create_dir_all(parent);
        }
    }

    match std::fs::write(path, &content) {
        Ok(_) => {
            let success = format!("Written {} bytes → '{}'", content.len(), path);
            if path.ends_with(".rs") {
                let test = tool_run_command("cargo test --quiet");
                format!("{} \n\nAuto test:\n{}", success, test)
            } else if path.ends_with(".py") {
                let test = tool_run_command("pytest --tb=no -q");
                format!("{} \n\nAuto test:\n{}", success, test)
            } else {
                success
            }
        }
        Err(e) => format!("Error writing '{}': {}", path, e),
    }
}

fn tool_list_dir(path: &str) -> String {
    let dir = if path.is_empty() { "." } else { path };
    match std::fs::read_dir(dir) {
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
        Err(e) => format!("Error listing '{}': {}", dir, e),
    }
}

pub fn tool_run_command(cmd: &str) -> String {
    let blocked = [
    "rm -rf /", "rm -rf ", "mkfs", "dd if=",
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
            let exit = out.status.code().unwrap_or(-1);

            let mut result = String::new();
            if !stdout.is_empty() { result.push_str(stdout.trim_end()); }
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
        Err(e) => format!("Error running command: {}", e),
    }
}

fn tool_http_get(url: &str) -> String {
    tool_run_command(&format!("curl -sL --max-time 15 '{}'", url))
}

fn tool_github_clone(url: &str, github_token: Option<&str>) -> String {
    let clone_url = if let Some(token) = github_token {
        if url.starts_with("https://github.com/") {
            url.replacen("https://", &format!("https://{}@", token), 1)
        } else {
            url.to_string()
        }
    } else {
        url.to_string()
    };
    tool_run_command(&format!("git clone '{}'", clone_url))
}

fn tool_github_push(commit_message: &str, github_token: Option<&str>) -> String {
    if commit_message.is_empty() {
        return "Error: commit message is required".to_string();
    }

    let auth_setup = if let Some(token) = github_token {
        format!(
            "git config credential.helper '!f() {{ echo username=token; echo password={}; }}; f' && ",
            token
        )
    } else {
        String::new()
    };

    let cmd = format!(
        "{}git add -A && git commit -m '{}' && git push",
        auth_setup,
        commit_message.replace('\'', "\\'")
    );

    tool_run_command(&cmd)
}