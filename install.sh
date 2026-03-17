#!/usr/bin/env bash
set -euo pipefail

# mig-gpu-mon installer
# Usage: ./install.sh
# Installs Rust (if needed), builds mig-gpu-mon, and registers it in PATH.

BOLD='\033[1m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
RED='\033[0;31m'
RESET='\033[0m'

info()  { echo -e "${GREEN}[INFO]${RESET} $*"; }
warn()  { echo -e "${YELLOW}[WARN]${RESET} $*"; }
error() { echo -e "${RED}[ERROR]${RESET} $*"; exit 1; }

# ── 1. Check OS ──────────────────────────────────────────────────────
if [[ "$(uname -s)" != "Linux" ]]; then
    error "This tool only supports Linux (detected: $(uname -s))"
fi

# ── 2. Install build dependencies ────────────────────────────────────
install_pkg() {
    local pkg_apt="$1" pkg_rpm="$2"
    if command -v apt-get &>/dev/null; then
        sudo apt-get update -qq && sudo apt-get install -y -qq $pkg_apt
    elif command -v dnf &>/dev/null; then
        sudo dnf install -y -q $pkg_rpm
    elif command -v yum &>/dev/null; then
        sudo yum install -y -q $pkg_rpm
    else
        error "Could not determine package manager (apt/dnf/yum). Install manually: $pkg_apt (Debian) or $pkg_rpm (RHEL)"
    fi
}

# curl — required for rustup
if ! command -v curl &>/dev/null; then
    warn "curl not found. Installing..."
    install_pkg "curl" "curl"
fi

# gcc/cc — required as C linker for cargo build
if ! command -v cc &>/dev/null && ! command -v gcc &>/dev/null; then
    warn "C compiler (gcc) not found. Installing..."
    install_pkg "build-essential" "gcc"
fi

# git
if ! command -v git &>/dev/null; then
    warn "git not found. Installing..."
    install_pkg "git" "git"
fi

# ── 3. Install Rust if not present ───────────────────────────────────
if command -v cargo &>/dev/null; then
    info "Rust already installed: $(rustc --version)"
else
    info "Rust not found. Installing via rustup..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    source "$HOME/.cargo/env"
    info "Rust installed: $(rustc --version)"
fi

# ── 4. Build ─────────────────────────────────────────────────────────
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

if [[ -f "$SCRIPT_DIR/Cargo.toml" ]]; then
    info "Building mig-gpu-mon (release mode)..."
    cd "$SCRIPT_DIR"
    cargo build --release
else
    error "Cargo.toml not found in $SCRIPT_DIR. Run this script from the project root."
fi

# ── 5. Install ───────────────────────────────────────────────────────
BINARY="$SCRIPT_DIR/target/release/mig-gpu-mon"
if [[ ! -f "$BINARY" ]]; then
    error "Build succeeded but binary not found at $BINARY"
fi

INSTALL_DIR=""
if [[ -d "$HOME/.cargo/bin" ]]; then
    INSTALL_DIR="$HOME/.cargo/bin"
elif [[ -w "/usr/local/bin" ]]; then
    INSTALL_DIR="/usr/local/bin"
else
    INSTALL_DIR="$HOME/.local/bin"
    mkdir -p "$INSTALL_DIR"
fi

cp "$BINARY" "$INSTALL_DIR/mig-gpu-mon"
chmod +x "$INSTALL_DIR/mig-gpu-mon"

# ── 6. Verify ────────────────────────────────────────────────────────
if command -v mig-gpu-mon &>/dev/null; then
    info "Installation complete!"
    echo ""
    echo -e "  ${BOLD}Binary:${RESET}   $INSTALL_DIR/mig-gpu-mon"
    echo -e "  ${BOLD}Size:${RESET}     $(du -h "$INSTALL_DIR/mig-gpu-mon" | cut -f1)"
    echo -e "  ${BOLD}Run:${RESET}      mig-gpu-mon"
    echo -e "  ${BOLD}Help:${RESET}     mig-gpu-mon --help"
    echo ""
else
    warn "Binary installed to $INSTALL_DIR/mig-gpu-mon"
    warn "but it is not in PATH. Add this to your shell profile:"
    echo ""
    echo "  export PATH=\"$INSTALL_DIR:\$PATH\""
    echo ""
fi
