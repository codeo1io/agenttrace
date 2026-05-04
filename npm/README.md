# agenttrace

Local-first AI coding agent session history for cost, tokens, time, and slow-run diagnosis.

Use it to answer two questions:

- Which coding agent sessions burned the most cost, tokens, and wall-clock time?
- Why did a specific agent task run slowly?

## Install

```bash
npm install -g agenttrace
agenttrace --version
```

Other install methods:

```bash
curl -fsSL https://raw.githubusercontent.com/luoyuctl/agenttrace/master/install.sh | sh
agenttrace --version
```

```bash
brew install luoyuctl/tap/agenttrace
agenttrace --version
```

## Maintainer Checks

From this directory:

```bash
node --check install.js
node --check run.js
AGENTTRACE_BIN=/path/to/agenttrace node run.js --version
AGENTTRACE_RELEASE_TAG=v0.4.0 node install.js
npm pack --dry-run
```

## Usage

```bash
# Open the TUI
agenttrace

# Diagnose local session discovery and cache status
agenttrace --doctor

# JSON overview for automation
agenttrace --overview -f json

# Try built-in sample sessions if this machine has no local agent logs yet
agenttrace --demo

# CI health gate
agenttrace --overview --fail-under-health 80 --fail-on-critical --max-tool-fail-rate 15
```

## Supported Sources

agenttrace auto-detects local sessions from Claude Code, Codex CLI, Gemini CLI, Qwen Code, OpenCode, OpenClaw, Copilot CLI, Oh My Pi, Kimi CLI, Hermes Agent, and Aider chat history.

For Aider repositories, run:

```bash
agenttrace -d /path/to/repo
```

The parser looks for `.aider.chat.history.md`.

## Links

- GitHub: https://github.com/luoyuctl/agenttrace
- Releases: https://github.com/luoyuctl/agenttrace/releases
- CI integration: https://github.com/luoyuctl/agenttrace/blob/master/docs/ci-integration.md
