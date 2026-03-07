# ⚡ Nion CLI

> **The Universal AI CLI — One tool. Every model. Every platform.**

[![CI](https://github.com/asikrshoudo/nion-cli/actions/workflows/ci.yml/badge.svg)](https://github.com/asikrshoudo/nion-cli/actions/workflows/ci.yml)
[![Release](https://github.com/asikrshoudo/nion-cli/actions/workflows/release.yml/badge.svg)](https://github.com/asikrshoudo/nion-cli/releases)
[![License: AGPL-3.0](https://img.shields.io/badge/License-AGPL%20v3-blue.svg)](LICENSE)
[![Built with Rust](https://img.shields.io/badge/Built%20with-Rust-%23CE4A00.svg?logo=rust)](https://www.rust-lang.org)
[![Platform](https://img.shields.io/badge/Platform-Linux%20%7C%20macOS%20%7C%20Windows%20%7C%20Android-brightgreen)]()

```
  ███╗   ██╗██╗ ██████╗ ███╗   ██╗
  ████╗  ██║██║██╔═══██╗████╗  ██║
  ██╔██╗ ██║██║██║   ██║██╔██╗ ██║
  ██║╚██╗██║██║██║   ██║██║╚██╗██║
  ██║ ╚████║██║╚██████╔╝██║ ╚████║
  ╚═╝  ╚═══╝╚═╝ ╚═════╝ ╚═╝  ╚═══╝
  ⚡ The Universal AI CLI
  One tool. Every model. Every platform.
```

---

## Why Nion?

Most AI CLI tools lock you into one provider. **Nion doesn't.**

Connect to **OpenAI**, **Claude**, **Gemini**, **Groq**, and **Grok** — all from one tool, one config, one command. Built in Rust for maximum speed and a single zero-dependency binary.

| Feature | Nion |
|---------|------|
| 🦀 Language | Rust (fast, tiny binary) |
| 🌍 Platforms | Linux, macOS, Windows, Android (Termux) |
| 🔄 Auto-update | ✅ Notifies on every new GitHub release |
| 💬 Multi-turn chat | ✅ Full conversation history |
| ⚡ Groq support | ✅ Free tier, blazing fast |
| 🔑 Key management | ✅ Simple `~/.nion/config.toml` |
| 🆓 License | AGPL-3.0 (open source forever) |

---

## Install

### ⚡ One-line (Linux / macOS / Android Termux)

```bash
curl -sSL https://raw.githubusercontent.com/asikrshoudo/nion-cli/main/install.sh | bash
```

### 📱 Android (Termux) — Build from source

```bash
pkg update && pkg upgrade
pkg install rust git openssl pkg-config
git clone https://github.com/asikrshoudo/nion-cli.git
cd nion-cli
cargo build --release
cp target/release/nion $PREFIX/bin/
```

### 🐧 Linux (any distro)

```bash
# Build from source
git clone https://github.com/asikrshoudo/nion-cli.git
cd nion-cli
cargo build --release
sudo cp target/release/nion /usr/local/bin/
```

### 🍎 macOS

```bash
git clone https://github.com/asikrshoudo/nion-cli.git
cd nion-cli
cargo build --release
cp target/release/nion /usr/local/bin/
```

### 🪟 Windows

```powershell
# Install Rust from https://rustup.rs first, then:
git clone https://github.com/asikrshoudo/nion-cli.git
cd nion-cli
cargo build --release
# Binary: target\release\nion.exe — add folder to PATH
```

---

## Quick Start

```bash
# 1. Add your API keys (interactive wizard)
nion config setup

# 2. Ask a question
nion ask "What is Rust?"

# 3. Start an interactive chat
nion chat
```

---

## Usage

### `nion ask` — Quick single question

```bash
nion ask "Explain recursion"
nion ask -p gemini "Write a haiku about the ocean"
nion ask -p groq -m llama-3.3-70b-versatile "Write a Python script"
nion ask -p claude "What are SOLID principles?"
nion ask -p grok "Explain transformers in AI"
```

### `nion chat` — Interactive multi-turn chat

```bash
nion chat                           # Default provider
nion chat -p openai                 # GPT-4o
nion chat -p anthropic              # Claude 3.5 Sonnet
nion chat -p gemini                 # Gemini 1.5 Pro
nion chat -p groq                   # Llama 3.3 70B (FREE ⚡)
nion chat -p grok                   # xAI Grok
nion chat -p groq -m gemma2-9b-it   # Specific model
```

#### In-chat commands

| Command | What it does |
|---------|-------------|
| `/exit` | End the session |
| `/clear` | Clear conversation history |
| `/model gpt-4o` | Switch model mid-chat |
| `/switch groq` | Switch to a different provider |
| `/help` | Show all commands |

### `nion config` — Manage settings

```bash
nion config setup                         # Interactive setup wizard
nion config set-key openai sk-...         # Set OpenAI key
nion config set-key anthropic sk-ant-...  # Set Claude key
nion config set-key google AIza...        # Set Gemini key
nion config set-key groq gsk_...          # Set Groq key (FREE!)
nion config set-key grok xai-...          # Set Grok key
nion config set-provider groq             # Set default provider
nion config set-model gpt-4o              # Set default model
nion config show                          # View current config
nion config list-models                   # All available models
```

### `nion update` — Manual update check

```bash
nion update
```

> Nion checks for updates **automatically** every time you run it.
> When a new release appears on GitHub, it asks:
>
> ```
> ──────────────────────────────────────────────────────────────
>   ⚡ Nion v0.2.0 is available! Would you like to update? [Y/n]
> ──────────────────────────────────────────────────────────────
>   Y  →  downloads new binary, replaces itself, done
>   N  →  keeps running as-is, update later with `nion update`
> ```

---

## Supported Providers & Models

| Provider | Models | Free Tier |
|----------|--------|-----------|
| **OpenAI** | gpt-4o, gpt-4o-mini, gpt-4-turbo, gpt-3.5-turbo | ❌ |
| **Anthropic** | claude-3-5-sonnet, claude-3-5-haiku, claude-3-opus | ❌ |
| **Google** | gemini-1.5-pro, gemini-1.5-flash, gemini-2.0-flash | ✅ |
| **Groq** ⚡ | llama-3.3-70b, llama-3.1-8b, mixtral-8x7b, gemma2-9b | ✅ |
| **xAI** | grok-2-latest, grok-beta | ❌ |

> 💡 **New to AI APIs?** Start with **Groq** — free, no credit card, blazing fast.
> Get your key: [console.groq.com](https://console.groq.com)

---

## Config File

Stored at `~/.nion/config.toml`:

```toml
default_provider = "groq"
default_model = "llama-3.3-70b-versatile"

[api_keys]
openai    = "sk-..."
anthropic = "sk-ant-..."
google    = "AIza..."
groq      = "gsk_..."
grok      = "xai-..."
```

---

## Contributing

PRs and issues welcome at [github.com/asikrshoudo/nion-cli](https://github.com/asikrshoudo/nion-cli)

```bash
git clone https://github.com/asikrshoudo/nion-cli.git
cd nion-cli
cargo build
cargo test
```

---

## License

**GNU Affero General Public License v3.0** — see [LICENSE](LICENSE)

Nion will always be free and open source.

---

*Made with ⚡ and 🦀 by [asikrshoudo](https://github.com/asikrshoudo)*
