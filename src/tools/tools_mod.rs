/// Shared tool execution module.
/// Used by both the CLI agent (agent/mod.rs) and Telegram bot (telegram/mod.rs).

pub const SYSTEM_PROMPT: &str = r#"You are Nion Agent — an agentic AI assistant that can read and write files, list directories, run shell commands, and interact with GitHub to complete tasks autonomously.

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

### http_get
Fetch a URL and return the response body (text only).
Input: the URL to fetch.
Example:
<tool>http_get</tool>
<input>https://api.github.com/repos/asikrshoudo/nion-cli</input>

### github_clone
Clone a GitHub repository into the current directory.
Input: the repository URL.
Example:
<tool>github_clone</tool>
<input>https://github.com/user/repo</input>

### github_push
Stage all changes, commit with a message, and push to the current branch.
Input: the commit message.
Example:
<tool>github_push</tool>
<input>fix: update config handling</input>

### github_status
Show git status of the current directory.
Input: (none, leave blank)
Example:
<tool>github_status</tool>
<input></input>

## Rules
- Think step by step before acting
- Use tools one at a time and wait for the result
- After receiving a tool result, continue working until the task is fully complete
- When done, write a clear summary of what you did — with NO more tool tags
- Never run destructive commands like `rm -rf /` or `format`
- For dangerous commands (deleting files, formatting), always warn the user first
"#;

/// Parse a tool call from the AI's response text.
/// Returns (tool_name, tool_input) if found.
pub fn parse_tool_call(text: &str) -> Option<(String, String)> {
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

/// Get the text before the first <tool> tag.
pub fn text_before_tool(text: &str) -> &str {
    match text.find("<tool>") {
        Some(pos) => text[..pos].trim(),
        None => text.trim(),
    }
}

/// Execute a tool by name with the given input.
/// github_token is used for authenticated GitHub operations.
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
        other => format!(
            "Unknown tool: '{}'. Available: read_file, write_file, list_dir, run_command, http_get, github_clone, github_push, github_status",
            other
        ),
    }
}

// ── Individual tools ──────────────────────────────────────────────────────

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
    let _ = lines.next(); // skip "---"
    let content: String = lines.collect::<Vec<_>>().join("\n");

    if let Some(parent) = std::path::Path::new(path).parent() {
        if !parent.as_os_str().is_empty() {
            if let Err(e) = std::fs::create_dir_all(parent) {
                return format!("Error creating dirs: {}", e);
            }
        }
    }

    match std::fs::write(path, &content) {
        Ok(_) => format!("✓ Written {} bytes → '{}'", content.len(), path),
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
        "rm -rf /", "rm -rf ~", "mkfs", "dd if=",
        ":(){ :|:& };:", "> /dev/sda", "format c:",
    ];
    for b in &blocked {
        if cmd.contains(b) {
            return format!("⛔ Blocked: '{}' is a dangerous command.", cmd);
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
        Err(e) => format!("Error running command: {}", e),
    }
}

fn tool_http_get(url: &str) -> String {
    // Synchronous HTTP fetch using ureq-style blocking or just curl
    // We use curl since it's available everywhere including Termux
    tool_run_command(&format!("curl -sL --max-time 15 '{}'", url))
}

fn tool_github_clone(url: &str, github_token: Option<&str>) -> String {
    let clone_url = if let Some(token) = github_token {
        // Inject token into HTTPS URL: https://TOKEN@github.com/...
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

    // Configure token-based auth if available
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
