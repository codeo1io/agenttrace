# Cycle 2 compound record — run 18f9b98a3492 (repository-maintenance 5cdbf259, cycle 2)

- **phase**: compound (action `compound:compound`, attempt `3dfde8465b924146b4c0902d3e4876ca`)
- **date**: 2026-10-21 (cycle evidence spans 2026-10-19 → 20)
- **wall**: df7c259076130487df90aecf39159fe11c5a7567 (== origin/master; porcelain in/out: the cycle-2
  implement delta, 13 M + 1 ??, byte-identical before/after this phase — compound touches NOTHING in the worktree)
- **tree delta (chain of custody)**: implement diff md5 `af7eb1bb15bdd98ee86b163195b149cd` (13 M files,
  708 insertions / 18 deletions); untracked fixture
  `crates/agenttrace-core/tests/fixtures/codex-compressed-rollout/rollout-2026-10-15-10-00-00-3f2a1b2c.jsonl`
  md5 `ecbd2d61c908b74072a36951d427823d`
- **roadmap chain**: wall ROADMAP.md md5 `42c833b62b239f77c64e0f9f7d1072e4` → band postimage
  `b39ec9a042b2613bf6a19cf215dfca48` (roadmap 7699dfb5) → **compound postimage
  `6fe52f9ed88479926961f61d47735f42`** (this phase). Round-trip proven in /tmp/rt-3dfde846:
  band patch → b39ec9a0, + compound-only patch → 6fe52f9e, and the single full-chain patch
  → 6fe52f9e. The compound banner states the gate's apply order.

## Disposition chain (pre-review evidence consumed; nothing re-validated locally)

| phase | attempt | outcome |
|---|---|---|
| assess | 3f9d2761 | REAPED mid-turn (provider streaming failure, 12 events) — zero durable work, redone |
| assess | 9158ade7 | 1424/0 battery, fmt+clippy rc0, live PoCs: rc101 ×2 (lib.rs:3201:17, reports.rs:1010:27), zst twin $0.0042×2, zst search 0-match false-clean, 1e400 silent-drop |
| research | 0418480c | upstream frozen at 15ed07f2; RC-1..RC-7 (argon unpriced in bundle; ccusage v20.0.36 mappings; codex twins DESIGNED transient state) |
| roadmap | 7699dfb5 | band spool-side: 2 mints (rm-2056, rm-2057) + 7 evidence-refresh riders; 474 def rows, def-max rm-2057 |
| prioritize | 148c507c | batch 'hostile-input truthfulness': P1 rm-2057, P2 rm-2056, S1 rm-1500 TUI, S2 csv gate, S3 rm-1533 1050 arm |
| stewardship | da9cb8b1 | structured stewardship request; surfaces re-verified at wall |
| implement | ab31a619 | REAPED 01:17:22Z after ~48 min of real work — durable trail = the uncommitted worktree delta |
| implement | 6b978103 | ADOPTED ab31a619's delta (mtime-window + event-log lineage + private-worktree proof) and completed validation |
| targeted_tests | 63d26aec | trio rc0 (478-test core lib, 114-test count_honesty block), fmt+clippy rc0, digest unchanged |
| full_tests | dfad4b18 | REAPED at ~23 s — zero durable work, redone |
| full_tests | 31642f31 | 90 suites 1433/0/0 (baseline 1424 + exactly the 9 new pins), fmt+clippy rc0, digest `validation:v1:975de2a8…` identical pre/post |

## Status updates delivered (in ROADMAP.compound-postimage.md)

1. **rm-2057 → implemented** (+VALIDATED rider): both extreme-ts clamps landed at consumption sites;
   explicit-file lane re-proven live rc0 ×2 (was rc101 ×2); red-first pins at both anchors.
2. **rm-2056 → implemented** (+VALIDATED rider): stem-twin admission collapses to the plain twin BY NAME
   in both lanes, cold+warm; disclosure via `--audit` JSON keys + `--doctor` row; in-repo fixture + 4 pins;
   rm-548/-d-keeps-both-names and rm-992 zst-only-still-admits stances preserved.
3. **rm-1500 EXECUTED arms (a)+(b) rider** (row stays implemented): TUI folds + total_cmp comparator via
   shared `finite_cost`; csv cost cells 0.0000. Arm (c) — intake `1e400` silent-drop disclosure — OPEN.
4. **rm-1533 EXECUTED arm rider** (row stays candidate): 1050 → gemini-3.1-flash-lite identity arm + pin;
   1418/1419 argon arms refresh-blocked on rm-176; unmapped-id WARN/exclusion clause OPEN.
5. Compound banner at the banner head (above the cycle-2 band banner); **zero mints** — census 474 in == out.

## Errata of record

- **E-1 (transcription, corrected here)**: the implement envelope's artifacts list names the fixture
  `crates/agenttrace-core/tests/codex-compressed-rollout/…`; the live path is
  `crates/agenttrace-core/tests/fixtures/codex-compressed-rollout/…` (verified at the wall this phase).
  The CHANGELOG, tests, and postimage riders carry the correct path.
- **E-2 (verified-true, no action)**: full_tests claimed +9 tests vs the assess baseline; recomputed from
  the tree delta: 9 added `#[test]` items (4 count_honesty_admission, 1 tui tests.rs, 1 lib.rs,
  1 reports.rs, 1 parser.rs, 1 csv_export.rs) — 1424 + 9 = 1433 exact.

