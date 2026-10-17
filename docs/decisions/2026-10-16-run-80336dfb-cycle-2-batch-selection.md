# Cycle 2 batch selection — 2026-10-16, run 80336dfb

Selection + outcome decision record for conductor run
`80336dfba0ec4bc28c3ccda384776a3b` (repository-maintenance campaign
`34a54fc881d84ee9a495ba8e7ffa5381`, cycle 2), written at the compound phase
(attempt `15ea79959f1c4a9e9e46629e26c27e25`) from pre-review cycle evidence
only. Review and shipping outcomes are recorded by later phases; the next
cycle's assessment carries them forward. Base for the whole cycle: `3e90993`
(worktree `run-80336dfba0ec-80336dfb`).

## Baseline

Fresh adversarial assessment (attempt `d895a3a9`) at `3e90993`, porcelain 0:
`cargo test --workspace --all-features --locked` 1118/0 across 62 suites,
clippy/fmt/cargo-deny/manifests/docs gates rc0. The only standing red was
`check-install-runtime.sh` test D (install receipt vs pinned default ref,
the `rm-051` family) — pre-existing, with the clearing fix already living
uncommitted in sibling run `07f36b2f`'s batch (reconcile by title; do not
re-fix here). One gate was not adjudicable in-environment
(`check-output-contract`'s opening `--doctor` wedged >35 min against the real
HOME corpus under host load — environmental, not a regression).

## Selection

Method: in-thread `ce-prioritize`-style weighted scoring over the unresolved
wall (392 def rows @ `3e90993`), intersected with a live spool claims census
(14 newest envelopes — no LEAD claimed the statusline capture lane) and a
region-level seam census (18 dirty sibling worktrees, per-hunk — no sibling
hunk within ±500 lines of the target region). Full record:
`delegate/27ade644…-scratch/batch-selection.md` (conductor spool).

**Selected batch (one change unit): "Statusline budget truth: clamp-and-disclose
hostile capture costs and unparsable journal lines"** — roadmap row minted as
campaign-local `rm-1503` (title-keyed; see Twin note below), from assessment
findings F1 (medium) + F3 (low):

- **F1 (medium, correctness/integrity):** `statusline.rs:1573` `capture_cost_usd`
  took `cost.total_cost_usd` `.as_f64()` raw — no sign, finiteness, or magnitude
  gate — into the budget series (`:1634`) and render (`:1803`). Live PoC through
  the real host capture path: a `-50.0` capture on a `--weekly-budget 10`
  rendered "$10.00 budget — **$60.00 remaining**" (phantom headroom), JSON
  `remaining_usd 60.0`, zero disclosure anywhere.
- **F3 (low, disclosure):** a `1e400`-float journal line (serde rejects at parse)
  yields `lines: 1, daily: [], captures_unknown_payload 0` — any unparsable
  capture line vanished with no counter.
- **F2 (low, NOT selected into this batch):** relative
  `AGENTTRACE_SESSION_CACHE_DIR` plants `./relcache/statusline.jsonl` in the
  CWD — the is-absolute discipline the `rm-248` row already landed for
  `XDG_CACHE_HOME` does not reach the explicit override knobs. Parked as a
  rider on `rm-248` (next-cycle candidate 1).

