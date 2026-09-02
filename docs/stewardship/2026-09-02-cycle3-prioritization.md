---
artifact_contract: "ce-handoff/v1"
created_at: "2026-09-02T11:55:00Z"
title: "Cycle 3 prioritization — trustworthy strings on untrusted input"
summary: "Scores every unresolved ROADMAP.md item (hardening lane + capability lane after cycle 2 and passes 6/research-5) by impact, risk, effort, dependency, and strategic value; selects the cycle-3 batch: P6-1 lead + P6-4 + riders P6-2, P6-3, C34, with rejected-alternative rationale and a red-to-green execution order."
keywords: ["agenttrace", "cycle-3", "prioritization", "utf-16", "hardening"]
cwd: "/work/projects/agenttrace"
repository: "luoyectl/agenttrace"
repo_root_sha: "e0059522b4fc74d53824f0e7ea7e4ac94d1465bb"
branch: "master"
head: "e005952"
---

# Cycle 3 prioritization (run 5d025d55, attempt 78f7bf28)

- run: `5d025d55b1194dd1a4dd8784146dfeeb`, attempt `78f7bf28c0614304b8a904d644f73e20`
- intent: `prioritize_repository_maintenance`
- grounding: HEAD `e005952` plus the uncommitted cycle-1/2 tree; no code has
  changed since assessment pass 6 verified `cargo test --workspace` 169/169
  (debug and release), fmt clean, clippy `--all-targets` clean. This phase
  edited only docs (`ROADMAP.md` in the roadmap phase; nothing here).
- inputs: `ROADMAP.md` (updated this run), assessment pass 6
  (`docs/reviews/2026-09-02-adversarial-repository-assessment-pass6.md`),
  research pass 5
  (`docs/research/2026-09-02-extensions-research-pass5.md`), the cycle-2
  implementation record, and the prior prioritization records.

**Skill note.** The compound-engineering roster is installed at
`/home/agent/.pi/agent/npm/node_modules/pi-compound-engineering/skills/`;
it has no prioritize skill, and the nearest match (`ce-plan`) is a full
implementation-plan workflow with interactive handoffs, not a
scoring/selection pass — heavier than this phase's objective and built for
a different artifact. Per fleet precedent (both prior prioritization
records), the scoring was performed in-thread: every open ROADMAP.md item
was scored on impact (1–5), risk-if-deferred (1–5), effort (XS–L),
external dependency, and strategic value to the two product jobs, then a
coherent batch was selected under the hard constraint that it must be
completable and verifiable **end to end on this host** (offline-capable,
Linux, no Windows runtime, no CI execution this cycle). No subagent
surface exists, so no claim below is independently corroborated;
`ROADMAP.md` is left untouched by this phase.

---

## 1. Verdict — cycle-3 batch: "Trustworthy strings on untrusted input"

> **Selected:** **P6-1** UTF-16 escape repair panic (lead) + **P6-4** honest
> fallback token accounting + riders **P6-2** CHANGELOG/`--version`
> claim-integrity, **P6-3** unique cache temp suffix, **C34**
> placeholder-title gate.

Five change units, one theme, zero external dependencies, every acceptance
criterion verifiable on this host. This completes the hardening trilogy
the prior two cycles began: cycle 1 closed the file-parser **arithmetic**,
cycle 2 closed the SQLite **ingestion and launch**, cycle 3 closes the
**string repair and estimation** path — the last reproducible-panic HIGH
on the board.

**Lead re-verified live this turn** (nothing about it is stale): on the
current tree and the release binary built this session,
`agenttrace <file> --overview` exits 101 with
`panicked at crates/agenttrace-core/src/parser.rs:3785:28: end byte index
17 is not a char boundary` for `{"prompt":"\u中文测试"}`; the directory
form `-d /tmp/assess-repro --overview` also exits 101; a freshly
regenerated reproducer reproduces identically. Nuance discovered while
re-verifying, for the implementer: `-d <single corrupt file>` takes a
different path and exits 1 "No session files found" instead of crashing —
write the regression tests with the **positional-file and directory**
forms that actually crash.

Why this batch wins every test applied to it:

- **It contains the only reproducible-panic HIGH left.** P6-1 kills every
  non-interactive surface (`--overview`, `--doctor`, `--waste`, `--latest`,
  `--sessions`, `--diagnostics`, positional files, directory scans) from
  *format detection* — before a session is even accepted — in the shipped
  release build. A one-line hostile log anywhere in a scanned directory
  takes down the whole report. Slicing panics are profile-independent.
