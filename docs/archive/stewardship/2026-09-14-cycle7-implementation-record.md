# Cycle 7 implementation record — trustworthy capture

- **Date**: 2026-09-14
- **Cycle**: 7 (batch: trustworthy capture — hardening-first)
- **Base**: `df3b621` ("docs: record cycle-6 closures and pass-10 findings in the roadmap")
- **Worktree**: `conductor-worktrees/agenttrace-80c75f65b7/run-125bf93302aa-125bf933`
- **Batch selection**: `docs/decisions/2026-09-14-cycle-7-batch-selection.md`
- **Status**: implemented; uncommitted worktree state (commit/push/PR deliberately out of scope)

## What shipped

| # | Item | Lineage | State |
|---|------|---------|-------|
| 1 | Go-flag shim `--no-baseline-gate` misregistration | cycle-4 review F2 + pass-11 A11-5 (both cited) | done, live-verified |
| 2 | Symlinked session-root discovery | Codex `#42135` via research pass 9 (candidate 54 hardening half) | done, live-verified |
| 3 | Statusline capture surface | research pass 9 candidate 53 | done, live-verified |
| 4 | Cache-bound accounting over the deduplicated key union | pass-11 A11-2 (+ cycle-7 F5-5 headerless-entry semantics, subsumed not reverted) | done, tests |
| 5 | Installer mode and checksum parity | pass-11 A11-3 | done, live-verified |
| 6 | TUI delivery-worker error surfacing | pass-11 A11-4 | done, tests |
| R1 | Pricing snapshot refresh + doctor age disclosure | candidate 51 local half | done, live-verified |
| R2 | CI hygiene: fork-tolerant dependency review + reachable TUI-smoke gate | A11-8 + cycle-6 fork observation | done (YAML only; run-level evidence deferred) |

## Item notes

### 1. Shim fix (both lineages, one fix)

`--no-baseline-gate` removed from `flag_takes_value` (`crates/agenttrace-cli/src/main.rs`).
Both accounts — cycle-4 independent-review F2 (never filed, tracked down by
pass-11 A11-1) and pass-11 A11-5 — close with this one change; the ROADMAP
closure entry names both. Shim unit tests pin bool/value-flag adjacency in
both orders; `no_baseline_gate_is_a_boolean_not_a_value_flag`
(`tests/entrypoints.rs`) pins the Go semantics end to end: the post-positional
`--overview` is ignored, the session list prints, and the "choose exactly one
report action" error cannot fire. Companion acceptance P4-2 (warn on
discarded post-positional args) did **not** land and stays open — recorded in
the ROADMAP CLI-surface-polish item.

### 2. Symlinked session-root discovery

- `discovery.rs`: `entry_is_dir_entry(&file_type, &path)` resolves a symlinked
  directory entry once (`fs::metadata` follows the link); both walks
  (`walk_session_files_cached`, `walk_session_files`) use it and thread a
  `SymlinkTargets` canonical-target visit set (`admit` guard) — cycles
  (self-link, parent-link) terminate; depth is already bounded.
- `session_cache.rs`: stored directory listings from the pre-symlink walker
  are retired by `DIR_LISTING_WALK_VERSION = 2` — a doc-level
  `dir_listing_version` key on load (dirty only when directories were
  non-empty), written on save. Chosen over salting listing keys because
  path-shaped keys must stay valid for `prune_dead_entries`' liveness checks
  (a salt would mark every listing dead and orphan them). The cached replay
  loop guards admits as well (belt-and-braces: stored listings are provably
  acyclic, but the replay path defends anyway).
- `doctor.rs`: `DoctorDirReport.symlink_target` (via
  `symlink_metadata` + `read_link`), rendered ` (symlink -> {target})`.
- Live probe (`/tmp/symprobe`): symlinked child discovered (found=2),
  top-level symlink named, self/parent cycle links terminate, 73 ms; cache
  round-trip shows `dir_listing_version: 2`, 4 listings, 2 entries.
