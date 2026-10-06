# Cycle-1 compound record — run 933058dcc2c5 (repository-maintenance 853a4f32)

Compound attempt 861f6429, 2026-10-06, tree 1511547 (run base; NO salvage
commit — branch `conductor/run-933058dcc2c5` == base exactly), worktree
`run-933058dcc2c5-933058dc`. Pre-review cycle evidence only — assessment
(ffa9038a), research (c8118d16 pass 11), roadmap (0a23429b), prioritization
(931016d4), stewardship (9e2c8a42), implement (0a0ad193), targeted_tests
(3c53b452), full_tests (bd2acc53). **No validation was re-run in this phase** —
the targeted and full-suite outcomes are consumed as recorded evidence, per the
compound contract.

## Cycle outcome in one paragraph

Batch "Report-layer numeric truthfulness" landed uncommitted as a 7-file delta
(+56/−17 tracked + new `tests/report_numeric_truthfulness.rs`): rm-529 —
`compute_overview` by_task_type token accumulation (lib.rs:1457-1458) converted
from plain `+=` to `saturating_add` with cap-once semantics documented in
`docs/guides/parser-guide.md` §"Numeric Bounding" (the adversarial-corpus
`--overview` panic went debug rc=101 → rc=0, and the release-mode silent wrap
is gone); rm-532 — the always-0.0 `cost_per_output_token` trend field replaced
by `output_cost_per_million_tokens` computed by a shared `per_million_output_cost`
helper used by BOTH the projects rows and the totals row (cannot drift; live
corpus shows 18.05 == 18.05 in text and `-f json`, was 0.0). Targeted: core
302/0 + tui 47/0, fmt/clippy rc0. Full suite (ci.yml full+deny mirror; dispatch
`full_command` was empty): 19 gates rc0 + 1 CI-conditional skip, **428 passed /
0 failed** across 19 ok targets including the 3 new pins; one regression fixed
in-gate — the known tag-visibility hazard red on `check-plugin-version.sh`,
cleared by appending the sanctioned `no-changelog-section` marker block to
CHANGELOG.md as the **4th byte-identical carrier** (after b163e538 / b10bf831 /
a26322ab). Wall: 167 ids, 95 candidate / 36 done / 36 implemented (rm-529 and
rm-532 flipped at this compound; done-flips reserved for the commit gate).

## Prevention rules (repo-durable, reuse verbatim)

**PR-1 — Numeric bounding is a layer-wide contract, not a parser feature.**
rm-046/rm-418 saturated the parser's token maps, yet the reports layer kept
plain i64 `+=` two releases later and shipped a reachable panic. Rule: any
aggregation site that sums parser-clamped i64 token counts MUST use
`saturating_add` with cap-once semantics; the repo-wide invariant
`grep -nE '\.tokens_(input|output) \+=' crates/` must stay at ZERO hits
(cost-accumulating dimensions are f64 and exempt). The rule is written where
the next implementer will look (`docs/guides/parser-guide.md` §"Numeric
Bounding") and enforced by `overview_by_task_type_saturates_instead_of_overflowing`.

**PR-2 — Fixtures without consumers are silent reds.**
`testdata/generated/adversarial/` carried `usage.input_tokens` 1e300 since
93aaf05 while zero tests or CI referenced it — the suite read 425/0 green at
the exact commit where `--overview` panicked on that directory. Rule: every
fixture corpus under `testdata/` must be wired into at least one test or
removed; assessment phases grep for orphan corpora before trusting green.
(`adversarial_corpus_overview_totals_finite` is now the standing consumer.)

**PR-3 — Derived-number truthfulness needs a shared helper + a cross-view pin,
not a rename.** The dead `cost_per_output_token` fix could have duplicated the
per-million formula in two places and re-diverged. Instead the derivation lives
in ONE function consumed by both the projects rows and the totals row, and
`context_trends_project_cost_per_million_matches_totals` pins project == totals
> 0 on a priced fixture. Any new derived metric follows this shape.

**PR-4 — Tag-visibility drift is a standing gate hazard with a sanctioned,
byte-fixed remedy.** Never `git fetch --tags` from a validation gate; when the
per-tag arm of `check-plugin-version.sh` reds on inherited merged tags
(v0.7.2–v0.7.7, v0.8.0), append the `<!-- no-changelog-section: vX.Y.Z -->`
marker block EXACTLY as the existing carriers (this worktree is the 4th; tails
verified byte-identical vs run-4a68825724aa and run-250cfd64686b) — never edit
history sections, never delete tags, never bump plugin versions to un-red a
gate.

## Process lessons (conductor-fleet facing)

- **Dead-attempt forensics before redo** (held twice this run). full_tests
  attempt d5200c49 was provider-dead: its event log holds exactly 4 events
  (start → progress at message 3 → session_reaped failed → completed failed),
  no typed artifact, no scratch — provably a null turn, redo declared in the
  full_tests record.
- **Empty `full_command` → the repo's CI file is the command authority.**
  Step-for-step mirror of `.github/workflows/ci.yml` jobs `full`+`deny` in CI
  step order (4th fleet run to do this), with the env mitigations: private
  `AGENTTRACE_CI_OUT` under /tmp, isolated HOME for binary-invoking gates,
  `TMPDIR=/tmp` for cargo gates, and cargo-deny flag order
  `cargo deny --all-features check`.
- **Declare dispatch digests verbatim only with the three-leg proof.** The
  full_tests dispatch stamp `0ca09f01` is not reproducible from (run base,
  worktree) under the engine's own replica code (which yields `33cb6ac8` even
  though it byte-exactly reproduces sibling 1f12309a's stamp `d6221bb9`) — the
  service stamps from a recipe invisible to read-only delegates. The rule that
  held: when nothing executable-classified changed in the turn — proven via
  `classify_surface`, a pre/post digest-equality reconstruction, and porcelain
  — declare VERBATIM (the fold re-derives from a byte-identical executable
  surface) and never substitute the replica's value. Full trail:
  spool `bd2acc53…-scratch/digest-forensics.md`.
- **Deny-gate green ≠ advisory-free.** lru 0.18.1 passes the authoritative
  `cargo deny --all-features check` against the live advisory DB while external
  research maps RUSTSEC-2026-0253 to "fixed exactly 0.18.2". Dependency bumps
  are roadmap decisions with their own cycle, never gate-driven edits inside a
  validation phase.

## Next-cycle context (concrete, for the cycle-2 assessment)

- **Recommended lead: rm-390's zh-CN README parity arm** (extended at this
  run's roadmap): 4 sections absent from `README.zh-CN.md` (CSV :110,
  Governance :136, baseline/compare :205, Statusline :218). Pure translation/
  doc work, zero fleet contention, acceptance line already written.
