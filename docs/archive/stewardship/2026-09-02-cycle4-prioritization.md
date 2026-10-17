---
artifact_contract: "ce-handoff/v1"
created_at: "2026-09-02T14:03:21Z"
title: "Cycle 4 prioritization — truthful reads, truthful gates, durable records"
summary: "Scores every unresolved ROADMAP.md item after cycle 3 (assessment pass 7 P7-1..P7-5, research pass 6 candidates 37-42, cycle-3 residuals) by impact, risk, effort, dependency, and strategic value; selects the cycle-4 batch: P7-1 lead + P7-2 + P7-3 + P7-5 + cycle-3 residuals as CU-6..CU-10, with rejected-alternative rationale and a red-to-green execution order."
keywords: ["agenttrace", "cycle-4", "prioritization", "silent-data-loss", "baseline-gate", "durability"]
cwd: "/work/projects/agenttrace"
repository: "luoyectl/agenttrace"
repo_root_sha: "66320145ae38163bce90b45668e3e4afd95d3c2a"
branch: "master"
head: "6632014"
---

# Cycle 4 prioritization (run 2a156259, attempt 103d0163)

- run: `2a15625945fc40419fc4691c59b42a7b`, attempt
  `103d01635ac14b85901e2bb1bb630ef2`
- intent: `prioritize_repository_maintenance`
- grounding: HEAD moved to `6632014` mid-run ("ci: use self-hosted
  runner"), a **workflows-only** commit (`git diff --stat
  93aaf05..6632014` → 3 files, all `.github/workflows/*.yml`
  `runs-on:` switches, zero Rust changes), so the pass-7 baseline
  (180/180 tests, clippy `-D warnings` clean, fmt clean at `93aaf05`)
  still describes the current source tree. The lead findings were
  **re-verified live this turn** on the release binary built from those
  identical sources (reproducers below, artifacts in `/tmp/pri4/`).
  This phase edited only this document; `ROADMAP.md` is untouched here.
- inputs: `ROADMAP.md` (updated this run, roadmap phase), assessment
  pass 7
  (`docs/reviews/2026-09-02-adversarial-repository-assessment-pass7.md`),
  research pass 6
  (`docs/research/2026-09-02-extensions-research-pass6.md`), the
  cycle-3 implementation record and prioritization records, live probes.

**Skill note.** Same disclosure as cycles 1–3 (all provenances read this
campaign): the compound-engineering roster has no prioritize skill, and
the nearest match (`ce-plan`) is a full implementation-plan workflow
with interactive handoffs — heavier than a scoring/selection pass and
built for a different artifact. Scoring was therefore performed
in-thread: every open item scored on impact (1–5), risk-if-deferred
(1–5), effort (XS–L), external dependency, and strategic value to the
two product jobs (cross-agent history review; slow/regressed-task
diagnosis), then a coherent batch selected under the hard constraint
that it be completable and verifiable **end to end on this host**
(offline Linux, no Windows runtime, CI execution prohibited this
cycle). No subagent surface exists; no claim is independently
corroborated.

---

## 1. Verdict — cycle-4 batch: "Truthful reads, truthful gates, durable records"

> **Selected:** **P7-1** silent line loss in the generic-JSONL fallback
> (lead) + **P7-2** BOM/encoding handling at the parse entry +
> **P7-3** baseline thresholds must gate the exit code + **P7-5**
> atomic pricing/history writes + **cycle-3 residuals** (backslash
> parity, snapshot schema version, orphan sweep) — as CU-6..CU-10.

Five change units, one theme, zero external dependencies, every
acceptance criterion verifiable on this host. Where cycle 3 closed the
last reproducible **panic**, cycle 4 closes the remaining ways the tool
can **lie or lose data without any signal**: lines that vanish (P7-1),
files that "unsupported" despite being valid JSON for a Windows user
(P7-2), a CI gate that cannot fail (P7-3), and the only durable record
being non-atomically overwritten (P7-5). All five were re-verified live
this turn; none is stale.

**Live re-verification this turn** (release binary from sources
identical to HEAD; all artifacts in `/tmp/pri4/`):

- **P7-1.** `mix3.jsonl` (three user lines: valid, lone-surrogate
  `\ud800`, valid) reports `Messages: 2 user` — the recoverable line is
  silently dropped (`lib.rs:382` strict `serde_json::from_str::<Value>`
  `let Ok(..) else { continue }`), while `data_health` is byte-identical
  to the clean file (`parsed: 1, skipped: 0`) — **no signal at all**.
  `evusage.jsonl` (`"usage":{"input":{"tokens":5}}`, Event-typed usage)
  reports `Messages: 0 user`: the whole line drops and the session
  parses empty. Honesty note discovered live: a *string-typed* usage
  value (`"input_tokens":"5"`) **survives** (`Input: 5 tokens`) — the
  concrete verified drop shapes are the lone-surrogate line (the exact
  shape `repair_lone_surrogates` exists to fix — the generic fallback
  simply never calls the lenient machinery the format detectors use)
  and Event-typed usage; the implementer should pin both shapes, not
  the string-typed one.
- **P7-2.** `bom.jsonl` (UTF-8 BOM + one valid Claude-Code line) →
  `exit 1, "Error: unsupported session format: bom.jsonl"`
  (`parser.rs:20-26` `read_to_string`, `:63` `parse_raw_session`).
- **P7-3.** Forged baseline (`summary.total_tokens = 0`) vs a run with
  tokens, `--baseline-max-token-delta-pct 1` → report says
  `token_delta_pct: 100.0, tokens_above_threshold: True`, process
  **exits 0** (`main.rs:388-421` gate covers only health/critical/
  tool-fail; `reports.rs:640-690` computes the deltas and booleans and
  nothing reads them).
- **P7-5.** `pricing.rs:334` (`write_pricing_cache`) and
  `history.rs:46` (`preserve_derived_history`) both call plain
  `std::fs::write` — an interrupted write truncates the only durable
  record (Claude Code's 30-day default transcript retention makes
  `history.json` exactly that).

Why this batch wins every test applied to it:

- **It contains the board's only silent-data-loss defect in the default
  read path.** Every non-Claude/opencode corpus walks the generic-JSONL
  fallback; a single unparseable-but-recoverable line silently vanishes
  with zero health signal. Data trust is the product's entire premise.
- **Every unit already has its acceptance infrastructure.** P7-1's
  counts extend the existing `DataHealth.skipped` field (already
  surfaced in text/MD/HTML/JSON reports, `insights.rs:107`); P7-1's
  repair already exists (`repair_lone_surrogates`, `parser.rs:3796`)
  and just needs calling from the fallback; P7-3's exit-2 semantics
  already exist (`evaluate_overview_gate` + `std::process::exit(2)`,
  `main.rs:388-421`); P7-5's unique-temp-and-rename pattern already
  exists from CU-4 (`session_cache.rs`). These are small, evidenced,
  low-design-risk units.
- **The residuals are promises the Completed record already makes.**
  `ROADMAP.md`'s cycle-3 entries say "Residual, tracked below" three
  times; this batch retires all three while the context is fresh.
- **It is the only batch that is simultaneously highest-value and
  end-to-end completable here** (see rejected alternatives).

## 2. Scoring — new items this round

Imp = impact (1–5) · Risk = risk if deferred · Eff = effort · Dep =
external dependency · Strat = value to the two product jobs.

| # | Item | Imp | Risk | Eff | Dep | Strat | Disposition |
|---|---|---|---|---|---|---|---|
| P7-1 | generic-JSONL fallback drops recoverable lines with zero health signal (`lib.rs:382`/`:393`, `usage: BTreeMap<String,i64>` `lib.rs:134`) | 5 | 4 | S–M | none | 4 | **batch lead (CU-6)** |
| P7-2 | no BOM strip at any parse entry; UTF-16 unsupported with misleading error (`parser.rs:22`/`:63`) | 4 | 3 | S | strip-vs-transcode decision | 3 | **in batch (CU-7)** |
| P7-3 | `--baseline-max-*-delta-pct` computes but never gates exit; CI guide documents it as a check (`main.rs:388-421`, `reports.rs:672-677`) | 4 | 3 | S | exit-2 vs flag decision | 3 | **in batch (CU-8)** |
| P7-5 | `pricing.json`/`history.json` non-atomic `std::fs::write` (`pricing.rs:329-336`, `history.rs:36-46`) | 3 | 3 | S | none | 2 | **in batch (CU-9)** |
| P7-res | cycle-3 residuals: backslash lookbehind (`parser.rs:3796`), snapshot schema v5 across CU-5 (`session_cache.rs:9`), unswept temp orphans (`session_cache.rs:237`) | 3 | 2 | S | none | 2 | **in batch (CU-10)** |
| P7-4 | SQLite `since` push-down dead (`None` at `sqlite_sessions.rs:164`/`:232`) | 2 | 1 | S (delete) / M (plumb) | either/or vs candidate 36 watermark | 2 | **defer, pair with C36** |
| C33 | per-turn model attribution (capability lead; research pass 6 added `CLAUDE_CODE_SUBAGENT_MODEL_FORCE`, advisor cache-miss fixtures) | 5 | 4 | M | none | 5 | **cycle-5 lead** |
| C37 | agent-skill distribution channel (kelviq/tare 174★ precedent) | 4 | 3 | M | npm scope + shared read-only tool contract with C19 | 4 | capability cycle |
| C38 | `--redact`/`--share` redaction surface | 4 | 3 | S–M | none | 3 | next capability rider |
| C39 | verification-command audit | 3 | 2 | M | per-source fixtures | 3 | later |
| C40 | CSV export (`-f csv`, RFC 4180) | 3 | 2 | S | none | 2 | next capability rider |

## 3. Standing dispositions (re-affirmed, not re-scored)

No Rust code has changed under these rows since they were last scored
(workflows-only commit since). Order after this batch:

- **C33 + C25 = the cycle-5 capability lead**, exactly as cycle 3's
  record sequenced it, now with stronger upstream grounding from
  research pass 6 (subagent models are the *default*, so mixed-model
  sessions are the norm, not the edge; advisor-model cache-miss
  re-sends deserve a fixture). **P7-4 rides that cycle** as C36's
  either/or decision: plumb `since` from CLI filters with an
  incremental-equals-full parity test, or delete the parameter in the
  same motion as the watermark design (deleting it now and re-adding in
  C36 is churn).
- **C38 + C40 are the smallest capability riders after C33** (redaction
  and CSV, both S–M, both fixture-verifiable here); they pair naturally
  with C37's packaging cycle, which needs an npm-scope decision and one
  shared read-only tool contract with C19.
- **P3-1 platform parity stays the top *deferred* item**, unchanged:
  its acceptance (per-OS fallback chains, Windows runtime) cannot be
  honestly executed on this host. Note recorded this turn: the new
  self-hosted-runner CI (`6632014`) may eventually close the
  verification gap for **CI-image items** (N5/P4-4/P4-5 were parked on
  exactly "needs `expect`/CI-image changes"), but observing CI runs is
  outside this cycle's permissions — re-examine next cycle.
- **Output-honesty sweep** (P3-4 + P3-7 + P4-2/P4-3 + N7) as its own
  cycle: every report byte changes; it needs its own corpus and docs
  sweep. Note: the live P7-1 probe also re-demonstrated the P4-2/P3-7
  family (positional-file form ignores `-f json`; `-d` honors it).
- **N4/dependency motion** (ureq 3.4.0, rusqlite 0.40.2, crossterm
  0.29.0, fold dependabot #278/#279) stays one-motion, registry-fetch,
  MSRV-coupled; not while read-path honesty is open.

## 4. Batch execution order (red → green)

1. **Fixtures first (red).** Extend the adversarial generator with a
   `generic-loss` family: three-line mix (valid / lone-surrogate /
   valid) asserting **2** messages **and** a skip count of 1 with
   reason; Event-typed-usage line asserting recovery or a counted skip;
   UTF-8-BOM variant of one committed corpus per family; UTF-16LE
   transcript asserting a *named* encoding error. Wire the drop shapes
   into `discovery_contract.rs`.
2. **CU-6 (P7-1, lead).** Route the generic-JSONL fallback
   (`parse_jsonl_session`, `lib.rs:374-395`) through the same lenient
   line machinery the detectors use (`repair_lone_surrogates`,
   `parse_jsonl_value_lenient`, `number_as_i64`-style coercion for
   numeric-adjacent usage); extend `DataHealth` with per-reason skip
   counts (the `skipped` field and its report surfaces already exist);
   tolerate Event-typed usage by extracting known numeric leaves or
   counting the skip with a reason. Acceptance: the mix3 reproducer
   reports the recovered line **and** counts what it could not recover,
   with reasons, in text/MD/HTML/JSON.
3. **CU-7 (P7-2).** Strip a UTF-8 BOM once, at offset 0, in the shared
   `parse_raw_session` entry (all formats inherit it); sniff UTF-16
   BOMs and return a diagnosis-grade error naming the encoding rather
   than "unsupported session format". Decision to record in-code:
   **strip + name, do not transcode** this cycle (no new encoding
   dependency; transcoding is a C36-adjacent perf/compat decision).
4. **CU-8 (P7-3).** Make baseline thresholds gate the exit: reuse the
   exit-2 machinery (`evaluate_overview_gate` / `--fail-under-health`
   pattern) for `slower_than_baseline`/`cost_above_threshold`/
   `tokens_above_threshold` — decision to record: **always-on exit 2 on
   breach with `--no-baseline-gate` (or inverse `--fail-on-baseline-
   regression`) opt-out**, no-baseline runs stay exit 0 with a labeled
   skip; update `docs/guides/ci-integration.md:116-124` to show the
   failing exit. Pin: forged-baseline reproducer exits 2; clean run 0.
5. **CU-9 (P7-5).** Apply the CU-4 unique-temp-and-rename pattern
   (`<name>.tmp.<pid>.<seq>`) to `write_pricing_cache`
   (`pricing.rs:329-336`) and `preserve_derived_history`
   (`history.rs:36-46`); sweep orphaned `*.tmp.*` siblings on cache
   load (retires the `session_cache.rs:237` residual). Interrupted-write
   fixture: a truncated file quarantines under a visible warning, never
   silently discarded.
6. **CU-10 (residuals).** Backslash-parity guard in
   `repair_lone_surrogates` (an escaped backslash before `\u` must not
   count as an escape start; corpus line for it) and a
   bump-or-compatible decision on `SQLITE_SNAPSHOT_SCHEMA_VERSION`
   (`session_cache.rs:9`) with a version test asserting stale
   pre-CU-5 snapshots invalidate rather than serve placeholder names.
7. **Verification matrix** (same bar as cycles 2–3): `cargo test
   --workspace` debug **and** release, `cargo fmt --check`, `cargo
   clippy --workspace --all-targets`, all runnable
   `scripts/ci/check-*.sh` against a fresh release build, and this
   turn's reproducers re-run end to end (mix3 recovered+counted, BOM
   parses, forged baseline exits 2).

## 5. Rejected batches (with reasons)

- **"Capability accuracy" (C33 + C25 + C24):** the highest strategic
  bundle and cycle 3's own designated successor — but assessment pass 7
  opened five fresh hardening findings, and the roadmap's standing
  principle is that hardening precedes capability work. C33 also wants
  its own mixed-model corpus; splitting this cycle's attention would
  risk both. It leads cycle 5, better-armed than before.
- **"Windows parity" (P3-1 + full UTF-16 transcode + P5-6):** contains
  a standing HIGH, but its acceptance needs a Windows runtime or CI
  observation — neither available to this cycle. Only the honestly
  verifiable slice (CU-7 strip + named error, fixture-provable on
  Linux) rides now.
- **"Output honesty" (P3-4 + P3-7 + P4-2/P4-3 + N7):** coherent but
  changes every report byte; needs its own corpus, docs sweep, and
  baseline-shift changelog note; shares no dependency with P7-1.
- **"Dependency motion" (N4 ureq 3 + rusqlite/crossterm + #278/#279):**
  registry fetch, MSRV coupling, one-motion rule; parked unchanged.
- **"Skill distribution" (C37 + C38 + C40):** strong market signal
  (tare's 174★ in three weeks) but a *distribution* play that presumes
  the engine's reads are already trustworthy; also needs npm-scope and
  tool-contract decisions that deserve their own cycle. Sequenced as
  the capability cycle after C33.
- **"P7-4 rider":** rejected for this batch — deleting the dead
  parameter now is churn if candidate 36 re-adds a watermark, and
  plumbing it properly is M-effort with parity-test surface; it belongs
  with C36.

---

**Artifacts:** this file. `ROADMAP.md` untouched by this phase.
**Verification of grounding claims this turn:** live reproducers above
(`/tmp/pri4/`: `bom.jsonl`, `mix.jsonl`, `mix3.jsonl`, `clean.jsonl`,
`strusage.jsonl`, `evusage.jsonl`, `base-zero.json`, `reg2.json`) against
`target/release/agenttrace`; anchors `lib.rs:134`/`:374-395`,
`parser.rs:22-26`/`:63`/`:3796`, `main.rs:88-94`/`:374-384`/`:388-421`,
`reports.rs:640-690`, `pricing.rs:329-336`, `history.rs:34-46`,
`session_cache.rs:9`, `insights.rs:107` re-checked against the tree;
`git diff --stat 93aaf05..6632014` → workflows-only.
