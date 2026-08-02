#!/bin/bash

CYAN='\033[0;36m'
GREEN='\033[0;32m'
PURPLE='\033[0;35m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
BOLD='\033[1m'
NC='\033[0m'

echo -e "${PURPLE}🎧  Installing otopod (Anime Audio Condenser)...${NC}\n"

BIN_DIR="$HOME/.local/bin"
mkdir -p "$BIN_DIR"

# Check Prerequisites
echo -e "${BLUE}🔍 Checking dependencies...${NC}"

# 1. Check ffmpeg
if command -v ffmpeg >/dev/null 2>&1; then
    echo -e "  ${GREEN}✔ ffmpeg is installed${NC}\n"
else
    echo -e "  ${YELLOW}⚠️  ffmpeg is not installed (required for single-pass audio extraction and condensing).${NC}"
    echo -e "     Install via your package manager (e.g. 'sudo pacman -S ffmpeg' or 'sudo apt install ffmpeg').\n"
fi

REPO="Praveensenpai/otopod"
RELEASE_URL="https://github.com/${REPO}/releases/latest/download/otopod-linux-x86_64.tar.gz"

LOCAL_DIR=""
if [ -n "${BASH_SOURCE[0]}" ] && [ -f "${BASH_SOURCE[0]}" ]; then
    LOCAL_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" 2>/dev/null && pwd)"
fi

if [ -n "$LOCAL_DIR" ] && [ -f "$LOCAL_DIR/Cargo.toml" ] && command -v cargo >/dev/null 2>&1; then
    VERSION=$(grep -m1 '^version' "$LOCAL_DIR/Cargo.toml" | cut -d '"' -f2 2>/dev/null || echo "latest")
    echo -e "${BLUE}📦 Local source detected. Building otopod v${VERSION} with Cargo...${NC}"
    cargo build --release --manifest-path "$LOCAL_DIR/Cargo.toml"
    cp "$LOCAL_DIR/target/release/otopod" "$BIN_DIR/otopod"
    INSTALLED_VER="v${VERSION}"
else
    LATEST_TAG=$(curl -4 -sSL "https://api.github.com/repos/${REPO}/releases/latest" 2>/dev/null | grep -o '"tag_name": "[^"]*"' | cut -d'"' -f4)
    [ -z "$LATEST_TAG" ] && LATEST_TAG="latest"
    echo -e "${BLUE}📦 Downloading otopod ${LATEST_TAG} pre-compiled binary from GitHub Releases...${NC}"
    TMP_DIR=$(mktemp -d)
    if curl -4 -fL --connect-timeout 10 --retry 3 -sS "$RELEASE_URL" -o "$TMP_DIR/otopod.tar.gz"; then
        tar -xzf "$TMP_DIR/otopod.tar.gz" -C "$TMP_DIR"
        if [ -f "$TMP_DIR/otopod" ]; then
            cp "$TMP_DIR/otopod" "$BIN_DIR/otopod"
        elif [ -f "$TMP_DIR/dist/otopod" ]; then
            cp "$TMP_DIR/dist/otopod" "$BIN_DIR/otopod"
        fi
        rm -rf "$TMP_DIR"
        INSTALLED_VER="${LATEST_TAG}"
    else
        rm -rf "$TMP_DIR"
        echo -e "${RED}❌ Failed to download pre-compiled release.${NC}"
        exit 1
    fi
fi

if [ ! -f "$BIN_DIR/otopod" ] || [ ! -s "$BIN_DIR/otopod" ]; then
    echo -e "${RED}❌ Error: Failed to install otopod binary!${NC}"
    exit 1
fi

chmod +x "$BIN_DIR/otopod"
echo -e "${GREEN}✔ Installed otopod ${INSTALLED_VER} to ${BIN_DIR}/otopod${NC}"

# Shell alias setup
SHELL_CONFIGS=("$HOME/.bashrc" "$HOME/.zshrc")
ALIAS_LINE="alias otopod='$HOME/.local/bin/otopod'"

for config in "${SHELL_CONFIGS[@]}"; do
    if [ -f "$config" ]; then
        if ! grep -q "alias otopod=" "$config" 2>/dev/null; then
            echo "" >> "$config"
            echo "$ALIAS_LINE" >> "$config"
            echo -e "${BLUE}📝 Added otopod alias to $config${NC}"
        fi
    fi
done

echo -e "\n${GREEN}${BOLD}🎉 otopod ${INSTALLED_VER} installation completed!${NC}"
echo -e "Run it anytime with: ${CYAN}otopod${NC}"
