# Cycle-6 batch selection and landing decision

- run: 6d2a1d8db1ca48ef83522a8d5e6e733c (repository-maintenance
  fe743fb3399f4df68940183a2956fbf5, cycle 6), base c4aa3a0
- decided 2026-10-15 (prioritize 20e80122 / stewardship ec673ff3);
  compounded 2026-10-16 after pre-review validation.

## Decision

Implement the batch "newest lanes lie silently" — rm-1864 (lead),
rm-1865, rm-1866 — over higher-priority-but-unschedulable rows, and split
it as four change units: an antigravity unit (rm-1864), one opencode
storage unit (rm-1865 + rm-1866 with distinct test files), a CHANGELOG
rider, and the ROADMAP band landing as its own unit.

## Why (not just priorities)

- rm-299 (78.0, opencode warm-cache fingerprint) outranks all three but
  needs a dedicated cycle: it touches cache semantics, requires the
  rm-230 `SESSION_CACHE_SCHEMA_VERSION` bump, and needs warm/cold
  equivalence proofs across every lane. Squeezing it beside three parser
  rows would have made the full-suite record unreviewable.
- rm-992 (64.0, codex zstd) was already implemented by a sibling lineage
  (run 1e4f6b95 cycle 4, rm-1516 per-frame ruzstd) — implementing it here
  would have manufactured a merge conflict, not value.
- The selected three rows share one root cause (see
  docs/solutions/reliability/new-producer-lanes-lie-silently.md), same-file
  locality in `parser.rs`, and each was pinned by live evidence minted
  this cycle — one coherent, low-variance batch that fits one implement +
  validation window.

## Numeric model-id map scope

The antigravity numeric map (`ANTIGRAVITY_NUMERIC_MODEL_MAP`) is
**identity-only**: 1417 → gemini-4-argon (ccusage v20.0.34 / PR #1894
provenance). It never carries pricing — a mapped id absent from the
pricing catalog degrades through the disclosed unknown-model path rather
than the map inventing a price. Unmapped numerics attribute to their
stringified id and still disclose `antigravity_model:numeric_id`. New
mappings arrive upstream weekly (ccusage #1897: 1277/1050); refreshing the
map is a rider, not a schema event.

## Validation record (pre-review, consumed by this decision)

- targeted (b8a1b996): 8 suites, 468/0 — lib 412/0 + 6 impacted
  integration suites; fmt clean; clippy 0 warnings.
- full (93a44b0b, CI's authoritative command
  `cargo test --locked -p agenttrace-core -p agenttrace-tui -p agenttrace`):
  69 suites, 1167/0/0, both new test binaries executed; validation digest
  stable (`validation:v1:56f2c1d8…ea97`) — crates-only deltas cannot move
  it.
- Live end-to-end: `--overview -f json -d testdata/opencode` →
  `by_project: ["repo"]` (baseline at cycle start: `unknown`).

## Carry-forward for the next cycle

- Fog seam: sibling run 477a0c5c minted the same antigravity-numeric
  subject under the same numeral (their rm-1864 cites this run's
  evidence) and a different rm-1865 — integration must reconcile **by
  title**, never by numeral.
- Next-cycle candidates, in order: rm-299 dedicated cycle (warm-cache
  fingerprint + schema bump — note rm-1437 landed without one);
  rm-1020 pricing refresh after the 2026-10-19 drift-cron census
  (+31.8% LiteLLM drift already measured); ccusage #1897 numeric-map
  refresh rider; rm-715 re-validation when gemini ships a stable ≥0.65
  (#29618 is in v0.65.0-preview.0).
