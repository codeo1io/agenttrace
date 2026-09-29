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

### De-duplicate aliased session roots by canonical path
- id: `rm-055` | track: reliability | priority: 91.0 | status: candidate
- signals: reliability.discovery_alias_double_count:crates/agenttrace-core/src/discovery.rs::L147 (root dedupe keys on exact path string), cross-root seen-set keys on path spelling in find_session_files_cached (discovery.rs::L707-L712); history ids hash session.path (history.rs::L105) so aliased spellings also fork history identity
- acceptance: discover_session_dirs dedupes roots by canonicalized target (fs::canonicalize is already used for walk-time dedupe at discovery.rs::L362-L363 — extend it to root admission); a HOME where ~/.pi is a symlink to ~/.config/pi reports 1 session and 1x cost (currently 2 Sessions / 2x cost, live-verified $0.0012 vs $0.0006); a regression test with a symlinked alias root asserts single count and stable history ids across spellings
- evidence: cargo test -p agenttrace-core green including the new alias-root test; repro fixture documented in the shipping PR (temp HOME + symlinked alias root, `agenttrace --overview -f json` session count == 1); full suite green at HEAD with conductor validation digest recorded in the shipping PR

### Reject non-regular and oversized files at discovery collect time
- id: `rm-056` | track: reliability | priority: 90.0 | status: candidate
- signals: reliability.special_file_hang:crates/agenttrace-core/src/discovery.rs::L599-L612 (collect loop matches filename pattern with no file-type check) feeding an unguarded std::fs::read in parse_file (parser.rs::L26-L27); positional paths ARE guarded by is_file() (main.rs::L806); live-verified: a FIFO named *.jsonl in a discovered dir hangs `--overview`, `--doctor`, `--doctor -d` (exit 124 under timeout 10)
- acceptance: discovery requires file_type().is_file() at collect time and skips non-regular files with a named skip reason surfaced in doctor/diagnostics; a configurable max-file-size cap is applied at the same predicate (closes the cycle-5 provisional max-size item — same collect-time check); the FIFO repro terminates under 2s with a skip diagnostic instead of hanging; tests cover FIFO-skip and oversized-file-skip
- evidence: cargo test -p agenttrace-core green including new skip tests; live FIFO repro before/after output in the shipping PR (mkfifo in temp HOME discovered dir, timeout 10 agenttrace --overview)

### Make statusline capture compaction concurrency-safe
- id: `rm-057` | track: reliability | priority: 89.0 | status: candidate
- signals: reliability.statusline_capture_race:crates/agenttrace-core/src/statusline.rs::L317 (fixed temp path path.with_extension("jsonl.compact")) plus read-whole-file→temp→rename window (statusline.rs::L304-L325); live-verified: 16 concurrent `agenttrace statusline` hosts over a 10.6MiB journal lost 7/16 payloads with 15/16 stderr `capture failed (No such file or directory)`; the session cache already solved this class with unique_temp_path (session_cache.rs::L284-L294)
- acceptance: compaction stages through a unique temp path (pid+counter pattern of unique_temp_path) so concurrent hosts never collide; compaction is append-preserving (no concurrent append dropped — lock or tail re-check), restoring the documented contract "compaction drops whole oldest lines" (it currently drops newest appends); a concurrency test spawns ≥8 writers and asserts every payload survives a compaction cycle
- evidence: new concurrency test green in cargo test -p agenttrace-core; 16-way race repro re-run shows 16/16 markers present and zero capture-failed stderr; full suite green at HEAD with validation digest recorded in the shipping PR