- Contract tests: `symlinked_child_directories_are_discovered_and_cycles_terminate`,
  `symlinked_child_directories_are_resolved_through_the_cached_walk`,
  `doctor_names_symlinked_session_roots_instead_of_silent_following`.

### 3. Statusline capture (candidate 53)

New `crates/agenttrace-core/src/statusline.rs`:

- `run_statusline_host()` — bounded (1 MiB) stdin read, exactly one stdout
  line, stderr-only diagnostics, exit 0 for valid/garbage/empty input.
- `render_status_line()` — `name | ctx N%(!) | 5h/7d N% until HH:MM |
  cache N% | $cost` (hit_ratio ×100; fixed mid-implementation bug where 0.904
  rendered as "1%").
- `append_statusline_capture()` / `compact_statusline_capture()` — JSONL
  journal at `$AGENTTRACE_SESSION_CACHE_DIR/statusline.jsonl` else
  `<user cache>/agenttrace/statusline.jsonl`; 10 MiB bound, compaction keeps
  newest whole lines under half the bound via temp+rename;
  `compact_statusline_capture_under()` parameterizes the bound for tests.
- `read_statusline_captures()` skips torn/malformed lines;
  `statusline_insights()` dedups by serialized-payload hash (disclosed:
  "N captures (M after dedup)"), latest-wins window state, peaks, crossings
  require before- and after-side usage around a `resets_at` (a bare timestamp
  or an out-of-journal reset proves nothing), per-session prompt-cache state
  and aggregated miss causes.
- CLI: `agenttrace statusline` intercepted in `run()` immediately after the
  `--version` early return (the host command must never see action
  validation); `--statusline-report` (text/json) as a registered primary
  action. Doctor: `DoctorStatuslineReport` + "Statusline capture:" line.
  TUI: Efficiency panel "Subscription limits" block, loaded once on panel
  open.

**Determinism guard honored**: a dedicated `--statusline-report` flag was
added instead of injecting statusline fields into `--overview`, because
`scripts/ci/check-deterministic-output.sh` (triple-run cmp) pins demo output;
statusline data is machine-local and must not enter that surface.

**Honesty**: fixtures are schema-faithful to the documented payload contract
(code.claude.com/docs/en/statusline), **not** recordings of a real
v2.1.251+ host (none available locally). Disclosed in
`docs/guides/statusline-capture.md` and in the ROADMAP closure note as a
stated deviation from the item's evidence bar.

### 4. Cache-bound dedup union (+ F5-5)

`cache_paths_sized_once(cache)` builds a `BTreeMap` union of raw and decoded
entries; size per path = max of the two serialized lengths. `enforce_entry_bound`
and `enforce_byte_bound` iterate it and remove from both maps (non-short-circuit
`|`), reaching their bound in one pass. Headerless entries order oldest via
`map_or(i64::MIN, |h| h.mod_time)` — this **builds on** the canonical tree's
uncommitted F5-5 headerless-entry semantics rather than reverting them (the
worktree rewrite subsumes those hunks). Tests:
`bounds_count_deduplicated_paths_once` (exact N distinct paths; byte bound
drops exactly the oldest two),
`headerless_cache_entries_are_oldest_not_unevictable`,
`stale_dir_listings_from_the_pre_symlink_walker_are_dropped_once`.

### 5. Installer parity

`install.sh`: `chmod 0755 "$TMP"` before the move (umask-independent);
checksum block downloads `${RELEASE_URL}.sha256` (release.yml already emits
per-asset sidecars and publishes `dist/*` — only install.sh needed parity),
compares via sha256sum/shasum (`cut -d' ' -f1`, `tr -d '\r\n'`), hard-fails on
mismatch with expected/actual + remediation, warns only when the sidecar is
absent (older releases) or no tool exists. `install.ps1` out of scope (Windows
modes differ; recorded in ROADMAP item).

