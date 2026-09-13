#!/bin/sh
set -eu

# agenttrace — single binary install (Rust + ratatui)
# Usage: curl -sL https://raw.githubusercontent.com/luoyuctl/agenttrace/master/install.sh | sh

REPO="luoyuctl/agenttrace"
BIN="agenttrace"
INSTALL_DIR="${AGENTTRACE_INSTALL_DIR:-}"

# — detect platform —
OS=$(uname -s | tr '[:upper:]' '[:lower:]')
ARCH=$(uname -m)
case "$ARCH" in
  x86_64|amd64)  ARCH="amd64" ;;
  aarch64|arm64) ARCH="arm64" ;;
  armv7l)        ARCH="armv7" ;;
  *)             echo "❌ Unsupported architecture: $ARCH"; exit 1 ;;
esac
case "$OS" in
  linux|darwin)  ;;
  *)             echo "❌ Unsupported OS: $OS"; exit 1 ;;
esac

# — resolve install directory —
if [ -n "$INSTALL_DIR" ]; then
  :
elif [ "$OS" = "darwin" ]; then
  INSTALL_DIR="${HOME}/.local/bin"
elif [ -w /usr/local/bin ]; then
  INSTALL_DIR="/usr/local/bin"
elif [ -w "${HOME}/.local/bin" ] || [ -d "${HOME}/.local/bin" ]; then
  INSTALL_DIR="${HOME}/.local/bin"
else
  INSTALL_DIR="${HOME}/.local/bin"
fi
mkdir -p "$INSTALL_DIR"
DEST="${INSTALL_DIR}/${BIN}"

# — resolve latest release asset —
echo "🔍 Fetching latest release..."
ASSET="${BIN}-${OS}-${ARCH}"
RELEASE_URL="https://github.com/${REPO}/releases/latest/download/${ASSET}"

# — download —
echo "⬇️  Downloading agenttrace (${OS}/${ARCH})..."
TMP=$(mktemp)
if ! curl -fsSL -o "$TMP" "$RELEASE_URL"; then
  rm -f "$TMP"
  echo "❌ No binary found for ${OS}/${ARCH}"
  echo "   Build from source: git clone https://github.com/${REPO}.git && cd agenttrace && cargo build --release -p agenttrace"
  exit 1
fi
chmod 0755 "$TMP"

# — checksum verification (parity with release.yml's .sha256 sidecars) —
# release.yml uploads "${ASSET}.sha256" next to every binary. Older
# releases predate the sidecar, so a missing sidecar warns instead of
# failing; a present sidecar that does not match fails the install
# (pass-11 A11-3, cycle 7).
CHECKSUM_URL="${RELEASE_URL}.sha256"
TMP_SHA=$(mktemp)
if curl -fsSL -o "$TMP_SHA" "$CHECKSUM_URL"; then
  EXPECTED=$(cut -d' ' -f1 "$TMP_SHA" | tr -d '\r\n')
  ACTUAL=""
  if command -v sha256sum >/dev/null 2>&1; then
    ACTUAL=$(sha256sum "$TMP" | cut -d' ' -f1)
  elif command -v shasum >/dev/null 2>&1; then
    ACTUAL=$(shasum -a 256 "$TMP" | cut -d' ' -f1)
  else
    echo "⚠️  No sha256 tool found (sha256sum or shasum); skipping checksum verification."
  fi
  if [ -n "$ACTUAL" ]; then
    if [ "$EXPECTED" != "$ACTUAL" ]; then
      rm -f "$TMP" "$TMP_SHA"
      echo "❌ Checksum mismatch for ${ASSET}."
      echo "   Expected: ${EXPECTED}"
      echo "   Actual:   ${ACTUAL}"
      echo "   The download may be corrupted or tampered with; not installing."
      exit 1
    fi
    echo "   SHA-256 verified."
  fi
else
  echo "⚠️  No checksum sidecar at ${CHECKSUM_URL}; skipping checksum verification."
fi
rm -f "$TMP_SHA"

# — size check —
SIZE=$(wc -c < "$TMP")
echo "   Binary size: ${SIZE} bytes"
if [ "$SIZE" -lt 1000000 ]; then
  rm -f "$TMP"
  echo "❌ Downloaded file is too small to be the agenttrace binary."
  exit 1
fi

# — install —
mv "$TMP" "$DEST"
echo "✅ Installed to ${DEST}"

# — PATH hint —
if ! echo "$PATH" | grep -q "$INSTALL_DIR"; then
  echo ""
  echo "⚠️  ${INSTALL_DIR} is not in your PATH."
  echo "   Add this to your shell profile:"
  echo "     export PATH=\"${INSTALL_DIR}:\$PATH\""
  echo ""
fi

# — quick test —
echo ""
echo "🎉 agenttrace installed! Try:"
echo "   agenttrace --latest"
echo "   agenttrace            # launch TUI"
