# Adversarial repository assessment — run a24bcf08, phase assess (attempt aa05d0d0)

Executed direct (no ce-* router installed in this environment; disclosed per
campaign convention, matching every prior phase artifact).

Baseline verified this run: `cargo test` green (all 3 crates), `cargo clippy
-- -D warnings` clean, `./target/release/agenttrace --demo --overview -f json`
and `--doctor -d testdata` behave as documented. HEAD = `696206f`, local
`master` 4 commits ahead of `origin/master` (upstream luoyuctl/agenttrace).

## Findings

1. **CRITICAL (stewardship)** — `.github/workflows/ci.yml:21`,
   `release.yml:13,40,117`, `dependency-review.yml:16`: commit `6632014`
   committed `runs-on: self-hosted` into all three workflows, violating
   AGENTS.md rule 3 (keep `ubuntu-latest`; self-hosted is operator-repo-only).
   The commit sits on local `master`, whose upstream is `origin/master` =
   upstream repo: a plain `git push` from `master` ships the runner override
   plus all fork work to upstream (rules 1–3). Fork PR #1 lineage is otherwise
   intentional per `2026-09-03-cycle5-reconciliation.md`, but the tracking
   hazard is live and unremediated.

2. **MEDIUM (reliability/perf)** — `crates/agenttrace-core/src/parser.rs:22`:
   `parse_file` slurps the whole file (`std::fs::read` → `String::from_utf8`,
   ~2× file size peak). Discovery accepts any `*.json/jsonl` under depth 4 with
   no size cap anywhere in the pipeline; a multi-GB or hostile neighbor file in
   a scanned directory stalls/OOMs the TUI and every report action. The session
   cache bounds entries (20k), not bytes.

3. **MEDIUM (supply chain)** — `install.sh:47-53` and `install.ps1` (download
   block): `curl | sh` installer fetches the release binary with no
   checksum/signature verification (only a >1 MB size heuristic), while the
   release publishes `.sha256` sidecars and `npm/scripts/install.js` verifies
   SHA-256 (`checksum mismatch for ...`). Installer channels are inconsistent;
   shell-installer users get no tamper detection.

4. **LOW (correctness/disclosure)** — `crates/agenttrace-cli/src/main.rs:249-251`
   and `292-294`: `--sample` exclusion reason hard-codes "sampled newest N",
   but `matched` is ordered by the user's `--sort` (`--sort name` yields
   name-first, not newest). The disclosure can be factually wrong.

5. **LOW (security/privacy)** — `crates/agenttrace-core/src/governance.rs:746-775`
   with `insights.rs:166-231`: `resolve_project` walks up from session `cwd`
   (log-controlled untrusted input) to the nearest `.git`, then
   `git -C <root> log --all` runs and commit timestamps of arbitrary local
   repos are embedded in delivery/governance reports. No shell/argv injection,
   but untrusted session content selects subprocess targets and can pull data
   from repos outside the audited tree into shared reports.

6. **LOW (perf)** — `insights.rs:166-206`: `resolve_project`/`git_root`
   (≥2 stat probes walking up per call; one `git` subprocess per distinct root
   in `git_commits_by_root`) is recomputed per session per report with no
   memoization → O(sessions × path depth) stat calls + multiple git spawns per
   overview/governance run on large corpora.

7. **LOW (data quality)** — `crates/agenttrace-core/src/sqlite_sessions.rs:180-196`:
   Hermes `state.db` ingestion sets `tool_calls_ok = tool_call_count` and never
   records failures, so Hermes sessions always report 0% tool fail-rate while
   file-backed providers surface real failures; health skews optimistic.

8. **LOW (reliability)** — `crates/agenttrace-core/src/session_cache.rs:24,637-639`:
   cache bound is entry-count only (20,000), no byte bound; entries carry full
   tool-arg maps and directory listings, so `sessions.json` can grow to
   hundreds of MB before the count bound trips, and is re-serialized whole per
   save.

9. **INFO (TUI)** — `crates/agenttrace-tui/src/app.rs:881-951`: background
   loader thread owns the cache while the main thread can `clear_session_cache()`
   (force reload); reloads are guarded by `pending_load`, but clear vs.
   concurrent thread save can race on cache files. Consequence bounded (stale
   cache resurrected / benign rename), no user-data corruption.

10. **INFO (release)** — `dependency-review.yml` fails on the fork
    ("not supported on this repository") — known, recorded as provisional work
    in the cycle-5 reconciliation; version scheme `0.0.0-dev` / npm
    `0.0.0-release` is rewritten by the release pipeline and checked by
    `check-plugin-version.sh`.

## Verified solid (no action)

Untrusted-input hardening (saturating arithmetic, negative clamps, non-finite →
`null` + `data_health.non_finite_costs`, BOM/UTF-16 handling, lone-surrogate
repair with escaped-backslash fix); atomic writes everywhere (unique temp +
rename, corrupt-history quarantine, orphan sweep); read-only, parameterized
SQLite access; consistent `html_escape` in HTML reports and
`go_json_escape` for JSON embedded in scripts; offline-by-default pricing with
dated snapshot pinned by test; `--doctor` double-write is consistent
(`write_output` newline-appended, `write_stdout` raw) with the rest of main;
Go-flag shim forwarding analyzed and harmless; docs commands CI-checked;
adversarial fixture corpus committed and asserted from file through
`data_health`.

## Provisional future work

- Revert `runs-on` to `ubuntu-latest` (or repoint local `master`'s upstream to
  `fork/master`) before any further work on `master`.
- Add a max-file-size guard (named skip reason surfaced in `data_health`).
- Add SHA-256 verification to `install.sh`/`install.ps1` mirroring the npm
  installer.
- Make the `--sample` reason reference the active `--sort` order.
- Memoize `resolve_project`/`git_root` per normalized path; consider
  sandboxing `git -C` roots to discovered session directories.
- Byte-bound the session cache in addition to the entry bound.
- Extract real Hermes tool-failure signal from `state.db` (schema research).