Live evidence, stubbed-`curl` release harness (`/tmp/fake-release`, PATH
first, `FAKE_RELEASE_DIR` fixture serving a ~2 MB fake asset + sidecar):
- **Positive under `umask 077`**: "SHA-256 verified." → "✅ Installed",
  destination `-rwxr-xr-x`, executes ("fake agenttrace binary payload").
- **Tampered sidecar** (all-zeros hash): "❌ Checksum mismatch … Expected …
  Actual … not installing", exit 1, nothing installed.
- **Absent sidecar**: "⚠️ No checksum sidecar …; skipping checksum
  verification." → installs (older-release path).

### 6. TUI delivery-worker errors

Channel carries `Result<DeliveryEvidence, String>`; worker wraps
`delivery_evidence_with_git` in `catch_unwind(AssertUnwindSafe(...))` mapping
panics to `delivery worker failed: {message}`; `poll_governance_delivery`
sets `snapshot.delivery_error` on `Ok(Err)` and on `Disconnected` ("delivery
worker exited without evidence"), clears on success. Presentation: status
"Evidence unavailable" (LightRed) + body with the message and a retry hint.
Test `delivery_worker_failures_surface_as_diagnostics_not_empty_states`
covers Err-message, silent disconnect, rendered diagnostic + retry hint, and
successful-retry reset. (Rendering-spawn nuance: the worker spawns on first
panel render, not on view switch — the test draws once before polling.)

### R1. Pricing snapshot + age disclosure

`scripts/pricing/update-snapshot.sh` run against the live catalog: 2026-09-02
(2,458 chat models) → 2026-09-13 (2,755). The pass-9 "3,923 keys" figure
counts all catalog keys including non-chat/unpriced entries — noted in the
ROADMAP status so neither number is read as a contradiction.
`pricing.rs`: `bundled_snapshot_date()`, `bundled_snapshot_model_count()`
(OnceLock parse of `_snapshot.models`), `bundled_snapshot_age_days()`
(chrono, UTC, clamped at 0). `DoctorReport.pricing` renders
"Pricing snapshot: LiteLLM snapshot 2026-09-13 (bundled, 2755 models, 0 days
old)" — verified live. `PRICING_SNAPSHOT_DATE` const updated in lockstep with
the payload (the existing drift-pin test enforces it). The `data_health` line
and scheduled quarantined workflow remain open (ROADMAP status note).

### R2. CI hygiene (YAML only, per the batch decision)

- `ci.yml`: the PTY TUI smoke step's `if: env.AGENTTRACE_TUI_REAL_DIR != ''`
  (never-true: the variable is set nowhere) is now
  `if: vars.AGENTTRACE_TUI_REAL_DIR != ''` with the variable forwarded as
  step env — the opt-in is reachable by setting the repository variable. The
  script itself was live-verified with `AGENTTRACE_TUI_REAL_DIR=/tmp/symprobe`.
- `dependency-review.yml`: the review step early-exits on fork PRs
  (`github.event.pull_request.head.repo.fork == true`) with an explanatory
  note; repo-owned runs unchanged. Run-level evidence (a cleanly skipped run
  on an actual fork PR) is deferred — no fork PR exists yet and CI execution
  is outside this cycle's gate; recorded as such in the ROADMAP.

## Verification

- `cargo test --workspace`: **230/230** (14 CLI unit + 8 entrypoints + 2
  launch guards + 87 core lib + 7 core integration + 70 TUI + 42 doctor).
  An intermittent lib-test failure first looked like a pre-existing
  `utf16…` flake; chasing it under repeated full-workpace runs identified
  the real cause — the two new env-mutating lib tests (session-cache
  listing invalidation, statusline journal) racing each other and the
  pricing helper on the shared process environment. Fixed with a
  crate-level `test_env::lock_env()` mutex (lib.rs, cfg(test)) now used by
  all three helpers, save/restore instead of remove. Eight consecutive
  full-workspace runs green after the fix.
