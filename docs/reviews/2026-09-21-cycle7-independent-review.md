---
artifact_contract: "ce-review/v1"
created_at: "2026-09-21T00:00:00Z"
title: "Independent adversarial review — cycle 7 batch CU-24..CU-28 (truth-telling debt, parser parity, honest arithmetic)"
summary: "Verdict: pass_with_findings. All five change units do what the cycle records claim, verified first-hand: CU-24 closes every F5-1..F5-5 acceptance from the cycle-5 review (disclosure, unit fix, rejection, shared coverage line, headerless eviction — plus the ROADMAP filing gap, now 9 F5- references), CU-25's Oh My Pi skip matches upstream 6848aa1 exactly (FETCH_HEAD compared hunk-for-hunk; sniffer dispatch boundary pinned by test), CU-26's saturating audit list is complete (only loop-index += remain in parser.rs), CU-27 verified end-to-end in the real binary (--no-baseline-gate --overview renders the overview), CU-28 metadata names the publishing remote. Every gate re-ran green first-hand: workspace suite exit 0 (224 tests across 7 binaries), fmt clean, clippy 0 warnings. The compound artifacts (ROADMAP cycle-7 entry, lessons, cycle-8 shortlist, learnings record) were reviewed with the same rigor: accurate against the code except where noted. Five findings, none blocking: one MEDIUM — the CU-27 'contract test pinned against clap's own definitions' claim (repeated in CHANGELOG, ROADMAP, implementation record, and learnings rule 2) is not what the test does; it pins the shim against hand-copied 49-flag snapshot lists, so the drift class CU-27 itself was (add a clap flag, miss the shim) still passes silently — a true pin is one `Args::command().get_arguments()` loop away; three LOW (the implementation record still cites two paraphrased test names that learnings rule 3 forbids, its per-binary test ledger is scrambled vs the real 224-test shape, and the legacy data_health path retains the files-minus-sessions subtraction that F5-2 fixed only on the scoped path — call sites currently defuse it); one INFO (ROADMAP paragraph chopping and one mid-word hyphen line break from in-place amendments)."
keywords: ["agenttrace", "independent-review", "cycle-7", "oh-my-pi", "saturating-arithmetic", "go-flag-shim", "data-health", "cache-eviction", "upstream-drift"]
run: "4e6ff52433d44aff92a85afa14400a58"
attempt: "718befd97fb24426bdd990f468b9e6cd"
repo_head: "df3b621adca6e8d3f850ba7f751033ff334482f3"
tree_state: "dirty (cycle-7 implementation CU-24..CU-28 uncommitted on HEAD df3b621; 10 modified files + 4 new stewardship docs; nothing staged/committed/pushed, per delegation policy)"
---

# Independent adversarial review — cycle 7 (CU-24..CU-28)

Reviewed against: the cycle goals in
`docs/stewardship/2026-09-21-cycle7-prioritization.md` (per-CU
acceptance criteria and the batch gate); the stewardship request
(`docs/stewardship/2026-09-21-cycle7-stewardship-request.md`, incl.
dirty-state preservation and must_remain_separate hints); the
implementation record
(`docs/stewardship/2026-09-21-cycle7-implementation-record.md`); the
cycle-5 review's F5-1..F5-7 acceptances
(`docs/reviews/2026-09-03-cycle5-independent-review.md:136-236`);
`ROADMAP.md` lane acceptance criteria as amended this cycle; the
compound step's artifacts (`ROADMAP.md` cycle-7 entry, lessons block,
cycle-8 shortlist; `docs/stewardship/2026-09-21-cycle7-learnings.md`);
the security boundaries the roadmap pins (offline by default, no new
dependencies, untrusted-input containment, no CI/workflow edits — CI
belongs to the PR stage); durability/recovery requirements (cache
bounds enforced on adversarial data, eviction persists via the dirty
flag, no debug-build panics on hostile token fields); and the test
evidence claimed by the implement, targeted-tests, full-tests, and
compound phases.

Routing disclosure: the compound-engineering router remains an empty
stub in this environment (consistent with every phase this run and the
constraint recorded in prior sessions); no ce-* skill content exists,
so the review ran in-thread with adversarial, correctness, security,
reliability, testing, and documentation lenses applied by one reviewer.
**Every load-bearing claim was re-executed first-hand on this
machine** — no finding below rests on the phase records alone.

## Verdict