## Reusable lessons / prevention rules (this cycle's increment)

- **L1 — reap-resilient disposition is now standing protocol.** Three reaps this cycle (3f9d2761,
  dfad4b18: zero durable → redo from scratch; ab31a619: ~48 min of real work salvaged). The adoption
  forensics that worked: (a) no typed envelope + event log ends mid-turn → nothing claimed; (b) uncommitted
  delta with file mtimes INSIDE the reaped session's event-log window + worktree private to the run →
  adopt, then re-verify every gate yourself. Never adopt without both legs.
- **L2 — precedence by NAME, not by walk order.** The stem-twin fix admits the plain `.jsonl` by name
  precisely because the discovery walk feeds newest-mtime-first (the duplicate-journal survivor hazard:
  first-seen == newest-mtime, filesystem-adversarial). Prevention rule: any admission/dedup/suppression
  policy that must pick a survivor derives precedence from a stable property of the data (name, bytes),
  never from enumeration order or mtimes.
- **L3 — clamp-at-consumption for hostile calendar extremes.** `checked_sub_signed(..).unwrap_or(MIN)`
  (quota window) and an explicit underflow arm rendering `(NaiveDate::MIN, 0.0)` (svg day series) keep
  spend fully attributed and honest where a reject would drop it. Days that cannot exist render AT the
  floor with an honest 0.0 instead of panicking the whole view.
- **L4 — machine-consumed cells are contracts.** CSV cost cells gate to `0.0000`; NaN/inf text in a float
  column breaks downstream parsers. When landing a rendering wave, sweep EVERY surface that renders the
  field — grep by format verb and fold pattern across all crates; the cycle-1 finiteness wave landed CLI
  lanes and missed an entire crate (tui) plus csv cells.
- **L5 — `partial_cmp().unwrap_or(Ordering::Equal)` is not a sort order.** NaN makes it inconsistent;
  use `total_cmp` over gated values. Same family as the comparator fix this cycle.
- **L6 — verify every inherited number before it enters a durable artifact.** +9, the 9 pins, the fixture
  path, 708/18, 1433/0/0 — each recomputed or re-grepped at compound time; E-1 was caught this way.
- **L7 — roadmap hygiene at compound: zero mints, flips only for FULL acceptance, `done` reserved to the
  commit gate (rm-012), census in == out, footer last.** Executed: 2 flips, 4 riders, 1 banner, 474→474.

## Candidates / context for the next maintenance cycle

1. **rm-1872 — zst content lane** (natural correctness lead): compressed twins are unfindable in
   `--search` (0-match false-clean; footer counts compressed bytes, 849 ≠ decompressed). The rm-992
   multi-frame decoder is reusable; rm-2056's admission work made the population of twins visible via
   `--doctor`, so the search gap is now measurable. Codex 0.163's rollout migration makes twins a DESIGNED
   transient state in the wild — disclosure is load-bearing.
2. **rm-1500 arm (c)** — intake `input_tokens:1e400` silent drop (cliff2.jsonl PoC: parsed=1, model
   'default', cost 0.0, no disclosure). Deliberately deferred at prioritize 148c507c (R4).
3. **rm-176 — pricing snapshot refresh**: bundle still 2026-10-08 / 3,130 rows with 0 argon rows while the
   live source ships 4; the 2026-10-20 06:00 UTC drift-cron deadline recorded by this cycle's evidence has
   passed by compound time — the next assessment owes a fresh drift census before anything else.
   Unblocks rm-1533's 1417/1418/1419 arms.
4. **rm-1533 residual** — unmapped-id WARN + cost-exclusion semantics (ccusage #1897 is defining the same
   semantics upstream; converge, don't invent).
5. **Upstream watch**: master 15ed07f2 frozen since 2026-10-06; same 6 unmerged PRs; fork's checkout-action
   pin ahead of dependabot #259/#319. No merge pressure.
6. **Fog discipline** (unchanged): def-max on disk rm-2057; next free numeral rm-2058 ONLY after a fresh
   live census at the next wall; reconcile by title, never numeral.

## Artifacts and verification

- builder: `build_compound_delta.py` (fail-loud, exact-once anchors; run: `python3 build_compound_delta.py`
  → census OK, postimage md5 6fe52f9e)
- postimage: `ROADMAP.compound-postimage.md` (md5 6fe52f9ed88479926961f61d47735f42)
- patches: `roadmap-compound-only-3dfde846.patch` (band→compound) and
  `roadmap-compound-3dfde846.patch` (wall→compound, full chain)
- round-trip: `cp <wall>/ROADMAP.md /tmp/x/ROADMAP.md && cd /tmp/x && git apply
  ~/.cache/at-roadmap-7699dfb5/roadmap-delta-7699dfb5.patch && git apply
  roadmap-compound-only-3dfde846.patch && md5sum ROADMAP.md` → 6fe52f9e…
- mirrors (sweep-proof): spool root `roadmap-compound-3dfde846-mirror/`, `/home/agent/.cache/at-compound-3dfde846/`,
  `/tmp/at-compound-3dfde846/` (twin)
- commit-gate option (not done here): land the dossier under `docs/stewardship/` per the 963662941/38b020c7
  convention; nothing in-tree was written by this phase.
