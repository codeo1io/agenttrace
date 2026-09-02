# Adversarial repository assessment — 2026-09-02 (second pass)

- **Scope:** whole repository at `e005952` **plus the uncommitted cycle-1 working tree** (15 tracked files
  modified, `docs/`, `scripts/pricing/`, `crates/agenttrace-core/src/pricing_snapshot.json` untracked). This
  is the tree a downstream phase would actually validate.
- **Relation to the prior assessment:** this pass deliberately starts from zero and does **not** re-derive
  `docs/reviews/2026-09-02-adversarial-repository-assessment.md` (F1–F19) or the cycle-1 implementation
  review. Old findings appear only in the status table at the end, and only where I re-verified them this
  pass. Everything in "Actionable findings" below is new.
- **Baseline:** `cargo test --workspace` → 159 passed / 0 failed (11 + 1 + 50 + 7 + 50 + 40).
  `cargo fmt --all -- --check` clean. `cargo clippy --workspace --all-targets` → **0 warnings**.
  `cargo build --release -p agenttrace` succeeds. All ten `scripts/ci/*.sh` pass locally when given
  `AGENTTRACE_BIN`.
- **Method disclosure:** the work order names a compound-engineering router / `ce-*` skill; no `ce-*` skill
  library is installed in this delegate environment (verified: `find / -name "ce-*"` returns only leftover
  run artifacts). As with the fleet's prior phases, I ran the adversarial review methodology directly and
  disclose that all lenses ran in-thread — agreement between lenses is not independent corroboration.
- **Empirically reproduced findings are marked [REPRODUCED]** with the command. Reproducers live in
  `/tmp/at-assess2/`.

---

## Actionable findings

### N1 — HIGH (correctness, reliability): the SQLite ingestion path was never hardened — the F1 overflow class is still fully reachable, in both build modes

The cycle-1 fix hardened `parser.rs::number_as_i64`, the `lib.rs` accumulators, `reports.rs`, `governance.rs`,
`insights.rs` and the TUI. It did **not** touch `sqlite_sessions.rs`, which contains its own private,
still-unsanitized copy of the same conversion plus four unguarded accumulators:

- `crates/agenttrace-core/src/sqlite_sessions.rs:613-621` — `number_as_i64` still does
  `number.as_u64().map(|n| n as i64)` (wraps `u64 > i64::MAX` to a **negative**) and
  `number.as_f64().map(|n| n as i64)` (saturates to `i64::MIN`/`i64::MAX`). The `parser.rs` twin was
  rewritten with clamping and explanatory comments; this duplicate was left behind.
- `crates/agenttrace-core/src/sqlite_sessions.rs:403` —
  `number_as_i64(tokens.get("output")) + number_as_i64(tokens.get("reasoning"))`, plain `+`.
- `crates/agenttrace-core/src/sqlite_sessions.rs:410-413` — `agg.input_tokens += input;` … four plain `+=`
  accumulations across every message row.
- `crates/agenttrace-core/src/sqlite_sessions.rs:179-182` — the Hermes `sessions` table columns are read as
  raw `i64` with `unwrap_or(0)` and **no negative clamp**, unlike the file path (`lib.rs:525-529` clamps with
  `.max(0)`).

**[REPRODUCED]** OpenCode database, one message with `tokens: {input: 10, output: 1e300, reasoning: 1e300}`:

```
$ HOME=/tmp/at-assess2/home1 XDG_CACHE_HOME=…/cache1 ./target/debug/agenttrace --sessions -f json
thread 'main' panicked at crates/agenttrace-core/src/sqlite_sessions.rs:403:18:
attempt to add with overflow                        # exit 101

$ … ./target/release/agenttrace --sessions -f json   # exit 0
"tokens_output": -2
$ … ./target/release/agenttrace --overview -f json
"summary": { "total_tokens": 8, "total_cost": 0.0 }
```

**[REPRODUCED]** Same database with `input: 18446744073709551615` (`u64::MAX`):

```
$ … ./target/release/agenttrace --sessions -f json
"tokens_input": -1        # u64::MAX wrapped through `as i64`
```

This is the *exact* symptom class F1 was filed against — debug panic, release silent negative — reached
through a provider path the hardening never visited. Both `opencode.db` (third-party agent data) and Hermes
`state.db` (an external store agenttrace only reads) reach it.

