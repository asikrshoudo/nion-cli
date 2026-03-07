#!/usr/bin/env bash
set -e

REPO="asikrshoudo/nion-cli"
INSTALL_DIR="$HOME/.local/bin"

OS=$(uname -s | tr '[:upper:]' '[:lower:]')
ARCH=$(uname -m)

case "$ARCH" in
  x86_64)          ARCH_LABEL="x86_64" ;;
  aarch64|arm64)   ARCH_LABEL="aarch64" ;;
  armv7*)          ARCH_LABEL="armv7" ;;
  *)
    echo "Unsupported architecture: $ARCH"
    exit 1 ;;
esac

case "$OS" in
  linux)   TARGET="${ARCH_LABEL}-unknown-linux-musl" ;;
  darwin)  TARGET="${ARCH_LABEL}-apple-darwin" ;;
  msys*|cygwin*|mingw*)
    echo "Windows: download nion.exe from https://github.com/${REPO}/releases"
    exit 0 ;;
  *)
    echo "Unsupported OS: $OS"
    exit 1 ;;
esac

printf '\n'
printf '  ███╗   ██╗██╗ ██████╗ ███╗   ██╗\n'
printf '  ████╗  ██║██║██╔═══██╗████╗  ██║\n'
printf '  ██╔██╗ ██║██║██║   ██║██╔██╗ ██║\n'
printf '  ██║╚██╗██║██║██║   ██║██║╚██╗██║\n'
printf '  ██║ ╚████║██║╚██████╔╝██║ ╚████║\n'
printf '  ╚═╝  ╚═══╝╚═╝ ╚═════╝ ╚═╝  ╚═══╝\n'
printf '\n'
printf '  Nion CLI Installer\n'
printf '  Target: %s\n\n' "$TARGET"

mkdir -p "$INSTALL_DIR"

# --- Try binary install from GitHub Releases first ---
LATEST=$(curl -sf "https://api.github.com/repos/${REPO}/releases/latest" \
  | grep '"tag_name"' | cut -d'"' -f4 2>/dev/null || true)

if [ -n "$LATEST" ]; then
  URL="https://github.com/${REPO}/releases/download/${LATEST}/nion-${TARGET}"
  printf '  Version : %s\n' "$LATEST"
  printf '  Method  : Binary download\n\n'

  if curl -sfL "$URL" -o "$INSTALL_DIR/nion"; then
    chmod +x "$INSTALL_DIR/nion"
    printf '  [OK] Installed to %s/nion\n\n' "$INSTALL_DIR"
  else
    printf '  Binary not found for this platform, falling back to source build.\n\n'
    LATEST=""
  fi
fi

# --- Fallback: build from source ---
if [ -z "$LATEST" ]; then
  printf '  Method  : Build from source\n\n'

  # Check for Rust
  if ! command -v cargo &>/dev/null; then
    printf '  Rust not found. Installing via rustup...\n'
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --no-modify-path
    export PATH="$HOME/.cargo/bin:$PATH"
  fi

  # Check for git
  if ! command -v git &>/dev/null; then
    printf '  Error: git is not installed.\n'
    printf '  Termux  : pkg install git\n'
    printf '  Ubuntu  : sudo apt install git\n'
    printf '  macOS   : xcode-select --install\n'
    exit 1
  fi

  TMPDIR=$(mktemp -d)
  printf '  Cloning repository...\n'
  git clone --depth 1 "https://github.com/${REPO}.git" "$TMPDIR/nion-cli" -q

  printf '  Compiling (this takes a few minutes)...\n'
  cd "$TMPDIR/nion-cli"
  cargo build --release -q

  cp target/release/nion "$INSTALL_DIR/nion"
  chmod +x "$INSTALL_DIR/nion"
  cd - > /dev/null
  rm -rf "$TMPDIR"

  printf '  [OK] Built and installed to %s/nion\n\n' "$INSTALL_DIR"
fi

# PATH reminder
if [[ ":$PATH:" != *":$INSTALL_DIR:"* ]]; then
  printf '  NOTE: Add this to your shell config (~/.bashrc or ~/.zshrc):\n\n'
  printf '    export PATH="$HOME/.local/bin:$PATH"\n\n'
  printf '  Then run: source ~/.bashrc\n\n'
else
  printf '  PATH is already configured.\n\n'
fi

printf '  Get started:\n'
printf '    nion config setup    <- add your API keys\n'
printf '    nion chat            <- start chatting\n'
printf '    nion ask "Hello"     <- quick question\n\n'
