# Cycle 3 compound record — run 8d2e5dd6db384793b65ea2085ffd9767 (repository-maintenance de7ac553b62d4fee9fed633a3eab9d45, cycle 3)

Written at compound attempt c4304e5a51e448c5b9e7078625d0c70a (redo of provider-dead
dded3556: typed result absent, no scratch, event log = `turn_start` → banking-failed at 0
output → `session_reaped:provider` → completed-failed; nothing durable, nothing adopted), at
worktree `run-8d2e5dd6db38-8d2e5dd6` @ `794b6e5`, **pre-review**, ZERO test execution
(compound contract — every outcome below is consumed from the recorded targeted/full
evidence, not re-run). Review and shipping outcomes land after this phase; the next cycle's
assessment carries them.

Base wall at build time: `origin/master` = `d2a02af`, `ROADMAP.md` md5
`695d7ee446f34ee546678176433bd9ab`, 400 def rows, def-max `rm-1502` (e4725315's cycle-1
mints rm-1500..1502 landed first; next free rm-1503). Batch tree at build time: exactly the 5
implement-delta M files, +121/−5, `git diff` md5 `36dec0f0a89bb7837e561cf03d23596a`
(byte-identical t0/t1 across targeted and full — never touched after implement).

## Phase ledger (8 phases, 5 provider deaths, 1 same-run adoption)

| phase | attempt | outcome |
|---|---|---|
| research | fde5eefd → **db113547** (adopted by f93f71c0) | fde5eefd dead 23 s, zero work; db113547 ran 07:49–08:23Z, wrote dossier + typed result at 08:22Z, then reaped before final message — **adopted** after full identity-leg re-verification (run/action/phase match, worktree porcelain 0 at dispatch/finish) |
| roadmap | 546809f8 | band vs canonical wall 3e90993; spool-side; scratch **reaped pre-compound** → rebuilt below |
| prioritize | 75696a65 | batch selection memo (spool; reaped) |
| stewardship | 18ec31b9 | request + mid-phase wall-restructure absorption (spool; reaped) |
| implement | e638e883 | 5-file delta, red-first, uncommitted |
| targeted_tests | e3402207 (dead) → 850972c7 | e3402207 dead ~36 s, nothing adoptable → redone from scratch |
| full_tests | df5a1f48 (dead) → b550d07b | df5a1f48 dead ~80 s, nothing adoptable → redone from scratch |
| compound | dded3556 (dead) → c4304e5a | dded3556 zero-output death → this record |

## Batch outcome — 'INGESTION TRUTH HARDENING' (prioritize 75696a65, stewardship 18ec31b9)

Selected ONE coherent batch from this run's own mints (546809f8): rm-1428 (lead) + rm-1429
(pair); rm-1427 ceded (blocked-on-environment). Both units implemented red-first and
validated **uncommitted** in this worktree:

1. **rm-1428 var_os env-parity (lead, CU-A).** `parser.rs` `opencode_storage_rel` now reads
   `std::env::var_os("OPENCODE_DATA_DIR")` with a parity comment vs `discovery.rs:1221`
   (the reference lane); empty-string guard and the `rel_from_root`/marker-fallback semantics
   preserved — no behavioral change for UTF-8 values (+72/−1). Display siblings decided per
   the stewardship contract: `governance.rs` PATH prepend rebuilt as `OsString` (+12/−4 —
   non-UTF-8 PATH segments survive intact), `presentation.rs` HOME pinned display-only (+4 —
   lossy render by design; ingestion owns parity). CHANGELOG `Unreleased/Fixed` entry (+7).
2. **rm-1429 multi-byte normalization pins (pair, CU-B — test-only, no-promotion rule held).**
   `pricing.rs` +31: `normalize_model_survives_hostile_multi_byte_names` pins the tokscale
   #1440 oracle shapes (len()−8 mid-char cut, provider path split, CJK date suffix, emoji
   version suffix, `strip_date_suffix` 9999+ISO arm, `strip_version_suffix` 12.5/v3.1/rocket
   arms); `parser.rs` `cap_session_cwd_backs_off_to_char_boundaries` pins the truncation
   back-off (4095+6-byte mid-char cut backs off to whole chars; exact-4096 no-op; all-CJK
   4098 bytes → 1365 chars). Zero production-code change in pricing.rs beyond the test mod.

## Validation outcomes (consumed, NOT re-run)

- **targeted 850972c7**: core `--lib` **384/0** (381 pre-batch + 3 new tests); filtered
  `opencode_storage_rel|cap_session_cwd|normalize_model|governance` **28/0** (all three new
  tests individually visible); `discovery_contract` **104/0**, `usage_occurrence_contract`
  **5/0**, `governance_cost_basis` **4/0** (added as a changed-surface contract);
  `agenttrace-tui --all-targets` **60/0** (covers presentation.rs); clippy `-D warnings` rc0;
  fmt rc0. Zero fixes needed. Delta md5 `36dec0f0…` byte-identical in/out.
