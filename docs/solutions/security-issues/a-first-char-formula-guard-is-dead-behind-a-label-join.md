---
title: A first-char formula guard is dead behind a label join
date: 2026-10-16
category: security-issues
module: agenttrace-core search text renderer and reports formula guards
problem_type: security_issue
component: cli
symptoms:
  - "Every --search evidence row rendered spreadsheet-formula introducers raw: `agenttrace --search cmd` over a hostile corpus printed `- tool: +cmd|' /C calc'!A0` with no leading quote marker (cat -A), re-arming the payload when the pasted output landed in a spreadsheet"
  - "The sibling row that carries no label prefix (the summary row) DID render with the `'` paste-marker — the one guarded row in the lane, which localized the defect to composition order rather than to the guard itself"
  - "The lane's doc comment claimed these fields were guarded; the guard existed and was called on every row, so a review that checks 'is the guard invoked' passes green"
root_cause: guard_applied_after_composition
resolution_type: code_fix
severity: medium
tags: [paste-safety, csv-injection, spreadsheet-formula, sanitize-on-render, guard-granularity, search-lane, rm-540]
---

# A first-char formula guard is dead behind a label join

## Problem

The `--search` text renderer composes each evidence row as `label: value`
(`add_match` in `crates/agenttrace-core/src/search.rs` — `name:`, `tool:`,
`tool argument:`, …) and then handed the **joined item** to
`guarded_segment`, which runs `reports::guard_formula_cell` after the
rm-383 control-byte sanitizer. `guard_formula_cell` only neutralizes a
formula introducer (`=`, `+`, `-`, `@`) when it sits at the **first
non-whitespace character** of the string it is given. Behind a label
prefix the payload never starts the string, so the guard could not fire
on any evidence row — a protection that was present, invoked on every
row, and dead by construction. Discovered as run `7fd83ab2` cycle 2
assess F1 (live-proven, 2026-10-15, base `e130845c`); fixed the same
cycle as the batch's lead unit (implement `87c78056`, red-first).

## Symptoms

- `agenttrace --search cmd tests/fixtures/formula-lane/hostile-tool.jsonl`
  printed `  - tool: +cmd|' /C calc'!A0` raw — no `'` marker (cat -A).
- `agenttrace --search review` printed
  `- name: =HYPERLINK("http://evil.example","click-me") please review` raw.
- The summary row (label-less) rendered the same payload class WITH the
  marker — the seam was the join, not the guard.
- No test failed before the fix: the lane's contract tests asserted
  markers on the rows the guard could actually reach.

## What Didn't Work

- **Guarding the joined line.** Calling `guard_formula_cell` on
  `format!("{}: {}", label, value)` is a no-op for any value whose
  introducer follows the label — first-char guards and string prefixes
  are mutually exclusive.
- **Trusting the doc comment.** The rm-985 doc block on
  `guarded_segment` said the transcript-derived fields of the search
  renderer are guarded; that claim had never been exercised by a test
  that could fail behind a label. Reviews of "is the guard called"
  cannot catch where in composition it is called.
- **Whole-output equality assertions.** The upgraded contract asserts
  the marker PRECEDES EVERY hostile occurrence (an occurrence walk), not
  that the rendered block equals one hand-picked clean string — equality
  pins whatever the guard happened to reach.

## Solution

Split the fixed label off and guard the **value** at the point of
composition (`crates/agenttrace-core/src/search.rs`,
`guarded_evidence_item`):

```rust
fn guarded_evidence_item(item: &str) -> String {
    match item.split_once(": ") {
        Some((label, value)) => format!("{}: {}", label, guarded_segment(value)),
        None => guarded_segment(item),
    }
}
```

The value passes through the unchanged rm-985 stack
(`sanitize_line_segment` → `guard_formula_cell`);
`reports::guard_formula_cell` itself is untouched, label-less items and
the summary row keep the whole-item path, and `-f json` stays
byte-lossless by design (the guard is a text-render concern; the stored
match item keeps its raw value — pinned). Text-to-columns and
`": "`-aware paste targets make the value a cell of its own, which is
exactly the cell the marker must precede.

## Why This Works

A paste target arms a formula when the introducer starts **the cell it
creates**. Once a renderer prefixes untrusted data with a fixed label,
the cell boundary the attacker controls is the start of the *value*,
not the start of the line. Guards must therefore run on each untrusted
field **before** it is joined into a larger string — the composition
boundary, not the render boundary, is the guard's input.

## Prevention

1. **Apply output guards at field composition, never after joins.**
   When a renderer adds a fixed prefix (label, bullet, column header) in
   front of untrusted data, enumerate the join sites and guard the
   field at each one — or route every join through a single
   label-aware helper (the pattern `guarded_evidence_item` introduces).
2. **Treat a first-char guard over compound strings as a smell.** If a
   guard inspects only the leading character, grep its call sites for
   string composition between the data and the call; any `format!`
   join in between is a dead guard waiting to be found by an attacker.
3. **Every protection claim in a doc comment needs a red-first test
   that fails if the protection goes dead.**
   `search_evidence_rows_carry_the_value_paste_marker` (in
   `crates/agenttrace-core/tests/formula_lane_contract.rs`) was written
   red against the joined-line guard and is the contract that keeps
   this lane honest; two cycles of believing the rm-985 doc is what this
   test cost.
4. **Localize seam defects by row contrast, not by guard inspection.**
   When one row in a lane is safe and its sibling is not, diff the two
   rows' composition order first — here the label-less summary row
   against its labeled siblings pointed at `add_match`'s join before
   any guard code was read.

## Related

- `docs/solutions/security-issues/terminal-injection-through-journal-derived-statusline-strings.md`
  — the same sanitize-on-render family one layer down (control bytes vs
  formula introducers), with the matching rule: sanitize at the
  terminal boundary, never corrupt the fidelity record.
- rm-540 row and the cycle-2 compound banner in `ROADMAP.md` (fleet
  ledger); formula-lane hostile fixtures under
  `crates/agenttrace-core/tests/fixtures/formula-lane/`.
- The compare-lane sibling-join sweep named in rm-540's acceptance is
  the open follow-up: other renderers that join labeled untrusted pairs
  get the same split-and-guard treatment.
