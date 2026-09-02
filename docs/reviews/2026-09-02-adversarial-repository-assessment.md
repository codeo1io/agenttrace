# Adversarial repository assessment — 2026-09-02

- **Scope:** whole repository at `e005952` (`docs: remove stale README run summary (#281)`), unmodified working tree.
- **Method:** routed via the compound-engineering router to `ce-code-review`. This harness exposes no
  subagent-dispatch or cross-model-peer surface, so the reviewer roster ran **in-thread** — disclosed here, and
  agreement between lenses below is *not* independent corroboration.
- **Baseline:** `cargo test --workspace` → 147 passed / 0 failed. `cargo fmt --check` clean. `cargo build
  --release` succeeds. Clippy unavailable in this environment (`rustup component add clippy` not installed), so
  no lint evidence is claimed.
- **Empirically reproduced findings are marked [REPRODUCED]** with the command used.

---

## Actionable Findings

### F1 — HIGH (correctness, reliability): token aggregation overflows on untrusted log input

- `crates/agenttrace-core/src/lib.rs:1076-1081` — `total_tokens()` sums four `i64` fields with plain `+`.
- `crates/agenttrace-core/src/lib.rs:527`, `:544`, `:558`, `:562` — the accumulators `tokens_input +=` /
  `tokens_output +=` are likewise unguarded.
- Root cause is at `crates/agenttrace-core/src/parser.rs:3576-3585` — `number_as_i64`:
  - `number.as_u64().map(|n| n as i64)` wraps any `u64 > i64::MAX` to a **negative** value.
  - `number.as_f64().map(|n| n as i64)` **saturates** to `i64::MAX`.

**[REPRODUCED]** A session file whose `message.usage` carries `input_tokens: 1e300` and
`output_tokens: 1e300` (both saturate to `i64::MAX`, then sum):

```
$ ./target/debug/agenttrace -d /tmp/at-hugelogs --sessions
thread 'main' panicked at crates/agenttrace-core/src/lib.rs:1077:5:
attempt to add with overflow            # exit 101

$ ./target/release/agenttrace -d /tmp/at-hugelogs --sessions
probe  96  detailed  claude_code  claude-sonnet-4  166020696663385.9375  -2  0  1
                                              # TOKENS = -2, exit 0
```

Debug builds panic; shipped release builds silently emit **negative token totals** and an absurd cost,
and `--overview -f json` propagates `"tokens": -2` to machine-readable output used by CI gates.
agenttrace's core job is parsing foreign log formats, so this is a routine-corruption path, not an exotic one.

**Fix direction:** use `checked_add`/`saturating_add` in `total_tokens` and the accumulators, and reject or
clamp out-of-range usage numbers in `number_as_i64` (return `None` above `i64::MAX`).

---

### F2 — HIGH (security/privacy, docs): undisclosed blocking network fetch on ordinary runs

- `crates/agenttrace-core/src/pricing.rs:236-248` — `pricing_catalog()` lazily calls
  `download_pricing(Duration::from_secs(5))` whenever the cached `pricing.json` is older than
  `CACHE_MAX_AGE` (24h, `pricing.rs:12`).
- Reached from the normal path: `crates/agenttrace-core/src/lib.rs:488` (`analyze()` → `lookup_price`),
  `crates/agenttrace-core/src/waste.rs:181`, `crates/agenttrace-core/src/governance.rs:84`,
  `crates/agenttrace-core/src/sqlite_sessions.rs:483`.
- `PRIVACY.md:5` states the tool is local-first and names **only** `--update-pricing` as the command that
  downloads pricing metadata. `README.md:8` and `README.md:31` headline "Local-first".

**[REPRODUCED]** With a >24h-old `$XDG_CACHE_HOME/agenttrace/pricing.json`:

```
$ ./target/debug/agenttrace --demo --overview -f json     # ordinary report path
--- cache file after: 2090796 bytes, mtime=<now>          # network fetch + rewrite
```

The same happens for `--test-match`, and for **`cargo test`** (see F3). There is no offline opt-out: the only
pricing-related env vars are `AGENTTRACE_PRICING_FILE`, `AGENTTRACE_HISTORY_DIR`,
`AGENTTRACE_SESSION_CACHE_DIR`, `AGENTTRACE_RELEASE_VERSION`.

