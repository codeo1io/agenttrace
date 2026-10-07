# Cycle 2 compound record — run a0d0d353 (2026-10-07, pre-review)

- **Run:** `a0d0d35371454f90b0299f5c900c6b0a` (repository-maintenance `d44cb46181004f10a9bc2ca76825a9c4`, cycle 2)
- **Base:** HEAD `89911442173d31f4eb21a1968bfd51ad46e32ed1` (unchanged all cycle; everything below is an uncommitted worktree delta in `run-a0d0d3537145-a0d0d353`)
- **Batch:** "SQLite-snapshot cache lane" — rm-626 (lead) + rm-627 (pair), ONE change-unit, guard-first ordering (prioritize attempt 59dcf6de designation; stewardship 88c34196 contract)
- **Status at compound:** implemented, pre-review. Review and shipping happen after this step; the next cycle's assessment carries them forward.

## Attempts and forensics

| Phase | Attempt | Note |
| --- | --- | --- |
| assess | a6790edb (prior 6887769d DEAD) | fresh adversarial audit @8991144 porcelain 0; 2 net-new PoC-proven findings (F1 poison home1, F2 slot home2/home3); 6887769d reaped provider-side ~17min in — event log shows a green full-suite battery then status:failed, NO typed result; its /tmp/at-assess-6887 scratch salvaged as a lane pointer only, phase redone from scratch |
| research | 9514a8e0 | upstream tip 15ed07f frozen (7th check), fork 30 behind / 115 ahead of be25c4c, FIRST full 30-commit port-gap enumeration; #311 workbuddy sum unported; LiteLLM live 4,480 keys vs bundled 3,099 |
| roadmap | 0ed7abcc | minted rm-626/rm-627 (+25 lines); third mint (#311) RETRACTED — full-band grep surfaced sibling 99d1c79c's claim, folded onto rm-450 |
| prioritize | 59dcf6de | batch = the snapshot lane; DESIGNATED notes on both rows (+2) |
| stewardship | e29e7224 (orphaned) → 88c34196 | e29e7224 wrote a COMPLETE result then died transport-reaped (result orphaned); 88c34196 re-established the request with every surface anchor re-verified live |
| implement | 764dedd8 | red-first (3 tests FAILED: clobber left "b"/right "a", schema 7≠8, poison persist), then full guard+ledger implementation, GREEN 323/0 |
| targeted_tests | d84a84f5 + 98b9a2f9 | d84a84f5 green; re-dispatched 98b9a2f9 re-verified every lane live — all reproduce (323/0, fmt/clippy 0, dependents clean, docs gate rc0) |
| full_tests | 257ca470 | 466/0 across 21 binaries + all 21 CI lanes of ci.yml lint+full+deny mirrored locally; zero regressions |
| compound | 21fd30d2 | this record; row flips + banner; ZERO mints; NO test execution (outcomes consumed) |

**Dead-attempt pattern (recurring, new instances):** 6887769d (assess) and e29e7224 (stewardship) join the fleet's transport-reap census. Two refinements this cycle: (1) a green battery tail in a dead attempt's scratch is a LANE POINTER, never adoptable evidence — redo from scratch; (2) a complete result JSON can be orphaned by a reap AFTER the write — the successor attempt re-verifies every anchor live instead of trusting either the artifact or carried summaries. Also: /tmp scratch is swept externally mid-cycle (`/tmp/at-assess-a679` and `/tmp/at-research-9514` were reaped between phases) — pin reproduction recipes in ROADMAP rows and spool JSONs at creation time; treat /tmp as volatile.

## What was implemented (uncommitted delta, 5 files +410/-85 + ROADMAP +27/+6)

- **rm-626 fail-closed snapshot guard** — `crates/agenttrace-core/src/sqlite_sessions.rs`: `open_sqlite_read_only` gains `busy_timeout` (first in-tree use; rusqlite method already available, no dep change) so transient SQLITE_BUSY stops being a silent total failure; `query_hermes_sqlite_sessions`/`query_opencode_sqlite_sessions` now return `Option` (None = the read FAILED) and both load arms skip `store_sqlite_snapshot` on None — a failed read is a MISS, never a store; the opencode leg adds a `sqlite_master` prepare guard (discovered red-first: a garbage "not a sqlite database either" file previously flowed through `opencode_sqlite_session_rows` as empty and WAS stored — the same poison class via a second door).
- **rm-627 per-database snapshot ledger** — `crates/agenttrace-core/src/session_cache.rs`: `SqliteSnapshotLedger` keyed per database path replacing the name-keyed single slot (`"{name}-sqlite.json"`), one-slot migration for existing homes, `SQLITE_SNAPSHOT_SCHEMA_VERSION` 7→8 (governance-guide sentence rider in `docs/guides/governance-reports.md`, `check-docs-commands.sh` green; `SESSION_CACHE_SCHEMA_VERSION` 27 untouched — separation verified). Composes rm-626's guard in the same unit: plural-DB homes now warm (N databases = N entries) AND a failing DB can no longer clobber a sibling's healthy snapshot.
- **Tests (red-first)** — `sqlite_snapshot_ledger_keeps_plural_databases_independently` (pinned the exact PoC clobber: database b's store served for database a's lookup, left "b" right "a"), `failed_hermes_read_never_persists_or_serves_an_empty_snapshot` ('a failed hermes read must not persist any snapshot'), `sqlite_snapshot_schema_eight_round_trips_provenance_and_rejects_older_schemas` (pin Number(7)≠8), healthy-empty-caching leg, opencode-garbage leg (red before the prepare guard).
- **Riders** — CHANGELOG.md Unreleased/### Fixed; ROADMAP.md band (roadmap+prioritize phases; compound appends this cycle).

## Recorded validation outcomes (pre-review; consumed at compound, NOT re-run here)

- **targeted (d84a84f5, re-verified by 98b9a2f9):** `cargo test -p agenttrace-core` 323 passed / 0 failed across 13 suites; `cargo fmt -p agenttrace-core --check` rc0; `cargo clippy -p agenttrace-core --all-targets` 0 warnings/errors; `cargo check -p agenttrace-tui -p agenttrace` clean; `check-docs-commands.sh` rc0. Engine block derived required_scope 'none' (classify_surface crates/** blind spot, fleet constraint #16653) — 'targeted' declared over the derived floor per run-1ecd791b precedent.
- **full (257ca470):** engine `full_command` empty (same blind spot) → suite derived from `.github/workflows/ci.yml` and mirrored step-for-step, ALL 21 LANES GREEN: fmt, clippy `--locked -D warnings`, `cargo test --locked` 466/0 across 21 binaries, release build, entrypoints 31/0, output-contract, deterministic-output, report-semantics, release-surfaces, example-workflows, install-runtime, docs-commands, real-cli-smoke (sampled_files=20), ruby -c formula, npm test, cargo-manifests, plugin-version, script syntax, locked-cargo, install-ref-drift, and `cargo deny check` (advisories/bans/licenses/sources ok). Sole deviation documented: local cargo-deny 0.20.2 rejects the action's legacy `--all-features` CLI flag; `deny.toml:13 all-features = true` makes plain `cargo deny check` the exact equivalent. check-rust-tui-real-smoke skipped exactly as CI gates it (repo variable unset).
- **Digest lineage:** dispatch token `validation:v1:a72443d225dc8d687e34505eb328bd65608884abc60d932d59f83678062b7b0c` declared VERBATIM by targeted AND full, re-derived live with the engine's own `validation_digest()` at both gate turn-ends (base-sha-derived: invariant across the uncommitted crates/** delta; would need re-derivation only if the base moves). Zero regressions, nothing fixed post-implement.
- **Prior cycle's rider RETIRED:** the `check-plugin-version.sh` no-changelog-section marker block (b1ff12f8 cycle-2) landed in-tree — markers present at CHANGELOG.md:359-360 and the gate is green at full_tests with tags fetched. No commit-gate rider outstanding from that cycle.

## Prevention rules (reusable)

1. **Failure-guarded cache stores.** Any cache store path must treat a failed read as a MISS — never persist the failure result. Two red-first tests + the sqlite_master prepare guard pin this for the snapshot cache; new cache families must adopt the same shape (a failure sentinel that load treats as a miss is the allowed alternative).
2. **Schema-bump lockstep.** A `*_SCHEMA_VERSION` bump must move the sentence the docs gate greps in the same change-unit (`check-docs-commands.sh:61` extracts the live const; `docs/guides/governance-reports.md` states it). Const families are kept separate deliberately — `SESSION_CACHE_SCHEMA_VERSION` 27 untouched by the snapshot bump.
3. **Multi-source caches key by source identity, never family name.** A name-keyed single slot is simultaneously a clobber surface (last store wins) and a poison multiplier (one poisoned store masks every sibling database). Per-source keying + fail-closed stores is the composed fix.

## Coordination map (unlanded, for integration)

- **ff0068ca rm-596** (ingestion-honesty band): touches the SAME load/store call sites with `SqliteIngestReport` unreadable/dropped-rows disclosure — compose, don't replace: its unreadable-DB signal is the natural predicate feeding this batch's skip-store guard. Integration resolves the textual overlap (hunk headers @@-57/-154/-182/-203/-273/-354 vs this batch's guard/ledger edits).
- **e5653f52 / c762c2b8** dirty `session_cache.rs` files with disjoint band titles (#312-accounting / subagent-attribution) — title-disjoint, expected to union-resolve.
- **Fleet id frontier at compound:** unlanded bands aa31de94 rm-628..631, 9a4d37af rm-640..646, f6a7e7f3 rm-647/648/650, c90a0f00 rm-655, 7e00d9cb rm-683/684, 2f02ecaf rm-685/686; next free rm-687 after a fresh census at mint time. origin/master advanced to 1c5edd1 (past the roadmap-phase da08338) — expect a rebase seam at the commit gate.

## Next-cycle context (recorded, not decided)

- **rm-006 pricing cadence** — strongest recurring signal: LiteLLM drift re-opened within 3 days of the 10-04 refresh (4,480 live keys vs 3,099 bundled; 3,161 priced). Candidate for an owned refresh cadence or automation row.
- **rm-042 #306 calendar/cutoff period reporting** — reference implementation exists upstream, 0 fork hits at 8991144 (0 `usage_points` refs).
- **rm-386 MSRV** — fork 1.80 vs upstream #303's 1.85, now 5 minors behind; natural rider on the ureq-3 lane if that lands.
- **rm-627 shipping evidence owed** — home3 10-DB warm re-measure at the PR (assess baseline: 6,223 parsed, cache_hits 0 cold AND warm; expect warm cache_hits > 0 after the ledger).
- **Watch negatives re-verified this cycle** — OTel semantic-conventions-genai still zero tags (rm-229 parked); models.dev 226; ccusage v20.0.26 flat; codeburn quiet since v0.9.25; token-monitor quiet.
