---
artifact_contract: "ce-handoff/v1"
created_at: "2026-09-02T12:10:00Z"
title: "Stewardship request — cycle 3 batch: trustworthy strings on untrusted input"
summary: "Hands the conductor the selected cycle-3 maintenance batch (P6-1 lead + P6-4 + riders P6-2, P6-3, C34) as repository change units with surfaces, overlap facts, and separation hints; makes no Git-topology decisions."
keywords: ["agenttrace", "stewardship-request", "utf-16-repair", "cycle-3", "change-units"]
cwd: "/work/projects/agenttrace"
resume_focus: "Establish the stewardship contract for the 'trustworthy strings on untrusted input' batch: inventory this repository, detect overlap (all seven batch surfaces already carry uncommitted cycle-1/2 edits), split unrelated concerns, preserve the dirty state listed below, and plan branches/worktrees before implementation begins."
repository: "luoyectl/agenttrace"
repo_root_sha: "e0059522b4fc74d53824f0e7ea7e4ac94d1465bb"
branch: "master"
head: "e005952"
---

# Stewardship request — cycle 3

This document is a **request**, not a contract. Per the conductor work order
(run `5d025d55b1194dd1a4dd8784146dfeeb`, phase stewardship, attempt
`8600fea3e7f54703b20f7c84a4f3b489`), it describes the selected maintenance
batch and stops there: **it chooses no branch, worktree, commit order, or
any other Git topology.** Those decisions belong to the conductor.

Routing note: no ce-* skill owns stewardship requests (same gap disclosed
by cycles 1–2, both provenances read this campaign). The narrowest
installed match is **ce-handoff**; this document follows its
`ce-handoff/v1` frontmatter and pointer-first body contracts at the
campaign's user-directed destination (`docs/stewardship/`). Harness
disclosure: this delegate session has no subagent surface; everything here
was produced in-thread.

## Title

agenttrace cycle 3 — "trustworthy strings on untrusted input"
(P6-1 lead + P6-4 + riders P6-2, P6-3, C34; five change units, CU-1..CU-5).

## Summary

Implement the batch selected by the prioritize phase
(`docs/stewardship/2026-09-02-cycle3-prioritization.md`), closing the last
reproducible-panic HIGH on the board and completing the hardening trilogy
(cycle 1 file-parser arithmetic, cycle 2 SQLite ingestion + launch, cycle 3
string repair + estimation):

- **CU-1 — P6-1 (HIGH): `repair_lone_surrogates` panics on non-char-boundary
  slicing.** The function walks by byte index but slices the original
  `&str`: `parser.rs:3785` (`&line[i+2..i+6]`) and `:3791`
  (`&line[i+8..i+12]`). A `\u` escape serde_json rejects, followed by
  multi-byte UTF-8 within four bytes, slices mid-character → exit 101 in
  debug **and release**. Reachable from format detection
  (`parse_jsonl_value_lenient` `parser.rs:3772` ← `jsonl_objects`
  `parser.rs:3762` ← every JSONL detector), so a single hostile line in any
  scanned file or directory kills `--overview`, `--doctor`, `--waste`,
  `--latest`, `--sessions`, `--diagnostics`, positional files, and
  directory scans. Re-verified live this campaign on the current tree
  (three invocation forms, `panicked at parser.rs:3785:28`). Fix: parse
  hex from bytes (collect into `[u8; 4]` only when all four are ASCII hex,
  else leave the escape untouched) or `is_char_boundary`-guard both edges;
  **the fix must bring its own corpus** — the committed adversarial corpus
  contains zero `\u` escapes and the mutation fuzzer never generated one
  before multibyte bytes (proven blind spot). Implementer note: the
  `-d <single corrupt file>` form exits 1 "No session files found" on a
  different path — write regression tests with the **positional-file and
  directory** forms that actually crash.
- **CU-2 — P6-4: honest fallback token accounting.** The bytes÷4 estimate
  under-counts CJK ~40–60% (`lib.rs:555`, `:577`) and `reasoning_chars`
  stores `event.reasoning.len()` **bytes** (`lib.rs:562`). Decide
  byte-vs-char per site, count `chars()` where characters are intended,
  name the estimator in provenance, and rename/fix `reasoning_chars`.
- **CU-3 — P6-2 (rider): `--version` claim integrity.**
  `validate_primary_action` (`main.rs:136`) runs before the version
  early-return (`main.rs:150`) and `--version` is itself an action, so
  `--overview --version` exits 1 while `CHANGELOG.md:11` claims
  "`--version` now wins over argument validation". Either hoist the
  early-return or reword the line; pin `--overview --version` with a CLI
  test either way.
- **CU-4 — P6-3 (rider): unique cache temp suffix.** Cache persist writes
  through a fixed `<name>.json.tmp` sibling (`session_cache.rs:226`, also
  `:518`), so concurrent agenttrace processes race on the same temp file.
  Unique suffix per writer, same atomic rename, concurrent-persist test.
