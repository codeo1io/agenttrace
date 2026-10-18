---
type: stewardship-record
id: cycle5-final-validation
cycle: 5
batch: honest-coverage-honest-cache-honest-math
date: 2026-09-03
base_commit: 998ade8
record_kind: ce-handoff/v1
status: validated
---

# Cycle 5 final validation — CU-11..CU-16

Final end-to-end validation of the uncommitted cycle-5 batch on HEAD
`998ade8` (master), re-run from scratch after the independent review.
Executed direct (no ce-\* router installed in this session; consistent with
every prior phase's disclosure). Nothing staged, committed, or pushed —
Conductor owns topology.

## Gates (all first-run pass, release binary current)

| Gate | Result |
| --- | --- |
| `cargo fmt --all --check` | clean (FMT_CLEAN) |
| `cargo build --workspace --all-targets` | exit 0, `Finished dev profile` |
| `cargo test --workspace` | **203 passed / 0 failed** (13+6+2+70+7+65+40) |
| `cargo clippy --workspace --all-targets -- -D warnings` | exit 0; 0 error/warning lines |
| `cargo build --release` | no-op (`Finished release profile 0.25s`) — binary current |
| `scripts/ci/check-*.sh` ×10 (`AGENTTRACE_BIN=target/release/agenttrace`) | **10/10 exit 0** |

## Live end-to-end probes (release binary, operator corpus)

- **CU-11** `--audit -f json --range all` → `audited_sessions=1411,
  total_sessions=1411, excluded_reason=null, total_estimated_cost=702.4922`
  (the pre-fix silent sample reported 3.4427). `--sample 5` →
  `excluded_reason="sampled newest 5 of 1411 sessions (--sample 5)"`.
  `--sample 0` → exit 1, `Error: --sample must be at least 1`. `--sample`
  documented in `--help`.
- **CU-12** `--overview -f json --range 1d` → `discovered=364, parsed=74,
  out_of_scope=290` (74+290=364; pre-fix reported `discovered=71`).
  `--range all` → `discovered=364, parsed=1411, out_of_scope=0`. Text:
  `Parse: 1411 sessions from 364 sources, 0 skipped, 330 cache hits`.
- **CU-13** session cache after runs: `721 entries, 0 dead paths,
  9,340,395 bytes` — identical to the independent-review post-prune state;
  eviction prune is stable/idempotent.
- **CU-16** synthetic zstd file (`28 B5 2F FD` magic) →
  `Error: session file … is zstd-compressed (Codex rollout format);
  decompress it to JSONL first`.

## Working-tree audit — only intended changes

- **15 modified files** = exactly the implementation record's "Files
  touched" list (`git diff --name-only`): 8 code, `ROADMAP.md`, `README.md`,
  `README.zh-CN.md`, `docs/guides/governance-reports.md`,
  `scripts/ci/check-docs-commands.sh`, `scripts/ci/check-rust-real-cli-smoke.sh`.
- `ROADMAP.md` is **+244/−14**, byte-identical in count to the roadmap
  phase's evidence (the `--stat` figure 258 = ins+del) — no drift since.
- **11 untracked**: 10 campaign artifacts (this run's research/review/
  stewardship/prioritization/implementation docs + the four pre-existing
  carryovers flagged as F8-10 hygiene debt) and `AGENTS.md` (operator
  stewardship policy — intentionally untracked environment config, must
  never be committed).
- **0 staged**; HEAD `998ade8`; no commit/push performed.

## Remediation during validation

- Deleted stray `err_note.txt` (1 line:
  `Note: --limit caps list views only; this overview's aggregates cover all
  74 sessions.`). Provenance: mtime 2026-09-02 17:45, inside the implement
  phase's verification window; content is verbatim the stderr note emitted
  at `main.rs:418` — a stray redirect byproduct of a `--range 1d` probe,
  referenced by no script or doc. Unintended debris; removed so the tree
  contains only intended changes.

## Verdict

**PASS.** All five gates green on first run, all live outcomes reproduce the
implementation record's numbers, and the working tree contains only intended
changes. Ready for the commit/push/PR phases (Conductor-owned; see
`2026-09-03-cycle5-stewardship-request.md`).
