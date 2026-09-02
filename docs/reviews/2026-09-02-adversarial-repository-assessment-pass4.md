# Adversarial repository assessment — 2026-09-02 (fourth pass)

- **Scope:** repository at `e005952` plus the uncommitted cycle-1 working tree (identical to pass 3's tree —
  verified by `git status` before starting). This pass attacks surfaces pass 3 did not open: the **TUI crate
  under a real PTY**, the **CLI argument-parsing shim**, **npm/homebrew release artifacts**, **CI smoke
  harnesses**, and a **full documentation command sweep**.
- **Relation to prior passes:** nothing here re-derives F1–F19 (pass 1), N1–N10 (pass 2) or P3-1…P3-9
  (pass 3). Prior findings appear only in the status table, where re-verified.
- **Baseline re-confirmed at start:** `cargo test --workspace` → 159 passed / 0 failed;
  `cargo clippy --workspace --all-targets` → 0 warnings. Debug binary `target/debug/agenttrace`.
- **Method disclosure:** compound-engineering router + `ce-code-review` skill loaded from
  `~/.hermes/skills/software-development/compound-engineering/`; this harness has no subagent dispatch, so
  all lenses ran in-thread (single-reviewer IDs, no independent corroboration).
- **New tooling this pass:** a Python PTY driver (`/tmp/pty_drive.py`) that spawns the real TUI on a pty and
  injects keystrokes — the same coverage class as the repo's own disabled `expect` harness.
