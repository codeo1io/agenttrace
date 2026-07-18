# AgentTrace Desktop Plan

## Goal

Build a local-first desktop app for people who use AI coding assistants but do not want to learn observability terminology.

The desktop app should answer three questions:

1. Did my AI sessions finish smoothly?
2. Which sessions need attention?
3. What can I do differently next time?

The current Rust core remains the source of truth. The desktop app is a presentation layer, not a second analysis implementation.

## Product Scope

The first release has five surfaces:

| Surface | User question | Existing capability reused |
| --- | --- | --- |
| Welcome | What data can AgentTrace find? | discovery and known session directories |
| Home | How did my assistants perform recently? | overview, data health, cost, health, anomalies |
| Sessions | What happened in each task? | list, filters, search, detail |
| Discover | What should I improve? | diagnostics, cost alerts, fix suggestions |
| Compare | Was this attempt better than the previous one? | pairwise session diff |

Keep advanced evidence available behind disclosure controls. Do not make tokens, P95 latency, loop fingerprints, or raw event fields the primary language.

## Architecture

Use Tauri 2 with a React and TypeScript frontend.

```text
React UI
   | Tauri commands and typed DTOs
agenttrace-desktop
   | direct Rust calls
agenttrace-core
   | local files, SQLite sources, cache
AI assistant session history
```

Rules:

- Reuse `agenttrace-core` for discovery, parsing, metrics, diagnostics, pricing, and comparison.
- Keep desktop-specific DTOs and user-facing copy in the desktop crate.
- Do not invoke the CLI or scrape text reports.
- Do not add a local HTTP server; Tauri commands are sufficient.
- Do not upload session content. Derived history remains opt-in.
- Keep the existing CLI and TUI behavior unchanged.

## Desktop API

Start with the smallest read-only command surface:

```text
detect_sources()                  -> detected sources and privacy state
load_home(range)                 -> summary, trend, attention items, recent sessions
list_sessions(filters, cursor)   -> paginated session summaries
get_session(session_id)          -> plain-language detail plus advanced evidence
list_findings(filters)           -> prioritized findings and suggestions
compare_sessions(left, right)    -> outcome and metric deltas
```

DTOs should use stable identifiers and already-formatted display fields where platform locale matters. Raw `Session` remains internal until the UI contract is proven.

## Delivery Plan

### Phase 1: App shell and real data

- Add one Tauri workspace member and one React frontend.
- Implement the four-item sidebar: Home, Sessions, Discover, Compare.
- Build first-run source detection and local-only privacy messaging.
- Connect Home and Sessions to real `agenttrace-core` results.
- Add loading, empty, partial-data, and parse-failure states.

Verify:

- The app opens against restored `testdata/` and real local sources.
- Home totals match the CLI overview for the same filters.
- Session selection opens the same session that the TUI reports.

### Phase 2: Plain-language diagnostics

- Map diagnostics to `what happened -> impact -> recommendation -> evidence`.
- Add Discover prioritization and the positive `Doing well` section.
- Add session detail with a simple timeline and optional technical details.
- Add current-versus-previous comparison.

Verify:

- Every visible finding links to local evidence.
- Cost, duration, failure, and loop deltas match core values.
- Technical terms are hidden by default but remain inspectable.

### Phase 3: Desktop quality

- Add keyboard navigation, focus states, VoiceOver labels, reduced-motion support, and 200% text scaling checks.
- Add system light/dark appearance and English/Chinese localization.
- Persist window state and non-sensitive UI preferences.
- Add macOS signing/notarization and Windows/Linux bundles.

Verify:

- Automated component tests cover empty, healthy, warning, and critical states.
- Tauri integration tests cover the six commands with fixture data.
- Screenshot tests cover the five reference surfaces at a fixed viewport.
- Release smoke tests install and launch packaged artifacts on each target OS.

## MVP Acceptance Criteria

- A new user reaches useful results without configuring a path.
- The first screen explains that analysis stays on the device.
- The primary navigation contains no more than four product destinations.
- Home shows completion, spend, and attention items before technical metrics.
- A finding explains its impact and offers a next step.
- Compare states the outcome in one sentence before showing numbers.
- All displayed metrics are traceable to `agenttrace-core` output.
- No network request is required to analyze local sessions.

## Explicitly Deferred

- Cloud sync and accounts
- Team dashboards
- Live session interception
- Editing agent configuration automatically
- Plugin marketplace
- Custom dashboard builders

Add these only after the read-only local desktop workflow is validated with users.

## Design Baseline

See [desktop-design-reference.md](desktop-design-reference.md) for the navigation, copy rules, Apple-style references, and generated 4K concept images.
