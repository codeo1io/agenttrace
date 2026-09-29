# agenttrace — Roadmap

> Autonomously maintained by the roadmap sync (reliability-first). Items cite reproducible codebase signals; acceptance is proven by cited evidence.

**Vision**: A reliable, customer-friendly repository advanced by evidence-cited roadmap cycles owned by the autonomy loop

**Pillars**: reliability work outranks customer-experience work; every roadmap item cites reproducible codebase signals; acceptance is proven by cited evidence, never claimed

## Fleet context

- dependents (changes here affect): (host), agent, maestro
- graph: evidence-derived (imports/refs/deploy surfaces); advisory

## Open items

### Add test coverage for 1 untested module(s)
- id: `rm-002` | track: reliability | priority: 83.0 | status: candidate
- signals: reliability.no_tests:scripts/fixtures/make-adversarial-sqlite.py
- acceptance: Every module in ['scripts/fixtures/make-adversarial-sqlite.py'] has a corresponding test file with at least one passing test
- evidence: full suite green (python -m pytest -q) at HEAD; conductor validation digest validation:v1:<sha> recorded in the shipping PR

### Refactor 3 high-complexity function(s)
- id: `rm-001` | track: reliability | priority: 79.0 | status: candidate
- signals: reliability.complexity_hot:npm/scripts/install.js::L24, reliability.complexity_hot:npm/scripts/install.js::L28, reliability.complexity_hot:npm/scripts/install.js::L40
- acceptance: Each flagged function is decomposed below the branch threshold with behavior locked by characterization tests
- evidence: ast-based branch-count check passes at HEAD (full suite green; conductor validation digest validation:v1:<sha> recorded in the shipping PR)

### Land per-DB SQLite snapshots (v7) and stop the multi-DB cache clobber
- id: `rm-068` | track: reliability | priority: 90.0 | status: candidate
- signals: cache.shared_snapshot_name:crates/agenttrace-core/src/session_cache.rs::L334 (sqlite_snapshot_path returns one fixed `hermes-sqlite.json` per source regardless of which database was cached) with SQLITE_SNAPSHOT_SCHEMA_VERSION=6 at session_cache.rs::L13, driven by sqlite_sessions.rs::L103/:L123 (main DB plus every profile/sibling DB) calling load/store per database at :L161/:L165/:L229/:L233 from the default discovery path discovery.rs::L248 — behaviorally proven 2026-09-30 (run 40208f3d2ca8 assess F1): a crafted 3-DB probe rewrote the shared snapshot on run 2 with zero database change, and on the live 9-DB hermes corpus the 2.4 GB main DB's 6.2 MB snapshot is clobbered to a 196-byte `sessions:[]` entry every invocation (~3.4s + ~1s/profile of re-query per run); the fix already exists on this campaign's fork branch (569c5c2 on conductor/run-2ae192428c54, +582/−47, schema v7 per-DB `<name>-sqlite-v7-<fnv1a64>.json`), which zero-overlap lands on 90a4ef5; branch hygiene: 569c5c2 also tracks 14 `.conductor/progress/*.ndjson` journals and .gitignore::L13 covers `.hermes/` but not `.conductor/` (assess F3)
- acceptance: snapshot filenames are per-database, keyed by a stable inline FNV-1a of the DB path (DefaultHasher banned for durable keys); the two-run zero-write rule holds (rebuild from tree, clear cache, run twice — run 2 rewrites only databases genuinely modified in the window); the legacy-scheme sweep retires v6 files and spares foreign ones; `--clear-cache` removes every v7 name; the landing carries the salvage's session_cache.rs change WITHOUT the 14 `.conductor/progress/*.ndjson` files and adds a `.conductor/` line to .gitignore
- evidence: cache-dir listing before/after two identical runs (per-DB files present, mtimes unchanged on run 2); crafted multi-DB fixture test green in cargo test -p agenttrace-core; git diff of the landed branch shows session_cache.rs + .gitignore only, zero `.conductor/` paths; full suite green; conductor validation digest validation:v1:<sha> recorded in the shipping PR

