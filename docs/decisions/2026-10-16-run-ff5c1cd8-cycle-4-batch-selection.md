# Run ff5c1cd8 cycle 4 batch selection + outcome — 2026-10-16

Run `ff5c1cd8914b4a01be9f3a785464595d` · repository-maintenance
`78307520b25c4b5a92e3f8e084c57254` cycle 4 · base `3e90993` (== origin/master
at dispatch) · batch tree salvaged whole by the engine as `dc7bcac`
("conductor-salvage: run=ff5c1cd8914b4a01be9f3a785464595d", 7 files,
+830/−58, porcelain 0 after). This document records the selection, the
pre-review outcome, the standing reds, and the next-cycle candidates, and
completes the cycle's compound phase (attempt `78c3277a2684442982b64d500246c167`).
Review and shipping outcomes happen after this record; the next cycle's
assessment carries them forward.

## Inputs (all consumed as recorded; none re-derived, no tests re-run)

| phase | attempt | dossier |
| --- | --- | --- |
| assess | `bcfcc14f` | findings F1–F8, hermetic battery 1118/0/0 across 62 suites, fmt/clippy rc0 |
| research | `cf58ea53` | upstream tip 15ed07f2 / v0.10.1, ccusage 20.0.31, opencode v1.19.0, claude-code 2.1.299, live LiteLLM diff |
| roadmap | `61c524f0` | spool-side wall delta: 15 riders + 1 banner + mint rm-1425 (392→393 rows) |
| prioritize | `d58b57af` | selection memo (this batch; deferred §6) |
| stewardship | `e6079caf` | CU-A/B/C pre-flight census, disjoint-claim check |
| implement | `1f2cb8e6` | the batch; validation digest re-derived with the engine's own code |
| targeted_tests | `a27e341a` | green, zero edits |
| full_tests | `b5b57c8f` | 6/6 legs rc0 at `dc7bcac`, porcelain 0 in/out |

Operational note: the spool reaper swept most of this run's scratch dirs
and `/tmp` mirrors (quarantined under `quarantine/scratches/20261014T*` /
`20261015T*`; the reaped dossiers were recovered from there). The
roadmap-phase `ROADMAP.patch` (md5 `07c8236e21dc0bff00d2aed00a8422d5`) was
re-proven byte-identical to a fresh deterministic regeneration from the
unchanged wall, and its postimage re-derived at the banked md5
`258ce2c04783de576b23f22f3ffc9619` — the quarantine copies are authentic.

## Selected batch — "Doctor completes honestly on real homes"

- **LEAD: rm-367 stages (b)(c)(d)** — reliability 82.0, `in_progress`, the
  wall's highest-priority genuinely-unimplemented row family
  (ROADMAP.md:339 area). The wall row already spec'd the exact fix family
  for assess F1, the cycle's top finding: `agenttrace --doctor -f json` on
  the operator's real home hit `rc=124` at BOTH `timeout 240` and
  `timeout 900` with zero bytes of stdout (61.9 % CPU mid-run).
- **PAIR: rm-1425** — reliability 54.0, this run's own roadmap mint: the
  output-contract gate's doctor probe ran against the operator's real
  `HOME` and mutated the real cache (assess F2: sessions.json md5
  `5e039ab9→3e22430` across a gate-equivalent run).

Stage (a) of rm-367 (caching finder + persist parses) had already landed
in prior cycles; stages (b)(c)(d) were the open arms. Competitor
precedent consumed from research: ccusage #1794 (v20.0.26) — stale-file
skip for date-bounded loads.

## Outcome (pre-review)

CU-A — **doctor completes honestly** (rm-367 b/c/d, `crates/agenttrace-core/src/doctor.rs` + `sqlite_sessions.rs`):

- Per-root wall-clock budget `AGENTTRACE_DOCTOR_ROOT_BUDGET_MS` (default
  60 s) and per-root parsed-byte cap `AGENTTRACE_DOCTOR_ROOT_MAX_BYTES`
  (default 512 MiB); bounded SQLite lane with per-db
  `AGENTTRACE_DOCTOR_DB_BUDGET_MS` and total
  `AGENTTRACE_DOCTOR_SQLITE_TOTAL_BUDGET_MS` (watcher-thread interrupt).
