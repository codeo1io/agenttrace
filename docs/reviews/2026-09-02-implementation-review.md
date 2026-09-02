# Code Review Results — independent_review (cycle 1, H1+H2 batch + F12/F14/F19)

**Scope:** working tree vs base `e005952` (root commit) — 10 tracked files modified, exec_lines 407; 3 untracked load-bearing artifacts reviewed by disclosed scope widening (`crates/agenttrace-core/src/pricing_snapshot.json`, `scripts/pricing/update-snapshot.sh`, `scripts/ci/check-plugin-version.sh`)
**Intent:** close F1 (adversarial token-count overflow: no panic, no negative totals, saturating aggregation, range-validating `number_as_i64`) and F6 (offline-by-default pricing: vendored dated snapshot, zero network on default/test paths, cache never rewritten, `--update-pricing` the only network path), plus ride-alongs F12 (`--lang` validation), F14 (`.gitignore` cleanup), F19 (plugin-version CI guard)
**Mode:** markdown, report-only (no local apply authorized)

**Reviewers (roster run in-thread, disclosed):** correctness, testing, maintainability, security, performance, api-contract, reliability, adversarial
- correctness/adversarial — untrusted-input arithmetic hardening is the cycle's core risk surface
- testing — new CI guard scripts and test claims constitute a silent-pass surface
- security — H2 network-boundary and cache-write claims
- performance — 533 KB vendored snapshot embedded in the binary
- api-contract — `pricing_source` label vocabulary and `--lang` value changes
- maintainability/reliability — duplicated aggregation helpers, new test env plumbing

**Independence disclosure (INDEPENDENCE_ACCOUNTING):** this harness exposes no subagent primitive; all eight personas ran in-thread as lenses of a single reviewer. The `independent_reviewers` lists produced by the mechanics helper are single-source artifacts, not corroboration. No cross-model adversarial pass ran: host-family attestation failed (`CLAUDECODE`/`CODEX_SANDBOX`/`CURSOR_AGENT` all unset -> unknown family), which cannot satisfy the same-family exclusion, so the automatic cross-model route was skipped deterministically. No Stage 5b validator was available for the same reason; every P1/P2 finding below is instead anchored to direct empirical reproduction (deterministic repro, both build modes, backtrace-pinned frames) and is marked validation-degraded in Coverage.

### Triage Groups

| Group | Findings | Context | Preferred Resolution | Why |
|-------|----------|---------|----------------------|-----|
| Adversarial token hardening finish (apply-queue) | #1, #4, #2, #5 | The F1 fix stopped at the per-session layer; three unguarded aggregation sites remain, and the detection net (#5) was never committed | Fix parser entry first (#1 `sum_numbers`, #4 workbuddy), then the report/TUI layer (#2), then commit the corpus as the regression net (#5) | One saturating-arithmetic pass plus one committed fixture closes all four; #5 is what keeps them closed |

#3 (CI wiring) stands alone: mechanical, independent of the arithmetic group.

### P1 — High

| # | File | Issue | Reviewer | Confidence |
|---|------|-------|----------|------------|
| 1 | `crates/agenttrace-core/src/parser.rs:3573` | `sum_numbers` alias aggregation overflows on legal JSON integers | adversarial | 100 |
| 2 | `crates/agenttrace-core/src/reports.rs:1518` | Cross-session token totals still overflow — original F1 symptom remains reachable | correctness | 100 |

- **#1** — `sum_numbers` sums alias keys with plain `.sum()`. Two legal in-range integers (`"input": 9223372036854775807, "input_tokens": 9223372036854775807`) overflow; the `number_as_i64` clamp cannot prevent it because each value is individually in range. Reachable from `oh_my_pi_usage` (parser.rs:1380-1383) and `qwen_usage` (parser.rs:1774-1791). Reproduced: debug binary panics inside `sum_numbers` (backtrace-pinned); release binary silently drops the wrapped-negative usage — session reports $0.0000 and no tokens. Fix: `.fold(0i64, i64::saturating_add)` at parser.rs:3570-3575.
- **#2** — `overview_summary` sums per-session totals with `.sum()`. Two session files whose usage saturates each session to `i64::MAX` (e.g. `input_tokens: 1e300` in claude_code-format logs) make the debug binary panic (`attempt to add with overflow`, frame `agenttrace_core::reports::overview_summary`) and the release binary print `summary.total_tokens = -2` — the exact original F1 symptom. The same unguarded pattern is duplicated in the TUI (`total_tokens_all` at `shared.rs:226-231`, `presentation.rs:3596-3601`, reached from `explorer.rs:667/875`). Fix: saturating fold at all three sites plus a two-session regression test.

