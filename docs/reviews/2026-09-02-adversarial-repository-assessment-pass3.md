# Adversarial repository assessment — 2026-09-02 (third pass)

- **Scope:** whole repository at `e005952` plus the uncommitted cycle-1 working tree — the same tree pass 2
  reviewed, re-attacked from zero with different methods (differential/contradiction hunting, environment
  manipulation, timezone manipulation, corpus fuzzing, timing).
- **Relation to prior passes:** nothing below re-derives
  `docs/reviews/2026-09-02-adversarial-repository-assessment.md` (F1–F19) or
  `...-assessment-pass2.md` (N1–N10). Prior findings appear only in the status table at the end, and only
  where I re-verified them. Everything in "Actionable findings" is new.
- **Baseline:** `cargo test --workspace` → 159 passed / 0 failed; `cargo clippy --workspace --all-targets`
  → 0 warnings; `cargo fmt --all -- --check` clean. Debug binary used throughout:
  `target/debug/agenttrace`.
- **Skill routing:** the work order names a compound-engineering router; I located it at
  `~/.hermes/skills/software-development/compound-engineering/` and ran its `ce-code-review` context fence.
  That skill's mode contract expects findings-mechanics + multi-lens dispatch; this delegate harness has no
  subagent dispatch, so **all lenses ran in-thread** — agreement between lenses is not independent
  corroboration, and finding IDs/IDs assignment is single-reviewer.
- **[REPRODUCED]** marks a finding I triggered end-to-end with a command you can re-run. Reproducers live in
  `/tmp/at3/` (`logs4` = control characters, `logs5` = percentile corpus, `tz2` = timezone corpus,
  `perf` = 1500-file corpus). `XDG_CACHE_HOME=/tmp/at3/cache` unless stated.

---

## Actionable findings

### P3-1 — HIGH (correctness, portability, DX): session auto-discovery is `HOME`-only; a default Windows install discovers nothing

`known_session_dirs()` bails out when `HOME` is unset:

```rust
// crates/agenttrace-core/src/discovery.rs:51-53
let Some(home) = std::env::var_os("HOME").map(PathBuf::from) else {
    return Vec::new();
};
```

The same pattern repeats at `sqlite_sessions.rs:47` and `sqlite_sessions.rs:61`. There is **no
`USERPROFILE` / `HOMEDRIVE`+`HOMEPATH` fallback anywhere** — `grep -rn "USERPROFILE\|cfg(windows)" crates/`
returns zero matches — and Windows does not set `HOME` by default. So on a stock Windows machine:

- `agenttrace` (TUI) starts with zero session directories;
- `agenttrace --overview` exits with `Error: No session files found in ` (note the dangling space);
- `--doctor` reports `Providers:` empty and recommends `--demo`.

Windows is a first-class target: `install.ps1` downloads `agenttrace.exe`,
`winget/` publishes `Luoyuctl.AgentTrace`, `npm/scripts/install.js` handles `win32`, and
`README.md:78-83` advertises "macOS, Linux, and Windows". F8 (pass 1) covered the *cache directory*
falling back to the temp dir; this is the larger sibling — the *discovery root itself* is unreachable.

[REPRODUCED]

```console
$ env -u HOME -u XDG_CACHE_HOME ./target/debug/agenttrace --doctor
AGENTTRACE Doctor
Version: 0.0.0-dev
Mode: auto-discovery
Session files: 0
...
Providers:

Recommendations:
  - No sessions found. Run `agenttrace --demo` to try the TUI immediately.
$ env -u HOME -u XDG_CACHE_HOME ./target/debug/agenttrace --overview
Error: No session files found in
```

**Fix:** one shared `home_dir()` helper that tries `HOME`, then `USERPROFILE`, then
`HOMEDRIVE`+`HOMEPATH` (or adopt the `dirs`/`home` crate); use it from discovery and sqlite_sessions;
add a unit test with `HOME` unset that asserts the Windows fallback chain is attempted.

---

### P3-2 — MEDIUM (correctness, i18n/time): `--range today` and its alias `--range 1d` are UTC-calendar-day windows, so they silently drop sessions from earlier in the user's local day

```rust
// crates/agenttrace-core/src/insights.rs:50
"today" | "day" | "1d" => Some(Self::Today),
// crates/agenttrace-core/src/insights.rs:65-70
Self::Today => now
    .date_naive()
    .and_hms_opt(0, 0, 0)
    .map(|value| value.and_utc()),
```

`since()` is called with `Utc::now()` (`main.rs:617` `range.since(Utc::now())`), and the TUI does the
same (`app.rs:1312` `let now = chrono::Utc::now();` feeding `session_matches_time_range` at
`app.rs:1397`), so "today" begins at **UTC** midnight, not the operator's. Two defects in one:

1. For anyone east of UTC, sessions from earlier in their own day fall before UTC midnight and vanish.
2. `1d` is an *alias* of the calendar-day window rather than a rolling 24 hours — surprising on its own,
   and it inherits the timezone bug.

`session_matches_time_range` (`insights.rs:229-233`) then filters the whole CLI view, so this also
silently narrows `--overview`, `--audit`, `--recommend`, `--mcp-governance`, `--context-trends`,
`--delivery-evidence`, `--compare` and `--baseline` CI gates — the README quickstart and
`docs/guides/ci-integration.md` all use `--range`.

[REPRODUCED] with a session stamped `2026-09-02T01:00:00+09:00` (= `2026-09-01T16:00Z`), machine clock
17:17 JST, `TZ=Asia/Tokyo`:

```console
$ agenttrace -d /tmp/at3/tz2 --sessions --range today
Error: No sessions match the requested filters
$ agenttrace -d /tmp/at3/tz2 --sessions --range 1d
Error: No sessions match the requested filters
$ agenttrace -d /tmp/at3/tz2 --sessions --range 7d   # includes it
$ agenttrace -d /tmp/at3/tz2 --sessions              # default --range all, includes it
```

**Fix:** compute `Today` as local midnight (`Local::now().date_naive()…`), and make `1d` a rolling
`now - Duration::hours(24)`. Add a test pinning a `+09:00` session against `TimeRange::Today`.

---

### P3-3 — MEDIUM (correctness): two divergent `percentile()` implementations contradict each other inside the same report

```rust
// crates/agenttrace-core/src/lib.rs:1205-1217  (used by detect_anomalies, lib.rs:676/680)
let idx = (len as f64 * percentile).floor() as usize;   // effectively trunc(len·p)
// crates/agenttrace-core/src/reports.rs:1715-1721     (used by latency reporting, reports.rs:60/62/311/313)
let idx = ((len - 1) as f64 * percentile).round() as usize;
```

Both run over the **same** `gaps` slice for one session: `detect_anomalies` decides whether to raise a
"long gaps" anomaly and what number to print in its `detail` string, while the report body prints
`latency.p95`. For any corpus where the two index rules pick different order statistics, the report
asserts two different p95s for the same session.

[REPRODUCED] — `/tmp/at3/logs5/g.jsonl`, 20 gaps = 18×1s + 29s + 31s:

```console
$ agenttrace -d /tmp/at3/logs5 --latest -f json
  "latency": { "p95": 29, "max": 31, ... }
  "anomalies": [ { ..., "detail": "p95 latency = 31.0s" } ]
```

The report body says p95 = 29 while its own anomaly says p95 = 31. Consequences: triage reads a
self-contradicting number; the anomaly's threshold test (`> 30s`) used the `round` rule, so a session
whose printed p95 is 29 is still flagged "p95 latency = 31.0s"; and `--inspect` ordering /
`--fail-on-critical` CI gates inherit whichever copy their call site happens to use.

**Fix:** delete one implementation, export the survivor once, and add a test asserting the anomaly
detail and the latency block agree on a fixed corpus.

---

### P3-4 — MEDIUM (security, output hygiene): C0 control characters from untrusted session content reach the terminal and every report format unfiltered

Session display names are taken from the first user message (`lib.rs:457` `session_display_name`), and no
layer strips control characters — `grep -rn "is_control\|u{1b}\|x07" crates/` returns nothing, and
`html_escape` (`reports.rs:2287`) handles only `&`, `<`, `>` (HTML *tag* injection is correctly escaped;
control bytes are not).

[REPRODUCED] — `/tmp/at3/logs4/a.jsonl` contains
`{"message":{"content":"\u001b]0;pwned\u0007ansi"}}`:

```console
$ agenttrace -d /tmp/at3/logs4 --sessions | cat -v
^[]0;pwned^Gansi   100  detailed  claude_code ...
$ agenttrace -d /tmp/at3/logs4 --search ansi | cat -v
  ^[]0;pwned^Gansi ...
$ agenttrace -d /tmp/at3/logs4 --overview -f html  | grep -c $'\x1b'   # → 1
$ agenttrace -d /tmp/at3/logs4 --overview -f markdown | grep -c $'\x1b'  # → 1
```

Raw ESC + BEL is emitted into the operator's terminal (OSC title rewriting, and the wider
terminal-escape-injection class: clipboard/kitty-keyboard-protocol abuse where the emulator supports it)
and into markdown/HTML artifacts that get committed or published. This is distinct from N8 (pass 2),
which covered markdown *structural* escaping. The vector is realistic for this tool: agenttrace's input
is foreign agent logs, and prompt content routinely round-trips text read from untrusted files or web
pages.

