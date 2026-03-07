# nion-cli

**The Universal AI CLI — One tool. Every model. Every platform.**

[![License: AGPL-3.0](https://img.shields.io/badge/License-AGPL%203.0-blue.svg)](LICENSE)
[![GitHub release](https://img.shields.io/github/v/release/asikrshoudo/nion-cli)](https://github.com/asikrshoudo/nion-cli/releases)
[![npm](https://img.shields.io/npm/v/nion-cli)](https://www.npmjs.com/package/nion-cli)

---

Most people who use AI from the terminal end up juggling multiple tools — one for OpenAI, another for Claude, a script for Groq. Every new provider means a new setup. Nion solves that by being a single binary that connects to all of them through one consistent interface.

You configure your keys once, pick a default provider, and from that point on `nion chat` just works — whether you are on your laptop, a remote Linux server, or an Android phone running Termux. There is also an agent mode where the AI can autonomously read and write files and run shell commands, similar to Claude Code or Gemini CLI.

It is written in Rust, so it is fast, produces small self-contained binaries, and has no runtime dependencies. The Android build uses the NDK directly, so it works natively in Termux.

---

## Installation

### npm (recommended — works on all platforms)

```bash
npm install -g nion-cli
```

This downloads the correct binary for your platform automatically.

### curl (Linux, macOS, Termux)

```bash
curl -sSL https://raw.githubusercontent.com/asikrshoudo/nion-cli/main/install.sh | bash
```

Then add to PATH if needed:

```bash
export PATH="$HOME/.local/bin:$PATH"
# Add to ~/.bashrc or ~/.zshrc to make it permanent
```

### Windows

Download the `.exe` from the [Releases page](https://github.com/asikrshoudo/nion-cli/releases/latest) and place it in a folder that is in your PATH.

### Build from source

```bash
git clone https://github.com/asikrshoudo/nion-cli
cd nion-cli
cargo build --release
# binary: ./target/release/nion
```

---

## First run

```bash
nion config setup
```

This opens an interactive numbered menu. Select a provider, paste your API key, and repeat for any others you want. The last provider you configure becomes your default. If you just want to try things out for free, Groq has a free tier and is very fast.

---

## Supported providers

| Provider | ID | Free tier | Get a key |
|---|---|---|---|
| OpenAI | `openai` | No | [platform.openai.com](https://platform.openai.com/api-keys) |
| Anthropic | `anthropic` | No | [console.anthropic.com](https://console.anthropic.com) |
| Google Gemini | `google` | No | [aistudio.google.com](https://aistudio.google.com/app/apikey) |
| Groq | `groq` | Yes | [console.groq.com](https://console.groq.com) |
| xAI Grok | `grok` | No | [console.x.ai](https://console.x.ai) |
| DeepSeek | `deepseek` | No | [platform.deepseek.com](https://platform.deepseek.com) |
| Mistral | `mistral` | No | [console.mistral.ai](https://console.mistral.ai) |
| Perplexity | `perplexity` | No | [perplexity.ai/settings/api](https://www.perplexity.ai/settings/api) |
| Together AI | `together` | No | [api.together.ai](https://api.together.ai) |
| Cohere | `cohere` | No | [dashboard.cohere.com](https://dashboard.cohere.com/api-keys) |

---

## Commands

### `nion chat`

An interactive conversation session with full history. You can switch providers or models mid-session without losing context.

```bash
nion chat
nion chat -p anthropic
nion chat -p groq -m llama-3.3-70b-versatile
nion chat -p openai -m gpt-4o
```

In-session commands:

```
/help                   show available commands
/exit                   end the session
/clear                  wipe conversation history
/model <name>           switch to a different model
/switch <provider>      switch to a different provider
/name <name>            change your display name
```

---

### `nion agent`

An agentic session where the AI can actually do things on your machine — read files, write files, list directories, and run shell commands. It works in a loop, using tools as needed until the task is complete. Every tool call is shown in real time so you always know what is happening.

```bash
nion agent
nion agent -p anthropic
nion agent -p openai -m gpt-4o
```

Available tools:

| Tool | What it does |
|---|---|
| `read_file` | Reads any file and shows its content to the AI |
| `write_file` | Creates or overwrites a file |
| `list_dir` | Lists files and folders in a directory |
| `run_command` | Runs a shell command and returns the output |

Things you can ask the agent to do:

```
create a snake game in snake.py and run it
read main.rs and add proper error handling throughout
list all files in this project and write a README
run the test suite and fix whatever is failing
write a Python script that checks my public IP every hour and logs it
```

Dangerous commands like `rm -rf /` are blocked regardless of what the AI decides.

In-session commands: `/exit`, `/clear`, `/help`

---

### `nion ask`

A one-shot command. Ask something, get an answer, done. No session, no history. Good for quick lookups and shell scripts.

```bash
nion ask "What does the Rust borrow checker actually do?"
nion ask "Write a one-liner to count lines in a file" -p groq
nion ask "Translate: good morning" -p anthropic -m claude-3-5-haiku-20241022
```

---

### `nion config`

Manages your keys and settings.

```bash
nion config setup                      # interactive setup wizard
nion config set-key groq gsk_xxx       # add or update a specific key
nion config set-key openai sk-xxx
nion config show                       # print current config
```

---

### `nion models`

Lists every available model across all providers.

```bash
nion models
```

---

### `nion update`

Checks GitHub for a newer version and updates the binary in place.

```bash
nion update
```

---

## Available models

**OpenAI** — `gpt-4o`, `gpt-4o-mini`, `gpt-4-turbo`, `gpt-3.5-turbo`, `o1`, `o1-mini`, `o3-mini`

**Anthropic** — `claude-3-5-sonnet-20241022`, `claude-3-5-haiku-20241022`, `claude-3-opus-20240229`, `claude-3-haiku-20240307`

**Google** — `gemini-1.5-pro`, `gemini-1.5-flash`, `gemini-2.0-flash`, `gemini-2.0-flash-thinking-exp`

**Groq** — `llama-3.3-70b-versatile`, `llama-3.1-8b-instant`, `llama3-70b-8192`, `mixtral-8x7b-32768`, `gemma2-9b-it`, `qwen-2.5-72b`

**xAI** — `grok-2-latest`, `grok-2-vision-latest`, `grok-beta`

**DeepSeek** — `deepseek-chat`, `deepseek-reasoner`

**Mistral** — `mistral-large-latest`, `mistral-small-latest`, `codestral-latest`, `open-mistral-nemo`

**Perplexity** — `sonar-pro`, `sonar`, `sonar-reasoning-pro`

**Together AI** — `meta-llama/Llama-3.3-70B-Instruct-Turbo`, `deepseek-ai/DeepSeek-V3`, `Qwen/Qwen2.5-72B-Instruct-Turbo`, `mistralai/Mixtral-8x22B-Instruct-v0.1`

**Cohere** — `command-r-plus-08-2024`, `command-r-08-2024`, `command-light`

---

## Where nion is useful

**On a remote server.** A single binary you can drop in and start using immediately. No Python environment, no npm, no runtime to install.

**On Android via Termux.** The `nion-aarch64-linux` binary is built with the Android NDK so it links against Bionic libc and works natively in Termux. TLS certificates are bundled via `rustls`, so there is no dependency on system OpenSSL.

**In shell scripts.** `nion ask` returns plain text to stdout, so you can pipe it into other commands or use it in automation.

**When comparing models.** Because all providers are in one tool, switching between GPT-4o, Claude, and Llama takes a single command in the same terminal session.

**For agentic coding tasks.** Give `nion agent` a task — write a script, fix a bug, refactor a module — and it handles it end to end without you needing to copy-paste code back and forth.

---

## Configuration file

Stored at `~/.nion/config.toml`. You can edit it directly if needed.

```toml
default_provider = "groq"
default_model = "llama-3.3-70b-versatile"
user_name = "sabab"

[api_keys]
groq = "gsk_..."
openai = "sk-..."
anthropic = "sk-ant-..."
```

---

## Platform support

| Platform | Binary |
|---|---|
| Linux x86_64 | `nion-x86_64-linux` |
| Android / Termux (aarch64) | `nion-aarch64-linux` |
| macOS Intel | `nion-x86_64-macos` |
| macOS Apple Silicon | `nion-aarch64-macos` |
| Windows x86_64 | `nion-x86_64-windows.exe` |

---

## Contributing

Contributions are welcome — bug fixes, new providers, new features, or documentation improvements.

### Reporting a bug

Open an issue on [GitHub](https://github.com/asikrshoudo/nion-cli/issues) with:

- The exact command you ran
- What you expected to happen
- What actually happened (paste the full error output)
- Your platform and nion version (`nion --version`)

### Suggesting a feature

Open an issue describing what you want and why. If it is a small change, a pull request without an issue first is fine too.

### Submitting a pull request

**Step 1 — Fork the repository**

Go to [github.com/asikrshoudo/nion-cli](https://github.com/asikrshoudo/nion-cli) and click **Fork** in the top right. This creates a copy of the repo under your own GitHub account.

**Step 2 — Clone your fork**

```bash
git clone https://github.com/YOUR_USERNAME/nion-cli
cd nion-cli
```

**Step 3 — Create a branch**

Name it something descriptive:

```bash
git checkout -b fix/describe-what-you-fixed
# or
git checkout -b feat/describe-what-you-added
```

**Step 4 — Make your changes**

If you are adding a new provider, `src/providers/groq.rs` is the simplest reference to follow. After writing your provider, you also need to register it in three places:

- `src/providers/mod.rs` — add it to the `get_provider()` match block
- `src/ui/mod.rs` — add its models to `print_models_list()`
- `src/config/mod.rs` — add it to the provider list in `run_setup_wizard()`

**Step 5 — Test your changes**

```bash
cargo build
cargo run -- chat
cargo run -- agent
```

**Step 6 — Commit**

Write a clear commit message:

```bash
git add .
git commit -m "feat: add XYZ provider"
# or
git commit -m "fix: describe what was broken and how you fixed it"
```

**Step 7 — Push to your fork**

```bash
git push origin feat/describe-what-you-added
```

**Step 8 — Open a pull request**

Go to your fork on GitHub. You will see a banner saying your branch is ahead of `asikrshoudo/nion-cli`. Click **Compare & pull request**.

Write a description explaining:
- What you changed and why
- How to test it
- Any known limitations

Then click **Create pull request**. That is it — the PR will be reviewed and merged if everything looks good.

### Code guidelines

- Run `cargo fmt` before committing
- Avoid `unwrap()` in code that runs at runtime — use `?` or handle the error explicitly
- Keep provider files self-contained — do not import from other provider files
- If your provider uses a different system prompt format, override `complete_with_system` in your implementation (see `src/providers/anthropic.rs` for an example)

---

## Project structure

```
nion-cli/
├── src/
│   ├── main.rs             entry point
│   ├── cli/mod.rs          all CLI commands and routing
│   ├── agent/mod.rs        agentic loop and tool execution
│   ├── config/mod.rs       config file and setup wizard
│   ├── session/mod.rs      Message type (user/assistant)
│   ├── ui/mod.rs           terminal output, menus, box rendering
│   ├── updater/mod.rs      auto-update from GitHub releases
│   └── providers/
│       ├── mod.rs          Provider trait and factory function
│       ├── openai.rs
│       ├── anthropic.rs
│       ├── google.rs
│       ├── groq.rs
│       ├── grok.rs
│       ├── deepseek.rs
│       ├── mistral.rs
│       ├── perplexity.rs
│       ├── together.rs
│       └── cohere.rs
├── npm/
│   ├── package.json        npm package definition
│   ├── install.js          post-install script (downloads binary)
│   └── bin/                binary lands here after install
├── .github/workflows/
│   └── release.yml         builds for all platforms + publishes to npm
├── install.sh              curl installer
└── Cargo.toml
```

---

## Security

API keys are stored only in `~/.nion/config.toml` on your machine and are sent only to the API endpoint of the provider you are talking to. Nion has no telemetry and collects no data.

In agent mode, a hardcoded blocklist prevents the AI from running commands like `rm -rf /`, `mkfs`, or `dd if=` regardless of what the model decides. That said, agent mode gives the AI real access to your filesystem and shell, so use your judgment about what tasks you give it.

---

## License

[AGPL-3.0](LICENSE). Free to use, fork, and modify. If you distribute a modified version, it must also be open source under the same license.

---

Built in Rust by [asikrshoudo](https://github.com/asikrshoudo).
