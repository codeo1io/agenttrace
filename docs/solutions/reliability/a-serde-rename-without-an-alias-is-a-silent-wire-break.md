---
title: A serde rename without an alias is a silent wire break
date: 2026-10-07
category: reliability
module: agenttrace-core (lib.rs Event wire model, parse_jsonl_session fallback)
problem_type: wire_compatibility
component: core
symptoms:
  - "Event.model_used deserialized only the PascalCase ModelUsed key (rename = \"ModelUsed\", no alias) — journals emitting snake_case model_used left the field empty with no error"
  - "The usage fold fired only for session_meta/meta roles, so usage riding conversation lines never reached metrics while text estimation stood in for it"
  - "The rejected line re-entered through the per-line estimation fallback: no skip counter, no disclosure — the session reported model 'default' with tokens 1/1 against journal truth gpt-5 in=7 out=3"
root_cause: missing_alias
resolution_type: wire_compat_repair
severity: medium
tags: [serde, wire-compatibility, alias, silent-fallback, disclosure-counters, generic-lane]
---

# A serde rename without an alias is a silent wire break

## Problem

`Event.model_used` carried `#[serde(rename = "ModelUsed")]` with no `alias` — by
design for the payload-mode variant upstream emits, but the struct is also the
deserialization target for *generic* journals of unknown family, whose writers
spell the key `model_used`. A rename silently *narrows* accepted wire shapes:
the snake_case key deserializes to the default (empty) instead of erroring, so
nothing downstream can tell a journal that never carried the field from one
that carried it under the other spelling.

Two more layers converted the gap into silent wrong numbers. The usage fold in
`analyze()` counted only `session_meta`/`meta` roles, so usage blocks riding
conversation lines (the generic-lane convention) never reached metrics. And the
strict parse's rejects re-entered through the per-line estimation fallback
(`parse_jsonl_session`), which produces plausible-looking totals with
`provenance.tokens = estimated_from_text`. The result: a perfectly valid journal
rendered model `default`, tokens 1/1 — no error, no counter, no way to tell.

## Symptoms

- Reported model `default` and 1/1 tokens for a session whose journal literally
  contains `model_used` and a usage block (PoC: `c2.json`, replayed via
  `agenttrace -d corpus --sessions -f json`).
- Both casings live in the wild: upstream Claude-derived writers use
  `ModelUsed`; generic/wire-neutral writers use `model_used`.
- Zero trace at the skip-counter level — the line was not rejected, it was
  *re-parsed on the estimation lane*.

## What Didn't Work

- Trusting the enum shape: upstream's `EventMode::Payload` carries `model_used`
  by design — the shape was right for *its* writer and wrong as a universal
  deserialization target; the defect was the missing alias plus the silent
  fallback, not the enum.
- Error-driven debugging: nothing errors. A casing gap on a defaulted field is
  invisible unless a test feeds both spellings or a disclosure counter fires.
- Wall greps for `model_used`: existing ROADMAP hits were ESC/CSV value
  families — the alias-gap class had never been claimed anywhere.

## Solution

Implemented in cycle 2 of run 91833f02 (worktree `run-91833f02565b-91833f02`,
uncommitted for the commit gate):

- `alias = "model_used"` beside `rename = "ModelUsed"` on `Event.model_used`
  (lib.rs) — both wire casings populate the field.
- `analyze()` extends the meta-only usage gate with a generic-lane arm:
  conversation-line usage where `source_tool == "generic"` sets
  `provenance.tokens = reported_by_agent` (estimation stands down) and folds
  with the meta arm's saturating contract, joining `usage_models` so a
  mid-session model switch prices per block. Scoped to the serde-origin lane
  precisely because native families fold usage through meta events — folding
  their conversation lines on top would double-count.
- Two disclosures ride `Metrics.line_skips`: `model_or_usage_dropped`
  (parse-side — a rejected line that *itself* carried model identity or a usage
  block is lost accounting, not a lost message) and
  `usage_present_not_counted` (analyze-side — usage observed outside its
  family's counted lane while the session estimated). `parse_jsonl_session`
  now *merges* its counters into `metrics.line_skips` instead of overwriting
  analyze()'s — clobbering them would silence exactly the disclosures that
  matter.
- 7 red-first tests (`tests/generic_model_usage_truth.rs`) with the PoC corpus
  copied verbatim into `tests/fixtures/rm-616-generic/` (md5-pinned) — /tmp
  gets reaped within a cycle; the in-tree fixture is the durable PoC home.

## Prevention rules

1. **Every `serde(rename = …)` on a journal-shaped struct needs an `alias` for
   the opposite casing** (or a casing-parity test proving the other spelling
   cannot occur). A rename narrows accepted input silently — review it as a
   wire-compat decision, not a style one.
2. **Fallback lanes disclose what they swallowed.** Any "re-read on the
   estimation/generic lane" must be paired with a counter naming what was
   dropped (`model_or_usage_dropped` here), or a silent substitution stays
   indistinguishable from a clean parse.
3. **Counted-lane discipline:** usage folds on exactly one role/surface per
   family (meta events for native families, conversation lines for the generic
   lane). Usage seen anywhere else increments `usage_present_not_counted`
   instead of folding — the fold would double-count.

## Vocabulary

- **Counted lane** — see `CONCEPTS.md`; the term the disclosures are phrased in.
