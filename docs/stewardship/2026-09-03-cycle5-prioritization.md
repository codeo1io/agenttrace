---
artifact_contract: "ce-handoff/v1"
created_at: "2026-09-02T16:42:51Z"
title: "Cycle 5 prioritization — honest coverage, honest cache, honest math"
summary: "Scores every unresolved ROADMAP.md item after cycle 4 (assessment pass 8 F8-1..F8-10, research pass 7 candidates 43-49, standing capability leads) by impact, risk, effort, dependency, and strategic value; selects the cycle-5 batch: F8-1 governance coverage honesty (lead) + F8-2 truthful discovery accounting + F8-3 session-cache eviction + F8-5/F8-6 float hygiene + F8-7/F8-8 docs-contract check as CU-11..CU-15, with C44's zstd named error as a drop-first stretch; re-sequences the standing C33+C25 capability lead to cycle 6 with rationale, plus rejected-alternative analysis and a red-to-green execution order."
keywords: ["agenttrace", "cycle-5", "prioritization", "governance-coverage", "discovered-counts", "cache-eviction", "float-hygiene"]
cwd: "/work/projects/agenttrace"
repository: "luoyectl/agenttrace"
repo_root_sha: "998ade8827820479069d7d3590082a33fbf80045"
branch: "master"
head: "998ade8"
---

# Cycle 5 prioritization (run dafd34b3, attempt 1ac59599)

- run: `dafd34b3940e497f9f1ac234573323ad`, attempt
  `1ac5959989244b6d8cd327fd70bb0691`
- intent: `prioritize_repository_maintenance`
- grounding: HEAD `998ade8` (cycle 4, committed; PR #282 open). Working
  tree carries this run's roadmap-phase edit to `ROADMAP.md` (docs
  only) and the untracked pass-8 review / pass-7 research records; no
  Rust source differs from the pass-8 baseline (189/189 tests, clippy
  `-D warnings` clean, fmt clean, all ten check scripts green). The
  three headline reproducers were **re-verified live this turn** on
  the existing release binary (below).
- inputs: `ROADMAP.md` (hardening lane 24 items, capability lane,
  Completed cycles 1–4), assessment pass 8
  (`docs/reviews/2026-09-03-adversarial-repository-assessment-pass8.md`,
  F8-1..F8-10), research pass 7
  (`docs/research/2026-09-03-extensions-research-pass7.md`, candidates
  43–49), the cycle-4 prioritization and implementation records, live
  probes.

**Skill note.** Same disclosure as cycles 1–4: no compound-engineering
router is installed in this environment, and the nearest historical
match (`ce-plan`) is a full implementation-plan workflow heavier than a
scoring/selection pass. Scoring ran in-thread: every open item scored
on impact (1–5), risk-if-deferred (1–5), effort (XS–L), external
dependency, and strategic value to the two product jobs (cross-agent
history review; slow/regressed-task diagnosis), under the hard
constraint that the batch be completable and verifiable **end to end on
this host** (offline Linux, no Windows runtime, CI execution prohibited
this cycle). No subagent surface exists; no claim is independently
corroborated.

---

## 1. Verdict — cycle-5 batch: "Honest coverage, honest cache, honest math"

> **Selected:** **F8-1** governance report coverage honesty (lead) +
> **F8-2** truthful discovery accounting + **F8-3** session-cache
> eviction + **F8-5/F8-6** float hygiene in costing and statistics +
> **F8-7/F8-8** docs-contract check — as **CU-11..CU-15**. Stretch
> (drop-first): **C44-min** zstd magic sniff with a named error.

Five change units, one theme, zero external dependencies, every
acceptance criterion verifiable offline on this host. Cycle 4 closed the
ways a *single session's parse* could lie or lose data; cycle 5 closes
the ways the *aggregates* lie or degrade: a governance audit that
reports 3.44 dollars over 20 sessions when the corpus spent 698 over
1,408 (F8-1), a "Parse coverage 71/71" that hides 1,340 files (F8-2), a
cache that only ever grows (F8-3), a report path that can panic on
adversarial catalog data and two percentiles that disagree (F8-5/F8-6),
and a guide that describes a schema two versions old and a network
refresh the code forbids (F8-7/F8-8).

