# Cycle 7 independent review — trustworthy capture

- **Date**: 2026-09-14
- **Run/phase**: 125bf93302aa4e308cb0739b67f16f33 / `independent_review` (attempt 49357ebd1b57477a80ab933f0a563391)
- **Review target**: uncommitted cycle-7 worktree at HEAD `df3b621` (21 modified + 8 untracked paths), i.e. the implement + targeted_tests + compound output, including the ROADMAP/CHANGELOG/docs claims.
- **Reviewer inputs**: `docs/decisions/2026-09-14-cycle-7-batch-selection.md` (goals/DoD), `docs/stewardship/2026-09-14-cycle7-implementation-record.md`, ROADMAP.md cycle-7 closures, and the code itself. All evidence below was re-derived live by this review; nothing was taken on faith from the implementation record.

## Verdict

**PASS WITH FINDINGS.** All six core items and both ride-alongs are present, behave as recorded, and meet their roadmap acceptance criteria; the batch-selection DoD (baseline green, CHANGELOG entries, CI recipe runs as written, closures recorded) holds. Verification claims independently reproduced: `cargo test --workspace` 230/230, clippy 0 warnings, fmt clean, all 10 `scripts/ci/check-*.sh` rc=0. Two Medium findings falsify specific sentences of the new statusline surface's shipped contract ("exactly one line", "exits 0 for any input") and should be fixed before or as the first act of cycle 8; neither invalidates the cycle's data-correctness goals (items 1, 2, 4 verified correct). Disclosures in the ROADMAP/record checked out under adversarial reading: P4-2 honestly recorded not landed (ROADMAP.md:779-782), R2 run-level evidence honestly deferred (:975-990), statusline fixtures honestly labeled schema-faithful not host-recordings (:1645-1652), pricing count discrepancy (2,755 vs pass-9's 3,923) explained in place (:1613-1616).

## Findings

### F1 — Medium — status line renders raw control characters (contract + terminal injection)

`crates/agenttrace-core/src/statusline.rs:186` (`parts.push(name)`; same class at `:177-180` for `model.display_name`).

`render_status_line` interpolates `session_name` (a user-typed chat title in Claude Code) and `model.display_name` into the status line without stripping control characters. Live reproduction with `AGENTTRACE_SESSION_CACHE_DIR` isolated:

```
printf '{"session_name":"evil\\nsecond\\rcarriage\\u001b[31mRED\\u001b]777;pwned\\u0007","model":{"display_name":"M"},"context_window":{"used_percentage":10}}' | agenttrace statusline
→ output bytes: e v i l \n s e c o n d \r c a r r i a g e 033 [ 3 1 m R E D 033 ] 7 7 7 ; p w n e d \a | c t x 1 0 %
```

This (a) breaks the module's own host contract "prints exactly one line to stdout" (`statusline.rs:132-134`, `docs/guides/statusline-capture.md` "Host contract", CHANGELOG Added entry) — a `\n` in the name yields two lines; and (b) passes ESC/OSC/CSI sequences to the host terminal (title-setting, SGR, and worse). The one-line test at `statusline.rs:825` only asserts the benign fixture. The repo already carries this class as an open item (pass-3 P3-4 control-character filter, ROADMAP CLI-surface polish); the new surface should have adopted it. The same unfiltered-payload-string class reaches `--statusline-report` text (miss-cause map keys, session ids) and the TUI Efficiency lines (`presentation.rs:1299+`), though those surfaces are pre-existing style. Recommended fix: strip/replace C0/C1 control chars (and DEL) in `render_status_line` inputs, keep the one-line assertion on a hostile fixture.

### F2 — Medium — stdout write failure panics, violating "never fails the host / exits 0"

`crates/agenttrace-core/src/statusline.rs:159` (`println!("{line}");`).

`println!` panics on write error. Live reproduction: `echo '{"session_name":"x"}' | agenttrace statusline > /dev/full` → `thread 'main' panicked ... failed printing to stdout: No space left on device (os error 28)`, **exit 101**. The same holds for EPIPE when the host closes the pipe. This falsifies the contract asserted at `statusline.rs:135-137` ("never fails … exits 0"), the guide ("exits 0 for any input … never a non-zero exit"), the CHANGELOG, and the entrypoints test's coverage claim (that test drives stdin shapes, not stdout failure). Recommended fix: `let _ = writeln!(io::stdout(), "{line}");` and return `Ok(())` regardless.

### F3 — Low — concurrent compaction can silently drop a concurrent capture

`crates/agenttrace-core/src/statusline.rs:275-297` (`compact_statusline_capture_under`) with `:246-264` (`append_statusline_capture`).

Two `agenttrace statusline` processes (multiple Claude Code sessions/panes is the normal case) racing across the 10 MiB bound: a line appended by process B between A's `read_to_string` and `fs::rename` is lost — the rename replaces the journal without B's line. Bounded (≤1 line per race), never corrupts (temp+rename is crash-safe), but the loss is silent and undisclosed; the record claims only crash-safety. Recommended fix for cycle 8: hold an flock on the journal during append+compact, or re-apply the tail after rename.

### F4 — Low — delivery retry keeps the stale error while the retry is in flight

`crates/agenttrace-tui/src/app.rs:1519-1541` (Delivery arm does not clear `delivery_error` when re-spawning the worker) with `crates/agenttrace-tui/src/presentation.rs:1057-1060` (status checks `error` before `delivery_pending`) and `:1134-1147` (body prefers the error over the loading text).

After a worker failure, reopening the panel (the documented retry, per the panel's own hint) spawns a new worker but leaves `delivery_error` set until the next successful poll, so the panel shows "Evidence unavailable" + the old message instead of "Scanning Git roots…" while the retry runs. Cosmetic/diagnostic accuracy on the A11-4 item itself; clear `delivery_error = None` when re-spawning (or check pending before error).

### F5 — Info — symlink descent is unbounded in I/O, not in correctness

`crates/agenttrace-core/src/discovery.rs:385-397` (`entry_is_dir_entry` follows any symlink-to-directory at any walk depth).

Correctness verified: symlinked children discovered, self/parent cycles terminate (live probe: 2 files, self-loop, 73 ms-class; contract test), duplicate canonical targets admitted once, doctor annotates `(symlink -> target)`. Residual: a session root containing a symlink to `/` or a huge tree is now walked (previously skipped), bounded only by `max_session_dir_depth` and the canonical-visit set — no per-walk entry/I/O budget. Matches the acceptance criteria as written; note for a future resource-bound hardening item.

### F6 — Info — installer checksum is corruption/mirror parity, not origin compromise

`install.sh:56-85`.

Verified logic: hard-fail on mismatch (expected/actual printed, nothing installed), warn-and-proceed on absent sidecar or missing tool, `chmod 0755` before move (umask-independent). Threat-model boundary worth one docs sentence: sidecar and asset share origin and channel, and the absent-sidecar path downgrades to a warning — so this protects against corrupted mirrors and drift, not a compromised release origin. Matches A11-3's scope ("the way npm's installer already does").

### F7 — Info — doctor output includes machine-local absolute paths (pre-existing class, new instance)

`crates/agenttrace-core/src/doctor.rs` statusline section (e.g. `Statusline capture: /home/<user>/.cache/agenttrace/statusline.jsonl`), including `--doctor --demo`. Pre-existing behavior for the cache path; the checkers only require valid JSON (`check-output-contract.sh:26-28`) and the deterministic-output pins don't cover doctor, so no regression — but portability of any future doctor-output pinning should account for it.

## What was verified (independently, this review)

- **Suite**: `cargo test --workspace` → 230 passed / 0 failed (14+8+2+87+7+70+42, doc-tests 0); `cargo clippy --workspace --all-targets` → 0 warnings; `cargo fmt --all --check` → clean; all 10 `scripts/ci/check-*.sh` → rc=0 (logs `/tmp/ir7-chk-*.log`).
- **Item 1 (shim)**: `flag_takes_value` no longer lists `--no-baseline-gate` (`main.rs:743` context); live: `--no-baseline-gate --baseline X --overview -f json <dir>` now parses past the flags (error is the truthful "positional path must be a session file", not the old misleading gate error). Unit + entrypoints tests present (`entrypoints.rs:315`).
- **Item 2 (symlinks)**: live HOME-isolated probe — symlinked child discovered, top-level symlinked provider root annotated `Codex CLI ... (symlink -> /tmp/.../actual)`, self-loop terminated; `dir_listing_version: 2` present in the real cache after upgrade runs (invalidation landed); unit/contract tests present; cached-walk replay also admits-guarded (`discovery.rs` walk_session_files_cached listing branch).
- **Item 3 (statusline)**: valid payload → one line, rc 0; invalid JSON / invalid UTF-8 / empty stdin / >1 MiB truncation → fallback line + stderr diagnostic, rc 0; journal honors `AGENTTRACE_SESSION_CACHE_DIR`; `--statusline-report` text + JSON valid; dedup disclosure rendered; torn-tail test real; no stray `statusline.jsonl` in the real user cache (test-hygiene claim held).
- **Item 4 (cache bounds)**: `cache_paths_sized_once` union sizing, both bounds one-pass, headerless→`i64::MIN` (F5-5) present in the worktree **and** semantically matched against the canonical checkout's uncommitted F5-5 diff (`git -C /work/projects/agenttrace diff` reviewed: `map_or(i64::MIN, …)` in both bounds + tests) — the "subsumed, not reverted" claim is verified, de-risking the commit-gate reconciliation.
- **Item 5 (installer)**: code-reviewed (above); implement-phase live harness evidence consistent with the script's logic.
- **Item 6 + TUI**: `Result` channel, `catch_unwind`, `Disconnected` → named error, success clears (F4 nuance aside); test covers Err/disconnect/diagnostic/retry-reset.
- **R1**: doctor line renders "LiteLLM snapshot 2026-09-13 (bundled, 2755 models, 0 days old)" live; `PRICING_SNAPSHOT_DATE` updated in lockstep (drift-pin test in suite).
- **R2**: both YAMLs parse; `ci.yml` gate now `vars.`-based with env forwarding; fork gate + complement note step; deferral honestly recorded.
- **Compound artifacts**: ROADMAP cycle-7 header, six "Closed in cycle 7" entries, two status notes, and the pre-review compound paragraph read against the code — claims match behavior except where F1/F2 falsify the statusline contract sentences; `docs/maintainers/test-flake-prevention.md` rules match the shipped `test_env::lock_env()` (poison-tolerant) and thread-keyed cfg(test) paths (`app.rs:1597-1607`); README/CHANGELOG/docs-README cross-references consistent; `check-docs-commands.sh` green over the new commands.
- **Non-finding cleared**: `discovery_contract.rs` greps as binary due to an intentional `b"SQLite format 3\0..."` fixture — not corruption.

## Recommended disposition

Ship-blocking: none for the six core correctness items. Fix F1 and F2 (both small, both contradict shipped contract sentences in CHANGELOG/guide) before the commit gate if convenient, else as cycle-8 P1s alongside P4-2, which is the same CLI-surface lane. F3/F4 are quality follow-ups; F5/F6/F7 are notes for the roadmap's hardening lane.