- **The acceptance net is proven blind, so the fix must bring its own
  corpus.** The 7,500-run mutation fuzzer never emitted a `\u` escape
  before multi-byte bytes, and the committed adversarial corpus contains
  none; only `\ud83c` + ASCII is tested anywhere. The batch converts the
  pass-6 reproducers into generated fixtures, closing the class.
- **P6-4 is the same defect family wearing a softer mask**: numbers derived
  from untrusted strings (bytes÷4 token estimates under-counting CJK
  ~40–60%, `reasoning_chars` storing bytes) — honest-numbers work that
  finishes the theme rather than starting a new one.
- **The riders are claim-integrity and reliability items cut from the same
  cloth**, each XS–S with zero design risk beyond one recorded either/or,
  and none shares files with the lead (P6-2 → `main.rs`/CHANGELOG; P6-3 →
  `session_cache.rs`; C34 → `sqlite_sessions.rs` naming).
- **It is the only batch on the board that is simultaneously highest-value
  and end-to-end completable here** (see rejected alternatives).

## 2. Scoring — new items this round

Imp = impact (1–5) · Risk = risk if deferred · Eff = effort · Dep =
external dependency · Strat = value to the two product jobs
(cross-agent history review; slow/regressed-task diagnosis).

| # | Item | Imp | Risk | Eff | Dep | Strat | Disposition |
|---|---|---|---|---|---|---|---|
| P6-1 | `repair_lone_surrogates` non-char-boundary slice; release exit 101 from format detection; all report actions (`parser.rs:3785`, `:3791`) | 5 | 5 | S | none | 4 | **batch lead** |
| P6-4 | bytes÷4 fallback estimate under-counts CJK; `reasoning_chars` stores bytes (`lib.rs:555`, `:562`, `:577`) | 3 | 2 | S–M | byte-vs-char design note | 3 | **in batch** |
| P6-2 | CHANGELOG over-claims `--version` precedence; `--overview --version` rejected (`main.rs:136` before `:150`) | 2 | 2 | XS | either/or: hoist vs reword | 1 | **rider** |
| P6-3 | fixed `*.json.tmp` sibling races across concurrent processes (`session_cache.rs:226`, `:518`) | 2 | 2 | XS–S | none | 2 | **rider** |
| C34 | provider placeholder titles used verbatim; 227/227 local opencode sessions junk-named (`sqlite_sessions.rs:672-676`) | 3 | 3 | S | none | 3 | **rider** |
| C33 | per-turn model attribution (`parser.rs:2161-2164` freezes session model; `isSidechain` unread) | 5 | 4 | M | none | 5 | capability lead, **next cycle** |
| C35 | Claude Code `modelPricing` org-contracted rates ingestion | 4 | 3 | S–M | sequenced after C24 | 4 | capability cycle |
| C36 | opencode event-log incremental cache sync (`max(seq)` watermark) | 3 | 2 | M | upstream event-type semantics | 3 | later cycle |

## 3. Standing dispositions (re-affirmed, not re-scored)

The prior records' rows remain valid; no code has changed under them. The
next candidates after this batch, in order:

- **P3-1 platform parity (HOME-only discovery, HIGH)** stays the top
  *deferred* item and the natural cycle-4 lead **only when** its
  verification gap closes (a Windows runtime or a CI job that exercises
  the resolver chain — CI execution is outside this cycle's permissions).
  It cannot be honestly closed on this host today; a batch that ships
  unverifiable acceptance is worse than a deferred one.
- **C33 per-turn model attribution** is the capability lead for the cycle
  after P6-1 lands: highest strategic score on the board (ccusage shipped
  its equivalent; mixed-model sessions misprice today), M effort, no
  external dependency, fully testable here with fixtures. Pair it with
  **C25** (`thoughtsTokenCount`) as its second change unit.
- **Output-honesty sweep** (P3-4 control characters + P3-7 newline parity +
  P4-2/P4-3 CLI guards) as its own cycle: it changes every report byte and
  needs its own corpus + docs sweep; smuggling P3-4 into this batch would
  blur two acceptance surfaces for no dependency savings.
- **N4/dependency motion** (ureq 3, rusqlite/crossterm refresh, MSRV) as
  one-motion cycle per its roadmap entry. **N5/P4-4/P4-5 CI-truth** needs
  `expect`/CI-image changes; **P5-6** needs a published artifact; both
  stay parked. **P6-4's byte-vs-char decision** is recorded in-batch (see
  order below) rather than deferred.

