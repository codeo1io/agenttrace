---
artifact_contract: "ce-review/v1"
created_at: "2026-09-03T00:00:00Z"
title: "Independent adversarial review — cycle 5 batch CU-11..CU-16 (honest coverage, honest cache, honest math)"
summary: "Verdict: pass_with_findings. All six change units do what the cycle records claim, every gate re-ran green first-hand (203/203 workspace tests, fmt clean, clippy -D warnings 0, 10/10 check scripts), and the three headline reproducers are inverted live on the operator corpus: default --audit now covers 1411/1411 sessions at $702.3671 with disclosure keys (was a silent 20-session sample at $3.44), the session cache is pruned to 721 entries / 0 dead paths / 9,340,395 bytes and stays byte-identical across further runs, and the docs contract fails red on the HEAD guide and passes green on the tree. Seven findings, none blocking: two MEDIUM partial-remediation honesty gaps inside the new code — (1) --sample's disclosure says 'newest N' but samples whatever --sort/--order selects (live: --sample 5 --order asc audits the five OLDEST sessions, cost $0.0, labeled 'newest'), and (2) data_health_scoped conflates sessions with source files, so on mixed corpora (SQLite/.db multi-session sources) out_of_scope is undercounted — live at --range 7d: parsed=572 > discovered=364 forces out_of_scope=0 while only 161 of 364 sources are in scope (~203 out-of-range files hidden; the exact F8-2 class CU-12 claimed to close). Plus three LOW (--sample silently ignored on non-governance actions; text --compare drops excluded_reason; enforce_entry_bound cannot evict headerless entries) and two INFO (prune trusts exists() across transient mounts; --recommend/--compare JSON shape change is deliberate and pinned)."
keywords: ["agenttrace", "independent-review", "cycle-5", "coverage-disclosure", "sampling", "data-health", "cache-eviction", "float-hygiene", "docs-contract", "zstd"]
run: "dafd34b3940e497f9f1ac234573323ad"
attempt: "3ae5fe2452e14bb6bf2f7145b0bbd9ec"
repo_head: "998ade8827820479069d7d3590082a33fbf80045"
tree_state: "dirty (cycle-5 implementation CU-11..CU-16 uncommitted on HEAD 998ade8; nothing staged/committed/pushed, per delegation policy)"
---

# Independent adversarial review — cycle 5 (CU-11..CU-16)

Reviewed against: the cycle goals in
`docs/stewardship/2026-09-03-cycle5-prioritization.md` §1/§5 (red→green
acceptance per unit) and the cycle-5 stewardship request
(`docs/stewardship/2026-09-03-cycle5-stewardship-request.md`, incl. its
`must_remain_separate` hints); the implementation record
(`docs/stewardship/2026-09-03-cycle5-implementation-record.md`);
`ROADMAP.md` hardening-lane acceptance criteria for F8-1/F8-2/F8-3/
F8-5/F8-6/F8-7/F8-8; the security boundaries the roadmap pins
(offline by default, no new dependencies, untrusted-input containment);
durability/recovery requirements (atomic cache writes, orphan-temp
sweep, schema versioning, derivable-cache recovery); and the test
evidence claimed by the implement, targeted-tests, and full-tests
phases.

Routing disclosure: no compound-engineering router or ce-* skill is
installed in this session (probed `which ce ce-code-review
compound-engineering`, `~/.agents/skills`, `/usr/local/bin` — only
agent-reach exists), consistent with the disclosure every prior phase
of this run made. The narrowest historical match (ce-code-review) is a
subagent workflow this harness cannot host, so the review ran
in-thread with adversarial, correctness, security, reliability,
testing, and api-contract lenses applied by one reviewer; **every
load-bearing claim was re-executed first-hand on this machine** — no
finding below rests on the phase records alone.

## Verdict

**pass_with_findings.** The batch advances: every acceptance criterion
is either met or met-with-a-disclosed-residual (the two MEDIUMs below),
all five gates pass first-hand, the pass-8 headline reproducer is
inverted on the operator's real corpus, and the batch respected every
separation hint (no Cargo.lock churn, no new dependencies, no workflow
edits, pricing.rs touched only for the finiteness guard). The findings
are quality-of-honesty residuals in the *new* code, not regressions,
and none blocks the commit gate; both MEDIUMs are cycle-6 candidates.

