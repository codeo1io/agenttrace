---
title: "Validate hand-crafted probe corpora before trusting a failure signal"
date: 2026-09-30
category: workflow-issues
module: "agenttrace-core diagnostics verification workflow"
problem_type: workflow_issue
component: testing_framework
severity: medium
applies_when:
  - "Verifying a parser or diagnostics fix by running the CLI over a hand-written transcript corpus instead of an in-repo fixture"
  - "A probe corpus is typed by hand (timestamps, tool_use ids, fields) rather than serialized by the code path that produces real transcripts"
  - "A probe result disagrees with a unit test that exercises the same code path"
resolution_type: workflow_improvement
tags: [probe-corpus, fixtures, rfc3339, false-negative, verification, diagnostics]
---

# Validate hand-crafted probe corpora before trusting a failure signal

## Context

agenttrace's parser and diagnostics are verified two ways: in-repo regression
fixtures (unit tests over `crates/agenttrace-core`) and ad-hoc probes — the CLI
run over a small hand-written transcript corpus in `/tmp`, built to exercise one
code path end to end. In maintenance cycle 1 (roadmap rm-230, minted as
campaign-local rm-025 — flat-transcript `tool_use_id` pairing), a probe of exactly this kind returned the opposite of
the unit tests: the fix's own regression tests were green, while the CLI over
the hand-built corpus still reported every tool call as unmatched. The
contradiction was resolved in the fixture, not the code — several timestamps in
the hand-written corpus were RFC 3339-invalid (`...T00:00:4Z`, seconds not
zero-padded), and the diagnostics pipeline drops such events silently.

## Guidance

- **Treat a hand-typed corpus as untrusted input to the verification itself.**
  Before interpreting a probe's output, assert the corpus's own well-formedness:
  every `timestamp` parses as RFC 3339, every tool result carries an id the
  transcript's tool calls actually used. A one-line `jq`/python lint over the
  fixture is minutes; a false "fix failed" verdict costs a diagnosis detour and
  can mask a real regression behind a bogus one.
- **Build fixtures with the same serializer that produces real transcripts when
  possible.** A corpus assembled by serializing typed values (or by trimming a
  real session) cannot drift out of format; one assembled by typing JSON by
  hand can, and did.
- **When a probe and a unit test disagree, adjudicate on the smaller surface.**
  The unit test names its inputs and its code path; the probe drags in CLI
  argument parsing, cache lookup, and fixture format. Re-run the probe with a
  lint-passed fixture before concluding anything about the code.
- **Record the fixture-validation status with every probe result** ("corpus
  lint: all timestamps RFC 3339-valid, ids consistent") so a later reader can
  tell a verified probe from an unverified one.

## Why This Matters

The failure mode is asymmetric and silent. `parse_time`
(`crates/agenttrace-core/src/diagnostics.rs:978`) is
`DateTime::parse_from_rfc3339` — strict, so an unpadded second fails the parse.
Its callers drop rather than warn:

- `tool_latencies` (`crates/agenttrace-core/src/diagnostics.rs:785`) builds the
  result-side map with `filter_map`, so a tool *result* whose timestamp fails
  to parse vanishes from the map entirely.
- The call-side loop (`crates/agenttrace-core/src/diagnostics.rs:793`) skips
  any tool *call* whose own timestamp does not parse.
- A call whose id is then absent from the results map is counted as
  `unmatched` (`crates/agenttrace-core/src/diagnostics.rs:810`) — by design,
  the honest verdict for a trace that cannot show a result.

And the signal is not merely lost — it is inverted: report rows with
`unmatched > 0` are surfaced to the user
(`crates/agenttrace-core/src/diagnostics.rs:533`), so one malformed timestamp
in a hand-built corpus manufactures precisely the report line a real pairing
bug would produce. The code is behaving correctly at every step; the corpus is
lying. A verifier who trusts the probe over the unit tests would have reverted
a correct fix.

## When to Apply

- Any CLI-or-binary probe over a fixture written by hand rather than serialized
  by product code — transcript corpora, config files, request logs.
- Any moment a probe result and an in-repo test disagree; the fixture is the
  first suspect, not the code.
- When constructing regression fixtures for roadmap items: prefer fixtures
  committed to the repo and exercised by tests over `/tmp` probes, precisely
  because committed fixtures get format-checked by the test suite on every run.

## Examples

Before — corpus hand-typed with an unpadded second:

```jsonl
{"type":"assistant","timestamp":"2026-09-30T00:00:04Z","message":{"tool_use":[{"id":"toolu_01","name":"Bash","input":{}}]}}
{"type":"user","timestamp":"2026-09-30T00:00:7Z","tool_result":{"tool_use_id":"toolu_01","content":"ok"}}
```

Probe output: `Bash count: 1 unmatched: 1` — the correctly-paired call reads as
unmatched, because the result event's timestamp (`:00:00:7Z`) failed
`parse_from_rfc3339` and the result vanished from the latency map before the
pairing ever ran. (The malformed timestamp must sit on the RESULT line for
this failure shape: on the call line the call itself vanishes and the report
shows no Bash row at all.)

After — same corpus, zero-padded:

```jsonl
{"type":"assistant","timestamp":"2026-09-30T00:00:04Z","message":{"tool_use":[{"id":"toolu_01","name":"Bash","input":{}}]}}
{"type":"user","timestamp":"2026-09-30T00:00:07Z","tool_result":{"tool_use_id":"toolu_01","content":"ok"}}
```

Probe output: `unmatched: 0`, latency `3.0s` — one character of padding was the
entire difference between "fix failed" and "fix verified". The lint that would
have caught it is one line:

```bash
jq -e 'try (.timestamp | fromdateiso8601) catch error("bad ts")' corpus.jsonl >/dev/null
```

## Related

- `docs/solutions/security-issues/rustsec-advisory-shipped-via-ungated-lockfile.md`
  — sibling learning in the same corpus (ungated lockfile); independent
  problem, no overlap.
- Roadmap items whose acceptance criteria involve probe re-runs
  (`ROADMAP.md` rm-230, rm-233 — minted as campaign-local rm-025/rm-028)
  inherit this rule: probe evidence is only
  acceptance-grade when the corpus's own format was asserted first.
