#!/usr/bin/env bash
#
# jjj installer.
#
# Downloads the prebuilt binary for this platform from the latest GitHub
# release, verifies its SHA-256, and installs it. Falls back to building from
# source when no prebuilt binary matches — or when run from inside a checkout
# with --from-source.
#
#   curl -fsSL https://raw.githubusercontent.com/doug/jjj/main/install.sh | bash
#   ./install.sh --from-source
#   ./install.sh --version 0.5.1

set -euo pipefail

REPO="doug/jjj"
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m'

FROM_SOURCE=0
VERSION=""

while [ $# -gt 0 ]; do
    case "$1" in
        --from-source) FROM_SOURCE=1; shift ;;
        --version) VERSION="${2:-}"; shift 2 ;;
        -h|--help)
            sed -n '3,12p' "$0" | sed 's/^# \{0,1\}//'
            exit 0 ;;
        *) echo "Unknown option: $1" >&2; exit 1 ;;
    esac
done

info()  { printf "${BLUE}%s${NC}\n" "$*"; }
ok()    { printf "${GREEN}%s${NC}\n" "$*"; }
warn()  { printf "${YELLOW}%s${NC}\n" "$*"; }
fail()  { printf "${RED}%s${NC}\n" "$*" >&2; exit 1; }

printf "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}\n"
printf "${BLUE}jjj installer${NC}\n"
printf "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}\n\n"

# --- jj is a hard requirement ------------------------------------------------

if ! command -v jj >/dev/null 2>&1; then
    warn "⚠️  jj (Jujutsu) not found in PATH"
    echo "   jjj operates on a jj repository and needs it at runtime."
    echo
    echo "   Install with:"
    echo "     macOS: brew install jj"
    echo "     Cargo: cargo install --git https://github.com/jj-vcs/jj jj-cli"
    echo
    if [ -t 0 ]; then
        read -p "Continue anyway? (y/N) " -n 1 -r
        echo
        [[ $REPLY =~ ^[Yy]$ ]] || exit 1
    else
        echo "   Continuing — install jj before running jjj."
    fi
fi

# --- where to install --------------------------------------------------------

if [ -n "${CARGO_HOME:-}" ]; then
    INSTALL_DIR="$CARGO_HOME/bin"
elif [ -d "$HOME/.cargo/bin" ]; then
    INSTALL_DIR="$HOME/.cargo/bin"
elif [ -d "$HOME/.local/bin" ]; then
    INSTALL_DIR="$HOME/.local/bin"
else
    INSTALL_DIR="/usr/local/bin"
fi

install_binary() {
    local src="$1"
    mkdir -p "$INSTALL_DIR" 2>/dev/null || true
    if [ -w "$INSTALL_DIR" ]; then
        info "📥 Installing to $INSTALL_DIR..."
        cp "$src" "$INSTALL_DIR/jjj"
        chmod +x "$INSTALL_DIR/jjj"
    else
        warn "⚠️  $INSTALL_DIR is not writable — using sudo"
        sudo cp "$src" "$INSTALL_DIR/jjj"
        sudo chmod +x "$INSTALL_DIR/jjj"
    fi
    ok "✅ Installed"
}

build_from_source() {
    command -v cargo >/dev/null 2>&1 || fail "❌ cargo not found. Install Rust: https://rustup.rs/"
    [ -f Cargo.toml ] || fail "❌ --from-source must run inside a jjj checkout"
    info "📦 Building from source..."
    cargo build --release || fail "❌ Build failed"
    ok "✅ Build successful"
    install_binary target/release/jjj
}

# --- prebuilt path -----------------------------------------------------------

detect_target() {
    local os arch
    os=$(uname -s)
    arch=$(uname -m)
    case "$os/$arch" in
        Darwin/arm64)         echo "aarch64-apple-darwin" ;;
        Darwin/x86_64)        echo "x86_64-apple-darwin" ;;
        Linux/x86_64)         echo "x86_64-unknown-linux-gnu" ;;
        Linux/aarch64|Linux/arm64) echo "aarch64-unknown-linux-gnu" ;;
        *)                    echo "" ;;
    esac
}

latest_version() {
    curl -fsSL "https://api.github.com/repos/$REPO/releases/latest" 2>/dev/null \
        | sed -n 's/.*"tag_name": *"v\{0,1\}\([^"]*\)".*/\1/p' \
        | head -1
}

install_prebuilt() {
    local target version url tmp
    target=$(detect_target)
    [ -n "$target" ] || { warn "⚠️  No prebuilt binary for $(uname -s)/$(uname -m)"; return 1; }

    command -v curl >/dev/null 2>&1 || { warn "⚠️  curl not found"; return 1; }

    version="$VERSION"
    [ -n "$version" ] || version=$(latest_version)
    [ -n "$version" ] || { warn "⚠️  Could not determine the latest release"; return 1; }

    local name="jjj-${version}-${target}"
    url="https://github.com/$REPO/releases/download/v${version}/${name}.tar.gz"
    info "⬇️  Downloading jjj $version for $target..."

    tmp=$(mktemp -d)
    trap 'rm -rf "$tmp"' RETURN

    curl -fsSL "$url" -o "$tmp/$name.tar.gz" || { warn "⚠️  Download failed: $url"; return 1; }

    # Verify before unpacking. An installer that skips this is a supply-chain
    # hole regardless of how the artifact was built.
    if curl -fsSL "$url.sha256" -o "$tmp/$name.tar.gz.sha256" 2>/dev/null; then
        info "🔐 Verifying checksum..."
        local expected actual
        expected=$(awk '{print $1}' "$tmp/$name.tar.gz.sha256")
        if command -v sha256sum >/dev/null 2>&1; then
            actual=$(sha256sum "$tmp/$name.tar.gz" | awk '{print $1}')
        else
            actual=$(shasum -a 256 "$tmp/$name.tar.gz" | awk '{print $1}')
        fi
        [ "$expected" = "$actual" ] || fail "❌ Checksum mismatch — refusing to install"
        ok "✅ Checksum verified"
    else
        warn "⚠️  No checksum published for this release; skipping verification"
    fi

    tar -xzf "$tmp/$name.tar.gz" -C "$tmp"
    install_binary "$tmp/$name/jjj"
    return 0
}

if [ "$FROM_SOURCE" -eq 1 ]; then
    build_from_source
elif ! install_prebuilt; then
    warn "Falling back to building from source."
    build_from_source
fi

echo
printf "${GREEN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}\n"
printf "${GREEN}🎉 jjj installed${NC}\n"
printf "${GREEN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}\n\n"
echo "Location: $INSTALL_DIR/jjj"
echo
echo "Verify:"
echo "  jjj --version"
echo
echo "Get started:"
echo "  cd /path/to/your/jj/repo"
echo "  jjj init"
echo "  jjj ui"
echo
echo "Docs: https://jjj.recursivewhy.com"
echo
