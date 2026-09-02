# Cycle 2 prioritization — update for run 9fcc0661 (2026-09-02)

- run: `9fcc0661af474e2783a0dee7541f6ddb`, attempt `8235b6ac5f344daebc46e6eb1223cb`
- intent: `prioritize_repository_maintenance`
- supersedes-in-part: `docs/stewardship/2026-09-02-cycle2-prioritization.md`
  (run `0a36c54199de4861b50ddc2dcb26fd8f`), which selected the cycle-2 batch
  from assessment pass 2 (N1–N10) and research pass 2 (candidates 8–12)
  only. This update re-validates that selection against everything folded
  into `ROADMAP.md` since: assessment passes 3–5 (P3-1–P3-9, P4-1–P4-8,
  P5-1–P5-6) and research passes 3–4 (candidates 13–27).
- grounding: same tree as every pass this campaign — HEAD `e005952` plus the
  uncommitted cycle-1 work; 159/159 tests, fmt/clippy clean; **no code has
  changed since the prior prioritization**, so its matrix rows are
  re-affirmed rather than re-scored, and only new items are scored below.
  Key batch facts were re-verified this turn: the four plain `+=`
  accumulators and local `number_as_i64` are still at
  `sqlite_sessions.rs:408-413`/`:590-600`; discovery is still `HOME`-only
  (`discovery.rs:51-53`); `.gitignore` still lacks `.hermes/`;
  `PRICING_SNAPSHOT_DATE` (`pricing.rs:16`) is still prose-synced only.

**Skill note.** The work order names the compound-engineering router and a
narrowest `ce-*` skill. No `ce-*` SKILL.md is installed in this delegate
environment (only leftovers under `/tmp/compound-engineering-1000`), and the
installed roster has no prioritize skill — the same gap the two prior fleet
runs and the cycle-2 record above disclosed. A direct prioritization was
performed in-thread (impact/risk/effort/dependency/strategic-value over the
updated ROADMAP.md); no subagent surface exists, so no claim below is
independently corroborated. Per fleet precedent, `ROADMAP.md` is left
untouched by this phase.

---

## 1. Verdict: the prior selection stands — with a falsification upgrade and three new riders

The prior record selected **H1 + C8 (totals scope) + H4's N7 half + H8 as a
droppable rider** ("Trustworthy SQLite ingestion"). Re-validated against
passes 3–5 and research 3–4, that choice survives every test applied to it:

- It still contains the only reproducible-**panic** HIGH on the board, and
  pass 5 made it worse in the right way: **P5-1/P5-2 falsify cycle 1's
  closure claims** — `CHANGELOG.md:7` says the hardening landed repo-wide
  while a debug build still exits 101 on an adversarial `opencode.db`
  (`sqlite_sessions.rs:410`) and a `u64::MAX` input reports `"input": -1`
  through `--latest -f json`. Closing the gap between what the project
  *claims* and what the binary *does* is now part of the batch's value, not
  just its bug count.
- Its zero-external-dependency property still holds (still the only
  genuinely end-to-end-completable batch on this offline, Linux-only host),
  and pass 5 supplied the missing test-lift path: the acceptance corpus in
  `discovery_contract.rs` covers JSONL only — exactly the fixture family the
  batch adds.
- Nothing in passes 3–5 or research 3–4 moved any competing item ahead of
  it. The two new HIGHs (P4-1, P3-1) are handled below — one as a rider,
  one as the next cycle's lead.

**Selected batch (cycle 2, final):**

> **"Trustworthy numbers on untrusted databases, and a launch that doesn't
> crash"** = H1 SQLite arithmetic hardening (P5-1/P5-2 evidence, N1/N2/N3)
> + C8 totals scope (stored session totals + stored-vs-derived delta) +
> N7 unknown-time bucketing + N9 `--version` rider + **P4-1 non-tty TUI
> guard** + **P5-3 snapshot-date pin test** + **P5-5 `.hermes/` ignore**.

## 2. Scoring deltas for the new items only

The prior matrix (13 rows) is unchanged; rows for the post-pass-2 items:
Imp = impact (1–5) · Risk = risk if deferred · Eff = effort · Dep =
external dependency · Strat = value to the two product jobs.

