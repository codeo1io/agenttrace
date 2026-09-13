# Cycle 7 review-fix record

- **Date**: 2026-09-14
- **Run/phase**: 125bf93302aa4e308cb0739b67f16f33 / `independent_review:fix` (attempt ea1d5f3585724b229d52f9b867db0633)
- **Input**: `docs/reviews/2026-09-14-cycle7-independent-review.md` (verdict PASS WITH FINDINGS; F1–F7)

## Dispositions

### F1 (Medium) — status line rendered raw control characters — FIXED

`crates/agenttrace-core/src/statusline.rs`: `render_status_line` now passes
`session_name` and `model.display_name` through `sanitize_line_segment`,
which replaces every control character (`char::is_control` — exactly C0
U+0000–U+001F, DEL U+007F, C1 U+0080–U+009F) with U+FFFD. Newlines can no
longer break the one-line host contract and ESC/OSC/CSI sequences can no
longer reach the host terminal. Numeric segments are formatted values and
were already safe.

- Test: `hostile_payload_names_render_without_control_characters`
  (statusline.rs) uses the review's exact hostile payload (`\n`, `\r`,
  `ESC[31m`, `ESC]777;`, BEL, and a C1 CSI in the fallback path) and
  asserts one line, zero control characters, visible U+FFFD, unaffected
  `ctx 10%`.
- Guide contract bullets extended (`docs/guides/statusline-capture.md`).
- CHANGELOG: Fixed entry added.

### F2 (Medium) — stdout write failure panicked (exit 101) — FIXED

`run_statusline_host` now writes the line with
`let _ = writeln!(io::stdout().lock(), "{line}")` and the two stdin-diagnostic
and capture-failure messages use the same write-and-ignore form on stderr, so
no stdout/stderr condition can panic the host. Exit 0 holds for `/dev/full`
(ENOSPC on every write) and for EPIPE.

- Test: `statusline_host_mode_survives_stdout_write_failure`
  (entrypoints.rs) spawns the real binary with stdout redirected to
  `/dev/full`, asserts exit 0 and no panic text on stderr.
- Guide contract bullets extended; CHANGELOG Fixed entry added.

### F4 (Low) — delivery retry kept the stale error — FIXED (and extended)

The review's repro showed the stale `delivery_error` surviving the retry
spawn. Fixing it revealed the deeper half of the same finding: after a
success→failure transition the stale `delivery: Some` made
`ensure_governance`'s `missing` predicate false, so the documented retry
(re-entering the panel) **never respawned the worker at all** — it just
un-hid the old error. The fix therefore lives in `open_governance`
(`crates/agenttrace-tui/src/app.rs`): entering the Delivery panel with a set
error clears the error *and* drops the pre-failure evidence it was hiding, so
the next render spawns a fresh worker and shows the scan in flight;
error-free successful states keep their cache, and renders alone never
respawn (no worker spam). The spawn arm also clears `delivery_error` as
belt-and-braces.

- Test: the delivery test gained a Case 2b driving the real retry
  (`run_command("delivery")` + draw), asserting the stale error is cleared,
  a fresh worker is pending, the old error text is gone from the rendered
  buffer, and the real worker settles within a deadline.
- CHANGELOG: the A11-4 Fixed entry was extended with the retry semantics.

### F3 (Low) — concurrent compaction can drop a concurrent capture — FILED

Deferred to cycle 8 exactly as the review recommends. Filed in the ROADMAP
hardening lane as **Statusline journal concurrent-append race** with
acceptance criteria (lock or tail re-apply + concurrency test + two-process
live probe).

### F5 (Info) — symlink descent unbounded in I/O — FILED

Filed as **Discovery walk I/O budget for symlink descent** (per-walk budget,
named diagnostic, synthetic wide-tree test).

### F6 (Info) — installer checksum threat-model boundary — DOCUMENTED

The one docs sentence the review asked for was added to
`docs/maintainers/distribution.md`: the sidecar check protects against
corrupted mirrors and asset drift, not a compromised release origin (asset
and sidecar share one origin and channel).

### F7 (Info) — doctor prints machine-local absolute paths — FILED

Filed as **Doctor output portability for path pinning**: a normalize-vs-
exclude decision is required before any doctor line is pinned by
`check-deterministic-output.sh`.

## Validation after the fixes

- `cargo test --workspace` → 232/232 (230 + two new tests — the
  hostile-fixture sanitize test and the `/dev/full` survival test —
  plus the retry-reset case extending the existing delivery test).
- `cargo clippy --workspace --all-targets` → 0 warnings;
  `cargo fmt --all --check` → clean.
- Release rebuild; all 10 `scripts/ci/check-*.sh` → rc=0 (TUI real smoke
  with `AGENTTRACE_TUI_REAL_DIR=/tmp/symprobe`).
- Live re-derivations of both review repros: the review's hostile payload
  now renders one line with U+FFFD replacements and no control bytes; the
  same payload against `/dev/full` exits 0 with no panic on stderr.
- ROADMAP hardening lane carries F3/F5/F7 under review IDs; the closing
  bookkeeping records the review verdict and the same-day dispositions.
