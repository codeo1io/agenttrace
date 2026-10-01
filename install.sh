#!/bin/sh
set -eu

# agenttrace — single binary install (Rust + ratatui)
# Usage: curl -sL https://raw.githubusercontent.com/luoyuctl/agenttrace/master/install.sh | sh

REPO="luoyuctl/agenttrace"
# Pinned ref for the build-from-source fallback (rm-051): the fallback
# must never clone a moving default branch — whatever master holds at
# clone time is not what this installer was tested against. Keep in
# lockstep with the newest published release tag; override for forks or
# testing with AGENTTRACE_SOURCE_REF (tag or branch name).
REF="${AGENTTRACE_SOURCE_REF:-v0.9.0}"
# Install receipt for the source-build fallback (rm-051): records the
# exact ref that was built, so an install can be audited later.
INSTALL_RECEIPT="${AGENTTRACE_INSTALL_RECEIPT:-${XDG_DATA_HOME:-$HOME/.local/share}/agenttrace/install-receipt}"
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
# AGENTTRACE_DOWNLOAD_URL overrides the release URL (used by offline CI tests).
RELEASE_URL="${AGENTTRACE_DOWNLOAD_URL:-https://github.com/${REPO}/releases/latest/download/${ASSET}}"

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

# — runtime verification —
# A downloaded artifact must be proven to run on THIS host before it is
# installed. Release binaries built on a newer distro (e.g. glibc 2.39 on
# ubuntu-latest) fail here on older hosts instead of installing silently
# broken (Maestro finding 47fa1154: `agenttrace --help` → GLIBC_2.39 error).
VERIFY_LOG="$TMP.verify"
if ! "$TMP" --version >"$VERIFY_LOG" 2>&1; then
  echo "❌ Downloaded agenttrace binary does not run on this host:"
  head -n 3 "$VERIFY_LOG"
  echo "   Usually the release binary was built against a newer libc (GLIBC) than"
  echo "   this host provides, or the architecture does not match."
  rm -f "$TMP" "$VERIFY_LOG"
  if [ "${AGENTTRACE_SKIP_SOURCE_BUILD:-0}" = "1" ]; then
    echo "❌ Install aborted (AGENTTRACE_SKIP_SOURCE_BUILD=1)."
    exit 1
  fi
  if command -v git >/dev/null 2>&1 && command -v cargo >/dev/null 2>&1; then
    echo "🔧 Falling back to building from source with this host's toolchain..."
    echo "   Cloning $REPO at pinned ref $REF (set AGENTTRACE_SOURCE_REF to override)."
    SRC=$(mktemp -d)
    if git clone --depth 1 --branch "$REF" "https://github.com/${REPO}.git" "$SRC" \
       && (cd "$SRC" && cargo build --release -p agenttrace) \
       && cp "$SRC/target/release/agenttrace" "$TMP" && chmod +x "$TMP"; then
      rm -rf "$SRC"
      RECEIPT_DIR=$(dirname "$INSTALL_RECEIPT")
      mkdir -p "$RECEIPT_DIR"
      {
        echo "installed_at=$(date -u '+%Y-%m-%dT%H:%M:%SZ')"
        echo "source=source-build"
        echo "repo=$REPO"
        echo "ref=$REF"
      } >"$INSTALL_RECEIPT"
      echo "   Built from source successfully (pinned ref $REF)."
      echo "   Receipt: $INSTALL_RECEIPT"
    else
      rm -rf "$SRC"
      rm -f "$TMP"
      echo "❌ Source build failed (pinned ref '$REF'; set AGENTTRACE_SOURCE_REF to a published tag or branch to override)."
      echo "   Install Rust (https://rustup.rs) and retry, or:"
      echo "   git clone --branch $REF https://github.com/${REPO}.git && cd agenttrace && cargo build --release -p agenttrace"
      exit 1
    fi
  else
    echo "❌ No compatible prebuilt binary and no Rust toolchain found."
    echo "   Install Rust (https://rustup.rs), then:"
    echo "   git clone https://github.com/${REPO}.git && cd agenttrace && cargo build --release -p agenttrace"
    exit 1
  fi
fi
rm -f "$VERIFY_LOG"

# — install —
mv "$TMP" "$DEST"

# — post-install confirmation —
# Runs the installed binary once at its final location: a noexec install
# directory or a last-mile incompatibility surfaces here, visibly.
if ! "$DEST" --version >"$VERIFY_LOG" 2>&1; then
  echo "❌ Installed binary at ${DEST} does not run on this host:"
  head -n 3 "$VERIFY_LOG"
  rm -f "$DEST" "$VERIFY_LOG"
  exit 1
fi
echo "   $(head -n 1 "$VERIFY_LOG")"
rm -f "$VERIFY_LOG"
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
