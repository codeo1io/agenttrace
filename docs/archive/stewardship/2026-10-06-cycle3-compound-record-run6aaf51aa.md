# Cycle 3 compound record — run 6aaf51aaa919 (repository-maintenance 92476171)

- Date: 2026-10-06. Phase attempt `9ecdbc87`. Worktree `conductor/run-6aaf51aaa919-6aaf51aa` at HEAD `ea5c41e` (wall base sha `0d3ac4ac`), branch uncommitted throughout.
- Deliverable of record: `rm-502` — Naive-ISO timestamps parse for `--overview` but vanish from `--sessions`/`--diagnostics`. Direction (a) unify-accept per prioritize `9225ea80`: every strict RFC3339 consumer routed onto `lib.rs parse_ts` (single source of timestamp truth); the lenient ingest arm untouched behind its pinned Go-parity contract.

## Implementation record (implement `9e4c079c`, transcribed — not re-run)

- 15 tracked files +406/−66; untracked twin fixtures `crates/agenttrace-core/tests/fixtures/naive-iso/{naive,aware}.jsonl`.
- RED-first evidence both sides of the fix: core `discovery_contract naive_iso_twin` failed at HEAD (`tool latencies must be identical: [] vs [read_file count 1 avg 1.0]`) and passed after; the TUI explorer timeline test failed at HEAD `explorer.rs`, passed with the patch (`explorer-rm502-fix.patch`).
- Pinned Go-parity contract `naive_iso_timestamps_match_go_utc_gap_rules` (lib.rs) green verbatim — zero edits to the lenient arm.
- `SESSION_CACHE_SCHEMA_VERSION` 22→23 moved at all four pin sites: `session_cache.rs:8`, `discovery_contract.rs` (x2), `agenttrace-tui/src/tests.rs` warm fixture, plus the `docs/guides/governance-reports.md` "schema 23" sentence (docs gate green).
- Live twin-corpus PoC: naive corpus now yields `tool_latencies [read_file 1x1.0s]`, step durations 1.0 — identical to the aware control; `--overview` phrase reports "1 naive-UTC timestamp sessions" (`data_health.naive_utc_sessions`).

## Validation outcomes (targeted `a5cdf60f`, full `4d0db5aa` — transcribed, not re-run)

- Targeted: 372/0 across the three impacted crates (core 258, tui 47, cli 67); fmt, clippy (`-D warnings`), docs-commands, output-contract, deterministic-output, report-semantics all rc0.
- Full (`cargo test --workspace --release` + repo CI lanes from `.github/workflows/ci.yml` lint/full/deny): 372/0 across 13 binaries — assess baseline was 368/0 on the same 13 at base; +4 = the new twin tests. Eleven of twelve `scripts/ci` gates + `cargo deny check` rc0; TUI real-smoke skipped per its own unset `AGENTTRACE_TUI_REAL_CI_DIR` repo-var gate (CI-faithful).
- The twelfth gate, `check-plugin-version.sh`, rc1 — pre-existing and proven delta-independent (below). Recorded as a full_tests finding, never patched at the gate.
- Envelope digest `validation:v1:d6221bb9…` declared VERBATIM at both gates and re-derived byte-exact post-battery with the engine's own `validation_digest` at base **full sha** `ea5c41e47b04…` (short shas do not reproduce). `crates/**` sit outside the digest's executable-surface set, so the digest never moved.

## Lessons and prevention rules (this cycle's net-new)

1. **Tag-complete environments flip `check-plugin-version.sh` red** — the rm-303-era per-tag arm demands a CHANGELOG heading or marker for every merged `^vX.Y.Z` tag; fork origin carries inherited upstream tags v0.7.2..v0.7.7 + v0.8.0 with neither, so the fork's own GitHub CI is red after its `git fetch --tags`, and every prior local green was vacuous under the sparse local tag namespace. Prevention rule: `docs/solutions/workflow-issues/tag-complete-environments-flip-plugin-version-gate.md`. Roadmap folds: rm-163 (dated append) + rm-018 (notes append). **Never backfill CHANGELOG inside a validation gate.**
2. **A `git fetch --tags` in any worktree mutates the shared canonical gitdir** (packed-refs rewrite) — fleet-visible, not run-local. Disclose at the gate that flips it; pinned as project memory #16697.
3. **Bare `--doctor` auto-discovery needs its own ≥30-min budget on a cold cache** — it consumed a shared 1800s timeout and killed an unrelated lane's first attempt; docs gate and real-cli smoke gate got separate timeouts and both passed.
4. **Dead-attempt forensics protocol works**: prior targeted attempt `e937bae6` (provider-reaped at message_count 8, typed artifact absent, no scratch dir, event log 5 lines) had zero durable work — verified, nothing adopted, phase redone. Adopt only against identity + tree census + cleanliness legs.
5. **Empty `full_command`/`targeted_command` in the dispatch block** (PCR-9 family): both validation phases discharged by repo authority (ci.yml lanes + the run's assess-envelope command). Process note stands for the engine.

## Next-cycle context (concrete, pre-review evidence only)

- **Lead**: the tag-hazard reconciliation — backfill the 7 markers with authored reasons OR rescope the per-tag arm to fork-minted tags; sequence with rm-018's mirror wave so the fork's first post-change commit is green. Fold, no new id.
- Assess F1 (by_task_type i64 overflow, `lib.rs:1237` plain `+=`) and F2 (whole-file `fs::read`, 1.28 GiB RSS on a 97.5 MB corpus) remain ceded to fleet rows rm-445/446 + rm-448 — verify their landing state at the next base before re-selection.
- rm-017's moved-distribution watch now carries a live npm registry artifact (`@zack78/agenttrace` 0.10.0) — research f4981da8.
- Fleet numbering at compound: in-tree ceiling rm-502 (this run's uncommitted mint); canonical committed ledger max rm-484; fleet reservations run to rm-544 → any new mint starts at rm-545 **after a live census** (per-worktree grep + git-show + spool mint-verb sweep; dashboard-repo numerals excluded by repo-content scoping). CORRECTED 2026-10-06 (review 6f96ac6f): rm-545 was already stale at review time — canonical conductor branches @ 8991144 hold walls through rm-545 (next-free >= rm-546); re-derive by live census at the moment of minting, never from this snapshot.
- Commit gate sequencing: apply roadmap patch `4b853965` (+15) then this compound postimage; ship the 15-file delta + twin fixtures + this record + the solutions doc as one unit behind the review outcome.