**Fix:** strip C0/C1 controls (except `\t`, `\n`) once, at the point strings enter
`Session`/report models — or centrally in the text/markdown/HTML cell writers — plus a fixture test.

---

### P3-5 — MEDIUM (DX, correctness): `--limit` is silently ignored by `--overview` — including the `--baseline` CI gate

`--limit` is applied by the governance reports (`main.rs:215-217`), `--compare` (`main.rs:239-240`) and the
session list (`main.rs:289` `render_session_list(..., args.limit)`), but the `--overview` branch
(`main.rs:326+`) calls `compute_overview(&sessions)` and `evaluate_overview_gate(...)` on the **full**
set with no `.take(args.limit)`.

[REPRODUCED] on a 3-session directory:

```console
$ agenttrace -d /tmp/at3/logs3 --overview --limit 1
Total Sessions: 3        # expected 1
$ agenttrace -d /tmp/at3/logs3 --sessions --limit 1   # honours it
```

Worse than cosmetic: `--baseline` thresholds (`--fail-under-health`, `--fail-on-critical`,
`--max-tool-fail-rate`) are evaluated on the unlimited corpus, so an operator narrowing a report with
`--limit` to scope a CI gate gets a gate verdict computed over everything. `docs/guides/governance-reports.md:15`
documents `--limit` as a supported control. (`--limit 0` is separately confusing: it is accepted and
yields "No sessions match the requested filters" rather than a range error.)

**Fix:** apply the limit in the overview branch, or reject `--limit` with `--overview` loudly.

---

### P3-6 — MEDIUM (performance): `--doctor` re-parses every uncached file on every run and never writes the session cache

```rust
// crates/agenttrace-core/src/doctor.rs:162-170
for file in files {
    if cached_session(file, cache).is_some() {
        cache_hits += 1;
        parsed += 1;
    } else if parse_file(file).is_ok() {   // full parse, result discarded
        parsed += 1;
    } ...
}
```

`store_session` / `save_session_cache` are never called from `doctor.rs` (the only callers are
`discovery.rs:234` and `session_cache.rs:324`), so the parse work is thrown away and repeated forever.
`--doctor` is the documented *first* command (`skills/agenttrace-session-audit/SKILL.md` step 3, and the
"if no sessions are detected" guidance), so the slowest path in the tool is the one every new user hits
first.

[REPRODUCED] on a 1500-file corpus with a fresh `XDG_CACHE_HOME=/tmp/at3/cache9`:

```console
$ time agenttrace -d /tmp/at3/perf --doctor   # 7.6s  (cold)
$ time agenttrace -d /tmp/at3/perf --doctor   # 7.7s  (no caching happened)
$ ls /tmp/at3/cache9/agenttrace/              # directory was never even created
$ time agenttrace -d /tmp/at3/perf --overview # 8.0s, and a second run is 0.5s
```

**Fix:** reuse the discovery load path (which stores and saves), or call `store_session` +
`save_session_cache` while doctor walks.

---

### P3-7 — LOW (DX/POSIX): report output to stdout never ends with a newline, while the same command's `-o` file does

All eight report branches do `write_output(&args.output, &(out.clone() + "\n"))?;` followed by
`write_stdout(&out)?;` — the `"\n"` is added for the file only (`main.rs:233, 249, 259, 276, 290, 309,
321, 381`).

[REPRODUCED]:

```console
$ agenttrace -d /tmp/at3/logs --overview -f markdown -o o.md ; wc -c o.md        # 956
$ agenttrace -d /tmp/at3/logs --overview -f markdown | wc -c                     # 955
$ agenttrace -d /tmp/at3/logs --overview -f json | tail -c 1 | xxd               # '7d' — no \n
```

Text streams that don't end in a newline break `wc -l`, `read` loops, `git diff --no-index`-based
artifact comparisons and a number of naive parsers.

**Fix:** `write_stdout(&(out + "\n"))`.

---

### P3-8 — LOW (maintainability): byte-identical `user_cache_dir()` exists three times in one crate, and the TUI's ranking helpers are duplicated across two files

- `doctor.rs:333`, `pricing.rs:1113`, `session_cache.rs:743` — three private, byte-identical copies of
  `user_cache_dir()` (verified by side-by-side read; the macOS / `XDG_CACHE_HOME` / `$HOME/.cache` /
  temp-dir chain).
- `tui/shared.rs:111, 131, 226` and `tui/presentation.rs:3315, 3335, 3596` — `top_driver`,
  `top_anomaly_driver`, `total_tokens_all` implemented twice each; the uncommitted cycle-1 hardening had
  to apply the same `saturating_add` fix in **both** files (the diff touches them identically).
- `percentile` (P3-3) and `number_as_i64` (N1) are the same disease with worse symptoms.

This duplication pattern is precisely why N1 is still open: `sqlite_sessions.rs` was the copy nobody
updated. The next hardening pass will miss one copy again.

