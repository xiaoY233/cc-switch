#!/usr/bin/env bash
set -euo pipefail

REPO="${CC_SWITCH_REPO:-xiaoY233/cc-switch}"
VERSION="${1:-${CC_SWITCH_REMOTE_HELPER_RELEASE_TAG:-remote-helper-latest}}"
BIN_DIR="${CC_SWITCH_BIN_DIR:-$HOME/.local/bin}"
BIN_NAME="${CC_SWITCH_BIN_NAME:-cc-switch-remote-helper}"
OS="$(uname -s)"
ARCH="$(uname -m)"
mkdir -p "$BIN_DIR"

case "$OS" in
  Linux) ASSET_OS="Linux" ;;
  Darwin) ASSET_OS="macOS" ;;
  *) echo "Unsupported OS: $OS" >&2; exit 1 ;;
esac

case "$ARCH" in
  x86_64|amd64) ASSET_ARCH="x86_64" ;;
  arm64|aarch64) ASSET_ARCH="arm64" ;;
  *) echo "Unsupported arch: $ARCH" >&2; exit 1 ;;
esac

if [ "$ASSET_OS" = "macOS" ]; then
  ASSET_ARCH="universal"
fi

API_URL="https://api.github.com/repos/$REPO/releases/tags/$VERSION"

ASSET_NAME="cc-switch-cli-${VERSION}-${ASSET_OS}-${ASSET_ARCH}"
ASSET_NAME_PATTERN="cc-switch-cli-.*-${ASSET_OS}-${ASSET_ARCH}$"

DOWNLOAD_URL="$(
  curl -fsSL "$API_URL" |
    grep -E '"browser_download_url":' |
    sed -E 's/.*"browser_download_url": "([^"]+)".*/\1/' |
    grep -E "$ASSET_NAME_PATTERN" |
    head -1
)"

if [ -z "$DOWNLOAD_URL" ]; then
  echo "No helper asset found for $ASSET_OS/$ASSET_ARCH from $API_URL" >&2
  exit 1
fi

curl -fL "$DOWNLOAD_URL" -o "$BIN_DIR/$BIN_NAME"
chmod +x "$BIN_DIR/$BIN_NAME"
"$BIN_DIR/$BIN_NAME" --json status