Rejected-with-reason (from the selection record): `rm-1373` first missing lane
(multi-cycle by its own acceptance; evidence side advanced instead — all 8
storage shapes source-pinned from ccusage's rust adapters); `rm-577`
edge-class bins (0-scoring, no PoC); `rm-581` (deferred; demand rider added);
`rm-248` residual (tiny but off the batch's single-file footprint).

**Twin note:** `rm-1503` is a numeral twin of sibling org run `3a431714`'s
13:57Z band (first-lander). House rule: first-lander keeps the numeral; this
row reconciles BY TITLE at the commit gate with a fresh census (next free
≥ `rm-1507`).

## Outcome (pre-review)

Implemented uncommitted in the worktree (attempt `8a9b0344`,
ADOPT-then-complete: provider-dead attempts `d99c1f27` + `9a1b7012` left the
verified dirty diff that WAS the deliverable; verified line-by-line against
the stewardship request's 11 surfaces before adoption):

| Path | Change |
| --- | --- |
| `crates/agenttrace-core/src/statusline.rs` | +176/−29: `CaptureCostSample{Spend,Negative,Invalid}` classification before the day fold; `negative/invalid_capture_cost_samples` counters disclosed in BOTH views (text AND `-f json`); `StatuslineJournalStats.unparsable_lines` census; `remaining_usd` floored `[0, budget]` in both json arms; `rm-771` baseline-unknown seed logic byte-preserved |
| `crates/agenttrace-core/tests/statusline_budget_truth.rs` | NEW, 538 lines, 11 tests (hostile-negative behavioral, disclosure/counting incl. torn + `1e400` census, floor/report-arm, non-numeric nodes, zero-noise healthy corpus) |
| `CHANGELOG.md` | one `### Fixed` entry (test count truth-repaired 9→11) |
| `README.md` | `--budget` row truth note |

**Live flips (recorded at implement + independently reproduced at targeted):**
F1 PoC → `7d total $0.00 … $10.00 remaining` +
"1 hostile cost sample discarded (1 negative, 0 not a finite number) —
excluded from the 7d total"; F3 PoC → `journal.lines=3
journal.unparsable_lines=2` + "2 unparsable journal lines — counted in the
totals above, excluded from every capture stat".

**Validation (consumed as recorded at compound; NOT re-run there):**

- Targeted (`e82fbd46`): 148/0 — core lib `-- statusline` 51/0,
  `--test statusline_budget_truth` 11/0, `--test otel_export` 7/0 (adjacent
  consumer), CLI consumers `entrypoints` 54/0 + `output_safety_matrix` 7/0 +
  `mcp_server` 18/0; clippy `-D warnings` rc0; fmt rc0; `cargo check
  --workspace` rc0. Digest carried VERBATIM (nothing changed that turn).
- Full (`4e27670d`): `cargo test --workspace --all-features --locked` RC0,
  167 s incl. fresh build, 63 result lines all ok, **1129/0/0** = assess
  baseline 1118 + the 11 new tests exactly. `validation:v1:694c7e1f…`
  re-derived == dispatch before and after the battery.
- Red-first honesty: the zero-coverage baseline is the assessment's
  `test_gap:zero_coverage` census (no fixture anywhere in `crates/` planted a
  hostile cost), not an in-attempt red run — the adopted-delta provenance
  preempted one. Declared rather than papered over.

Reusable lessons (L1–L6, full text in the ROADMAP compound banner): classify
hostile values at ingestion and disclose in every render arm; the emission-time
digest rule; the adopted-delta verification protocol; reaper-recovery via
`quarantine/scratches/<ts>/<attempt>/` (this cycle recovered the roadmap band
patch byte-exact, md5-verified against the envelope record); fleet disk
pressure handling; numeral-twin title reconciliation.

## Standing reds (carried, not this batch's)

1. `check-install-runtime.sh` test D (`rm-051` install-ref family) — re-proven
   at `3e90993`; clearing fix uncommitted in sibling `07f36b2f`.
2. `check-output-contract` under real-HOME `--doctor` on a loaded host —
   environmental hazard (budget or scope HOME), not a repo defect.

## Ranked next-cycle candidates

1. **`rm-248` residual env-knob arm** — `AGENTTRACE_SESSION_CACHE_DIR`
   (and siblings at `pricing.rs:525-535`, `statusline.rs:194-205`) accept
   relative paths; extend the landed `is_absolute` parity. Live PoC banked.
2. **`rm-1373` acceptance arm-1, first lane** — cheapest is a jsonl lane
   (Grok or Amp); all 8 missing-lane storage shapes are now source-pinned
   from ccusage's rust adapters (this cycle's rider).
3. **Fog `rm-1500` (8ae91e99)** — lib.rs aggregate/render lane for
   non-finite costs: the other arm of the hostile-cost family this batch
   closed on the statusline lane.
4. **Standing red `rm-051`** — title-reconcile the sibling clearing fix
   before any re-attempt.
5. **Gate hermeticity** — the `--doctor` wedge above; also re-check
   `check-plugin-version` posture at the next base per the standing banner.

## Evidence trail

Typed envelopes (conductor spool `delegate/`): assess `d895a3a9…`, research
`1ce14d32…`, roadmap `7edaa2fd…`, prioritize `27ade644…`, stewardship
`7ec93804…`, implement `8a9b0344…`, targeted `e82fbd46…`, full
`4e27670d…`. Roadmap band patch (md5 `3caf733d0fce9b28c4085010647f1215`,
postimage md5 `66076ec58c6d3746711d1e448a697b7f`, 393 def rows) recovered
byte-exact from `quarantine/scratches/20261014T200103Z/` and mirrored with
the compound chain (band patch + compound patch + chained postimage, md5s
inside) at `delegate/15ea7995…-scratch/` and
`/home/agent/.cache/at-compound-15ea7995/`. The commit gate applies
wall ← band ← compound in order and stages the four batch paths plus this
record and `ROADMAP.md`.
