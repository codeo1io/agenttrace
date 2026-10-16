---
title: "Envelope-lane tests must exercise parse_raw_session, not the lenient fallback"
date: 2026-10-15
category: quality-gates
module: agenttrace-core
problem_type: process_gap
component: crates/agenttrace-core/src/parser.rs
symptoms:
  - "A red-first contract test for a new envelope-lane capture compiled against lib::parse_jsonl_session and read every captured field back empty"
  - "The empty read looks like a parser bug (capture never fires) rather than a test-dispatch bug (wrong entry point)"
  - "The same fixture parses correctly through the real CLI, so the test and the binary disagree about the same file"
root_cause: wrong_assumption
resolution_type: doc_change
severity: medium
tags:
  - parser
  - testing
  - claude-code
  - dispatch
problem_summary: "Session parsing has two entry points with different behavior: parser::parse_raw_session (via parse_file -> parse_session_bytes) is the real lane dispatcher that selects the claude_code envelope lane, while lib::parse_jsonl_session is the generic lenient fallback. Pointing an envelope-lane feature test at the fallback silently tests the wrong lane."
applies_when:
  - "Writing contract tests for captures that live inside a specific envelope lane (claude_code, codex, opencode, …)"
  - "A new fixture parses through the binary but reads empty in a unit test"
  - "Choosing the entry point for a red-first test that must fail before the capture exists"
---

## Rule

Any feature that lives inside a *specific* parser lane must be tested through the
dispatch surface that selects that lane: `parser::parse_raw_session` (reached via
`parse_file` / `parse_session_bytes`). `lib::parse_jsonl_session` is the generic
lenient fallback used when no structured lane claims the file — an envelope-lane
capture never runs on that path, so a test pointed at it reads the capture as
absent no matter what the fixture contains.

## Case (run 4f2af62e cycle 1, "Claude org-dimension truth")

The org-identity capture (`$.session.identity {orgIdentifier, loginEmail}`) was
implemented inside the claude_code envelope lane. The first red-first test draft
called `lib::parse_jsonl_session` and every `org` read came back empty — which
looked like the capture never firing. Re-dispatching the test through
`parser::parse_raw_session` made the red-green cycle behave as designed. The
landed contract, `crates/agenttrace-core/tests/claude_org_identity_contract.rs`,
is the canonical shape: fixture in `tests/fixtures/claude-org/`, dispatch via
the real parser entry point.

## Corollary: fixture lane keying

A synthetic fixture must actually key the lane it is meant for. The claude_code
envelope lane requires `parentUuid` on line 1; the flat transcript lane keys on
`tool_use` entries. A fixture carrying neither matches no lane and falls through
to the lenient fallback — the same silent-empty symptom from the other end.
When a fixture-driven test reads empty, check the entry point *and* the lane key
before suspecting the capture.
