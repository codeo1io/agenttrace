# Prioritization — Repository Maintenance Cycle 2

- run: `0a36c54199de4861b50ddc2dcb26fd8f`
- attempt: `c8298d0e6495435eb116804e4552b84d`
- intent: `prioritize_repository_maintenance`
- grounded at `e005952` plus the uncommitted cycle-1 tree (cycle-1 work is the
  working tree; nothing committed), 159/159 tests passing re-confirmed this
  turn: `cargo test --workspace` → 11+1+50+7+50+40 = 159 passed, 0 failed
- inputs: `ROADMAP.md` (266 lines, updated this run), assessment pass 2
  (`docs/reviews/2026-09-02-adversarial-repository-assessment-pass2.md`,
  findings N1–N10, all reproduced), research pass 2
  (`docs/ideation/2026-09-02-agenttrace-extensions-ideation.md`,
  candidates 8–12)
- date: 2026-09-02

**Skill note:** the installed compound-engineering roster (32 ce-* skills)
has no prioritize skill; this matches the two prior fleet runs
(`/tmp/compound-engineering-1000/ce-prioritize/…/provenance.txt`, 2026-08-31
and 2026-09-01), both of which disclosed the same gap. `ce-plan` plans work
already chosen; `ce-pov` judges one idea already on the table — both operate
downstream of this decision. A direct prioritization was performed and this
substitution is disclosed. **Harness disclosure:** this delegate session has
no subagent surface; all passes ran in-thread and no claim below is
independently corroborated.

Per the fleet precedent, `ROADMAP.md` is left untouched by this phase; the
selection lives here and the conductor feeds it forward.

---

## 1. Scoring matrix

Dimensions: **Imp** = impact if done (1–5) · **Risk** = risk if deferred
(1–5) · **Eff** = effort (S/M/L) · **Dep** = external dependencies ·
**Strat** = strategic value to the product's two jobs (review agent history;
diagnose slow/regressed tasks) (1–5).

| # | Item | Imp | Risk | Eff | Dep | Strat |
|---|---|---|---|---|---|---|
| H1 | SQLite arithmetic remainder (N1 HIGH panic, N2 negative costs, N3 waste panic) | 5 | 5 | S–M | none | 5 |
| C8 | Trust upstream totals — stored-cost/token preference + delta (candidate 8, 88%) | 5 | 4 | M | none (columns verified live last phase) | 5 |
| H4 | No silent data loss (N7 unknown-time bucketing half; F16/F18 halves stay) | 3 | 3 | S | none | 4 |
| H8 | CLI surface polish (N9 `--version` ordering) | 2 | 2 | S | none | 2 |
| H5 | Proxy-governable networking (N4 + candidate 12: ureq 3, MSRV 1.85, NO_PROXY) | 4 | 3 | M | ureq 3 API spike; Cargo.lock churn | 3 |
| H7 | CI that tells the truth (N5 dead TUI step, N6 stale prose, F13) | 3 | 3 | S | best merged with H5's ci.yml edits | 3 |
| H6 | Report output injection (N8 md_escape) | 2 | 2 | S | none | 2 |
| H9 | Delivery-evidence cost ceiling (N10 git-log concurrency) | 2 | 2 | M | partially displaced by C8's `summary_*` columns | 3 |
| H2 | Cache and history durability (F4 re-verified, F6/F7) | 3 | 3 | M | digest-scheme design (F7) | 3 |
| H3 | Platform parity remainder (F8/F9/F15; F15 folds into H5) | 2 | 2 | M | Windows runtime untestable on this host; F9 needs a published artifact | 2 |
| C9 | Statusline output surface (84%) | 4 | 2 | M | brainstorm scoping (stdin contract) | 4 |
| C11 | Upstream schema-drift tracker (80%) | 3 | 3 | S–M | after C8 settles column expectations | 3 |
| C10 | Compaction/re-cache analytics (78%) | 3 | 2 | M | per-provider event inventory; after C8 | 3 |
| C1–C5 | Older candidates (limit-pressure, 2nd pricing source, canary, OTel, baseline config) | 3 | 2 | M–L | each needs its own brainstorm pass per roadmap | 3 |

## 2. Selected batch — **Trustworthy SQLite ingestion**

**H1 + C8 (totals scope) + H4's N7 half + H8 as a droppable rider.**
Closes N1 (HIGH), N2, N3, N7, N9 and delivers the highest-confidence
researched capability (88%) in one motion.