**Live re-verification this turn** (release binary at HEAD, 2026-09-03):

- **F8-1.** `--audit -f json --range all` → `total_estimated_cost`
  **3.4427** (default, newest-20 sample) versus **697.9935** with
  `--limit 2000` — a **203×** understatement on today's corpus (pass 8
  measured 176× on 2026-09-02; the corpus grew a day's sessions and the
  understatement *worsened*), exit 0 both times, no disclosure field.
- **F8-2.** `--overview -f json --range 1d` → `discovered = 71,
  parsed = 71` while 1,407+ files exist; the ranged denominator erases
  ~1,340 out-of-range files and every parse failure inside them.
- **F8-3.** `~/.cache/agenttrace/sessions.json` → **10,530,891 bytes,
  1,487 entries, 761 (51%) dead paths** — the file *grew 12,772 bytes
  in one day* with zero cleanup, confirming the compounding behavior
  (pass 8 measured 10,518,119 B).
- **F8-5/F8-6/F8-7** stand as recorded in pass 8 (panic-reachable
  `json_float` via non-finite `convert_litellm` products; percentile
  20-vs-19 standalone repro; `governance-reports.md:52-55` schema-4 and
  auto-refresh claims vs `session_cache.rs:13` schema 6 and a
  network-free loader pinned by test).

Why this batch wins every test applied to it:

- **It contains the board's only HIGH finding, on the flagship
  surface.** Governance reports are the product's differentiator
  against ccusage-class aggregators; an audit that silently samples 20
  sessions and exits 0 is a lying aggregate — the same defect class
  cycle 4 was themed on, one level up, and the one class this campaign
  has never left standing across cycles 1–4.
- **The batch compounds if deferred; none of it rots if taken now.**
  F8-3 grows monotonically (measured), F8-1's understatement *widens*
  as corpora grow (176×→203× in one day), F8-2 misleads on every ranged
  run, and F8-5 is a live panic path in report rendering.
- **Every unit has its acceptance infrastructure already.** F8-1's
  disclosure extends existing JSON fields and the overview loader
  already returns the truth (`LoadReport.discovered`); F8-3's pruning
  data is already in hand at load time; F8-5's guard is one match arm
  and F8-6 keeps either helper as the single definition; F8-7/F8-8 is a
  check script of the same shape as the ten that already pass. These
  are small, evidenced, low-design-risk units.