**Fix direction:** delete the local `number_as_i64` and reuse the hardened `parser` version (or apply the
same clamp), use `saturating_add` at lines 403/410-413, and clamp the Hermes row reads to `>= 0`.

---

### N2 — HIGH (correctness, machine-readable output): adversarial SQLite usage yields negative costs and absurd totals in `--audit` / `--context-trends` JSON, stamped `confidence: "high"`

Same root cause as N1; listed separately because the damage lands in the governance reports that CI gates
consume.

**[REPRODUCED]** `opencode.db` message with `tokens: {input: -1e300, output: 5, cache: {read: 1e300}}`
(`input` saturates to `i64::MIN`, `cache.read` to `i64::MAX`):

```
$ … ./target/release/agenttrace --audit -f json
{
  "total_estimated_cost": -24903104499507.895,
  "stored_estimated_cost_usd": -24903104499507.895,
  "pricing_coverage": { "exact_pricing_pct": 100.0, "confidence": "high" },
  "by_provider_model": [ { "tokens": { "input": -9223372036854775808,
                                       "output": 5, "cache_read": 9223372036854775807, "total": 4 } } ]
}

$ … ./target/release/agenttrace --context-trends -f json
  "totals": { "output_cost_per_million_tokens": -4.980620899901579e+18, … }
  "projects": [ { "cost_per_output_token": -4980620899901.579, … } ]
```

`tokens.total = 4` (the saturating sum of `i64::MIN` and `i64::MAX`), a negative fleet cost, and
`confidence: "high"` — the report asserts high confidence in numbers that are arithmetically nonsense.
**[REPRODUCED]** identically via Hermes `state.db` with `input_tokens = -9223372036854775808,
cache_read_tokens = 1` → `total_estimated_cost: -27670116110564.33`.

**Fix direction:** same as N1, plus a guard in `cost_audit`/`context_trends` that refuses to report
`high` confidence when any component total is negative.

---

### N3 — HIGH (reliability): `--waste` panics in debug and prints inverted garbage in release

- `crates/agenttrace-core/src/waste.rs:180` —
  `let wasted_tokens = (metrics.tokens_input - metrics.tokens_cache_r).max(0);` — plain subtraction of two
  `i64` fields that the SQLite path can drive to `i64::MIN` and `i64::MAX`.

**[REPRODUCED]** (both providers, both build modes):

```
# opencode.db: input=-1e300, cache.read=1e300
$ … ./target/debug/agenttrace --waste
thread 'main' panicked at crates/agenttrace-core/src/waste.rs:180:25:
attempt to subtract with overflow

$ … ./target/release/agenttrace --waste
  -- Cache --
  none (hit 0%, 9223372.0T read / -9223372.0T input)

# hermes state.db: input_tokens=i64::MIN, cache_read_tokens=1  → same debug panic
```

Note `waste.rs:180` is independently reachable from the **Hermes** columns, which are never clamped — so
fixing only `add_opencode_tokens_from_map` does not close this site.

**Fix direction:** `metrics.tokens_input.saturating_sub(metrics.tokens_cache_r).max(0)`.

---

### N4 — MEDIUM (security/privacy, policy compliance): `--update-pricing` ignores every proxy environment variable

- `crates/agenttrace-core/src/pricing.rs:316` — the sole network call is `ureq::get(PRICING_URL)` with no
  explicit proxy. `ureq` is pinned at **2.12.1** (`Cargo.toml:34`, `Cargo.lock`), and ureq 2.x does **not**
  read `http_proxy` / `https_proxy` / `all_proxy` from the environment (env-var proxy support arrived in
  ureq 3.x; in 2.x a proxy must be passed to `AgentBuilder::proxy`).

**[REPRODUCED]**

```
$ env -i PATH=/usr/bin:/bin HOME=/tmp/at-assess2/up2 XDG_CACHE_HOME=… \
      https_proxy=http://127.0.0.1:9 HTTPS_PROXY=http://127.0.0.1:9 \
      ./target/release/agenttrace --update-pricing
Downloading pricing from LiteLLM...
Loaded 1218 model prices
Cache saved: /tmp/at-assess2/up2/.cache/agenttrace/pricing.json      # 2,090,796 bytes, 0.17 s

# same env, curl correctly refuses:
$ env -i PATH=/usr/bin:/bin https_proxy=http://127.0.0.1:9 curl -sS https://raw.githubusercontent.com/…
curl: (7) Failed to connect to 127.0.0.1 port 9: Connection refused
```

