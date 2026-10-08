# Cycle-1 compound record — run 24ec00eb (repository-maintenance c899be74)

Phase: `compound:compound` · attempt `0b6b826758604e8d88f0a91414a65b29` · 2026-10-08 ·
worktree `run-24ec00eb0037-24ec00eb` @ base `39bd06b` · pre-review (review and shipping
outcomes land after this phase; the next cycle's assessment carries them forward).

**NO test execution at compound.** Every outcome below was *consumed* from the recorded
targeted/full-validation artifacts, never re-run. The validation digest
`validation:v1:3e36838e29b4e66ef560d2f7cff475dc553217515b12c3253d7111cfeb30c254`
(MATCH-DISPATCH, re-derived after all lanes by the full_tests turn) is anchored at HEAD
`39bd06b` over the changed executable surfaces — this phase's output is markdown-only
(ROADMAP.md + this record), which cannot move it.

## 1. Provenance: adoption of reaped 9749d8fd's layer, corrected

This phase had two provider-dead predecessors on the same action. The durable trail:

- **7523b28b** (06:19:05–06:29:32Z, ≤7 messages): no envelope, no scratch dir, no /tmp
  fingerprint, and the live ROADMAP delta was still cmp-identical to the roadmap-phase
  spool mirror → zero durable work. Redone from scratch.
- **9749d8fd** (06:58:39–07:09:43Z): no envelope, no scratch dir — but it had ALREADY
  written the complete ROADMAP compound layer (ROADMAP.md mtime 07:07:48Z, two minutes
  before its reap): the `compound c1` banner, the rm-389 dated append, the rm-783
  title-twin note, and the three candidate→implemented flips with EXECUTED c1 bullets.

Disposition (7e00d9cbe20a / 4d04171e precedent): **ADOPTED after leg-by-leg live
re-verify, attribution kept, two factual defects corrected in place.** The banner's
pointers to its own never-written scratch mirrors and to this record were repointed to
this attempt's artifacts, with the provenance recorded in the banner itself.

## 2. Verification legs performed by this attempt (no tests)

Identity/state:
- `git rev-parse HEAD` → `39bd06b3…`; porcelain = the implement delta 9 M + 2 ?? plus
  this phase's markdown (ROADMAP.md grown, this record added); HEAD never moved.
- Live ROADMAP delta vs `ffe67b02…-scratch/roadmap-delta.patch`: the mint layer is
  byte-identical (the increment is purely the compound layer); full delta +41/−1 on
  ROADMAP.md, tree numstat 479/−83 = implement's 472/−83 + the 7-line compound increment.

Banner claims re-verified first-hand:
- Census: 269 def rows = 255 backtick + 14 legacy plain; zero duplicate ids in either
  form; status 130 candidate / 68 implemented / 57 done (flips: 133→130, 65→68); max id
  rm-786, ascending tail …753→783/784/785/786; managed footer still the file's last line.
- Code claims: `mod filters` + `pub use filters::{…}` (core lib.rs:5/:34);
  `session_start_cmp` lib.rs:1747; `parse_operator_value` zero hits in the TUI;
  `creating parent directory {dir} for -o {target}` main.rs:1577; README `-o` row and
  `--health`/`--cost` rows document the dialect; governance-reports.md carries it too.
- Recorded outcomes vs artifacts: targeted log (core 225/0, cli 135/0 across 8 result
  lines, tui 50/0, fmt rc0, clippy `-D warnings` rc0, digest MATCH-DISPATCH before and
  after) and the full machine log `/tmp/at-full-57351c55/full-suite.log`: exactly 23
  `LANE-END` lines, zero non-`rc=0`, `ALL LANES PASSED`,
  `MACHINE-SUM: passed=657 suites_ok=33 failed_lines=0`.
- Title-twin claim: sibling run 2023f222 envelopes confirm implement `6919a8ea` +
  full_tests `c4a7834c` both succeeded at base 611242d1 — rm-790 IS executed through its
  own full validation. Do not double-implement rm-783 against it.

## 3. Defects found in the adopted layer and corrected in place

1. **reports.rs:734 anchor false at this base** (banner next-cycle lead #2 and the new
   rm-389 dated append both repeated the landed rider's stale anchor). Live re-verify:
   :730-738 is `render_recommendations_text`; the only baseline-file path is rm-569's
   `load_baseline_report` (reports.rs:783-813), landed by `dfc3b36` ("truthful --baseline
   gating … (rm-569, rm-573, rm-389, rm-212)"), which names `--baseline`, the path, and
   the cause on every failure arm (UTF-8 :787, read :790, JSON :794, summary :797,
   version :813). The landed rider went stale when that landing did not refresh it.
   Correction recorded in both new texts: next cycle's assess pins a live
   raw-propagation reproduction or retires the arm; if none is pinned, rm-389 is closable
   at its next landing by title-flip citing this. Do NOT implement against :734.
2. **Dead-attempt count undercounted** ("4 total" written from phase memory). Events-dir
   sweep: EIGHT provider-reaped sessions run-wide — assess ×3 (`367ee375` ~15min,
   `808e9ec1` ~51min, `261ed69b` 26s; zero artifacts each), roadmap `e0af1eae` (adopted
   via ffe67b02's envelope), implement `f266c3a6` (one orphan unwired draft,
   dispositioned), targeted `ce413ded` (mtime-proven zero), compound `7523b28b` (zero),
   compound `9749d8fd` (this layer's author); plus recovered-with-envelope prioritize
   `fb0a6961`. Corrected in the banner.

## 4. Cycle-1 batch and recorded outcomes (summary)

Batch "one honest query surface across CLI and TUI" (prioritize 88c9f854, amended from
adopted fb0a6961; stewardship f89f61cd; implement a5e4003e; targeted 53c95f18; full
57351c55):

- **rm-785 (LEAD)** — shared parsed-instant recency: new
  `agenttrace_core::session_start_cmp` (absent/unparseable = oldest) backing
  `canonical_sessions`, `--sort recent`, `newer_session_order` (mtime fallback + path
  tie-break kept), and the TUI `SortKey::Recent` comparator. Mixed-offset corpus unit
  pinned at every site.
- **rm-786** — ONE finite numeric-filter dialect: new `agenttrace_core::filters`
  (operator-optional `>= <= > < =` + finite number; bare = `>=`; `total_cmp` basis)
  ADOPTED VERBATIM from e97ae6c9's unmerged rm-364 design (conductor-salvage `a5daa6b`)
  per rm-389's adopt-don't-reinvent rider; CLI `--health`/`--cost` gate + TUI
  `:cost`/`:health`/`:context` parsers all route through it; TUI
  `parse_operator_value` removed; `=` is exact. E2E `tests/numeric_filters.rs` (3/0).
  Premise correction recorded: at base the CLI REJECTED bare numerics outright — the
  mint row's "CLI `--cost 1.5` therefore means >= 1.5" evidence line was false; the
  executed dialect ACCEPTS bare = `>=` on both surfaces, documented.
- **rm-784** — `-o` mkdir-p KEPT (documented, fleet-relied-upon) + honest refusal
  context riding the landed rm-610 cause-chain handler; wording composed with sibling
  de600e73's executed rm-704.

CHANGELOG's 3 Unreleased→Fixed riders and the README/governance-guide dialect docs were
minted by implement — not duplicated at compound. Done-flips (implemented→done) stay
reserved to the commit gate (rm-012 convention).

## 5. Next-cycle leads (recorded, not minted)

1. **rm-783** (HIGH 78.0) stays candidate and remains the natural LEAD — but carries the
   rm-790 title-twin (see §2). Re-census landed walls for rm-790 BEFORE implementing
   here; reconcile BY TITLE at integration per
   `docs/solutions/workflow-issues/roadmap-campaign-id-collision-at-integration.md`
   (880a7b9e convention). Sequencing caution: 2023f222's rm-790 touches the same
   history.rs / sqlite_sessions.rs surfaces.
2. **rm-389's last listed arm** — see §3.1: pin a reproduction or retire the arm; the
   row may be closable outright.
3. Codex 0.161.0 SQLite thread-store drift arm stays radar — mint nothing until a
   0.161+ journal lands on this host.
4. Watches re-armed: claude-code 2.1.292/2.1.293 on-host validation still blocked
   (newest local journal 09-30); LiteLLM gap 1,386 monotonic vs bundled 3,100
   @2026-10-04.

## 6. Commit-gate seams

- Stage ALL 12 paths: the 9 M source/test/doc files + 2 ?? (`core/src/filters.rs`,
  `cli/tests/numeric_filters.rs`) + this record (`git commit -am` DROPS the three
  untracked paths).
- ROADMAP.md carries the roadmap-phase mint (+33/−1) AND the compound layer in one file.
  Replay from base 39bd06b: apply `ffe67b02…-scratch/roadmap-delta.patch`, then
  `0b6b8267…-scratch/roadmap-compound-0b6b8267.patch` (round-trip verified
  byte-identical); the single-file full delta is mirrored at
  `0b6b8267…-scratch/roadmap-full-final-delta.patch` (applies clean on base,
  byte-identical).
- Flip rm-784 / rm-785 / rm-786 implemented→done BY TITLE at landing.
- Sibling overlap: 2023f222's rm-790 (same surfaces as rm-783); de600e73's rm-704 `-o`
  wording is already composed in CHANGELOG.

## 7. Prevention rules (reusable lessons)

- **PR-A (reaped-banner fact-check, hit again):** a reaped attempt's in-tree prose is
  adopted only after line-by-line re-verification. This run: the reaped layer's
  `reports.rs:734` anchor was false and its dead-attempt count undercounted. Same class
  as 7e00d9cbe20a/4d04171e (`:240/:245` defect).
- **PR-B (riders go stale at their own landing):** a landed rider that says "X is open
  at :NNN" is falsified silently when a later landing (here `dfc3b36`) fixes X without
  refreshing the rider. Before implementing ANY rider-listed arm, re-verify the anchor
  live; when a rider is falsified, record the correction as a dated append on the owning
  row (done here for rm-389) rather than editing landed history.
- **PR-C (events-dir is the dead-attempt ground truth):** count reaped sessions from
  `events/*.jsonl`, not from phase memory — the drafting attempt's "4 total" missed
  three pre-roadmap assess reaps.
- **PR-D (mirror at write time):** 9749d8fd's banner pointed at mirrors it died before
  writing. Scratch mirrors and the typed envelope are written BEFORE the final message,
  and this phase regenerated the mirrors under the adopting attempt's id with round-trip
  proofs.
