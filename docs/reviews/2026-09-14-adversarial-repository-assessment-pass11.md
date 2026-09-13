# Adversarial repository assessment — 2026-09-14 (eleventh pass)

Run: `58910360ca80424aa2e3c8c9820e6ee5` · phase `assess` · attempt `0f37d3b6392c4139ae6d54e419f22f84`
Head: `df3b621` (2026-09-07; worktree clean except `.conductor/` run artifacts).
Method: fresh build + full test/clippy/fmt sweep, live runs of the debug binary against a
synthetic three-session corpus plus hand-built hostile fixtures (non-report JSON baseline,
XSS-bearing model names, mixed ok/fail Antigravity trajectory), targeted code reading of
`parser.rs`, `lib.rs`, `reports.rs`, `diagnostics.rs`, `session_cache.rs`, `history.rs`,
`sqlite_sessions.rs`, `governance.rs`, `main.rs`, plus cross-checks against passes 1–10 so
nothing below repeats a filed finding. Known-open items (P3-2 local-calendar windows,
P3-7 stdout newline, N8 Markdown escaping, P3-4 control characters, P4-4 vacuous npm test,
the ungated TUI smoke step, history eviction) were verified live but are **not** re-filed.

Baseline before findings: `cargo build` clean, `cargo test --workspace` 213/213 green,
`cargo clippy --workspace --all-targets -- -D warnings` silent, `cargo fmt --check` clean,
`bash -n` clean on shipped scripts.

---

## F11-1 — HIGH (correctness, CI-gate integrity): Antigravity tool-success accounting erases successful command results

`parse_antigravity_trajectory` (`crates/agenttrace-core/src/parser.rs:483-517`) emits
RUN_COMMAND, VIEW_FILE, and ERROR_MESSAGE steps as role-`tool` **result** events that carry
no paired assistant tool call; only PLANNER_RESPONSE `toolCalls` increment
`tool_calls_total`. `analyze()` then clamps `tool_calls_ok` to
`tool_calls_total - tool_calls_fail` — the policy for paired call/result streams — while
failures are never clamped. For a result-only source the clamp deletes every successful
outcome and the failure count over-weights.

Live repro (debug binary, `/tmp/at-adv/traj.json`: one PLANNER_RESPONSE call, three
`exitCode: 0` RUN_COMMANDs, one `exitCode: 2` RUN_COMMAND, one VIEW_FILE):

```
agenttrace --latest -f json traj.json
  -> activity.tool_calls_total=1, tool_calls_ok=0, tool_calls_fail=1,
     tool_success_rate=0
  -> anomalies: tool_failures HIGH "1/1 failed (100%)"  (true: 1 of 5 outcomes)
  -> health_score=40
agenttrace -d antdir --overview --max-tool-fail-rate 15
  -> exit 2, stderr: "Gate failed: tool failure rate 100.0% exceeds 15.0%",
     "critical sessions: 1", "tool fail rate: 100.0%"
```

So a ~20%-failure Antigravity session trips the `--max-tool-fail-rate` CI gate at 100%,
drags the health score into critical territory, and feeds `--fail-on-critical`. Every
Antigravity session with successful commands is misreported in text, TUI, and JSON.

Fix direction: emit synthetic assistant tool-call events for RUN_COMMAND/VIEW_FILE steps
(so results pair with calls), or track result-only outcomes separately from the paired-call
clamp; pin with a mixed ok/fail trajectory fixture asserting a ~80% success rate and no
HIGH tool-failures anomaly. Shares a cycle with candidate 52's corpus work.

## F11-2 — HIGH (release infrastructure): the release build matrix's `os:` dimension is wired nowhere

`.github/workflows/release.yml:40` hardcodes `runs-on: self-hosted` while the matrix at
`:43-61` defines six `os:` keys (`ubuntu-latest` ×2, `macos-15-intel`, `macos-14`,
`windows-latest` ×2) that no `runs-on` references. Git archaeology: upstream `e005952`
correctly used `runs-on: ${{ matrix.os }}`; `6632014` replaced it with a literal;
`bfa4f22` ("restore portable CI runners") restored literals for the verify/publish jobs
but left the build job as a literal `ubuntu-latest`; `fbbf751` set it to `self-hosted`
(`git log -S "matrix.os"` touches only `e005952` and `6632014`).

Consequence: all six legs build on one machine. On a Linux runner the two `*-apple-darwin`
and two `*-pc-windows-msvc` targets cannot cross-compile (no Apple SDK, no MSVC), so a tag
push — the repo carries `v0.0.4`..`v0.3.16`-era tags and an active release history — fails
the build job and the publish job (`needs: [build, verify]`) never runs. The portable
variant is equally broken (all six legs on `ubuntu-latest`). `check-release-surfaces.sh`
greps surface strings only and cannot catch it.