- **CU-5 — C34 (rider): placeholder-title gate.** OpenCode populates
  `title` with `New session - <timestamp>` (227/227 live local sessions)
  and `sqlite_sessions.rs:672-676` uses it verbatim, so every opencode
  session is junk-named today. Treat the placeholder pattern as absent,
  fall back to message-derived naming (the `message` rows are already
  scanned at `sqlite_sessions.rs:419`), stamp naming provenance
  `provider:placeholder`.

Acceptance criteria and evidence expectations per unit are in
`ROADMAP.md` (hardening lane, first two entries + the extended
"CLI surface polish" and "Cache and history durability" entries; capability
lane candidate 34); the red-to-green execution order and full verification
matrix (fixtures first; debug+release tests, fmt, clippy, check scripts,
reproducers exiting 0) are in the cycle-3 prioritization doc.

## Repository candidate

- `luoyectl/agenttrace`, working tree `/work/projects/agenttrace`,
  branch `master`, HEAD `e0059522b4fc74d53824f0e7ea7e4ac94d1465bb`.
- **Dirty state to preserve:** 20 modified tracked files + 12 untracked
  paths (`git status --short`), all from the uncommitted cycle-1/2 work
  plus this campaign's docs. **Overlap fact for planning:** every code/doc
  surface this batch touches already carries uncommitted cycle-1/2 edits —
  `parser.rs`, `lib.rs`, `session_cache.rs`, `sqlite_sessions.rs`,
  `discovery_contract.rs`, `main.rs`, `CHANGELOG.md`. The batch builds on
  that state (baseline 169/169 tests, fmt/clippy clean); it must not be
  clobbered, reverted, or rebased away. No `git worktree` other than this
  tree existed at cycle-2 time (per its implementation record).
- Baseline for verification: `cargo test --workspace` 169/169 (debug and
  release), `cargo fmt --check`, `cargo clippy --workspace --all-targets`
  0 warnings — re-confirmed this campaign on the dirty tree.

## Surfaces (file:line)

| Unit | Surfaces |
|---|---|
| CU-1 (P6-1) | `crates/agenttrace-core/src/parser.rs:3785`; `crates/agenttrace-core/src/parser.rs:3791` (reachability anchors `:3772`, `:3762`); fixture generator `scripts/fixtures/` (new unicode-escape family, sibling of `make-adversarial-sqlite.py`) + `testdata/generated/adversarial/`; test wiring `crates/agenttrace-core/tests/discovery_contract.rs` |
| CU-2 (P6-4) | `crates/agenttrace-core/src/lib.rs:555`; `crates/agenttrace-core/src/lib.rs:562`; `crates/agenttrace-core/src/lib.rs:577` |
| CU-3 (P6-2) | `crates/agenttrace-cli/src/main.rs:136`; `crates/agenttrace-cli/src/main.rs:150`; `CHANGELOG.md:11` |
| CU-4 (P6-3) | `crates/agenttrace-core/src/session_cache.rs:226`; `crates/agenttrace-core/src/session_cache.rs:518` |
| CU-5 (C34) | `crates/agenttrace-core/src/sqlite_sessions.rs:672` (block through `:676`); message-scan input at `crates/agenttrace-core/src/sqlite_sessions.rs:419` |

## Must remain separate

1. `CU-1 parser.rs string-repair fix` ✂ `CU-2 lib.rs token-estimate
   accounting` — different files and defect classes; each independently
   revertable, each with its own corpus/tests.
2. `CU-3 main.rs --version ordering + CHANGELOG.md:11` ✂ `CU-4
   session_cache.rs temp-suffix race` — zero shared surface.
3. `CU-5 sqlite_sessions.rs placeholder-title gate` ✂ `candidate 33
   per-turn model attribution (parser.rs:2161-2164)` — C33 is explicitly
   the **next** cycle's capability lead; it must not ride this batch even
   though it also lives in the parser family.
4. `cycle-3 batch (CU-1..CU-5)` ✂ `dependency motion (Cargo.toml /
   Cargo.lock: ureq 3, rusqlite 0.40, crossterm 0.29 — roadmap N4 item)` —
   the one-motion dependency cycle is its own batch by roadmap rule.
5. `cycle-3 batch (CU-1..CU-5)` ✂ `output-honesty sweep (P3-4 control
   characters, P3-7 newline parity — reports.rs and every output format)`
   — `reports.rs` is untouched by this batch; keep two acceptance
   surfaces out of one change.

## What this batch deliberately does not include

(deferred with rationale in the cycle-3 prioritization doc: P3-1 platform
parity — unverifiable on this host; C33+C25+C24 capability accuracy —
next cycle; P3-4 output honesty; N4 dependency motion; N5/P4-4/P4-5 CI
truth; cache-eviction design.)

---

**Artifacts:** this file. Git topology untouched; nothing committed,
pushed, or CI'd.