## 4. Batch execution order (red → green)

1. **Fixtures first (red).** Extend the fixture generator with a
   `unicode-escape` family under `testdata/generated/adversarial/`:
   `{"prompt":"\u中文测试"}` (slice at `:3785`),
   `{"prompt":"\ud800\u中文测试"}` (slice at `:3791`), a valid surrogate
   pair `\ud83d\ude00` (must stay repaired, not dropped), and `\u` +
   non-hex-ASCII. Wire into `discovery_contract.rs` using the
   **positional-file and directory** forms (the `-d <single file>` path
   exits 1 before the crash). Assert: no panic in either build mode, the
   file is labeled unparseable-or-degraded, neighboring clean sessions
   still count.
2. **CU-1 (P6-1).** Parse hex from bytes — collect `bytes[i+2..i+6]` into
   a `[u8; 4]` only when all four are ASCII hex, else leave the escape
   untouched — or `is_char_boundary`-guard both slice edges. Unit tests:
   valid pairs repaired, lone surrogates handled as today, multibyte after
   rejected `\u` never panics.
3. **CU-2 (P6-4).** Record the byte-vs-char decision per estimate site
   (`lib.rs:555`, `:577` estimate; `:562` `reasoning_chars`): count
   `chars()` where characters are intended, or keep bytes and rename the
   field + label the estimator in provenance. CJK fixture asserting the
   estimate within a stated tolerance.
4. **CU-3 (P6-2).** Either hoist the `--version` early return above
   `validate_primary_action` (`main.rs:150` before `:136`) or reword
   `CHANGELOG.md:11`; decide at implementation time by which produces the
   least surprising flag grammar, and pin `--overview --version` with a
   CLI test either way.
5. **CU-4 (P6-3).** Unique temp suffix (pid + counter) for the cache
   persist temp file, same atomic rename; concurrent-persist test with two
   writers asserting no failed rename and a loadable cache.
6. **CU-5 (C34).** Treat `^New session - ` titles as absent in
   `sqlite_sessions.rs:672-676`, fall back to message-derived naming from
   the already-scanned `message` rows, stamp naming provenance
   `provider:placeholder`; fixture with the placeholder pattern asserting
   the derived name wins.
7. **Verification matrix** (same bar as cycle 2): `cargo test --workspace`
   debug and release, `cargo fmt --check`, `cargo clippy --workspace
   --all-targets`, `scripts/ci/check-*.sh` against a fresh release build,
   and the pass-6 reproducers re-run end to end exiting 0 with degraded,
   labeled output.

## 5. Rejected batches (with reasons)

- **"Platform parity" (P3-1 lead + resolver unification + P5-6):** contains
  a HIGH and real user pain, but its acceptance (per-OS fallback-chain
  tests, MSRV CI) cannot be executed or observed on this offline Linux
  host — the batch would end "implemented, unverified," which the fleet's
  completion-lock forbids valuing.
- **"Capability accuracy" (C33 + C25 + C24):** the highest strategic
  bundle, but three M-effort units touching pricing/report contracts in
  one cycle exceeds the demonstrated batch size and would compete with
  closing the last panic HIGH. C33 leads the next cycle instead.
- **"Output honesty" (P3-4 + P3-7 + P4-2/P4-3 + N7 residue):** coherent,
  but every report format's bytes change; it needs its own corpus, docs,
  and baseline-shift changelog note, and shares no dependency with P6-1.
- **"Dependency motion" (N4 ureq 3 + refresh + MSRV):** one-motion rule,
  registry fetch, CI-dependent validation; not while a release-reachable
  panic is open.
- **"Cache durability" (P4-6/P4-8 + P6-3):** real, but its centerpiece
  (eviction policy) is a design decision deserving its own acceptance
  note; only the XS race fix (P6-3) rides this cycle.

---

**Artifacts:** this file. `ROADMAP.md` untouched by this phase.
**Verification of grounding claims this turn:** release-binary crash
re-runs above; anchors `parser.rs:3785/:3791`, `lib.rs:555/:562/:577`,
`session_cache.rs:226`, `main.rs:136/:150` re-checked against the tree.
