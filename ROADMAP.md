# agenttrace roadmap

agenttrace is focused on two jobs:

1. Review AI coding agent history across cost, tokens, elapsed time, and tool authority.
2. Diagnose why an agent task ran slowly or regressed.

This roadmap keeps the project pointed at local post-run evidence instead of
becoming a generic observability dashboard.

## Strategic directions

- Broaden reliable local parser coverage for active coding-agent session formats.
- Keep local-first privacy and public asset hygiene explicit in screenshots,
  reports, and launch materials.
- Make first-screen TUI triage faster for cost, tokens, tool failures, latency,
  anomalies, health, and incident evidence.
- Strengthen shareable evidence through terminal text, JSON, Markdown, and HTML
  reports, local baseline comparison, and repeatable CI gates.
- Keep tool authority categories conservative, deterministic, and clearly framed
  as report evidence rather than policy enforcement.
- Keep doctor, install paths, release artifacts, Homebrew, and site surfaces
  consistent with the current project state.
- Improve discoverability around local coding-agent session observability,
  distinct from generic tracing SDKs.
- Route ecosystem feedback into parser, quality, growth, product, or radar lanes
  with clear ownership.
- Track adjacent surfaces such as local session search, multi-agent dashboards,
  token attribution, persistent memory, and upstream log fidelity before
  committing implementation.
- Prefer facts the provider already recorded over facts we re-derive; where a
  provider publishes authoritative totals or schema contracts, read them.

## Current foundation

- One Rust binary provides CLI reports and a ratatui/crossterm TUI.
- Local session caches and SQLite-backed sources load without a hosted service.
- `Detailed`, `Aggregate`, and `Limited` labels expose source capability gaps.
- Tool steps retain timing/status metadata only and omit conversation and tool
  payload bodies.
- Deterministic generated fixtures protect provider parsers and degradation
  behavior.

## Non-goals

Non-goals: hosted prompt storage, billing-grade invoice reconciliation, replacing
agent chat UIs, live tracing while a model is streaming, security enforcement,
release promises, package-publish promises, and internal platform targets.

## Planned work (added 2026-09-02)

Baseline when this section was added: v0.7.1 at HEAD e005952,
`cargo test --workspace` 147/147 passing, `cargo fmt --check` clean. Sources:
the adversarial assessment at
`docs/reviews/2026-09-02-adversarial-repository-assessment.md` and the
ecosystem research pass at
`docs/ideation/2026-09-02-agenttrace-extensions-ideation.md`. Hardening
precedes capability work; every item names acceptance criteria and the
evidence a completing change must show.

Updated 2026-09-02 after cycle 1 and the second research pass. Cycle 1 is
the uncommitted tree on the same HEAD e005952: 159/159 tests passing,
`cargo fmt --check` and `cargo clippy --workspace --all-targets` clean. Two
new sources are folded in below: the second adversarial assessment at
`docs/reviews/2026-09-02-adversarial-repository-assessment-pass2.md`
(findings N1–N10, all reproduced) and the second research pass appended to
`docs/ideation/2026-09-02-agenttrace-extensions-ideation.md` (candidates
8–12).

Updated again 2026-09-02 after assessment passes 3–5 and research passes
3–4, same tree and baseline (159/159 tests, fmt/clippy clean at e005952
plus cycle 1 — no code changed between the pass-2 and pass-5 reviews). New
sources folded in below: pass 3
(`docs/reviews/2026-09-02-adversarial-repository-assessment-pass3.md`,
findings P3-1–P3-9), pass 4 (`…-pass4.md`, P4-1–P4-8), pass 5
(`…-pass5.md`, P5-1–P5-6), research pass 3 (candidates 13–23, appended to
the ideation doc), and research pass 4
(`docs/research/2026-09-02-extensions-research-pass4.md`, candidates 24–27).
Pass 3–5 findings are additions or status changes, not repeats; where a new
finding extends an earlier entry, both IDs are named in that entry. Verified
clean this cycle and not re-listed: offline-by-default behavior under a
network-blocked sandbox, 7,500-run UTF-8-safe mutation fuzzing with zero
panics, the vendored snapshot's price sanity (2,458 entries, none negative
or absurd), and Codex cumulative-usage delta handling.

Updated again 2026-09-02 after cycle 2 and assessment pass 6 plus
research pass 5, on the same HEAD e005952 with the uncommitted cycle-1/2
tree now at 169/169 tests, `cargo fmt --check` and `cargo clippy
--workspace --all-targets` clean (debug and release). New sources folded
in below: the cycle-2 implementation record
(`docs/stewardship/2026-09-02-cycle-2-implementation-record.md`, change
units CU-1..CU-7), pass 6
(`docs/reviews/2026-09-02-adversarial-repository-assessment-pass6.md`,
findings P6-1–P6-4), and research pass 5
(`docs/research/2026-09-02-extensions-research-pass5.md`, candidates
33–36). Competitive posture recorded there: ccusage (18.3k stars, now
`ccusage/ccusage`) parses agenttrace's full source list including Hermes
and pi and ships a statusline beta, but remains usage/cost aggregation
only — the defensible center is diagnosis (loops, latency, health,
governance, TUI drill-down), not accounting.

