---
title: Sanitizer widenings ship as named predicates with bounds tests
date: 2026-10-16
category: security-issues
module: agenttrace-core (statusline.rs shared sanitizers)
problem_type: partial_spoofing_coverage
component: core
symptoms:
  - "od -c of --sessions TSV/CSV rows showed raw E2 80 8D (ZWJ) / E2 80 8C (ZWNJ) in session-name and model cells while RLO and NUL on the SAME rows were U+FFFD-replaced"
  - "the enumerated directional set (RLO/LRO/RLI/…) was sanitized but the zero-width joiner pair gluing cells together was not — single-cell spoofing survived the sanitizer"
  - "widening a shared predicate in place risked dragging machine lanes (JSON losslessness) and the TUI emoji lane into the substitution"
root_cause: enumerated_set_not_class
resolution_type: scoped_predicate_widening
severity: medium
tags: [output-safety, unicode, Cf, ZWJ, ZWNJ, U+FFFD, spoofing, lane-carve-outs]
---

# Sanitizer widenings ship as named predicates with bounds tests

## Problem

The output-safety sanitizers died for an *enumerated* set — the directional
bidi format controls (RLO & co.) — while the rest of the Cf general
category rode through raw. A hostile (or merely emoji-rich) journal could
carry ZWJ U+200D / ZWNJ U+200C into rendered TSV/CSV cells byte-for-byte:
invisible characters that glue what a terminal renders as separate cells —
name onto model onto cost — the same single-cell spoofing class the
enumerated set already dies for. The gap was only visible at the byte
level; every rendered row looked clean.

Widening the existing predicate in place was the tempting fix and the wrong
one: the JSON lane's losslessness carve-out and the TUI display filter's
emoji handling are both *pinned against the directional predicate*. A
one-line widening of that predicate would silently strip legitimate JSON
payloads and TUI emoji sequences.

## What Didn't Work

- Broadening `is_bidi_format_control` itself — it is the anchor two other
  lane carve-outs are pinned to; widening it changes contracts nobody
  asked to change.
- Waiting for a "full Cf class" pass — the joiner pair is live-spoofable
  today, and the remaining non-directional set (U+FEFF/U+2060/U+00AD/
  U+180E) has different display semantics worth deciding one by one.

## Solution

The widening shipped as its own named predicate —
`is_joiner_format_control` (exactly U+200C, U+200D) — subscribed only by
the two shared boundaries (`sanitize_line_segment`,
`sanitize_output_document`), so every human-readable lane substitutes
while machine lanes keep the bytes. The directional predicate and its
pinned carve-outs are untouched *by construction*, not by vigilance.

## Prevention rules

- When a sanitizer's coverage is an enumerated subset of a Unicode class,
  widen by adding a NEW named predicate for exactly the members you can
  defend, not by editing the existing one — existing predicates are
  anchors for lane carve-outs you will not find by grep alone.
- Ship every new predicate with a bounds test that asserts the NON-widened
  neighbors: the directional member that belongs to the other predicate,
  the still-open class members, and a plain Cc control. The bounds test is
  the durable record of what was deliberately NOT widened.
- Verify coverage at the byte level (`od -c`), not the rendered screen:
  invisible-format-control gaps are invisible by definition — assert the
  replacement bytes (EF BF BD) appear and the raw pair (E2 80 8C/8D)
  does not, per lane, while the lossless lane keeps the raw pair on
  purpose.
- Compose at shared boundaries, not per-renderer: one line-segment
  sanitizer + one document sanitizer that both subscribe to the predicate
  covers text/TSV/CSV/markdown/html in one move.

## Vocabulary

- **lane carve-out pin**: a test that locks a machine lane (JSON) to
  byte-losslessness; any predicate that lane's filter is derived from is
  frozen by that pin.
- **bounds test**: asserts membership AND non-membership of the named
  neighbors, recording the deliberate scope.