### Adopt upstream Codex token high-water baseline (PR #286 port)
- id: `rm-058` | track: reliability | priority: 87.0 | status: candidate
- signals: reliability.codex_rewind_double_count:crates/agenttrace-core/src/parser.rs::L2250-L2267 (codex_token_count_usage stores the RAW event total as next baseline via Some(total)); upstream v0.9.0 token_usage_high_water (upstream parser.rs:2077-2080, 2133-2146 — "Codex can briefly rewind total_token_usage (e.g. after compaction) and then climb back"), released as "Fix Codex cost double counting"; PR #286's companion codex_line_is_ignorable (upstream parser.rs:2242-2248) has 0 matches in this tree
- acceptance: a rewound-total event with a positive sibling component no longer lowers the stored baseline (component-wise high-water mark); tokens re-fed after a compaction rewind are not counted twice — regression test feeds rewind→climb event sequences and asserts monotonically-consistent cost; compacted/non-token event_msg lines are skipped before JSON parse (codex_line_is_ignorable port)
- evidence: cargo test -p agenttrace-core green including the new rewind-corpus regression test; upstream reference (v0.9.0 parser.rs snapshot fetched 2026-09-30) cited in the shipping PR; before/after estimated cost on a rewind corpus recorded in the PR body

### Move Efficiency and Action-Center governance off the TUI render thread
- id: `rm-059` | track: reliability | priority: 80.0 | status: candidate
- signals: reliability.render_thread_io:crates/agenttrace-tui/src/presentation.rs::L907 (render_workspace calls ensure_governance) with the Efficiency arm running mcp_governance + context_trends + load_statusline_insights synchronously (app.rs::L1538-L1548); load_statusline_insights reads and parses up to a 10MiB journal (statusline.rs::L512, bounds at statusline.rs::L82-L120); only Delivery got the background worker (app.rs::L1550-L1571, cycle-7 A11-4)
- acceptance: no governance evidence computation runs on the render thread for any panel (route Efficiency/Action-Center through the existing delivery-worker pattern); the statusline journal is not re-read every frame while insights are None; the frame-draw path performs no file I/O beyond cached state; Efficiency panel opens without UI freeze on a ≥10MiB journal corpus
- evidence: TUI smoke run over the 10.6MiB race journal fixture with perceived-freeze note in the PR body; code-reading anchors re-verified at review; cargo test full suite green at HEAD

### Restore PR-gate test coverage and delete dead self-hosted cache machinery
- id: `rm-060` | track: reliability | priority: 76.0 | status: candidate
- signals: reliability.pr_gate_blind:'.github/workflows/ci.yml'::L31-L33 (pull_request gate runs the Lint job only: fmt+clippy) while every cargo test invocation lives in the full job gated `if: github.event_name != 'pull_request'` (ci.yml::L82, L102, L108) — broken tests first surface after merge to master; ci.yml::L45-L56 keeps the 'Self-hosted cargo target persistence' RUNNER_TOOL_CACHE tar block plus comments referencing 'the single self-hosted runner' (ci.yml::L24-L31) although the job now runs on ephemeral GitHub-hosted ubuntu-latest (ci.yml::L33)
- acceptance: the pull_request gate includes at least a focused test slice (cargo test -p agenttrace-core --lib) so broken tests fail before merge; the dead self-hosted cache block is deleted and the stale runner comments updated to the ephemeral-runner reality; a deliberately broken test pushed to a PR branch demonstrably fails the PR gate (probe once, then revert)
- evidence: live PR/CI run on the campaign branch showing the test lane executing on pull_request (run URL in the shipping PR); ci.yml diff; existing full+deny lanes on master unchanged

### Merge upstream PR #285+#286 drift and reconcile cache schema 17→19
- id: `rm-061` | track: reliability | priority: 75.0 | status: candidate
- signals: reliability.upstream_drift:crates/agenttrace-core/src/session_cache.rs::L8 (SESSION_CACHE_SCHEMA_VERSION=17) vs upstream v0.9.0 =19 — every future upstream merge touching cache invalidates cross-binary caches and conflicts; fork lacks PR #285 (MSVC static CRT linking) and PR #286 (TUI triage rules, harness-wrapper title cleanup, parallel cache-miss parsing, HH:MM:SS timeline, top-15 tools, schema bump), verified per-PR with git merge-base --is-ancestor (2026-09-30)
- acceptance: #285 and #286 content merged EXCEPT the token high-water half which rm-058 owns (do not double-port); SESSION_CACHE_SCHEMA_VERSION reconciled to upstream 19 or a documented fork-divergent value with a migration note; cross-binary cache invalidation behavior documented; no behavioral regressions vs the full suite
- evidence: merge-base --is-ancestor checks pass for both upstream PRs post-merge; cargo test full suite green at HEAD after the merge; CHANGELOG entry naming the schema reconciliation; coordination note in the PR that rm-058 lands first or folds in here

