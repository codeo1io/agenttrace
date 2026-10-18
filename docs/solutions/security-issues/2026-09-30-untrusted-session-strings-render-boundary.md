---
title: "Sanitize untrusted session strings at one render boundary — not per sink"
date: "2026-09-30"
category: "security-issues"
module: "agenttrace-core render policy (reports, statusline, TUI output)"
problem_type: "best_practice"
component: "frontend"
severity: "high"
applies_when:
  - "adding any output path that interpolates session-derived strings (model, name, agent label)"
  - "rendering report, statusline, or TUI text from parsed session files"
  - "writing hostile-payload regression tests for terminal output"
symptoms:
  - "OSC-52 clipboard-write and SGR sequences stored in a session model string execute verbatim in the user's terminal"
  - "a fixed sink (statusline) stays safe while sibling sinks (report text) still emit raw ESC bytes"
  - "hostile-fixture tests pass while the built binary still emits raw escape bytes"
root_cause: "missing_validation"
resolution_type: "code_fix"
tags:
  - terminal-injection
  - osc-52
  - sanitize-boundary
  - untrusted-input
  - render-policy
  - hostile-fixtures
---

## Problem

agenttrace parses session files it does not control — a cloned repo, a shared
workspace, or a compromised agent writing its own journal can set any string
the parser stores (session `model`, session name, agent label). During the
2026-09-30 cycle-1 assessment a crafted session with
`model = "EVIL\u001b]52;c;aGVsbG8=\u0007\u001b[31mRED"` made `agenttrace
--overview` and the single-session report emit the OSC-52 clipboard-write
sequence and an SGR color reset verbatim — a live clipboard-injection path on
terminals with clipboard writes enabled (kitty, iTerm2, alacritty, wezterm).

The structural failure was not the single sink: a prior fix (rm-012) had
sanitized the statusline path only, and it worked — but every sibling sink
(reports text/markdown, the TUI's fallback transliteration) was left raw, and
the TUI's actual safety was an undocumented load-bearing behavior (ratatui
drops zero-width control graphemes). Per-sink fixes rot because new sinks are
added without an inventory.

## Root cause

No single render-safety boundary and no sink inventory: sanitization was an
ad-hoc, per-call-site property instead of a policy every terminal-bound string
must pass through.

## Resolution

The rm-034 batch (validated 256 passed / 0 failed
by the cycle's targeted, full, and post-review-fix gates) established the boundary:

- `crates/agenttrace-core/src/render.rs` — one policy module owning
  `sanitize_line_segment` (hoisted from statusline.rs behavior-preserving:
  ESC, BEL and other C0/C1 controls become U+FFFD; the rm-012 hostile
  regression test stayed green unchanged through the move).
- `crates/agenttrace-core/src/reports.rs` — every session-derived text and
  markdown sink now routes through it: the session-report model line, the
  overview By Model and By Agent loops, `text_cell`, `markdown_cell`
  (newline→`<br>` applied before sanitizing so multi-line cells keep their
  semantics), `markdown_inline_code`, the tool-display-name fall-through, and
  the sessions-table name truncation.
- `crates/agenttrace-tui/src/filters.rs` — `terminal_safe_report` gained an
  explicit control-character arm (previously `_ => ch`, a false-safe
  transliteration that passed control bytes through while looking like
  terminal-safety code).
- `crates/agenttrace-cli/src/main.rs` (`render_session_list`) and
  `crates/agenttrace-core/src/search.rs` (`report_search_text`) — two more
  sibling sinks (`--sessions`, `--search`) that the first boundary pass
  missed and the cycle's adversarial review caught live (raw OSC-52 + SGR
  through `--sessions`, plus a hostile directory name reaching the `path:`
  line of `--search`); both route through `sanitize_line_segment` and both
  carry payload tests.
- Negatives preserved deliberately: HTML output keeps escaping at the sink
  (`html_escape`) and JSON output keeps serde's escaping — serde escapes
  C0 controls (`\u001b`, `\u0007`) but C1 (U+0080–U+009F) and DEL pass
  raw, so the JSON path is itself a documented negative for C1 — the
  boundary applies to terminal-bound strings only.

Byte-level verification after the fix: the same crafted session produces zero
raw ESC bytes in both report paths (baseline: two), with the payload rendered
inert as U+FFFD.

## Prevention

1. **One boundary, not per-sink fixes.** Any new code path that prints,
   pushes, or formats a session-derived string for a terminal must route
   through `sanitize_line_segment` in `crates/agenttrace-core/src/render.rs`.
   A new sink without that call is a regression, even if every existing test
   passes.
2. **The hostile corpus is the sink inventory.** The render battery in
   `crates/agenttrace-core/src/lib.rs` exercises one hostile payload per sink
   class; when you add a sink, add its payload test in the same change.
   Enumerate sinks from a sweep (grep for terminal writers — `println!`,
   `format!`, `push_str` into output strings), not from memory of the files
   you touched: this cycle's first boundary pass felt complete, and its own
   review still found two live sibling sinks (`--sessions`, `--search`) by
   sweeping. A boundary you believe you have finished is not an inventory.
3. **Build hostile fixtures with single-backslash JSON escapes.** A fixture
   line containing `\\u001b` stores the six characters `\u001b` — serde never
   decodes an ESC byte, so an escape-count probe against it passes trivially
   and proves nothing. This cycle's first verification fell into exactly that
   trap and reported a false all-clear; the fixture must contain `\u001b`
   (single backslash) so the decoded record carries a real ESC byte.
4. **Verify with byte counting, not eyeballs.** `grep -c` for the ESC byte
   over the command's captured output (or `cat -v`) — terminal-emulator
   rendering hides control bytes from visual inspection in both directions:
   it can execute what you cannot see and display what was not emitted.