### Stop reporting fabricated 100% tool success for hermes sessions
- id: `rm-069` | track: reliability | priority: 85.0 | status: candidate
- signals: correctness.fabricated_metric:crates/agenttrace-core/src/sqlite_sessions.rs::L195-L196 (tool_calls_total and tool_calls_ok both read column 5, sessions.tool_call_count) mapped into sessions at :L683-L685; consumers: reports.rs::L86/:L223-L224/:L310 (tool success rate), governance.rs::L486-L494 (tool-anomaly detection can never fire for hermes_db — the dominant source on this box), main.rs::L1187 (--max-tool-fail-rate is a no-op), insights.rs::L24 (capability deltas spuriously improve); source-signal proof 2026-09-30: messages.effect_disposition is NULL for 181110/181383 live tool rows, so ok/fail is NOT recoverable from recorded data (known since cycle-5, re-filed with the downstream impact map)
- acceptance: hermes_db sessions report tool success as unknown or are excluded from tool-rate surfaces — the total is never mirrored into tool_calls_ok; governance tool-anomaly detection and --max-tool-fail-rate operate on real signals or explicitly exclude hermes_db with disclosure; a fixture asserts a hermes_db session can never surface a nonzero success rate
- evidence: new fixture test green in cargo test -p agenttrace-core; before/after output on the live corpus showing hermes tool-rate as unknown/N-A rather than 100%; full suite green; conductor validation digest validation:v1:<sha> recorded in the shipping PR

### First-party pricing priority: stop resellers outbidding vendors for their own models
- id: `rm-070` | track: reliability | priority: 84.0 | status: candidate
- signals: pricing.vendor_loses_own_model:crates/agenttrace-core/src/pricing.rs::L483-L498 (provider_priority hardcodes a vendor subset; zai and future first-party vendors fall to the `_ => 0` arm) combined with :L469-L476 (highest-priority entry wins per normalized key) and the deepseek special case at :L509-L511; measured 2026-09-30: 10 `glm-5.3*` keys across 6 providers in the live LiteLLM catalog with ~20% input-rate spread (aihubmix 1.1268e-06 vs zai 1.4e-06 per M); sibling campaign 902db834 measured this fix class moving this corpus's total cost $2801.50 → $4308.93 (+53.8%, glm-5.3 +54.0%)
- acceptance: the first-party vendor set is derived from catalog provider names rather than a hardcoded subset, so zai/kimi/qwen-class vendors win their own normalized models against resellers; a fixture catalog with vendor+reseller entries asserts the vendor entry is selected; headline cost baseline movement rides the accepted-drift flow and is recorded in the PR
- evidence: unit test over a fixture catalog green in cargo test -p agenttrace-core; before/after cost totals on the live corpus recorded in the PR (expected +50%-class correction); must hold for the refreshed catalog when sibling rm-062's snapshot refresh lands too; validation digest validation:v1:<sha> recorded in the shipping PR