**pass_with_findings** — the batch is fit to ship through the commit
gate. All five change units are implemented as claimed and verified
first-hand; all gates re-ran green first-hand; no security boundary,
durability requirement, or roadmap acceptance criterion is violated.
Five findings: one MEDIUM (a repeated overclaim about what the CU-27
contract test pins — the fix itself is correct and pinned; the gap is
regression protection weaker than four artifacts state), three LOW
(record-accuracy residuals), one INFO (ROADMAP prose hygiene). None
blocks the commit gate.

## What was verified first-hand (per acceptance criterion)

**Batch gate.** `cargo test --workspace --quiet` → exit 0 (224 tests:
core lib 82, core binary 7, discovery_contract 71, CLI bin 15,
entrypoints 6, demo_contract 2, TUI 41; plus two 0-test doc binaries).
`cargo fmt --check` → clean. `cargo clippy --workspace --all-targets
--quiet` → exit 0, zero warnings. Run on the exact dirty tree
described by the record (10 modified files + 4 untracked stewardship
docs; `git status --porcelain` unchanged from the implement-phase
close; no workflow files touched).

**CU-24 (F5-1..F5-5 landing + filing-gap repair).** Every acceptance
from the cycle-5 review re-verified in source, not in the record:

- F5-1: `main.rs:262-267` and the `--compare` twin (~`main.rs:305-310`)
  both disclose `sampled first N of M sessions in the --sort {sort}
  --order {order} view`; the review's unpinned case is closed by
  `entrypoints.rs:219-223` (`--sample 2 --sort cost --order asc` pins
  the reason naming both).
- F5-2: `insights.rs` `data_health_scoped` counts distinct in-scope
  source paths before subtracting from file-unit `discovered`
  (`insights.rs:326-355`); contract-pinned in
  `discovery_contract.rs`
  (`data_health_out_of_scope_counts_sources_not_sessions`,
  mixed .db/jsonl corpus shape).
- F5-3: `--sample` outside the six governance-class actions is
  rejected with a named error (`main.rs:164-175`); re-run end-to-end on
  the real binary (`--demo --overview --sample 5` → the named error;
  exit code pinned by the entrypoints test that ran green).
- F5-4: the text compare path renders the shared
  `audit_coverage_line` (`main.rs:617-626`), pinned by
  `(auditing 2 of 3 sessions); ` + `sampled first 2` in
  `entrypoints.rs:305-317`.
- F5-5: both cache bounds map headerless entries to `i64::MIN`
  (`session_cache.rs:627-643`, `:672-692`) so eviction is total;
  eviction sets `dirty` (`session_cache.rs:654-656`); two unit tests
  pin entry-bound and byte-bound eviction.
- Filing gap: `grep -c 'F5-' ROADMAP.md` → 9 (was 0 before the cycle).
  F5-6/F5-7 restated as recorded decisions, correctly unchanged.
- Riders verified complete: the two unguarded
  `load_sessions_from_dir` walks in `discovery_contract.rs` now run
  under isolated cache dirs, and the TUI language-preference fix is
  **provably total** — only `App::new_loading` reads the preference
  (`app.rs:406`), both `new_loading` constructions in the test binary
  sit inside pinned tests (`tests.rs:1609/:1628`), the guard holds a
  mutex and restores prior content on drop, and all 40 `App::new`
  constructions never touch disk.

**CU-25 (upstream drift port).** Parity confirmed against upstream
itself, not the record: `FETCH_HEAD` is still `6848aa1` and
`git show 6848aa1` displays exactly the same `bail!`→`continue`
transformation inside the `!seen_header` branch, with the post-loop
`!seen_header` bail preserved — our `parser.rs:1321-1332` matches it
in semantics and placement. The dispatch boundary claim also holds:
`is_oh_my_pi_jsonl` (`parser.rs:1302-1312`) requires a real session
header, and `rust_headerless_oh_my_pi_style_file_falls_back_to_generic`
(`discovery_contract.rs:1803`) pins the generic fallback. The
title-led fixture parses to an `oh_my_pi` session
(`discovery_contract.rs:1775`). Drift census recorded with the
required port-or-decline per commit (`a34dea2` declined with a
specific reason, not silence).

