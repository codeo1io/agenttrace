# Adversarial repository assessment — 2026-09-02 (fifth pass)

- **Scope:** repository at `e005952` plus the uncommitted cycle-1 working tree (the offline-pricing +
  arithmetic-hardening batch). Verified before starting: `git status` matches the tree passes 3–4 reviewed,
  with the implementation-review remediations now applied (saturating sums in `lib.rs`/`parser.rs`/
  `reports.rs`/`insights.rs`/`governance.rs`/TUI, vendored `pricing_snapshot.json`, `--lang` validation,
  plugin-version CI guard, committed adversarial JSONL corpus).
- **Relation to prior passes:** nothing re-derives F1–F19 (pass 1), N1–N10 (pass 2), P3-1…P3-9 (pass 3) or
  P4-1…P4-8 (pass 4). Prior findings appear only where their **status changed** because cycle-1 landed, and
  each such item carries a fresh end-to-end reproduction from this pass.
- **Baseline re-confirmed at start:** `cargo test --workspace` → 159 passed / 0 failed;
  `cargo clippy --workspace --all-targets` → 0 warnings; all ten `scripts/ci/check-*.sh` pass when given
  `AGENTTRACE_BIN` (the four output-contract scripts were run with the release binary, matching `ci.yml`).
- **Method disclosure:** the work order names the compound-engineering router and a `ce-*` skill for
  adversarial assessment; no `ce-assess`/`ce-code-review` SKILL.md is installed in this delegate environment
  (only leftover run artifacts under `/tmp/compound-engineering-1000`), so the ce-code-review adversarial
  methodology was applied in-thread, as the fleet's prior phases recorded. Reproducers: `/tmp/at-assess/`.
- **New tooling this pass:** a 7,500-run UTF-8-safe mutation fuzzer over the committed corpus
  (`/tmp/at-assess/fuzz_mutate.py`) and an opencode SQLite generator
  (`/tmp/at-assess/repro_sqlite_overflow.py`).

---

## Actionable findings

### P5-1 — HIGH (correctness, reliability; H1 closure claim falsified): the opencode SQLite ingestion path still panics in debug — the freshly committed acceptance corpus and test cannot see it

`CHANGELOG.md:7` now claims token aggregation "can no longer print negative token totals or panicked and
absurd costs", and `docs/decisions/2026-09-02-cycle-1-batch-selection.md` scopes H1 as covering session logs
generally. The hardening landed in `parser.rs`, `lib.rs`, `reports.rs`, `insights.rs`, `governance.rs` and the
TUI — but not in `sqlite_sessions.rs`, which accumulates per-message usage with plain checked-at-runtime `+=`:

```rust
// crates/agenttrace-core/src/sqlite_sessions.rs:410-413
agg.input_tokens += input;      // i64 += i64 — overflow panics in debug
agg.output_tokens += output;
agg.cache_read_tokens += cache_read;
agg.cache_write_tokens += cache_write;
```

The new acceptance test (`crates/agenttrace-core/tests/discovery_contract.rs:26`,
`generated_adversarial_corpus_stays_bounded_and_non_negative`) loads only
`testdata/generated/adversarial/` — four JSONL files. There is no SQLite fixture, so the acceptance
criterion "no panic on adversarial logs" is enforced on a strict subset of ingestion surfaces.

[REPRODUCED] (`/tmp/at-assess/repro_sqlite_overflow.py`, temp `$HOME`, isolated cache):

```console
$ HOME=<tmp> XDG_CACHE_HOME=<tmp> ./target/debug/agenttrace --overview -f json
thread 'main' panicked at crates/agenttrace-core/src/sqlite_sessions.rs:410:5:
attempt to add with overflow          # exit 101, no report at all
```

Trigger: an opencode `~/.local/share/opencode/opencode.db` with a session whose two assistant messages each
carry `tokens.input = 9223372036854775807` (two legal in-range values; no malformed JSON needed).
Release builds wrap silently instead. Lineage: N1/P3-3 flagged this class pre-implementation; the new fact is
that the fix batch claims the class closed while this surface was skipped, and the acceptance net added to
keep it closed does not cover it.

**Fix:** apply the same `saturating_add` treatment to `SqliteSessionAgg` accumulation
(and `:403`), clamp the row-level columns at `sqlite_sessions.rs:179-182`, and extend the committed corpus
with an opencode SQLite fixture so `discovery_contract.rs` exercises this path.

---

