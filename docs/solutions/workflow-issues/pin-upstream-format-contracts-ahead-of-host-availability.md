---
title: Pin upstream format contracts ahead of host availability
date: 2026-10-16
category: workflow-issues
module: agenttrace-core claude transcript parser, format-compat maintenance
problem_type: workflow_issue
component: parser
tags: [format-compat, upstream-contracts, hand-built-fixtures, disclosure-counters, forward-compatibility, rm-1522]
applies_when: "An upstream dependency ships a transcript/ledger format change you cannot yet capture locally (host runs an older version), or a parser lane's kind-matching grew by accretion and now silently drops unknown shapes"
---

# Pin upstream format contracts ahead of host availability

## Context

Upstream Claude Code 2.1.300 (CHANGELOG line 66, 2026-10-15) changed the
local session transcript format to also record each API request's system
prompt, tools, and parameters. The host's installed claude-code was
2.0.76 — **no live capture of the new format was possible**. Meanwhile
the claude lane's `parse_claude_code_jsonl` armed only
`"user"`/`"assistant"` behind a bare `_ => {}` fall-through, so the new
per-request records *and* long-known kinds (`summary`, `system`, …) were
silently uncounted — while the sibling qwen and codex lanes already
carried unmatched-kind disclosure counters. Discovered as research
candidate C2 of run `7fd83ab2` cycle 2 (2026-10-15) and landed the same
cycle (implement `87c78056`, red-first 6 FAILED → 7/7).

## Guidance

1. **Pin the published contract, not a wish.** When the changelog or
   spec wording is the only source, hand-build the vendored fixture
   from that wording and make provenance explicit: the fixture is
   *hand-built from the upstream contract*, not a capture. Pin its
   known lines byte-for-byte against an inline control corpus so a
   future edit that silently rewrites the fixture fails loudly
   (`tests/fixtures/claude-code-2-1-300/request-reproduction.jsonl` +
   `vendored_fixture_known_lines_match_the_inline_control_corpus`).
2. **Make the deliverable shape-agnostic.** The durable fix is not a
   parser arm for one new record shape — 2.1.301 can change it again.
   It is a disclosure counter: `claude_unmatched_type:{kind}` (mirroring
   the qwen/codex bars, `parser.rs` unmatched-kinds block) so every wire
   kind the lane does not arm is *visible in `--doctor`, forever*, at
   one counter increment per record.
3. **Gate disclosure carriers on lane content.** Disclosure meta-events
   ride the lane's carrier only when the lane actually mined content
   (`!events.is_empty()`). The ungated carrier was this cycle's live
   regression: it claimed contentless claude-detected journals for the
   claude lane and stole them from the generic fallback lane
   (`discovery_contract`:
   `rust_generic_jsonl_accepts_legacy_type_events_without_role` FAILED,
   then pinned green).
4. **Keep counters bounded by construction.** Count kinds; never retain
   the per-record content (a 2,000-record duplicate corpus pins that
   event count stays independent of record count).
5. **Schedule the re-capture.** Record the open arm in the roadmap: when
   the host upgrades past 2.0.76, capture live and diff against the
   hand-built fixture — the fixture's value is proven the day it
   matches, and its errors surface the same day.

## Why This Matters

Transcript formats change on upstream's cadence, not yours. A lane that
parses silently either breaks loudly (best case) or drops records
silently (the actual case here — for years, `summary` and `system` were
uncounted and nobody knew). The counter converts the lane's ignorance
into a durable, user-visible fact at zero per-record cost, and the
hand-built fixture keeps the next format change a *diff* instead of a
surprise. Both are cheap precisely because they demand nothing of the
host: no version bump, no live capture, no one-shape guess.

## When to Apply

- An upstream changelog/release note describes an input-format change
  you cannot reproduce locally yet.
- A parser's match arms accreted over time (grep for bare `_ => {}`
  fall-throughs) — retrofit the counter before the next unknown shape
  ships.
- A fixture would otherwise be "representative guesswork": provenance
  labeling (hand-built vs captured) is what keeps it trustworthy.

## Examples

Before — new and known kinds vanished behind a bare arm:

```rust
// parse_claude_code_jsonl (pre rm-1522): only two kinds armed
match kind {
    "user" => { /* … */ }
    "assistant" => { /* … */ }
    _ => {}                       // summary, system, request_reproduction, … silently dropped
}
```

After — disclosure by type on a content-gated carrier (abridged from
`crates/agenttrace-core/src/parser.rs`):

```rust
let mut unmatched_kinds: BTreeMap<String, i64> = BTreeMap::new();
// … in the fall-through arm:
let key = disclosure_key("claude_unmatched_type", shape);
*unmatched_kinds.entry(key).or_insert(0) += 1;   // count, never retain content
// … at fold time — only when the lane mined content:
if !events.is_empty() {
    for (key, count) in unmatched_kinds { /* one meta event, doctor-visible */ }
}
```

`--doctor` on the fixture then discloses
`claude_unmatched_type:request_reproduction=3` end to end.

## Related

- rm-1522 row and the cycle-2 compound banner in `ROADMAP.md`; CHANGELOG
  §Unreleased "rm-1522" bullet for the user-facing record.
- Sibling disclosure bars: `parser.rs` qwen/codex unmatched-kind
  counters (the asymmetry that flagged this lane).
- `docs/solutions/security-issues/a-first-char-formula-guard-is-dead-behind-a-label-join.md`
  — the same cycle's other lesson; the two compose (the fixture's
  hostile payloads ride the paste-safe search lane).