### Completed in cycle 1 (recorded 2026-09-02)

Preserved history; each entry names the evidence that closed it, as
re-verified independently by the second assessment pass.

- **Offline-by-default pricing** (assessment F2/F3/F5, research candidate 1).
  Closed with: a dated vendored snapshot
  (`crates/agenttrace-core/src/pricing_snapshot.json`), offline as the
  default, a clock-free `pricing_source` label, and `--update-pricing` as
  the only downloader. Evidence recorded: a network-blocked
  `--overview -f json` run succeeds end to end, writes no cache file, and
  reports the bundled snapshot; `cargo test` passes under a dead proxy
  without mutating the user cache; consecutive runs produce byte-identical
  JSON; PRIVACY.md matches observed behavior. Residual, tracked below:
  `docs/guides/governance-reports.md:51` still documents the removed
  24-hour auto-refresh (N6).
- **Parser-path arithmetic hardening** (assessment F1 and implementation
  review #1/#2/#4, partially). Closed for the file parsers, `lib.rs`
  accumulators, reports, governance, insights, and the TUI: a clamped
  `number_as_i64`, saturating aggregation, and adversarial fixtures under
  `testdata/generated/adversarial/`. Evidence recorded: ~590 mutation-fuzz
  runs over all 33 testdata seeds across 11 report actions with zero
  panics. The SQLite ingestion path and `waste.rs:180` were missed and
  remain open as N1/N2/N3 below.
- **CLI and CI surface fixes** (assessment F12/F14/F19). Closed with:
  `--lang` rejects unknown values; `.gitignore` covers generated
  artifacts; `scripts/ci/check-plugin-version.sh` exists and is wired into
  CI (`ci.yml:106-107`). Evidence recorded: the script passes locally and
  runs in CI. Residual, tracked below: `--version` is unreachable when
  `--lang` is invalid (N9).

### Completed in cycle 2 (recorded 2026-09-02)

Preserved history; evidence as recorded in
`docs/stewardship/2026-09-02-cycle-2-implementation-record.md` and
independently re-verified by assessment pass 6.

- **SQLite arithmetic remainder** (assessment F1 residue, pass-2
  N1/N2/N3, pass-5 P5-1/P5-2). Closed with CU-1: the local wrapping
  `number_as_i64` deleted in favour of the hardened shared one,
  `saturating_add` accumulators, clamped Hermes token reads,
  `waste.rs` `saturating_sub`, governance `cost_audit` downgrading
  `confidence` on negative components, and the CHANGELOG over-claim
  corrected. Evidence recorded: red→green guards for the P5-1 overflow
  and P5-2 wrap reproducers; committed adversarial SQLite fixtures
  (`testdata/generated/adversarial/sqlite/{overflow,wrap}.db` loaded
  through the normal discovery path); 169/169 tests in debug and release;
  overflow DB exits 0 (was 101) and `u64::MAX` input reports `i64::MAX`
  (was `-1`). Residual, tracked below: none in this family — but pass 6
  opened P6-1, a release-reachable panic in a different untrusted-input
  path (JSON string repair).
- **TUI launch safety** (pass-4 P4-1). Closed with CU-5: `app.rs:78` bails
  with a "stdout is not a terminal … use `agenttrace --overview`" error
  before `ratatui::init()`. Evidence recorded: guard
  `tui_launch_fails_cleanly_when_stdout_is_not_a_terminal` (exit 101 →
  exit 1 with guidance); re-verified by pass 6 on a piped run.
- **Snapshot-date pin** (pass-5 P5-3). Closed with CU-6: a test pins
  `PRICING_SNAPSHOT_DATE` to the bundled payload's `_snapshot.date`; red
  by construction on drift.
- **Repository hygiene** (pass-5 P5-5). Closed with CU-7: `.hermes/` in
  `.gitignore`.
- **`--version` early-return order** (pass-2 N9, the residual named in
  cycle 1's record). Closed with CU-4: the version early return moved
  above `report_language`; guard `version_wins_over_invalid_lang` (exit 1
  → banner). Residual, tracked below: P6-2 — the CHANGELOG wording still
  over-claims (`--overview --version` is rejected because action
  validation runs first).
- **Trust upstream totals — stored-totals half** (capability candidate 8,
  CU-2/CU-3). The opencode reader prefers stored `cost` and the five
  token columns when present (lenient typed reads; unparseable values
  fall back to derived), records `stored_totals_delta` with
  `stored_session_totals` provenance, snapshots at cache schema v5, and
  unknown-time SQLite sessions stay visible under `--range` with a
  `data_health.unknown_time_sessions` count. The hierarchy and delivery
  columns (`parent_id`, `summary_*`, `time_compacting`, `agent`) remain
  open under candidate 8. Evidence recorded: guards
  `opencode_stored_session_totals_preferred_with_delta`,
  `opencode_unknown_time_session_stays_visible_in_range`, and the
  TEXT-typed-column regression the batch's own DB-mutation fuzzing found.

### Hardening lane

- **UTF-16 escape repair must never slice mid-character** (pass-6 P6-1,
  HIGH). `repair_lone_surrogates` slices `&line[i+2..i+6]` and
  `&line[i+8..i+12]` after a rejected `\u` escape (`parser.rs:3785`,
  `:3791`); when the "hex" bytes contain multi-byte UTF-8 the slice lands
  off a char boundary and the process panics. Reproduced on freshly
  built debug **and release** binaries (exit 101) from
  `{"prompt":"\u中文测试"}` and `{"prompt":"\ud800\u中文测试"}`, and
  reachable through format detection (`parse_jsonl_value_lenient` →
  `jsonl_objects` → every JSONL detector), so `--overview`, `--doctor`,
  `--waste`, `--latest`, `--sessions`, `--diagnostics`, direct `-d`
  files, and directory scans all die; the TUI survives only because its
  loader thread dies ("loader disconnected"). The 7,500-run mutation
  fuzzer never produced a `\u` escape before multi-byte bytes and the
  adversarial corpus contains none — a proven blind spot in the
  acceptance net (the only related test covers `\ud83c` followed by
  ASCII). Acceptance: hex parsed from bytes with ASCII checks (no `&str`
  slicing), or both slice edges guarded by `is_char_boundary`; rejected
  escapes pass through untouched; the adversarial corpus gains
  `\u`-before-multibyte cases. Evidence: the pass-6 reproducers committed
  as fixtures under `testdata/generated/adversarial/`; debug and release
  binaries exit 0 with degraded, labeled output on both reproducers; a
  discovery contract test asserts the file is reported
  unparseable-or-degraded, never fatal.
- **Honest fallback token accounting** (pass-6 P6-4). The
  estimate-bytes÷4 fallback under-counts CJK by ~40–60% and
  `reasoning_chars` stores `event.reasoning.len()` bytes, not characters
  (`lib.rs:555`, `:562`, `:577`). Acceptance: an explicit
  byte-versus-character decision at each estimate site, CJK-aware where
  characters are intended, the estimator named in provenance;
  `reasoning_chars` fixed or renamed to match its unit. Evidence: a CJK
  fixture asserting the estimate within a stated tolerance; a test
  pinning the unit in the field name.
- **Local-calendar day windows** (pass-3 P3-2). `--range today` and its
  alias `--range 1d` are UTC-calendar-day windows, silently dropping
  sessions from earlier in the user's local day. Acceptance: `today`
  computes local midnight, `1d` is a rolling 24-hour window, and both are
  documented as such. Evidence: a fixture stamped `2026-09-02T01:00+09:00`
  is visible to `--range today` under `TZ=Asia/Tokyo`. (The absolute-time
  scoping capability below builds on this.)
- **Self-consistent statistics and deduplicated helpers** (pass-3
  P3-3/P3-8, P3-9). Two divergent `percentile()` implementations
  contradict each other inside one report; ranking helpers are duplicated
  across two TUI files; `count_by_root` in `doctor_directories` is dead
  code. Acceptance: one `percentile` exported from a single module, a test
  asserting anomaly scores and report percentiles agree, duplicate
  helpers hoisted, the dead block deleted. Evidence: a test that fails if
  the two implementations diverge again.
- **Control characters never reach output** (pass-3 P3-4). C0/C1 control
  characters from untrusted session content reach the terminal and every
  report format unfiltered. Acceptance: strings are stripped of C0/C1
  (except `\t` and `\n`) once at the model boundary, closing terminal,
  text, JSON-passthrough, Markdown, and HTML surfaces together. Evidence:
  a fixture with control characters in model and session names produces
  clean output in all four formats.
- **`--limit` and gate scoping** (pass-3 P3-5). `--limit` is silently
  ignored by `--overview`, including the documented `--baseline` CI gate
  recipe. Acceptance: the limit applies in the overview branch or the
  combination errors loudly. Evidence: a CLI test pinning
  `--overview --limit N` behavior.
- **Doctor caches what it parses** (pass-3 P3-6). `--doctor` re-parses
  every uncached file on every run and never writes the session cache,
  making the first-run triage path the slowest command on large corpora.
  Acceptance: doctor reuses the discovery load path so parses populate the
  cache. Evidence: a second doctor run is warm-cache fast on a multi-file
  corpus; a test asserts cache entries exist after a doctor run.
- **Cache and history durability** (assessment F4/F6/F7).
  Acceptance: session-cache entries for deleted files are pruned on save;
  history writes are atomic (temp file plus rename) and a truncated history
  is quarantined rather than silently discarded; history IDs use a
  versioned, toolchain-stable digest. Evidence: a delete-and-rerun test
  shows stale keys removed; an interrupted-write fixture recovers with a
  visible warning; a digest-stability test pins the ID scheme. Extended by
  pass 4 (P4-6/P4-8): preserved history is rewritten in full with no
  eviction (unbounded growth), and `AGENTTRACE_SESSION_CACHE_DIR` isolates
  only the session cache — the pricing cache ignores it, so tests can leak
  into the user cache. Additional acceptance: history eviction above a
  documented size or entry bound; the pricing cache honors the isolation
  variable. Evidence: a growth test writing N history entries then
  asserting a bounded file; an isolation test asserting no writes outside
  the sandboxed cache root. Extended by pass 6 (P6-3): cache persist
  writes through a fixed `<name>.json.tmp` sibling (`session_cache.rs:226`,
  `:518`), so two concurrent agenttrace processes race on the same temp
  file (load self-heals, so the impact is a failed or torn save under
  concurrency). Additional acceptance: a unique temp suffix per writer
  with the same atomic rename. Evidence: a concurrent-persist test (two
  writers, no failed rename, no torn cache). Research pass 5 context:
  Claude Code deletes transcripts after 30 days by default
  (`desktopSessionCleanupPeriodDays`), so the preserved history is
  increasingly the only durable record — the durability docs should cite
  that upstream retention policy.
- **Platform and channel parity — remainder** (assessment F8/F9/F15; F12,
  F14, and F19 closed in cycle 1; extended by pass-3 P3-1/P3-8 and pass-4
  P4-7). Status change, pass 3 (P3-1, HIGH): discovery itself is
  `HOME`-only (`discovery.rs:51-53`, `sqlite_sessions.rs:47`/`:61`), so a
  stock Windows install discovers zero sessions and `--overview` errors
  with "No session files found in" — the larger sibling of the cache-dir
  gap. Acceptance: one shared resolver tries `HOME`, then `USERPROFILE`,
  then `HOMEDRIVE`+`HOMEPATH` and serves discovery, the SQLite sources,
  cache directories, and the history path (a fourth, differently-shaped
  resolver found by P4-7 in `history.rs:25-34`); the hand-rolled
  `user_cache_dir()` copies (`pricing.rs:1113`, `doctor.rs:333`,
  `session_cache.rs:743`) collapse into it with a Windows
  `%LOCALAPPDATA%` branch; `install.sh` verifies a published checksum the
  way the npm channel does and `install.ps1` verifies a SHA-256 before
  executing its download (pass-5 P5-6: `install.ps1:51` has none); CI
  exercises the declared MSRV. Evidence: per-OS directory-resolution
  unit tests including a `HOME`-unset case asserting the Windows fallback
  chain; a checksum test vector; a CI MSRV job that fails when the
  toolchain drops below `rust-version`.
- **No silent data loss** (assessment F16/F18, pass-2 N7, research
  candidate 6). Acceptance: skipped files and unparseable timestamps are
  counted with reasons and surfaced in `data_health`; SQLite-backed
  sessions with no timestamp are bucketed as unknown-time rather than
  dropped from every `--range` view; token totals carry an attributed
  versus unattributed split so parser gaps are visible. Evidence: a
  non-UTF-8 fixture appears in the health report with a cause; `--since`
  and `--range` report how many sessions were excluded and why; an
  `opencode.db` fixture with `time_created = 0` stays visible under
  `--range 7d` with an explicit unknown-time label instead of vanishing;
  overview JSON exposes the unattributed-token remainder. Status change,
  cycle 2: the unknown-time bucketing acceptance is met (`data_health.
  unknown_time_sessions`, CU-3 — see Completed); the skip-reason counts
  and the attributed-versus-unattributed token split remain open.
- **Proxy-governable networking** (pass-2 N4, research candidate 12).
  `--update-pricing` is the one documented network action and it bypasses
  operator proxy policy: pinned ureq 2.12 ignores
  `http_proxy`/`https_proxy`/`all_proxy`, so the full catalog downloads
  directly even through a dead proxy. Acceptance: the downloader honours
  `ALL_PROXY`/`HTTPS_PROXY`/`HTTP_PROXY` with lowercase variants and
  `NO_PROXY` (exact, wildcard, dot-suffix, and `*` forms); PRIVACY.md
  documents the proxy behaviour; the MSRV moves to 1.85 with a CI job
  exercising it. Evidence: a regression test that sets a dead proxy env
  var and asserts the download fails fast instead of bypassing it; an MSRV
  CI job; the dependency refresh (ureq 3.x, opportunistically rusqlite and
  crossterm) lands in one motion.
- **Report output injection** (pass-2 N8). Markdown reports emit
  log-derived strings unescaped, so a hostile model name renders as HTML
  in GitHub-flavored Markdown while the HTML format escapes correctly.
  Acceptance: a `md_escape` counterpart to `html_escape` covers `<`, `>`,
  `&`, backtick, and pipe in every Markdown cell. Evidence: a fixture
  whose model name is `<img src=x onerror=alert(3)>` produces escaped
  output in both `-f markdown` and `-f html`.
- **CI that tells the truth** (pass-2 N5 and N6, assessment F13). The
  "Rust TUI real-data smoke" step can never run because its condition
  reads an env var defined nowhere, and the docs check passes on prose
  that contradicts the shipped binary (the 24-hour auto-refresh claim at
  `docs/guides/governance-reports.md:51`). Acceptance: the TUI step
  either executes (with `expect` installed and an observable gate) or is
  removed; the stale pricing prose is rewritten and a check catches that
  class of drift. Evidence: a green CI run that visibly executes or omits
  the step; a docs check that fails on the removed "refreshed
  automatically" phrasing. Extended by passes 4–5: P4-4 — the published
  npm tarball contains zero tests, so `npm test` passes vacuously
  (acceptance: the `files` list includes the test tree or the invocation
  moves to repo CI with a tarball-content check); P4-5 — the disabled
  PTY-driven TUI smoke harness is complete and needs no real data, giving
  N5 an ungating path (acceptance: an ungated `--demo` PTY smoke job runs
  the existing expect script against a temp-dir corpus); P5-4 — the
  stale auto-refresh prose now directly contradicts `PRIVACY.md:7`
  ("runs fully offline by default"), so the drift check must fail on the
  removed phrasing, not merely warn.
- **CLI surface polish** (pass-2 N9, closed for `--lang` in cycle 2;
  extended by pass-3 P3-7, pass-4 P4-2/P4-3, and pass-6 P6-2, with the N7
  residue). `--version` no longer depends on `--lang` parsing first, but
  the CHANGELOG still over-claims: `agenttrace --overview --version`
  exits 1 because action validation runs before the version early-return
  while `--version` is itself an action (P6-2; `main.rs:136` precedes
  `main.rs:150`). Acceptance: either the version early-return wins over
  action validation or `CHANGELOG.md:11` is reworded to match, and
  `--overview --version` is pinned by a CLI test. Also: stdout report output never ends
  with a newline while the same command's `-o` file does (P3-7); the
  Go-flag shim silently discards every argument after the first
  positional (P4-2 — warn on stderr whenever a discarded argument begins
  with `-`); filter-only invocations silently launch the full-screen TUI
  (P4-3 — error or scope the default action when `--range`, `--project`,
  `--source`, or `--model-filter` appears without an action); the
  empty-directory error message ends with a dangling space (N7).
  Evidence: a CLI test asserting the early-return order; a test asserting
  the shim's truncation warning; a test asserting filter-only invocations
  error; both message strings pinned.
- **Delivery-evidence cost ceiling** (pass-2 N10). `--delivery-evidence`
  runs one synchronous `git log --all` per project root with no
  parallelism, timeout, or cap; measured 0.61s versus 0.011s for
  `--overview` on a 31k-commit repository. Acceptance: per-root git
  queries run concurrently under a bounded pool with a timeout, or are
  replaced by upstream-stored summary columns where the provider offers
  them (capability candidate 8). Evidence: a timing assertion on a
  multi-root fixture; a timeout test with a stubbed slow git.

### Capability lane (researched, prioritized)

- **Subscription limit-pressure diagnostics** (research candidate 3;
  claude-code issues #16157/#38335/#9424/#41930). Acceptance: sessions map
  onto rolling windows (block, daily, weekly) computed from existing
  `Metrics` timestamps and token fields, showing burn rate and which
  sessions consumed a window. Evidence: fixture-driven test asserting
  window math; a demo command showing drain attribution without any
  billing claim. Framed as diagnosis, honoring the invoice non-goal.
- **Second pricing and model-metadata source** (research candidate 4,
  models.dev). Acceptance: the catalog accepts multiple upstreams with
  per-model provenance and a documented precedence rule; context-window
  metadata upgrades context trends to a percentage of the model ceiling.
  Evidence: offline fixture for both sources; a test asserting context
  percentage math against a known model ceiling. Strengthened by research
  pass 5: models.dev now lists 212 providers / 7,492 models (7,056 with
  cost) and ccusage adopted it as a pricing source (v20.0.18) — priority
  raised; it is the ecosystem's default second source.
- **Upstream format canary** (research candidate 5). Acceptance: a
  quarantined, network-explicit CI workflow parses live upstream session
  samples and fails loudly on coverage or label drift; the default test
  path stays network-free. Evidence: a canary run log diffed against
  expectations; a policy note that samples are synthetic or consented and
  redacted.
- **OTel GenAI export, then ingest** (research candidate 2). Acceptance:
  `agenttrace export` maps sessions to pinned `gen_ai` semantic-convention
  spans and metrics loadable by a stock collector; ingest of agents' own
  telemetry output lands later and stays post-hoc. Evidence: an export
  validated against the pinned schema version; a documented round trip
  into an off-the-shelf local backend; no live-streaming path added.
  Strengthened by research pass 5: the GenAI conventions moved to the
  dedicated `open-telemetry/semantic-conventions-genai` repo (active, no
  stable tag yet) — pin the schema only when tagged; and Claude Code
  itself now emits an OTel cost metric and events (fed by `modelPricing`),
  giving the later ingest half a live upstream producer.
- **Shareable baseline config and multi-machine merge** (research candidate
  7). Acceptance: a committed `.agenttrace.toml` carries gate thresholds,
  pricing overrides, and model aliases with defined precedence over flags;
  `agenttrace merge` aggregates exported JSON reports serverlessly.
  Evidence: precedence tests for file versus flag versus env; a merge test
  over multi-machine fixtures with a schema version stamp in output.
- **Trust upstream totals** (research candidate 8, confidence 88%).
  OpenCode now stores authoritative session-level `cost`, all five token
  counters, `parent_id` hierarchy, `summary_additions/deletions/files`
  delivery diffs, and `time_compacting`; agenttrace reads five columns and
  re-derives the rest through the accumulation the hardening lane is
  clamping. Acceptance: the reader prefers stored totals when the columns
  exist, retains the derived path for older databases, and surfaces the
  stored-versus-derived delta in `data_health` as a parser-fidelity
  signal; `parent_id` semantics are verified against upstream code before
  any subagent label ships. Evidence: fixtures with and without the new
  columns; a reconciliation test asserting the delta is zero on well-formed
  data and non-zero when a field is dropped. Sequencing note: this is also
  the root-cause fix for N1/N2 and the git-free path for N10 — schedule it
  alongside the hardening lane rather than after it. Status change,
  cycle 2 + research pass 5: the stored-totals half landed (CU-2 — see
  Completed), closing the N1/N2 root cause this item was sequenced for;
  `parent_id` is set on 98/227 (43%) live local sessions while
  `summary_*` delivery columns are schema-present but 0/227 populated,
  so delivery evidence must degrade gracefully; `title` handling is
  refined by candidate 34 (placeholder gate).
- **Statusline output surface** (research candidate 9, confidence 84%).
  Claude Code's statusline is a documented scriptable surface now fed
  `prompt_cache` and `rate_limits.spend_limit` objects, and statusline
  plugins are the fastest-growing competitor category. Acceptance: a new
  output mode reads host-agent statusline JSON on stdin and prints spend,
  burn rate, and cache hit ratio for the current session or project,
  cross-provider, from the warm cache. Evidence: a test feeding sample
  statusline JSON and asserting the rendered line; a latency budget test
  on a warm cache; no live-streaming path added. Strengthened by research
  pass 5: ccusage ships a `statusline` beta, and the local machine's Node
  HUD needs a shell cache wrapper purely to stay render-fast — the
  latency budget must be shell-fast, which a single Rust binary
  satisfies natively.
- **Compaction and re-cache cost analytics** (research candidate 10,
  confidence 78%). Both majors standardized on compaction and re-cache
  vocabulary in one release cycle: OpenCode stores durable compaction
  events with a reason plus `session_context_epoch`, and Claude Code's
  `/cost` reports hit ratio, misses, tokens re-cached, and warm/cold.
  Acceptance: per-session compaction events and their re-cache cost are
  reported where a source records them, with an aggregate trend, and
  capability labels carry the coverage honestly. Evidence: fixtures per
  provider shape; a test asserting re-cache cost math from cache-write
  tokens and pricing.
- **Upstream schema-drift tracker** (research candidate 11, confidence 80%;
  the low-risk prefix of the canary item above). OpenCode publishes a
  dated public schema changelog and per-change migration files while its
  storage layer is mid-migration, and agenttrace's column-sniffing reader
  degrades silently on drift. Acceptance: a quarantined, network-explicit
  job diffs upstream's published schema contracts against what the
  readers expect and files drift loudly; the default test path stays
  network-free. Evidence: a drift run that fails on an intentionally
  introduced expectation mismatch; a policy note that no session samples
  are fetched. Strengthened by research pass 5: upstream moved to
  `anomalyco/opencode` (v1.18.26) and added migration
  `20260511173437_session-metadata` (`ALTER TABLE session ADD metadata
  text`) — watch the dated migration directory for drift.
  are fetched.
- **Absolute-time scoping** (research candidate 13). The CLI exposes only
  `--range` presets; ccusage ships `--since`/`--until` and `--timezone`.
  Acceptance: `--since`/`--until` inclusive bounds and a `--timezone`
  flag accepted by insights and the TUI range views, building on the
  local-calendar fix in the hardening lane. Evidence: unit tests pinning
  day boundaries in two timezones; the pass-3 P3-2 reproducer visible in
  the local zone.
- **Sub-agent attribution** (research candidate 14). Claude Code marks
  sub-agent turns with `isSidechain`; our own fixture carries it and
  `parser.rs` never reads it (grep: zero matches). Acceptance: sidechain
  turns counted per parent session, `by_sidechain` in JSON overviews, TUI
  drill-down showing sub-agent share of cost/tokens. Evidence: fixtures
  with and without the marker; parity note against ccusage `--by-agent`.
- **Tier- and server-tool-aware cost accuracy** (research candidate 15,
  strengthened by research pass 4). Live transcripts record
  `usage.server_tool_use` and `usage.service_tier` that `parser.rs` reads
  neither of; the live LiteLLM catalog carries 86 tiered
  (`above_200k`), 68 reasoning-rate, 112 priority, 41 flex, and 3
  dashscope `tiered_pricing` entries that `convert_litellm` drops today.
  Acceptance: server-tool counters surfaced in `--audit`, tier recorded
  on sessions, and the snapshot gains optional tier/server-tool rate keys
  with an explicit "not priced" label instead of a silent zero.
  Evidence: a live-transcript-derived fixture; rate-table tests for the
  tier keys. Strengthened by research pass 5: the live matrix grew ~5×
  (`tiered_pricing` 3→51 models, plus `above_272k` priority/flex and
  `batches` field families) — scope the first cut to a handful of
  flagship providers and label the rest not-priced.
  tier keys.
- **Provider-recorded git branches** (research candidate 16). Prefer
  transcript-recorded `gitBranch` in `--delivery-evidence` over
  git-timestamp correlation, falling back with an explicit label.
  Evidence: fixtures with and without the field; the fallback labeled.
- **Reconcile against Claude Code's own totals** (research candidate 17;
  the Claude sibling of "Trust upstream totals").
  `~/.claude/stats-cache.json` publishes `totalSessions`,
  `dailyModelTokens`, `longestSession`, and `lastComputedDate`. Acceptance:
  an overview field reports agreement/divergence against the cache when
  fresh and a stale-as-of note when `lastComputedDate` trails the newest
  transcript. Evidence: a fixture pair asserting both branches.
  Strengthened by research pass 5: with server-managed `modelPricing`,
  Claude Code reports list price until a session's settings fetch
  confirms the table — reconciliation must expect a list-price session
  prefix (see candidate 35).
- **Prompt-history index for discovery and search** (research candidate
  18). `~/.claude/history.jsonl` and `~/.codex/history.jsonl` are KB-scale
  indexes of every prompt while discovery parses MB-scale transcripts.
  Acceptance: history-derived session titles and a `--search` pass over
  the prompt index that works before transcripts parse, with explicit
  degradation when a provider has no history file. Evidence: a title
  derivation test; a degradation label.
- **`agenttrace mcp`** (research candidate 19). No usage-analytics MCP
  server occupies this space, and the repo already ships agent-facing
  surfaces. Acceptance: a read-only stdio MCP server exposing
  `session_overview`, `top_sessions`, `anomalies`, and `compare_baseline`
  over existing core functions. Evidence: a fixture-driven integration
  test; no new blocking network dependency. Strengthened by research
  pass 5: MCP published stable revision 2026-07-28 (elicitation, tasks,
  sampling, structuredContent) — target it; the read-only analytics
  tools need no redesign under the new optional capabilities.
- **SARIF 2.1.0 export for CI gates** (research candidate 20). Gate and
  anomaly findings exist only in agenttrace's own JSON; SARIF is the
  interchange format GitHub code scanning ingests from any CLI.
  Acceptance: gate failures map to SARIF results with stable rule ids;
  output validates against the published schema; a CI recipe uploads it
  via `github/codeql-action/upload-sarif`. Evidence: schema validation in
  a test; an uploaded artifact in the recipe job.
- **Configuration file with a published JSON Schema** (research candidate
  21). agenttrace is env-var-only today; ccusage ships config files with
  IDE validation. Acceptance: an XDG-aware `config.json` for
  range/format/project/source defaults with flag > env > file > default
  precedence, schema shipped in-repo. Evidence: precedence tests;
  schema-referenced config in docs.
- **Shell completions and man page** (research candidate 22). No
  completion or man generation exists in the workspace. Acceptance:
  `agenttrace completions <bash|zsh|fish>` via `clap_complete`, exercised
  in CI, with a man page in release artifacts. Evidence: a CI step that
  completes a partial flag in a spawned shell.
- **Dependency-currency lane** (research candidate 23). Generalizes the
  ureq/MSRV motion above: rusqlite 0.32→0.40 moves the bundled SQLite
  from 3.46.0 (2024-05) to current — a security-posture item for a tool
  that parses untrusted foreign databases; crossterm 0.28→0.29 has a
  documented route through ratatui 0.30; clap 4.5→4.6 rides along.
  Acceptance: lockfile bumps with the full workspace suite green; an
  update-deps CI job that fails on drift beyond one minor; the MSRV
  statement re-checked. Evidence: the refreshed `Cargo.lock` diff plus
  the passing suite; the new CI job visible.
- **Cost provenance for priced sessions** (research candidate 24,
  confidence 86%; implements the promotion path named in issue #103).
  The vendored snapshot collapses every provider of a model onto one bare
  rate chosen by a heuristic: `glm-4.7` — a model string present in real
  local transcripts — prices at OpenRouter's $0.4/$1.5 per M instead of
  Z.AI's $0.6/$2.2 (32% low), and `gpt-oss-120b` prices at OpenRouter's
  rate because no first-party key exists in the file; 237 model suffixes
  carry conflicting rates (up to 11 distinct). Acceptance: provider-
  scoped keys retained alongside a documented deterministic bare-key
  choice; per-session rate provenance (provider used, collision count,
  min–max spread) in report JSON, `--audit`, and `data_health`;
  `AGENTTRACE_PRICING_FILE` aliases documented as the user pin;
  `has_specific_price` coverage made honest. Evidence: the research-pass-4
  glm-4.7 reproducer prices at the chosen provider's rate with provenance
  fields present; the snapshot regenerated with `PRICING_SNAPSHOT_DATE`
  kept in sync (see the hardening item above).
- **Gemini-family thinking tokens** (research candidate 25, confidence
  80%). `usageMetadata.thoughtsTokenCount` — billed at the output rate —
  is read by none of the three Gemini usage sites (`parser.rs:1779-1795`,
  `:3530`, `:3828`), undercounting every thinking-model session across
  the Gemini CLI, Qwen Code, and Antigravity paths while the snapshot
  already carries gemini-3.x rates. Acceptance: thinking tokens folded
  into output with a reasoning breakdown, gemini-3.x fixtures, reasoning
  share in `--audit`, and a changelog note that baselines shift.
  Evidence: a fixture with `thoughtsTokenCount` asserting the output
  includes it.
- **Qwen Code dual-output transcripts** (research candidate 26,
  confidence 62%; radar issue #237). Qwen Code documents Dual Output
  `--json-file` as "a canonical machine-readable transcript" and changed
  session-list metadata loading; our probe at `~/.qwen/projects` has no
  local evidence yet. Acceptance: dual-output files accepted as a
  documented `-d` input with fixtures, after first verifying whether the
  Gemini parser already accepts the shape. Evidence: a real dual-output
  fixture in `testdata/`; doctor and docs updated.
- **Release-channel identity and an all-channel version guard** (research
  candidate 27, confidence 70%; issue #272). The npm package is
  `@zack78/agenttrace` while Homebrew and WinGet publish as
  `Luoyuctl.AgentTrace`, and the cycle-1 guard covers `plugin.json` only.
  Acceptance: the scope decision resolved (#272), and
  `scripts/ci/check-plugin-version.sh` extended into one guard comparing
  CHANGELOG, `plugin.json`, npm `package.json`, and the rendered
  Homebrew/WinGet channels. Evidence: the guard passes against
  `scripts/release/render-channels.sh` output and fails on an injected
  version skew.

- **Per-turn model attribution** (research candidate 33, confidence high;
  the model-dimension sibling of candidates 14 and 24).
  `parse_claude_code_jsonl` freezes the session model on the first
  assistant message (`parser.rs:2161-2164`) and `isSidechain` appears
  nowhere in core, so model switches, advisor-model turns, and sub-agent
  rows are all priced under the first model — the exact class ccusage
  fixed in v20.0.17 ("Count advisor model usage"), now that Claude Code
  ships PreModelSwitch/PostModelSwitch hooks. Acceptance: each assistant
  turn's `model` reaches its own `Event.model_used`; sessions expose a
  model-mix summary; pricing and waste use the per-turn model; reports
  disclose mixed-model sessions whenever more than one model appears.
  Evidence: a fixture with a mid-session model switch and sidechain rows
  asserting the per-model cost split; a parity note against ccusage's
  advisor fix.
- **Placeholder-title gate for provider-recorded titles** (research
  candidate 34, confidence high). OpenCode populates `title` with
  `New session - <timestamp>` on 227/227 live local sessions and the
  reader uses it verbatim (`sqlite_sessions.rs:672-676`), so every
  opencode session currently gets a junk name. Acceptance: titles
  matching the placeholder pattern are treated as absent, message-derived
  naming (candidate 18) wins, and naming provenance records
  `provider:placeholder`. Evidence: a fixture reproducing the placeholder
  pattern asserting derived names; the live census recorded in research
  pass 5.
- **Claude Code `modelPricing` ingestion — org-contracted rates** (research
  candidate 35, confidence high after docs confirmation; extends
  candidate 24). Since Claude Code v2.1.242 a `modelPricing` managed
  setting — optional `multiplier` (0 < x ≤ 1, scales every figure) plus
  optional `overrides` mapping model ID to `{input, output, cacheRead,
  cacheWrite}` USD-per-million rates (all four required, per-row
  drop-on-parse-error) — makes `/cost`, the status line, the Agent SDK,
  and its OTel cost metric report contracted rates instead of list; the
  key is managed-scope only (server-managed settings, MDM,
  `managed-settings.json`, policy helper) and ignored in user settings.
  Acceptance: agenttrace probes the managed-settings paths (not just
  `~/.claude/settings.json`), converts per-million rates, takes
  precedence over the catalog, stacks `multiplier`, drops unparseable
  rows like upstream does, and labels provenance `org-contracted` in the
  candidate-24 fields. Evidence: a `managed-settings.json` fixture
  asserting provenance and precedence; a list-price-prefix case
  documenting the server-managed timing nuance for candidate 17.
- **Incremental cache sync from OpenCode's event log** (research candidate
  36, confidence medium). The database carries an append-only
  `event`/`event_sequence` change feed (27,769 rows locally, keyed
  `aggregate_id`/`seq`/`type`) that the session cache ignores; refresh
  re-derives sessions wholesale. Acceptance: the cache stores a
  `max(seq)` watermark and refreshes only aggregates with newer events
  when the tables exist, falling back to a full scan on older databases;
  incremental output equals full-scan output. Evidence: a fixture DB with
  event rows asserting incremental-equals-full parity; a before/after
  refresh-cost measurement.

Items leave this section only when their acceptance criteria and evidence
expectations are met and recorded in the Completed record above, and the
release notes name the change.
