---
title: Terminal injection through journal-derived statusline report strings
date: 2026-09-30
category: security-issues
module: agenttrace-core statusline journal and report renderer
problem_type: security_issue
component: frontend
symptoms:
  - "agenttrace --statusline-report printed journal-derived session_id and miss-cause strings verbatim, so an ESC byte in a session id reached the terminal as a live SGR sequence instead of inert text"
  - "An OSC-52 sequence inside a miss-cause key could write the operator's clipboard when the report was viewed, observed as raw escape bytes in a cat -v of the report output"
  - "The one-line statusline rendering path already neutralized control characters for the same payload class, but the report path shared none of that defense"
root_cause: missing_output_sanitization
resolution_type: code_fix
severity: high
tags: [terminal-injection, osc-52, ansi-escape, sanitize-on-render, statusline-journal]
---

# Terminal injection through journal-derived statusline report strings

## Problem

The statusline capture journal stores whatever JSON payload the statusline hook emitted, verbatim. The report path (`--statusline-report`) read that journal and printed payload-derived strings — `session_id`, the joined `last_miss_causes`, and the per-cause breakdown keys — directly into the terminal. Any control characters embedded in those strings (ANSI SGR, OSC sequences) therefore executed in the operator's terminal when the report was viewed, including OSC-52 clipboard-write sequences. The one-line rendering path for the same payloads had always sanitized; the report path was a second, unsanitized consumer of the same untrusted data.

## Symptoms

- A crafted journal entry with `session_id` containing `ESC [31m` rendered as colored text in the report instead of visible junk (live campaign repro, 2026-09-30, piped through `cat -v`).
- A miss-cause key carrying `ESC ]52;c;<base64> BEL` appeared as a raw OSC-52 clipboard-write attempt in report output.
- No test failed: the existing sanitization tests covered only the one-line render, so the report path's raw prints were invisible to the suite.

## What Didn't Work

- **Hand-writing a journal line containing raw ESC bytes.** The journal is JSONL; a literal control byte makes the line invalid JSON and the reader rejects it. The realistic vector is a JSON-escaped `\u001b`, which decodes to a real ESC at parse time — so fixtures must be escaped, and any live repro must inject through a JSON writer rather than a text editor.
- **Asserting the full sanitized output equals one expected string.** The sanitizer replaces control characters with U+FFFD but leaves printable tails alone, so `ESC [31mX` legitimately becomes `U+FFFD [31mX` — the `[31m` text is visible but inert without its ESC. An equality assertion against a hand-picked "clean" string fails on those tails and encodes the wrong contract. Assert *no control bytes*, not *specific characters*.
- **Sanitizing at persistence.** Rejected by design: the journal is the fidelity record, the JSON report format is consumed programmatically, and both need the bytes as received. Sanitizing `append_statusline_capture` would corrupt every non-terminal consumer to fix one terminal consumer.

## Solution

Sanitize at the print boundary, reusing the helper the one-line path already uses. `render_statusline_report_text` now wraps each journal-derived string in `sanitize_line_segment` (crates/agenttrace-core/src/statusline.rs:265), which maps every `char::is_control()` character to U+FFFD:

- session_id at crates/agenttrace-core/src/statusline.rs:619
- joined `last_miss_causes` at crates/agenttrace-core/src/statusline.rs:632
- miss-cause keys at crates/agenttrace-core/src/statusline.rs:645

Nothing else moved. `append_statusline_capture` (crates/agenttrace-core/src/statusline.rs:275) still persists the payload verbatim, and the JSON report format still carries raw escape sequences as `\u001b` — that is the fidelity contract, and the regression test asserts it explicitly.

The regression test `statusline_report_sanitizes_journal_derived_strings` (crates/agenttrace-core/src/statusline.rs:909) drives the public path end to end: it points `AGENTTRACE_SESSION_CACHE_DIR` (honored at crates/agenttrace-core/src/statusline.rs:107) at a temp root under the shared env lock (crates/agenttrace-core/src/lib.rs:1607), appends a hostile payload through the public `append_statusline_capture`, renders the report, and asserts (a) no control bytes anywhere in the text report, (b) no raw ESC in particular, and (c) the JSON format still contains the raw `\u001b` escape, proving persistence fidelity did not regress.

## Why This Works

Control characters are exactly the terminal-instruction carriers — an SGR or OSC sequence is inert text the moment its ESC is replaced. Replacing every control character (not just ESC) also neutralizes BEL, NUL-in-string tricks, and CRLF line-structure attacks in one rule, while printable data stays readable. Sanitizing at the print boundary is what makes the fix complete and minimal at once: the terminal is the only consumer that interprets control bytes, so the terminal-facing renders (`render_status_line` at crates/agenttrace-core/src/statusline.rs:187, and now the report text path) each sanitize, while every other consumer keeps raw fidelity.

## Prevention

- **Rule:** any code that renders journal-derived strings into terminal output routes the string through `sanitize_line_segment` at the print site. Numeric-only consumers (counts, percentages) are exempt. The one-line path's tests (`hostile_payload_names_render_without_control_characters`, crates/agenttrace-core/src/statusline.rs:873) and the report path's test now both lock the rule for their respective renderers; a new render surface needs its own test in the same shape.
- **Test shape:** point `AGENTTRACE_SESSION_CACHE_DIR` at a temp root under `crate::test_env::lock_env()`, append the hostile payload through the public append function, assert through the public render function — never by calling the sanitizer directly (that tests the helper, not the path). Assert "no control bytes", not equality with an expected clean string.
- **Live verification:** `agenttrace --statusline-report | cat -v` shows `M-oM-?M-=` (U+FFFD) where escapes were; `grep -c` for a raw ESC byte returns 0. The JSON format still shows `\u001b`, and that difference is the proof that render-time sanitization did not leak into storage.
- **Discrimination check:** when adding such a test, temporarily revert only the production sanitization hunk and confirm the test fails on the pre-fix code — a sanitization test that passes on the unfixed path proves nothing.