Fix direction: restore `runs-on: ${{ matrix.os }}` semantics (self-hosted equivalents or
per-target labels), and add a guard that fails when a matrix defines `os:` but `runs-on`
does not reference `matrix.os`.

## F11-3 — LOW/MEDIUM (robustness, DX): `--baseline` accepts any JSON as a baseline

`add_baseline_comparison` (`crates/agenttrace-core/src/reports.rs:700-716`) deserializes
any JSON — object or array — into sessions with no format check. A non-report file acts as
an all-zero baseline and produces a misleading regression verdict with exit 2 (live: a
`not-a-report.json` gate-fails with nonsense deltas instead of a clear error). Acceptance:
require the report schema (kind/version marker or sessions-with-metrics shape) and fail
with a named error; tests for non-report JSON, partial JSON, and an array-of-sessions file.

## F11-4 — LOW (consistency): two divergent token estimators

`context_utilization` (`crates/agenttrace-core/src/diagnostics.rs:771-800`) divides bytes
by two (`(content.len() + reasoning.len()) / 2`) while `estimate_tokens_from_text`
(`lib.rs:582`) is CJK-aware. On CJK-heavy sessions the utilization estimate can be ~2× off
the estimator used elsewhere, skewing `utilization_pct` and its risk level. Fold the
history term through `estimate_tokens_from_text`; ties into candidate 4's per-model
context-window metadata (the hardcoded window fallback at `diagnostics.rs:780`).

## F11-5 — LOW (durability approximation): byte ceiling ignores the `dirs` map

`enforce_byte_bound` (`session_cache.rs:652-706`) sizes only `raw_entries` + `entries`
against the 64 MiB ceiling; the `dirs` map (per-source parsed-path lists) and other cache
fields are uncounted, so the on-disk file can exceed the nominal bound. Bounded but
imprecise: include a serialized `dirs` size in the total or document the approximation.
Residual of the CU-22 acceptance.

## F11-6 — LOW (blast radius): orphan sweep deletes by substring

`sweep_orphaned_temps` (`session_cache.rs:288-312`) deletes any cache-dir file whose name
merely **contains** `.tmp.` and is older than one hour. The cache dir is user-settable
(`AGENTTRACE_SESSION_CACHE_DIR`), so pointing it at a folder holding personal
`*.tmp.*`-named files destroys them. Narrow the match to agenttrace's own
`<name>.json.tmp.<pid>.<seq>` pattern. Residual of the CU-9 orphan-sweep acceptance.

## F11-7 — NIT (CLI shim): `--no-baseline-gate` treated as value-taking

`flag_takes_value` (`main.rs:726`) lists `--no-baseline-gate` although it is a boolean
(`main.rs:99`). Benign in practice (the Go-flag shim stops truncating earlier than Go
semantics would), but the list should match the real value-taking flags. Residual of the
P4-2 shim acceptance.

## F11-8 — INFO (records drift): removed guard still described as active

`ROADMAP.md` (cycle-6 Completed record) describes `scripts/ci/check-no-self-hosted.sh` as
a live CI step, but commit `c148ffc` removed it the same day per operator policy, and
`fbbf751` restored `runs-on: self-hosted` on all five workflow spots. `branch.master.remote`
still prints `fork` (verified 2026-09-14), so the upstream-push hazard of pass 9 remains
mitigated by tracking, but the roadmap text and the re-opened upstream-portability question
need a dated status-change note rather than silent divergence.

---

## Cleared with evidence (not bugs)

- `history.rs` torn-file handling quarantines a corrupt history as `<name>.json.corrupt`
  preserving bytes, with tests — pass-7 P7-5 holds.
- "Parse coverage N/M can exceed 100%" is handled: `parse_coverage_phrase`
  (`reports.rs:2517`) switches to "N sessions from M sources" when `parsed > discovered`
  (F8-2's fix covers the sqlite/history merge).
- `sqlite_sessions.rs` opens read-only, uses parameterized SQL throughout, snapshots via
  the session cache; the `?1 is null` arm in `query_hermes_sqlite_sessions` is dead
  (`since` is always `None`) but harmless.
- `npm/scripts/install.js` verifies the binary against the downloaded `.sha256` before
  install, HTTPS-only, 5-redirect cap (the same-origin checksum caveat is the recorded
  accepted risk).
- `analyze_health_trend_full` guards the empty case before any `len()-1` indexing or
  averaging.
- `enforce_byte_bound`'s bitwise `|` on the two `remove(...).is_some()` results is
  intentional-or-harmless (entries are mutually exclusive by design); style only.

Baseline integrity: no source files were modified during the pass; the worktree stayed
clean apart from run artifacts.
