#!/usr/bin/env bash
set -e

REPO="asikrshoudo/nion-cli"
INSTALL_DIR="$HOME/.local/bin"

OS=$(uname -s | tr '[:upper:]' '[:lower:]')
ARCH=$(uname -m)

printf '\n'
printf '  ███╗   ██╗██╗ ██████╗ ███╗   ██╗\n'
printf '  ████╗  ██║██║██╔═══██╗████╗  ██║\n'
printf '  ██╔██╗ ██║██║██║   ██║██╔██╗ ██║\n'
printf '  ██║╚██╗██║██║██║   ██║██║╚██╗██║\n'
printf '  ██║ ╚████║██║╚██████╔╝██║ ╚████║\n'
printf '  ╚═╝  ╚═══╝╚═╝ ╚═════╝ ╚═╝  ╚═══╝\n'
printf '\n'
printf '  Nion CLI Installer\n\n'

show_path_hint() {
  if [[ ":$PATH:" != *":$INSTALL_DIR:"* ]]; then
    printf '  NOTE: Add the following to your ~/.bashrc or ~/.zshrc:\n\n'
    printf '    export PATH="$HOME/.local/bin:$PATH"\n\n'
    printf '  Then run: source ~/.bashrc\n\n'
  fi
}

show_getstarted() {
  printf '  Get started:\n'
  printf '    nion config setup    <- add your API keys\n'
  printf '    nion chat            <- start chatting\n'
  printf '    nion ask "Hello"     <- quick question\n\n'
}

# Detect binary name based on OS + ARCH
BINARY=""
case "$OS" in
  linux)
    case "$ARCH" in
      x86_64)        BINARY="nion-x86_64-linux" ;;
      aarch64|arm64) BINARY="nion-aarch64-linux" ;;
    esac
    ;;
  darwin)
    case "$ARCH" in
      x86_64)        BINARY="nion-x86_64-macos" ;;
      arm64|aarch64) BINARY="nion-aarch64-macos" ;;
    esac
    ;;
  *)
    printf '  Unsupported OS: %s\n' "$OS"
    printf '  Please build manually: cargo build --release\n'
    exit 1
    ;;
esac

mkdir -p "$INSTALL_DIR"

# Try prebuilt binary first
if [ -n "$BINARY" ]; then
  LATEST=$(curl -sf "https://api.github.com/repos/${REPO}/releases/latest" \
    | grep '"tag_name"' | cut -d'"' -f4 2>/dev/null || true)

  if [ -n "$LATEST" ]; then
    URL="https://github.com/${REPO}/releases/download/${LATEST}/${BINARY}"
    printf '  Version  : %s\n' "$LATEST"
    printf '  Platform : %s\n' "$BINARY"
    printf '  Method   : Binary download\n\n'
    printf '  Downloading...\n'

    if curl -sfL "$URL" -o "$INSTALL_DIR/nion"; then
      chmod +x "$INSTALL_DIR/nion"
      printf '  [OK] Installed to %s/nion\n\n' "$INSTALL_DIR"
      show_path_hint
      show_getstarted
      exit 0
    else
      printf '  Download failed, falling back to source build.\n\n'
    fi
  else
    printf '  No release found yet, building from source.\n\n'
  fi
fi

# Fallback: build from source
printf '  Method   : Build from source\n\n'

if ! command -v cargo &>/dev/null; then
  printf '  Rust not found. Installing...\n\n'
  if command -v pkg &>/dev/null; then
    # Termux
    pkg install -y rust git
  else
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --no-modify-path
    export PATH="$HOME/.cargo/bin:$PATH"
  fi
fi

if ! command -v git &>/dev/null; then
  if command -v pkg &>/dev/null; then
    pkg install -y git
  elif command -v apt-get &>/dev/null; then
    sudo apt-get install -y git
  else
    printf '  Please install git and re-run.\n'
    exit 1
  fi
fi

TMPDIR_BUILD=$(mktemp -d)
printf '  Cloning repository...\n'
git clone --depth 1 "https://github.com/${REPO}.git" "$TMPDIR_BUILD/nion-cli" -q

printf '  Compiling (first run takes a few minutes)...\n'
cd "$TMPDIR_BUILD/nion-cli"
cargo build --release -q

cp target/release/nion "$INSTALL_DIR/nion"
chmod +x "$INSTALL_DIR/nion"
cd - > /dev/null
rm -rf "$TMPDIR_BUILD"

printf '  [OK] Built and installed to %s/nion\n\n' "$INSTALL_DIR"
show_path_hint
show_getstarted