- **Alternate: the dependency-refresh wave from research pass 11's crates.io
  census** — patch-level riders serde_json 1.0.150→1.0.151 and clap
  4.6.2→4.6.7 are cheap; ureq 2.12.1→3.4.2 and rusqlite 0.32.1→0.40.2 are
  majors that each deserve a dedicated cycle. lru→0.18.2 advisory clearance
  rides the patch wave (verify sibling research 4ffc4fbb C5's row before
  minting — the roadmap lane may already own it).
- **Cheap rider: enable the TUI real-smoke gate** — `vars.AGENTTRACE_TUI_REAL_DIR`
  is unset, so CI permanently skips `check-tui-real-smoke.sh`; setting the repo
  variable + a small fixture corpus converts a dead gate into live coverage.
- **Verify, don't re-derive, the upstream v0.9.1→v0.10.1 wave:** sibling bands
  own the decomposition (unlanded: db6b7626 525-528, 66e75e39/…530,
  e486dc1adb9b 530-531; landed: 533-540 at origin/master 6b03087). Cycle-2
  assessment should check which arms LANDED and mint only genuine gaps.
- **Collision zones to respect:** rm-539 is double-claimed (cb38b958 vs
  2d92ee95 — reconcile by TITLE at integration); parser.rs accounting lanes
  are owned by the unlanded 1511547 bands; this run's rm-529/rm-532 are absent
  at origin (verified live) and clear to land.
- **Numerals:** live census at this compound — landed wall defmax rm-540;
  unlanded defmax 550 (9873fc06); **next free rm-551**.

## Carry-forward for the review/commit gates (do not double-implement)

- **ONE commit lands the whole batch**: 6 modified (CHANGELOG.md, ROADMAP.md,
  governance.rs, lib.rs, presentation.rs, parser-guide.md) + 1 new test file +
  this stewardship record; CHANGELOG includes the implement entries AND the
  marker-block tail.
- **The marker block must land** (4th carrier; dedupes byte-identically
  against the three earlier carriers — whichever commit lands first wins, the
  others drop the duplicate append).
- **Done-flips for rm-529/rm-532 reserved for the commit gate** (rows are
  `implemented` now, per house convention).
- Review/shipping outcomes are deliberately absent here; the next cycle's
  assessment carries them forward.

## Artifacts (this phase)

- `ROADMAP.md` — compound annotations only: banner line (compound c1, evidence
  + re-sweep + accounting), rm-529/rm-532 status flips candidate→implemented.
  No new mints. Patch byte-copies re-synced to the spool scratch and
  `/tmp/at-roadmap-933058/`.
- This record. No test execution, no source changes.
