# Parser Guide

agenttrace supports multiple AI coding-agent session formats by normalizing them into `Event` records in `agenttrace-core`. The rest of the system works from that normalized shape.

## What a Parser Should Extract

Extract as much as the source format safely provides:

- `Role`: user, assistant, tool, meta, or session_meta
- `Timestamp`: original event timestamp when available
- `Content`: message or tool result text
- `Reasoning`: thinking/reasoning text when available
- `ToolCalls`: tool call id, name, and arguments
- `ToolCallID`: id linking a tool result back to a tool call
- `IsError`: whether a tool result failed
- `Usage`: token usage maps such as input_tokens and output_tokens
- `ModelUsed`: model name for pricing lookup
- `SourceTool`: stable source identifier, such as `claude_code`

Partial extraction is fine. Incorrect extraction is worse than missing data.

## Add Format Detection

Start in `crates/agenttrace-core/src/parser.rs`:

- update `parse_raw_session`
- add a stable `SourceTool` id
- add display-name/report handling in `crates/agenttrace-core/src/reports.rs` when needed
- route the format before the generic JSON/JSONL fallbacks

Detection should be conservative. If the file could be a generic JSON/JSONL session, avoid claiming it unless there is a clear field or shape unique to the tool.

## Add Tests

Use small synthetic fixtures under `testdata/` or focused Rust tests in `crates/agenttrace-core/tests/`. Reusable synthetic provider fixtures live under `testdata/generated/` and are covered by Rust integration tests.

Keep source capability honest: raw event formats may produce Detailed sessions;
aggregate SQLite rows must not gain invented timestamps, tool spans, or content.

Cover:

- detection
- successful parse
- malformed or partial input
- at least one representative tool call if the format supports tools
- token/model extraction if the format has usage data

Keep fixtures tiny. A few records are better than a full private session.

## Validate the End-to-End Flow

After parser tests pass, run:

```bash
cargo test
cargo build --release -p agenttrace
target/release/agenttrace --doctor
target/release/agenttrace --demo --overview -f json
```

If you add a new default session directory, make sure `agenttrace --doctor` reports it clearly.

For local-first tools like Aider, the default history may live in the current repository (`.aider.chat.history.md`) rather than a global session directory. In that case, support explicit `-d <repo>` and only add auto-discovery when a clear marker file exists.

For SQLite-backed tools like Cursor, prefer a documented JSON export path unless direct database support is worth the dependency and platform cost.

## Numeric Bounding (cap-once semantics)

Token counts are untrusted input: a corrupt or hostile journal can carry `1e300`, and the parser clamps every such value into `i64` with a saturating cast (`1e300` → `i64::MAX`) before it ever reaches a struct. Every layer that *aggregates* those counts must saturate too — a plain `i64 +=` on two clamped values panics the debug binary (`attempt to add with overflow`) and silently wraps in release. The parser's `add_usage` and the report layer's overview accumulations (e.g. `compute_overview`'s per-task-type totals, rm-541) therefore accumulate with `saturating_add`: a row saturates at `i64::MAX` **once** and stays there no matter how many more clamped sessions arrive, so downstream rates and shares always divide by finite non-negative totals. Costs are `f64` dollars, which cannot overflow-panic. When you add a new aggregation site, use `saturating_add` for token counts and add a fixture with two `i64::MAX` sessions to the report-layer contract tests (`tests/report_numeric_truthfulness.rs`).

## Privacy Notes

Do not commit real agent logs unless they are fully synthetic or carefully redacted. Session logs often include prompts, source code, file paths, tool arguments, and secrets.

When opening a parser request, use the Parser request issue template and include only the smallest redacted sample that preserves the format shape.