**CU-26 (saturating remainder).** The audit list re-derived
independently: `grep '+=\|-=\|saturating_' parser.rs` shows every
token-counter op saturating and the only bare `+=` are the escape
scanner's loop indices (`parser.rs:4053/:4069/:4073/:4080/:4087`) —
matching the record's list exactly. No token `+=` survives outside
parser.rs (reports/insights/app grepped). The `i64::MAX` fixture
parses without panic in a debug build (`discovery_contract.rs:1474`;
the suite runs debug), with totals pinned at the ceiling and the
`i64::MAX - 100` uncached-input shape asserted.

**CU-27 (Go-flag shim).** `--no-baseline-gate` removed from
`flag_takes_value`'s value list; re-verified end-to-end on the real
binary: `agenttrace --demo --no-baseline-gate --overview` renders the
overview (exit 0) — the acceptance sentence literally true. The
contract test's 22 booleans + 27 value flags were counted against the
`Args` struct field-by-field: they match today. See F7-1 for what the
test does *not* do.

**CU-28 (publish metadata).** `Cargo.toml:15-16` names
`https://github.com/codeo1io/agenttrace`; the remote that publishes is
`codeo1io/agenttrace` (verified by the stewardship phase's
`git remote -v`). Attribution lanes untouched, as stated.

**Compound artifacts** (reviewed with the same rigor as the code):
the ROADMAP cycle-7 entry, lessons block, lane amendments, and
cycle-8 shortlist were checked claim-by-claim against the tree; the
learnings record's evidence index and prevention rules were checked
against the artifacts they cite. Everything is accurate except the
items below. The four prevention rules are sound and durably phrased;
notably, rule 2 states the correct standard that F7-1 shows the cycle
did not itself meet.

**Security and durability.** No production-code `unwrap()`/`expect()`
added (all additions are in test modules); no new dependencies; no
network, secret, or filesystem-boundary expansion (test temp dirs
only); offline-by-default untouched; no `.github/` or CI file touched
(the dependency-review fix correctly stays deferred to the PR stage);
cache eviction remains persistence-safe (dirty-flag → next save).

## Findings

### F7-1 (MEDIUM) — CU-27's contract test pins a hand-copied snapshot, not "clap's definitions"; the claim is repeated in four artifacts

`crates/agenttrace-cli/src/main.rs:1386-1437`. The test
`go_flag_shim_matches_clap_flag_arity` iterates two hardcoded arrays
(22 booleans, 27 value flags) and asserts shim behavior for those 49
names. It never consults the `Args` struct: there is no
`CommandFactory` usage (`main.rs:17` imports only `clap::Parser`), and
clap derive is not reflectable from a plain list. Consequences:

- Adding a new clap **value** flag and forgetting the shim passes the
  test silently (the flag is in neither array) — the shim's
  default-boolean misparse (`--newflag value` treated as flag +
  positional) is exactly the silent-misparse class, and it is
  unprotected.
- Misclassifying a **new** boolean into the shim's value list is also
  invisible to the test for the same reason.

