# Cycle 1 compound record — run 0583fb6e (2026-10-07, pre-review)

- **Run:** `0583fb6ef1244b8f9654eabeca05caf6` (repository-maintenance `78307520b25c4b5a92e3f8e084c57254`, cycle 1)
- **Base:** HEAD `ee67b22ff222dce4ad458ce8e47daeda12adacb1` (unchanged all cycle; everything below is an uncommitted worktree delta in `run-0583fb6ef124-0583fb6e`)
- **Batch:** "Truthful aggregates, valid exports, honest CLI contract" — rm-607 (LEAD) + rm-605 + rm-608, one coherent batch, mutually file-disjoint units (stewardship 882894b1)
- **Status at compound:** implemented, pre-review. Review and shipping happen after this step; the next cycle's assessment carries them forward.

## Attempts and forensics

| Phase | Attempt | Note |
| --- | --- | --- |
| assess | 0c74979f | fresh adversarial audit at ee67b22; F1-F4 PoC-proven; envelope 21/21 suites, 463/0, fmt/clippy rc0 |
| research | a67f1b51 | HEADLINE CORRECTION: fork pins ureq 2.12 (not 3.4.2) — prior dossier/memory claim false, memory #16822 corrected; dep-drift table; dossier `/tmp/at-assess-0c74/research-dossier.md` |
| roadmap | 85f2ad71 | minted rm-605..rm-609 (+60/-0 ROADMAP delta, commit-gate reserved) |
| prioritize | bfca9f43 (dead) → ab5c79ff | selection memo `/tmp/at-prioritize-ab5c/selection-memo.md`; dead attempt = 4-line event log, no typed result, zero tree drift |
| stewardship | 882894b1 | companion `/tmp/at-stewardship-882894/stewardship-companion.md`; fleet dirtiness re-census (integration-161fd4b514d0 staging otel.rs verified hunk-disjoint from span-id derivation) |
| implement | 34358c3b | red-first, 20 files +461/-41 (19 code + ROADMAP carry) |
| targeted_tests | a98200679e (dead) → 9ab5dcd5 | dead attempt = 4-line event log, tree byte-identical to implement's end-state; logs `/tmp/at-targeted-9ab5c/` |
| full_tests | 5da13f42 | 22-lane ci.yml mirror; runner + logs `/tmp/at-full-5da1/` |
| compound | b6b66008 | this record; ZERO test execution per the compound contract |

**Dead-attempt pattern (recurring, third observed family):** bfca9f43 (prioritize) and a98200679e (targeted_tests) both died provider/infra-side with no result envelope. The reliable discriminator stays the porcelain census: a `git status --porcelain` + `git diff --stat` identical to the prior phase's recorded end-state proves the dead attempt changed nothing, so the phase is redone from scratch. See `docs/stewardship/2026-10-06-cycle2-compound-record-runb1ff12f8.md` for the pattern's first write-up.

## What was implemented (uncommitted delta, 21 files +462/-42 at compound)