- Truncation is always disclosed: `skipped_budget`/`skipped_size` counts,
  `sqlite_partial_dbs` listing (never persisted to the snapshot), and a
  `scan_partial` "scanned N of M roots" block that is OMITTED when the
  scan is complete — so clean/`--demo` outputs stay byte-identical
  (pinned by contract).
- Progress on stderr before each root's first parse and before each
  database ingest; cache hits are budget-exempt (pinned).

CU-B — **call-site honesty**: 13 call-site fixes; `LoadReport` boxed in
agenttrace-tui `app.rs` to keep TUI builds honest.

CU-C — **gate scoping** (`scripts/ci/check-output-contract.sh`, rm-1425):
doctor probe now runs against a FIXED throwaway home under the gate
TMPDIR (`HOME`, `XDG_CONFIG_HOME`, `XDG_CACHE_HOME`, `XDG_DATA_HOME`,
`AGENTTRACE_SESSION_CACHE_DIR` overridden in a subshell). Fixed path, not
`mktemp`: `DoctorReport.cache_path` embeds the cache location, so a
random temp path would leak into the captured artifact. Scoping template
for the remaining `check-*.sh` gates recorded in CHANGELOG + the script
comment.

**Acceptance on the red's own corpus** (implement `1f2cb8e6` artifacts,
swept with the scratch — the envelope record is authoritative): the F1
home that red rc=124 at both timeouts now completes **rc 0 in 4m1.410 s
real / 2m59.991 s user under `timeout 300`** — an 11,553-byte valid-JSON
report carrying `scan_partial` + 2 truncating roots +
`sqlite_partial_dbs: 1`; the `sessions.json` cache-persist side effect is
disclosed (md5 `702aa07e→637f53e9`, never deleted). Gate legs rc 0 ×2,
captured `doctor.json` byte-deterministic (md5 `5973f4d9…` both legs),
real sessions.json md5 `702aa07e` unchanged before == after, NO-RESIDUE.

**Validation, consumed as recorded (not re-run by the compound phase):**

- targeted `a27e341a`: `doctor_scan_budget_contract` 6/0 (new file),
  `doctor_honesty_contract` 3/0, core lib 399/0, tui lib 60/0 — all rc 0
  under hermetic env.
