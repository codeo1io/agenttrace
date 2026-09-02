# Adversarial repository assessment — 2026-09-02 (seventh pass)

- **Scope:** repository at `93aaf05` ("fix: harden untrusted-input handling across parsing,
  accounting, and durability", maintenance cycles 1–3). Working tree carries only the two
  known untracked stewardship files. Baseline re-verified this pass:
  `cargo test --workspace` → **180 passed / 0 failed**; `cargo clippy --workspace --all-targets`
  → 0 warnings; `cargo fmt --check` → clean.
- **Relation to prior passes:** nothing re-derives F1–F19 (pass 1), N1–N10 (pass 2),
  P3-1…P3-9 (pass 3), P4-1…P4-8 (pass 4), P5-1…P5-6 (pass 5), P6-1…P6-4 (pass 6), or the
  cycle-2/cycle-3 independent-review findings. Known-open items are listed separately with
  fresh verification only.
- **Method:** ce-code-review adversarial methodology applied in-thread (no subagent dispatch
  surface in this harness; no independent-corroboration claims). Fresh runtime probing on
  the release binary: malformed/hostile session corpora (lone surrogates, BOM, string-typed
  usage, `1e400`, control chars, 5000-deep nesting, NUL bytes), 20 MB / 40k-line file,
  400-file directory scan (+ non-UTF-8 file + directory named `*.jsonl`), CLI filter
  fuzzing, HTML injection attempts, doctor/pricing/baseline actions, TUI non-tty path.
  Reproducers live in `/tmp/assess7/`.

---

## P7-1 — MEDIUM (correctness, silent data loss): generic-JSONL fallback drops recoverable lines; whole-file rejection when every line is recoverable-only

The format-detector chain parses each line through the **lenient** pipeline
(`jsonl_objects` → `repair_lone_surrogates` → `number_as_i64` coercion), but the final
generic fallback does not:

```rust
// crates/agenttrace-core/src/lib.rs:382-383 (parse_jsonl_session)
let Ok(value) = serde_json::from_str::<Value>(line) else {
    continue;                                  // strict parse; no repair path
};
// crates/agenttrace-core/src/lib.rs:393-395
let Ok(mut event) = serde_json::from_value::<Event>(value) else {
    continue;                                  // typed Event; usage is BTreeMap<String, i64> (lib.rs:134)
};
```

Consequences, all reproduced on the release binary this pass:

1. **Whole-file rejection.** A plain `{"role":"user","content":"\ud800 lone",...}` file
   (every line recoverable by the existing repair) → `Error: unsupported session format`
   (exit 1). Same file in Claude-Code shape parses fine via the lenient detector — the
   inconsistency is purely which branch runs.
2. **Silent message/token under-count.** One good line + one lone-surrogate line →
   `--latest` reports `Messages: 1 user` for a 2-line file, with no skipped-line signal
   anywhere in the report (DataHealth counts files, not lines).
3. **Typed-value drop.** `"usage":{"input_tokens":"123"}` (string, which
   `number_as_i64` accepts on detector paths) fails `BTreeMap<String, i64>` → the entire
   event (message content included) is discarded.

This extends cycle-3-review I6 (info-level, lone surrogates only) with the Event-typed
drop and the whole-file rejection, and matters because the generic path is the one
unrecognized-but-JSONL tools fall into — silent under-counting of tokens/cost there
undercuts the "trustworthy local evidence" claim.

**Fix:** route the generic fallback through the same lenient line parser and
`number_as_i64` coercion the detectors use, and surface a skipped-line count in
`DataHealth` so partial parses are visible instead of silent.

## P7-2 — MEDIUM (robustness, Windows-first DX): no UTF-8 BOM stripping anywhere in the parse entry path

`parse_file` reads the file raw (`parser.rs:22-24`) and `parse_raw_session`
(`parser.rs:63`) never strips a leading `\uFEFF`; grep confirms no BOM handling in any
crate. A UTF-8-BOM-prefixed session file (what Windows Notepad and several PowerShell/WSL
round-trips produce — and this project ships `install.ps1` + winget manifests) fails the
strict per-line parse of **every** detector, including the lenient ones, because the BOM
glues to the first `{`:

```console
$ printf '\xef\xbb\xbf{"role":"user","content":"bom","timestamp":"2026-01-01T00:00:00Z"}\n' > t10.jsonl
$ target/release/agenttrace --latest t10.jsonl
Error: unsupported session format: t10.jsonl     # exit 1 — file is otherwise valid
```

Pass 3 mentioned byte-order marks only as a wishlist corpus item (pass-3 appendix,
"byte-order marks … × every subcommand"); it was never filed as a defect. **Fix:** strip
one leading `\uFEFF` in `parse_raw_session` (3-byte check, no allocation).

## P7-3 — MEDIUM (CI contract): `--baseline` regression thresholds cannot fail CI — the documented baseline step is always green

`add_baseline_comparison` turns the threshold flags into booleans only
(`reports.rs:672-677`: `slower_than_baseline`, `cost_above_threshold`,
`tokens_above_threshold`), and the CLI's gate path evaluates only health/failure-rate
gates (`main.rs:388-410`), with `exit(2)` at `main.rs:421`. So the thresholds the
CI guide advertises have no exit-code effect:

```console
$ agenttrace --overview -f json --baseline small.json --baseline-max-duration-delta-pct 0 -o cmp.json big.jsonl
exit 0            # cmp.json: "slower_than_baseline": true
```

`docs/guides/ci-integration.md:116-124` presents "Compare against local baseline" as a CI
step whose `run:` block depends entirely on the command's exit status — identical shape to
the health-gate step above it (`--fail-under-health` etc., which does exit 2). Every
adoption of that snippet silently stays green on duration/cost/token regressions and on
every `new_*` surface flag. **Fix:** mirror the health gate — exit 2 when any
`*_above_threshold` boolean is true (or add an explicit `--fail-on-baseline-regression`).
Until then, the guide should say the step is evidence-only and cannot block a merge.

## P7-4 — LOW (perf, API contract): the `since` parameter is dead in SQLite ingestion

`load_hermes_sqlite_sessions` / `load_opencode_sqlite_sessions` accept `since` but always
forward `None` to the query layer:

```rust
// crates/agenttrace-core/src/sqlite_sessions.rs:164
let sessions = query_hermes_sqlite_sessions(path, None);
// crates/agenttrace-core/src/sqlite_sessions.rs:232
let sessions = query_opencode_sqlite_sessions(path, None);
```

The parameterized SQL binding built from it (`:187-188`) is therefore unreachable;
results are filtered in memory by `filter_since` after a full-table scan. Output is
correct; cost is unnecessary work on large `opencode.db`/`state.db` files during
auto-discovery (which the TUI re-runs every 10 s), and the signature promises a push-down
that does not exist. **Fix:** pass `since` through (it is already plumbed at the
call-site level), or delete the parameter.

## P7-5 — LOW (durability): `pricing.json` and `history.json` are written non-atomically

Both use plain `std::fs::write` (`pricing.rs:329-336`, `history.rs:36-41`), unlike the
cycle-3 atomic pattern (`unique_temp_path` + rename, `session_cache.rs:226-236` /
`:535`). A crash mid-write tears the file. Pricing self-heals (parse failure falls back
to the bundled snapshot, so the cost is one re-download), but a torn `history.json` is
**silently discarded**: `decode_records` returns empty on any parse failure
(`history.rs:56-60`), so all previously preserved derived sessions are lost on the next
run. **Fix:** reuse `unique_temp_path` + rename for both writes.

---

## Prior findings — verified status (no re-derivation)

- **P3-4 (control chars in output) — still open, by campaign decision.** Fresh repro:
  session name containing `ESC ]0;pwned BEL` and an OSC-52 clipboard sequence is emitted
  verbatim in the `--sessions` row (`lib.rs` `display_title_from_text` → text renderer).
  Deferred to the planned "output honesty" cycle
  (`docs/stewardship/2026-09-02-cycle3-prioritization.md:128`), so listed here for status
  only.
- **F9 / P5-6 (installers verify no checksums) — still open.** `install.sh:44-58`
  downloads over HTTPS and applies only a ≥1 MB size floor; the release workflow publishes
  `checksums.txt` and per-asset `.sha256` that installers never read.
- **Cycle-3 independent-review residuals — all still open at `93aaf05`:**
  (a) CU-5's naming-semantics change did not bump `SQLITE_SNAPSHOT_SCHEMA_VERSION`
  (`session_cache.rs:9`, still 5), so warm pre-CU-5 snapshots keep serving placeholder
  names; (b) `repair_lone_surrogates` (`parser.rs:3796`) has no escaped-backslash
  lookbehind and can rewrite literal `\uXXXX` text on already-failing lines (the current
  byte-scan still treats `\`+`u` after an escaped backslash as an escape);
  (c) per-writer temp suffixes (`session_cache.rs:237`) are never swept, so crashed
  writers leak `*.tmp.<pid>.<seq>` orphans. None was addressed by `93aaf05`
  (`git show 93aaf05` touches none of these).
- **P6-1 (surrogate-repair panic) — fixed and re-verified.** This pass's hostile corpus
  (lone/paired surrogates, `\uzzzz`, truncated escapes, 5000-deep nesting, `1e400`, NUL
  and C0 controls, non-UTF-8 bytes) produced zero panics on the release binary; the
  all-corrupt file degrades to a clean `Error: unsupported session format` (see P7-1 for
  why that message is still misleading).

## Surfaces probed clean this pass (fresh evidence, not repeats)

- **Performance:** 20 MB / 40k-line session → `--latest` in 0.81 s; 400-file directory
  scan + overview JSON in 0.33 s; non-UTF-8 file and a directory named `*.jsonl` inside
  the scan root are both skipped safely (exit 0).
- **CLI validation:** `--health/--cost/--sort/--order/--range/--search-limit` reject
  invalid values with specific errors; `--sessions --inspect N` mutual exclusion
  enforced; `--baseline` requires `--overview -f json`; `--version` precedence honored in
  both orders. (Caveat: flags placed **after** the positional path are silently dropped —
  intentional Go-flag compatibility, `main.rs:481` + pinned test, but invalid values then
  go unvalidated; a one-line README note would cheaply remove the trap.)
- **HTML injection:** raw `<script>`/`<img onerror>` content from a session file → 0
  unescaped occurrences in `--overview -f html` and `--audit -f html` output.
- **Subprocess safety:** `git -C <untrusted-root> log --format=%ct`
  (`governance.rs:759-768`) uses an arg array with no shell and `-C` consuming the root as
  a value, so a hostile `cwd` cannot inject flags; it is spawned only when the governance
  Delivery panel opens, not on the 10 s auto-refresh.
- **TUI:** non-tty invocation exits 1 with the cycle-2 guidance error; governance
  background-thread receiver is correctly restored on `TryRecvError::Empty`
  (`app.rs:1548-1551`); `expect("governance initialized")` sites (`app.rs:1496-1517`) are
  guarded by `get_or_insert_with` immediately above.
- **Arithmetic:** spot-audit of `insights.rs`, `waste.rs`, `governance.rs`,
  `diagnostics.rs`, `reports.rs` found saturating ops and zero-denominator guards at
  every division site checked; `context_utilization`'s `total` is a constant ≥128 000 so
  its `%` can't divide by zero.
- **Docs/DX:** `scripts/ci/check-docs-commands.sh` green on the release binary; every
  `--flag` in README resolves to a real CLI flag (the four apparent misses are
  winget/cargo arguments, not agenttrace flags).
- **Workflow hygiene:** GitHub Actions pinned by full SHA, `permissions` blocks present,
  release secrets gated with `test -n` guards; npm launcher spawns the platform binary
  with inherited stdio and maps signal-kill to exit 1.

## Assessment side effects (disclosure)

- `--doctor` refreshed the operator session cache (`/home/agent/.cache/agenttrace/
  sessions.json`) — normal app behavior on this machine.
- `--update-pricing` (run to verify the offline-default claim's counterpart) fetched
  LiteLLM data and overwrote `/home/agent/.cache/agenttrace/pricing.json` with a fresh
  dated snapshot — the flag's documented effect, noted for the record.
- No repository files were modified; nothing staged, committed, pushed, or PR'd.