Also confirmed with `all_proxy`/`ALL_PROXY`. Impact: the *one* documented network action
(`PRIVACY.md:7`) is not controllable by the operator's standard proxy mechanism. In a corporate network that
mandates an egress proxy the connection either fails confusingly or — worse — **succeeds directly**,
bypassing the proxy's monitoring/DLP. It also invalidates `https_proxy=127.0.0.1:9` as a way to prove
"this build makes no network calls", which is how several of this repository's own privacy claims get
tested. (The offline-by-default report path itself is genuinely offline — verified separately with no cache
and no proxy: exit 0, no files written, `pricing_source: "LiteLLM snapshot 2026-09-02 (bundled)"`.)

**Fix direction:** build the request through an `Agent` configured from the environment, or upgrade to
ureq 3.x, and document the proxy behaviour in `PRIVACY.md`.

---

### N5 — MEDIUM (test efficacy, CI): the "Rust TUI real-data smoke" CI step can never execute

- `.github/workflows/ci.yml:84` — `if: env.AGENTTRACE_TUI_REAL_DIR != ''`.

`AGENTTRACE_TUI_REAL_DIR` is defined nowhere: `grep -rn AGENTTRACE_TUI_REAL_DIR` over the repository returns
only `ci.yml:84` and the script's own `${AGENTTRACE_TUI_REAL_DIR:-…}` default. The workflow has no
`env:` block, and the `env` context in a step `if` does **not** read repository variables (`vars.`) or
secrets — so the condition is structurally always false. The step is permanently skipped while its name in
the CI job listing implies interactive TUI coverage exists. `scripts/ci/check-rust-tui-real-smoke.sh` also
requires `expect`, which is not installed on `ubuntu-latest`, so enabling it would fail immediately.

(The TUI itself is robust — I drove it in a pty at 1×1, 3×3, 5×20, 12×40, 24×80, 50×6 and 40×200 and
through ~28 scripted/shuffled key sequences at six sizes, with no panic.)

**Fix direction:** delete the step, or gate it on a workflow input / repository variable it can actually
observe and add an `expect` install step.

---

### N6 — MEDIUM (documentation, privacy): the user guide still documents the automatic network refresh that was removed

- `docs/guides/governance-reports.md:51` —

  > "Pricing data is cached locally. A cache older than 24 hours is refreshed automatically when a price
  > lookup starts; if the refresh fails, the stale cache remains usable and is reported as stale. Use
  > `--update-pricing` to force an explicit refresh."

That behaviour was deleted by the H2 "offline by default" change (uncommitted `CHANGELOG.md` "Unreleased";
`PRIVACY.md:7` now says "no report or test path contacts the network"). The shipped binary serves a stale
cache as-is and never downloads (`pricing.rs:292-307`, and I verified a 7-day-old cache survives a run
byte-identical). The guide therefore re-asserts the exact claim F2 was filed against, contradicting both the
privacy policy and the binary. `scripts/ci/check-docs-commands.sh` only executes fenced commands, so prose
drift like this is invisible to CI.

**Fix direction:** rewrite that paragraph to describe snapshot + explicit `--update-pricing`, and consider a
docs grep that fails on the phrase "refreshed automatically".

---

### N7 — MEDIUM (correctness, data fidelity): SQLite-backed sessions with no timestamps silently vanish from every time-bounded report

- `crates/agenttrace-core/src/sqlite_sessions.rs:243-253` — `filter_since` keeps a session only when
  `DateTime::parse_from_rfc3339(&session.metrics.session_start)` succeeds (`is_some_and`).
- `crates/agenttrace-core/src/sqlite_sessions.rs:560-570` — `unix_seconds_rfc3339` returns `""` for any
  `value <= 0.0`, so `session_start` is empty whenever `time_created` is 0/NULL.

**[REPRODUCED]** `opencode.db` with `session(time_created = 0, time_updated = 0)` and one message:

```
$ … ./target/release/agenttrace --range all  --overview -f json   →  "total_sessions": 1
$ … ./target/release/agenttrace --range 7d   --overview -f json   →  Error: No session files found in
$ … ./target/release/agenttrace --range 30d  --overview -f json   →  Error: No session files found in
$ … ./target/release/agenttrace --range today --overview -f json  →  Error: No session files found in
```

