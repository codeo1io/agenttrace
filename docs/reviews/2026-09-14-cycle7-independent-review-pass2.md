# Cycle 7 independent review, pass 2 — post-fix re-review

- **Date**: 2026-09-14
- **Run/phase**: 125bf93302aa4e308cb0739b67f16f33 / `independent_review` (attempt 6047195a43c04988937356ad4b283887)
- **Review target**: the cycle-7 batch *including* the review-fix pass (attempt ea1d5f35…) that consumed `docs/reviews/2026-09-14-cycle7-independent-review.md` (F1–F7). The fix pass's code, tests, ROADMAP/CHANGELOG/guide/distribution claims are part of this review with the same rigor as the original batch.
- **Method**: every fix re-derived live against the rebuilt release binary; full suite, clippy, fmt, and all 10 CI checkers re-run by this review; dispositions audited against the code, not the record.

## Verdict

**PASS WITH FINDINGS (residual Low/Info only — no ship-blockers).** Both Medium findings from pass 1 are verified **fixed** live and pinned by real tests; F4's fix is correct on the documented retry path and additionally fixed a deeper non-respawn bug the fix pass discovered itself. F3/F5/F7 deferrals are legitimately filed as hardening-lane items with acceptance criteria; F6's docs sentence landed. Two residual items remain from the original finding classes (R1: the unfiltered-string class survives on the report/TUI surfaces; R2: an ungated worker-respawn loop in the Delivery failure path), plus one Info note on the sanitizer's scope. Nothing blocks the commit gate.

## Prior findings — verified dispositions

| ID | Severity (pass 1) | Disposition claimed | Verified how |
|----|-------------------|---------------------|--------------|
| F1 | Medium | Fixed (`sanitize_line_segment`) | Live: the pass-1 hostile payload (`\n`, `\r`, `ESC[31m`, `ESC]777;`, BEL, C1 CSI via `model.display_name`) now renders **one line** with `EF BF BD` (U+FFFD) replacements, ` \| ctx 10%` intact, rc=0. Code: `statusline.rs:265-270` sanitizer, applied at `:193` and `:199` (both name sources). Test `hostile_payload_names_render_without_control_characters` (`statusline.rs:869+`) asserts one-line + zero control chars + visible U+FFFD on both paths. |
| F2 | Medium | Fixed (write-and-ignore) | Live: same payload with stdout `> /dev/full` → **rc=0**, no panic (was exit 101). Code: `run_statusline_host` uses `let _ = writeln!(io::stdout().lock(), …)` (`:172`) and the same form for both stderr diagnostics (`:148`, `:156`, capture-failure note); no `println!`/`eprintln!` remains on the host path. Test `statusline_host_mode_survives_stdout_write_failure` (`entrypoints.rs:441+`) spawns the real binary against `/dev/full` and asserts exit 0. |
| F4 | Low | Fixed + extended | Code: `open_governance` (`app.rs:1468-1484`) clears `delivery_error` **and** drops the pre-failure `delivery` it was hiding, so `ensure_governance`'s missing predicate triggers a genuine respawn; spawn arm clears the error as belt-and-braces. The extension is real: pre-fix, a success→failure transition left `delivery: Some`, making the predicate false — the documented retry never respawned at all. Test Case 2b (`tests.rs`) drives the real retry (`run_command("delivery")` + draw) and asserts error cleared, fresh worker pending, stale text gone from the rendered buffer, worker settles. Residual: see R2. |
| F3 | Low | Filed (cycle 8) | Code confirmed unchanged (compaction still read→temp→rename, no lock) and ROADMAP hardening item "Statusline journal concurrent-append race" (:1117ff) carries acceptance criteria (lock or tail re-apply, concurrency test, two-process probe). Legitimate deferral, disclosed. |
| F5 | Info | Filed | ROADMAP "Discovery walk I/O budget for symlink descent" (:1130ff) with budget/diagnostic/test acceptance. |
| F6 | Info | Documented | `docs/maintainers/distribution.md` now states the sidecar check protects against corrupted mirrors/asset drift, not a compromised origin (asset and sidecar share origin/channel). Accurate. |
| F7 | Info | Filed | ROADMAP "Doctor output portability for path pinning" (:1140ff) requiring a normalize-vs-exclude decision before any doctor pin. |

## New findings (this pass)

### R1 — Low — the F1 unfiltered-string class survives on the report and TUI surfaces

`crates/agenttrace-core/src/statusline.rs:628` (`last_miss_causes.join(", ")`), `:641`/`:644` (`Miss causes: {causes}`), and `crates/agenttrace-tui/src/presentation.rs:1381`/`:1395` (TUI Efficiency lines).