### P2 — Moderate

| # | File | Issue | Reviewer | Confidence |
|---|------|-------|----------|------------|
| 3 | `.github/workflows/ci.yml:51` | `check-plugin-version.sh` shipped but never executed by CI | testing | 100 |
| 4 | `crates/agenttrace-core/src/parser.rs:639` | Workbuddy cache subtraction underflows on adversarial usage | adversarial | 100 |
| 5 | `docs/decisions/2026-09-02-cycle-1-batch-selection.md:105` | Committed adversarial repro-corpus fixture (H1 evidence) never landed | testing | 75 |

- **#3** — the F19 guard exists as an untracked script and passed when run manually, but `.github/workflows/ci.yml` (steps at lines 51-104) references the other eight check scripts and never this one; no other script calls it either (exhaustive grep). A guard that never runs detects nothing. Fix: add a workflow step alongside `check-cargo-manifests.sh` (~line 104).
- **#4** — `*input = (*input - cached).max(0)` underflows when `input_tokens` clamps to `i64::MIN` (e.g. `-1e300`) and `cache_read_input_tokens` is positive. Reproduced: debug panics with `attempt to subtract with overflow` at parser.rs:639; release wraps to a huge positive that survives `.max(0)` — report shows `total_tokens = 9223372036854775807`. Fix: `input.saturating_sub(cached)`.
- **#5** — both ROADMAP.md:63-68 and the decision doc make a committed generated fixture from the repro corpus part of H1's acceptance evidence. Nothing exists: `testdata/` and `testdata/generated/` hold provider-format fixtures only; `git ls-files` has no adversarial corpus. This missing net is why #1/#2/#4 survive: nothing in CI exercises them. Fix (design note): commit a generated corpus under `testdata/generated/` (two 1e300-usage sessions; an oh-my-pi usage with two `i64::MAX` aliases; a workbuddy negative-input entry) plus a pipeline test asserting no panic and non-negative totals. The three reproducers used in this review are ready-made seeds.

### Requirements Completeness (plan_source: inferred — ROADMAP.md + docs/decisions/2026-09-02-cycle-1-batch-selection.md)

