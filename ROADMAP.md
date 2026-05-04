# agenttrace roadmap

agenttrace is focused on two jobs:

1. Review AI coding agent history across cost, tokens, and elapsed time.
2. Diagnose why an agent task ran slowly.

This roadmap keeps the project pointed at those jobs instead of becoming a generic observability dashboard.

## Now

- Rework the overview around spend, tokens, time, and slow-run triage.
- Improve the large-history loading state so users can see source counts, cache hits, and parsing progress.
- Turn the detail page into a diagnosis-first layout with issue, impact, evidence, next action, and confidence.
- Strengthen diagnostics around slow tools, hanging gaps, retry loops, large params, and context pressure.
- Make diff explain why one run is slower, costlier, or lower quality.

Tracked issues:

- [#142](https://github.com/luoyuctl/agenttrace/issues/142) Rework overview around spend, tokens, time, and slow-run triage
- [#143](https://github.com/luoyuctl/agenttrace/issues/143) Improve large-history loading state and cache progress
- [#145](https://github.com/luoyuctl/agenttrace/issues/145) Turn detail view into a diagnosis-first layout
- [#146](https://github.com/luoyuctl/agenttrace/issues/146) Strengthen diagnostics as a slow-run analyzer
- [#147](https://github.com/luoyuctl/agenttrace/issues/147) Make diff view explain why one run is slower or costlier

## Next

- Add reproducible slow-run fixtures that demonstrate expensive, slow, hanging, and context-heavy sessions.
- Add per-agent guides for Claude Code, Codex CLI, Gemini CLI, Cursor, Aider, OpenCode, and Hermes Agent.
- Improve parser contribution ergonomics with a small scaffold/template and a clearer parser contract.
- Publish more shareable reports and screenshots from redacted local runs.
- Expand install coverage and keep npm, Homebrew, Go install, and shell installers aligned.

## Later

- Add a dedicated "top slow sessions" workflow.
- Add local report comparison across time windows.
- Add optional team-oriented rollups that can be exported without uploading private logs.
- Explore plugin-style parsers if third-party session formats keep changing.

## Non-goals

- Hosted prompt storage.
- Billing-grade invoice reconciliation.
- Replacing agent chat UIs.
- Live tracing while a model is streaming.
