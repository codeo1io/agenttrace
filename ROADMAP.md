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

<!-- cycle 1 (campaign e7206fe5): IDs are campaign-local — numbered past sibling campaign cc2f32d5's uncommitted rm-003..rm-008, but 2326f88e's campaign separately added rm-009..rm-011 in ITS worktree; the ID overlap must be reconciled at the merge/commit phase -->

### Port upstream v0.9.0 onto the fork and stand up a fork-vs-upstream delta report
- id: `rm-009` | track: reliability | priority: 88.0 | status: implemented
- signals: reliability.upstream_drift:crates/agenttrace-core/src/diagnostics.rs::L9,L15 (fork loop_cost/stuck_patterns vs upstream v0.9.0 loop_fingerprints via PR #286); reliability.upstream_drift:crates/agenttrace-core/src/session_cache.rs (upstream +/-32 in the v7-fix file); fork branch conductor/run-2ae192428c54 ~55 commits behind main 90a4ef5; upstream v0.9.0 released 2026-09-29T20:46Z, active #287-290 on 2026-09-30T03:48Z
- acceptance: (1) PR #286 (Codex cost double-count fix, loop_fingerprints model, ATTENTION_* triage thresholds) and #284 (Oh My Pi leading-non-session-lines parser fix) content present on the rebased fork branch with session_cache.rs conflicts resolved preserving the v7 fix's semantics; (2) a scripts/upstream-delta report (git log --oneline fork-base..upstream/master -- per-file) regenerates on demand into the spool; (3) full suite green at the new head
- evidence: cargo test -q green at rebased head; upstream-delta report artifact in the delegate spool; conductor validation digest validation:v1:<sha> recorded in the shipping PR
- cycle 1 (e7206fe5): EXECUTED — merge of v0.9.0 (3 conflicts resolved: session_cache.rs schema 17/19→20 with all fork bounds machinery preserved, lib.rs title-cleaning composed as a strict superset, parser.rs upstream fn spliced whole), 13 files +1303/−333 staged uncommitted at HEAD 90a4ef5 with MERGE_HEAD = v0.9.0; scripts/upstream-delta landed (+69 lines, --json/--fetch); full suite 240/240 (targeted 4a68e1b1 + full 2cc1c189, digest validation:v1:c4983b24…); procedure compounded to docs/solutions/workflow-issues/port-upstream-release-preserving-fork-semantics.md; status flip to implemented deferred to the commit gate

### Re-verify cycle-1 assess findings against post-v0.9.0 upstream before implement spend
- id: `rm-010` | track: reliability | priority: 85.0 | status: implemented
- signals: reliability.anchor_drift:crates/agenttrace-tui/src/explorer.rs (upstream #286 rewrote +618/-164 while cycle-1 findings AF-1 (app.rs:1786-1799 cfg(test) dual renderer) and AF-2 (app.rs:1371 write-only overview recompute per keystroke) were assessed at 90a4ef5); blocked-by rm-009
- acceptance: every open assess finding (AF-1..AF-3 plus any cross-campaign batch item touching ported files) has a written disposition after the rm-009 rebase — re-anchored with new line evidence, dropped as upstream-fixed with the fixing PR cited, or kept verbatim — recorded in a verification table before any implement phase spends budget on those items
- evidence: disposition table in the delegate spool citing per-finding diffs against post-rebase HEAD; suite green; conductor validation digest validation:v1:<sha> recorded in the shipping PR
- cycle 1 (e7206fe5): EXECUTED — disposition table filed at spool/cycle1-disposition-8ef6659c-2026-09-30.md: AF-1 → app.rs:1792-1802 (re-anchored), AF-2 → 1346/1398 (re-anchored), AF-3 → search.rs:9 (byte-identical, unaffected by #286); all three findings SURVIVE the port with fresh line evidence, so rm-011/rm-012/rm-013 anchors are implement-ready next cycle; status flip deferred to the commit gate

### Eliminate the TUI dual-renderer test/production binding
- id: `rm-011` | track: reliability | priority: 82.0 | status: candidate
- signals: reliability.test_prod_divergence:crates/agenttrace-tui/src/app.rs::L1786-1799 (mod presentation cfg(test) vs mod shared cfg(not(test)); use presentation::* under test, use shared::* in production); reliability.dead_code:crates/agenttrace-tui/src/shared.rs (26 helpers compiled only in production yet exercised only by the test copy at drifted semantics — shared.rs top_anomaly_driver counts anomalies as sessions where presentation.rs dedupes per session)
- acceptance: one helper implementation compiled identically under test and production (either unify shared.rs/presentation.rs or migrate tests off the legacy renderer and delete it), with a parity test asserting identical outputs for the duplicated helper set; no allow(dead_code) remains on the duplicated symbols
- evidence: cargo test -q green including the parity test; grep shows a single definition per duplicated helper; conductor validation digest validation:v1:<sha> recorded in the shipping PR

### Drop write-only overview recompute from the refresh_filtered hot path
- id: `rm-012` | track: reliability | priority: 76.0 | status: candidate
- signals: reliability.hot_path_waste:crates/agenttrace-tui/src/app.rs::L1371 (refresh_filtered computes OverviewDerived + self.overview per keystroke; self.overview has zero production readers at app.rs:1367; includes deep clone of every visible session and ~13 linear passes)
- acceptance: refresh_filtered performs no OverviewDerived/self.overview computation per keystroke (removed, or computed once per filter-commit / lazily on first read); a test or benchmark demonstrates the per-keystroke work drops from O(visible sessions) clone to O(matching) filter; visible behavior unchanged
- evidence: cargo test -q green; hot-path before/after measurement (criterion or timing harness) attached to the shipping PR; conductor validation digest validation:v1:<sha> recorded in the shipping PR

### Unicode-aware case folding in CLI search and TUI filters
- id: `rm-013` | track: reliability | priority: 68.0 | status: candidate
- signals: reliability.ascii_case_folding:crates/agenttrace-core/src/search.rs::L9 (to_ascii_lowercase) and crates/agenttrace-tui/src/filters.rs contains/add_match (ASCII-only fold) — non-ASCII queries silently miss case-variant matches
- acceptance: case-insensitive matching in the CLI search path and TUI filters uses Unicode-aware folding; tests with non-ASCII case-variant queries (e.g. full-width Latin, Cyrillic, Turkish dotted-I documented as out-of-scope or handled) pass on both paths
- evidence: new tests green in cargo test -q; conductor validation digest validation:v1:<sha> recorded in the shipping PR

### Refresh the install surface to the moved upstream distribution
- id: `rm-014` | track: reliability | priority: 74.0 | status: candidate
- signals: reliability.stale_install_source:install.ps1::L10 ($REPO pinned while bare npm name agenttrace 404s on registry.npmjs.org as of 2026-09-30 — upstream moved to @zack78/agenttrace + GitHub Release binaries with checksums + winget Luoyuctl.AgentTrace + brew luoyuctl/tap)
- acceptance: install.ps1 and install.sh download from a pinned GitHub release artifact with checksum verification (extending the install-verification mechanism already planned cross-campaign), docs name the current package surfaces, and a dry-run/manual install on each target OS succeeds from a clean environment
- evidence: install scripts fetch+verify a pinned release in a clean-room test (CI job or recorded manual run); docs diff shows only the distribution section; conductor validation digest validation:v1:<sha> recorded in the shipping PR

### Mirror upstream's CI governance wave on the fork
- id: `rm-015` | track: reliability | priority: 72.0 | status: candidate
- signals: reliability.governance_gap: fork ci/ lane lacks cargo-deny, OpenSSF Scorecard, coverage/Codecov and git-cliff changelog CI that upstream added 2026-09-30 (#287-290)
- acceptance: the four governance additions run green on the fork (cargo-deny advisories+licenses config committed, Scorecard or equivalent badge lane, coverage reporting wired, git-cliff changelog generated on release); no existing gate weakened
- evidence: four green CI lanes on the fork's first post-change commit; deny.toml + scorecard/coverage config in-tree; conductor validation digest validation:v1:<sha> recorded in the shipping PR

### Add an `agenttrace gate` CI exit-code capability riding the post-#286 triage model
- id: `rm-016` | track: reliability | priority: 60.0 | status: candidate
- signals: reliability.missing_capability: no gate/exit-code subcommand in src/main.rs while PR #286 stabilizes triage semantics (ATTENTION_FAIL_MIN 3 / FAIL_RATE 0.2 / COST_USD 10 / P95_GAP_SEC 120, first-matching-signal ordering) — the CI-gates lane the 2026-09-02 ideation ledger named uncontested but never converted
- acceptance: `agenttrace gate --max-cost USD --max-fail-rate R <session-dir>` exits 0/1 listing the offending sessions, thresholds documented and defaulting to the post-#286 triage constants; golden tests cover pass/fail boundary cases; parked until rm-009 lands so the gate rides the stable triage model
- cycle 1 (e7206fe5): rm-009 content executed (staged merge) — UNPARKED for cycle-2 scoping, still gated on the commit landing the port
- evidence: subcommand tests green in cargo test -q; example CI snippet in docs; conductor validation digest validation:v1:<sha> recorded in the shipping PR

<!-- managed by hermes-roadmap render; do not edit by hand -->
