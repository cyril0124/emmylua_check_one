#!/usr/bin/env bash

set -euo pipefail

REPO="cyril0124/emmylua_check_one"
BIN_NAME="emmylua_check_one"

# Detect platform
OS=$(uname -s)
ARCH=$(uname -m)

case "$OS" in
    Linux)
        case "$ARCH" in
            x86_64) TARGET="x86_64-unknown-linux-gnu" ;;
            *) echo "Unsupported architecture: $ARCH on Linux"; exit 1 ;;
        esac
        ;;
    Darwin)
        case "$ARCH" in
            x86_64) TARGET="x86_64-apple-darwin" ;;
            arm64)  TARGET="aarch64-apple-darwin" ;;
            *) echo "Unsupported architecture: $ARCH on macOS"; exit 1 ;;
        esac
        ;;
    *)
        echo "Unsupported OS: $OS"
        exit 1
        ;;
esac

# Resolve install directory
if [ -n "${INSTALL_DIR:-}" ]; then
    INSTALL_DIR="$INSTALL_DIR"
elif [ -w "/usr/local/bin" ]; then
    INSTALL_DIR="/usr/local/bin"
else
    INSTALL_DIR="$HOME/.local/bin"
fi

# Resolve version
if [ -n "${VERSION:-}" ]; then
    VERSION="${VERSION#v}"  # strip leading v if present
    RELEASE_URL="https://github.com/$REPO/releases/download/v$VERSION"
else
    echo "Fetching latest release..."
    # Use redirect URL instead of GitHub API to avoid rate limits
    LATEST_URL=$(curl -fsSLI -o /dev/null -w '%{url_effective}' "https://github.com/$REPO/releases/latest")
    LATEST=$(basename "$LATEST_URL" | sed 's/^v//')
    if [ -z "$LATEST" ] || [ "$LATEST" = "latest" ]; then
        echo "Failed to determine latest version"
        exit 1
    fi
    VERSION="$LATEST"
    RELEASE_URL="https://github.com/$REPO/releases/download/v$VERSION"
    echo "Latest version: v$VERSION"
fi

ARCHIVE="${BIN_NAME}-${TARGET}.tar.gz"
URL="$RELEASE_URL/$ARCHIVE"

# Download
echo "Downloading $ARCHIVE..."
TMP_DIR=$(mktemp -d)
trap 'rm -rf "$TMP_DIR"' EXIT

curl -fsSL "$URL" -o "$TMP_DIR/$ARCHIVE"

# Extract
echo "Extracting..."
tar -xzf "$TMP_DIR/$ARCHIVE" -C "$TMP_DIR"

# Install
mkdir -p "$INSTALL_DIR"
cp "$TMP_DIR/$BIN_NAME" "$INSTALL_DIR/"
chmod +x "$INSTALL_DIR/$BIN_NAME"

echo ""
echo "$BIN_NAME v$VERSION installed to $INSTALL_DIR/$BIN_NAME"

# Check PATH
if ! command -v "$BIN_NAME" >/dev/null 2>&1; then
    echo ""
    echo "WARNING: $INSTALL_DIR is not in your PATH."
    echo "Add the following to your shell profile:"
    echo ""
    echo "  export PATH=\"$INSTALL_DIR:\$PATH\""
    echo ""
fi

echo "Usage:"
echo "  $BIN_NAME -c .emmyrc.json src/path/to/File.lua"
echo "  $BIN_NAME src/path/to/File.lua"