Secondary impact: it is a **blocking 5-second** synchronous call (`pricing.rs:239`) inside a `OnceLock`
initializer on whichever thread first needs a price — i.e. first paint of the TUI, or the start of every
report, on an offline/slow network.

**Fix direction:** restrict the fetch to `--update-pricing` (as documented), or gate it behind an explicit
opt-in plus an `AGENTTRACE_OFFLINE=1` kill-switch, and correct `PRIVACY.md`.

---

### F3 — HIGH (test, CI): `cargo test` performs live network I/O and mutates the user cache

**[REPRODUCED]**

```
$ XDG_CACHE_HOME=/tmp/at-price-test/cache2  # stale 1-entry pricing.json, mtime -3 days
$ cargo test -p agenttrace-core --lib pricing
test result: ok. 6 passed
$ stat pricing.json
2090796 bytes, mtime=<now>       # tests downloaded and rewrote it
```

`.github/workflows/ci.yml:44` (`Rust tests`) runs with no network isolation, so the suite is non-hermetic and
depends on live `raw.githubusercontent.com` content. `ci.yml:6-7` even schedules a nightly cron run, which
makes this a standing external dependency and a flake source.

**Fix direction:** pin a `test data` pricing fixture via `AGENTTRACE_PRICING_FILE` in the test harness, or
inject the catalog.

---

### F4 — MEDIUM (performance, reliability): session cache is never pruned of deleted files

- `crates/agenttrace-core/src/session_cache.rs:528-549` — `cached_session` deletes an entry only when that
  exact path is visited again; `delete_cached_session_key` (`session_cache.rs:476-478`) has no other callers
  that sweep.

**[REPRODUCED]**

```
run1 keys: ['/tmp/at-logs/a.jsonl']
run2 (file removed) keys: ['/tmp/at-logs/a.jsonl']          # still present
run4 (all removed) keys: [a.jsonl, f1.jsonl, f2.jsonl, f3.jsonl]  size=8429
```

Entries for rotated/deleted logs accumulate forever and are re-serialized and rewritten on every save
(`session_cache.rs:492-526`). Switching `HOME`/`CLAUDE_CONFIG_DIR`/`CODEX_HOME` orphans the old entries
permanently.

**Fix direction:** drop entries whose `fs::metadata` fails after a scan, and/or cap entry count.

---

### F5 — MEDIUM (correctness, CI): "deterministic demo output" is cache-state dependent

- `pricing.rs` formats the provenance string as `"LiteLLM (fetched {now})"` / `"LiteLLM (cached {ts})"` /
  `"built-in fallback"`, and this string is emitted into `--overview -f json`.

**[REPRODUCED]** Consecutive identical invocations of `--demo --overview -f json`:

```
<     "pricing_source": "LiteLLM (fetched 2026-09-02 04:08)",
>     "pricing_source": "LiteLLM (cached 2026-09-02 04:08)",
```

and comparing a no-cache machine against a warm-cache machine changes **actual cost values**:

```
<           "cache_read": 0.0,      # built-in pricing
>           "cache_read": 0.125,    # LiteLLM pricing
```

`scripts/ci/check-deterministic-output.sh` byte-compares three runs inside one job, so it passes only because
all three share the same cache state. On a runner with a pre-populated stale cache the first run would emit
`fetched` and the later runs `cached`, and the check would fail. More importantly, two developers get
different demo output for the same binary.

**Fix direction:** exclude `pricing_source` from the compared payload, or normalize it to a stable token.

---

### F6 — MEDIUM (reliability): `preserve_derived_history` writes non-atomically and can destroy all history

- `crates/agenttrace-core/src/history.rs:32-44` — `fs::write(path, …)` directly on the accumulated records
  map, with no temp-file + rename. Contrast `session_cache.rs:515-526`, which does `tmp` + `rename`.
- `crates/agenttrace-core/src/history.rs:62-70` — `decode_records` maps any parse error to an empty map, so a
  truncated file is silently read as "no history" and then **overwritten** on the next `--preserve-history`.

An interrupt mid-write loses the entire accumulated history with no warning or recovery path.

**Fix direction:** mirror the tmp+rename pattern used by the session cache, and back up/quarantine an
undecodable file rather than discarding it.

---

### F7 — MEDIUM (maintainability, reliability): history IDs depend on `DefaultHasher` stability