- `cargo clippy --workspace --all-targets`: clean (0 warnings).
- `cargo fmt --all --check`: clean.
- All ten `scripts/ci/check-*.sh` green against `cargo build --release`:
  output-contract, docs-commands (incl. the guide truth-pins), report
  semantics, deterministic-output (triple-run cmp), cargo-manifests,
  plugin-version, release-surfaces, real-cli-smoke, release-local,
  tui-real-smoke (env-provided corpus).
- No `final_validation`, commit, push, PR, or CI actions performed (out of
  scope for this phase).

## Deltas and disclosures

- **Statusline evidence bar**: schema-faithful fixtures, not real-host
  recordings — disclosed everywhere the claim appears (ROADMAP closure,
  guide, this record).
- **P4-2 not landed**: the shim truncation-warning acceptance explicitly
  remains open; the ROADMAP status note says so rather than claiming
  co-landing.
- **R2 run-level evidence deferred**: YAML landed, fork-PR observation
  pending (needs a fork PR; CI out of scope).
- **Canonical tree**: untouched. Canonical `/work/projects/agenttrace`
  carries uncommitted F5-5 session_cache hunks inside item-4's target
  functions and a duplicate ROADMAP shim entry; this worktree's rewrite
  subsumes the F5-5 semantics — reconcile at the commit gate (dedupe the
  roadmap entry; verify no F5-5 semantics lost against canonical's diff).
- **Test env hygiene**: the statusline journal unit test sets
  `AGENTTRACE_SESSION_CACHE_DIR` before the first append (the earlier
  ordering wrote one fixture to the real `~/.cache/agenttrace/statusline.jsonl`;
  that file was deleted; the real cache now contains only the four
  pre-existing files). Env-mutating lib tests (pricing, session-cache,
  statusline) serialize on one shared `test_env::lock_env()` mutex with
  save/restore — per-module locking let them re-point each other's cache
  roots mid-flight (intermittent `journal_roundtrip…` failure, ~1 in 6
  full-workspace runs, root-caused and fixed this cycle).
- **Dev-dependency added**: `serde_json` (workspace) to `agenttrace-tui`
  for the Efficiency-panel fixture test.

## Files touched (this phase)

- `crates/agenttrace-cli/src/main.rs` — shim fix + tests; statusline host
  intercept; `--statusline-report`; compare_args fixture field.
- `crates/agenttrace-core/src/discovery.rs` — `SymlinkTargets`,
  `entry_is_dir_entry`, both walks.
- `crates/agenttrace-core/src/session_cache.rs` — listing versioning;
  `cache_paths_sized_once`; both bounds; 3 tests.
- `crates/agenttrace-core/src/doctor.rs` — symlink_target; statusline
  section; pricing line.
- `crates/agenttrace-core/src/statusline.rs` — new (module + tests).
- `crates/agenttrace-core/src/lib.rs` — module/export; cfg(test)
  `test_env` shared env lock.
- `crates/agenttrace-core/src/pricing.rs` — snapshot date; three
  accessors. `crates/agenttrace-core/src/pricing_snapshot.json` — refreshed.
- `crates/agenttrace-tui/src/app.rs`, `presentation.rs` — delivery errors;
  statusline Efficiency block.
- `crates/agenttrace-tui/src/tests.rs` — 2 tests;
  `crates/agenttrace-tui/Cargo.toml` — dev-dependency.
- `crates/agenttrace-core/tests/discovery_contract.rs` — 3 tests.
- `crates/agenttrace-cli/tests/entrypoints.rs` — 2 tests.
- `install.sh`; `.github/workflows/ci.yml`;
  `.github/workflows/dependency-review.yml`.
- `README.md`; `docs/README.md`; `docs/guides/statusline-capture.md` (new);
  `CHANGELOG.md`; `ROADMAP.md` (cycle-7 update + six closures + two status
  notes).