- full `b5b57c8f` at `dc7bcac`, porcelain 0 in/out: 6/6 legs rc 0 — fmt;
  workspace `--all-features --no-fail-fast` 63 `test result` lines,
  **1124/0/0** (+1 suite / +6 tests vs the assess-at-base 62 lines /
  1118 — exactly the new contract file; lineage marker `sqlite_partial_dbs`
  ×3 in the battery's own build); clippy `--all-targets --all-features
  -D warnings`; `bash -n`; the scoped gate itself rc 0 over the fresh
  debug binary.

**Redo-chain provenance** (why implement was attempted 4×): `1e7b1b93`
died of provider failure ~35 min in leaving an uncommitted tree delta;
`e6f07cfe` adopted it and completed the batch but wrote an envelope whose
`validation_evidence` was NULL — rejected by the fold gate under KTD13
(repository-maintenance ≥ v6: implement folds need a structured
`changed_surfaces` attestation; this delta holds 1 executable surface);
`f7a4f90b` died of ENOSPC at 3 min. `1f2cb8e6` redid the record with the
digest re-derived by the engine's own code
(`validation:v1:d6933a697a500cf116a7a5c07da622140e88fefdc3cb6a5cdfdbe5c08a759ff3`,
== dispatch) and the batch then salvaged whole as `dc7bcac`.

## Standing reds at this base (NOT batch regressions — all 6 full legs green)

1. **check-install-runtime test D / rm-051** — rc 1 "install receipt must
   record the default pinned ref (rm-051)" reproduced live at `3e90993`
   (assess F6). The batch touches no install surface. Rider-recorded by
   the roadmap phase on rm-012.
2. **TMPDIR-inside-`.git` environmental flake** (assess F5):
   `upstream outside_a_git_repository_fails` exit 101 under the delegate
   TMPDIR, 1 passed under `TMPDIR=/tmp`. Rider on rm-241; prevention rules
   9–10 live in sibling run 5c144613's unlanded band (reconcile by title,
   no duplicate mint).

## Durable lessons recorded this phase

- `docs/maintainers/test-flake-prevention.md` **Rule 8 postscript** — the
  doctor stall Rule 8 documented is now FIXED (budget + honest partial
  disclosure); the isolation guidance stands for determinism.
- `docs/maintainers/test-flake-prevention.md` **Rule 11 (new)** — a dead
  attempt's scratch logs describe a past tree, not this one: this cycle's
  targeted forensics watched a reaped attempt's `focused-battery.log`
  show 16× E0609 "no field `sqlite_partial_dbs` on `DoctorReport`" while
  the live tree HAS the field at `doctor.rs:159`; re-derive compile-failure
  claims against the live tree before treating them as defects.
- Engine-side (recorded here, not a repo rule): a completed batch whose
  envelope lacks the structured attestation is rejected at the fold —
  machine-derive every digest at record time; and quarantine banking +
  deterministic builders make reaped roadmap patches recoverable
  byte-exactly (proven above by regeneration).

## Deferred — ranked next-cycle candidates

1. **Install hardening** (assess F3 + upstream #314/#317/#315): PSReadLine
   arch fallback (`install.ps1:14-16` is the exact pre-fix code),
   `AGENTTRACE_VERSION` env pin (README.md:88-89 documents a pin that
   cannot reach `irm | iex`), install.cmd retirement, unbounded downloads
   (`install.ps1:51/:64`, `install.cmd:7`, npm `install.js download()` no
   timeout). The install.sh/install.js TIMEOUT arms are claimed by
   sibling run 96287e10's unlanded rm-1515 — reconcile by title; the
   ps1/cmd + env-pin arms stay open. Pairs with rm-550 Windows ports.
2. **openclaw claim surfaces** (assess F4: 6 doc surfaces, 0 code hits) —
   ceded to fog rm-1402 (978e457a band) per the roadmap cession.
3. **Pricing drift** (assess F8 + research §D): PRICING_SNAPSHOT_DATE
   2026-10-08, live 4,083 rows vs bundled 3,130; GPT-6 family
   priority/ultrafast/above_272k tier rates carried live but
   `lookup_price_tiered` maps only flex (`pricing.rs:148-152`) while
   opencode v1.19.0 emits `ultrafast` turns now. Blocked behind the
   pricing storm (sibling rm-1508's landing + rm-176 reconcile), then
   tier completion is the correctness lead.
4. **metadata-only --search replay** (assess F7) → fog rm-1409/rm-1415
   numeral storm — reconcile by title.
5. **rm-1054 dir-scope banking** (standing starter; read 339a76fe's
   landed semantics first).

## Commit-gate seams

- ROADMAP lands as TWO spool-side patches IN ORDER: wall (md5
  `1d59adda…`) ← `61c524f0/ROADMAP.patch` (→ postimage md5
  `258ce2c0…`) ← `78c3277a/ROADMAP.compound.patch` (→ final postimage
  md5 `db39d18bdcb3ec34657e176074332f9a`). Both round-trip-proven
  (`git apply` exact + `patch -p1 -F3`) on fresh copies.
- rm-367 and rm-1425 flip to `implemented 2026-10-16 (…)` with dated
  outcome riders in the compound patch; **done-flips stay reserved to the
  shipping gate** (rm-012 convention). Sibling title-adjacent rm-1509
  (run 5c144613, unlanded) is discharged by this batch — flip it with a
  cross-ref rider at integration, do not re-implement.
- Stage exactly: this file, `docs/maintainers/test-flake-prevention.md`,
  and `ROADMAP.md` (after both patches). The worktree otherwise stays at
  `dc7bcac`, porcelain clean.

## Verified facts

- Worktree HEAD `dc7bcac544caf02bf822273983c27831594c8b36`; `git diff
  --stat 3e90993..dc7bcac` == 7 files, +830/−58 == the declared batch
  surfaces (CHANGELOG.md, doctor.rs, lib.rs, sqlite_sessions.rs,
  doctor_scan_budget_contract.rs new, app.rs, check-output-contract.sh).
- Worktree ROADMAP.md untouched by this phase (md5 `1d59adda…`, 392 def
  rows); the compound wall delta is spool-side per the rm-012 convention.
- Compound wall census: 393 def rows in/out, def-max rm-1425 (numeric),
  0 duplicate ids, managed footer last, ZERO ids minted.
- No test, gate, or validation command was executed by this phase.