| # | Item | Imp | Risk | Eff | Dep | Strat | Disposition |
|---|---|---|---|---|---|---|---|
| P4-1 | TUI panics (exit 101) when stdout is not a tty — the README quickstart, reproduced from five documents (`app.rs:71-76` `ratatui::init()`) | 4 | 4 | XS | none | 3 | **in batch (rider)** |
| P3-1 | Discovery is `HOME`-only; stock Windows discovers nothing (HIGH) | 5 | 4 | M | Windows runtime untestable here | 4 | next cycle lead |
| P3-2 | `--range today`/`1d` are UTC windows, drop local-day sessions | 3 | 3 | S | TZ-determinism design note | 3 | deferred |
| P3-3/P3-8/P3-9 | Divergent `percentile()`s, duplicated helpers, dead code | 2 | 2 | S | none | 2 | deferred |
| P3-4 | C0/C1 control characters reach terminal + all report formats | 3 | 2 | S | none | 3 | deferred |
| P3-5 | `--limit` silently ignored by `--overview` (incl. `--baseline` gate recipe) | 2 | 3 | XS | fix-choice decision (apply vs reject) | 2 | deferred |
| P3-6 | `--doctor` never writes the session cache | 3 | 2 | S–M | none | 3 | deferred |
| P5-3 | `PRICING_SNAPSHOT_DATE` ↔ `_snapshot.date` synced by prose only | 2 | 2 | XS | none | 2 | **in batch (rider)** |
| P5-5 | `.hermes/` (conductor plans incl. remote-push instructions) untracked, unignored | 1 | 2 | XS | none | 1 | **in batch (rider)** |
| P5-6 | `install.ps1` downloads with no checksum | 2 | 2 | XS | published artifact to verify against | 1 | platform cycle |
| P4-2/P4-3 | Flag-shim silent truncation; filter-only invocations open full TUI | 2 | 2 | S | none | 2 | CLI sweep cycle |
| P4-4/P4-5 | npm tarball has zero tests; PTY smoke harness built but gated off | 3 | 3 | S | `expect` in CI image | 3 | CI cycle (with H7) |
| P4-6/P4-8 | Pricing cache ignores isolation var; history has no eviction | 2 | 2 | S–M | none | 2 | durability cycle (H2) |
| C24 | Cost provenance / provider-scoped rates (86%, issue #103) | 5 | 4 | M | JSON contract + snapshot regen | 5 | capability lead, next |
| C25 | Gemini `thoughtsTokenCount` dropped (80%) | 4 | 3 | S | none | 4 | pairs with C24 |
| C26 | Qwen dual-output transcripts (62%, radar #237) | 2 | 2 | S | real fixture | 2 | after fixture lands |
| C27 | All-channel version guard (70%, issue #272) | 2 | 2 | S | npm scope decision | 2 | release-cycle rider |
| C13–C23 | Research pass 3 candidates | 3 | 2 | S–L | per-item | 3 | per roadmap |

Why the three riders and nothing more:

- **P4-1** is a HIGH that is *cheaper to fix than to defer* — one guard or a
  manual `Terminal::new` in a function that already returns
  `anyhow::Result`, fully verifiable on this host (`agenttrace < /dev/null`
  must stop exiting 101). Every non-tty context — pipes, CI, cron, docker
  without `-t`, IDE consoles — currently gets a backtrace from the README's
  first command. It shares no files with the core batch.
- **P5-3/P5-5** are XS claim-integrity items cut from the same cloth as the
  batch's theme (things the project asserts vs things that are true), cost
  minutes, and carry zero design risk.
- **P3-5 was considered and explicitly left out**: it is XS, but pass 3
  offers two legitimate fixes (apply the limit vs reject the combination)
  that change the `--baseline` CI gate recipe — a semantics decision that
  deserves its own acceptance note in the CLI sweep, not a rider smuggled
  into a database-hardening batch.

## 3. Batch execution order (updated from the prior record)

1. **Red-today regression guards first.** Commit the pass-5 reproducers as
   generated fixtures under `testdata/generated/adversarial/sqlite/` (the
   overflow DB and the `u64::MAX` wrap DB from
   `/tmp/at-assess/repro_sqlite_overflow.py`), wire them into
   `discovery_contract.rs`, and assert: no panic in debug, bounded
   non-negative totals in release across `--sessions`, `--waste`,
   `--audit`, `--context-trends`; governance never pairs
   `confidence: "high"` with a negative component.
2. Delete the local converter at `sqlite_sessions.rs:590-600`; route
   through the hardened `parser.rs` `number_as_i64` (adapter for the
   `Option<&Value>` signature).
3. Saturating arithmetic: `:403` `+` and `:410-413` `+=` →
   `saturating_add`; clamp the Hermes row reads at `:179-182` like the
   adjacent lines; `waste.rs:180` → `saturating_sub` before `.max(0)`.
4. C8-totals: sniff `cost` and the five token columns via
   `sqlite_has_column`; prefer stored totals when present; keep the derived
   path for older schemas; expose the stored-vs-derived delta in
   `data_health`. (Deferred inside C8, as before: `parent_id` hierarchy —
   gated on upstream-semantics verification, now also covered by candidate
   14.)
5. N7: timestamp-less SQLite sessions bucket as unknown-time instead of
   vanishing from every `--range` view; report how many were excluded and
   why.
6. N9 rider: move the `--version` early return above `report_language`;
   test `--lang fr --version`.
7. **P4-1 rider:** replace `ratatui::init()` at `app.rs:71` with a manual
   `Terminal::new(CrosstermBackend::new(io::stdout()))?` (or an is-tty
   guard with a "use --overview" message); regression test with piped
   stdout asserting a normal exit code and message, not 101.
8. **P5-3 rider:** unit test asserting `PRICING_SNAPSHOT_DATE` equals
   `pricing_snapshot.json`'s `_snapshot.date`.
9. **P5-5 rider:** add `.hermes/` to `.gitignore`.
10. Full `cargo test --workspace` (debug and release), `cargo fmt
    --check`, `cargo clippy --workspace --all-targets`, all ten
    `scripts/ci/check-*.sh` with `AGENTTRACE_BIN` set; correct the
    `CHANGELOG.md:7` over-claim in the same motion.

**Definition of done** (prior record's, plus riders): `grep "fn
number_as_i64" sqlite_sessions.rs` → 0; adversarial SQLite fixtures
committed and loaded by `discovery_contract.rs`; delta exposed in
`data_health` with zero-on-well-formed / non-zero-on-dropped-field tests;
`time_created = 0` fixture stays visible under `--range 7d`; `--lang fr
--version` prints the version; `agenttrace < /dev/null` exits non-101 with
a message; the snapshot-date test passes and fails red on a mismatched
const; `git status` no longer lists `.hermes/`; CHANGELOG no longer claims
a repo-wide hardening that the SQLite path disproves; suite, fmt, clippy,
and all ten check scripts green.

**Residual risk, accepted** (unchanged in kind): preferring stored totals
moves reported numbers on OpenCode databases — mitigated by retaining the
derived path and surfacing the delta.

## 4. Explicitly not selected, with reasons (deltas only)

- **P3-1 (HIGH, Windows discovery)** — the strongest deferred item and the
  cycle-3 lead. Not ridden here because the shared-resolver refactor spans
  discovery, SQLite sources, cache dirs, and the history path (a fourth
  resolver, P4-7) — a platform theme, not a numbers theme — and Windows
  runtime behavior is unverifiable on this Linux host beyond unit tests.
  Scheduling note: it touches `sqlite_sessions.rs:47/:61`, which this batch
  also edits — land this batch first so the resolver work rebases onto
  hardened code.
- **C24 cost provenance** — the top research candidate (86%, issue #103)
  and the clear *capability* lead for the cycle after platform parity, but
  it changes the report JSON contract and regenerates the snapshot; pairing
  it with a database-hardening batch would entangle a contract change with
  a correctness fix. Its snapshot-regeneration step will ride on top of the
  P5-3 pin test this batch adds. C25 ships beside it (same baseline-shift
  changelog note).
- **H5 + H7 (+ P4-4/P4-5)** — the prior record deferred them as one
  CI-touching motion; passes 4–5 added P4-4 (vacuous npm tests) and P4-5
  (the ungating path for the dead PTY smoke step) to that same motion. The
  rationale stands: touch `ci.yml` once.
- **H2 durability (+ P4-6/P4-8)**, **P3-4/P3-2/P3-3/P3-6** correctness and
  hygiene sweep, **C26/C27** — unchanged in kind from the prior record's
  per-item reasons; see the queue below.

## 5. Updated deferred queue (ordered)

1. **Cycle 3 — platform parity:** P3-1 (lead) + P4-7 + P5-6 + the F8/F9
   remainder + P3-8's shared-helper consolidation; unit-test the resolver
   fallback chain with `HOME` unset.
2. **Cycle 4 — network and CI truth:** H5 (ureq 3, MSRV 1.85, proxy
   semantics) + H7 (N5 dead step, N6/P5-4 stale prose + drift check) +
   P4-4 + P4-5.
3. **Cycle 5 — durability:** H2 + P4-6/P4-8.
4. **Cycle 6 — capability lead:** C24 + C25 (cost provenance + thinking
   tokens, one baseline-shift changelog note), snapshot regenerated under
   the P5-3 test.
5. **Interleaved small sweeps** (riders on whichever cycle has room):
   P3-4, P3-5, P3-2, P3-3/P3-9, P3-6, N8/H6, N10 re-measured post-C8,
   C27 on the next release.
6. Then C11 → C9 → C10 → C1–C5 and research-pass-3 candidates per their
   roadmap acceptance criteria.
