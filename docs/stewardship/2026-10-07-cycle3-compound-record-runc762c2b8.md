# Cycle 3 compound record — run c762c2b8 (repository-maintenance 0937d5fb)

- date: 2026-10-07
- run: c762c2b86baf4367a6962f426ebeff6c (repository-maintenance:0937d5fbbaa94f9bac28b5aa4925135e:cycle:3)
- worktree: run-c762c2b86baf-c762c2b8 @ 89911442173d31f4eb21a1968bfd51ad46e32ed1, porcelain in = 12-path implement band (9 M + 3 ??) + the uncommitted roadmap payload on ROADMAP.md; compound attempt 0c932e66a9074137b855d91415f3fce1
- contract: pre-review compounding only — assessment, research, roadmap, prioritization, implementation, and recorded test outcomes consumed as evidence; ZERO test/validation commands executed at compound; review and shipping outcomes deliberately absent (they land after this phase; the next cycle's assessment carries them)

## Batch

"Claude Code session truth + honest CLI contract", selected by prioritize c949 (adopting b2987416's durable memo with two amendments):

| id | track / priority | subject | disposition at compound |
| --- | --- | --- | --- |
| rm-545 (LEAD) | upstream-sync 74.0 | port upstream #305: attribute Claude Code subagent cost/tokens to the parent session | flipped candidate → implemented, EXECUTED bullet on the row |
| rm-489 | reliability 70.0 | `-o` must write special destinations without destroying them | flipped candidate → implemented, EXECUTED bullet on the row |
| rm-207 | developer-experience 62.0 | document the CLI flag surface (46-flag row; drifted to 51 fields) | flipped candidate → implemented, EXECUTED bullet on the row |

Done-flips stay reserved for the commit gate (rm-012 convention). rm-600 (minted this cycle by roadmap 2760e8b1, customer-experience 68.0) stays candidate — it is the ready next-cycle lead, not implemented work.

## Phase inputs consumed (recorded outcomes, not re-run)

- **assess 09e8fcce** @8991144 (after provider-dead 848807ab, which left no typed result and four repo-root scratch files — p3.err/p3.json/p3b.err/p3b.out — deleted at attempt start): 45/51 blank `--help` descriptions (F1, later folded to rm-207), per-session identity disclosure gap (F2 → minted rm-600), base battery 463/0 across 21 result lines, fmt/clippy/deny/release gates rc0. Dossier /tmp/at-assess-09e8/.
- **research 8903081f** @8991144: corrected assess F1 to a repeat of landed rm-207 (assess no-repeat grep was `head -8`-truncated); ecosystem — codeburn #1645 CLOSED with `sessions --id <uuid> --why` shipped, #1647/#1648 Command Code `~/.commandcode` integration open, token-monitor 2636★, ccusage f5a30078 models.dev snapshot; crates at/near pins (ureq 3.4.2, rusqlite 0.40.2, crossterm 0.29.0, clap 4.6.7, ratatui 0.30.2); npm @zack78/agenttrace 0.10.1 flat. Dossier /tmp/at-research-8903/.
- **roadmap 2760e8b1**: +18/-0 uncommitted ROADMAP.md delta; minted rm-600; wall 198 → 199 def rows. Diff /tmp/at-roadmap-2760/roadmap.diff.
- **prioritize c949**: adopted provider-reaped b2987416's /tmp/at-prioritize-b298/selection-memo.md (verified against this work order's identity/tree/wall before consuming) with two amendments; batch as above.
- **stewardship f887**: implement surface = this worktree @8991144 (canonical /work/projects/agenttrace @ea5c41e lags and carries a foreign uncommitted band — reference-only); upstream port source 4506b7e byte-verified (8 files +197/−4); fork has no locales dir, so upstream yml keys became text() literals; rm-489 defect re-proven live at HEAD.
- **implement 6a30** (chain 22ad4f43 → 3d053f1b infra no-op → 07756ff4 → 6a30): the only fold defect across the chain was the missing KTD13 `validation_evidence.changed_surfaces` attestation; 6a30 replicated the fold gate with the engine's own code (17 changed surfaces, testable = scripts/ci/check-docs-commands.sh). Gates fmt/clippy×2/release/docs-gate rc0; impacted 364/0. Live PoCs: subagent TSV (SUBAGENTS/SUBAGENT_COST; 'research x' 2/$0.0053, 'do subtask 1' 1/$0.0004, orphans 0/$0.00), `-o /dev/null` rc0 + 'Saved: /dev/null' + 3808B stdout, `--help` 49 entries 0 blank. Memo /tmp/at-impl-6a30/implement-memo.md.
- **targeted_tests 6917**: validation-only; porcelain byte-identical in/out; 268/0 (core lib 186, subagent_attribution 4, entrypoints 34, cli bin 44); digest re-derived byte-equal with the engine's own code.
- **full_tests 63a0**: dispatch `validation.full_command` EMPTY → ci.yml full+deny mirrored lane-for-lane (53c9af4d precedent); pass 2 = 21 lanes executed rc0 + lane 14 condition-skipped (unset AGENTTRACE_TUI_REAL_DIR, CI parity), 03-tests 22 result lines / 473 passed / 0 failed (base 463/0); pass-1 forensic + fix below. Record /tmp/at-full-63a0/full-tests-record.md.
- **Digest lineage**: validation:v1:a9b25d1cb00964de93c0437ee9a6bf0d0ae42af7895e82bff9a8b08d260a8578 — declared VERBATIM at targeted and full, re-derived byte-identical three times (pre-fix, post-fix, post-sweep). Compound touches ROADMAP.md and docs/** only — outside the engine's executable set — so the digest cannot move at compound.

## Dead-attempt registry (this cycle)

| attempt | phase | fate | disposition |
| --- | --- | --- | --- |
| 848807ab | assess | provider-dead; no typed result; 4 repo-root scratch files | redone from scratch (09e8); scratch deleted |
| b2987416 | prioritize | provider-reaped 12:23Z; no typed result, but selection-memo.md durable at 12:21:46 | memo ADOPTED with two amendments (c949) |
| 22ad4f43 / 3d053f1b / 07756ff4 | implement | completed work / infra no-op / fold-rejected on attestation only | band adopted; attestation repaired (6a30) |
| f2e7de5a | compound | provider-dead at +27s; 2 messages; event log = start + 1 ping + reap; zero durable work | redone from scratch (this attempt, 0c932e66) |

Lesson reaffirmed (fleet pattern): the event log, not the phase evidence, is the complete attempt registry; an absent envelope is not proof no work happened, and a durable non-envelope artifact (b2987416's memo) can be adoptable after identity/tree verification.

## Prevention rules (this cycle)

- **PR-1 (cites standing fleet PR-1/PR-2, third+ fleet hit):** every implement fold carries `validation_evidence.changed_surfaces` derived with the engine's own code on the first attempt. This cycle burned two delegate attempts on an attestation-only rejection with zero code defect. Standing text: docs/stewardship/2026-10-05-cycle1-compound-record-run96762b67.md.
- **PR-2 (new doc):** integration tests that spawn the release binary must guard against its absence — cargo's artifact GC evicted `target/release` between phases and turned a green band into a pass-1 panic. Doc: docs/solutions/workflow-issues/release-binary-integration-tests-must-guard-against-artifact-gc.md (fix landed in tests/subagent_attribution.rs).
- **PR-3:** no-repeat / prior-art sweeps must read full output — assess 09e8's `head -8`-truncated grep manufactured a "net-new F1" that landed row rm-207 already owned; the owning row was found only by research's full-wall read. A net-new claim cites its sweep's coverage, never a truncated window.
- **PR-4:** when dispatch `validation.full_command` is empty, the repo's own ci.yml (full + deny jobs) is the suite authority, mirrored lane-for-lane — recorded here as the second in-worktree confirmation of the sibling-lane rule (fb1addd5's vacuous-validation-block doc owns the general form; reconcile by title at integration).
- **PR-5:** repo-root scratch from dead attempts is collected at the next attempt's start (848807ab's p3.* files) — scratch lives under /tmp or the delegate spool only.

## Commit-gate payload (one commit; review/shipping follow)

- 9 modified tracked files (README.md, ROADMAP.md, main.rs, entrypoints.rs, discovery.rs, lib.rs, session_cache.rs, presentation.rs, check-docs-commands.sh) + 3 new paths (subagents.rs, tests/subagent_attribution.rs, tests/fixtures/subagent-corpus/) — the implement band, untouched by compound.
- ROADMAP.md now also carries this cycle's roadmap delta (+18, minted rm-600) and the compound delta (banner + 3 flips + 3 EXECUTED bullets).
- docs/stewardship/2026-10-07-cycle3-compound-record-runc762c2b8.md and docs/solutions/workflow-issues/release-binary-integration-tests-must-guard-against-artifact-gc.md (this record's two new docs).
- CHANGELOG.md deliberately untouched at compound — the commit gate owns the riders.

Gate actions performed at commit (2026-10-07, attempt dd70a563): three Fixed riders written under Unreleased (rm-545/rm-489/rm-207, house bullet format); rm-545/rm-489/rm-207 flipped implemented → done per the rm-012 convention (status mix after: 126 candidate / 47 implemented / 40 done).

## Next-cycle context

- Leads: **rm-600** (68.0; consumes rm-407's title primitive; codeburn #1645 parity shipped in the ecosystem leader), **rm-449** (82.0; ~5-line qwen dual-key port + fixture — cheapest correctness win, deferred at prioritize as off-theme), **rm-407** (title primitive; dated note appended this cycle).
- Sibling uncommitted bands to sweep at next assess (all 8991144-class bases): 460d0633 (parser.rs/doctor.rs/reports.rs/lib.rs/parser_property_invariants.rs/hostile_journal_disclosure.rs), 0583fb6e (entrypoints.rs/lib.rs/otel.rs/session_cache.rs/sqlite_sessions.rs), d6432dd5 (entrypoints.rs/csv_export), fb1addd5 (rm-584/583/585 flips + vacuous-validation-block prevention doc), ff0068ca (rm-596/610/611 flips + mid-cycle-landing prevention doc). Fleet def-row high-water rm-599 (run-84be17b33814). This cycle's flips (rm-207/489/545) are numeral-disjoint from every recorded sibling flip band; banners and same-subject prose reconcile by TITLE at integration.
- Watch: codeburn #1647/#1648 (Command Code), token-monitor (2636★, local-first), ccusage models.dev snapshot lane, LiteLLM pricing drift (rm-006/rm-176 own the classes), deps at/near pins.
- Wall after compound: 213 def rows total (199 backticked + 14 bare old-format), 0 duplicate ids across both formats, max rm-600, managed footer last; status mix 126 candidate / 50 implemented / 37 done.

## Review fix appendix (2026-10-07, fix a8dfb757 over independent_review fd1e2f7d: NEEDS_CHANGES)

Verdict rested on roadmap-text honesty — the review cleared the code first-hand (every recorded test claim re-verified live: help parity 49/49/0, TSV rollup arithmetic incl. the no-double-count invariant, rm-489 PoC, cache write shape, CI matrix, wall integrity 213 total). Repairs, all landed this attempt:

- **F1 (medium)** rm-489 bullet reworded to the code's truth — special targets are char devices + fifos (`is_special_output_target` = `is_char_device() || is_fifo()`, main.rs:1284-1292); REMAINING SCOPE rider records block-device targets still staging+renaming; F5's latent cfg(unix) note recorded on the same row.
- **F2 (medium)** rm-545 'stub transcripts stop inflating bare listings' claim withdrawn; REMAINING SCOPE rider names the two unmet acceptance clauses (provenance disclosure via MetricProvenance lib.rs:334; stub deflation/folding incl. the missing PARENT column for TSV consumers).
- **F3 (medium)** rm-487 (owning row for the linkage semantics + TSV column contract, cited at tests/subagent_attribution.rs:5,:58) given a dated partial-landing note — the landed half is enumerated so the next implementer does not re-derive it.
- **F4 (low, code)** tests/subagent_attribution.rs skip-leg tightened: absent binary = the only skip (reason on stderr); a PRESENT binary exiting non-zero now panics with status + stderr — a crashing binary is a regression, not an environment gap.
- **F6 (low)** census precision fixed in banner + this record: 213 def rows total (199 backticked + 14 bare, 0 cross-format dupes); '21 lanes executed rc0 + 1 condition-skip', not '22 lanes ALL rc0'.
- Chosen over the review's optional alternative (adding `is_block_device()` to the predicate): keeping the reviewed band code-frozen — the reword + rider preserves the finding durably without unreviewed behavior; the one-line arm stays available to the next cycle.

Validation after the F4 code edit (targeted scope; logs /tmp/at-fix-a8df/): `cargo fmt --all --check` rc0; `cargo clippy -p agenttrace-core --all-targets --locked -- -D warnings` rc0; `cargo test -p agenttrace-core --test subagent_attribution --locked` = 4/0 with the full TSV leg exercised (no skip); `scripts/ci/check-docs-commands.sh` rc0 sandboxed; digest re-derived post-change with the engine's own code = validation:v1:a9b25d1cb00964de93c0437ee9a6bf0d0ae42af7895e82bff9a8b08d260a8578 (byte-equal to the dispatch value — the digest classifies the delta, and the test-content/docs changes do not move it); SurfaceDelta = 22 surfaces, testable = ('scripts/ci/check-docs-commands.sh',), derivation_failed=False. 3 sessions.json.tmp.* residue files from the test run swept; porcelain ends at the same 14 paths as compound-end (9 M + 5 ??).