Pass 1's F1 explicitly flagged that the same unfiltered-payload-string class reaches `--statusline-report` text and the TUI; the fix sanitized only `render_status_line`, and the disposition record's "numeric segments … were already safe" does not address this half. Live reproduction (isolated `AGENTTRACE_SESSION_CACHE_DIR`): a journal capture whose `prompt_cache.miss_causes` key and `last_miss_cause.causes` contain `\n` and `ESC[31m` renders `--statusline-report` output with raw control bytes — the "last miss" line breaks mid-parenthesis and the ESC passes to the terminal (`Miss causes: evil\nESC<ESC>[31m x3`). JSON output is safe (serde escapes); text and TUI are not. Threat is weaker than F1 (machine-local journal read back to the owner's terminal), hence Low — but it is the same `sanitize_line_segment` remedy, applied at the report/TUI render sites, and belongs beside the open P3-4 control-character-filter item.

### R2 — Low — Delivery worker auto-respawn is ungated: persistent failure respawns workers indefinitely, showing the stale error over each rescan

`crates/agenttrace-tui/src/app.rs:1510` (`missing` predicate for Delivery is `delivery.is_none() && delivery_pending.is_none()` — no `delivery_error` term) with the loop at `app.rs:89-108` and `presentation.rs:907` (`ensure_governance` called during draw).

Mechanics (code-verified): after a worker failure, `poll_governance_delivery` returns true → draw → `ensure_governance` sees `missing == true` → **respawns** a worker on the very next render, without any re-entry by the user. A persistent failure therefore loops: spawn → git-scan (`delivery_evidence_with_git`) → fail → redraw → respawn, at a cadence bounded only by worker duration (≥ `POLL_INTERVAL` = 120 ms, `app.rs:36`) for as long as the panel is open. Two consequences: (a) unbounded background git-scan churn with no backoff; (b) during each auto-respawn the presentation's error-before-pending ordering (`presentation.rs:1057-1060`, `:1134-1147`) shows the stale error instead of "Scanning Git roots…" — the exact F4 symptom, on the path the fix did not cover. The fix-pass record's claim "renders alone never respawn (no worker spam)" is accurate for the success state but **not** for the failure state. This behavior is inherited from the implement phase (the predicate predates the fix), not a regression. Remedy: gate the predicate with `delivery_error.is_none()` (respawn only on explicit retry) or add a backoff; no test currently pins respawn cadence (`grep respawn tests.rs` → 0).

### R3 — Info — sanitizer scope: control characters only

`crates/agenttrace-core/src/statusline.rs:265-270`. `char::is_control` (C0, DEL, C1) neutralizes escape/break injection but not Unicode display-spoofing characters (bidi overrides such as U+202E, zero-width joins) that can reorder the visible status line. Display-spoofing only — no terminal command execution; note-level, reasonable to fold into the P3-4 filter item whenever it lands.

## Verification matrix (this pass, all live)

- `cargo test --workspace` → 14+9+2+88+7+70+42 = **232 passed, 0 failed** (matches the fix-record claim; +2 = hostile-fixture and `/dev/full` tests, +Case 2b extends the delivery test).
- `cargo clippy --workspace --all-targets` → 0 warning/error lines; `cargo fmt --all --check` → clean.
- All 10 `scripts/ci/check-*.sh` → rc=0 against the rebuilt release binary (source mtime 18:40 < binary 18:41 — probes exercised the fixed code).
- Release-binary probes: hostile payload → one sanitized line, rc=0 (`od -c` shows `357 277 275`); `/dev/full` → rc=0; hostile journal → report text reproduces R1.
- Docs/claims audit: CHANGELOG carries the F1/F2 Fixed entries and the F4 retry-semantics extension verbatim-matching the code; the guide's contract bullets now state sanitize + stdout-condition guarantees (both true); ROADMAP's disposition paragraph (:1830ff) accurately summarizes F1/F2/F4 and the F3/F5/F7 filings; `distribution.md` sentence accurate; `dir_listing_version`, shim, symlink, installer, pricing surfaces unchanged since pass 1 (fix pass touched only statusline/app/tests/docs; `git status` confirms no other code files changed).
- F3 code-confirmed still present as filed (no silent partial fix).

## Recommended disposition

Ship: no blockers. R1 is a small same-remedy fix (apply `sanitize_line_segment` at report/TUI render sites) and R2 is a one-line predicate gate or backoff — both natural cycle-8 P1/P2 items beside the already-filed F3; R3 folds into P3-4. The fix-pass record's "renders alone never respawn" sentence should be corrected when R2 is addressed.
