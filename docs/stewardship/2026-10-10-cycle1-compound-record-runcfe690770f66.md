# Cycle-1 compound record — run cfe690770f66 (repository-maintenance 40c448f6)

Compound attempt 53176fc5, 2026-10-10, tree f9aa0eff (fork/master), worktree
`run-cfe690770f66-cfe69077`. Pre-review cycle evidence only — assessment
(8b24227c), research (73f45769), roadmap (768ded26), prioritization
(4accd9ca), stewardship (5f68ad93), implement (ac81b991), targeted_tests
(5f78d07d), full_tests (7092ed67). **No validation was re-run in this phase** —
the targeted (9/19/2/3 green + one layout-only fmt fix) and full-suite
(gate-wrapped `cargo test`, 49 targets, 839 passed / 0 failed, digest
`validation:v1:03d3b153…af8` re-derived identical) outcomes are consumed as
recorded evidence, per the compound contract.

## Cycle outcome in one paragraph

The accounting-truth wave was selected (prioritize 4accd9ca: rm-617 + rm-618 +
rm-619 + rm-893), stewardship then corrected it against live-tree reality
(5f68ad93: 617/618/619 already landed by run 7f9c6d24's upstream-#312 wave —
prioritize's parser anchors were stale tool output), and the surviving live
member rm-893 was implemented and fully validated: the five sqlite
enrichment-lane `filter_map(Result::ok)` silent drops now count into the
rm-753 per-file disclosure (`SqliteFileFailures::record_dropped`), with
`SQLITE_SNAPSHOT_SCHEMA_VERSION` 9→10 so pre-fix snapshots on drop-carrying
files regenerate, governance sentence + version history in-delta, CHANGELOG
Fixed entry, 4 hostile red-first arms, and one live `--doctor` e2e. Wall after
compound: rm-893 flipped implemented (this run) and rm-617/618 flipped
implemented as superseded-by-landed with dated reconciliation riders (both
verified TRUE against the tree by independent review 271d084c); rm-619's flip
was REVERTED by that review's F1 — its supersede justification was false at
f9aa0eff (add_opencode_tokens at parser.rs:5593-5605 folds no tokens.reasoning;
a reasoning-only tokens object still sets message_had_usage at :5310 and
suppresses the step-finish fallback at :5424-5426 while folding zero), so rm-619
stays a live candidate with a re-anchored code-debt rider; roadmap band
rm-921..929 sits in the spool chain, not the wall, until integration.

## Reusable lessons (repo-durable)

**RL-1 — SQLite column affinity makes INTEGER-literal "hostile" fixtures
inert.** The four rm-893 red arms first used INTEGER literals in TEXT columns;
affinity converted them to TEXT at storage, the decode never failed, and
`record_dropped` never fired (proven by eprintln instrumentation). BLOB
literals (`x'39'`) bypass affinity and are the minimal genuine hostile value.
Folded as a prevention doc:
`docs/solutions/reliability/hostile-sqlite-fixtures-need-blob-literals.md`
(YAML front matter matches the solutions family schema).

**RL-2 — A reaped prior attempt may have left worktree drift + banked
baselines: verify, then consume.** This run's implement dispatch opened on a
worktree with one drifted file (the prior attempt's draft test arms) and a
scratch dir holding sha-verified pristine baselines. The draft carried three
latent fixture bugs (TEXT timestamps against an f64 decode, a join on the
wrong id, a row that never met the lane's WHERE); all three were caught by
reading the lanes' actual SQL before trusting the tests. Envelope-absent ≠
no-work; sweep the scratch first.

**RL-3 — Implement phases still omit the fmt lane (third fleet recurrence).**
One rustfmt violation shipped in this run's implement delta
(session_cache.rs:2431, a renamed two-line let wanting one line) and was caught
at targeted_tests with `cargo fmt --check`, fixed layout-only (proven by
whitespace-normalized diff), pins re-issued post-fmt. The lane-01 prevention
doc stands; implement-phase checklists should run `cargo fmt --check` before
delivering (cheap) rather than relying on the later gate.

**RL-4 — Stale tool output drove a phantom batch selection; the stewardship
reconciliation is the cheap fix.** Prioritize scored rm-617 (88) / rm-618 (87)
against anchors from output that predated the landed #312 wave; the tree at
f9aa0eff already contained the fixes (read-verified: rm-603 own-line reasoning
at parser.rs:4176-4182, input_token_count zero hits, part-walk at
:5408-5435). Only rm-893 was live. Re-deriving anchors with fresh reads (or a
compile) before batch selection would have saved the flip-only bookkeeping —
but the reconciliation cost one read phase, which is the designed safety net.

## Context for the next maintenance cycle

- **Batch lead**: rm-547 (76) + rm-546 (62) — same truth family as this
  wave's cache accounting, prioritized to head the next cycle (prioritize
  4accd9ca rationale); rm-921 (P95/P95 cache-write accounting) is the
  strategic follow-on; rm-927 (pricing traceability narrative) is
  decision-gated.
- **rm-921..929 band**: minted spool-side this run (768ded26 patch, sha
  2f92f964…, base f9aa0eff); unlanded. A fresh census is required before the
  next mint — the fleet numeral frontier moves (sibling runs mint rm-9xx in
  parallel lineages; the canonical wall at this writing is ahead of this
  worktree's 370-row wall).
- **Integration-gate items from this cycle**: done-flips for rm-893/617/618
  reserved to the gate (rm-012); rm-619 stays OPEN as a candidate (review-fix
  7b1f5b68/15f781d4 reverted its compound flip — code-debt: fold tokens.reasoning or
  disclose per the rm-408 family; no opencode member exists in
  tests/fixtures/usage-accounting/); the rm-931 two-way title collision and
  the spool reconciliation noted fleet-side.
- **Unminted assess findings**: F7 (crates/agenttrace-cli/src/upstream.rs:548
  -- the npm drift probe shells out to `curl` via Command::new while ureq
  3.4 is already a workspace dependency, so the probe forks a subprocess
  and inherits curl's absence and quirks) and F8
  (crates/agenttrace-cli/src/mcp.rs:150-158 -- initialize_result echoes any
  client-sent protocolVersion verbatim, and DEFAULT_PROTOCOL_VERSION at
  :50 still reads 2024-11-05 while the module implements 2025-06-18
  batch-rejection semantics) remain wall-less; next roadmap phase should
  mint or explicitly defer them. (Corrected 2026-10-10 by independent
  review 306d68a4: this bullet previously described the two MINTED
  findings under the F7/F8 numbers -- the statusline-budget item is F5,
  minted rm-926, and the zh-CN parity item is F4, minted rm-928; the mint
  band's own F-citations were always correct.)
- **Docs co-travel rule (PR-B) held again**: the snapshot-schema bump moved
  constant + ladder comment + governance sentence + version history together
  and passed `check-docs-commands.sh` at rc=0.

## Artifacts

- ROADMAP.compound.patch (this record + the prevention doc + three wall
  flips with riders, plus the review-time rm-619 flip REVERT) — verify with
  the roundtrip chain in the compound scratch NOTES.md (pristine f9aa0eff +
  roadmap patch 2f92f964… + compound patch == postimage, byte-identical).
  AMENDED by the review fix (see the section below); the amended patch at
  this scratch's canonical path ROADMAP.compound.patch (mirrored in the
  15f781d4…-scratch) supersedes a6c0995a….

## Amendment — review fix (2026-10-10): authored by reaped attempt
7b1f5b68, adopted and independently re-verified by 15f781d4

Independent review 271d084c returned NEEDS_CHANGES on the compound artifacts.
F1 (medium): the rm-619 supersede-flip narrated above rested on a false
justification — the opencode tokens.reasoning drop is LIVE at f9aa0eff
(`add_opencode_tokens` at parser.rs:5593-5605 folds input/output/cache only; a
reasoning-only `tokens` object still returns true at the :5310 call, setting
`message_had_usage` and suppressing the step-finish fallback at :5424-5426
while folding zero usage). Fix applied spool-side by attempt 7b1f5b68:

- rm-619 reverted to `candidate` with a corrected, re-anchored rider; the
  framing moved from fixture-debt to code-debt (the fixture the original
  rider proposed would FAIL against the current code — that failure is the
  disproof of the supersede). Ledger note: the mint band rm-921..929 carries
  no opencode-reasoning row, so this revert restores the only open ledger
  entry for the defect.
- The rm-617 and rm-618 closure riders, which the compound phase had inserted
  at the head of the NEXT row's block (off-by-one in row-block boundaries),
  were relocated to the ends of their own rows — text unchanged.
- F4 (low, CHANGELOG.md:14 precision nit): DEFERRED with F2/F3 — the
  review verdict prescribed one spool-side amendment, no code change;
  landing a worktree edit post-review would churn the validated implement
  delta's pins (batch.diff d57e73b6 + postimages) with no review of the
  new wording. Left for a future CHANGELOG rider. Adoption correction:
  the reaped attempt's draft of this section claimed the fix had landed
  in the worktree — FALSE; the tree's CHANGELOG.md is byte-identical to
  the implement-phase postimage 28f66258….
- F2 (wording, main.rs:1690) and F3 (test hardening,
  sqlite_hostile_disclosure.rs) were explicitly deferred by the review as
  future passes and remain open.

The amended patch chain roundtrip was re-verified fresh by the adopting
attempt 15f781d4 (f9aa0eff + roadmap
patch 2f92f964… + amended compound patch == amended postimage,
byte-identical; reverse-apply and double-apply-rejection checks green).