- `crates/agenttrace-core/src/history.rs:55-60` — `session_id` builds a `DefaultHasher::new()` over
  `session.path` + `session_start`. `DefaultHasher` is documented as having an *unspecified* algorithm that is
  not guaranteed stable across Rust releases. A toolchain bump can silently re-key every preserved record,
  which makes `merge_preserved_history` (`history.rs:46-58`) treat old records as new sessions
  (duplication) and orphan the originals.

**Fix direction:** use an explicit, versioned digest (e.g. a documented FNV/BLAKE3 or a plain string key).

---

### F8 — MEDIUM (correctness, platform): Windows cache/data directories fall back to the temp dir

- `crates/agenttrace-core/src/pricing.rs:1089-1105`, `crates/agenttrace-core/src/doctor.rs:333-346`,
  `crates/agenttrace-core/src/session_cache.rs:743` — three near-identical hand-rolled `user_cache_dir()`
  implementations covering macOS and XDG/Linux only.
- `crates/agenttrace-core/src/history.rs:21-29` — same shape for `history_path()`.

On stock Windows `HOME` is unset, so the pricing cache, session cache, and preserved history all land in
`std::env::temp_dir()` (`%TEMP%`), which the OS and disk-cleanup tasks periodically purge — silently
discarding the session cache and re-downloading pricing. Windows is a first-class target:
`.github/workflows/release.yml` builds `windows-amd64`/`windows-arm64`, `install.ps1` exists, and WinGet is a
documented channel. The workspace has no `dirs`-style dependency, so this is not handled anywhere else.

**Fix direction:** use the `dirs` crate (or add a `%LOCALAPPDATA%` branch) and consolidate the three
duplicated `user_cache_dir()` copies into one function.

---

### F9 — MEDIUM (security, supply chain): `install.sh` verifies no checksum

- `install.sh:52-70` — the only integrity control is `SIZE=$(wc -c < "$TMP")` with a `> 1000000` floor, after a
  `curl | sh` download from `github.com/.../releases/latest/download/...`.
- `npm/scripts/install.js:75-85` — the npm channel *does* fetch and verify a `.sha256` sibling.

The two official install channels have inconsistent supply-chain posture; a tampered release asset installs
silently via the shell channel. (The npm checksum is fetched from the same origin over the same channel, so it
guards transport corruption, not a compromised release — but it is still strictly better than nothing.)

**Fix direction:** download and verify the `.sha256` asset in `install.sh` exactly as the npm installer does.

---

### F10 — LOW (security): `git` is executed inside a directory taken from untrusted logs

- `crates/agenttrace-core/src/governance.rs:733-744` — `Command::new("git").args(["-C", root, "log", "--all",
  "--format=%ct"])`, where `root` comes from `resolve_project` over session-log `cwd` fields.
- No shell is involved, so there is no shell injection. But a crafted repository's local `.git/config` can set
  hooks-adjacent settings (e.g. `core.fsmonitor`) that execute a command when git runs in that worktree.

Read-only intent, low likelihood, worth a note in `SECURITY.md` at minimum.

---

### F11 — LOW (reliability): concurrent cache writes race on a fixed temp filename

- `crates/agenttrace-core/src/session_cache.rs:515-526` — writes `sessions.json.tmp` then `fs::rename`. Two
  concurrent `agenttrace` processes (a TUI plus a CLI gate, two TUIs) share that path; last writer wins and a
  lost race can surface as `ENOENT` on rename, which `discovery.rs:239` swallows via `let _ =`.

**Fix direction:** include the PID in the temp name, or take an advisory lock.

---

### F12 — LOW (DX): `--lang` silently accepts any value

- `crates/agenttrace-cli/src/main.rs:574-579` — unknown values map to English with no diagnostic.
  `--lang fr` runs happily. Contrast `--format`, which is a clap `value_enum` and rejects bad input.

---

### F13 — LOW (test efficacy): release-surface CI checks assert text presence, not behavior

- `scripts/ci/check-release-surfaces.sh` — e.g. the requirement that `install.sh` contain
  `cargo build --release -p agenttrace` is satisfied by the `echo` string at `install.sh:63`; the requirement
  that `npm/scripts/install.js` contain `checksum mismatch` is satisfied by the error message alone. Several
  sibling checks have the same shape. They are drift alarms, not behavioral guarantees.

---

### F14 — LOW (maintainability): `.gitignore` carries entries from a different project

