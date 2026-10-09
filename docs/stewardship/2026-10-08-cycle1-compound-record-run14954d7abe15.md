# Cycle 1 compound record — run 14954d7abe15-14954d7a (2026-10-08, pre-review)

- **Run:** `14954d7abe1547cdb81e89de795fe04c` · repository-maintenance `e2b222adc9784aa2af0335184dcd1a6d` cycle 1 · base `aa5544af58d29ac64db9967cd8fa5a7c1fdfc48d6`
- **Compound attempt:** `6bab4ea172d24bb6ab62af9a2eb207a4` (this record). Zero test execution at compound — all validation outcomes below are the recorded results of the implement/targeted_tests/full_tests phases, consumed as evidence.

## Batch

"journal truth: contain hostile input, surface hidden wire" — selected by prioritize `9b8cf63d` from the roadmap minted at `ddfa5c28` (rm-776..rm-779 + 20 riders; the batch's rm-776/rm-777 were renumbered → rm-880/rm-881 at the 2026-10-09 commit gate — see the execution section at the end), stewarded as four units at `bfcde79c`.

## Units and dispositions

| Unit | Row | Disposition at compound |
|---|---|---|
| U1 LEAD | rm-778 reliability 72.0 | **candidate → implemented** + dated EXECUTED bullet. cap_session_cwd() at all 5 cwd read sites, cwd_truncation_disclosure, depth-bounded find_git_root, size-bounded memo; deep-cwd fixture regression (bounded work, not wall-clock). Hostile-corpus PoC 133.90s → 0.032s cold. |
| U2 | rm-880 compatibility 74.0 | **candidate → implemented** + dated EXECUTED bullet. Codex 0.160.1 session_meta identity/lineage → Session.wire_metadata; session_configured arm (thread_name/model_provider_id/service_tier, no longer ignorable-swallowed); rate_limits quota side-channel disclosed-not-folded; codex_fork_lineage_unfollowed disclosure. Token totals UNCHANGED. Fixtures in-tree. |
| U3 | rm-406 dated arm only | **stays candidate** + dated EXECUTED bullet for the advisor arm: iterations[] per-model attribution (advisor_message carries its own model), by_model + session attribution now show claude-fable-5-1 beside opus; type-"message" iterations[] never folded (sub-turn duplicates). Core unknown-kind census arm REMAINS OPEN. |
| U4 | rm-779 developer-experience 34.0 | **candidate → implemented** + dated EXECUTED bullet. Documentation arm: governance-guide matching rule + --project help block; contract pinned (match kept / no-match loud error rc!=0). |

Done-flips for all implemented rows are reserved to the commit gate per rm-012; compound flips only candidate→implemented.

## Validation record (consumed, not re-run)

- **targeted_tests (`e63bcd642`):** `cargo test -p agenttrace-core --locked` rc0 419/0 over 21 suites (incl. journal_truth_batch 13/0); `cargo test -p agenttrace-tui --locked` 47/0; `cargo test -p agenttrace --locked --bin agenttrace` 61/0; `cargo fmt --all --check` rc0; clippy `--workspace --all-targets --locked -- -D warnings` rc0. Tree byte-identical after.
- **full_tests (`6147bdb7`):** dispatch full_command EMPTY ⇒ 21 ci.yml lanes executed verbatim (memory #16961 procedure). ALL rc0 first pass; tests lane 585/0 over 30 binaries — exact reconciliation: assess baseline 572 + 13 new journal_truth_batch tests. Gate-admitted cargo block (workers=1); AGENTTRACE_CI_OUT redirected to /tmp; MSRV floor + TUI real smoke excluded per ci.yml's own event/var gating.
- Validation digest through both phases: `validation:v1:ab5fc426b7c18d2939481f5888e883b670db730c97ed26ffc53da0e6a18c2845` (no executable surface changed in either validation phase; porcelain byte-identical pre/post).

## Prior-attempt forensics (three reaped attempts, all zero-durable)

assess `95792417` (19 ping lines), implement `e3779309` (pings + reap; tree at dispatch = roadmap handoff), targeted_tests `caeef554` (4-line event log). All census-MATCH ⇒ redo-from-scratch, all redone and declared in the successor results. Dated addendum filed on `docs/solutions/workflow-issues/provider-reaped-delegate-attempts-redo-from-scratch-on-census-match.md` (fleet pattern now 7 instances / 6 actions; artifact-present adoption branch still never fired).

## Deferred register (by design, from prioritize `9b8cf63d`)

- **rm-881** Amp (Sourcegraph) adapter — deferred by scale; fixture-first is MANDATORY (no local Amp install; `~/.local/share/amp` absent) and fixtures do not exist yet. Next cycle must build annotated fixtures from the ccusage adapter contract BEFORE any discovery code.
- **rm-402 / rm-164** — deferred by dependency on this batch's landings (rm-402's budget window and rm-164's tier application compose with the attribution surfaces this batch touched).

## Next-cycle leads

1. rm-406 core census arm — drift driver now live upstream (CC 2.1.289 agent.spawn teammates / idle-waiting line classes).
2. rm-881 fixture-first (see deferred register).
3. rm-239 (P87) — strongest untouched candidate on this wall.
4. Title-twin reconciliation at their landings: rm-771/772/774 (run aa41d9b5), rm-763 (run ec762a61) — disjoint seams recorded in the mint band.

## Commit-gate checklist (owed, not performed here)

1. Stage the **4 untracked paths explicitly**: `crates/agenttrace-core/tests/journal_truth_batch.rs`, `crates/agenttrace-core/tests/fixtures/journal-truth/`, `crates/agenttrace-core/tests/fixtures/usage-accounting/claude-advisor/`, and this record (`docs/stewardship/2026-10-08-cycle1-compound-record-run14954d7abe15.md`).
2. CHANGELOG Unreleased riders beyond the implement batch bullet.
3. Done-flips for rm-778/rm-776/rm-779 (+ rm-406 stays candidate) per rm-012.
4. SESSION_CACHE_SCHEMA_VERSION 33 collision: run 32f3b7a1's 32→33 is COMMITTED at its HEAD `4a13e1f` (2026-10-08, not yet on origin) — whoever lands second re-bases to 34 and merges the governance-guide sentence.
5. RENUMBER at integration (review finding F3): this batch's `rm-776`/`rm-777` numerals are TAKEN on the landed wall — `origin/master` carries landed `rm-776` (compatibility 66.0, campaign-local rm-622) and `rm-777` (customer-experience 58.0, campaign-local rm-624), rebound at integration 2026-10-07. Rebind this batch's `rm-776`/`rm-777` rows together with EVERY artifact reference (banner, EXECUTED bullets, CHANGELOG batch bullet, deferral register, this record, prevention-doc addendum); `rm-778`/`rm-779` are free on the landed wall.

## Review-fix addendum (2026-10-08, attempt `eb5b04f0`)

Independent review `7d51cacd` returned NEEDS_CHANGES (2 blocking medium + 1 medium commit-gate amendment + 2 advisory low). All five fixed in this attempt, pre-final_validation: **F1 (blocking)** `lib.rs` `by_model` cost column re-partitioned — when a session carries `model_attribution`, the headline `multiple` bucket keeps its session count but contributes zero cost, restoring `sum(by_model[].cost) == total_cost` for `agenttrace.overview.v1` machine consumers; pinned by exact-sum assertions in BOTH `journal_truth_batch.rs` overview tests (unit arm + real `usage-accounting/claude-advisor` fixture arm). **F2 (blocking)** rm-779's "pinned by test" claim made true: unit pin `project_matches_is_case_insensitive_substring_over_all_identities` (insights.rs) + entrypoints rc pin `project_filter_no_match_exits_loud` (`--overview -f json --project zzz-nomatch-pin` → rc≠0, "No sessions match the requested filters" on stderr). **F3 (medium, commit-gate amendment)** checklist items 4–5 above + ROADMAP banner corrected (sibling 32→33 COMMITTED at `4a13e1f`; landed rm-776/rm-777 numeral collision named with the rebind mandate). **F4 (low)** duplicated "so cached sessions regenerate under the corrected totals" clause removed from the governance guide's schema-history paragraph. **F5 (low)** `journal_truth_batch.rs` header pointer corrected (the named `usage_truth.rs` suite does not exist; fixtures named instead). Validation after the fixes: targeted re-run of the touched surfaces (see the attempt envelope) — fold scope unchanged, no new changed-file paths.

## Wall state after this compound

ROADMAP.md now: compound c1 banner newest-first at the Open-items top; rm-778/rm-776/rm-779 flipped implemented with EXECUTED bullets; rm-406 EXECUTED arm bullet appended, status candidate; managed footer still the file's last line. No ids minted; wall pointer unchanged (max rm-779, next free rm-780 after a fresh live claim census).

## Adoption postscript (2026-10-08, attempt `af809889`)

This record's author attempt (`6bab4ea1`) was reaped at 03:11:01Z, 84s AFTER writing this record and its typed result — transport loss only, work complete. One further zero-durable compound reap followed (`51e8d7bf`, 27s, 2 messages, no writes). Verified and adopted as-is by `af809889` (adoption legs: work-order identity match, artifact-mtime census inside `6bab4ea1`'s live window, wall census 251 unique ids / 3 dated flips / footer, parser.rs anchor reads, fixtures on disk). Adopter corrections folded in: CHANGELOG batch bullet test count 10→13 (`journal_truth_batch.rs` carries 13 `#[test]`; targeted log 13/0), assess `95792417` ping count 12→19, and a dated sub-addendum on the prevention doc recording the fleet's FIRST artifact-present adoption (sibling-envelope enumeration + mtime cross-check as the rule). Sibling `run-32f3b7a1fcd7` corroborated carrying schema 33 at its HEAD — the collision checklist item stands for the commit gate.

## Commit-gate execution (2026-10-09, attempt `d0a53fc6`)

Checklist items 3–5 EXECUTED at the commit gate, with the wall re-censused live (the checklist's fixed targets were written 2026-10-08 against origin/master `4a13e1f`'s ceiling; the wall moved overnight):

- **RENUMBER EXECUTED — rm-776 → rm-880, rm-777 → rm-881.** Live census at the gate (post-fetch, origin/master `190b706`): landed wall def-max rm-838 and carries landed `rm-776` (compatibility 66.0) / `rm-777` (customer-experience 58.0); unlanded claims above it rm-843/844/845 (fbc4581f), rm-846..848 (2e7758d3), rm-849/850 (9d45a4ec), rm-851..853 (972cf938), rm-855 (run 6cb2756a local landing `8eb6ad40`, unpushed), rm-864..870 (91c7faf5 spool patch); rm-899 was a census sweep RANGE with zero def hits and the rm-99x grep hits were substring false positives — minted rm-880/rm-881, 9 clear of the frontier as live-contention margin. Rebound: ROADMAP rows + banner + riders, CHANGELOG batch bullet, this record's table/deferral register, governance schema sentence, code comments (parser.rs, lib.rs, session_cache.rs, journal_truth_batch.rs). Historical narration (mint band, mandates, this record's earlier sections) keeps campaign-local numerals with the mapping recorded in ROADMAP at the mint band.
- **SESSION_CACHE_SCHEMA_VERSION re-based 33 → 40** (origin/master landed ceiling 39; the checklist's "re-base to 34" was written against the then-ceiling 33 — 34..39 landed with other campaigns while this cycle ran, and landing 33 would have downgraded the cache format). tui fixture literal + history comment, governance guide, CHANGELOG follow. SQLite snapshot version untouched (ours 7; origin's landed 8 takes precedence at merge — no action here).
- **Done-flips EXECUTED per rm-012**: rm-778, rm-880, rm-779 → `status: done` with dated commit-gate bullets (validation chain recorded inline); rm-406 stays candidate (dated arm only).
- **Staging**: all four untracked paths (journal-truth fixtures ×2 dirs + advisor fixture + prevention doc) staged explicitly; no generated backups, no unrelated files.