No `skipped` accounting, no `data_health` signal — the session simply stops existing for every default
time-bounded view. This is the same *shape* as the prior F18 but a different site and a different blast
radius: F18 dropped individual file-backed sessions; this drops a whole provider's rows and is the path every
OpenCode/Hermes user hits. (Cosmetic companion: the error message ends with an empty directory name —
`"No session files found in "`.)

**Fix direction:** bucket unparseable-start sessions as "unknown time" and surface them in `data_health`
rather than filtering them out; include the search root in the error.

---

### N8 — LOW (security, DX): Markdown reports emit log-derived strings with no escaping

- `crates/agenttrace-core/src/reports.rs:2287` defines `html_escape`, used on every HTML cell; the Markdown
  writer has no counterpart. README.md advertises "Shareable evidence | JSON, Markdown, and self-contained
  HTML reports".

**[REPRODUCED]** A session log whose `message.model` is `claude-sonnet-4-5<img src=x onerror=alert(3)>`:

```
$ … ./target/release/agenttrace -d … --overview -f markdown
| evil | Claude Code | claude-sonnet-4-5<img src=x onerror=alert(3)> | 100 | $0.0105 | 0 |
```

The HTML output is correctly escaped (`&lt;img …&gt;`, verified). Most Markdown renderers — including
GitHub's — render that inline HTML, so a hostile session log becomes stored HTML/JS in a report the project
tells users to share. `PRIVACY.md` does warn users to review reports before sharing, which is why this is
LOW.

