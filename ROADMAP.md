# agenttrace roadmap

agenttrace is focused on two jobs:

1. Review AI coding agent history across cost, tokens, and elapsed time.
2. Diagnose why an agent task ran slowly.

This roadmap keeps the project pointed at those jobs instead of becoming a generic observability dashboard.

## Now

- Rework overview, detail, diagnostics, and diff around spend, tokens, time, health, and slow-run evidence. Tracked by [#142](https://github.com/luoyuctl/agenttrace/issues/142), [#145](https://github.com/luoyuctl/agenttrace/issues/145), [#146](https://github.com/luoyuctl/agenttrace/issues/146), and [#147](https://github.com/luoyuctl/agenttrace/issues/147).
- Improve large-history loading state with source counts, cache hits, and parsing progress. Tracked by [#143](https://github.com/luoyuctl/agenttrace/issues/143).

## Next

- Add reproducible slow-run fixtures that demonstrate expensive, slow, hanging, and context-heavy sessions.
- Add per-agent guides for Claude Code, Codex CLI, Gemini CLI, Cursor, Aider, OpenCode, and Hermes Agent.
- Expand install coverage and keep npm, Homebrew, Go install, and shell installers aligned.

## Later

- Add a dedicated "top slow sessions" workflow.
- Add local report comparison across time windows.

## Non-goals

Non-goals: hosted prompt storage, billing-grade invoice reconciliation, replacing agent chat UIs, and live tracing while a model is streaming.