## What was independently re-verified (all first-hand, 2026-09-03)

Gates, re-run from scratch this turn:

- `cargo fmt --all --check` → clean.
- `cargo test --workspace` → 13+6+2+70+7+65+40 = **203 passed, 0
  failed** (baseline 189; +14 new tests, each mapped to a unit below).
- `cargo clippy --workspace --all-targets -- -D warnings` → 0
  errors/warnings.
- All ten `scripts/ci/check-*.sh` with
  `AGENTTRACE_BIN=target/release/agenttrace` → **10/10 exit 0**
  (release binary confirmed current with the tree: `cargo build
  --release` no-op, `--help` shows `--sample`).

CU-11 (F8-1) — inverted live: `--audit -f json --range all` →
`audited_sessions=1411, total_sessions=1411, excluded_reason=null,
total_estimated_cost=702.3671` (the record's 1410/$701.0454 a day
earlier is consistent corpus growth). `--sample 5` → 5/1411 with
reason; `--sample 0` → exit 1 loudly; `--limit 2000` no longer changes
any total; `--delivery-evidence`/`--mcp-governance`/`--recommend`/
`--compare` all carry the coverage keys (spot-checked each); markdown
renders the leading `(auditing N of M sessions)` line. The Go-flag shim
knows `--sample` takes a value (main.rs:736) — the live-bug fix the
record discloses is real (`--sample 20 -f json` keeps `-f json`).
Audit-totals-equal-overview-totals is pinned by
`governance_audit_matches_overview_totals_and_discloses_coverage`
(entrypoints.rs:164).

CU-12 (F8-2) — partially inverted (see finding 2): `discovered` is now
loader-sourced and range-independent (364 across 1d/7d/30d/all, live),
`out_of_scope` is a separate field, and `--range 1d` reports
`74/364 parsed, 290 outside range/filters` where the old code reported
`discovered=71`. The unit test
`data_health_discovered_is_range_independent_and_splits_out_of_scope`
(discovery_contract.rs:2145) covers the 1-file-1-session case.

CU-13 (F8-3) — inverted live and **idempotent**: operator cache now
721 entries / 333 dirs / 0 dead paths / 9,340,395 bytes (from 1,487
entries / 761 dead / 10,530,891 bytes at prioritization), and a further
full run leaves it byte-identical (no thrash, no regrowth). Save stays
atomic (temp + rename, session_cache.rs:649-651), the orphan sweep is
untouched, no schema bump is needed (the format is unchanged; pruning
only removes entries). Persistence across runs is pinned by
`cache_dead_paths_are_persisted_away_across_runs`
(discovery_contract.rs:2210).

CU-14 (F8-5/F8-6) — `json_float` (reports.rs:1462) renders inf/-inf/NaN
as `null` (unit test reports.rs:2728); `convert_litellm` (pricing.rs:434)
rejects entries whose ×1e6-scaled rates are non-finite (unit test with a
hostile 1.797e308 fixture); `DataHealth.non_finite_costs` counts them
and forces confidence `low` (discovery_contract.rs:2266). The divergent
percentile copy is gone from reports.rs and a source-level test forbids
re-introduction (reports.rs:2742). Live: a 1e308-per-million override
file produces finite-huge totals (`2e+302`) with no panic — the writer
is total.

CU-15 (F8-7/F8-8) — red→green independently replicated: the script's
four contract greps all fail against `git show HEAD:` versions of the
guide/README (schema-6 absent, "refreshed automatically" present,
"schema 4" present, README note absent) and `bash
scripts/ci/check-docs-commands.sh` exits 0 on the tree. The script
greps the real constants out of session_cache.rs, so the pin cannot
drift. README en + zh both carry the flags-before-positional note with
failing/succeeding examples — which this reviewer then hit for real by
accident (`agenttrace file.jsonl --overview -f json` silently produced
a text report, exit 0): the trap is real and now documented.