### Why this batch (evidence-backed, not preference)

1. **It removes the only HIGH finding on the board, and it panics today.**
   N1: debug builds panic at `sqlite_sessions.rs:403` on adversarial token
   counts; release builds emit wrapped negatives (`tokens_input: -1`). N2
   then ships those negatives into `--audit`/`--context-trends` JSON as
   `total_estimated_cost: -24903104499507.895` with
   `confidence: "high"` — the most credibility-damaging wrongness a
   cost-review tool can emit, reproduced twice this campaign.

2. **It is one root cause, already solved once, extended to the path that
   missed it.** Cycle 1 hardened `parser.rs:3582` `number_as_i64`
   (test at `parser.rs:4178`:
   `number_as_i64_saturates_extreme_numbers_instead_of_wrapping`) but
   stopped at the file-parser boundary. `sqlite_sessions.rs:590` still
   carries the unsanitized twin — `as_u64().map(|n| n as i64)` wraps,
   `as_f64().map(|n| n as i64)` saturates only to overflow later at the
   unguarded `+` (`:403`) and four `+=` accumulators (`:410-413`); the
   Hermes row reads at `:179-182` are unclamped while the adjacent
   `events`/`tool_calls` lines already `.max(0)`; `waste.rs:180` is
   `(input - cache_r).max(0)` — the `.max(0)` guards the sign, not the
   overflow. *(Line note: assessment pass 2 cited the converter at
   `:613-621`; in today's tree the function sits at `:590-599` and `:613`
   is inside the test module — today's grep is authoritative.)*

3. **Blast radius is bounded and measured.** One ingestion chokepoint
   (`discovery.rs:237` + the `lib.rs:68` export); the change set is
   `sqlite_sessions.rs` (641 lines), `waste.rs:180`, the governance
   confidence rule, `DataHealth` (home `insights.rs:279`, already wired at
   `reports.rs:477`), and the `main.rs:149-151` rider. The test lift has
   precedent: `discovery_contract.rs:75-82` already builds SQLite fixtures
   via `rusqlite::Connection`, with both-provider tests at `:1455`/`:1488`.

4. **Zero external dependencies — genuinely completable end-to-end this
   cycle.** No crate upgrades, no MSRV move, no network, no live-upstream
   verification: the authoritative column list (`cost`,
   `tokens_input/output/reasoning/cache_read/cache_write`, `summary_*`,
   `time_compacting`) was fetched live via `gh api` last phase, and the
   column-sniffing helper to extend (`sqlite_has_column`, `:581`, already
   used at `:156` and `:261`) is in place. Contrast H5, which couples a
   ureq 2→3 API rewrite, a Cargo.lock refresh, an MSRV 1.85 bump, and a
   new CI job.

5. **The roadmap's own sequencing note pairs these.** "Trust upstream
   totals … is also the root-cause fix for N1/N2 and the git-free path for
   N10 — schedule it alongside the hardening lane rather than after it."
   Splitting them would either ship hardened arithmetic that still
   disagrees with upstream's own numbers, or ship stored totals reached
   through arithmetic that can still wrap.

6. **N7 and N9 ride for near-zero cost.** `filter_since`
   (`:243-253`) drops every timestamp-less SQLite session whenever a range
   is set — same file, same fixture family. The `--version` fix is moving
   the early return at `main.rs:150-152` above `report_language(&args.lang)?`
   at `:149`.

### Scope boundary inside C8

In scope: prefer stored session totals when columns exist; retain the
derived path for older databases; surface the stored-versus-derived delta
in `data_health`. Deferred: the `parent_id` hierarchy/subagent half — it is
gated in the roadmap on verifying `parent_id` semantics against upstream
code before any label ships, i.e. it carries a research dependency the rest
of the batch does not.

### Batch execution order (dependency-driven)

1. **Red-today regression guards first**, from the pass-2 reproducers
   (`/tmp/at-assess2/mk_opencode_db.py`, `mk_db2.py`, `mk_db3.py`)
   committed as generated fixtures under
   `testdata/generated/adversarial/sqlite/`: debug-mode tests assert no
   panic and release-mode assertions assert bounded, non-negative totals
   across `--sessions`, `--waste`, `--audit`, `--context-trends`; one
   assertion that governance never reports `confidence: "high"` alongside
   a negative component.
2. Delete the local converter at `sqlite_sessions.rs:590`; route through
   the hardened `parser.rs:3582` (adapter for the `Option<&Value>`
   signature).
3. Saturating arithmetic: `:403` `+` → `saturating_add`; `:410-413` four
   `+=` → `saturating_add`; Hermes `:179-182` clamped like the adjacent
   lines; `waste.rs:180` → `saturating_sub` before `.max(0)`.
4. C8-totals: extend the `session` select with `sqlite_has_column` sniffing
   for `cost` and the five token columns; prefer stored when non-null;
   compute the stored-versus-derived delta; expose its count and magnitude
   in `DataHealth`.
5. N7: `filter_since` buckets timestamp-less SQLite sessions as
   unknown-time; `--range`/`--since` output reports how many sessions were
   excluded and why.
6. H8 rider: move the `--version` early return above `report_language`;
   add the CLI test `--lang fr --version` prints the version.
7. Full `cargo test --workspace`, `cargo fmt --check`,
   `cargo clippy --workspace --all-targets`, and all ten
   `scripts/ci/*.sh` check scripts locally.

**Definition of done:** `grep "fn number_as_i64" sqlite_sessions.rs` → 0;
adversarial SQLite fixtures committed; the step-1 guards green in debug and
release; `DataHealth` exposes the stored-versus-derived delta with a test
asserting zero delta on well-formed data and non-zero when a field is
dropped; an `opencode.db` fixture with `time_created = 0` stays visible
under `--range 7d`; `--lang fr --version` prints the version; suite, fmt,
clippy, and all check scripts green.

**Residual risk, accepted:** preferring stored totals changes reported
numbers on OpenCode databases. Mitigations are in the acceptance itself —
the derived path is retained for older schemas, and the delta is surfaced
rather than hidden, so a reviewer can see exactly what moved and why.

## 3. Explicitly not selected, with reasons

- **H5 proxy/ureq 3/MSRV (N4 + candidate 12)** — the second-best batch and
  the cycle-3 lead. Not first because it bundles three coupled risks (ureq
  3's Agent-based proxy API is a rewrite of `pricing.rs:316`, not a
  version bump; `Cargo.lock` churn across the workspace; MSRV 1.85 plus a
  new CI job) into one cycle, and N4's blast radius is confined to the
  opt-in `--update-pricing` path. It also wants a NO_PROXY-semantics
  spike (wildcard, dot-suffix, `*`) that deserves its own tests.
- **H7 CI that tells the truth (N5/N6/F13)** — real, but the right edit to
  `ci.yml` includes the MSRV job H5 owes; touching CI twice in two cycles
  is waste. The N6 prose rewrite can ride H5's cycle; the dead TUI step
  (`ci.yml:84`) is deleted in the same motion.
- **H6 md_escape (N8)** — small, self-contained, cosmetic until a hostile
  model name lands in a log; natural rider for a future reports cycle, not
  worth gating this one.
- **H2 cache and history durability (F4/F6/F7)** — F4 was re-reproduced
  this campaign (cache entries for deleted files persist) but it is silent
  hygiene: no wrong numbers, no panic. F7's digest-stability is a design
  decision, not a patch. Next cycle after the SQLite batch.
- **H3 platform parity remainder** — the Windows branch is untestable on
  this Linux host and F9's checksum needs a published release artifact to
  verify against; poor fit for an autonomous batch here. F15's MSRV half
  already folded into H5.
- **H9 delivery-evidence cost ceiling (N10)** — perf-only (0.61s vs
  0.011s); C8's `summary_*` columns are the git-free path the roadmap
  names, so re-measure after this batch lands before writing a concurrency
  pool.
- **C9 statusline (84%) / C10 compaction analytics (78%) / C11 drift
  tracker (80%)** — each needs its own scoping per the roadmap (C9 a stdin
  contract and latency budget; C10 a per-provider event inventory, easiest
  after C8 lands in the same reader; C11 should follow C8 so it tracks the
  new column expectations). None is implementable end-to-end this cycle
  without skipping a required design step.
- **C1–C5 older candidates** — unscheduled per the roadmap's own rule;
  C3's canary is partially subsumed by C11; C4 is blocked on GenAI
  semconv stability (research found it still churning).

## 4. Deferred queue (ordered)

After this batch: **H5 + H7 in one CI-touching motion (cycle-3 lead)** →
**H2** → **H6** → **H9 re-measured post-C8** → **C11** → **C9** → **C10**
→ C1–C5 as their own brainstorm passes.