### Settle the fork npm distribution identity before the first fork release
- id: `rm-071` | track: reliability | priority: 80.0 | status: candidate
- signals: release.foreign_package:npm/package.json::L2 (fork publishes `@zack78/agenttrace`) + .github/workflows/release.yml::L175-L182 (builds and publishes ./dist/zack78-agenttrace-*.tgz with NPM_TOKEN) while upstream owns that package and published 0.9.0 to it 2026-09-29T20:48:58Z (fork's own version is 0.7.9); upstream's open PR #272 additionally moves upstream itself to a scoped package — the collision resolves against the fork either way; a fork release today either fails on the token or pollutes upstream's package page
- acceptance: the fork's package name/scope is fork-owned across npm/package.json, release.yml, and installer/docs references — or the npm publish job is explicitly removed from the release lane; a dry-run of the release lane resolves the publish target unambiguously; no job can publish to upstream's package
- evidence: diff across npm/package.json + release.yml + docs; dry-run release log (or workflow lint) showing the resolved package name; the decision (own scope vs no npm channel) recorded with rationale in the PR description; validation digest validation:v1:<sha> recorded in the shipping PR

### Stable hash for durable history session ids (retire DefaultHasher)
- id: `rm-072` | track: reliability | priority: 72.0 | status: candidate
- signals: durability.unstable_key:crates/agenttrace-core/src/history.rs::L104-L107 (session_id hashes session.path + session_start with DefaultHasher; the 16-hex result is the persistent key of history.json and the dedup identity in merge_preserved_history) — DefaultHasher's algorithm stability across Rust releases is unspecified, so toolchain churn silently re-keys every preserved record and the store accumulates old-id/new-id duplicates; same hazard class the snapshot lineage already banned for durable keys (stable inline FNV-1a adopted there)
- acceptance: history ids use a stable inline FNV-1a over the same fields; a defined migration for existing ids (one-time reset or dual-read) so no preserved record is duplicated by the scheme change; a golden-fixture test locks ids against a fixed vector
- evidence: golden id-vector fixture test green; migration demonstrated on a fixture history.json (zero old-id/new-id duplicate pairs after one upgrade run); full suite green; conductor validation digest validation:v1:<sha> recorded in the shipping PR

### Codex session-envelope fidelity batch (parse the fields we already carry)
- id: `rm-073` | track: customer-experience | priority: 70.0 | status: candidate
- signals: parser.dropped_envelope_fields:crates/agenttrace-core/src/parser.rs::L2114-L2128 (turn_context yields only payload.model; approval_policy/effort/summary/current_turn are dropped) plus parser.rs::L2278/:L2298/:L2320 (reasoning_output_tokens is billed as input with no reasoning-share surface); verified absent 2026-09-30: async_delegations[].display_name, thread_goals, isSidechain, gitBranch, server_tool_use/service_tier, session_model_usage, todos/summary capture — the harness this box runs most is mined shallowest
- acceptance: envelope fields are captured storage-first into the session model with the existing unknown-field discipline and surfaced in --doctor first (analytics later); a reasoning-share view over the billed reasoning tokens exists; a crafted fixture per captured field parses correctly and survives unknown-field churn
- evidence: new fixture tests green per field in cargo test -p agenttrace-core; --doctor output on a fixture corpus showing the captured fields; no regression in the existing parser suite; validation digest validation:v1:<sha> recorded in the shipping PR

### Prefer hermes stored cost/reasoning columns when present
- id: `rm-074` | track: reliability | priority: 65.0 | status: candidate
- signals: fidelity.ignored_authoritative_columns:crates/agenttrace-core/src/sqlite_sessions.rs::L179-L180 (the hermes SELECT omits sessions.actual_cost_usd/estimated_cost_usd and reasoning tokens; cost is always derived from tokens × catalog) — the columns exist in the live schema but are all NULL today, so the defect is latent and fires the moment the hermes daemon populates them; the opencode path already prefers stored totals with provenance disclosure (stored_session_totals), making this an asymmetry, not a deliberate exclusion
- acceptance: populated stored columns are preferred over token-derived cost with the same provenance disclosure mechanism opencode uses; all-NULL columns leave current behavior unchanged; a fixture DB with populated columns proves the preference and the disclosure
- evidence: fixture tests green for populated and NULL variants; provenance output shown for a populated fixture; full suite green; conductor validation digest validation:v1:<sha> recorded in the shipping PR

### One cache-dir resolver: stop doctor duplicating session_cache precedence
- id: `rm-075` | track: reliability | priority: 60.0 | status: candidate
- signals: drift.duplicated_resolver:crates/agenttrace-core/src/doctor.rs::L427-L449 re-implements the cache-dir resolution session_cache.rs owns (AGENTTRACE_SESSION_CACHE_DIR → macOS ~/Library/Caches → XDG_CACHE_HOME fallback); the copies agree today, but drift (a new override or precedence fix applied to one) would make --doctor report on a different cache than the one the CLI uses
- acceptance: a single resolver is exposed from session_cache.rs and doctor.rs calls it; a test locks precedence equality across env override, macOS path, and XDG fallback
- evidence: resolver precedence test green; grep shows no second precedence chain in doctor.rs; full suite green; conductor validation digest validation:v1:<sha> recorded in the shipping PR

### Cache or bound the per-root git log scans in governance
- id: `rm-076` | track: reliability | priority: 58.0 | status: candidate
- signals: perf.uncached_git_walk:crates/agenttrace-core/src/governance.rs::L780-L781 (git -C <root> log --all --format=%ct runs once per unique project root per report with no caching, walking full repository histories on every --governance invocation for data that changes only when a repository gains commits; execution is safe — argv, never a shell)
- acceptance: first/last commit timestamps are cached alongside the session cache keyed by root (or the scan is bounded with -n 1/--since); repeat invocations skip the walk; returned timestamps are identical to the unbounded scan on a fixture repository
- evidence: equivalence test on a fixture repo (cached vs uncached timestamps equal); before/after timing on a long-history repository recorded in the PR; full suite green; conductor validation digest validation:v1:<sha> recorded in the shipping PR

### OpenTelemetry GenAI export, consumer-gated (watch upstream v0.13.x)
- id: `rm-077` | track: customer-experience | priority: 50.0 | status: candidate
- signals: dx.no_telemetry_export (repo-wide grep 2026-09-30: zero otel/OTLP hits in the tree; only the Copilot env var at discovery.rs::L239); upstream's roadmap plans an OTLP metrics push at v0.13.x while its claimed "OpenTelemetry JSON output (v0.7.0, done)" has zero code-search hits upstream — intent evidence, not state evidence; market signal: ccusage grew ~13.5k → 18,805 stars in four weeks (+39%)
- acceptance: build only when a local consumer materializes (recorded demand) or upstream ships v0.13.x to converge with; emit semconv-conformant GenAI spans/metrics from the existing session model rather than a bespoke JSON shape, with the GenAI semconv stability status re-verified at implementation time (it is in active churn)
- evidence: consumer-demand record (issue or user ask) attached before implementation starts; exported payload validated against the semconv spec version pinned at implementation; offline fixture test green; conductor validation digest validation:v1:<sha> recorded in the shipping PR

<!-- New items rm-068..rm-077 appended 2026-10-01 by run 40208f3d2ca8 roadmap phase (repository-maintenance fac497e77cb549d49b73452905bacd52 cycle 1, attempt 2c893c428e9b4d58a84342cb9dd594f0) at HEAD 90a4ef5. Sources: this run's assess phase (attempt 96236eb0, findings F1-F7 — F1→rm-068 with F3 folded into its acceptance, F2→rm-069, F4→rm-072, F5→rm-074, F6→rm-075, F7→rm-076) and research phase (attempt 83554d6f, prior attempt c87eb900, candidates RC-1..RC-10 — RC-2≡F1→rm-068, RC-3→rm-070, RC-4→rm-071, RC-8→rm-073, RC-9→rm-077). NOT filed, covered elsewhere (verified live 2026-10-01): RC-1 (absorb upstream #285/#286) is stale-at-master — origin/master 9d88b36 already landed the Codex high-water fix and statusline sanitization (rm-012/rm-013, done) plus the crt-static release wave (rm-023) and the parse-once refactor (rm-014, candidate); RC-5 (pricing snapshot refresh + drift detection) = sibling rm-062 of run 83642957d130, campaign-local until that PR integrates — RC-5's extra cadence/CI-freshness-gate delta should ride rm-062 at its implementation; RC-6 (zstd rollouts) = sibling rm-066; RC-7 (dependency currency) = master rm-021/rm-022 (crossterm unify, ureq+rusqlite majors); RC-10 remains a research-phase watch posture, not an item. Research-pass rejections stand (web dashboard, PureScript NPX, models.dev second source, MCP surface, SARIF, pre-absorbed SPQS index, config-file system, C-bound zstd, fork winget/homebrew channel, antigravity fixtures, self-authored self-analysis recipe, stats-cache). IDs minted rm-068+ past every live claimant verified 2026-10-01: origin/master 9d88b36 holds rm-001..rm-027 (ids 012..023 appear TWICE there — the duplicate-id defect the staged integration-9c6b6f4e1107 tree resolves); fork PR #19 (conductor/run-c7dca75d30e3) is open carrying pre-renumber ids; in-flight worktrees: integration-9c6b6f4e1107 (staged rm-034..rm-054), run-0a279c440c10 (rm-034..rm-054), run-83642957d130 (rm-055..rm-067), run-304846327112 (rm-028..rm-032). rm-IDs are campaign-local until integration; if any claimant lands rm-068+ first, renumber at the commit phase (record-not-renumber now). Integration note: rm-072's history.rs::L104-L107 neighborhood is adjacent to sibling rm-055's alias-dedupe anchor at history.rs::L105 — same file, different defect, expect a text-level conflict only. No existing item or status was modified. -->

<!-- managed by hermes-roadmap render; do not edit by hand -->