**Fix:** hoist to single modules (`crate::paths::user_cache_dir`, `crate::stats::percentile`,
`crate::jsonnum::number_as_i64`) and re-export; delete the copies.

---

### P3-9 — LOW (maintainability, minor perf): `count_by_root` in `doctor_directories` is dead code

```rust
// crates/agenttrace-core/src/doctor.rs:124-133
let mut count_by_root = BTreeMap::new();
for candidate in known_session_dirs() {
    count_by_root.insert(candidate.path, 0usize);
}
for file in files {
    for (root, count) in &mut count_by_root {
        if is_under(file, root) { *count += 1; }
    }
}
```

`count_by_root` is built and incremented but never read; the per-directory counts are recomputed below
with a fresh `files.iter().filter(...)`. This also adds an O(files × known-roots) pass that the report
doesn't use.

**Fix:** delete the block.

---

## Verified clean this pass (negative results, so the next pass doesn't re-walk them)

- **Fuzz sweep:** 21 malformed files (deep JSON nesting, NaN/Infinity, 10 MB strings, negative/huge
  usage, zero timestamps, byte-order marks, mixed-format lines, empty files) × every subcommand → no
  panics, no hangs, no negative totals; debug build (overflow checks on) and release build agree.
- **Performance:** 1500-file corpus → 8.0 s cold / 0.55 s cached for `--overview`; memory flat.
- **Search:** `search.rs` is substring-only — no regex, so no ReDoS surface; case-insensitive matching is
  hand-rolled and correct on ASCII.
- **Bounds:** the reports.rs index guards (2034/2048) and `sessions.len()` fallback (306) behave under
  empty inputs.
- **Packaging consistency:** `plugin.json` 0.7.1 matches CHANGELOG and `check-plugin-version.sh`;
  `pricing_snapshot.json` is internally consistent (2458 models, `PRICING_SNAPSHOT_DATE` =
  2026-09-02); `npm/scripts/install.js` pins a sha256 and caps redirects (same-origin checksum caveat
  noted in pass 1 remains).
- **HTML structural escaping:** `<script>`, `<img onerror>` from session names are correctly neutralised
  in `--overview -f html` (only the control-character class of P3-4 survives).
- **Skill accuracy:** every command in `skills/agenttrace-session-audit/SKILL.md` runs as written.
- **Minor, not filed:** `--doctor`'s `Session files: N` uses the unfiltered file list while `Providers:`
  shows the per-root breakdown; `npm/package.json` `files: ["lib", ...]` references a directory that
  does not exist in-repo; directory walking skips symlinked session dirs (`entry.file_type()` does not
  follow symlinks).

---

## Status of prior findings re-verified this pass

| ID | Status | Evidence this pass |
|---|---|---|
| N1 (sqlite ingestion unhardened) | **still open** | `sqlite_sessions.rs:590-599` wrapping `as i64`; `:403` plain `+`; `:410-413` plain `+=`; the cycle-1 `adversarial_token_counts_stay_bounded_and_non_negative` test covers only `lib.rs` |
| N5 (TUI real-smoke gate `if: env.AGENTTRACE_TUI_REAL_DIR` never set) | still open | `.github/workflows/ci.yml` unchanged |
| N6 (governance-reports.md claims automatic network refresh) | still open | `docs/guides/governance-reports.md:51` |
| N7 (empty-dir error trailing space) | still open | `Error: No session files found in ` reproduced in P3-1 |
| N8 (markdown structural escaping) | still open | `markdown_cell` (`reports.rs:2429-2431`) escapes only `|` and `\n` — not backticks, `<>`/HTML passthrough, or control characters (P3-4) |
| F8 (Windows cache dir → temp) | still open | `user_cache_dir()` chain unchanged; see also P3-1/P3-8 |
| F4/F7/F11+ (pass-1 items fixed in cycle 1) | fixed, regression-tested | `lib.rs` saturating accumulators + `total_tokens` + two new tests; `testdata/generated/adversarial/` corpus wired into `discovery_contract.rs` |

---

## Prioritised remediation order

1. **P3-1** (Windows discovery) — blocks the entire product on a supported platform.
2. **P3-2** (UTC `--range today`/`1d`) — silent data loss in every report and CI gate, east of UTC.
3. **P3-3** (percentile split) — self-contradicting numbers; single-module fix unlocks P3-8.
4. **P3-4** (control chars) — one strip at the model boundary closes four surfaces.
5. **P3-5** (`--limit` on `--overview`) — gate-scoping hazard for CI users.
6. **P3-6** (`--doctor` never caches) — largest visible latency win for the first-run path.
7. **P3-7, P3-9**, then **P3-8** as the structural enabler for N1's permanent fix.