| Requirement | Status |
|---|---|
| H1 saturating aggregation across aggregation paths | partially addressed — per-session paths hardened (`lib.rs:527` clamp, `:534-543` saturating accumulators, `:1087-1094` saturating `total_tokens`); cross-session and parser sums remain (#1, #2, #4) |
| H1 `number_as_i64` range-validates | partially addressed — clamps (saturates) with comment and tests; roadmap wording diverges (residual risk, demoted P3) |
| H1 committed adversarial fixture | not addressed (#5) |
| H2 zero network on default/test paths | met — dead-proxy + empty-cache `--demo --overview -f json` exits 0, no cache files created, no download attempt |
| H2 dated vendored snapshot in binary | met — `pricing_snapshot.json` 533,393 bytes, 2,458 models, `_snapshot.date` 2026-09-02 matches `PRICING_SNAPSHOT_DATE`; `claude-sonnet-4-5` present |
| H2 wall-clock-free `pricing_source` | met — labels derive from the date constant; consecutive runs byte-identical |
| H2 cache never rewritten, stale served as-is | met — backdated `pricing.json` survives a default run byte-identical (md5 verified); unit test asserts same |
| H2 PRIVACY.md accuracy | met — matches shipped offline/snapshot/`--update-pricing` behavior |
| F12 `--lang` validation | met — `--lang fr` errors with exit 1 and a clear message; `--lang zh` renders Chinese |
| F14 `.gitignore` cleanup | met — no dangling `agentwaste`/`apps/desktop` references remain |
| F19 plugin-version guard wired into CI | partially addressed — script exists, CI never runs it (#3) |

### Actionable Findings

| # | File | Issue | Route | Notes |
|---|------|-------|-------|-------|
| 1 | `crates/agenttrace-core/src/parser.rs:3573` | `sum_numbers` `.sum()` overflow | `gated_auto -> downstream-resolver` | `suggested_fix` present — one-line saturating fold, covers all 8 alias sums |
| 2 | `crates/agenttrace-core/src/reports.rs:1518` | Cross-session `.sum()` overflow (+2 TUI copies) | `gated_auto -> downstream-resolver` | `suggested_fix` present — three-site saturating fold + regression test |
| 3 | `.github/workflows/ci.yml:51` | New guard script not wired | `gated_auto -> downstream-resolver` | `suggested_fix` present — one workflow step |
| 4 | `crates/agenttrace-core/src/parser.rs:639` | Workbuddy subtraction underflow | `gated_auto -> downstream-resolver` | `suggested_fix` present — `saturating_sub` |
| 5 | `docs/decisions/2026-09-02-cycle-1-batch-selection.md:105` | Adversarial fixture not committed | `manual -> downstream-resolver` | `suggested_fix` present — corpus + pipeline test; seed reproducers are in this review |

### Coverage

- **Roster:** full 8-persona roster, in-thread (no lite path; exec_lines 407). No subagent primitive in this harness; disclosed above.
- **Cross-model:** skipped — host-family attestation failed (unknown family), same-family exclusion unsatisfiable. Recorded in `routing.md`.
- **Validator (Stage 5b):** unavailable for the same reason; all findings kept as validation-degraded. Evidence basis in lieu: direct empirical reproduction of #1, #2, #4 (deterministic corpora, both debug and release builds, backtrace-pinned frames, isolated caches) and exhaustive-grep proof for #3, absence-proof for #5.
- **Mechanics:** first helper pass demoted 5 findings at the quote-the-line gate (missing top-level `first_evidence`); quotes were re-anchored as verbatim motivating lines and the gate passes on the second pass. 2 findings suppressed at anchor 50 and demoted to residual risks (mode-aware demotion count: 2). 0 malformed findings/returns. Stable `#` assignment from the final helper pass.
- **Settlement suppression:** not evaluated — no `plans/`/`solutions/` KTD store exists in this repo.
- **Baseline re-verified:** `cargo test --workspace` 154 passed / 0 failed; `cargo fmt --all -- --check` clean.
- **Residual risks:** session cache can mask parser regressions on re-runs (isolated `XDG_CACHE_HOME` required for adversarial repros); CI runs tests in debug only, so release-mode wrapping stays silent; `--update-pricing` download path untestable offline (out of H2 scope); 533 KB snapshot embedded in an 11.1 MB binary (acceptance-mandated); `std::env::set_var` in parallel tests (documented in code); ROADMAP "range-validates" wording vs shipped clamping; latent pid-keyed temp-dir race in `with_isolated_cache_env` (0/40 stress runs); duplicated `total_tokens_all` implementations drift-prone; pre-existing F11 (concurrent session-cache rename race) untouched and out of scope.
- **Testing gaps:** no multi-session adversarial corpus test; no alias-pair `i64::MAX` test for oh-my-pi/qwen; no workbuddy negative-usage test; no CI execution of `check-plugin-version.sh`; no release-mode non-negative-totals assertion.

---

**Verdict: Not ready.**

The H1 acceptance criterion — "no panic and no negative totals on adversarial token counts" — is empirically violated in both build modes: the debug binary panics and the release binary prints `total_tokens = -2` on a two-file adversarial corpus (#2), and the parser itself both panics (debug) and silently drops usage (release) on legal alias-pair integers (#1). H2 is fully verified and clean; F12/F14 are done; F19 is half-done (#3). Fix order: #1 -> #4 -> #2 (one saturating-arithmetic pass), then #5 (commit the corpus so the fix stays closed), then #3 (wire the CI step). All five have concrete suggested fixes and are downstream-resolver actionable; nothing requires a design decision beyond #5's fixture shape.

Actionable recap (self-sufficient):
- **#1 P1** `parser.rs:3573` — `sum_numbers` `.sum()` overflow on alias pairs; saturating fold. gated_auto, conf 100, reproduced.
- **#2 P1** `reports.rs:1518` (+`shared.rs:226`, `presentation.rs:3596`) — cross-session totals overflow; `-2` in release; saturating fold + test. gated_auto, conf 100, reproduced.
- **#4 P2** `parser.rs:639` — workbuddy subtraction underflow; `saturating_sub`. gated_auto, conf 100, reproduced.
- **#5 P2** decision doc :105 — commit adversarial corpus fixture (H1 evidence). manual, conf 75.
- **#3 P2** `ci.yml:51` — wire `check-plugin-version.sh` into CI. gated_auto, conf 100.
