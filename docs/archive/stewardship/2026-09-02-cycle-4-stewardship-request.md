---
artifact_contract: "ce-handoff/v1"
created_at: "2026-09-02T14:07:17Z"
title: "Stewardship request — cycle 4 batch: truthful reads, truthful gates, durable records"
summary: "Hands the conductor the selected cycle-4 maintenance batch (P7-1 lead + P7-2 + P7-3 + P7-5 + cycle-3 residuals as CU-6..CU-10) as repository change units with surfaces, overlap facts, and separation hints; makes no Git-topology decisions."
keywords: ["agenttrace", "stewardship-request", "silent-data-loss", "baseline-gate", "cycle-4", "change-units"]
cwd: "/work/projects/agenttrace"
resume_focus: "Establish the stewardship contract for the 'truthful reads, truthful gates, durable records' batch: inventory this repository, detect overlap (unlike cycle 3, the code tree is committed-clean; the only dirty tracked file is ROADMAP.md), split unrelated concerns, preserve the dirty state listed below, and plan branches/worktrees before implementation begins."
repository: "luoyectl/agenttrace"
repo_root_sha: "66320145ae38163bce90b45668e3e4afd95d3c2a"
branch: "master"
head: "6632014"
---

# Stewardship request — cycle 4

This document is a **request**, not a contract. Per the conductor work order
(run `2a15625945fc40419fc4691c59b42a7b`, phase stewardship, attempt
`fa7cf54bd20d45f3ab3afed5918cfd32`), it describes the selected maintenance
batch and stops there: **it chooses no branch, worktree, commit order, or
any other Git topology.** Those decisions belong to the conductor.

Routing note: no ce-* skill owns stewardship requests (same gap disclosed
by cycles 1–3, all provenances read this campaign). The narrowest
installed match is **ce-handoff**; this document follows its
`ce-handoff/v1` frontmatter and pointer-first body contracts at the
campaign's user-directed destination (`docs/stewardship/`). Harness
disclosure: this delegate session has no subagent surface; everything here
was produced in-thread.

## Title

agenttrace cycle 4 — "truthful reads, truthful gates, durable records"
(P7-1 lead + P7-2 + P7-3 + P7-5 + cycle-3 residuals; five change units,
CU-6..CU-10).

## Summary

Implement the batch selected by the prioritize phase
(`docs/stewardship/2026-09-02-cycle4-prioritization.md`), closing every
way the tool can currently lie or lose data without a signal — where
cycle 3 closed the last reproducible panic, cycle 4 closes the silent
defects: lines that vanish, files that "unsupported" despite being valid,
a CI gate that cannot fail, and the only durable record overwritten
non-atomically. All five findings were re-verified live this campaign on
the release binary (reproducers listed in the prioritization doc,
`/tmp/pri4/`).

- **CU-6 — P7-1 (lead): the generic-JSONL fallback silently drops
  recoverable lines with zero health signal.** `parse_jsonl_session`
  (`lib.rs:374`) strict-parses each line: `serde_json::from_str::<Value>`
  at `lib.rs:382` and `from_value::<Event>` at `lib.rs:393` both
  `continue` on error, and `usage: BTreeMap<String, i64>` (`lib.rs:134`)
  coerces nothing. Live: a three-line file (valid / lone-surrogate /
  valid) reports **2** messages with `data_health` byte-identical to the
  clean file (`parsed: 1, skipped: 0`); an Event-typed `usage` line
  reports **0** messages. Fix: route the fallback through the lenient
  machinery the format detectors already use (`repair_lone_surrogates`
  `parser.rs:3796`, `parse_jsonl_value_lenient` `parser.rs:3772`,
  `number_as_i64` `parser.rs:3582`), tolerate Event-typed usage by
  extracting known numeric leaves, and extend `DataHealth`
  (`insights.rs:107`, computed at `insights.rs:292`) with per-reason skip
  counts — the `skipped` field already flows to text/MD/HTML/JSON
  reports (`reports.rs:532`, `:535`, `:561`, `:577`). Implementer notes:
  (a) the lenient helpers are private — prefer a small parser-side
  wrapper over exporting internals to `lib.rs`; (b) pin the **verified**
  drop shapes (lone-surrogate line, Event-typed usage) — a string-typed
  usage value like `"input_tokens":"5"` actually survives coercion today
  (discovered live; do not write a red test for it).
- **CU-7 — P7-2 / candidate 41a: no BOM handling at the parse entry.**
  `parse_session_file` reads bytes straight through (`parser.rs:20-26`)
  into the shared `parse_raw_session` entry (`parser.rs:63`); a UTF-8
  BOM before one valid Claude-Code line yields `exit 1, "unsupported
  session format"` (live), and UTF-16 fails the same misleading way.
  Fix: strip a UTF-8 BOM once, at offset 0, in `parse_raw_session` (all
  formats inherit it; every caller funnels through `parser.rs:63`);
  sniff UTF-16 BOMs and return a diagnosis-grade error naming the
  encoding. Decision recorded in the prioritization doc: **strip + name,
  do not transcode** this cycle (no new encoding dependency).