- **full b550d07b**: `.github/workflows/ci.yml` verbatim (empty `full_command` → bde3b724
  precedent), 30 lanes — locked trio **1092/0** (exact: 1089 clean baseline under neutral
  TMPDIR + 3 new batch tests; the 3 assess-time TMPDIR-false failures reconciled and pass),
  clippy rc0 (incl. `-p agenttrace` — closes the implement/targeted clippy-scope gap), fmt
  rc0, msrv 1.89 rc0, release build rc0, conformance 13 cases (1 xfail-pinned), all green
  smoke gates rc0, changelog-structure rc0 (first gate run over the batch's CHANGELOG edit).
- **Standing reds at clean HEAD 794b6e5 — wall-owned, NOT batch-owned** (surfaces
  byte-proven untouched by porcelain): `check-plugin-version` rc1 (v0.0.4 section-missing),
  `check-install-runtime` rc1 (test D, rm-051), actionlint SC2001 `ci.yml:192`, `cargo deny`
  RUSTSEC-2026-0346..0349 (rustls ×4). Fixing them here would edit tracked files outside the
  batch and corrupt the implement delta — disclosed instead, per fleet precedent.

## Red-first record

`opencode_storage_rel_accepts_non_utf8_data_dir` (OPENCODE_DATA_DIR = non-UTF-8 `OsString`
via `OsStringExt` over a real `storage/session/proj-1/ses.json` tree + symlink) FAILED
pre-fix: `left: None, right: Some("storage/session/proj-1/ses.json")` — the exact predicted
silent marker-heuristic degradation; green post-fix. `cap_session_cwd_backs_off_to_char_boundaries`
pin ran green verbatim against the existing implementation (prevention unit: the oracle is
the tokscale #1440 panic class, not a live bug here).

## Reap ledger & provenance recovery (why the roadmap layer below is a rebuild)

Reaped before compound: roadmap 546809f8 scratch (postimage md5 6f1b27d3 / patch / seam /
builder + the `/tmp/at-roadmap-546809f8` mirror), prioritize 75696a65, stewardship 18ec31b9,
implement e638e883, targeted 850972c7 scratches and every `/tmp` mirror. Surviving: the eight
typed envelopes + full b550d07b's scratch (`full-validation-record.md` + 30 lane logs +
`/var/tmp/at-ft-b550d7`). The roadmap band therefore existed nowhere durable; per the
13212e2f reaped-band precedent this compound REBUILT it from the typed envelope
(delegate/546809f8….json — mints, riders, cessions, census, and verification facts verbatim;
rider PROSE is reconstructed from the envelope's descriptions and the rebuild is declared
in-band). The rebuilt band is REBASED onto the current wall `d2a02af` (md5 695d7ee4, 400
rows), absorbing the stewardship-recorded gate-side rebase instruction after the mid-cycle
wall restructure (ecf84dc1: repo-wide prune, ROADMAP rewritten 367 rows / def-max rm-984 /
rm-142x absent, then re-grown by sibling bands to 400). At the commit gate, apply NOTHING
from reaped spool history — the compound postimage REPLACES the reaped 546809f8 patch.

## Durable lessons & prevention rules

- **L1 — evidence durability (3rd recorded recurrence: 13212e2f → 1a4cce3a → this run):**
  the spool reaper sweeps current-run scratch within a day; early-phase scratch and every
  `/tmp` mirror were gone by compound. RULE: each phase mirrors its durable artifacts to
  `~/.cache/at-<phase>-<id>/` (targeted/full did — b550d07b's scratch survived) AND the
  roadmap phase embeds its rider texts VERBATIM in the typed result (this run's envelope
  carried descriptions only, forcing prose reconstruction — the band below is now the
  authoritative copy and lives in-tree at the gate).
- **L2 — provider-death triage:** 5 deaths this cycle; 4 left nothing adoptable (23 s–80 s,
  zero commands of substance); 1 (research db113547) completed its typed result and was
  reaped before final-message delivery — the same-run recovery pattern (adopt + re-verify
  identity legs, done by f93f71c0) worked exactly as codified in
  `docs/workflow-issues/provider-dead-attempts-may-leave-real-worktree-diffs.md` rules 1–5.
  Event-log forensics (turn_start → progress depth → reap → completed-failed) is the first
  triage step every time; zero-output deaths need no scratch sweep beyond confirming absence.
- **L3 — environment discipline:** every battery ran private `CARGO_TARGET_DIR` + neutral
  `TMPDIR` (3 TMPDIR-false failures at assess reconciled to green at full — the A/B proof
  holds); fleet `/home/agent` at 99 % disk — keep mirrors in `/var/tmp` or minimal text-only.
- **L4 — wall churn:** the wall restructured mid-cycle and advanced 12+ commits; building
  the compound band against the CURRENT tip (not the roadmap phase's base) absorbs the
  rebase at compound instead of deferring it to the gate. Numerals are fleet-ledgered and
  reconcile BY TITLE, never id.

## Next-cycle candidates & context (pre-review)

1. **rm-1427** (this run's third mint) — Antigravity session-store freshness; first pick the
   moment a host has `~/.gemini/antigravity` (probe not executable on the 2026-10-15 fleet
   host, recorded by prioritize 75696a65).
2. **rustls 0.23.45 → 0.23.46** — clears all four RUSTSEC-2026-0346..0349 standing deny
   reds (sibling research f4bdc13e C1, cited by our full record; re-verify advisory-db +
   crate availability at adopt time). One-bump deny-green.
3. **check-install-runtime test D (rm-051 standing red)** — derive the expected ref from
   install.sh (fix shape recorded on the wall's rm-012 gate-red rider).
4. **actionlint SC2001 `ci.yml:192`** — the conformance audit's `sed 's/^/  changed: /'`
   (printf-class one-liner).
5. **rm-939 config-side watch** — pi-coding-agent 1.2.0 journal-side DISCHARGED (format
   docs byte-identical); new `schemas/` dir is config-side only; watch 1.2.x > 1.2.0.
6. **Standing channel watches:** gemini 0.65 stable promotion (rm-715 trigger unpulled;
   0.64.0 current), codex 0.163 stable (rust-v0.163.0-alpha.11 bodyless; state sqlite.rs sha
   e35a074d unchanged), tokscale #1434–#1440 open (#1440 = the multi-byte normalize oracle;
   #1435 = Antigravity process-residency mechanics), models.dev api.json md5 c3099323
   flat-shape 229 providers / 8,834 models vs LiteLLM 4,084 keys (re-count on refresh),
   CC 2.1.299 attribution-block/queue door/usage warnings/idle compaction.
7. **Wall fog:** this run's numerals rm-1427..rm-1429 are fleet-ledgered; the landed wall's
   next free is rm-1503 — next cycle's roadmap mints from its own fresh live census.

## Commit-gate seams

1. Stage EXACTLY: the 5 implement paths (`crates/agenttrace-core/src/parser.rs`,
   `crates/agenttrace-core/src/pricing.rs`, `crates/agenttrace-core/src/governance.rs`,
   `crates/agenttrace-tui/src/presentation.rs`, `CHANGELOG.md`) + this record doc +
   `ROADMAP.md`.
2. `ROADMAP.md` = the compound postimage (delegate/c4304e5a…-scratch/ROADMAP.postimage.md,
   md5 `c565a985c32771b13e155b920ee8800a`): envelope-rebuilt roadmap layer + compound riders,
   built against wall `d2a02af` (md5 `695d7ee4…`). Apply via
   `patch -p1 < delegate/c4304e5a…-scratch/ROADMAP.patch` from the repo root against a
   pristine `d2a02af` ROADMAP, or carry the postimage; then re-verify
   `md5sum ROADMAP.md` == `c565a985c32771b13e155b920ee8800a`. If origin/master moved again,
   rebase by title (riders to owner rows, banners at head, mints at foot) — never renumber.
3. Apply NOTHING from reaped spool history (the original 546809f8 patch no longer exists).
4. Flip `rm-1428` + `rm-1429` candidate → implemented WITH the code (done-flips reserved to
   the shipping gate — rm-012 convention; the EXECUTED/VALIDATED riders carry the evidence).
5. Standing reds stay disclosed as wall-owned (plugin-version, install-runtime, actionlint,
   deny) — do not fix inside this commit.

## Census & verification legs

- Wall in: `git show origin/master:ROADMAP.md` → md5 `695d7ee446f34ee546678176433bd9ab`
  (builder-asserted), 400 def rows, def-max rm-1502, zero dup ids.
- Postimage: 403 def rows, def-max rm-1502, zero dup ids, managed render footer still the
  last line, all 11 rider-anchor rows present exactly once, banner order compound → roadmap
  → prior banners, insertion-only diff (40 added lines, 0 deletions; 13 hunks).
- Round-trip: `git apply` on a pristine wall copy → md5 `c565a985c32771b13e155b920ee8800a`
  == postimage; `patch -F3 -p1` clean, no fuzz, byte-exact.
- Original-build facts preserved from the roadmap envelope: base 3e90993 md5 `1d59adda…`
  (392 rows), original postimage md5 `6f1b27d3…` (395 rows), 13-hunk pure-additive patch,
  fog stack test rc0 with -F3 ≤2 fuzz at the mint seam.
- Builder + wall + postimage + patch + verification mirrored at
  `delegate/c4304e5a51e448c5b9e7078625d0c70a-scratch/` and `/var/tmp/at-compound-c4304/`.
