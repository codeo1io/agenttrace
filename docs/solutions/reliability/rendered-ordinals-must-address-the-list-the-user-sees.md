---
title: Rendered ordinals must address the list the user sees
date: 2026-10-16
category: reliability
module: agenttrace-cli (main.rs inspect/selection), agenttrace-core (diagnostics.rs)
problem_type: contract_mismatch
component: cli
symptoms:
  - "--inspect 1 rendered a different session than --sessions row 1 (README promised the 1-based listing index; code fed N to a hidden ≤5-session diagnostics priority list)"
  - "--inspect 3..5 exited rc1 'inspect rank exceeds available priority sessions' while 7 sessions were listable"
  - "the rendered inspection report carried zero session identity (no name, no journal path), and the selection reason field existed (diagnostics.rs:182) but was never rendered"
root_cause: internal_ordering_leak
resolution_type: contract_alignment
severity: medium
tags: [ordinals, cli-contract, inspect, session-identity, docs-drift, gate-arm]
---

# Rendered ordinals must address the list the user sees

## Problem

Every ordinal a CLI accepts is a *contract with a rendered list*. `--inspect
N` documented itself as "the `--sessions` list index (1-based)" in README's
flag table and `--help`, but the implementation fed N to `inspect_first()` —
a hidden, dedup-ordered, ≤5-session diagnostics priority list built from a
different ordering than the listing. Three defects fell out at once:

1. **Mis-addressing**: `--inspect 1` rendered the anomaly session the
   priority list ranked first while `--sessions` row 1 was a different
   session.
2. **False exhaustion**: ranks 3–5 exited rc1 "exceeds available priority
   sessions" while the listing happily showed 7+ rows.
3. **Undetectable wrongness**: the report carried no session identity and
   the selection-reason field was never rendered, so nothing in the output
   itself could tell the user which session they were looking at or why.

The docs gate made it worse by blessing it: `check-docs-commands.sh` locks
step flag *count* parity between README and `--help`, never prose semantics
— so the prose-vs-code contract drifted with a green gate.

## What Didn't Work

- Fixing the prose instead of the code would have legalized the hidden
  ordering: users do not click a help line, they count rows on the listing
  they can see.
- Keeping the priority list as the selection source required re-documenting
  an ordering the user cannot render or reproduce (dedup order of a
  diagnostics ranker).

## Solution

`--inspect N` now indexes `sessions[N-1]` — the same post-filter,
`--sort`/`--order`-respecting 1-based view the `--sessions` listing renders.
Out-of-range ranks fail loudly *against the listing* ("valid ranks are
1..=T in --sessions order"), not against an internal list. Human-readable
lanes self-identify via an identity header — session name, journal path,
`--sessions row N of T`, and the triage reason (rendered honestly as
`none` when absent); the JSON lane stays lossless and header-free by
design.

## Prevention rules

- An ordinal parameter must be resolved against the SAME array that
  renders the list the user counts — one ordering function, two consumers;
  never a parallel internal ranking.
- A report selected by ordinal must self-identify (name, source path,
  ordinal, selection reason) so mis-addressing is detectable from the
  output alone.
- Docs gates that pin only flag counts cannot protect prose contracts.
  Pin the contract itself: a test asserting README flag-table prose,
  `--help` text, and rendered behavior agree on the same ordinal semantics
  (`inspect_prose_and_rendered_behavior_agree`) — prose or code drift then
  fails CI.

## Vocabulary

- **rendered-truth contract**: the version of a behavior the user can see
  (listing, help, report) is the contract; internal orderings are
  implementation details and must not surface through documented ordinals.
