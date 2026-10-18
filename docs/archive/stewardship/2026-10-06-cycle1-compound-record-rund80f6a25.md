# Cycle 1 compound record — run d80f6a25 (repository-maintenance 8fbbf166)

- **Run / cycle**: `d80f6a25bbd14913b3bb28d8a5408e32` / cycle 1, at worktree base `700a67c` (4 commits behind origin/master `e389f1a`; base is the merge that landed run 555a174d's cycle 1).
- **Compound attempt**: `f874395c6b0e428cb9c89dde2a201a91` (2026-10-06). Prior-attempt forensics: none needed this phase (first attempt dispatched).
- **Pre-review status**: review and shipping outcomes deliberately absent — they land after this phase; the next cycle's assessment carries them forward.

## Phase lineage (attempts, in order)

| phase | attempt | note |
|---|---|---|
| assess | b6c398bf | adversarial whole-tree review, ~20 live probes; base envelope 17 suites / 410 / 0 |
| research | 515de7fb | three-home no-repeats census + live external probes; dossier C73–C76 |
| roadmap | ecf17bfd | patch-artifact convention (enforcing-hygiene rider); minted rm-533..536 + 6 dated riders |
| prioritize | 65329729 | selected batch rm-533 + rm-534 + rm-239 rider arms, theme "CLI boundary integrity" |
| stewardship | f5ff0564 | one change unit (~5 files), no Git topology chosen |
| implement | 42234fef | 5 files, +236/−4 tracked + 93-line new lane-contract suite |
| targeted_tests | 04f72caf | focused lanes 255/0 (attempt 1dd83ce4 died provider-dead at message 3 — nothing durable, redone from scratch) |
| full_tests | 48fb81b6 | ci.yml full+deny verbatim mirror, 21 lanes rc0 after one sanctioned fix |

## The batch (implemented, validated pre-review)

1. **rm-533 — demo-lane hermeticity** (`crates/agenttrace-cli/src/main.rs`): `validate_primary_action` now rejects `--demo --preserve-history` with a flag-naming error (rc=1) before any load/render/write. Arm B of the row's acceptance: demo replaces the whole session set with fabricated rows, so preserve-under-demo never had legitimate meaning — refusing beats silently ignoring. Asses PoC had fabricated rows (4 sessions, $0.8950, health 100/100) reaching real `history.json` and resurfacing via `--overview --include-history`.
2. **rm-534 — `-o` input-journal collision** (`crates/agenttrace-cli/src/main.rs`): `ensure_output_is_not_an_input` + `resolve_output_destination` at the two loader choke points (`prepare_explicit_sessions` and the directory-walk arm). Collision exits 1 naming both roles; the check runs against the ACTUAL loaded session set. Asses PoC had `--overview -o X.jsonl X.jsonl` rc=0 with the positional journal atomically replaced by the text report.
3. **rm-239 rider arms — shared-terminal sanitizer** (`crates/agenttrace-core/src/reports.rs:1469`, `waste.rs:366`): the `--compare` session-name cell and the `--waste` tool-name line now route through the landed shared `sanitize_line_segment` (rm-383 family). Asses live PoC flipped compare 1→0 / waste 1→0 raw `ESC ]52;c`. New suite `crates/agenttrace-core/tests/compare_waste_sanitization_contract.rs` (93 lines) pins both lanes. rm-239 itself stays candidate: overview text timeline + by-model sites (`reports.rs:851/:936/:953/:967`) remain raw; md/html siblings owned by rm-348/rm-506.

Test deltas: +1 suite / +6 tests (4 new `launch_guards` end-to-end pins + 2 lane contract tests).

## Validation outcomes (consumed as recorded — NO test execution at compound)

- **targeted 04f72caf**: focused lanes over the 5 changed surfaces — fmt clean, clippy 0 warnings, 255 tests / 0 failed.
- **full 48fb81b6**: ci.yml `full`+`deny` jobs mirrored verbatim (dispatch `full_command` is empty for this crates/** layout — engine `classify_surface` matches only root prefixes; run db6b76260a11 precedent). 21 lanes rc0: 18 suites / **416 passed / 0 failed**; 8 release-binary artifact gates; homebrew / npm / cargo-deny green. ONE fix during the phase: `check-plugin-version.sh` red on merged fork tags v0.7.2–v0.8.0 — the known tag-complete-clone artifact (#16652 lineage), remedied by appending the ratified `no-changelog-section` marker block to CHANGELOG.md (byte-identical 5th carrier, md5 `e7a2784e199879d6b9ef5314c87e35dd`).

## Prevention rules (reusable lessons from this cycle)

- **PR-1 — loader choke points own input/output collision guards.** `write_output` has 10+ callers with varied session provenance; a membership check inside it cannot know which files are "inputs". Any NEW entry lane that feeds `write_output` (explicit paths, `-d` walks, demo) must resolve its destination through `ensure_output_is_not_an_input` against the loaded session set before any write. Guard placement is a review surface: check the caller, not the helper.
- **PR-2 — fabricated-data lanes must fail closed on host-state flags.** When a lane fabricates its whole dataset (`--demo`), any flag implying merge-with-host-state (`--preserve-history`, cache-touching flags) is semantically void — reject the pair at `validate_primary_action` with an error naming both flags (the rm-341 piped-error pattern), never silently ignore. Audit in PAIRS whenever a new fabrication lane or a new host-state flag lands.
- **PR-3 — terminal sanitization is a shared-helper contract, not per-site.** Any content-derived string printed by a default text-mode lane routes through `sanitize_line_segment` (never a per-site copy, never raw `truncate_runes` output), and each lane carries a contract test with OSC-52 + CSI fixtures asserting zero raw control bytes while JSON siblings stay byte-identical. rm-239's acceptance ("ANY default text-mode lane") is the umbrella; the overview-text sites are the known residual.
- **PR-4 — repo full-validation = ci.yml full+deny verbatim mirror.** With an empty dispatch `full_command` (expected for this layout), the authoritative suite is `.github/workflows/ci.yml`'s `full` job step-for-step plus `cargo deny --all-features check` (flag order corrected for local cargo-deny 0.20.2). Always: private `AGENTTRACE_CI_OUT` mktemp dir (the scripts share `/tmp/agenttrace-ci` otherwise), never `git fetch` in gates, skip CI-conditional lanes (TUI real smoke repo-var, lint job) with documentation.
- **PR-5 — CHANGELOG marker carrier discipline.** The `no-changelog-section` block for fork tags v0.7.2–v0.8.0 is byte-frozen (md5 `e7a2784e…`); every full-suite run on a tag-complete clone before it reaches master re-appends it as an uncommitted tail. Whichever commit gate lands first carries the block; the others must dedupe byte-identically (never re-type it). Fork GitHub CI stays red on the per-tag arm until the block reaches master.

## Wall accounting (this compound)

- FLIPS candidate→implemented: **rm-533, rm-534** (EXECUTED bullets transcribed from recorded pre-review evidence; review/shipping pending).
- rm-239: stays candidate, cycle-1-arms bullet appended (partial execution; residual = overview text sites).
- rm-535: cycle-2 lead recommendation appended (fully grounded at this base, self-contained in `sqlite_sessions.rs` + doctor rows, zero merge-forward dependency).
- NO new mints: def rows 164 both before and after; next free stays **rm-537** (cb38b958 queued C77/C78), then rm-539.

## Next-cycle context (for cycle 2 assessment)

- **Recommended lead: rm-535** (hermes profiles doctor/double-count) — see its rider for the exact seam (`sqlite_sessions.rs:103-114` vs `:60-78`).
- Alternates: rm-536 (SVG usage card — wants rm-239's sweep complete first); rm-239 residual (overview text sites `reports.rs:851/:936/:953/:967`); rm-506 sibling arms (overview md/html/csv sanitizer) once the ceiling merges forward past this base.
- Watch items carried from research: upstream luoyuctl/agenttrace quiet at `706bf58` (v0.10.1); codeburn #1640 (SVG card) OPEN; LiteLLM live 4,473 models / 116 flex rows; npm `agenttrace` name still free.

## Commit-gate manifest (expected worktree census at that gate)

- `4 M` batch code: `crates/agenttrace-cli/src/main.rs`, `crates/agenttrace-cli/tests/launch_guards.rs`, `crates/agenttrace-core/src/reports.rs`, `crates/agenttrace-core/src/waste.rs`.
- `1 M` `CHANGELOG.md`: sanctioned marker tail (verify `tail -c 1731` md5 `e7a2784e…`) + the 3 Unreleased/Fixed batch riders at the top of the section.
- `2 ??`: `crates/agenttrace-core/tests/compare_waste_sanitization_contract.rs`, `docs/stewardship/2026-10-06-cycle1-compound-record-rund80f6a25.md` (this file).
- `ROADMAP.md` stays PRISTINE in the worktree — apply the CUMULATIVE patch `/tmp/at-compound-d80f/roadmap-run-d80f6a25-cycle1-compound.patch` (mirrored to spool `f874395c-scratch`); it supersedes the roadmap-phase patch `/tmp/at-roadmap-d80f/roadmap-run-d80f6a25-cycle1.patch` — apply ONE, never stack. Postimage alongside for byte-verification (`ROADMAP.postimage-compound.md`).
- One commit for the whole batch (code + tests + CHANGELOG + ROADMAP + record doc).

## Artifacts

- Cumulative roadmap patch + postimage + base: `/tmp/at-compound-d80f/` (mirrored to `/home/agent/.hermes/conductor-delegate-spool/delegate/f874395c6b0e428cb9c89dde2a201a91-scratch/`).
- Validation logs (consumed evidence): `/tmp/at-targeted-d80f/`, `/tmp/at-full-d80f/`.
