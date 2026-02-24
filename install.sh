#!/usr/bin/env bash
#
# jjj Installation Script
# Builds and installs jjj to a location in your PATH

set -euo pipefail

# Colors
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo -e "${BLUE}jjj Installation Script${NC}"
echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo

# Check if cargo is installed
if ! command -v cargo &> /dev/null; then
    echo -e "${RED}❌ Error: cargo not found${NC}"
    echo "   Please install Rust: https://rustup.rs/"
    exit 1
fi

# Check if jj is installed
if ! command -v jj &> /dev/null; then
    echo -e "${YELLOW}⚠️  Warning: jj (Jujutsu) not found in PATH${NC}"
    echo "   jjj requires Jujutsu to function."
    echo
    echo "   Install with:"
    echo "     macOS: brew install jj"
    echo "     From source: cargo install --git https://github.com/jj-vcs/jj jj-cli"
    echo
    read -p "Continue anyway? (y/N) " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        exit 1
    fi
fi

# Determine install location
if [ -n "${CARGO_HOME:-}" ]; then
    INSTALL_DIR="$CARGO_HOME/bin"
elif [ -d "$HOME/.cargo/bin" ]; then
    INSTALL_DIR="$HOME/.cargo/bin"
else
    INSTALL_DIR="/usr/local/bin"
fi

echo -e "${BLUE}📦 Building jjj...${NC}"
cargo build --release

if [ $? -ne 0 ]; then
    echo -e "${RED}❌ Build failed${NC}"
    exit 1
fi

echo -e "${GREEN}✅ Build successful${NC}"
echo

# Check if install directory is writable
if [ -w "$INSTALL_DIR" ]; then
    echo -e "${BLUE}📥 Installing to $INSTALL_DIR...${NC}"
    cp target/release/jjj "$INSTALL_DIR/"
    chmod +x "$INSTALL_DIR/jjj"
    echo -e "${GREEN}✅ Installation successful${NC}"
else
    echo -e "${YELLOW}⚠️  $INSTALL_DIR is not writable${NC}"
    echo "   Attempting to install with sudo..."
    sudo cp target/release/jjj "$INSTALL_DIR/"
    sudo chmod +x "$INSTALL_DIR/jjj"
    echo -e "${GREEN}✅ Installation successful (with sudo)${NC}"
fi

echo
echo -e "${GREEN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo -e "${GREEN}🎉 jjj installed successfully!${NC}"
echo -e "${GREEN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo
echo "Installation location: $INSTALL_DIR/jjj"
echo
echo "Verify installation:"
echo "  jjj --version"
echo
echo "Get started:"
echo "  cd /path/to/your/jj/repo"
echo "  jjj init"
echo "  jjj ui"
echo
echo "Documentation:"
echo "  README.md          - Project overview"
echo "  https://dougfritz.com/jjj/ - Full documentation"
echo