- **[REPRODUCED]** = triggered end-to-end; reproducers under `/tmp/at4/` (plus pass 3's `/tmp/at3/`).

---

## Actionable findings

### P4-1 — HIGH (reliability, DX): the default TUI launch panics with exit 101 whenever stdout is not a TTY

```rust
// crates/agenttrace-tui/src/app.rs:71-76
fn run_with_app(app: App) -> anyhow::Result<()> {
    let mut terminal = ratatui::init();   // panics on ENXIO when stdout is not a tty
    ...
}
```

`ratatui::init()` panics instead of returning an error, and `run_with_app` already returns
`anyhow::Result<()>`, so a graceful message was available for free.

[REPRODUCED]:

```console
$ ./agenttrace < /dev/null > /tmp/out 2>/tmp/err; echo $?
101
$ cat /tmp/err
thread 'main' panicked at .../ratatui-0.30.2/src/init.rs:366:16:
failed to initialize terminal: Os { code: 6, kind: Uncategorized, message: "No such device or address" }
```

This is the **README quickstart command** (`agenttrace`, README.md:99 and README.zh-CN.md) and it fails in
every non-tty context: pipes (`agenttrace | cat`), `--demo` in cron/CI, `docker run` without `-t`,
non-interactive SSH, IDE consoles. A documentation sweep (below) hit this panic from **five different
documents**. Exit 101 + a Rust backtrace note is also what a CI log shows, which misleads users into filing
a crash bug.

**Fix:** construct the terminal manually (`Terminal::new(CrosstermBackend::new(io::stdout()))?`) or guard
with `stdout().is_tty()` and print `agenttrace: not a terminal — use --overview for non-interactive output`,
mapping to a normal error exit (1).

---

### P4-2 — MEDIUM (DX, correctness): the Go-flag shim silently discards every argument after the first positional

```rust
// crates/agenttrace-cli/src/main.rs:498-501
if is_go_flag_positional(&arg) {
    out.push(arg);
    break;                    // everything after this point is thrown away, unseen by clap
}
```

`go_flag_compatible_args` reproduces Go's `flag` semantics by truncating argv at the first non-flag
argument, and `flag_takes_value` (main.rs:514-549) is a hand-maintained list of the 24 value-taking flags.
The truncation is deliberate (there is a unit test at main.rs:1076) but **silent** — no warning, no error,
empty stderr.

[REPRODUCED]:

```console
$ agenttrace /tmp/at4/a.jsonl --latest -f json | head -1     # flags dropped, text report
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
$ agenttrace /tmp/at4/a.jsonl -f json --lang zh 2>&1 >/dev/null   # stderr: empty
$ agenttrace -- /tmp/at4/a.jsonl /tmp/at4/b.jsonl --sessions | head -1
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━   # 2nd file AND --sessions dropped
```

The same command with flags first works perfectly (`agenttrace --latest -f json a.jsonl` → JSON). Users
have no signal that their flags were ignored. The hand-maintained `flag_takes_value` list is the structural
trap: any future value-taking flag forgotten there turns its *value* into `args.path` ("session path does
not exist: 2026-01-01").

**Fix:** keep the shim, but when truncation discards any argument beginning with `-`, print one stderr line
(`ignoring arguments after <path>; put flags before the positional path`). Longer term, consider clap's
native interspersed parsing — the Go shim only matters for `agenttrace file.jsonl` alone.

---

### P4-3 — MEDIUM (DX): filter-only invocations silently launch a full-screen TUI

`has_session_action` (main.rs:912-924) treats `--range/--project/--source/--model-filter/--health/--cost`
as *filters*, not actions, so with no report flag the code falls through to the TUI
(main.rs:196-203).

[REPRODUCED] — this is the **first example** in `docs/guides/governance-reports.md:12`:

```bash
agenttrace --range 30d --project storefront --source claude_code
```

Under a PTY this renders the full-screen TUI (`\x1b[?1049h`, "Look here first" — verified with the PTY
driver); under a pipe it panics per P4-1. A reader who copies the doc's canonical "scope" line into a
script gets either a surprise TUI or a crash. `--help` gives no hint that these flags cannot stand alone.

**Fix:** if scope/filter flags are present without an action, either error ("--range requires a report
action such as --overview") or print the scoped `--sessions` list.

---

### P4-4 — MEDIUM (test, packaging): the published npm tarball contains zero tests — `npm test` passes vacuously

`npm/package.json` `files: ["bin", "lib", "scripts", "LICENSE", "README.md"]`:

- `npm/test/package.test.js` exists and passes in-repo (1 test) but is **not shipped** — `npm pack
  --dry-run` lists 5 files, no `test/`. Extracting exactly the tarball contents and running `npm test`
  yields `pass 0 / fail 0` (vacuous green).
- `files` also lists `lib`, a directory that does not exist anywhere in the repo — dead manifest entry
  confirmed by the pack listing.

So the only JavaScript test in the repository — the one covering `install.js`, the network-facing installer
— never runs against the artifact users actually download, and a tarball-based CI check would report
success with no coverage.

**Fix:** add `test` to `files` (and remove `lib`), or move the test invocation to a repo-level CI step and
drop the `scripts.test` entry; add an `npm pack --dry-run` assertion to `check-release-surfaces.sh`.

---

### P4-5 — MEDIUM→LOW (test coverage): the PTY-driven TUI smoke harness is complete, works, and is permanently disabled — and it does not need real data to run

`scripts/ci/check-rust-tui-real-smoke.sh` is a genuine PTY harness (spawns on a pty, sends Enter, →, v, f,
Ctrl-K, ?, q, asserts screen text). It is gated by `.github/workflows/ci.yml:84`
`if: env.AGENTTRACE_TUI_REAL_DIR != ''`, a variable set nowhere in the workflow or repository, so it has
never executed in CI (this is N5, re-verified).

New evidence this pass: the harness's *reason* for gating — it defaults `source_dir` to
`$HOME/.pi/agent/sessions`, i.e. private maintainer logs — is avoidable. I drove the equivalent key
sequence against `--demo` with my own PTY driver (28 keystrokes across views, governance tabs, the filter
modal, search, help, paging, detail): **rc=0, no panic**, and the same under an empty dir, a
garbage-only dir, and a nonexistent dir. The demo corpus is deterministic, so the expect script can run
ungated against `--demo` in CI today, with the real-data variant kept as an opt-in.

**Fix:** add an ungated `--demo` PTY smoke job that runs the existing expect script against a temp dir of
exported demo sessions (or add a `--demo-stdout` fixture mode), keeping `AGENTTRACE_TUI_REAL_DIR` as the
opt-in for real logs.

---

### P4-6 — LOW (correctness, test isolation): `AGENTTRACE_SESSION_CACHE_DIR` isolates only the session cache; the pricing cache ignores it

- `session_cache.rs:154` and `doctor.rs:325` honor `AGENTTRACE_SESSION_CACHE_DIR`;
- `pricing_cache_path()` (`pricing.rs:137-139`) calls its own `user_cache_dir()` copy directly and never
  checks the variable, so pricing reads/writes `~/.cache/agenttrace/pricing.json` regardless.

Consequences: CI and tests that set the variable to isolate state still share one pricing cache (racy for
parallel jobs; a job that refreshes pricing perturbates concurrent runs), and the repo's own
`discovery_contract.rs:1550-1578` tests set this variable believing it isolates the process. This is the
first *behavioral* divergence caused by the `user_cache_dir()` triplication recorded as P3-8 — the copies
were byte-identical, but their call sites grew different env handling.

**Fix:** route `pricing_cache_path()` through the same path resolver that honors
`AGENTTRACE_SESSION_CACHE_DIR` (or introduce `AGENTTRACE_CACHE_DIR` for both).

---

### P4-7 — LOW (maintainability, portability): a fourth, differently-shaped home-directory resolver

`history_path()` (`history.rs:25-34`) resolves `AGENTTRACE_HISTORY_DIR` → `XDG_DATA_HOME` →
`HOME/.local/share` → temp dir. That is a **fourth** private home-resolution routine in the same crate,
with different precedence and different fallbacks from the three `user_cache_dir()` copies (P3-8): no
macOS `~/Library/Application Support`, no `USERPROFILE` (so on Windows the preserved-history file lands in
the temp dir — same platform class as P3-1). It also means `history.json` and `sessions.json`/`pricing.json`
can live in three different roots on one machine.

**Fix:** one `paths` module exporting `home_dir()`, `cache_dir()`, `data_dir()` used by all four call
sites; add the Windows fallbacks once.

---

### P4-8 — LOW (robustness, growth): preserved history has no eviction and is rewritten in full

`preserve_derived_history` (`history.rs:36-48`) loads the entire `history.json`, inserts, and rewrites the
whole map; `merge_preserved_history` (`history.rs:50-60`) then materialises every record ever kept into
memory and into the visible session list. There is no cap, no TTL, and no dedupe-by-age, so `--preserve-history`
+ `--include-history` grow without bound (one record per session ever scanned) and each run pays a full
read+write. Errors from the discovery-path write are also discarded (`discovery.rs:242`
`let _ = preserve_derived_history(&sessions);`), so a full disk or permission failure is invisible.

**Fix:** cap the record count (e.g. most-recent N) and surface write failures at least once.

---

## Documentation command sweep (new method this pass)

Extracted every fenced `bash`/`sh`/`console` block from all 25 `docs/**/*.md` + `README*.md` +
`skills/**/*.md` and executed the 37 `agenttrace …` commands against the real binary
(`/tmp/docsweep.py`):

- Extracted and executed **37** `agenttrace …` commands. **27 pass**; **6 fail genuinely** (all via
  P4-1 and P4-3): README.md ×2, README.zh-CN.md ×2, docs/maintainers/launch-kit.md ×1,
  docs/guides/governance-reports.md ×1. The remaining 4 are not defects: 3 use placeholder paths
  (`cursor-export.json`, `path/to/session-or-export.json`) and error clearly; 2 contain `|| true` which my
  shlex-based runner mishandled.
- Every other documented command succeeds and every JSON output parses. `check-docs-commands.sh` covers 12
  `--demo` commands but never the bare `agenttrace` form, which is why P4-1 was never caught by CI.

---

## Verified clean this pass (negative results)

- **Real TUI under a PTY, driven with 28 keystrokes** (`--demo`): views, governance tabs, filter modal,
  search, help, list indices, paging, detail — rc=0, no panic, clean exit on `q`.
- **TUI on hostile inputs:** empty dir, garbage-only dir (non-JSON), nonexistent `-d` path — all rc=0, no
  panic.
- **Guarded `expect("governance initialized")`** (app.rs:1484/1493/1505): `ensure_governance`
  (`get_or_insert_with` at app.rs:1466-1468) always runs first; unreachable.
- **`--version` matches the Homebrew formula's `assert_match "agenttrace v"`** (`agenttrace v0.0.0-dev`);
  the formula is head-only by design and README discloses the not-yet-published state.
- **`flag_takes_value` list is currently complete** against all 24 value-taking flags in `Args` — the risk
  is future drift (P4-2), not a present mis-parse.
- **`AGENTTRACE_SESSION_CACHE_DIR` is honored** by the session cache and doctor (P4-6 covers the pricing
  exception).
- Docs: 27 of 37 documented commands pass outright; all JSON outputs valid (detail above).

---

## Status of prior findings re-verified this pass

| ID | Status | Evidence this pass |
|---|---|---|
| N1 (sqlite ingestion unhardened) | still open | unchanged tree; `sqlite_sessions.rs:590-599/:403/:410-413` |
| N5 (TUI real smoke gated off) | still open | `ci.yml:84`; **upgraded** by P4-5 with a viable ungating path |
| N6 (governance-reports.md auto-refresh claim) | still open | `docs/guides/governance-reports.md:51` |
| N7 (empty-dir trailing space) | still open | reproduced in pass 3 |
| P3-1 (`HOME`-only discovery) | still open | `discovery.rs:51-53`; extended by P4-7 (fourth resolver) |
| P3-5 (`--limit` ignored by `--overview`) | still open | `main.rs:326` |
| P3-8 (`user_cache_dir()` ×3) | still open | **first behavioural divergence found** — see P4-6 |
| P3-2/P3-3/P3-4/P3-6/P3-7/P3-9 | still open | unchanged tree |

---

## Prioritised remediation order

1. **P4-1** — one-line guard or manual `Terminal::new`; unblocks CI/docs/non-tty usage of the quickstart.
2. **P4-2** — stderr warning on truncated flags; removes a silent-wrong-output class.
3. **P4-3** — error (or scoped default action) for filter-only invocations; fixes the doc's first example.
4. **P4-4** — ship the npm test or drop the script; assert on `npm pack` contents in CI.
5. **P4-5** — ungated `--demo` PTY smoke in CI using the existing expect script.
6. **P4-6 + P4-7** — single `paths` module; fixes isolation and the Windows data-dir case together.
7. **P4-8** — history eviction + surfaced write errors.
8. Then pass-3's P3-1/P3-2/P3-3 (still the highest-severity correctness items overall).
