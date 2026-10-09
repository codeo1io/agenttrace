# Cycle-3 compound record — run 2a5cdb9ef522 (repository-maintenance e687a0cd)

Compound attempt 3a319238, 2026-10-09, base 5ed9ebcc (fork/master lineage),
campaign worktree `run-2a5cdb9ef522-2a5cdb9e`. Pre-review cycle evidence
only — assessment (ca2da504), research (48b0622b), roadmap (f499765f),
prioritization (5e907d5f), stewardship (119c2a45), implement (734d716c,
adoption + gap-closing), targeted_tests (f4b72d82), full_tests (2f51eaf1).
**No validation was re-run in this phase** — the targeted (3-crate battery
39× `test result: ok`, 691/0 + fmt/clippy rc0) and full-suite (ci.yml
full+deny mirror lane-for-lane rc0; release build; entrypoints 44/0; 8
scripts/ci smoke gates; homebrew/npm/manifests/plugin/lockfile; cargo-deny)
outcomes are consumed as recorded evidence, per the compound contract.
Envelope digest `validation:v1:714f807c050a27f7ca71ce060fcd1e0f…` was
declared VERBATIM at both validation turns with no executable change
between them.

## Cycle outcome in one paragraph

Batch "truthful-quantities trio" landed uncommitted as an 8-path delta
(+280/−28) at 5ed9ebcc, then passed targeted and full validation: rm-871
(context-utilization numerator now uses vendor-measured usage tokens —
input + cache_read + cache_creation + output, saturating-clamped, with
`used` + `numerator_source` serde fields and the byte heuristic demoted
to a disclosed fallback; assess probe 1.44 % → **40.17 %** against the
1 M catalog window; risk gate rebuilt as stricter-of an occupancy ladder
(≥90/75/40 % critical/warning/caution — `caution` display-only, gates
still fire from `warning`) and the historic absolute-headroom ladder,
arms agreeing on 200 k windows up to one boundary quantum), rm-873 (negative usage clamps
disclosed via per-field `metrics.disclosure_counters
negative_usage_zeroed:<field>`; clean journals emit none), rm-872
(`fmt_duration` carries sub-minute seconds across the minute boundary —
3597.0 s renders `59m57.0s`, 3599.4 s `59m59.4s`, and from 3599.5 s the
hour branch takes over as `1h 0m`, so minutes never render 60; HEAD
rendered 3597.0 s as `60.0m`). Two provider-reaped
implement attempts' uncommitted code was adopted after line-by-line
verification against the minted acceptance; the adopting attempt closed
the gaps the leftovers could not have known (risk gate, governance test,
i18n/color arms) and proved both risk arms red-first. Tests: 683 → 691
(8 new), all green at both scopes.

## Adoption forensics (provider-reaped attempts, pattern worth reusing)

attempt_seq=3 meant two prior implement attempts. Both died to provider
failures with **no typed artifacts** (dispatch + heartbeats + reap only),
but their uncommitted worktree state (mtime 2026-10-08 20:50) was intact.
The adoption checklist that worked:

1. Diff every dirty path hunk-by-hunk against the minted acceptance
   criteria (not against the attempt's own claims — there are none).
2. Enumerate acceptance sub-bullets NOT covered by the leftover; the
   leftover's tests only assert what the leftover implemented (here:
   numerator/zeroing/duration pinned, risk gate + governance + display
   arms absent). Those gaps are the adopting attempt's work list.
3. Treat red-first provenance per-arm, honestly: archived red runs exist
   only for legs this attempt reverted live (both risk arms, temp
   in-place ladder revert). Adopted arms' reds are **by-construction vs
   HEAD** (asserting values HEAD cannot compute — 40.17 vs 1.44, counters
   absent at HEAD) and are recorded as exactly that, never as archived
   runs. See the rm-871 implemented bullet on the wall.
4. MTIME-ORDER the adopted edits before writing prose about them
   (surfaces touched before the attempt's later code changes can still
   tell the pre-change story — half-finished doc/code consistency is
   invisible to tests).

## Prevention rules (repo-durable, reuse verbatim)

**PR-1 — Red-first temp reverts on a fully-uncommitted batch: /tmp copy,
never `git checkout`.** When the whole batch is uncommitted, HEAD is the
PRE-batch tree; `git checkout -- <path>` / `git show HEAD:<path> > <path>`
destroys the batch instead of exposing the bug. Protocol: `cp <file>
/tmp/<name>.good`, apply the unfixed variant by hand-edit, run the single
test expecting the exact failure, `cp /tmp/<name>.good <file>`, verify
md5 equality, re-run green. Recorded as
`solutions/workflow-issues/uncommitted-batch-red-first-reverts-use-tmp-copy-not-git-checkout.md`.