- **CU-11 settles `--limit` semantics, closing the P3-5 ambiguity as a
  side effect.** Decision recorded for the implementer: `--limit`
  becomes a *display* cap for list views (applied in `--overview`'s
  tables too, closing P3-5's "silently ignored" half) and never a data
  filter on aggregate commands; explicit sampling moves behind a new
  `--sample N` flag that discloses `audited_sessions`/
  `total_sessions`. One rule, one place, documented once.
- **It is simultaneously highest-value and end-to-end completable
  here** (see §4 for the alternatives that fail one of the two).

## 2. Scoring — new items this round

Imp = impact (1–5) · Risk = risk if deferred · Eff = effort · Dep =
external dependency · Strat = value to the two product jobs.

| # | Item | Imp | Risk | Eff | Dep | Strat | Disposition |
|---|---|---|---|---|---|---|---|
| F8-1/C45 | audit-class commands silently sample newest 20 (`main.rs:122-123`, `.take` `:225`/`:249`); 203× live understatement, exit 0 | 5 | 5 | S–M | none | 5 | **batch lead (CU-11)** |
| F8-2/C46 | `data_health.discovered` recomputed from `sessions.len()+skipped`, discards `LoadReport.discovered` (`main.rs:337-341`); ranged denominator lies | 4 | 4 | S | none | 4 | **in batch (CU-12)** |
| F8-3/C47 | session cache never evicts dead paths (`session_cache.rs:597-616`); 10.5 MB, 51% dead, grew 12.7 KB/day | 4 | 4 | S–M | none | 3 | **in batch (CU-13)** |
| F8-5/F8-6/C49 | `json_float` `.expect` on inf/NaN (`reports.rs:1446-1452`) reachable via `convert_litellm` ×1e6 (`pricing.rs:330-345`); divergent percentile impls (`lib.rs:1309` vs `reports.rs:1777`) | 3 | 4 | S | none | 3 | **in batch (CU-14)** |
| F8-7/F8-8/C48 | guide claims schema 4 + 24h auto-refresh (code: schema 6, network-free); README flags-before-positional note absent | 3 | 3 | S | none | 3 | **in batch (CU-15)** |
| C44-min | zstd magic sniff → named actionable error (`parser.rs:20-34`; Codex PR #41357 merged 2026-08-28) | 3 | 4 | XS–S | none (decode would add a dep) | 3 | **stretch (CU-16), drop-first** |
| C43 | catalog-tiered cache & reasoning pricing (`Price` v2 + snapshot regeneration; 134/84/56/35/28/69-model census; corpus 94% cache-read) | 5 | 3 | M+ | snapshot schema + goldens churn | 5 | **cycle-6 lead** |
| F8-4 | rusqlite 0.32→0.40.2 (SQLite 3.46.0→3.53.2), MSRV 1.88, cargo-audit CI | 3 | 3 | M | MSRV bump; dependabot #278/#279 open | 2 | defer, post-#282 |
| F8-9/F8-10 | installer checksum parity; carryover files; `OnceLock` builtin pricing; `parse_file` size cap; ROADMAP slimming | 2 | 2 | S | release plumbing | 1 | chores rider, post-merge |

## 3. Standing dispositions (re-affirmed or re-sequenced)

- **C33 + C25 (per-turn model attribution + Gemini thinking tokens) —
  the standing "cycle-5 capability lead" from cycle 4's record — moves
  to cycle 6.** This is the only re-sequencing this pass makes, and it
  is deliberate: cycle 4's promise predates assessment pass 8, which
  landed a HIGH finding on the governance surface *after* that
  prioritization was written. The roadmap's own rule — "hardening
  precedes capability work" — has governed every cycle 1–4 selection,
  and the board's only HIGH outranks a promised capability lead.
  C33/C25's grounding actually *strengthened* this round (research
  pass 7: subagent models remain the default upstream; no drift), so
  nothing about the deferral is staleness — it is purely ordering. C43
  joins them as the cycle-6 conversation: it is the top accuracy lever
  on the board but M+ with golden churn across every cost figure,
  which would also muddy CU-11's before/after coverage evidence if
  landed in the same tree.
- **P3-5 (`--limit` and gate scoping) is fully settled by CU-11's
  semantics** — display cap everywhere, never a silent data filter;
  the roadmap item can close with CU-11 if the implementer applies the
  cap in the overview tables as specified.
- **F8-4 dependency lane:** post-#282, folding dependabot #278/#279;
  the MSRV 1.88 statement and a cargo-audit CI job ride it. Bumping
  SQLite under an open PR invites rebase churn for no user value.
- **P3-4 (control characters in output)**: still deferred by standing
  campaign decision; no new evidence changes it.
- **N8 (Markdown escaping)**: S-effort and honesty-adjacent; first
  rider if cycle 5 finishes with slack, else cycle 6.
- **C44 full zstd decode**: deferred with the dependency lane (adds a
  crate); the named-error minimum is this cycle's stretch.

## 4. Rejected alternatives (with reasons)

- **C33+C25 capability cycle (the standing sequence).** Highest
  strategic value on the board, but M-effort with fixture-heavy
  semantics work; a fresh HIGH finding outranks it under
  hardening-precedes-capability, and splitting focus across a
  capability lead and the F8-1 lead risks shipping neither end to end.
  Re-sequenced, not demoted (§3).
- **C43 pricing-tiers cycle.** The single biggest accuracy lever (94%
  of corpus tokens are cache reads priced flat), but it regenerates
  the snapshot, extends `Price`, and churns every cost golden — M+
  with schema decisions that deserve their own review, and its cost
  churn would contaminate CU-11's coverage evidence in the same tree.
- **F8-4 "big bang" dependency refresh inside this batch.** Mixing
  SQLite/MSRV churn with behavior fixes muddies attribution; the open
  dependabot PRs and PR #282's pending merge make it rebase-prone.
- **All-F8s mega-batch (hardening + chores + deps in one cycle).**
  F8-9/F8-10 are release-plumbing chores with no user-facing lie;
  carrying them dilutes the theme and the verification surface for no
  honesty value.
- **F8-3-only or F8-1-only micro-cycles.** Each is shippable alone but
  the five share one theme, one verification pass, and one docs sweep;
  splitting them multiplies cycle overhead (record, review, PR
  updates) for the same total risk.

## 5. Execution order (red → green per unit)

1. **CU-11 (lead, F8-1/C45).** Red: today's reproducer (default audit
   = 3.4427/20 vs `--limit 2000` = 697.9935/1408, no disclosure
   fields). Green: default audit-class runs unbounded (or bounded only
   by `--range`), JSON emits `audited_sessions`/`total_sessions`(+exclusion
   reason), text/HTML print "(auditing N of M sessions)",
   `--limit` applies as a display cap in list views *including*
   `--overview` (closing P3-5), `--sample N` restores explicit bounded
   sampling with disclosure. Test: audit totals equal overview totals
   on a shared fixture at default flags.