CU-16 (stretch) — live: a `28 B5 2F FD` file fails with the named,
actionable zstd error and exit 1 (`zstd_rollouts_fail_with_a_named_error_not_generic_utf8`,
discovery_contract.rs:2306); no new dependency.

Security boundaries — no new dependencies (Cargo.lock absent from the
diff), no new network paths (the guide now states the offline truth the
loader test pins), the zstd check is a 4-byte prefix comparison before
any UTF-8 decode, disclosure strings are counts or escaped in HTML
(`escape_html(disclosure.trim_end())`, main.rs:566), and the compare
branch's double-serialization only parses the crate's own output.

Durability/recovery — atomic rename retained; prune marks the cache
dirty so the shrink persists on the next save (verified live);
`save_session_cache` errors remain swallowed at call sites
(`let _ =`, discovery.rs:234) — pre-existing behavior, unchanged.

## Findings

### F5-1 (MEDIUM) — `--sample` discloses "newest N" but samples whatever `--sort`/`--order` selected

`crates/agenttrace-cli/src/main.rs:247-253` (governance branch) and
`main.rs:290-296` (compare branch): sampling runs
`matched.into_iter().take(sample)` **after** `prepare_cli_view` has
sorted per the user's `--sort`/`--order` (main.rs:897-918), yet the
reason string hardcodes `sampled newest {sample} of …` (main.rs:251,
294). The help text, README, and guide all promise "the newest N
sessions".

Live reproducer (operator corpus, release binary, `--audit -f json
--range all`):

| invocation | audited | total_estimated_cost | disclosure says |
|---|---|---|---|
| `--sample 5` | 5/1411 | 0.5577 | "sampled newest 5" (true) |
| `--sample 5 --order asc` | 5/1411 | **0.0** | "sampled newest 5" (**false** — oldest 5) |
| `--sample 5 --sort cost --order desc` | 5/1411 | **172.3847** | "sampled newest 5" (**false** — 5 most expensive) |
| `--sample 5 --sort turns` | 5/1411 | 167.5921 | "sampled newest 5" (false) |

This is the F8-1 honesty class (an undisclosed selection rule wearing a
false label) resurfacing inside the fix, scoped to non-default sort
flags — hence MEDIUM, not HIGH. Fix is small: sort a copy by
`recent`/`desc` for sampling, or make the reason name the active sort.
None of the 14 new tests covers a non-default `--sort` with `--sample`.

### F5-2 (MEDIUM) — `out_of_scope` conflates sessions with source files; hides out-of-range files on mixed corpora

`crates/agenttrace-core/src/insights.rs:328`
(`out_of_scope = discovered.saturating_sub(parsed + parse_failures)`)
mixes units: `discovered` counts **files**
(`discovery.rs:189, files.len()`), while `parsed` counts **sessions**
including the SQLite/.db-backed sessions appended after the file loop
(`discovery.rs:237`). Each multi-session source inflates `parsed` by
more than one and silently consumes out-of-scope budget.

Live reproducer (operator corpus, `--overview -f json`):

| range | discovered | parsed | reported out_of_scope | true in-scope sources | hidden |
|---|---|---|---|---|---|
| all | 364 | 1411 | 0 | 364 | 0 (correct) |
| 7d | 364 | 572 | **0** | **161** (verified via `--sessions --limit 5000`: 572 sessions from 125 `.jsonl` + 36 `.db` sources) | **~203 files** |
| 1d | 364 | 74 | 290 | **38** | 36 |