### P5-2 — HIGH (correctness; contradicts CHANGELOG): the SQLite `number_as_i64` still wraps u64 → negative i64, and reports print negative token counts

`parser.rs:3576-3599` fixed exactly this bug this cycle, with a comment explaining the clamp. The copy in
`sqlite_sessions.rs` was not updated:

```rust
// crates/agenttrace-core/src/sqlite_sessions.rs:590-600
number.as_u64().map(|n| n as i64)   // 18446744073709551615 -> -1
```

[REPRODUCED]: opencode DB, one assistant message with
`tokens.input = 18446744073709551615` (u64::MAX, valid JSON):

```console
$ HOME=<tmp> ./target/debug/agenttrace --latest -f json   # exit 0
"tokens": { "cache_read": 0, "cache_write": 0, "input": -1, "output": 1, "total": 0 }
```

`"input": -1` in a shipped report directly contradicts `CHANGELOG.md:7` ("negative usage values are clamped,
and reports can no longer print negative token totals") and the philosophy asserted by the new acceptance
test. Same lineage as P5-1 (N1's class); reported because the closure claim is now on record.

**Fix:** clamp exactly as `parser.rs:3583-3592` does (`n.min(i64::MAX as u64) as i64`), ideally by sharing one
`number_as_i64` instead of the two diverging copies (P3-8's dedup theme).

---

### P5-3 — MEDIUM (maintainability, release integrity): `PRICING_SNAPSHOT_DATE` has no automated tie to the snapshot it labels

```rust
// crates/agenttrace-core/src/pricing.rs:16
const PRICING_SNAPSHOT_DATE: &str = "2026-09-02";
// crates/agenttrace-core/src/pricing_snapshot.json (first key)
"_snapshot": { "date": "2026-09-02", "models": 2458, ... }
```

`scripts/pricing/update-snapshot.sh` regenerates the file and prints the date, but the only sync mechanism for
the const is a comment telling the maintainer to update it manually (`pricing.rs:13-14`). Nothing reads
`_snapshot.date` — verified: the only consumers of the const are the label at `pricing.rs:108` and a label
test at `pricing.rs:1277` that asserts against the const itself. This cycle added
`scripts/ci/check-plugin-version.sh` precisely to prevent this drift class for `plugin.json` ↔ `CHANGELOG`
(wired at `ci.yml:106-107`), so the snapshot date is the one dated artifact left unguarded. After an
unaccompanied `update-snapshot.sh` run, every report footer and `pricing_source` field would assert a false
catalog date with no CI signal.

**Fix:** unit test asserting `PRICING_SNAPSHOT_JSON`'s `_snapshot.date == PRICING_SNAPSHOT_DATE` (parse the
first key with `serde_json`), or derive the label from the file at parse time.

---

### P5-4 — MEDIUM (documentation, privacy): the governance guide still documents the automatic network refresh that cycle-1 removed — now in direct contradiction with PRIVACY.md

```text
docs/guides/governance-reports.md:51
"Pricing data is cached locally. A cache older than 24 hours is refreshed automatically when a price lookup
starts; if the refresh fails, the stale cache remains usable..."

PRIVACY.md:7 (updated this cycle)
"agenttrace runs fully offline by default. ... no report or test path contacts the network."
```

Both statements are current in the same tree. Code side, `load_catalog_for_current_env`
(`pricing.rs:255-263`) serves any cache as-is regardless of age and never downloads, so the guide describes
behavior that no longer exists. N6 lineage; the contradiction pair is new because PRIVACY.md was rewritten
this cycle while the guide was left behind — a reader reconciling the two cannot tell which is true.

**Fix:** rewrite the guide paragraph to match the cache/snapshot/stale semantics and the
`--update-pricing`-only network path; add the guide to whatever check keeps PRIVACY honest.

---

### P5-5 — LOW (hygiene, privacy): `.hermes/` conductor metadata is untracked and unignored, one `git add -A` away from the public repo

`git status` shows `?? .hermes/`; `.gitignore` (edited this very cycle for F14 cleanup) has no `.hermes/`
entry. The directory contains automation state including
`.hermes/plans/autonomy-prop_a9630aba58334018.md`, which records internal workflow instructions ("Push
feature branches to the git remote named `fork` (codeo1io fork), never to `origin`"). The same status output
shows `?? docs/stewardship/2026-09-02-roadmap-cycle2-update.diff` — a raw diff committed as documentation.
None of this belongs in a public release surface, and the tree is one batch-add away from publishing it.

**Fix:** add `.hermes/` to `.gitignore`; decide deliberately which `docs/` artifacts are release surfaces.

---

### P5-6 — LOW (security, supply chain; unfixed instance of F9's class): `install.ps1` installs release assets with no checksum

```powershell
# install.ps1:51
Invoke-WebRequest -Uri $url -OutFile $tmp   # no hash check before expand/use
```

F9 flagged `install.sh` for verifying no checksum; the PowerShell installer has the identical gap and was not
in F9's remediation scope. A compromised CDN mirror or MITM on the GitHub asset fetch yields arbitrary code
executed in the user's PATH. Fix: embed per-release SHA-256 in the script (or fetch from the release API's
`digest` field) and verify before use.

---

## Clean surfaces verified this pass (negative results, with method)

- **Offline claims are true.** `unshare -rn cargo test --workspace` → 159/159 passed with no network
  namespace; `--demo --overview -f json` runs in ~70 ms cold-cache; `--update-pricing` under `unshare -rn`
  exits 1 with a clear `Dns Failed` message and no cache write on failure paths.
- **Snapshot data audit.** All 2,458 entries in `pricing_snapshot.json`: 0 negative costs, 0 per-token
  prices above $1 (max $0.135/$0.54), `_snapshot.date` == `PRICING_SNAPSHOT_DATE` today, meta-key ignored
  correctly by `convert_litellm` (fails `LiteLlmModel` deser, skipped).
- **Mutation fuzz.** 30 batches × 50 UTF-8-safe mutated files (byte flips, `1e309`, `i64::MAX`/`u64::MAX`
  substitutions, emoji inserts, line truncation/duplication) drawn from the committed corpus, each run
  through `--overview/-f json`, `--waste`, `--audit`, `--doctor`, `--context-trends` on the debug binary:
  0 panics (all exits in {0,1,2}).
- **TUI under PTY on the adversarial corpus** (`script -qec ... -d testdata/generated/adversarial`):
  exits 0, no panic.
- **HTML reports** escape log-derived strings via `html_escape` throughout (`reports.rs:570-1097` spot
  check) — N8's markdown gap does not extend to HTML.
- **All ten `scripts/ci/check-*.sh` pass** with `AGENTTRACE_BIN=target/release/agenttrace` (the four
  output-contract scripts, plus plugin-version, manifests, release surfaces, real-CLI smoke).
- **`--lang` → TUI handoff** is consistent: `main.rs:575-578` and `app.rs:65-66` accept the same alias set.
- Remaining `.sum()`s audited (`lib.rs:1066-1078`, `governance.rs:330/610/928`, TUI `explorer.rs:667/875`,
  `presentation.rs:3609`, `shared.rs:239`): f64 costs, usize counts, or i32 health — no i64 token overflow.

## Status of prior findings re-verified this pass

| ID | Status | Evidence this pass |
|---|---|---|
| N1 (sqlite ingestion unhardened) | **still open after H1 landed** | fresh end-to-end repro: debug panic exit 101 at `sqlite_sessions.rs:410` + `"input": -1` report (P5-1/P5-2) |
| N6 (guide auto-refresh claim) | still open, now contradicts PRIVACY.md:7 | P5-4 |
| N9 (`--version` behind `--lang` validation) | still open | `--lang fr --version` → `unsupported --lang value 'fr'`, exit 1, no version |
| P4-1 (TUI panics when stdout is not a tty) | still open | `./target/release/agenttrace </dev/null` → exit 101, `failed to initialize terminal: Os { code: 6 ... }` |
| F2/F3/F5 (network + cache nondeterminism) | **fixed by cycle-1** | `unshare -rn cargo test` 159/159; determinism script passes; `load_catalog_for_current_env` never writes |
| F12 (`--lang` validation) | fixed (N9 caveat stands) | `report_language` errors; TUI aliases match |
| F14 (`.gitignore` cruft) | fixed, but `.hermes/` gap remains | P5-5 |
| F19 (plugin version) | fixed | `check-plugin-version.sh` passes and is wired at `ci.yml:106-107` |

## Recommended fix order

1. **P5-1 + P5-2** — one saturating/clamping pass over `sqlite_sessions.rs`, plus an opencode SQLite fixture
   in the committed corpus (keeps H1 closed).
2. **P5-3** — snapshot-date sync test (three lines, prevents a silent lying label).
3. **P5-4** — guide paragraph rewrite (privacy-facing consistency).
4. **P5-5, P5-6** — ignore `.hermes/`; checksum in `install.ps1`.
