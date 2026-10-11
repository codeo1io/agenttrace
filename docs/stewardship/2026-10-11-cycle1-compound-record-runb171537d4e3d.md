# Cycle 1 compound record — run `b171537d4e3d4e1ebe8904ee2ccdb1fb` (repository-maintenance `aa99dbf962a74dd593db6ce7f7647f41`, cycle 1)

Date: 2026-10-11. Base: `dbc5b767ab19c340943f7ada7d8fee69df6cf023` (== origin/master at cycle start). Status at compound: full suite green on the batch-carrying tree (927/0/53 rc0, +7 tests over the 920/0/53 baseline; validation digest `validation:v1:44345ab1ea6635afebcfe7be25ed3a1cebfdde395134662fdb21f02b1e932921`, re-derived via the engine's own `validation_policy.validation_digest` — see L1). Code batch UNCOMMITTED (6 files, +340/−50) awaiting the commit gate together with the spool patch chain described below.

## Phase ledger
| phase | attempt | outcome |
|---|---|---|
| assess | 785b4707 (30644b33 provider-reaped, ~23 s / 4 events, redone-from-scratch declared) | fresh adversarial pass @dbc5b76; F1 git-drain byte caps, F2 bidi doc overclaim, F3 TUI raw display, K1 flag-pair residual; envelope gates fmt/clippy/920-0-53 green |
| research | d3dbc0b1 | ecosystem deltas: ccusage #1859 codex flex config.toml, #1838 claude id-class split, #1824/#1823 Copilot (ours ahead), v20.0.29 untagged, upstream frozen 15th check |
| roadmap | 61636bb6 | ZERO mints, 5 riders + banner (every candidate title-matched a landed owner); spool patch + postimage, apply --check rc0 |
| prioritize | 610b0233 | batch "Adversarial-input honesty — display, actions, drains" = rm-933 slice 2 + rm-052 git caps + rm-301 pair |
| stewardship | 6757a4a5 | structured request; canonical dirty-28 foreign state flagged preserve-only |
| implement | e7beab23 | 3/3 units landed, red-first tests + live red→green PoC |
| targeted_tests | 6e517a4d (8bf83c46 first, digest rejected — see L1) | engine-derived scope none; focused suites 318/60/94 + fmt/clippy rc0 |
| full_tests | 2c874f9c | full_command VERBATIM rc0, 53 suites, 927/0 |

## Batch and where it lives (all pending the commit gate)
- **rm-052 git-lane byte caps** — `crates/agenttrace-cli/src/upstream.rs`: `UPSTREAM_GIT_MAX_BYTES` (256 KiB, sibling of `NPM_REGISTRY_MAX_BYTES`), `GitRunError::OverCap`, chunked capped drains (`DrainOutcome`), kill-the-moment in-loop check, under-cap stash; 3 new tests; deadline pins (rm-543/583/731) green.
- **rm-933 slice 2 TUI display sanitization** — `crates/agenttrace-tui/src/filters.rs` `sanitize_display` via the existing core path dep, choke point inside `short()` + 5 raw-site wraps (`explorer.rs`, `presentation.rs`); 4 tests; `statusline.rs` doc narrowed to directional-only truth (predicate bytes unchanged).
- **rm-301 pair residual** — `crates/agenttrace-cli/src/main.rs`: `has_followup_action` / `has_post_pricing_action` split; pair runs both actions in either order (live PoC: cache cleared + pricing.json written, 1319 models).
- Landing order for the gate: ROADMAP.patch (61636bb6) → compound.patch (this cycle's EXECUTED bullets + this record) → the 6-file code diff. CHANGELOG riders reserved to the gate.

## EXECUTED status written into the wall (this record's patch)
rm-052 / rm-301 / rm-933 get EXECUTED bullets appended under their pending 61636bb6 riders. NO status flips: rm-933 stays candidate (its acceptance arms — non-directional Cf widening through export lanes, U+FFFD replacement — remain open), rm-052/rm-301 were already implemented/done.

## Prevention rules / reusable lessons (this cycle, first-hand)
- **L1 — digest authority.** The fold gate re-derives `validation:v1:` via `hermes_conductor.validation_policy.validation_digest(base, worktree)`. The local gate ENVELOPE's `digest` field is a different derivation (digest_base "unknown") — declaring it is rejected (cost one re-dispatch this run: attempt 8bf83c46 declared the envelope string; 6e517a4d declared verbatim after deriving with the engine's own code). RULE: before declaring any digest, run the engine function against the current tree; declare what it prints (== the work-order string when nothing executable changed this turn).
- **L2 — wall id-format duality.** ROADMAP def rows come in two shapes: legacy `- id: rm-NNN` (19 rows) and current "- id: \`rm-NNN\`" (408). Any census/tail grep must UNION both or it silently undercounts and mis-locates the def-max.
- **L3 — numeral ≠ title.** Landed code comments may cite a row numeral whose wall subject has drifted (rm-052 cited by the npm cap while the row's subject is npm-channel truthfulness). Reconcile by TITLE at integration; never mint a new numeral off a code-comment citation alone (this cycle: zero mints for exactly this reason).
- **L4 — choke-point reuse.** When a lane needs an existing landed predicate, check the crate's path deps first: `agenttrace_core::is_bidi_format_control` was already re-exported, so the TUI arm needed zero new dependencies and zero predicate moves.
- **L5 — disputed-evidence notes close first-hand.** rm-933's row carried a disputed static read ("verify terminal bytes at implement either way"); the implement slice resolved it at the cell level (TestBackend-grade tests) and the EXECUTED bullet records the resolution.
- **L6 — reaped attempts.** The 30644b33 forensics convention (event log + artifact absence → redo from scratch, declare) worked again; no phantom adoption.

## Context for the next maintenance cycle
1. **Standing leads (evidence-shaped, ready):** rm-164 codex flex config.toml resolution (ccusage #1859 merged 2026-10-10T21:51Z — tier is config-resident; sizing + fixture shapes in the pending rider; consumption rides unlanded rm-957) and rm-601 claude id-class split (#1838 merged — red-first fixture shape: two responses sharing placeholder id "gateway-1" report 1x on the current tree, expected 2x; pick when sibling run 331684778f6b's rm-251 arm has landed).
2. **Still open on rm-933:** non-directional Cf widening (U+FEFF/U+2060/U+00AD/U+180E) through export lanes + the U+FFFD-replacement acceptance arm; the JSON carve-out (main.rs) and otel lane stay raw by design decisions recorded in-row.
3. **Numeral census at compound time:** wall def-max rm-939; fleet spool def-row frontier rm-965 (636bf1d1 postimage, run 331684778f6b); rm-966 seen in prose pointers only → NEXT FREE rm-966 after a fresh live census; reconcile by title, never id.
4. **Sibling/cross-run unions pending the gate:** 514cf6cb bullet (run 331684778f6b) vs our rm-301 EXECUTED bullet — same subject, title-reconcile; rm-601 rider's fixture shape vs that run's rm-251 arm.
5. **Repository hazards:** canonical `/work/projects/agenttrace` at the same commit but carrying 28 foreign staged entries (workflows/plugin/issue-template edits) — preserve-only, never absorb; upstream luoyuctl frozen (15th consecutive check) — next sync cycle should re-check before any rm-448 sizing.
6. **Watches:** ccusage v20.0.29 UNTAGGED (release PR #1847 open; last tag v20.0.28) — re-validate flex/credit semantics at the tag before citing; models.dev flat 226/8443 vs bundled 3130 (rm-957 premise); pi harness 1.1.0 == installed (no journal-format drift).
7. **Validation records for the gate:** engine digest `4434…` (see L1) on base+batch; full suite 927/0/53 rc0 (/tmp/at-full-2c87/gate.log, envelope result-3961806-418631999.json); focused logs /tmp/at-tgt-6e51/.