At `--range 7d` the phrase renders "572 sessions from 364 sources; 0
skipped" while ~203 out-of-range files are invisible — exactly the
"parse failures/exclusions hidden by the ranged denominator" defect
class F8-2/CU-12 was accepted to close ("Parse coverage N/M is true for
every range"). The acceptance is fully met only for corpora where one
file yields one session (the shipped test's shape). Fix direction:
count in-scope sources (or track per-source parsed counts) rather than
subtracting session counts from file counts; until then the field is a
lower bound and should be labeled approximate.

### F5-3 (LOW) — `--sample` is silently ignored on non-governance actions

`crates/agenttrace-cli/src/main.rs:158-160` validates `--sample 0`
globally (so `--overview --sample 0` errors), but
`--overview --sample 5` (also `--sessions`, `--doctor`, `--search`)
accepts the flag and ignores it with no note — the very
"flag silently does nothing" hygiene class (F8-8) this cycle documents
in the README. Verified live: overview ran all 74 sessions, stderr
carried only the `--limit` note. Reject or warn when `--sample` is set
without a governance-class action.

### F5-4 (LOW) — text `--compare` drops `excluded_reason` from the human disclosure

`crates/agenttrace-cli/src/main.rs:310-315` prints only
`(auditing N of M sessions)` for non-JSON compare, while the
governance-class text path appends `; {reason}` via
`audit_coverage_phrase` (main.rs:582-594) and the compare **JSON**
carries the reason (verified live: `--compare --sample 2` text shows no
reason, JSON shows `sampled newest 2 of 1411`). One contract, two
disclosure depths.

### F5-5 (LOW) — `enforce_entry_bound` cannot evict headerless entries

`crates/agenttrace-core/src/session_cache.rs:618-621`:
`filter_map(cached_entry_header(...))` skips entries with no decodable
`mod_time`, so a cache dominated by headerless raw entries could exceed
`MAX_SESSION_CACHE_ENTRIES` with nothing evictable. Latent only —
current writers always emit headers — but the bound is then best-effort
and the doc ("bounded at 20,000 entries") is stronger than the code.

### F5-6 (INFO) — load-time pruning trusts `Path::exists()` across transient unavailability

`session_cache.rs:395-423` prunes every entry whose path `exists()` is
false at load. A corpus on a temporarily unavailable mount (NFS stall,
not-yet-mounted drive, permission change) loses its cache entries and
the next save persists the deletion; recovery is a full re-parse. This
trade-off is explicitly blessed by the roadmap acceptance ("the cache
is derivable; pruning costs only re-parse time") and is acceptable for
local corpora — recorded so it is a decision, not an accident.

### F5-7 (INFO) — `--recommend` / `--compare` JSON shapes deliberately broke

`--recommend` is now `{"recommendations": [...]}` and `--compare` now
`{"sessions": [...], audited_sessions, total_sessions, excluded_reason}`
(main.rs:262-265, 299-309). Breaking, but disclosed in the
implementation record, pinned by the updated
`check-rust-real-cli-smoke.sh` assertions, consumed nowhere else
in-repo, and the crates are `publish = false` (no semver surface).
Downstream automation reading the old bare arrays will need the one-line
adjustment; the release notes / PR description should call it out.

### Notes (not findings)

- The record's CU-13 before/after (737→721 entries,
  9,366,610→9,340,395 B) documents the *second* load cycle; the headline
  shrink (1,487→~737, 10.53 MB→9.37 MB) happened during the phase's own
  first run and is only derivable by combining records. The
  prioritization and implementation records together contain the full
  evidence chain, so this is a readability nit, not a gap.
- P3-5 ("`--limit` and gate scoping") is genuinely closed by CU-11:
  live `--overview --limit 2` → `recent_sessions` capped at 2,
  `summary.total_sessions` still 74, stderr note printed. The roadmap
  item can close with this batch.
- The demo golden stayed byte-identical (parse-coverage phrase
  byte-compatible when nothing is out of scope;
  `check-deterministic-output.sh` green first-hand).
- Separation hints all honored: `git status` shows no Cargo.lock, no
  workflow, no dependency churn; pricing.rs carries only the
  `convert_litellm` guard.

## Disposition recommendation

Proceed to final validation / commit with this batch. File F5-1 and
F5-2 as cycle-6 hardening items (they are honesty debt on the two
surfaces cycle 5 itself introduced: sampling disclosure and
scope accounting — the natural first entries for the next assessment
fold-in); F5-3/F5-4 are S-effort riders; F5-5 can ride CU-13's next
touch; F5-6/F5-7 are recorded decisions.
