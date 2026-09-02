# Adversarial repository assessment — 2026-09-02 (sixth pass)

- **Scope:** repository at `e005952` plus the uncommitted cycle-1/cycle-2 working tree
  (20 modified + 11 untracked entries). Verified before starting: `git status` matches the
  tree the cycle-2 implementation record describes; `cargo test --workspace` → **169 passed /
  0 failed**; `cargo clippy --workspace --all-targets` → 0 warnings.
- **Relation to prior passes:** nothing re-derives F1–F19 (pass 1), N1–N10 (pass 2),
  P3-1…P3-9 (pass 3), P4-1…P4-8 (pass 4) or P5-1…P5-6 (pass 5). Status of prior findings was
  spot-checked only where this pass's testing crossed them (see "Prior-finding status").
- **Method:** no `ce-*` SKILL.md is installed in this delegate environment, so the
  ce-code-review adversarial methodology was applied in-thread, as prior phases recorded.
  Reproducers live in `/tmp/assess-repro/`; both debug and release binaries were exercised.

---

## P6-1 — HIGH (correctness, reliability): `repair_lone_surrogates` panics on non-char-boundary slicing — release binary, all report actions, from format detection

`repair_lone_surrogates` walks the line **by byte index** but slices the original `&str`:

```rust
// crates/agenttrace-core/src/parser.rs:3783-3791
while i < bytes.len() {
    if bytes[i] == b'\\' && bytes.get(i + 1) == Some(&b'u') && i + 6 <= bytes.len() {
        let hex = &line[i + 2..i + 6];                       // :3785 — panics here
        ...
            && u16::from_str_radix(&line[i + 8..i + 12], 16) // :3791 — and here
```

The `i + 6 <= bytes.len()` guard bounds the slice *length* but not its *char boundaries*.
When a `\u` that serde_json rejects is followed by multi-byte UTF-8 inside the next four
bytes, `&line[i+2..i+6]` slices mid-character → `panicked at ... end byte index N is not a
char boundary` → **exit 101**. Slicing panics are profile-independent, so the shipped
release build crashes too (verified below).

Reachability is broad: `parse_jsonl_value_lenient` (parser.rs:3772) is the fallback for
**every** line that fails strict JSON parse, and `jsonl_objects` (parser.rs:3762) feeds all
the JSONL format detectors (parser.rs:156, 251, 362, 423, 470 …), so the panic fires during
*format detection*, before the file is even accepted.

[REPRODUCED] (release binary, `target/release/agenttrace`, built this pass):

```console
$ printf '{"prompt":"\\u中文测试"}\n' > corrupt.jsonl
$ ./target/release/agenttrace corrupt.jsonl --overview   # exit 101
thread 'main' panicked at crates/agenttrace-core/src/parser.rs:3785:28:
end byte index 17 is not a char boundary; it is inside '文' (bytes 16..19 of string)

$ printf '{"prompt":"\\ud800\\u中文测试"}\n' > corrupt2.jsonl
$ ./target/release/agenttrace corrupt2.jsonl --overview   # exit 101
thread 'main' panicked at crates/agenttrace-core/src/parser.rs:3791:53:   # second slice
end byte index 23 is not a char boundary; it is inside '文' (bytes 22..25 of string)

$ ./target/release/agenttrace -d /tmp/assess-repro --overview             # exit 101 — directory scans die too
```

Blast radius (release binary, same 1-line file): `--doctor`, `--waste`, `--latest`,
`--sessions`, `--diagnostics`, `--overview`, `-d <dir>` all **exit 101** with no report.
The TUI survives (the load runs on a spawned thread; the channel disconnect surfaces as a
"reload failed: loader disconnected" status) but every non-interactive surface dies.

**Why every net missed it.** The committed adversarial corpus
(`testdata/generated/adversarial/*.jsonl`) contains **no** `\u` escape at all; the only
surrogate case anywhere in tests is `discovery_contract.rs:1527`'s `"lenient surrogate
\ud83c result"` — `\ud83c` followed by an **ASCII space**. The cycle-2 record's
"JSONL mutation harness → 0 panics" was therefore true but vacuous for this input class:
no harness ever placed a multi-byte character within four bytes after a `\u`.

**Fix:** parse the hex from `bytes` (e.g. collect `bytes[i+2..i+6]` into a `[u8; 4]` when
all four are ASCII hex, else fall through unchanged), or `line.is_char_boundary()`-check
both slice edges before slicing; then add `{"prompt":"\u中文测试"}` to the generated corpus
and a contract test that the file yields a (repaired or skipped) line instead of a panic.

---

## P6-2 — MEDIUM→LOW (docs accuracy, DX): CHANGELOG still over-claims `--version` precedence; `--overview --version` is rejected

`CHANGELOG.md:11` states: *"`--version` now wins over argument validation"*. CU-4 only
moved the early return above `report_language` (`main.rs:150` vs `main.rs:155`) —
`validate_primary_action(&args)?` still runs first at `main.rs:136`, and `--version` is
itself a member of the action list (`main.rs:951`), so combining it with any report action
errors before the banner can print:

```console
$ ./target/debug/agenttrace --lang fr --version      # exit 0 — the documented case works
agenttrace v0.0.0-dev
$ ./target/debug/agenttrace --overview --version     # exit 1
Error: choose exactly one report action
$ ./target/debug/agenttrace --demo --doctor --version # exit 1 — same
```

The guard test (`launch_guards.rs`, `version_wins_over_invalid_lang`) covers only the
`--lang` form. Either hoist the `--version` early return above `validate_primary_action`
(conventional CLI behavior) or narrow the CHANGELOG sentence to the `--lang` case.

---

## P6-3 — LOW (reliability, concurrency): cache persist uses a fixed `*.tmp` sibling name shared by concurrent processes

Both cache writers write a **fixed** temporary path next to the target and rename it:

- `session_cache.rs:226` — `let tmp = path.with_extension("json.tmp");`
- `session_cache.rs:518` — `format!("{}.tmp", …file_name…)` → `sessions.json.tmp`

Two concurrent agenttrace processes (the TUI's background auto-refresh thread + a cron'd
CLI, or two TUIs on different `-d` dirs sharing the default cache) open/truncate/write the
same tmp file, and the interleaved `fs::rename` can install a truncated document. Impact is
bounded — `load_cached_sessions_from_cache`/`load_sqlite_snapshot_from` fail parse and fall
back to a cold scan (self-healing), so reports stay correct — but the cache is silently
lost and last-writer-wins also discards the other process's entries. Use a unique tmp name
(`.tmp.<pid>.<nonce>`) like `pricing.rs:1198` already does for its test helper.

## P6-4 — LOW (accuracy): fallback token estimate divides byte length by 4; CJK under-counted, `reasoning_chars` is bytes

The fallback estimator (used only when no usage block is present) is
`std::cmp::max(1, event.content.len() as i64 / 4)` at `lib.rs:555` and `lib.rs:577` —
**byte** length / 4. CJK text is ~3 bytes/char but ≥1 token/char under real tokenizers, so
`"中文测试"` (12 bytes) estimates 3 tokens vs ~4–6 actual — a ~40–60% under-count, i.e. the
estimate is systematically wrong in exactly the corpus this project's lenient-surrogate
work targets. Related mislabel at `lib.rs:562`: `let chars = event.reasoning.len();`
stores **bytes** into `metrics.reasoning_chars` (and `avg_reason` divides by it). Consider
`chars().count()` for the reported `reasoning_chars` and a CJK-aware divisor for the
fallback estimate, or document the heuristic's error band for non-ASCII.

---

## Prior-finding status (spot checks only, not re-derivations)

- **P5-1/P5-2 (SQLite overflow / wrap)** — fixed in this tree as claimed:
  `saturating_add` accumulators and `parser::number_as_i64` routing present
  (`sqlite_sessions.rs` agg sites), fixtures under `testdata/generated/adversarial/sqlite/`.
- **P4-1 (TUI panic on non-TTY)** — fixed and re-verified this pass: piped `agenttrace`
  exits 1 with the `--overview` guidance (`app.rs:78` guard).
- **N5/P4-5 (TUI real-smoke step gated on unset `AGENTTRACE_TUI_REAL_DIR`)** — still open,
  unchanged (`.github/workflows/ci.yml:84`); not re-tested.
- **P5-4/P5-6** — not re-checked this pass.

## Negative results (probed, clean)

- HTML report: `html_escape` applied at every interpolation point checked
  (`reports.rs:570-1173`), including timeline/anomaly/session tables; JSON builders route
  strings through `json_string`/serde (`reports.rs:1371-1381`).
- SQLite access is `SQLITE_OPEN_READ_ONLY` + parameterized queries; the only `format!`
  SQL is `pragma table_info({table})` with a hardcoded table name (`sqlite_sessions.rs:716`).
- `git_commits` (`governance.rs:760`) uses `Command::new("git").args(["-C", root, …])` —
  no shell, arguments-only; no injection vector found.
- BrokenPipe is handled (`main.rs:425`), so `agenttrace … | head` doesn't error.
- Perf sanity: 400 synthetic sessions → cold `--overview -f json` 0.37 s, warm 0.25 s;
  per-session `resolve_project` walks are bounded filesystem stats, not a hotspot.
- Baseline: 169/169 workspace tests green, clippy `--all-targets` clean, CI workflow pins
  third-party action SHAs, `check-plugin-version` keeps plugin/CHANGELOG at v0.7.1.

## Recommended order

1. **P6-1** — byte-based hex parse in `repair_lone_surrogates` + corpus/test additions
   (a two-function fix; closes a release-binary crash reachable from any `.jsonl` input).
2. **P6-2** — one-line either-way fix (hoist `--version` return or reword CHANGELOG:11).
3. **P6-3, P6-4** — unique tmp suffix; byte-vs-char accounting decision.
