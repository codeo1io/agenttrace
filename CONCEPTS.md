# CONCEPTS

Short definitions of agenttrace's core concepts, one section each, so new
contributors (human or agent) can orient without reading the whole tree.
Keep each section to a few sentences and point to the durable doc where the
full treatment lives.

## Session-derived strings are untrusted at every output boundary

Parsed session files (any harness format) can carry arbitrary bytes in
fields like `model`, session names, and tool labels. Terminal-text sinks
(report lines, overview tables, session-list rows, search output,
statusline) must route every session-derived string through the shared
policy in `crates/agenttrace-core/src/render.rs`
(`sanitize_line_segment`); HTML escapes at its sinks (markup only —
control bytes are a documented negative, see `html_escape`) and JSON
escapes via serde (C0 controls are escaped, but C1 (U+0080–U+009F)
and DEL pass raw — documented negative; do not rely on the JSON path
for terminal safety). The report pane's TUI safety is local to
`terminal_safe_report` (control bytes → U+FFFD); ratatui also drops
zero-width control graphemes, but nothing load-bearing rests on that —
do not bypass `terminal_safe_report` with raw writes. Full treatment:
`docs/solutions/security-issues/2026-09-30-untrusted-session-strings-render-boundary.md`.

## Hostile fixtures must decode to real control bytes

A JSONL fixture carrying `\\u001b` (double backslash) holds literal text,
so a "does it emit ESC?" probe against it passes vacuously. Write fixtures
so serde actually decodes the payload (single-backslash `\u001b`), and
verify by counting raw ESC bytes in the output (`grep -c` on the byte),
not by eyeballing rendered text.

## The go-flag shim hoists known flags past a positional path

`crates/agenttrace-cli/src/main.rs` keeps Go-flag compatibility: unknown
flags and extra positionals after the first path argument are dropped
(tolerance), but KNOWN value-bearing flags (`--format`, `--dir`, `-m`,
…) are hoisted so they still take effect. When adding a flag, register
both its short and long spelling in `flag_takes_value`, and pin the
behavior with a shim test so the guard-order contract (markdown/html bail)
keeps firing.