- **rm-607 cross-db duplicate-session dedup (LEAD)** — `Session.sqlite_session_id` (serde skip-if-empty, set only at `sqlite_sessions.rs` `session_from_sqlite_agg` — field mapping :858/:888, identifier corrected per review 5968db61 F2) so the dedup identity survives the snapshot cache; `SQLITE_SNAPSHOT_SCHEMA_VERSION` 7→8 (pin test re-based to v8-round-trip + v7-reject; `SESSION_CACHE_SCHEMA_VERSION` untouched at 27 — cached aggregate semantics did not change); `SqliteLoadReport`/`SqliteDuplicateDb` + canonical-first glob ordering (exact `opencode.db` / `state.db` + `profiles/*/state.db` before prefixed siblings) in `opencode_db_paths` — `hermes_state_db_paths` was already primary-first structurally (`vec![primary]` then profiles, no edit needed, the ordering property holds); `load_sqlite_backed_sessions_report` dedups by `(source_tool, sqlite_session_id)` keeping the canonical db's copy; `DoctorReport.sqlite_duplicate_dbs` (skip-if-empty) rendered "Duplicate sqlite databases (rm-607): <path> — N duplicate session(s) suppressed, canonical db kept"; all 30 `Session` literals across core/cli/tui seeded. Red-first: `discovery_contract` "left: 4, right: 2" (backup db doubled the count, cold-cache).
- **rm-605 OTel span-id validity** — `span_id_for` hashes `'agenttrace-span'+seed` via `DefaultHasher` exactly like `trace_id_for` (16-hex), clamps the astronomically-unlikely all-zero hash to 1, doc comment cites the OTLP/W3C reserved all-zero id; golden `span_ids_are_valid_nonzero_and_deterministic` pins non-zero AND determinism. Red-first: "reserved all-zero span id on an export span (rm-605): left 0000000000000000".
- **rm-608 doctor runs the -d guard** — the landed Cycle-4 B2 guard extracted to `validate_explicit_dir` (semantics unchanged) and the doctor arm routes through it BEFORE `render_doctor_report` (rc=2 on nonexistent/non-dir `-d`); `doctor_dir_guard_exits_two_like_other_lanes` pins both bad shapes. Red-first: entrypoints doctor rc mismatch.
- **Docs truth-fix (full_tests)** — `docs/guides/governance-reports.md:72` "SQLite snapshot is schema 7"→"schema 8": lane 14 `check-docs-commands.sh` red because the schema bump had not carried the grepped sentence; prevention rule durabled at `docs/solutions/workflow-issues/persisted-schema-version-bumps-must-carry-the-docs-gate-sentence.md` (second observed instance of the class after cf755698 c2).

## Recorded validation outcomes (pre-review; consumed at compound, NOT re-run here)

- **implement (34358c3b):** green core 329/0 (all suites), cli bins 43/0, entrypoints 32/0, tui 47/0; fmt rc0; clippy `--workspace --all-targets -D warnings` rc0. PoC replays on the persisted sandboxes: h1 vs h2 both "aggregates cover all 309 sessions" (was 309→618 with every aggregate doubled); `--doctor` on h2 discloses "opencode-backup.db — 309 duplicate session(s) suppressed, canonical db kept"; `--doctor -d /nonexistent/xyz` rc=2; `-d README.md` rc=2; otelprobe span[0] `spanId=1a89542530667bdf` (non-zero, distinct, deterministic). ENV footnote: real-HOME `--doctor` stalls ≥5 min under current host load identically on the PRE-CHANGE sibling binary — pre-existing/environmental, not batch-introduced.
- **targeted (9ab5dcd5):** fmt rc0; clippy `--workspace --all-targets -D warnings` rc0; core lib 191/0; discovery_contract 82/0; otel_export 4/0; demo_contract 7/0; cli bins 43/0; entrypoints 32/0; tui 47/0; porcelain 20 M in==out (validation-only turn).
- **full (5da13f42):** dispatch `full_command` EMPTY → suite derived VERBATIM from `.github/workflows/ci.yml` (`full` job + `lint` gates + `deny` job) per the db6b76260a11 fleet convention; 22 lanes via `/tmp/at-full-5da1/run-full.sh`, ALL rc0, incl. `cargo test --locked` 473/0 across 21 suites (debug AND release), entrypoints 32/0, `cargo deny --all-features check`; documented skips: TUI real-smoke (CI-gated on unset repo var), `git fetch --tags` pre-gate (never-fetch), `check-rust-release-local.sh` (strict subset of lanes 05/06/07/22). Lane 14 red→fix→green as above; the one-character docs edit is inert for the other 21 lanes.
- **Digest lineage:** `validation:v1:042d34fd24695cdeed29270729f9ad24bec604146944e8f2cd11850aa15e6f3e` re-derived with the engine's own `validation_policy.validation_digest('ee67b22…', worktree)` on BOTH the pre-run and post-fix trees — `docs/` paths are not executable-classified, so the token stays provably current through this compound's ROADMAP/docs edits too.

## Fleet collision registry (commit gate — must_remain_separate disclosures)