- **CU-8 — P7-3 / candidate 42: baseline thresholds never gate the exit
  code.** `add_baseline_comparison` (`reports.rs:634`) computes
  `slower_than_baseline`/`cost_above_threshold`/`tokens_above_threshold`
  (`reports.rs:672-677` area) and nothing reads them; the CLI gate
  `evaluate_overview_gate` (`lib.rs:1071`, invoked from
  `main.rs:389-395`, `std::process::exit(2)` at `main.rs:420`) covers
  only health/critical/tool-fail. Live: a forged baseline with
  `--baseline-max-token-delta-pct 1` yields `token_delta_pct: 100.0,
  tokens_above_threshold: True` and **exit 0** — while
  `docs/guides/ci-integration.md:116-124` documents the step as a CI
  check. Fix: reuse the exit-2 machinery for the three baseline booleans
  (always-on with an opt-out flag — exact spelling is an implement-time
  decision recorded in the prioritization doc), no-baseline runs stay
  exit 0 with a labeled skip, flags declared at `main.rs:88-94`, call
  site `main.rs:374-384`, guide snippet updated to show the failing exit.
- **CU-9 — P7-5: non-atomic writes of the only durable record.**
  `write_pricing_cache` (`pricing.rs:329-336`, `std::fs::write` at
  `:334`) and `preserve_derived_history` (`history.rs:34-46`, at `:46`)
  write in place; an interrupted write truncates `history.json` — the
  one durable record given Claude Code's 30-day default transcript
  retention. Fix: apply the CU-4 pattern (`unique_temp_path`,
  `session_cache.rs:237`) — `<name>.tmp.<pid>.<seq>` plus atomic rename —
  to both, sweep orphaned `*.tmp.*` siblings on cache load
  (`load_session_cache`, `session_cache.rs:270`; retires the pass-7
  residual), and quarantine a truncated file under a visible warning
  rather than silently discarding it.
- **CU-10 — cycle-3 residuals (promises the Completed record already
  makes).** (a) `repair_lone_surrogates` (`parser.rs:3796`) has no
  escaped-backslash lookbehind: literal `\\uXXXX` text on an
  already-failing line can be rewritten — add a backslash-parity guard
  plus a corpus line; (b) `SQLITE_SNAPSHOT_SCHEMA_VERSION` stayed 5
  across CU-5's naming change (`session_cache.rs:9`): bump-or-compatible
  decision plus a version test asserting stale pre-CU-5 snapshots
  invalidate rather than serve placeholder names. Note the genuine
  overlap with CU-6 below.

Acceptance criteria and evidence expectations per unit are in
`ROADMAP.md` (hardening lane: "No silent data loss" extended entry, "BOM
handling at every parse entry", "Baseline thresholds must gate the exit
code", "Cache and history durability", "Cycle-3 residuals on closed
items"); the red-to-green execution order (fixtures first: a
`generic-loss` adversarial family plus BOM/UTF-16 variants wired into
`crates/agenttrace-core/tests/discovery_contract.rs`, generator sibling
of `scripts/fixtures/make-adversarial-sqlite.py`) and the full
verification matrix (debug+release tests, fmt, clippy `--all-targets`,
all runnable `scripts/ci/check-*.sh`, reproducers re-run end to end) are
in the cycle-4 prioritization doc.

## Repository candidate

- `luoyectl/agenttrace`, working tree `/work/projects/agenttrace`,
  branch `master`, HEAD `66320145ae38163bce90b45668e3e4afd95d3c2a`
  ("ci: use self-hosted runner" — a workflows-only commit atop `93aaf05`,
  the commit carrying cycles 1–3). PR #282 is open upstream; its merge
  timing is the conductor's concern, not this request's.
- **Dirty state to preserve:** exactly one modified tracked file —
  `ROADMAP.md` (this campaign's roadmap phase; deliberately uncommitted;
  commit is a later conductor gate) — plus six untracked paths
  (`docs/research/2026-09-02-extensions-research-pass6.md`,
  `docs/reviews/2026-09-02-adversarial-repository-assessment-pass7.md`,
  `docs/stewardship/2026-09-02-cycle4-prioritization.md`,
  `docs/stewardship/2026-09-02-reconciliation-record.md`,
  `docs/stewardship/2026-09-02-roadmap-cycle2-update.diff`,
  `docs/stewardship/2026-09-02-roadmap-cycle3-update.diff`). None may be
  clobbered, reverted, or absorbed into code commits.
- **Overlap fact for planning — the opposite of cycle 3:** the code tree
  is **committed-clean**. No uncommitted edits sit under `crates/` or
  `scripts/`, so this batch has no in-flight code to build upon or
  collide with; the only pre-existing dirty file (ROADMAP.md) is
  untouched by every unit below. `git worktree list` shows a single
  worktree (`/work/projects/agenttrace 6632014 [master]`).