### Refresh the pricing snapshot and add drift detection
- id: `rm-062` | track: reliability | priority: 74.0 | status: candidate
- signals: reliability.pricing_drift:crates/agenttrace-core/src/pricing_snapshot.json::L1 (_snapshot date 2026-09-13, 2755 models) lacks zai/glm-4.7-flash and zai/glm-4.5-flash which the live LiteLLM catalog prices at $0.18/M input (fetched 2026-09-30) — those sessions silently fall to fallback pricing; doctor discloses snapshot age only (doctor.rs::L131-L135), no drift DETECTION exists
- acceptance: snapshot regenerated from live LiteLLM with bumped _snapshot date/version; glm-4.7-flash and glm-4.5-flash sessions price from the catalog, not fallback; doctor (or overview) cross-references unknown/fallback-priced models against a fetched live catalog and reports drift, degrading gracefully offline; snapshot regeneration is scripted and repeatable
- evidence: regenerated snapshot committed with _snapshot meta updated; live-run output showing both zai models priced; offline doctor run unchanged (no network failure surfaced); cargo test green

### Point install.sh at fork releases and enforce the glibc 2.35 baseline
- id: `rm-063` | track: reliability | priority: 73.0 | status: candidate
- signals: reliability.installer_skew:install.sh::L7 (REPO="luoyuctl/agenttrace") and install.sh::L44 (RELEASE_URL defaults to that repo's releases/latest = v0.9.0, whose binary requires GLIBC_2.39 per objdump) — loader failure on Ubuntu 22.04/glibc 2.35, the same class recorded for v0.8.1, while this fork's own CI moved Test-and-build to ubuntu-22.04 to enforce exactly the 2.35 baseline (HEAD 90a4ef5)
- acceptance: installer defaults to codeo1io/agenttrace releases (or a documented build-from-source fallback) so `curl | sh` never silently installs an upstream binary lacking the fork's hardening; checksum verification retained; on an incompatible glibc the installer fails with a truthful diagnostic naming the required glibc instead of a loader error
- evidence: scripts/ci/check-install-runtime.sh green (offline behavioral gate; sh -n under dash included); live install probe in a glibc-2.35 (ubuntu-22.04) environment recorded in the shipping PR

### Bound cache and history write costs
- id: `rm-064` | track: reliability | priority: 70.0 | status: candidate
- signals: reliability.double_serialization:crates/agenttrace-core/src/session_cache.rs::L710-L716 (enforce_byte_bound builds by_age via cache_paths_sized_once, serializer at :L645, BEFORE the total<=max early-out) then save_session_cache serializes everything again — ~2x full-cache JSON serialization per save on a large cache; reliability.unbounded_history:crates/agenttrace-core/src/history.rs::L37-L47 (preserve_derived_history inserts into an unbounded map and rewrites the whole pretty-printed file on every opted-in run)
- acceptance: under-bound saves skip the full by_age build (cheap size proxy checked first) — exactly one serialization per save when far under the byte bound, proven by test or instrumented count; history.json gains a retention cap (bounded entries or bytes) with documented eviction; a long-history run's rewrite cost stays O(cap)
- evidence: new tests for the early-out and cap eviction; serialization-count or timing measurement before/after on a large fixture cache recorded in the PR body; cargo test green

### Truthful --statusline-report format error
- id: `rm-065` | track: customer-experience | priority: 64.0 | status: candidate
- signals: dx.misleading_format_error:crates/agenttrace-cli/src/main.rs::L176-L182 (format guard's action list omits statusline_report, so `--statusline-report -f html` and `-f markdown` fail with "markdown and html formats require --overview or a governance report action" — live-verified for both formats)
- acceptance: either html/markdown output is supported for statusline reports, or the error names the real constraint (statusline-report supports text and json only); CLI help documents the supported format set per action; a test covers the guard for every action/format combination
- evidence: live CLI runs of both formats with the new output pasted in the shipping PR; cargo test green including the new guard test

### Decode zstd-compressed Codex rollouts in place
- id: `rm-066` | track: customer-experience | priority: 58.0 | status: candidate
- signals: dx.zst_rollout_undecoded:crates/agenttrace-core/src/parser.rs::L32-L38 (zstd magic detected, user told to "decompress it first"; no zstd decoder dependency in tree); forward-looking demand — local corpus today is 96 rollout files, all .jsonl, 0 .zst (Codex ≥0.152 writes zstd frames)
- acceptance: .zst rollouts parse transparently via a pure-Rust streaming decoder (e.g. ruzstd — no C dependency, consistent with the offline/deny posture); the manual-decompress diagnostic is replaced by real decode; a crafted zst-encoded rollout fixture parses correctly in tests; cargo-deny posture stays clean and no C toolchain requirement is added
- evidence: new fixture test green in cargo test -p agenttrace-core; cargo deny check clean; clean-runner build log in the PR showing no system zstd requirement

### Antigravity SQLite conversation source (deferred pending corpus)
- id: `rm-067` | track: customer-experience | priority: 54.0 | status: candidate
- signals: feature.antigravity_source:ccusage v20.0.21 (2026-09-17) reads conversation usage from SQLite .db files under ~/.gemini/antigravity{,-cli,-ide,-backup}/conversations and ~/.config/antigravity/conversations (gen_metadata table, steps schema, protobuf trajectory metadata — verified in their rust/adapters/antigravity/paths.rs); upstream user-need issue #236 (Antigravity transition) OPEN; NO antigravity corpus exists on this box, so behavior would be unverifiable today
- acceptance: gated on a real corpus being located (user-supplied or a published-schema CI fixture) — until then this item stays deferred and must not be implemented blind; once gated: discovery lists antigravity roots, sessions parse with model/cost attribution modeled on sqlite_sessions.rs, protobuf trajectory metadata decoded or explicitly skipped with a named reason, fixture-driven tests green
- evidence: corpus provenance documented in the implementing PR before any code lands; fixture-driven tests green; discovery/doctor output showing the new source on a machine with real antigravity dirs

## Rejected / deferred candidates (2026-09-30 research)

Recorded so later cycles do not re-litigate; re-open only on a changed premise.

- Grok Build CLI source adapter (ccusage v20.0.20): no local corpus, no open user-need issue — reject.
- Desktop app (codeburn v0.9.25 asset): scope mismatch with the TUI focus — reject.
- Dependency-refresh wave (rusqlite 0.32→0.40.2, clap 4.5→4.6.7, crossterm 0.28→0.29, ureq 2.12→3.4.2): churn without a driver — fold into the next natural touch of each dependency.
- OTel GenAI export: semantic conventions still Development status (verified 2026-09-30) — keep tracking, do not adopt.
- Codex originator usage breakdowns (ccusage): competitor-mirroring without a user ask; natural add-on if rm-058/rm-061 land — defer.
- Claude cross-session copied-request dedupe (ccusage): same dedupe class as rm-055 — defer to rm-055's design.

<!-- New items rm-055..rm-067 + rejected-candidate record appended 2026-09-30 by run 83642957d130 roadmap phase (repository-maintenance 902db834 cycle 1, attempt a39f9396) at HEAD 90a4ef5. Sources: this run's assess phase (attempt 8c5e8e4f, findings F1-F9, each live-probed with the tree's own release binary) and research phase (attempt 1899d5f7, keeps K1-K6 -> rm-058/rm-061/rm-062/rm-063/rm-066/rm-067, rejects R1-R6 recorded above). IDs minted rm-055+ past every live claimant verified 2026-09-30: origin/master 9d88b36 holds rm-001..rm-027 + rm-033; PR #19 branch conductor/run-c7dca75d30e3 @ 4ef993c holds through rm-034; in-flight worktrees run-52465b9e5642 (rm-034..rm-039), run-cbe30a9c42e5 (rm-025..rm-034), run-0a279c440c10 (rm-025..rm-054). rm-IDs are campaign-local until integration; if any claimant lands rm-055+ first, renumber at the commit phase (record-not-renumber now). No existing item or status was modified. -->

<!-- managed by hermes-roadmap render; do not edit by hand -->