1. **rm-605 × 8abf79ed rm-568 band + integration-161fd4b514d0:** same files `otel.rs` + `tests/otel_export.rs`, DISJOINT subject (id derivation vs system attribution / finite-attr sanitization); their tests assert attributes only → union-mergeable.
2. **rm-607 × 9873fc06 rm-545..550 opencode-fork band:** `discovery.rs` same function zone — their delta rewrites the exact `sessions.extend` call into a tuple-return; this dedup inserts after it. `sqlite_sessions.rs` same file, different functions. Same-file-disjoint with 6aaf51aa, de96d4cc, integration-ca7eeeabb1a6.
3. **rm-608 × main.rs-dirty siblings:** doctor-arm zone untouched by all seven dirty lanes — verify again at the gate with a fresh census.
4. **rm-606 numeral:** reconciles BY TITLE vs 8abf79ed's rm-568 (implemented-uncommitted twin) — do not double-implement; retire/rebind the numeral at integration per the 96762b67/b33b22279a precedent.
5. **Schema slot — RESOLVED DIFFERENTLY THAN FORESEEN:** implement bumped `SQLITE_SNAPSHOT_SCHEMA_VERSION` 7→8 (the sqlite snapshot constant; the guide sentence realigned), NOT `SESSION_CACHE_SCHEMA_VERSION` (stays 27); the prioritize-time worry about the 36c9140f lineage's slot-28 claim on the session-cache constant therefore does not interact with this delta.

## Commit-gate checklist

- ONE commit: 19 code/test files + `docs/guides/governance-reports.md` digit + ROADMAP.md (mint + compound delta) + CHANGELOG Unreleased entries (1 Fixed family covering the three units) + this stewardship record + the prevention-rule doc.
- done-flips for rm-605/rm-607/rm-608 (status implemented→done) reserved to the commit gate per rm-012.
- Re-run the def-row and title-twin sweeps live at the gate (fleet moved during the cycle).

## Next-cycle context (concrete candidates, priority order)

1. **rm-007 dependency-refresh wave — natural cycle-2 lead.** ureq `2.12.1`→`3.4.2` with upstream #318 as the reference migration (sole call site `crates/agenttrace-core/src/pricing.rs:673`; the 2→3 port changes the agent API — dedicated cycle, full-suite budget); lru 0.18.1 RUSTSEC-2026-0253 UAF advisory (fix 0.18.2/max 0.18.5) is the recorded promotion trigger; rusqlite 0.40.2 and the itertools 0.15/sha2 0.11 additions ride the same wave. Cargo.lock is textually contested by sibling bands — sequence, never fold.
2. **rm-609 month-pace projection** — deferred lead; interlocks with rm-385's budget/threshold lane; reports.rs/main.rs are the fleet's most-contested surfaces, re-census before implementing.
3. **rm-606** — moves ONLY if 8abf79ed's rm-568 twin fails to land (cede recorded on the row).
4. Watch items carried: upstream frozen at `15ed07f2` (v0.10.1, #318 — no new commits rechecked live at research); LiteLLM field-set mutates intraday (4480/3348/233/128/51 vs same-day 215/108/49) — do not pin mid-cycle; models.dev api.json now requires a UA header (ops footnote); OTel semconv-genai still zero tags.

## Repro sandboxes (persist across delegate sessions; /tmp is sweep-volatile — re-verify before citing)

- `/tmp/at-assess-0c74/` — h1/h2 sqlite-dedup PoC HOMEs (byte-identical backup db), `otelprobe/` span-id probe crate (path-dep on this worktree), `assessment.md`, `research-dossier.md`, `litellm-now.json`, `modelsdev.json`.
- `/tmp/at-targeted-9ab5c/` — targeted gate logs (fmt/clippy/core-lib/core-integ/cli/tui).
- `/tmp/at-full-5da1/` — `run-full.sh`, `lane-results.txt`, per-lane `logs/`, `ci-out/`.
- Durable: the delegate envelopes under `/home/agent/.hermes/conductor-delegate-spool/delegate/` (roadmap 85f2ad71, stewardship 882894b1, implement 34358c3b, targeted 9ab5dcd5, full 5da13f42).