**Fix direction:** escape `<`, `>`, `&`, `` ` `` and `|` in Markdown cells (a `md_escape` next to
`html_escape`).

---

### N9 — LOW (DX): `--version` is unreachable when `--lang` is invalid

- `crates/agenttrace-cli/src/main.rs:147` — `let language = report_language(&args.lang)?;` runs *before* the
  `--version` branch at `main.rs:150`.

**[REPRODUCED]**

```
$ ./target/release/agenttrace --lang fr --version
Error: unsupported --lang value 'fr'; supported languages: en, zh     # no version printed, exit 1
```

The F12 fix added validation in the right place for reports but made an unrelated flag able to suppress a
metadata query.

**Fix direction:** hoist the `--version` early-return above the language parse.

---

### N10 — LOW (performance): `--delivery-evidence` runs `git log --all` synchronously per distinct project root

- `crates/agenttrace-core/src/governance.rs:740-747` — `git_commits_by_root` spawns one `git` subprocess per
  root; no parallelism, no timeout, no commit-count cap.

**[MEASURED]** Three sessions all rooted at a 31,319-commit repository:

```
$ … --delivery-evidence -f json   real 0m0.606s   (vs --overview on the same data: 0m0.011s)
```

A fleet report spanning N large monorepos pays that serially on every invocation, inside the TUI's report
view as well as the CLI.

**Fix direction:** run the per-root `git log` concurrently with a bounded pool and a timeout, and/or cache
commit timestamps by `(root, HEAD)`.

---

## Coverage

**Read in depth this pass:** `sqlite_sessions.rs` (all 641 lines), `session_cache.rs` (snapshot/prune/save
paths), `pricing.rs` (offline rewrite, snapshot, override and update paths), `waste.rs`, `insights.rs`
(`resolve_project`/`git_root`/scope), `governance.rs` (cost audit, context trends, delivery evidence),
`history.rs`, `discovery.rs` (walk/symlink/range), `reports.rs` (health-trend, percentile, markdown/HTML
emitters), `main.rs` (arg ordering, gates), `install.sh`, `scripts/pricing/update-snapshot.sh`,
`scripts/ci/check-plugin-version.sh`, `.github/workflows/ci.yml`.

**Built and ran:** `cargo test --workspace` (159 passed, incl. under a dead proxy with an isolated
`XDG_CACHE_HOME`/`HOME` — hermetic, no pricing cache written), `cargo fmt --check`,
`cargo clippy --workspace --all-targets` (0 warnings) plus a `clippy::pedantic` survey,
`cargo build --release`, `--overview`/`--sessions`/`--waste`/`--audit`/`--recommend`/`--context-trends`/
`--mcp-governance`/`--delivery-evidence`/`--search`/`--diagnostics` in text/JSON/Markdown/HTML,
`--range all|7d|30d|today|bogus`, `--update-pricing` under four proxy configurations, all ten CI check
scripts, `check-plugin-version.sh`.

**Fuzzed:** ~590 mutated-corpus runs across three seeds (150 + 220 + 220) seeded from all 33 `testdata/` fixtures (byte splice/swap/delete plus
hostile numeric literals `1e400`, `-1e400`, `u64::MAX`, `i64::MIN`, alias-pair `i64::MAX`) across eleven
report actions under isolated `HOME`/caches — **zero panics**, zero non-{0,1} exits. The file parser is solid
(0 panics in ~590 mutated runs); the bugs above are all in the SQLite path the fuzzer cannot reach.

**TUI exercised:** pty-driven at seven terminal sizes and six scripted/shuffled key sequences — no panics.

**Verified clean this pass:** HTML escaping covers log-derived model/session names; JSON emission routes
through `serde_json`; `cargo test` performs no network I/O and mutates no pricing cache; the offline report
path writes no cache file and reports the bundled snapshot; `check-plugin-version.sh` passes and is now
wired into CI; doc links resolve; no `ce-*` skill side effects; no secrets in tracked files; `discover`
walk does not follow directory symlinks (`DirEntry::file_type` is non-following) so there is no symlink-loop
path.

**Not covered:** `explorer.rs`/`app.rs` read only at the call sites cited above; no live Windows or macOS
run; `.codex-plugin`/winget submission flows reviewed by inspection; ureq's proxy behaviour established from
version pinning plus a controlled experiment rather than from reading ureq's source.

**Independence:** none — single context, disclosed above.

---

## Status of previously reported findings (re-verified this pass, not re-derived)

| Prior ID | Status now |
|---|---|
| F1 (token overflow), impl-review #1/#2/#4 | **Fixed** for the file-parser, report and TUI paths — but see **N1/N2/N3**: the SQLite path and `waste.rs:180` were missed |
| F2 (undisclosed network fetch) / F5 (clock in `pricing_source`) | **Fixed** — offline by default, clock-free labels, byte-identical consecutive runs |
| F3 (`cargo test` network) | **Fixed** — hermetic under a dead proxy |
| F12 (`--lang`), F14 (`.gitignore`), F19 (plugin version) | **Fixed** (F19 now wired at `ci.yml:106-107`) — with the new **N9** caveat on `--version` |
| impl-review #5 (adversarial corpus) | **Landed** — `testdata/generated/adversarial/` (4 fixtures) is present and exercised |
| F4 (cache never pruned of deleted files) | **Still open** — re-reproduced: after deleting every file and re-running twice, `sessions.json` still lists all three entries |
| F6, F7 (`history.rs` atomicity / `DefaultHasher`) | **Still open** — `history.rs:46` still `fs::write` on the full map; IDs still `DefaultHasher` (`history.rs:78`) |
| F8 (Windows cache dirs) | **Still open** — three hand-rolled `user_cache_dir()` copies, no `%LOCALAPPDATA%` branch (`pricing.rs:1113`, `doctor.rs:333`, `session_cache.rs:743`) |
| F9 (`install.sh` no checksum) | **Still open** — size floor only (`install.sh:63-70`) |
| F10 (git exec in untrusted cwd) | **Still open** — `governance.rs:740` |
| F11 (fixed `.tmp` race), F13 (text-presence CI checks), F15 (MSRV untested — `ci.yml:30` still `stable` only), F16 (non-UTF-8 → `skipped`), F17 (fixed `/tmp` paths), F18 (`--since` drops unparseable starts) | **Still open** |

---

## Verdict

The uncommitted cycle-1 work did what it claimed on the paths it touched: the offline pricing rewrite is
clean and verifiable, tests are hermetic, formatting and clippy are spotless, and the parser survived ~700
mutation runs without a single panic. The material risk is that the hardening stopped at the file-parser
boundary. **N1–N3** are the same overflow class as F1, still fully reachable through the SQLite ingestion
path and `--waste`, producing debug panics and negative, `confidence: "high"` machine-readable costs in the
release builds users run. **N4** means the one documented network action is not governable by proxy policy.
N5–N10 are CI/docs/DX defects that cost little to close.

Nothing was fixed, staged, committed, or pushed — this phase is assessment only.
