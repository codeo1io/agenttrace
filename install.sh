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
# AGENTTRACE_DOWNLOAD_URL overrides the release URL (used by offline CI tests).
RELEASE_URL="${AGENTTRACE_DOWNLOAD_URL:-https://github.com/${REPO}/releases/latest/download/${ASSET}}"
# AGENTTRACE_SOURCE_URL overrides the source-build clone/fetch URL (offline CI tests).
SRC_URL="${AGENTTRACE_SOURCE_URL:-https://github.com/${REPO}.git}"

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
# release.yml uploads "${ASSET}.sha256" next to every binary. A missing
# sidecar, like a mismatched one, means the bytes about to be installed
# cannot be verified: absence from a fetched URL is itself a red flag,
# not a condition to tolerate (rm-029; install.ps1 and npm refuse the
# same way -- see lib.rs all_three_installers_refuse_…).
CHECKSUM_URL="${RELEASE_URL}.sha256"
TMP_SHA=$(mktemp)
if ! curl -fsSL -o "$TMP_SHA" "$CHECKSUM_URL"; then
  rm -f "$TMP" "$TMP_SHA"
  echo "❌ No checksum sidecar at ${CHECKSUM_URL}."
  echo "   The release is incomplete or the download URL was tampered with; not installing."
  exit 1
fi
EXPECTED=$(cut -d' ' -f1 "$TMP_SHA" | tr -d '\r\n')
ACTUAL=""
if command -v sha256sum >/dev/null 2>&1; then
  ACTUAL=$(sha256sum "$TMP" | cut -d' ' -f1)
elif command -v shasum >/dev/null 2>&1; then
  ACTUAL=$(shasum -a 256 "$TMP" | cut -d' ' -f1)
else
  rm -f "$TMP" "$TMP_SHA"
  echo "❌ No sha256 tool found (sha256sum or shasum); cannot verify the download, not installing."
  exit 1
fi
if [ "$EXPECTED" != "$ACTUAL" ]; then
  rm -f "$TMP" "$TMP_SHA"
  echo "❌ Checksum mismatch for ${ASSET}."
  echo "   Expected: ${EXPECTED}"
  echo "   Actual:   ${ACTUAL}"
  echo "   The download may be corrupted or tampered with; not installing."
  exit 1
fi
echo "   SHA-256 verified."
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
    # rm-030: the fallback builds a PINNED, verifiable ref -- never a
    # floating master tip. AGENTTRACE_SOURCE_REF overrides the pin for
    # CI (a release tag or a full 40-char commit id); otherwise the
    # release tag matching the artifact that just failed to run is
    # resolved and the built commit is recorded -- and checked against
    # the pin -- so the install states exactly what it placed.
    FALLBACK_REF="${AGENTTRACE_SOURCE_REF:-}"
    if [ -z "$FALLBACK_REF" ]; then
      FALLBACK_REF=$(curl -fsSL "https://api.github.com/repos/${REPO}/releases/latest" | sed -n 's/.*"tag_name": *"\([^"]*\)".*/\1/p' | head -n 1) || FALLBACK_REF=""
    fi
    if [ -z "$FALLBACK_REF" ]; then
      rm -rf "${SRC:-}" 2>/dev/null || true
      rm -f "$TMP" "$VERIFY_LOG"
      echo "❌ Cannot resolve a pinned release tag for the source build; refusing to build an unverified master tip."
      echo "   Set AGENTTRACE_SOURCE_REF=<tag-or-sha> to pin the build explicitly."
      exit 1
    fi
    SRC=$(mktemp -d)
    # rm-030 (review F3): a full commit id cannot be resolved by
    # `clone --branch` (it takes ref NAMES only; proven live on git
    # 2.34.1: "Remote branch <sha> not found in upstream origin"), so a
    # SHA pin fetches the commit directly -- and the fetched HEAD is
    # checked against the pin so the install verifies, not just
    # records, exactly what it built.
    PINNED_SHA=""
    CLONE_OK=0
    if printf '%s' "$FALLBACK_REF" | grep -qE '^[0-9a-fA-F]{40}$'; then
      PINNED_SHA=$(printf '%s' "$FALLBACK_REF" | tr '[:upper:]' '[:lower:]')
      if git init -q "$SRC" \
         && (cd "$SRC" && git remote add origin "$SRC_URL") \
         && (cd "$SRC" && git fetch -q --depth 1 origin "$PINNED_SHA") \
         && (cd "$SRC" && git checkout -q --detach FETCH_HEAD); then
        CLONE_OK=1
      fi
    elif git clone -q --depth 1 --branch "$FALLBACK_REF" "$SRC_URL" "$SRC"; then
      CLONE_OK=1
    fi
    SRC_SHA=""
    if [ "$CLONE_OK" = 1 ]; then
      SRC_SHA=$(cd "$SRC" && git rev-parse HEAD 2>/dev/null) || SRC_SHA=""
    fi
    if [ "$CLONE_OK" != 1 ] || { [ -n "$PINNED_SHA" ] && [ "$PINNED_SHA" != "$SRC_SHA" ]; }; then
      rm -rf "$SRC"
      rm -f "$TMP" "$VERIFY_LOG"
      if [ -n "$PINNED_SHA" ] && [ "$CLONE_OK" = 1 ]; then
        echo "❌ Fetched commit ${SRC_SHA} does not match the pinned ref ${PINNED_SHA}; refusing."
      else
        echo "❌ Cannot fetch pinned ref ${FALLBACK_REF} for the source build; refusing to build an unverified master tip."
        echo "   Set AGENTTRACE_SOURCE_REF=<tag-or-full-commit-sha> to pin the build explicitly."
      fi
      exit 1
    fi
    if (cd "$SRC" && cargo build --release -p agenttrace) \
       && cp "$SRC/target/release/agenttrace" "$TMP" && chmod +x "$TMP"; then
      rm -rf "$SRC"
      echo "   Built from source successfully."
      if [ -n "$SRC_SHA" ]; then
        echo "   Built from ${FALLBACK_REF} (commit ${SRC_SHA})"
      fi
      if [ -n "$PINNED_SHA" ]; then
        echo "   Pin verified: fetched commit matches ${PINNED_SHA}"
      fi
    else
      rm -rf "$SRC"
      rm -f "$TMP" "$VERIFY_LOG"
      echo "❌ Source build failed. Install Rust (https://rustup.rs) and retry, or:"
      echo "   git clone https://github.com/${REPO}.git && cd agenttrace && cargo build --release -p agenttrace"
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