- Baseline for verification: assessment pass 7's `cargo test --workspace`
  180/180, `cargo fmt --check`, `cargo clippy --workspace --all-targets`
  clean at `93aaf05`; `git diff --stat 93aaf05..6632014` proves zero Rust
  changes since, so the baseline describes this tree. Re-run the matrix
  at implementation time.

## Surfaces (file:line)

| Unit | Surfaces |
|---|---|
| CU-6 (P7-1) | `crates/agenttrace-core/src/lib.rs:374` (block through `:395`; strict drops `:382`, `:393`; `usage` map `:134`); lenient machinery to route through: `crates/agenttrace-core/src/parser.rs:3772`, `:3796`, `:3582`; skip counts: `crates/agenttrace-core/src/insights.rs:107`, `:292`, surfaced `crates/agenttrace-core/src/reports.rs:532`, `:535`, `:561`, `:577`; fixtures `scripts/fixtures/` (new generic-loss family) + `crates/agenttrace-core/tests/discovery_contract.rs` |
| CU-7 (P7-2) | `crates/agenttrace-core/src/parser.rs:20-26` (read path); shared entry `crates/agenttrace-core/src/parser.rs:63` (single BOM strip point); fixture variants per corpus family |
| CU-8 (P7-3) | `crates/agenttrace-cli/src/main.rs:88-94` (flags), `:374-384` (comparison call), `:388-421` (gate + exit 2); `crates/agenttrace-core/src/lib.rs:1071` (`evaluate_overview_gate`); `crates/agenttrace-core/src/reports.rs:634` (block through `:690`; unread booleans `:672-677`); `docs/guides/ci-integration.md:116-124` |
| CU-9 (P7-5) | `crates/agenttrace-core/src/pricing.rs:329-336` (`fs::write` at `:334`); `crates/agenttrace-core/src/history.rs:34-46` (`fs::write` at `:46`); pattern source `crates/agenttrace-core/src/session_cache.rs:237`; orphan sweep `crates/agenttrace-core/src/session_cache.rs:270` |
| CU-10 (residuals) | `crates/agenttrace-core/src/parser.rs:3796` (backslash parity, shares the function CU-6 calls); `crates/agenttrace-core/src/session_cache.rs:9` (snapshot schema version) + a version test near the load path `:270` |

## Must remain separate

1. `CU-6 lib.rs generic-JSONL leniency + skip counts` ✂ `CU-7 parser.rs
   BOM strip at the entry` — adjacent phases of one parse pipeline but
   different units and corpora: CU-7 is offset-0 pre-processing shared by
   every format, CU-6 is per-line fallback semantics for one format
   family. Each independently revertable with its own fixtures.
2. `CU-8 main.rs/lib.rs/reports.rs baseline gate + guide` ✂ `CU-9
   pricing.rs/history.rs/session_cache.rs atomic writes` — zero shared
   surface; different crates-level concerns (CLI exit semantics vs
   durability).
3. `CU-10 parser.rs backslash-parity guard` ✂ `CU-6 lenient fallback` —
   **overlap fact, not disjointness**: both touch
   `repair_lone_surrogates` (`parser.rs:3796`) — CU-6 *calls* it from a
   new fallback path, CU-10 *hardens* it. They must remain distinct
   change units with distinct acceptance (a parity corpus line; a
   recovered-line count), and should be sequenced in one implementation
   lane (CU-6 then CU-10) rather than split across parallel worktrees —
   but neither may absorb the other.
4. `cycle-4 batch (CU-6..CU-10)` ✂ `candidate 33 per-turn model
   attribution (parser.rs model freeze, cycle-5 lead)` — C33 is
   explicitly the next cycle's capability lead; it must not ride this
   batch even though it also lives in the parser family.
5. `cycle-4 batch (CU-6..CU-10)` ✂ `dependency motion (Cargo.toml /
   Cargo.lock: ureq 3.4.0, rusqlite 0.40.2, crossterm 0.29.0; fold
   dependabot #278/#279)` — the one-motion dependency cycle stays its
   own batch by roadmap rule.
6. `CU-6's additive skip-reason fields in reports.rs` ✂ `the
   output-honesty sweep (P3-4 control characters, P3-7 newline parity,
   P4-2/P4-3 CLI guards)` — CU-6 necessarily touches the same report
   surfaces (`reports.rs:532/:535/:561/:577`) but only additively (new
   per-reason counts); the byte-format restructuring of the
   output-honesty lane must stay a separate batch with its own corpus
   and docs sweep.

## What this batch deliberately does not include

(deferred with rationale in the cycle-4 prioritization doc: C33+C25
capability accuracy — cycle-5 lead; P7-4 SQLite `since` push-down — pairs
with candidate 36's watermark either/or; P3-1 platform parity —
unverifiable on this host; P3-4-family output honesty; N4 dependency
motion; N5/P4-4/P4-5 CI truth — re-examine now that CI is self-hosted;
C37/C38/C40 capability riders.)

---

**Artifacts:** this file. Git topology untouched; nothing committed,
pushed, or CI'd.