The shipped fix for `--no-baseline-gate` is correct and behaviorally
pinned; the gap is purely that the regression protection is weaker
than stated. The claim "pinned against clap's own definitions /
matches clap's definitions / fails by name on drift" appears in
`CHANGELOG.md` (Unreleased, CU-27 entry), `ROADMAP.md` (cycle-7
Completed, CU-27 evidence: "a contract test asserting the shim's
value-flag set matches clap's definitions"), the implementation record
(CU-27: "Adding a new flag without updating the shim now fails this
test by name" — true only if the test's arrays are updated too), and
the learnings record rule 2 ("arity tables from clap's"). The
prioritization acceptance asked for exactly the clap-derived pin.
Fix is small and was available: `Args::command().get_arguments()`
(clap `CommandFactory`, already derivable) exposes every flag's arity
at test time; asserting shim arity per real clap argument closes the
hole and makes all four claims true. Recommended as the first cycle-8
ride-along; until then the CU-27 acceptance is met-with-a-residual,
not met.

### F7-2 (LOW) — the implementation record still cites two paraphrased test names, violating the cycle's own learnings rule 3

`docs/stewardship/2026-09-21-cycle7-implementation-record.md` (CU-25
and CU-26 sections). It names
`rust_oh_my_pi_session_tolerates_leading_title_line` and
`rust_codex_rollout_with_i64_extreme_token_counts`; the actual tests
are `rust_parses_oh_my_pi_session_with_leading_title_line`
(`discovery_contract.rs:1775`) and
`rust_codex_rollout_i64_extreme_token_counts_parse_without_panicking`
(`discovery_contract.rs:1474`). Learnings rule 3 records that the
targeted-tests pass caught exactly this drift and prescribes
`cargo test -- --list` — but the record that motivated the rule was
never corrected. The third name
(`rust_headerless_oh_my_pi_style_file_falls_back_to_generic`) is
exact. Fix the two names in the record (a docs-only edit) so the
record satisfies the rule it birthed.

### F7-3 (LOW) — the implementation record's test ledger is scrambled relative to the real suite

`docs/stewardship/2026-09-21-cycle7-implementation-record.md`,
"Verification summary": "agenttrace-core lib 68+, discovery_contract
71, demo_contract 7, CLI 41+2 new, TUI 7". The real shape (verified
first-hand and published by the full-tests phase): core lib 82, core
binary 7, discovery_contract 71, CLI bin 15, entrypoints 6,
demo_contract 2, TUI 41 — total 224. The per-binary labels/counts in
the record are shuffled versions of the true ones. The record should
carry the correct ledger (it is the artifact a future archaeologist
reads first).

### F7-4 (LOW, pre-existing — cycle-8 candidate) — the legacy `data_health` path keeps the files-minus-sessions subtraction F5-2 fixed only on the scoped path

`crates/agenttrace-core/src/insights.rs:306-314`: `skipped =
discovered.saturating_sub(parsed)` mixes file-unit `discovered` with
session-unit `parsed` — the same unit conflation the cycle-5 review
rated MEDIUM on the scoped path. Today every call site defuses it
(`main.rs:432` and `main.rs:1231` pass `sessions.len()`; `app.rs:923`
passes 0; the TUI overview at `app.rs:1375-1378` reconstructs
`discovered = sessions.len() + skipped` so the subtraction inverts
cleanly), and `parse_coverage_phrase` (`reports.rs:2517-2525`) guards
`parsed > discovered` into the "N sessions from M sources" phrasing.
Residual exposure: the TUI's reconstructed denominator mixes units
when multi-session (.db) sources coexist with skipped files (sources
overcounted by the skipped-file count), and any future caller passing
a real file count to legacy `data_health` reintroduces the F5-2 bug.
Not a regression from this cycle (the function predates it and its
docstring declares the legacy semantics); fold into the coverage lane
as a small unit-hygiene item.

### F7-5 (INFO) — ROADMAP in-place amendments chop paragraphs and one hyphen break

`ROADMAP.md:949-956` (hygiene lane: "...publish-metadata mismatch:
Acceptance: carryovers committed or gitignored;" — the insertion
splits the sentence the original wrapped around) and
`ROADMAP.md:838-845` (CLI-polish lane amendment interleaved with the
old sentence), plus `ROADMAP.md:1605-1606` where "installer-" ends a
line and "checksum" starts the next — markdown joins them as
"installer- checksum". All readable, none change meaning; the
file's long-line discipline holds (only pre-existing lines 5/13/46
exceed 80). Ride-along cleanup on the next ROADMAP touch.

## Disposition

- F7-1 → cycle-8 implement ride-along (convert the contract test to
  `Args::command()`-derived arities; optionally also correct the four
  claim sites in the same commit).
- F7-2, F7-3 → docs-only corrections to the cycle-7 implementation
  record (ride-along before or at the commit gate; not gating).
- F7-4 → coverage-lane unit-hygiene item (prioritize with the cycle-8
  batch).
- F7-5 → ride-along on the next ROADMAP edit.

## Gates re-run first-hand (2026-09-21, this review)

- `cargo test --workspace --quiet` → exit 0; 224 tests, 0 failures
  (per-binary counts above).
- `cargo fmt --check` → clean.
- `cargo clippy --workspace --all-targets --quiet` → exit 0, zero
  warnings.
- `git show 6848aa1` vs our `parser.rs` → parity confirmed (CU-25).
- `cargo run --bin agenttrace -- --demo --no-baseline-gate --overview`
  → overview renders, exit 0 (CU-27 acceptance, live).
- `cargo run --bin agenttrace -- --demo --overview --sample 5` → the
  named F5-3 rejection (exit code pinned by the green entrypoints
  test).
- `git status --porcelain` → 14 entries, identical to the
  implement-phase close; no CI/workflow file touched; nothing staged,
  committed, or pushed (per delegation policy — commit is a later,
  prohibited stage for this review phase).