- `.gitignore:13-16` — `apps/desktop/vite.config.js`, `apps/desktop/src-tauri/gen/`,
  `apps/desktop/coverage/`, plus `agentwaste`. No `apps/` directory or `agentwaste` exists in this Rust-only
  tree. The `vite.config.js` entry in particular is a footgun: it would silently ignore a real config file if
  such a path ever appeared.

---

### F15 — LOW (CI): declared MSRV is never tested

- `Cargo.toml:12` declares `rust-version = "1.80"`, but `.github/workflows/ci.yml:33-36` sets up only
  `stable`. Nothing prevents a dependency or language feature from silently raising the real MSRV.

---

### F16 — LOW (DX): non-UTF-8 session files disappear into `skipped` with no reason

- `crates/agenttrace-core/src/parser.rs:21` — `read_to_string` fails for non-UTF-8 files;
  `discovery.rs:225-228` increments `skipped` without recording why. `data_health` reports a count but not a
  cause, so a whole provider can silently vanish.

---

### F17 — LOW (CI hygiene): fixed `/tmp` scratch paths

- `scripts/ci/check-docs-commands.sh:26-27` — writes `/tmp/agenttrace-docs-md.stdout` and
  `/tmp/agenttrace-docs-html.stdout`, predictable paths shared across concurrent jobs/users.

---

### F18 — LOW (correctness): `--since` silently drops sessions with unparseable start times

- `crates/agenttrace-core/src/discovery.rs:246-251` — `is_some_and(|time| … >= since)` means a session whose
  `session_start` fails RFC-3339 parsing is excluded from every `--since` result, rather than being explicitly
  bucketed or surfaced.

---

### F19 — LOW (maintainability): `.codex-plugin/plugin.json` version is hand-maintained

- `.codex-plugin/plugin.json:3` pins `"version": "0.7.1"` against `CHANGELOG.md:3` (`v0.7.1 - 2026-07-20`),
  with no check in `scripts/ci/check-release-surfaces.sh` tying the two together. The workspace version is a
  `0.0.0-dev` placeholder, so drift would be silent.

---

## Coverage

- **Read in full or in depth:** `crates/agenttrace-core/src/{pricing,session_cache,history,discovery,search}`
  and the JSON/HTML emit paths of `reports.rs`; targeted reads of `parser.rs`, `lib.rs`, `diagnostics.rs`,
  `governance.rs`, `sqlite_sessions.rs`, `app.rs`, `presentation.rs`, `i18n.rs`, `main.rs`.
- **Built and run:** `cargo test --workspace` (147 passed), `cargo build --release`, `cargo fmt --check`,
  `--help`, `--doctor`, `--demo --overview` in text/markdown/json, `--sessions`, `--waste`, `--audit`,
  `--recommend`, `--list-models`, `--test-match`, `--search`, gate exit codes.
- **Empirically reproduced:** F1, F2, F3, F4, F5 (commands inline above).
- **Checked clean:** HTML escaping (`reports.rs:2282-2295` covers `& < > " '`), JSON string emission routes
  through `serde_json` (`reports.rs:1373-1375`), search is literal substring (no ReDoS), no `.unwrap()`/panic
  sites in the 4,120-line `parser.rs`, preserved history stores only derived fields (verified by its own test
  at `history.rs:100-145`), no secrets in tracked files, GitHub Actions pinned by full SHA, every README/docs
  command I ran succeeded.
- **Not covered:** `crates/agenttrace-tui/src/explorer.rs` and `presentation.rs` were skimmed, not line-read;
  no clippy run (component not installed here); no live TUI interaction (no terminal in this harness); the
  `.codex-plugin` and winget submission flows were reviewed by inspection only.
- **Independence:** none. All lenses ran in this single context per the disclosure at the top.

## Verdict

The repository is in good shape on the dimensions most often botched — escaping, injection, secret hygiene,
doc-command accuracy, formatting, and a real 147-test suite. The material risk concentrates in two places:
**untrusted-input arithmetic (F1)**, which produces silently wrong output in the binaries users actually run,
and the **pricing subsystem's relationship to the network and to its own cache (F2, F3, F5)**, which
contradicts the project's central local-first privacy claim and makes both tests and "deterministic" output
non-hermetic. F6–F8 are durability/platform gaps that will bite Windows users and long-history users first.

Nothing here was fixed, staged, committed, or pushed — this phase is assessment only.
