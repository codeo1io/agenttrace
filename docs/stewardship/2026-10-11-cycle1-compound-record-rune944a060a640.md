# Cycle 1 compound record — run `e944a060a640` (repository-maintenance `9c0675055f78478aa9320c8f01edeac2`)

Cycle: 1 · compound attempt `9ea66a38542d471f9aa1f56740645957` · 2026-10-11 ·
worktree HEAD `8982722` (upstream `428f0e9d` + 12 maintenance landings) ·
batch landed in-worktree, **commit gate pending**.

## Phase ledger (attempts, all succeeded)

| phase | attempt | one-line outcome |
|---|---|---|
| assess | `bdbd50cb` | 8 findings, 4 live-PoC'd; envelope 888/0 + fmt/clippy clean |
| research | `f8bdbf49` | 8-candidate dossier (redo after dead `0745262e`) |
| roadmap | `5e585bbb` | spool patch: mints rm-954/rm-955 + 12 folds (redo after dead `ee0df3d3`) |
| prioritize | `f4db79c1` | selected batch under fleet-contention constraint |
| stewardship | `5e491307` | structured stewardship_request; canonical dirty state inventoried |
| implement | `2ba22694` | rm-619 + rm-954 + rm-955, all red-first |
| targeted_tests | `d36d5c7f` | gate `cargo test` → 891/0, digest `fc316b85…` verified current |
| full_tests | `f0e56af3` | gate `cargo test` → 891/0, 53 suites ok (redo after dead `037d8bcd`) |
| compound | `9ea66a38` | this record; wall flips + prevention docs; zero mints |

## The batch — "usage truthfulness & honest ordering"

- **rm-619 (LEAD, correctness/usage-truthfulness)** — opencode message-level
  `tokens` objects with only non-folded key classes suppressed the
  step-finish rescue: `add_opencode_tokens` returned presence-bool `true`,
  arming suppression and disarming the part lane → zero usage, silently.
  Fix: `OpencodeTokensFold{billing,reasoning}` recognized-class semantics;
  part lane keys on `!message_fold.billing`; message-level `tokens.reasoning`
  folds once onto `tokens_reasoning` (CU-20; part lane folds its own reasoning
  only when the message did not). Red-first twin fixture
  `tests/fixtures/usage-accounting/opencode-reasoning-rescue/`
  (13/1 → 14/0); live PoC 0/3-est/$0.0000 → 100/50/$0.0011 == control,
  `tokens_reasoning` 777.
- **rm-954 (correctness/honest-ordering)** — `sqlite_value_asc_rank`
  (NULL < numeric < wrong-typed, ties by rowid) honors the "matches SQLite's
  ASC semantics exactly" claim in the earliest-user-text selection.
  Red-first mixed-typed part table.
- **rm-955 (correctness/honest-ordering)** — statusline five_hour/seven_day
  latest-state pick ranks `(captured_at, journal-position)` (rm-921
  crossings discipline). Red-first via python-revert (stale 41.0/21.0 →
  fresh 84.0/46.0).
- **`SESSION_CACHE_SCHEMA_VERSION` 51 → 53** — fleet census mint (sibling
  worktree `run-57c491338818` holds uncommitted 52), full same-unit sweep:
  ladder rung, governance sentence (`check-docs-commands.sh:73`), occurrence
  oracle `schema_53`, sweep-doc sites 5/6, TUI dynamic const.

## Validation (recorded outcomes; compound re-ran nothing)

- targeted gate: GATE_RC=0, workspace 891 passed / 0 failed
- full gate: GATE_RC=0, 891/0, 53/53 suites ok
- implement-time: fmt `--check` rc0 (run FIRST), clippy
  `--workspace --all-targets -- -D warnings` rc0
- digest `validation:v1:fc316b85…` verified current by canonical
  recomputation at both gates; worktree diff pin sha256
  `1fc281aad1c0a8b9e0209f1184c3c52abcd9e90f1ac9dc6600c4b04d2cbe8c28`
  (see `delegate/2ba22694…-scratch/pins.txt`)

## Compounded artifacts

- Wall chain (spool, commit gate lands it): pristine `f51a09f0`
  →`5e585bbb/roadmap-delta.patch`→ intermediate `927b6afb`
  →`9ea66a38…-scratch/ROADMAP-compound.patch`→ compound `4cc0c1ba`;
  forward/reverse roundtrip proven byte-exact in sandbox; flips
  rm-619/rm-954/rm-955 → implemented (pre-review credit), ZERO mints,
  next free id **rm-961**.
- `docs/solutions/quality-gates/usage-fold-gates-must-count-folded-classes-not-object-presence.md`
- `docs/solutions/workflow-issues/schema-version-mints-must-census-uncommitted-fleet-siblings.md`
- This record.

## Context for the next cycle

1. **rm-961 lead (recommended mint):** opencode message with PARTIAL billing
   (`tokens: {input: N}`, no output) still sets `billing: true` and suppresses
   the step-finish part lane — drops that step's output tokens. Same seam as
   rm-619, one notch narrower. Discovered at implement review; pre-existing
   semantics, deliberately untouched by this batch.
2. Deferred-under-contention (fresh census required before re-prioritizing):
   by_model per-add round4 drift (rm-938/rm-949, live PoC corpus
   `/tmp/at-assess-bdbd/corpus/`), npm-probe byte cap (c23d7ee8b598
   read_capped batch + rm-948), unbounded session reads (e4917d16
   rm-927/929/930 + rm-700, RSS PoC 64 MiB → 269 MiB), fmt_duration carry
   (rm-953), MSRV statement (rm-946), antigravity-acp roots (e4eb2254
   rm-947 batch complete-unlanded).
3. Watch: LiteLLM pricing schema churning (openrouter purges #45665-#45669;
   priority fields 0 → 934) — gate any fast/priority-tier work (rm-164
   extension) on re-verified stability; CC 2.1.296 adaptive-thinking remains
   corpus-gated.
4. Commit-gate chain for THIS cycle: wall patches (5e585bbb leg, then
  9ea66a38 leg) BEFORE the code batch; code batch diff pin `1fc281aa…`;
   untracked deliverables to stage: fixture tree + this record + the two
   prevention docs.