2. **CU-12 (F8-2/C46).** Red: `--range 1d` reports discovered=71.
   Green: `discovered` from `LoadReport.discovered`; ranged runs split
   parsed/out-of-range. Test: `discovered` is range-independent on one
   corpus.
3. **CU-13 (F8-3/C47).** Red: 1,487 entries/761 dead after load+save.
   Green: load-time pruning of nonexistent paths (cache is derivable;
   pruning costs only re-parse time) plus a documented entry/byte
   bound. Test: eviction idempotent across saves; live before/after
   size on the operator corpus recorded in the implementation record.
4. **CU-14 (F8-5/F8-6/C49).** Red: poisoned-catalog inf rate panics in
   `json_float`; percentile parity repro returns 20 vs 19. Green:
   `json_float` total (null + data-health flag for non-finite),
   finiteness check in `convert_litellm`, one percentile definition
   everywhere. Tests: poisoned-catalog fixture renders JSON; parity
   test pins the survivor.
5. **CU-15 (F8-7/F8-8/C48).** Red: check fails on
   `governance-reports.md` schema-4/auto-refresh text; README lacks
   flags-before-positional. Green: guide corrected to schema 6 and
   offline-only pricing (naming `--update-pricing` as the sole network
   path, matching `PRIVACY.md`), README note added, `check-docs-*`
   script pins the schema constant against `session_cache.rs` — run
   **after** CU-11/12 settle the semantics it documents.
6. **Stretch CU-16 (C44-min), drop-first.** zstd magic (`28 B5 2F FD`)
   sniff beside the UTF-16 check → named actionable error + a
   doctor/data-health signal; committed fixture. Only if CUs 11–15
   land green with the full suite and scripts passing.

**Batch exit criteria (all required):** `cargo test --workspace` green
(189 + new tests, expect ≥ ~200), `cargo fmt --check` clean,
`cargo clippy --workspace --all-targets -- -D warnings` clean, all ten
`scripts/ci/check-*.sh` pass (now including the CU-15 docs check),
both F8-1/F8-2 reproducers inverted on the rebuilt release binary, and
the CU-13 before/after cache measurement recorded. No commit/push/PR
from this phase — the commit gate is a later action in this run.

**Artifacts:** this file. This phase edited no code and left
`ROADMAP.md` exactly as the roadmap phase produced it (`git diff
ROADMAP.md` unchanged from that phase's +244/−14).