**PR-2 — Empty dispatch validation block ⇒ honest scope from real
surfaces.** This run hit BOTH variants: implement phase_results carry no
`validation_evidence`, so later dispatches derived
`changed_testable_surfaces: []`. For `required_scope: full` the rule is
already repo-durable (`empty-full-command-mirrors-ciyml-lane-for-lane.md`,
re-proven here). The targeted variant is newer: when the block says
`none` but the run's real changed surfaces exist, declare scope `targeted`
over the impacted-crate battery and say why — never declare `none` while
a testable surface changed, and never re-run full validation at the
targeted gate.

**PR-3 — Absolute-headroom gates encode a window size.** Any threshold
expressed in raw tokens (`available < 20_000`) is calibrated to one
window class; validate gate ladders at 32 k / 200 k / 1 M before shipping
them, and when adding a display-only tier (`caution`), verify every gate
consumer's match set (`warning | critical` at governance.rs:520/:1030,
app.rs:1423) rather than assuming level-name equality.

**PR-4 — Host/tool trivia that bites twice:** ratatui has no
`DarkYellow` (use `LightYellow`); the CLI package is named `agenttrace`
while its directory is `crates/agenttrace-cli`; cargo-deny 0.20.2 wants
`--all-features` BEFORE the subcommand; `/usr/bin/time` does not exist on
delegate hosts (shell `date +%s.%N` wall timing); CI smoke gates write
`ci-artifacts/` into the repo unless `AGENTTRACE_CI_OUT` points elsewhere
— point it at /tmp.

## Commit-gate seams (for the next phase, exact)

- Expected porcelain at commit: the 8 batch paths (5 M + 3 ?? from
  implement) **plus** this record and the new PR-1 solutions doc (both
  `??`) **plus** the 5 review-fix files (dc5540ae: M session_cache.rs,
  usage_occurrence_contract.rs, tui tests.rs, presentation.rs,
  docs/guides/governance-reports.md — diagnostics.rs was already M from
  the batch; 8f871301's further edits stayed inside those same files)
  = 15 entries, **plus** the docs-gate guard anchor in
  scripts/ci/check-docs-commands.sh (8f871301, see addendum; NOTE:
  byte-identical hunk to sibling run-5eb82325eb's uncommitted edit at the
  same line — duplicate hunk, first-lander wins at the commit gate)
  = 16 entries (11 M + 5 ??). Nothing else — ROADMAP.md in the worktree
  stays pristine (md5 `1a3a3818061393696ff8fb41a064950c`).
- ROADMAP delta: apply `delegate/3a319238…-scratch/roadmap-delta-CUMULATIVE.patch`
  (103 lines; regenerated at the 8f871301 review fix after the F2/F5/F6
  prose corrections, roundtrip re-proven: `git apply --check` clean,
  postimage md5 `f41133f48dcb2a165d3a5f6c1a5a4fd3` == `ROADMAP.after`;
  earlier postimages f88cf67b5c6487faff8e92514ff34620 (compound) and
  e47d98b48e997bfba924f1f0a152f685 (dc5540ae) are superseded). It is
  CUMULATIVE — it supersedes the f499765f mint patch; apply it alone.
  Then flip `rm-871/872/873` `implemented → done` with the commit hash
  (done-flips reserved to the commit gate per wall convention).
- CHANGELOG is owed entries under its FIRST `### Fixed` section by line
  index (the file has several — gotcha already on the wall): utilization
  numerator + risk ladder, negative-usage disclosure counters,
  fmt_duration minute-carry.
- must_remain_separate (stewardship 119c2a45): pricing.rs logic (rm-852
  in review), session_cache.rs internals (3f6b86bc's lane), and the
  canonical repo's staged release-eng wave (51 entries at ea5c41e —
  divergent integration line, preserve).

## Candidates left for the next cycle (ranked, evidence in hand)

1. **Tier-aware pricing (rm-164 rider refreshed; research C1)** — now
   first-party documented (Anthropic two-column ≤100 k / >100 k prompt
   length; 5-min cache write 1.25×, 1-h 2×); LiteLLM live diff shows 39
   rate-mutated + 15 window-mutated models. pricing.rs is contended —
   re-census before minting.
2. **Vendored catalog refresh** — 1,406 keys absent / 3 dropped vs live
   LiteLLM; the fbc4581f refresh is NOT on fork master — coordinate, do
   not double-land.
3. **Retained-thinking accounting (rm-871 acceptance (e), deliberately
   left open)** — vendor rule: thinking blocks count toward the window
   on keep-models and are stripped on strip-models; probe whether claude
   usage blocks expose the split before designing the lane.
4. **rm-408 present-but-zero usage blocks** — distinct from the negatives
   lane just landed (rm-873 owns silently-zeroed negatives; rm-408 owns
   usage-present-never-counted).
5. **Deps modernization riders (rm-044)** — ureq 3.4.2 vs 2.12 pin,
   crossterm 0.29 vs 0.28, rusqlite 0.40 vs 0.32, clap 4.6 vs 4.5;
   upstream frozen at 15ed07f, fork master moved to 190b706 — next
   assess must rebase its diff baseline first.

Evidence trail: implement notes at
`delegate/734d716cfbf1439999cf33969290c727-scratch/implement-notes.md`;
full validation logs at `delegate/2f51eaf17dd44a91bc473a3677447442-scratch/`;
prioritization `delegate/5e907d5f…-scratch/prioritization.md`;
stewardship `delegate/119c2a4580ce4ecba2cc1317f29c99e1.json`.

## Review-fix addendum (8f871301, 2026-10-09)

Independent review 834a5133 returned NEEDS_CHANGES with six findings. The
first fix turn (dc5540ae — result envelope written, engine re-dispatched)
landed F1's schema same-unit set (38 -> 49 + oracle + tui fixture + guide
sentence), F3's caution/注意 arm + zh pin test, and F4's diagnostics
comment — but its disposition mis-numbered the findings and left the
review's actual F2, F5, F6 open. This turn verified every dc5540ae edit
against the live tree (all held), then closed the remainder:

- **F2** — rm-872 prose rewritten to the shipped truth everywhere it
  ships (this record's outcome sentence; rm-872 acceptance example +
  implemented bullet in ROADMAP.after -> regenerated CUMULATIVE patch):
  3597.0 s -> `59m57.0s`, 3599.4 s -> `59m59.4s`, and from 3599.5 s the
  hour branch renders `1h 0m`; HEAD rendered 3597.0 s as `60.0m`. The
  confabulated `59m 59.8s` / `60m 0s` strings never occur — re-derived
  live this turn via a path-dep probe crate over the tree's own
  `fmt_duration`, with HEAD's `{:.1}m` branch read at @5ed9ebcc.
- **F5** — citations corrected to governance.rs:520 (gate) / :1030
  (trends) / app.rs:1423 — re-grepped live this turn — and the prose
  fallback value renamed to `heuristic_bytes` (what the binary emits),
  in PR-3, the rm-871 implemented bullet, and the regenerated patch.
- **F6** — the assess probe corpora were swept from the spool before
  review. fakehome-pos byte-equality is proven twice: live by the review
  (diff empty) and re-diffed empty this turn against the review's
  preserved copy (`/tmp/at-review-834a/fakehome`, md5 `01d4d365…` ==
  fixture). fakehome-neg/negout are unrecoverable (assess scratch swept;
  no generator survives in the event log), so their byte-match stays
  implement-recorded — the rm-871 notes bullet and rm-873 implemented
  bullet now say exactly that.
- **F4 echo** — the one-quantum honesty carried into this record's
  outcome sentence and the rm-871 bullet ("up to one boundary quantum"),
  and the oracle's two stale `37` literals re-based to 49.
- Schema ceiling re-censused at fix time: this tree 49 unique; sibling
  frontier 50 (run-5eb82325eb, minted max+1 over our 49); origin/master
  42 — no collision, no re-base.
- CUMULATIVE patch regenerated and roundtrip re-proven (103 lines,
  postimage md5 `f41133f48dcb2a165d3a5f6c1a5a4fd3`). Worktree ROADMAP.md
  untouched (md5 `1a3a3818061393696ff8fb41a064950c`).
- **Docs gate at 49 (F1 completion)** — running
  scripts/ci/check-docs-commands.sh at the bumped const exposed a latent
  contradiction: the gate dynamically requires the guide to say "session
  cache is schema 49" (const extracted from session_cache.rs) while a
  Pass-8-era guard (696206f) forbade any `schema 4` substring —
  unsatisfiable at every 40s const, since "schema 49" contains
  "schema 4". Guard anchored to `schema 4([^0-9]|$)` (standalone 4 only,
  intent preserved); gate rc0 at 49 with the sentence truthful.
- **Fresh emission digest** — this tree now prints
  `validation:v1:002d949a2686e7705e4941134fe56d824e47f75977c8665391b92f3aa7302edf`
  (local_validation_gate `--digest-base-sha 5ed9ebcc6355c225da0c19f76683d4c2dc99d8c7`
  over the 3-crate battery, rc0; envelope preserved in 8f871301 scratch).
  dc5540ae's envelope claiming the dispatch token `714f807c…` post-fix
  was hand-assembled (`digest_base: "unknown"` — the exact mistakable
  shape the script's docstring warns about); the dispatch token described
  the 15-path dispatch tree and must not be reused after tree changes.
- Retest (this turn, fresh): 3-crate battery 39x `test result: ok`,
  692/0, rc0; `cargo fmt --all -- --check` clean; clippy `-D warnings`
  rc0; release build rc0 (161 s); docs gate rc0; gate-wrapper battery
  re-run rc0. dc5540ae's core+tui-only "700/0" was not reproduced
  suite-for-suite — the 3-crate superset battery is the fresh green of
  record.
