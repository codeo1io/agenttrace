# agenttrace — Roadmap

> Autonomously maintained by the roadmap sync (reliability-first). Items cite reproducible codebase signals; acceptance is proven by cited evidence.

**Vision**: A reliable, customer-friendly repository advanced by evidence-cited roadmap cycles owned by the autonomy loop

**Pillars**: reliability work outranks customer-experience work; every roadmap item cites reproducible codebase signals; acceptance is proven by cited evidence, never claimed

## Fleet context

- dependents (changes here affect): (host), agent, maestro
- graph: evidence-derived (imports/refs/deploy surfaces); advisory

## Open items

> **compound c1 (2026-09-30, run 0a279c44, pre-review):** cycle-1 batch "Trustworthy token accounting on hostile journals" = rm-046 + rm-047 implemented at HEAD 9d88b36 (uncommitted worktree state, review/shipping pending). Recorded outcomes: core-lib red 99/4 → green 103/0; targeted gate RC=0 (envelope result-430870-326193046.json) and full gate RC=0 (envelope result-493778-326216181.json), each 268 passed / 0 failed / 0 ignored across 10 suites; declared digest validation:v1:155d1e3d82ccd7bc7369ace33562cface32567490ee3cde875121137d0a6c738 at base 9d88b36 re-derived byte-exact pre- and post-gate. Cycle context compounded to docs/stewardship/2026-09-30-cycle1-implementation-record-run0a279c44.md, docs/reviews/2026-09-30-adversarial-repository-assessment-pass12.md, docs/research/2026-09-30-extensions-research-pass10.md; prevention rule for the roadmap id-collision class in docs/solutions/workflow-issues/roadmap-campaign-id-collision-at-integration.md. Cycle-2 leads: rm-051 (clean, offline gate scripts/ci/check-install-runtime.sh exists), rm-020 (per-model pricing anchor), rm-053 (conformance harness; its first case is now the landed rm-046 overflow test).

### Add test coverage for 1 untested module(s)
- id: `rm-002` | track: reliability | priority: 83.0 | status: done
- signals: reliability.no_tests:scripts/fixtures/make-adversarial-sqlite.py
- acceptance: Every module in ['scripts/fixtures/make-adversarial-sqlite.py'] has a corresponding test file with at least one passing test
- evidence: full suite green (python -m pytest -q) at HEAD; conductor validation digest validation:v1:<sha> recorded in the shipping PR
- compound c1 2026-09-30: implemented in the campaign worktree (uncommitted, pre-review) as scripts/fixtures/test_make_adversarial_sqlite.py — first pytest convention in this repo, 5 tests; recorded outcomes: targeted gate 5 passed, full workspace suite 251/0. Flipped to done at the commit gate 2026-09-30: independent review APPROVED (five-lens, zero fixes), full workspace 251/0, targeted 5 passed, digest validation:v1:bde0b51bf06c9c706fef41075750 at base 90a4ef5, shipped in this commit
- integration 2026-10-01 (conflict case 55e6e239, merge of 334b5a8): run 30484632 implemented rm-002 independently (7 tests, add/add on the same path); suite reconciled to the union — 10 tests, all green on this host (committed-fixture shape + exact stdout announcements from fe8b316; module-form regeneration, schema/session-row shape, run-to-run byte-determinism, script-form coverage from 334b5a8). Its regenerated-vs-committed raw-byte equality was made portable as schema+row dump comparison: SQLite stamps the writing library version into the file header (committed fixtures written by sqlite 3.53.1; this host regenerates with 3.37.2 — only header bytes 96-99 differed, content identical)

### Refactor 3 high-complexity function(s)
- id: `rm-001` | track: reliability | priority: 79.0 | status: candidate
- signals: reliability.complexity_hot:npm/scripts/install.js::L24, reliability.complexity_hot:npm/scripts/install.js::L28, reliability.complexity_hot:npm/scripts/install.js::L40
- acceptance: Each flagged function is decomposed below the branch threshold with behavior locked by characterization tests
- evidence: ast-based branch-count check passes at HEAD (full suite green; conductor validation digest validation:v1:<sha> recorded in the shipping PR)

### Decode Codex zstd rollouts in-process

- id: rm-003
- track: compatibility
- priority: high
- status: candidate
- signals: compatibility.rejected_format:crates/agenttrace-core/src/parser.rs::L32, compatibility.version_drift:codex-cli >=0.152 zstd rollout frames
- acceptance: A zstd-framed Codex rollout fixture (magic 28 B5 2F FD) parses transparently with no manual decompress step; decoding uses a pure-Rust crate (e.g. ruzstd) so static MSVC/MUSL builds gain no C toolchain dependency; doctor reports the Codex format acceptance rate; existing JSONL fixtures unchanged and suite green
- evidence: crates/agenttrace-core/src/parser.rs:32-40 bails with "decompress it to JSONL first" at HEAD 90a4ef5; Codex latest release rust-v0.159.2 published 2026-09-29 (GitHub API, fetched 2026-09-30, research pass 7); local codex 0.147.0 still writes JSONL, so the defect is forward compatibility for mainstream-current CLI users

### Make diagnostics and waste reporting truthful

- id: rm-004
- track: correctness
- priority: high
- status: implemented
- signals: correctness.over_reporting:crates/agenttrace-core/src/diagnostics.rs::L660, correctness.baseline_poisoning:crates/agenttrace-core/src/diagnostics.rs::L428, correctness.bucket_miscount:crates/agenttrace-core/src/diagnostics.rs::L730, correctness.weak_dedup_key:crates/agenttrace-core/src/diagnostics.rs::L866, correctness.double_count:crates/agenttrace-core/src/waste.rs::L54, correctness.cost_fabrication:crates/agenttrace-core/src/waste.rs::L239
- acceptance: Each of the six assessed defects is fixed or its display explicitly annotated as an estimate - loop_groups reports an actual group count; predict_cost_anomaly excludes zero-cost sessions from its baseline or labels the result; tool_latencies stops counting unmatched calls as timeouts and no longer drops calls >=3600s; repeated_response keys on full response identity, not a 50-char prefix; stuck detection counts each long-gap pattern once; per-tool cost comes from tool-share data or renders as a session-level estimate, never fabricated per-tool currency; every changed diagnostic carries a golden-value test pinning expected output on a fixture session
- evidence: assess phase b2488fec at HEAD 90a4ef5 (9 findings, 6 in this cluster); anchors re-verified with fresh pinned greps (diagnostics.rs:660, :428-441, :727-733, :866-877; waste.rs:54 and :56-70, :239-247 rendered at :324-334; consumers presentation.rs:2398 and :2412-2424); CYCLE-1 IMPLEMENTED pre-review (run cc2f32d5, implement attempt f7c2990d, worktree run-cc2f32d56918-cc2f32d5 @ 90a4ef5): all six defects fixed - loop_groups counts distinct consecutive-tool runs >=3 (diagnostics.rs:631/:644/:655/:679), predict_cost_anomaly baseline excludes cost_estimated==0.0 sessions (:432), the >=3600s latency cap is removed and the field renamed timeouts->unmatched (:66; consumers governance.rs, explorer.rs, presentation.rs), repeated_response keys on full message content (no 50-char prefix), stuck long-gaps scored exactly once (waste.rs:63-68), per-tool bloat cost renders as a labeled call-count allocation of WasteReport.session_cost (waste.rs:245, :351 'N% of session'); six golden-value tests added in-module (loop_groups_counts_distinct_runs_not_a_flag, cost_anomaly_baseline_ignores_unpriced_sessions, tool_latencies_keep_long_calls_and_report_unmatched, repeated_response_requires_identical_full_content, stuck_long_gaps_scored_once, per_tool_cost_is_allocated_share_of_session_cost); validation: targeted gate 243/243 (result-3832098-324310608.json) and full-command gate 243/243 returncode 0 (result-3934489-324343777.json); changes uncommitted in the worktree awaiting the commit phase

### Verify downloaded artifacts in install.ps1

- id: rm-005
- track: security
- priority: high
- status: implemented
- signals: security.unverified_download:install.ps1::L51, supply_chain.parity_gap:install.ps1 vs install.sh and npm/scripts/install.js
- acceptance: install.ps1 fetches the .sha256 sidecar emitted by release.yml for the Windows asset, compares via Get-FileHash, and refuses to install on mismatch or missing sidecar; a parity check names the sidecar each of the three installers verifies (install.sh, install.ps1, npm/scripts/install.js)
- evidence: install.ps1:37-51 downloads via Invoke-WebRequest with no hash check and only probes at :62; install.sh:57-72 verifies the sidecar and names the parity intent; npm/scripts/install.js verifies SHA-256 pre-install; release.yml emits .sha256 for every asset including Windows (assess A11-9, re-verified at HEAD 90a4ef5); upstream distributes npm-only with zero GitHub releases or tags, making the sidecar channel this fork's differentiating trust anchor; CYCLE-1 IMPLEMENTED pre-review (run cc2f32d5, implement attempt f7c2990d): install.ps1:62-71 fetches $url.sha256 into a temp sidecar, validates the digest is a 64-hex token, compares via Get-FileHash, and refuses the install (throw -> catch -> exit 1) on missing, malformed, or mismatched sidecar BEFORE Move-Item, sidecar temp cleaned in finally (pwsh Parser::ParseFile PARSE OK); parity locked by agenttrace-core test all_three_installers_verify_sha256_sidecars (lib.rs:1620) asserting the sidecar mechanics of install.sh, install.ps1, and npm/scripts/install.js together; validation: full-command gate 243/243 returncode 0 (result-3934489-324343777.json); changes uncommitted in the worktree awaiting the commit phase (Windows-side smoke of the gate still owed at final_validation)

### Establish pricing snapshot refresh cadence and drift surfacing

- id: rm-006
- track: data-freshness
- priority: medium
- status: candidate
- signals: data_freshness.stale_snapshot:crates/agenttrace-core/src/pricing.rs::L16, data_freshness.catalog_drift:LiteLLM main vs bundled snapshot
- acceptance: The bundled snapshot refreshes on a stated cadence (per maintenance cycle at minimum) using the landed refresh mechanism; doctor reports snapshot age and entry delta against upstream LiteLLM; --overview notes pricing confidence when the snapshot is older than the cadence allows
- evidence: pricing.rs:16 pins the snapshot at 2026-09-13 with 2,755 chat-priced entries at HEAD 90a4ef5; LiteLLM main counted 4,436 entries / 3,680 costed on 2026-09-30 (research pass 7); the refresh tooling already landed per CHANGELOG pass 9 - the gap is cadence ownership and user-visible drift, not mechanism

### Stage a dependency refresh wave

- id: rm-007
- track: dependencies
- priority: medium
- status: candidate
- signals: dependency.drift:Cargo.toml rusqlite 0.32 to 0.40.1, ratatui 0.30 to 0.31.0, crossterm 0.28 to 0.29.0, ureq 2.12 to 3.4.2, clap to 4.5.48
- acceptance: Refresh lands in stages - minors first (clap, crossterm, ratatui), then rusqlite 0.40 with the full adversarial-sqlite fixture regression, then the ureq 3 rewrite last with its API migration; each stage pins exact versions re-verified against the registry at bump time (the 0.40.2 and 4.6.7 pins from the prior research doc were not reproducible on 2026-09-30 - registry yank churn); static build profile unchanged and suite green per stage
- evidence: crates.io max_versions fetched 2026-09-30 (research pass 7): rusqlite 0.40.1, ureq 3.4.2, clap 4.5.48, crossterm 0.29.0, ratatui 0.31.0; repo pins at HEAD 90a4ef5: rusqlite 0.32, ureq 2.12, crossterm 0.28, ratatui 0.30

### Prune the API surface before any 1.0 lock

- id: rm-008
- track: api-hygiene
- priority: medium
- status: candidate
- signals: api_hygiene.dead_exports:crates/agenttrace-core/src/lib.rs::L33, api_hygiene.suppressed_lint:crates/agenttrace-tui dead_code allow, api_hygiene.dead_code:crates/agenttrace-tui/src/doctor.rs::L191
- acceptance: The five dead re-exports in lib.rs are consumed or removed; the TUI dead_code allow is removed and the workspace compiles clean under clippy -D dead_code; doctor.rs count_by_root is consumed or deleted; removals land before a 1.0 semver lock (removals after it are breaking)
- evidence: assess A11-7 and A11-8 at HEAD 90a4ef5, re-verified: lib.rs:33 re-export block exports five never-consumed symbols; doctor.rs:191 count_by_root written-never-read; the TUI crate carries a crate-level dead_code suppression

<!-- New items 2026-09-30 (run 2326f88e research, attempt 3c3a6d95): numbered rm-009+ to stay disjoint from sibling campaign cc2f32d5's uncommitted rm-003..rm-008 -->

### Patch RUSTSEC-2026-0285: ship rustls >= 0.23.45 in release binaries
- id: `rm-009` | track: reliability | priority: 88.0 | status: done
- signals: security.advisory:RUSTSEC-2026-0285:Cargo.lock::L1633 (rustls 0.23.42 locked, reached via ureq 2.12.1 pricing transport; upstream fix commit 56df4c47 "cargo update -p rustls 0.23.42 -> 0.23.45" merged 2026-09-30T02:58Z)
- acceptance: Cargo.lock resolves rustls >= 0.23.45 with no other dependency regressions; cargo tree -i rustls still shows the single pricing-fetch path; release artifacts rebuilt against the patched lock
- evidence: cargo audit (or cargo-deny advisories check) reports zero RUSTSEC-2026-0285 findings at HEAD; full suite green; conductor validation digest validation:v1:<sha> recorded in the shipping PR
- cycle-1 (2026-09-30, run 2326f88ec8b4): implemented uncommitted in worktree run-2326f88ec8b4-2326f88e — rustls 0.23.42→0.23.45 (plus companion webpki 0.103.13→0.103.15, 8-line lock diff); full suite green (core 52, tui 45, entrypoints 10, doc-tests clean, digest validation:v1:403a28e2…); local cargo-deny advisories ok on the patched lock; learning recorded in docs/solutions/security-issues/rustsec-advisory-shipped-via-ungated-lockfile.md; status flipped to done by the commit gate

### Adopt upstream supply-chain hardening wave (cargo-deny + OpenSSF Scorecard)
- id: `rm-010` | track: reliability | priority: 85.0 | status: done
- signals: reliability.ci_gap:.github/workflows (no cargo-deny, deny.toml, or OpenSSF Scorecard lanes; the pattern landed upstream as PRs #287/#288/#289/#290, merged 2026-09-30T01:53Z-03:38Z: CodeRabbit+Codecov, cargo-deny+deny.toml+Scorecard, git-cliff, badges)
- acceptance: deny.toml enforces advisories + licenses + sources + bans; a cargo-deny CI job gates every PR and push; an OpenSSF Scorecard workflow publishes results (SARIF); the fork's own lock passes all deny checks
- evidence: CI green including the new deny job at HEAD; Scorecard run artifact published; full suite green; conductor validation digest validation:v1:<sha> recorded in the shipping PR
- cycle-1 (2026-09-30, run 2326f88ec8b4): deny.toml added (yanked=deny, wildcards=deny, unknown sources denied) + third CI job `deny` (cargo-deny-action pinned SHA, --all-features) in ci.yml — committed by the cycle-1 commit gate with rm-009; local cargo-deny 0.20.2 check all four categories ok; Scorecard DEFERRED with rationale (private fork remote — Scorecard scores public repos only), revisit if repo goes public; next unexercised step: the deny job's first remote run (CI gate)
- cycle-2 truth-correct (2026-09-30, run 30484632 assess, attempt f69de232): both cycle-1 closure caveats are now FALSE — the remote is PUBLIC (gh repo view codeo1io/agenttrace --json isPrivate -> false, live 2026-09-30) and the deny job HAS run remotely green twice (CI runs 36677489765 pull_request @ b898caa, 36678452544 push @ 7bb4dcb); the Scorecard deferral rationale is void, enablement landed as rm-155

### Report-contract truthfulness batch (search truncation disclosure, rates.total, unknown-reason labels)
- id: `rm-011` | track: reliability | priority: 81.0 | status: candidate
- signals: correctness.contract_gap:crates/agenttrace-core/src/search.rs::L203 (search JSON omits total_matches/truncated; per-session cap 8; --sort/--order unapplied on the search path, main.rs::L409-419), correctness.constant_field:crates/agenttrace-core/src/governance.rs::L97+L104 (rates_per_million_usd.total serialized as constant 0.0 while components.total is computed at L106; serialization sites L326/L434), correctness.fallback_mislabel:crates/agenttrace-tui/src/i18n.rs::L147-L155 (unknown inspect_reason values render as "ok"/"正常" while every sibling fallback says "unknown ...")
- acceptance: search JSON carries total_matches and truncated fields and either applies --sort/--order or exits with an explicit unsupported-flag error; rates_per_million_usd.total is computed from its components or the field is removed across all cost-audit serializations; inspect_reason fallback labels unknown reasons as unknown in every locale
- evidence: golden JSON tests covering search truncation disclosure, rates-total computation, and reason-fallback rendering; full suite green; conductor validation digest validation:v1:<sha> recorded in the shipping PR
- cycle-2 anchor: deliberately deferred this cycle (prioritization a818550b — three sub-defects × golden-test scaffolding ≈ a full session alone); untouched this cycle, anchors assess AF-1..AF-3
<!-- cycle 1 (campaign e7206fe5) - integrated 2026-09-30 (run 02993de2 merge into master): campaign-local IDs renumbered rm-009>rm-012, rm-010>rm-013, rm-011>rm-014, rm-012>rm-015, rm-013>rm-016, rm-014>rm-017, rm-015>rm-018, rm-016>rm-019, because 2326f88e's rm-009..rm-011 (rustls, cargo-deny/Scorecard, report-contract truthfulness) landed on master first and keep their IDs; item bodies otherwise verbatim -->

### Port upstream v0.9.0 onto the fork and stand up a fork-vs-upstream delta report
- id: `rm-012` | track: reliability | priority: 88.0 | status: implemented
- signals: reliability.upstream_drift:crates/agenttrace-core/src/diagnostics.rs::L9,L15 (fork loop_cost/stuck_patterns vs upstream v0.9.0 loop_fingerprints via PR #286); reliability.upstream_drift:crates/agenttrace-core/src/session_cache.rs (upstream +/-32 in the v7-fix file); fork branch conductor/run-2ae192428c54 ~55 commits behind main 90a4ef5; upstream v0.9.0 released 2026-09-29T20:46Z, active #287-290 on 2026-09-30T03:48Z
- acceptance: (1) PR #286 (Codex cost double-count fix, loop_fingerprints model, ATTENTION_* triage thresholds) and #284 (Oh My Pi leading-non-session-lines parser fix) content present on the rebased fork branch with session_cache.rs conflicts resolved preserving the v7 fix's semantics; (2) a scripts/upstream-delta report (git log --oneline fork-base..upstream/master -- per-file) regenerates on demand into the spool; (3) full suite green at the new head
- evidence: cargo test -q green at rebased head; upstream-delta report artifact in the delegate spool; conductor validation digest validation:v1:<sha> recorded in the shipping PR
- cycle 1 (e7206fe5): EXECUTED — merge of v0.9.0 (3 conflicts resolved: session_cache.rs schema 17/19→20 with all fork bounds machinery preserved, lib.rs title-cleaning composed as a strict superset, parser.rs upstream fn spliced whole), 13 files +1303/−333 staged uncommitted at HEAD 90a4ef5 with MERGE_HEAD = v0.9.0; scripts/upstream-delta landed (+69 lines, --json/--fetch); full suite 240/240 (targeted 4a68e1b1 + full 2cc1c189, digest validation:v1:c4983b24…); procedure compounded to docs/solutions/workflow-issues/port-upstream-release-preserving-fork-semantics.md; status flip to implemented deferred to the commit gate

### Re-verify cycle-1 assess findings against post-v0.9.0 upstream before implement spend
- id: `rm-013` | track: reliability | priority: 85.0 | status: implemented
- signals: reliability.anchor_drift:crates/agenttrace-tui/src/explorer.rs (upstream #286 rewrote +618/-164 while cycle-1 findings AF-1 (app.rs:1786-1799 cfg(test) dual renderer) and AF-2 (app.rs:1371 write-only overview recompute per keystroke) were assessed at 90a4ef5); blocked-by rm-012
- acceptance: every open assess finding (AF-1..AF-3 plus any cross-campaign batch item touching ported files) has a written disposition after the rm-012 rebase — re-anchored with new line evidence, dropped as upstream-fixed with the fixing PR cited, or kept verbatim — recorded in a verification table before any implement phase spends budget on those items
- evidence: disposition table in the delegate spool citing per-finding diffs against post-rebase HEAD; suite green; conductor validation digest validation:v1:<sha> recorded in the shipping PR
- cycle 1 (e7206fe5): EXECUTED — disposition table filed at spool/cycle1-disposition-8ef6659c-2026-09-30.md: AF-1 → app.rs:1792-1802 (re-anchored), AF-2 → 1346/1398 (re-anchored), AF-3 → search.rs:9 (byte-identical, unaffected by #286); all three findings SURVIVE the port with fresh line evidence, so rm-014/rm-015/rm-016 anchors are implement-ready next cycle; status flip deferred to the commit gate

### Eliminate the TUI dual-renderer test/production binding
- id: `rm-014` | track: reliability | priority: 82.0 | status: candidate
- signals: reliability.test_prod_divergence:crates/agenttrace-tui/src/app.rs::L1786-1799 (mod presentation cfg(test) vs mod shared cfg(not(test)); use presentation::* under test, use shared::* in production); reliability.dead_code:crates/agenttrace-tui/src/shared.rs (26 helpers compiled only in production yet exercised only by the test copy at drifted semantics — shared.rs top_anomaly_driver counts anomalies as sessions where presentation.rs dedupes per session)
- acceptance: one helper implementation compiled identically under test and production (either unify shared.rs/presentation.rs or migrate tests off the legacy renderer and delete it), with a parity test asserting identical outputs for the duplicated helper set; no allow(dead_code) remains on the duplicated symbols
- evidence: cargo test -q green including the parity test; grep shows a single definition per duplicated helper; conductor validation digest validation:v1:<sha> recorded in the shipping PR

### Drop write-only overview recompute from the refresh_filtered hot path
- id: `rm-015` | track: reliability | priority: 76.0 | status: candidate
- signals: reliability.hot_path_waste:crates/agenttrace-tui/src/app.rs::L1371 (refresh_filtered computes OverviewDerived + self.overview per keystroke; self.overview has zero production readers at app.rs:1367; includes deep clone of every visible session and ~13 linear passes)
- acceptance: refresh_filtered performs no OverviewDerived/self.overview computation per keystroke (removed, or computed once per filter-commit / lazily on first read); a test or benchmark demonstrates the per-keystroke work drops from O(visible sessions) clone to O(matching) filter; visible behavior unchanged
- evidence: cargo test -q green; hot-path before/after measurement (criterion or timing harness) attached to the shipping PR; conductor validation digest validation:v1:<sha> recorded in the shipping PR
- integration note (2026-09-30, run 3099db98 merge): rm-023 (incremental explorer index, landed candidate-side) flags itself as the architectural fix superseding this narrower dead-recompute removal; the supersession decision is deliberately deferred to cycle-2 planning per that item's anchor — re-verify both items' anchors at the cycle-2 tree before choosing which to plan

### Unicode-aware case folding in CLI search and TUI filters
- id: `rm-016` | track: reliability | priority: 68.0 | status: candidate
- signals: reliability.ascii_case_folding:crates/agenttrace-core/src/search.rs::L9 (to_ascii_lowercase) and crates/agenttrace-tui/src/filters.rs contains/add_match (ASCII-only fold) — non-ASCII queries silently miss case-variant matches
- acceptance: case-insensitive matching in the CLI search path and TUI filters uses Unicode-aware folding; tests with non-ASCII case-variant queries (e.g. full-width Latin, Cyrillic, Turkish dotted-I documented as out-of-scope or handled) pass on both paths
- evidence: new tests green in cargo test -q; conductor validation digest validation:v1:<sha> recorded in the shipping PR

### Refresh the install surface to the moved upstream distribution
- id: `rm-017` | track: reliability | priority: 74.0 | status: candidate
- signals: reliability.stale_install_source:install.ps1::L10 ($REPO pinned while bare npm name agenttrace 404s on registry.npmjs.org as of 2026-09-30 — upstream moved to @zack78/agenttrace + GitHub Release binaries with checksums + winget Luoyuctl.AgentTrace + brew luoyuctl/tap)
- acceptance: install.ps1 and install.sh download from a pinned GitHub release artifact with checksum verification (extending the install-verification mechanism already planned cross-campaign), docs name the current package surfaces, and a dry-run/manual install on each target OS succeeds from a clean environment
- evidence: install scripts fetch+verify a pinned release in a clean-room test (CI job or recorded manual run); docs diff shows only the distribution section; conductor validation digest validation:v1:<sha> recorded in the shipping PR

### Mirror upstream's CI governance wave on the fork
- id: `rm-018` | track: reliability | priority: 72.0 | status: candidate
- signals: reliability.governance_gap: fork ci/ lane lacks cargo-deny, OpenSSF Scorecard, coverage/Codecov and git-cliff changelog CI that upstream added 2026-09-30 (#287-290)
- acceptance: the four governance additions run green on the fork (cargo-deny advisories+licenses config committed, Scorecard or equivalent badge lane, coverage reporting wired, git-cliff changelog generated on release); no existing gate weakened
- evidence: four green CI lanes on the fork's first post-change commit; deny.toml + scorecard/coverage config in-tree; conductor validation digest validation:v1:<sha> recorded in the shipping PR
- integration note (2026-09-30, run 02993de2 merge): cargo-deny, deny.toml, and the CI deny job already landed on master via rm-010 (done) — remaining scope is the coverage/Codecov and git-cliff lanes plus rm-010's deferred Scorecard revisit; do not re-implement the deny lane

### Add an `agenttrace gate` CI exit-code capability riding the post-#286 triage model
- id: `rm-019` | track: reliability | priority: 60.0 | status: candidate
- signals: reliability.missing_capability: no gate/exit-code subcommand in src/main.rs while PR #286 stabilizes triage semantics (ATTENTION_FAIL_MIN 3 / FAIL_RATE 0.2 / COST_USD 10 / P95_GAP_SEC 120, first-matching-signal ordering) — the CI-gates lane the 2026-09-02 ideation ledger named uncontested but never converted
- acceptance: `agenttrace gate --max-cost USD --max-fail-rate R <session-dir>` exits 0/1 listing the offending sessions, thresholds documented and defaulting to the post-#286 triage constants; golden tests cover pass/fail boundary cases; parked until rm-012 lands so the gate rides the stable triage model
- cycle 1 (e7206fe5): rm-012 content executed (staged merge) — UNPARKED for cycle-2 scoping, still gated on the commit landing the port
- evidence: subcommand tests green in cargo test -q; example CI snippet in docs; conductor validation digest validation:v1:<sha> recorded in the shipping PR

<!-- rm-017..rm-024 appended by run 3099db98 roadmap phase (campaign 4a20d61e cycle 1, attempt 69d23ead) at HEAD 90a4ef5. IDs start past the highest sibling-worktree allocation (02993de2 rm-009..016, cc2f32d5 rm-003..008); rm-IDs are campaign-local until merge and ID reconciliation is the commit phase's concern. Sources: this run's assess phase (df972c9e, AR2-1..AR2-4) and research phase (183a4126, PE-1..PE-6, pass 8). -->

<!-- cycle 1 (campaign 4a20d61e, run 3099db98, PR #17) - integrated 2026-09-30 (merge of fe8b316 into post-02993de2 master ad503da): campaign-local IDs renumbered rm-017>rm-020, rm-018>rm-021, rm-019>rm-022, rm-020>rm-023, rm-021>rm-024, rm-022>rm-025, rm-023>rm-026, rm-024>rm-027, because 02993de2's integration landed rm-017..rm-019 on master first (install surface, CI governance mirror, gate capability) and keep their IDs; internal cross-references updated to the final IDs, item bodies otherwise verbatim. The 02993de2 merge this set was authored against has now landed, clearing rm-020's next-cycle deferral condition. -->

### Price multi-model sessions per model

- id: rm-020
- track: correctness
- priority: high
- status: candidate
- signals: correctness.single_model_pricing:crates/agenttrace-core/src/lib.rs::L510, correctness.discarded_model_data:crates/agenttrace-core/src/lib.rs::L147, correctness.misleading_provenance:crates/agenttrace-core/src/lib.rs::L770
- acceptance: Analysis accumulates a per-model token ledger keyed on per-event model_used; session cost is the sum over models of lookup_price(model) x that model's tokens; report --overview, JSON output, and the TUI session detail view render a per-model breakdown (model, tokens in/out, rate source, cost); provenance.cost becomes a per-model enumeration or an explicit mixed-model marker; a golden test pins a two-model fixture's expected per-model costs; the sqlite_sessions.rs multi-model acknowledgment test is updated to assert the new exact behavior
- evidence: Anchors re-verified at HEAD 90a4ef5: lib.rs:509-521 retains one model string per session (per-event model_used captured at :147/:344 is discarded at pricing time), lib.rs:596-601 does one lookup_price, lib.rs:763-772 applies that single price to all tokens and stamps provenance.cost=calculated_from_tokens at :770; sqlite_sessions.rs:857 test admits the analogous imprecision on the SQLite path. Research PE-1 (pass 8, 92% confidence): no competitor prices multi-model sessions per-model (ccusage 18,811 stars, tokenmaxxing both apply one rate), making this both the residual correctness defect (assess AR2-1) and a differentiating capability
- next-cycle anchor (compound c1): top unselected alternate — deferred value-based on merge contention (render surface overlaps 02993de2's staged report_overview_* edits), not on value; becomes the top cycle-2 anchor once that merge lands, with rm-021 riding its per-model ledger
- cycle-1 selection note (2026-09-30, run 0a279c44): passed over again, now solely on effort (M-L across ledger+report/JSON/TUI+goldens) and fresh render-surface churn — the contention is gone (both merges landed); cycle-2 anchor status reaffirmed

### Stamp pricing provenance into every artifact

- id: rm-021
- track: data-freshness
- priority: medium
- status: candidate
- signals: data_freshness.unattributed_pricing:crates/agenttrace-core/src/pricing.rs::L98, data_freshness.unconsumed_accessors:bundled_snapshot_date/model_count have zero artifact consumers
- acceptance: Every report footer, JSON output, and TUI about/overview surface embeds the pricing snapshot date, model count, catalog content hash, and the override-file fingerprint when AGENTTRACE_PRICING_FILE is active; two artifacts priced against different snapshots are distinguishable without repo access; JSON changes are additive only; a test pins the stamp format
- evidence: pricing.rs:98-105 exposes bundled_snapshot_date() and bundled_snapshot_model_count() with zero artifact consumers at HEAD 90a4ef5 (grep-verified research pass 8; the accessors were added for cycle 7 R1 disclosure and never rendered); research PE-3 (88% confidence, low complexity); compounds with rm-020's rate-source column and with sibling campaign rm-006's doctor drift surfacing (that item surfaces at read time, this one pins into the artifact)

### Sign and attest release artifacts

- id: rm-022
- track: security
- priority: medium
- status: candidate
- signals: supply_chain.hash_only:.github/workflows/release.yml::L103, supply_chain.upstream_hardening:luoyuctl/agenttrace #287-#290 + 30-release channel on 2026-09-30
- acceptance: Release tarballs and installers are cosign-signed; the build workflow emits SLSA provenance attestations; an SBOM ships with each release (cargo-auditable or syft); install.sh and npm/scripts/install.js verify a signature rather than only the sidecar hash; README documents verification including a Windows path; verification failure refuses install
- evidence: release.yml emits SHA-256 sidecars only (sha256sum at :103-106, checksums.txt at :140) with no signing or attestation lane at HEAD 90a4ef5; upstream landed #287 (CodeRabbit+Codecov), #288 (cargo-deny+Scorecard+rustls advisory patch), #289 (git-cliff), #290 (badges) and grew a 30-release/tag channel between same-day probes 03:40Z-06:1xZ on 2026-09-30 (gh api, research pass 8); research PE-2 (80% confidence). Complementary to - not duplicating - the sibling campaign's in-flight cargo-deny/Scorecard batch: that gates dependencies, this attests artifacts

### Materialize an incremental explorer index

- id: rm-023
- track: performance
- priority: medium
- status: candidate
- signals: performance.per_frame_rebuild:crates/agenttrace-tui/src/explorer.rs::L544, performance.duplicate_resolution:Projects comparator resolves each project twice
- acceptance: The explorer's session index (filtered/sorted views, project resolution) is materialized into a SQLite sidecar keyed by session path + mtime; TUI launch appends/updates incrementally instead of enumerating and parsing every file; explorer_indices() remains the in-memory shape over indexed rows; comparator project resolutions are cached; the schema carries a version field for migration; full suite green
- evidence: explorer.rs:544-601 rebuilds filtered+retained+sorted indices per call across ~5 call sites per rendered frame and the Projects comparator calls resolve_project() twice per comparison at HEAD 90a4ef5 (assess AR2-2); rusqlite is already a workspace dependency; research PE-4 (75% confidence, medium-high complexity). This is the architectural fix beneath the per-frame waste and the substrate rm-025 needs; it supersedes the narrower dead-recompute-removal idea on the sibling e7206fe5 roadmap (rm-012 there, rm-015 since that campaign's integration) - reconcile at merge
- next-cycle anchor (compound c1): deferred on lane contention (explorer.rs claimed by cc2f32d5's presentation/explorer work); also carries the rm-015 supersession decision (sibling e7206fe5's campaign-local rm-012) — settle both after the merge landscape clears; re-verify explorer.rs anchors at the cycle-2 tree before planning

### Ship an `agenttrace upstream status` subcommand

- id: rm-024
- track: upstream-sync
- priority: medium
- status: done
- signals: upstream_sync.manual_drift_rederivation:fork maintenance loop re-derives drift by hand each cycle, upstream_sync.channel_volatility:+4 commits and 30 releases in one day; npm package name moved once
- acceptance: `agenttrace upstream status` compares the running build against upstream main, the GitHub release channel, and the npm package - commits ahead/behind, PR-level delta grouped by area (parser/diagnostics/CI), advisory drift, distribution-channel state; offline by default with an explicit fetch flag; output schema documented and stable for scripting
- evidence: Upstream master moved be25c4c9 -> e54831e (#287-#290) and gained 30 releases/tags between same-day probes 03:40Z-06:1xZ on 2026-09-30; npm @zack78/agenttrace stabilized at 0.9.0 after unpublish churn while bare `agenttrace` 404s (all fetched live, research pass 8); the fork's own maintenance loop re-derives this drift by hand each cycle via internal-only scripts; research PE-6 (72% confidence, low-medium complexity)
- compound c1 2026-09-30: implemented in the campaign worktree (uncommitted, pre-review) as `agenttrace upstream` — crates/agenttrace-cli/src/upstream.rs, offline-by-default (local remote-tracking refs) + explicit `--fetch` (git fetch + npm probe), `-f json` append-only schema documented in docs/guides/upstream-status.md; 6 unit + 9 hermetic integration tests; live-verified offline (ahead 37 / behind 8 / 8 unported #285-#292 / 124 diverged files) and via --fetch (npm 0.9.0); recorded outcomes: targeted 5 passed, full suite 251/0. Solution doc: docs/solutions/workflow-issues/fork-upstream-drift-status-subcommand.md. Flipped to done at the commit gate 2026-09-30: independent review APPROVED (five-lens, zero fixes), full workspace 251/0, targeted 5 passed, digest validation:v1:bde0b51bf06c9c706fef41075750 at base 90a4ef5, shipped in this commit

### Watch mode on session files

- id: rm-025
- track: customer-experience
- priority: medium
- status: candidate
- signals: capability_gap.no_live_tail:ccusage monitor mode has no fork equivalent, existing_machinery:TUI already re-scans and re-renders on change inside a full launch
- acceptance: `agenttrace watch [paths]` tails session files on append, re-analyzes only the changed file incrementally, and re-renders the live overview/sessions view (cost so far, waste signals, loop detection); implementation stays file-tail re-analysis of written events and never stream interception, honoring the recorded non-goal; cost stays linear in changed files via rm-023's index or a per-file cache; the notify dependency is evaluated against the static-build profile
- evidence: ccusage's monitor mode is that project's flagship (org repo, 18,811 stars, pushed 2026-09-30T02:50Z); the fork's differentiating diagnostics (waste/loop/governance) have no live-mode equivalent anywhere; the TUI already re-scans and re-renders on change, so the machinery exists but only inside a full-launch directory sweep; research PE-5 (70% confidence, high complexity; scoped to file-tail per the ROADMAP non-goal - live tracing while a model is streaming - as the repeat-check in pass 8 established)

### Exclude untimestamped sessions from trend windows

- id: rm-026
- track: correctness
- priority: low
- status: candidate
- signals: correctness.trend_pollution:crates/agenttrace-core/src/reports.rs::L2075, correctness.unguarded_none:reports path lacks the !session_start.is_empty() guard the explorer path has
- acceptance: Sessions with no parseable timestamp are excluded from health-trend window bucketing (or bucketed into an explicit unknown bucket); direction and regression verdicts no longer change when untimestamped sessions are present; a golden test pins a mixed fixture with and without timestamps
- evidence: reports.rs:2071-2076 sorts (None, Some) => Ordering::Greater, placing untimestamped sessions after all timestamped ones - into the newest trend window; analyze() leaves session_start default-empty when no event timestamp parses (lib.rs) and parse_rfc3339 then yields None; the explorer path guards !session_start.is_empty() for its comparison (explorer.rs) while the reports path does not; assess AR2-3 (attempt df972c9e), anchors re-verified at HEAD 90a4ef5

### Use display-width math for report column budgets

- id: rm-027
- track: correctness
- priority: low
- status: candidate
- signals: correctness.rune_width:crates/agenttrace-core/src/reports.rs::L2447, inconsistency:reports count chars while the TUI uses unicode-width
- acceptance: Width-aware truncation and padding (unicode-width) for label and name columns in text reports; CJK content renders within the budgeted width; parity with the TUI's existing unicode-width approach; golden tests pin CJK-label fixtures in the affected columns
- evidence: reports.rs:2447-2461 truncate_text_runes counts chars (value.chars().count()) not display cells, so CJK labels/names render up to 2x the budgeted width; the TUI already measures with unicode_width::UnicodeWidthStr (filters.rs:379/:411, dependency unicode-width 0.2) - the two rendering surfaces disagree; zh translations ship in the i18n files, making this reachable in normal use; assess AR2-4 (attempt df972c9e), anchors re-verified at HEAD 90a4ef5
- live anchor addendum (2026-10-01, run 792ef47b assess 5d6eb40c F4, re-verified at HEAD 9d88b36): the compare table's `format!("  {:<28} ...", name)` padding at reports.rs:1290/:1313 visibly misaligns a CJK session-name row against ASCII rows in live `--compare` output — same char-vs-display-width class as truncate_text_runes; folded into this item rather than minted separately (dedup decision recorded in the run's roadmap diff)

<!-- New items 2026-09-30 (run 88feec46 assess attempt 6c039674 + research attempt b31b51bd): numbered rm-012+ to stay disjoint from sibling campaign cc2f32d5's rm-003..rm-008 and this fork's own rm-001/rm-002/rm-009..rm-011 -->
<!-- ID-collision record (commit gate 2026-09-30, review P2 of attempt c429ef80): campaign-local rm-012..rm-019 COLLIDE with distinct integrated items on origin/master ad503da (run 02993de2 / PR #16 renumber, campaign e7206fe5, landed while this campaign ran). Disposition per the PR #16 precedent: renumber this campaign's block at integration to the next free contiguous range above all landed AND claimed IDs at that time (today: master up to rm-019; open PR #17 claims rm-017..rm-024 stale-low; run cbe30a9c's tree claims rm-025..rm-033) and record the old>new mapping here; all phase artifacts, the independent review, and the commit message use the campaign-local IDs -->
<!-- RENUMBER EXECUTED (2026-09-30, run 0a279c44 roadmap phase, campaign 47e4432e cycle 1, attempt 8f3fe81a): integration (merge 9d88b36) landed without applying the disposition above, leaving rm-012..rm-023 doubly-defined with the first-block items. Per that record, this block renumbered rm-012>rm-034, rm-013>rm-035, rm-014>rm-036, rm-015>rm-037, rm-016>rm-038, rm-017>rm-039, rm-018>rm-040, rm-019>rm-041, rm-020>rm-042, rm-021>rm-043, rm-022>rm-044, rm-023>rm-045 — ceiling above all landed (rm-027) and claimed ranges (cbe30a9c tree rm-025..rm-033; run-304846327112 branch rm-026..rm-032; open PR #19 carries campaign-local rm-012..rm-023 to be renumbered at its own integration). Item bodies verbatim; block-internal cross-references updated (rm-013→rm-035 in the parse-once notes, rm-016→rm-038 in the statusline-schema pair note). Historical phase artifacts (docs/stewardship/2026-09-30-cycle1-prioritization.md, docs/solutions/*) retain campaign-local IDs per the convention above; translate via this mapping. -->

### Sanitize statusline report output (terminal control injection from journal payloads)
- id: `rm-034` | track: reliability | priority: 89.0 | status: done
- signals: security.terminal_injection:crates/agenttrace-core/src/statusline.rs::L615+L628+L636-644 (report path prints journal-derived session_id and miss_cause strings raw; the render path sanitizes the same class at L263-268 sanitize_line_segment; ingestion stores hostile payloads verbatim at L275 append_statusline_capture; live repro 2026-09-30: crafted journal payload emitted raw ANSI SGR + OSC-52 through `agenttrace --statusline-report`)
- acceptance: every string printed by --statusline-report passes the same control-character sanitization as the render path (shared helper), locked by a regression test asserting ESC/OSC sequences appear sanitized in report output; TUI panel stays numeric-only
- evidence: cargo test green including the new injection regression test; live repro re-run shows no raw ESC bytes in report output (cat -v); conductor validation digest validation:v1:<sha> recorded in the shipping PR — compound-c1 (2026-09-30, run 88feec46): implemented uncommitted at 7bb4dcb — all three report print sites route through the shared sanitizer (statusline.rs:619/:632/:645 over sanitize_line_segment:265); regression test statusline_report_sanitizes_journal_derived_strings (statusline.rs:909); targeted+full gates green 238/0, digest validation:v1:9c9a1d4d5e3067b03657b0b4984c45be5b0358487a19497c64241c6224082088; live cat -v re-run shows U+FFFD and zero raw ESC; learning recorded at docs/solutions/security-issues/terminal-injection-through-journal-derived-statusline-strings.md; done-flip reserved for the commit gate — flipped 2026-09-30 (commit gate, run 88feec46 cycle 1)

### Fix Codex token double-counting after compaction (adopt upstream #286 high-water fix)
- id: `rm-035` | track: reliability | priority: 88.0 | status: done
- signals: correctness.double_count:crates/agenttrace-core/src/parser.rs::L2257 (codex_token_count_usage computes token_usage_delta(&total, prev_total) on the raw cumulative total; when Codex rewinds total_token_usage after compaction and climbs back, the rebound is re-counted; upstream fix be25c4c "Track Codex cumulative token high-water mark so rewound totals are not re-counted" merged 2026-09-30 and present in the local git object store; our fork point 6848aa1 predates it)
- acceptance: token accounting tracks a cumulative high-water mark so rewinds never double-count; a fixture with rewind+rebound yields exactly the sum of true per-event deltas; estimated Codex costs are unaffected by compaction events
- evidence: new regression test with a rewound-then-risen total_token_usage fixture asserting single-counted tokens; cargo test green; conductor validation digest validation:v1:<sha> recorded in the shipping PR — compound-c1 (2026-09-30, run 88feec46): be25c4c's high-water hunk transplanted hunk-level (token_usage_high_water at parser.rs:2313 feeding the codex_token_count_usage call at parser.rs:2258), never the 11-file wave; regression test codex_total_usage_rewind_after_compaction_is_single_counted (parser.rs:4443) asserts single-counted 620 input/590 output/480 cache-read vs 870/630 pre-fix, proven by a revert-production-hunk discrimination re-run; targeted+full gates green 238/0, digest validation:v1:9c9a1d4d5e3067b03657b0b4984c45be5b0358487a19497c64241c6224082088; done-flip reserved for the commit gate — flipped 2026-09-30 (commit gate, run 88feec46 cycle 1)

### Parse JSONL once across format probes
- id: `rm-036` | track: reliability | priority: 80.0 | status: candidate
- signals: performance.reparse:crates/agenttrace-core/src/parser.rs (parse_raw_session probes each candidate format by re-parsing the raw JSONL text per probe; upstream be25c4c refactored the same chain to parse objects once and probe the parsed form as part of its +1249/-369 wave)
- acceptance: format detection parses the source text once and shares the parsed objects across all probes; unknown-format files show a single parse pass; existing parser golden behavior unchanged
- evidence: cargo test green (existing parser goldens); instrumentation or benchmark note documenting single-pass detection on a mixed corpus; conductor validation digest validation:v1:<sha> recorded in the shipping PR
- notes: compound-c1 sequencing held — deliberately behind rm-035 so the parse-once refactor could not muddy the rewind fixture's baseline; when implementing, transplant the be25c4c probe-refactor hunks hunk-level (never the whole 11-file wave) and prove with a revert-production-hunk discrimination re-run, the pattern rm-035 used

### Catch up statusline schema: spend_limit window + structured repo identity
- id: `rm-037` | track: customer-experience | priority: 78.0 | status: candidate
- signals: contract.gap:crates/agenttrace-core/src/statusline.rs::L215+L405-502 (only five_hour/seven_day rate-limit windows modeled; live Claude Code statusline docs 2026-09-30 additionally document rate_limits.spend_limit.used_percentage/resets_at, silently dropped), contract.gap:workspace.repo.{host,owner,name}+workspace.git_worktree+added_dirs+session_name+effort.level+exceeds_200k_tokens (documented fields absent from the payload model; repo identity would also feed insights attribution and reduce resolve_project filesystem walks)
- acceptance: spend_limit parsed and rendered alongside the two existing windows and recorded in journal insights when present; workspace.repo identity captured into the journal record and used by insights attribution when available; payload model tolerates unknown future fields without error
- evidence: statusline unit tests with a fixture carrying spend_limit and workspace.repo; round-trip journal entry showing the new fields; cargo test green; conductor validation digest validation:v1:<sha> recorded in the shipping PR
- notes: compound-c1 disposition — named the cycle-2 pair with rm-038 per the prioritize record (run 88feec46 attempt 51899064)

### Memoize session project identity in TUI hot paths
- id: `rm-038` | track: reliability | priority: 77.0 | status: candidate
- signals: performance.hot_walk:crates/agenttrace-tui/src/app.rs::L1427-1429 (filter predicate calls resolve_project -> parent-dir git-root walk per session per refresh), performance.hot_walk:crates/agenttrace-tui/src/explorer.rs::L556-559+L568 (sort comparator and retain walk per element, O(n log n) walks), performance.hot_walk:crates/agenttrace-tui/src/filters.rs::L251-253 (label lookup per render); walk implementation crates/agenttrace-core/src/insights.rs::L148-184 with no cache
- acceptance: resolve_project results memoized per session identity and populated at load; TUI filter/sort/label paths perform zero additional filesystem walks after initial resolution; behavior identical for sessions outside git repos
- evidence: instrumentation or test demonstrating walk count independent of sort/refresh; cargo test green; conductor validation digest validation:v1:<sha> recorded in the shipping PR
- notes: cross-campaign corroboration (2026-09-30, run 0a279c44 assess): additional unmemoized is_dir probes sit on the same hot paths at insights.rs:231/:278/:281 — fold them into this item's memoization pass

### Remove dead self-hosted cache steps from hosted-runner CI lanes
- id: `rm-039` | track: reliability | priority: 74.0 | status: candidate
- signals: ci.dead_step:.github/workflows/ci.yml::L44-63+L69-78 (Restore/Save cargo-target tar steps with self-hosted persistence comments run on ubuntu-latest since 6ba55ea #11 2026-09-22; the 369M target tar is written to a path no later hosted job can restore), docs.drift:.github/workflows/dependency-review.yml::L7 (comment claims ci.yml runs "cargo audit + cargo deny"; only cargo-deny exists)
- acceptance: lint lane uses actions/cache keyed on Cargo.lock (or the dead steps are deleted) so PR wall-time no longer includes a dead 369M tar; dependency-review comment names exactly the gates that exist
- evidence: CI run on the changed workflow shows the lint lane green without the tar step; grep shows no stale self-hosted persistence comments; conductor validation digest validation:v1:<sha> recorded in the shipping PR

### Fix --range today UTC boundary
- id: `rm-040` | track: reliability | priority: 72.0 | status: candidate
- signals: correctness.timezone:crates/agenttrace-core/src/insights.rs::L65-70 (Today range computes UTC midnight boundaries rather than the user's local day)
- acceptance: today range anchors to local midnight with documented timezone handling; a test pinning the boundary around a fixed offset passes
- evidence: new unit test for the local-day boundary; cargo test green; conductor validation digest validation:v1:<sha> recorded in the shipping PR
- live anchor addendum (2026-10-01, run 4b9cc093 assess 5290a1cd at HEAD ec8acdc): re-verified live — insights.rs::L66-70 still derives TimeRange::Today from Utc::now().date_naive(); both consumers pinned (CLI main.rs::L850 `--range today`; TUI app.rs::L1433 session_matches_time_range); the codebase convention is local-day (parser.rs::L1214-1221 handles aider timestamps as local-zone), so UTC-midnight "today" is the outlier, not the rule. Assess finding A1 re-anchors here rather than minting a duplicate id.

### Harden discovery-cache keys and listing freshness
- id: `rm-041` | track: reliability | priority: 73.0 | status: candidate
- signals: reliability.lossy_key:crates/agenttrace-core/src/session_cache.rs::L1031-1033 (cache keys built via to_string_lossy: non-UTF-8 paths vanish from cached discovery or collide), reliability.stale_listing:crates/agenttrace-core/src/session_cache.rs::L513 (directory-listing freshness keyed on mtime alone; same-tick creates after store stay invisible until the next mtime change)
- acceptance: cache keys encode paths losslessly (OsStr bytes); listing freshness detects same-tick changes (size/inode or forced rescan); regression tests cover both cases
- evidence: new tests for non-UTF-8 path round-trip and same-tick create visibility; cargo test green; conductor validation digest validation:v1:<sha> recorded in the shipping PR

### Add transcript-derived 5-hour billing block analytics
- id: `rm-042` | track: customer-experience | priority: 76.0 | status: candidate
- signals: user_need.ccusage_blocks (ccusage/ccusage 18,813 stars; blocks report groups usage into 5-hour billing windows with active-block burn rate and projections — fetched live 2026-09-30), capability.gap (our rate-limit visibility exists only via the opt-in statusline journal; CLI-only users have no 5h window view derived from local transcripts)
- acceptance: a blocks command/report groups transcript usage into 5-hour windows aligned to first use, reports per-block cost and token totals by model, and marks the active block with its current burn rate; works with no statusline configured
- evidence: golden test on a synthetic transcript corpus asserting block boundaries and totals; report rendered for a fixture corpus; conductor validation digest validation:v1:<sha> recorded in the shipping PR

### Unify crossterm on 0.29
- id: `rm-043` | track: reliability | priority: 75.0 | status: candidate
- signals: deps.duplicate:cargo tree -i crossterm (0.28.1 direct plus 0.29.0 via ratatui 0.30.2 -> ratatui-crossterm 0.1.2; two copies compiled into every binary; crossterm 0.29.0 is latest per crates.io 2026-09-30)
- acceptance: a single crossterm version in the dependency graph; TUI behavior unchanged (full tui test suite green); no new advisories introduced
- evidence: cargo tree -i crossterm shows one version; cargo test green; cargo-deny advisories ok; conductor validation digest validation:v1:<sha> recorded in the shipping PR

### Refresh ureq and rusqlite majors
- id: `rm-044` | track: reliability | priority: 71.0 | status: candidate
- signals: deps.stale:ureq 2.12.1 (ureq 3.4.2 current per crates.io 2026-09-30; our rustls RUSTSEC exposure rode the ureq 2.x graph), deps.stale:rusqlite 0.32 (0.40.2 current per crates.io 2026-09-30)
- acceptance: pricing transport migrated to ureq 3.x with identical request behavior locked by existing pricing-fetch tests; hermes/opencode sqlite reads migrated to current rusqlite with golden db fixtures passing
- evidence: cargo test green including pricing and sqlite fixture suites; cargo-deny advisories ok; conductor validation digest validation:v1:<sha> recorded in the shipping PR

### Adopt upstream release-engineering wave (crt-static, git-cliff CHANGELOG, republish workflows)
- id: `rm-045` | track: reliability | priority: 70.0 | status: candidate
- signals: release.gap (upstream #285 .cargo/config.toml crt-static MSVC builds merged 2026-09-30 while this fork has no .cargo/config.toml, so Windows users need the VC redistributable; #289 git-cliff automated CHANGELOG addresses the hand-maintained Unreleased section; #291/#292 republish plus pinned wingetcreate harden the distribution channels)
- acceptance: Windows release binaries link the static CRT (no VC redist requirement, verified by dumpbin or a CI artifact check); CHANGELOG generated by git-cliff with existing history preserved as prior context; release republish steps pinned and exercised once
- evidence: release CI artifacts built with crt-static; generated CHANGELOG diff reviewed against current hand-maintained entries; conductor validation digest validation:v1:<sha> recorded in the shipping PR
- notes: roadmap correction (2026-09-30, run 0a279c44, re-verified at HEAD 9d88b36): .cargo/config.toml with crt-static for both MSVC targets IS present in this tree — the "no .cargo/config.toml" signal above is stale (written pre-merge); remaining scope is git-cliff CHANGELOG generation and the pinned-republish/wingetcreate hardening only

<!-- New items 2026-09-30 (run 0a279c44 assess attempt fcd470bd + research attempt 75f3f367, campaign 47e4432e cycle 1): numbered rm-046+ — above the post-renumber ceiling rm-045 and all known in-flight claims (cbe30a9c tree rm-025..rm-033; run-304846327112 branch rm-026..rm-032; open PR #19 carrying campaign-local rm-012..rm-023, to renumber at its own integration). Every signal below was re-verified at HEAD 9d88b36 in THIS worktree before writing: a grounding pass in the earlier assess/research phases had read some file inventories from the shared /work/projects/agenttrace checkout, which a sibling campaign keeps on the ci/glibc-baseline lineage (deny.toml present here, rustls already 0.23.45 here, .cargo/config.toml present here) — the roadmap reflects this tree, not that one. -->

### Saturate token accumulation (i64 overflow panics debug builds, silently corrupts release)
- id: `rm-046` | track: correctness | priority: 87.0 | status: implemented
- compound c1 2026-09-30: implemented (pre-review) at HEAD 9d88b36 — parser.rs add_usage/add_usage_value saturating_add; lib.rs provenance.cost = "calculated_from_tokens_clamped" when any token class sits at i64::MAX. Recorded outcomes: red→green core-lib 99/4→103/0 (live adversarial fixture /tmp/at-adv/opencode: debug panic gone, release no longer wraps); targeted 268/0 (result-430870-326193046.json); full 268/0 (result-493778-326216181.json). Flip to done only after the shipping gate.
- signals: correctness.overflow:crates/agenttrace-core/src/parser.rs::L3466+L3472 (add_usage :3464 and add_usage_value :3470 accumulate token maps with plain `+=`; live in-tree repro 2026-09-30 at HEAD 9d88b36: crafted 2-event opencode journal carrying input=i64::MAX per event — target/debug/agenttrace panics `attempt to add with overflow` at parser.rs:3472; target/release on the same fixture wraps negative, the `>0` filters then drop the tokens, and the report prints tokens.input=0 — a silent UNDER-count, the mirror image of the over-count class AgentMeasure's 2026-09 audit documents across ~110 usage tools)
- acceptance: every token-map merge site (add_usage, add_usage_value, add_opencode_tokens) uses saturating or checked arithmetic; an adversarial fixture with two i64::MAX inputs parses without panic in debug builds and reports clamped totals with an explicit saturation marker in cost provenance; release cost math on the same fixture stays bounded and flagged; regression test token_accumulation_saturates_not_wraps pins the fixture
- evidence: cargo test green including the new regression test; live repro re-run at the shipping HEAD shows debug no-panic and release clamped+flagged output; conductor validation digest validation:v1:<sha> recorded in the shipping PR
- notes: cycle-1 SELECTION (2026-09-30, run 0a279c44 prioritize attempt 21cd5f95, campaign 47e4432e): selected with rm-047 as the cycle-1 batch "Trustworthy token accounting on hostile journals" — the only open candidates with live in-tree failing evidence at this HEAD; full scoring and rejected-alternative rationale at /tmp/at-c1-prioritization/2026-09-30-cycle1-prioritization.md

### Codex head-classification probes bytes and substrings, not parsed JSON
- id: `rm-047` | track: correctness | priority: 73.0 | status: implemented
- compound c1 2026-09-30: implemented (pre-review) at HEAD 9d88b36 — json_key_present() key-boundary anchoring, whole-line token_count rescue, skips counted in Metrics.line_skips["codex_ignorable_line"] (existing field, additive serialization, no schema/cache churn). Recorded outcomes: red→green with rm-046 (99/4→103/0); live rescue verified beyond the 160-byte head window (input 100/output 40 where the old probe dropped the usage line; line_skips {codex_ignorable_line: 3}). Flip to done only after the shipping gate.
- signals: correctness.head_probe:crates/agenttrace-core/src/parser.rs::L2262-2269 (codex line classification slices the first 160 BYTES and substring-matches `"type":"compacted"` / `"type":"event_msg"` — lines whose leading bytes merely QUOTE those substrings inside string content, or whose multi-byte UTF-8 shifts the byte window, are misrouted or dropped with no skip accounting)
- acceptance: classification parses the leading JSON object and switches on the parsed `type` field (or anchors matches to field boundaries); a fixture whose content quotes the marker substrings is classified by its true type; skip decisions are counted in parse diagnostics
- evidence: new parser fixture with quoting lines asserting true-type classification and a correct skip count; cargo test green; conductor validation digest validation:v1:<sha> recorded in the shipping PR
- notes: cycle-1 SELECTION (2026-09-30, run 0a279c44 prioritize attempt 21cd5f95, campaign 47e4432e): paired with rm-046 in the cycle-1 batch — same file, same adversarial-fixture corpus, one review surface; red-to-green order records the panic repro as the first failing test

### Upstream drift report trusts FETCH_HEAD without disclosing its age
- id: `rm-048` | track: upstream-sync | priority: 67.0 | status: candidate
- signals: reliability.stale_ref:crates/agenttrace-cli/src/upstream.rs::L294-301 (freshness falls back to FETCH_HEAD mtime and reports nothing when neither ref exists; a stale FETCH_HEAD lets the report assert freshness it cannot prove — no age disclosure)
- acceptance: FETCH_HEAD-derived answers carry the ref's age and a downgraded authority; when no fresh-enough ref exists the report prints an explicit stale/unknown marker instead of an unqualified value
- evidence: unit tests over fixture refs of varying mtime asserting the age label; cargo test green; conductor validation digest validation:v1:<sha> recorded in the shipping PR

### Archived and relocated projects collapse into one "unknown" attribution bucket
- id: `rm-049` | track: correctness | priority: 66.0 | status: candidate
- signals: correctness.attribution_collapse:crates/agenttrace-core/src/insights.rs::L178-179+L192 (project identity falls back to a single shared id/display_name "unknown"; sessions from distinct archived or moved projects merge into one bucket and insights/leaderboards attribute them jointly)
- acceptance: fallback identities are disambiguated (parent-dir or path-hash suffix) so distinct unknown projects never merge; genuinely unidentifiable sessions keep a single explicit bucket or have it explicitly retired; tests pin two distinct non-git directories yielding distinct buckets
- evidence: new insights tests over two fixture non-git trees; cargo test green; conductor validation digest validation:v1:<sha> recorded in the shipping PR

### Pricing HTTP response body read is unbounded
- id: `rm-050` | track: reliability | priority: 64.0 | status: candidate
- signals: reliability.unbounded_read:crates/agenttrace-core/src/pricing.rs::L348 (override/network pricing path converts the entire response via .into_string() with no size cap — an oversized or hostile body becomes unbounded memory in a diagnostics tool)
- acceptance: body read is capped at a documented limit with a clear error on breach; cap and error path covered by a truncated-body fixture test
- evidence: new test asserting the cap error; cargo test green; conductor validation digest validation:v1:<sha> recorded in the shipping PR
- roadmap correction (2026-10-01, run 4b9cc093 assess A7 at HEAD ec8acdc): the "no size cap → unbounded memory" premise is stale — ureq 2.x caps into_string() at 10 MiB, so pricing.rs::L343-351 fails HARD (transport-flavored io::Error) once the LiteLLM catalog crosses that cap rather than reading without bound. The original acceptance stands; the fixture should pin the 10 MiB boundary explicitly and the breach error must name the limit.

### install.sh source-build fallback clones the unpinned default branch
- id: `rm-051` | track: security | priority: 78.0 | status: candidate
- signals: supply_chain.unpinned_fallback:install.sh::L118 (after a failed runtime verify the installer falls back to `git clone --depth 1 https://github.com/${REPO}.git` and builds whatever the default branch holds — no ref pin, no checksum, nothing recorded; the verified-binary path's sidecar-hash story does not cover this branch; research pass 10 candidate 63)
- acceptance: the fallback clones a pinned ref (tag or recorded commit) with the pin echoed to the user and written to the install receipt; an unpinned clone is never executed; an unresolvable pin aborts with a clear message
- evidence: script test or recorded run showing the fallback cloning the pinned ref and the receipt recording it; sh -n clean; conductor validation digest validation:v1:<sha> recorded in the shipping PR
- notes: cycle-1 selection note (2026-09-30, run 0a279c44): alternate-2 — isolated from the selected theme; clean cycle-2 lead paired with rm-045's remaining git-cliff/republish scope and rm-017's distribution move (scripts/ci/check-install-runtime.sh already in-tree as its offline gate)

### Upstream-status labels conflate absent-probe with registry failure and rank unknown authority as least-fresh
- id: `rm-052` | track: upstream-sync | priority: 58.0 | status: candidate
- signals: truthfulness.labels:crates/agenttrace-cli/src/upstream.rs::L157 ("unavailable (registry probe failed or curl absent)" merges tool-absent with endpoint failure with bad payload; authority ranking treats an unknown source as least trustworthy even when it is the only fresh one)
- acceptance: npm channel state distinguishes curl-absent from HTTP failure from unparseable payload; authority ranking presents unknown explicitly rather than as least-fresh; JSON schema changes additive only
- evidence: unit tests per channel-state branch; cargo test green; conductor validation digest validation:v1:<sha> recorded in the shipping PR

### Token-accounting conformance harness in CI
- id: `rm-053` | track: reliability | priority: 84.0 | status: candidate
- signals: capability.gap:no token-accounting conformance harness anywhere in .github/ or scripts/ (grep-verified 2026-09-30 at HEAD 9d88b36); external yardstick exists: AgentMeasure's 2026-09 audit of ~110 usage tools (github.com/roy-tong/AgentMeasure, campaigns/audit-report-2026-09.md) documents five recurring billing-bug classes (re-emitted/resumed events double-counted, cache-tokens priced as input, price-table drift vs vendor consoles, resume/fork lineage loss, overflow/clamp handling) and ships a conformance pack plus a GitHub Action; this fork's rm-046 live repro proves it is not immune to the class
- acceptance: a conformance job runs a checked-in fixture pack derived from the five audited classes against parser+pricing and fails on any silent double-count, mis-price, or unprovable total; at minimum the overflow/clamp and re-emitted-event classes are represented by fixtures at landing; the job gates PRs touching parser.rs or pricing.rs
- evidence: conformance job green on CI with the fixture pack in-tree; the rm-046 adversarial fixture imported as the first conformance case; conductor validation digest validation:v1:<sha> recorded in the shipping PR
- notes: cycle-1 selection note (2026-09-30, run 0a279c44): deliberate sequencing, not a value deferral — the acceptance's first conformance case IS rm-046's fixture, so rm-046 lands first; this is the cycle-2 strategic lead

### Render UNPROVABLE — not zero — where evidence is absent
- id: `rm-054` | track: customer-experience | priority: 62.0 | status: candidate
- signals: truthfulness.zero_evidence: data-health and governance surfaces (crates/agenttrace-core/src/diagnostics.rs data_health*, report renderers) print numeric zeros where the underlying evidence was never captured; external design principle: AgentMeasure's audit treats UNPROVABLE as a first-class outcome distinct from zero (research pass 10 candidate 64; capability candidate — no defect anchor yet)
- acceptance: surfaces whose inputs are absent render an explicit UNPROVABLE (or equivalent) marker in text plus an additive JSON field; no numeric zero is printed where evidence was never captured; golden tests pin both branches
- evidence: golden tests for the unknown branch on a fixture corpus lacking the input evidence; docs updated for the marker; conductor validation digest validation:v1:<sha> recorded in the shipping PR

<!-- rm-084..rm-090 appended by run 792ef47bdeaf roadmap phase (campaign 1f5ad3cf cycle 1, attempt fd26ee61) at HEAD 9d88b36. IDs start past the highest sibling-worktree allocation (e602bb69 rm-079..rm-083); rm-IDs are campaign-local until merge and ID reconciliation is the commit phase's concern. Sources: this run's assess phase (5d6eb40c F1-F3) and research phase (5b2d04a1 RC-1..RC-4). Dedup decisions: assess F4 (CJK compare padding) folded as a live anchor addendum into rm-027 above; cad2c25d research RC-1 (~/.pi profiles) folded into rm-084 per the research ledger's fold guidance; boundary noted vs run-304846327112 rm-029 (outgoing-report redaction, not at-rest disclosure) and rm-018 (today-range UTC bug, not user-facing scoping). -->

### Discover pi-fork and pi-profile session homes (~/.senpi, ~/.omo, ~/.pi/<profile>)
- id: `rm-084` | track: compatibility | priority: 86.0 | status: implemented
- compound c1 2026-10-01: implemented in the campaign worktree (uncommitted, pre-review) — discovery.rs grew `pi_family_known_session_dirs` (structural enumeration of every pi-family home: ~/.pi, ~/.omp, ~/.senpi, ~/.omo × {agent, agent-*, sessions} shapes, root-relative path matching, session-preserving dedup), parser.rs `pi_source_for_path` now labels by actual home root (`pi` for any `.pi` home, `pi_senpi`/`pi_omo` for the fork homes; XDG `.pi` no longer mislabeled `oh_my_pi`), reports.rs renders `Pi (senpi)`/`Pi (omo)`, main.rs `-d` got non-blank help, README names the fork homes, statusline.rs `--clear-cache` doc comment updated to name the registry-owned clear set (comment-only, no behavior change); golden test crates/agenttrace-core/tests/pi_family_discovery.rs (10-home fixture, proven RED first: 3-of-10 discovered + XDG mislabel). Recorded outcomes: focused gates green — core lib 101/0, pi_family_discovery 2/0, discovery_contract 72/0, cli 41/0, clippy --workspace --all-targets -D warnings clean; live default --overview on this host 15338 sessions / $15.7K with `Pi 8.4K $8.0K` + `Pi (senpi) 1.4K $3.5K` + `Pi (omo) 34 $62.73` (was 5700 / $4.4K / Pi 199, no fork lines); -d ~/.senpi/agent-cliproxy-only/sessions now attributes `Pi (senpi)`. Compound append (c1, pre-review): full-command gate `local_validation_gate.py --shell-command 'cargo test'` returncode 0 — workspace 268/0 (core 182 incl. pi_family_discovery 2, cli 41, tui 45, doc-tests 0; envelope result-1618525-330496923.json, spool full-tests-1c40ce3a…); batch shape: rm-084 lead + rm-086 rider, theme 'Count every session, disclose every artifact'; done-flip reserved for the commit gate
- signals: compat.undiscovered_homes:crates/agenttrace-core/src/discovery.rs::L103-L116 (the pi-family registry registers exactly three homes — ~/.pi/agent/sessions, ~/.config/pi/agent/sessions, ~/.omp/agent/sessions — while crate-wide greps for senpi|omo return 0), compat.profile_blindness:PI_CODING_AGENT_DIR-relocatable homes and ~/.pi/<profile>/sessions variants are unregistered (cad2c25d research RC-1, live-proven at 9d88b36), mislabel:crates/agenttrace-core/src/parser.rs::L1416-L1424 (pi_source_for_path labels every non-default root "oh_my_pi", so discovered fork corpora would render under the wrong source name)
- acceptance: discovery enumerates pi-family homes across forks (senpi/omo class) and profile variants under each home with the same dedup/uniqueness rules as ~/.pi; default --overview on a host carrying such corpora reports them, or the opt-in escape (-d / env) is documented as the supported path; the source label matches the actual home (pi_source_for_path extended or replaced); -d <DIR> gets non-blank --help text; golden tests pin a multi-home, multi-profile fixture corpus
- evidence: live 2026-10-01 (research 5b2d04a1, debug binary, this worktree @ 9d88b36): `agenttrace -d ~/.senpi/agent-cliproxy-only/sessions --overview` → 1403 sessions, $3.5K (1404 jsonl across 558 project dirs, mtimes 2026-08-30..2026-09-14); `agenttrace -d ~/.omo --overview` → 20 sessions, $31.36 (150 jsonl); default-discovery --overview on the same host → 5691 sessions with no senpi/omo line; both corpora are pi `version:3` transcripts the existing parser ingests unmodified via -d — the gap is discovery-only; combined with cad2c25d's profile finding (~/.pi/agent-cliproxy-only/sessions = 8010 jsonl vs 200 discovered, host spend undercounted ~2.6x), pi-family spend invisible to default discovery on this host ≈ $11K

### Honor --lang in every report renderer
- id: `rm-085` | track: correctness | priority: 82.0 | status: candidate
- signals: ux.i18n_noop:crates/agenttrace-core/src/reports.rs::L458+L565+L604+L626+L767+L918+L1066 (report_overview_{json,text,markdown,html}* take no ReportLanguage), ux.i18n_noop:crates/agenttrace-cli/src/main.rs::L1066 (render_session_list) plus the audit/recommend/mcp-governance/context-trends/delivery-evidence render paths, contrast: report_text_with_language (reports.rs:220), report_json_with_language (:74), report_compare_with_language (:1267) and the diagnostics/waste/TUI surfaces all translate — the flag is parsed globally at main.rs:806-810 and silently ignored on the remaining surfaces
- acceptance: --lang is effective for every user-facing action (overview in all four formats, sessions, audit, recommend, mcp-governance, context-trends, delivery-evidence) or an unsupported action exits with an explicit flag-not-supported error; golden tests pin non-English output for each renderer family; --help documents the flag's coverage
- evidence: live 2026-10-01 (assess 5d6eb40c F1, this worktree @ 9d88b36): `--overview --lang en` vs `--lang zh` byte-identical in text/markdown/html/json (diff clean on /tmp/at-probe/{en,zh}.txt) while `--diagnostics/--waste/--compare --lang zh` render Chinese (e.g. 浪费分析, 多会话对比) — a silent no-op, not a partial translation
- rider folded (2026-10-01, run 4b9cc093 assess N5 at HEAD ec8acdc): the ~55 inline bilingual t(language, ("en", ...), ("zh", ...)) pairs in reports.rs drift independently of the TUI's i18n.rs map — when threading ReportLanguage per this item, route both behind one shared translation surface so en/zh pairs cannot drift. Counts re-verified at ec8acdc.

### Disclose local at-rest artifacts in PRIVACY.md and align --clear-cache
- id: `rm-086` | track: security | priority: 80.0 | status: implemented
- compound c1 2026-10-01: implemented in the campaign worktree (uncommitted, pre-review) — session_cache.rs grew `cache_artifact_paths()` (the single registry: sessions.json, hermes-sqlite.json, opencode-sqlite.json, statusline.jsonl via `statusline::statusline_capture_path()`, pricing.json via `pricing::pricing_cache_path()` — each cleared where its own env-aware constructor says it lives) and `clear_session_cache` now consumes it, so --clear-cache removes all five artifact classes; PRIVACY.md rewritten as an artifact table (data class + purge command per artifact, XDG/AGENTTRACE_SESSION_CACHE_DIR relocation, history.json explicitly preserved-not-cleared with rationale, pricing catalog marked public-data); two new unit tests pin the registry — `clear_cache_removes_every_artifact_and_only_those` (5 artifacts gone, bystander kept, second clear a no-op) and `privacy_disclosure_lists_every_artifact` (PRIVACY.md must contain every artifact file name derived from the same constructors + history.json). Recorded outcomes: focused gates green (core lib 101/0 incl. both new tests, cli 41/0, clippy clean); live sandboxed end-to-end: HOME=/tmp/…/.cache/agenttrace with all 5 artifacts + keep-me.json → --clear-cache empties it except the bystander. Compound append (c1, pre-review): full-command gate returncode 0 — workspace 268/0 including both new registry tests (envelope result-1618525-330496923.json); rider beside rm-084 in the 'Count every session, disclose every artifact' batch; done-flip reserved for the commit gate. Review fix (c1, independent_review b7a525aa): `--clear-cache` additionally sweeps superseded-version leftovers (`hermes-sqlite-v<v>-*.json`, `opencode-sqlite-v<v>-*.json`, new `legacy_cache_artifact_paths()`; prefixes pinned to the two stores so near-miss names stay), `remove_cache_artifacts` clears everything it can before reporting failures (no first-error stranding), the clear-cache test now drives the real `clear_session_cache()` e2e under pinned AGENTTRACE_SESSION_CACHE_DIR + XDG_CACHE_HOME (pricing resolves via user_cache_dir, not the session-cache env) with legacy fixtures, the privacy test requires the legacy patterns disclosed, and PRIVACY.md names the sweep + the pricing bundled-snapshot fallback after a clear.
- signals: privacy.undisclosed_local_store:PRIVACY.md::L1-L7 (the whole 7-line file discloses only -o reports and the LiteLLM pricing cache; ~/.cache/agenttrace is never named), privacy.derived_content:crates/agenttrace-core/src/session_cache.rs::L104-L190 (GoSession persists conversation-derived Name, CWD, FileUsage paths, ToolUsage into sessions.json), privacy.sqlite_snapshots:session_cache.rs::L220-L222 (hermes-sqlite.json / opencode-sqlite.json re-cache whole sqlite-derived session sets incl. titles), privacy.journal_purge_gap:crates/agenttrace-core/src/statusline.rs::L29+L275-L298 (statusline.jsonl journal, 10MiB bounded, written from every statusline hook run) while clear_session_cache at session_cache.rs:216-L229 removes only the three cache files and skips the journal (ce360ed1 assess F3, unminted sibling finding folded here)
- acceptance: PRIVACY.md enumerates every artifact the tool writes under ~/.cache/agenttrace (sessions.json, hermes-sqlite.json, opencode-sqlite.json, statusline.jsonl journal, pricing cache) with the data class each carries and the purge command; --clear-cache removes or explicitly reports every listed artifact including the statusline journal; a registry test keeps the PRIVACY list in sync with the code's cache-path construction so new stores cannot ship undisclosed
- evidence: live 2026-10-01 (assess 5d6eb40c F2, this worktree @ 9d88b36): clear_session_cache enumerated paths verified against PRIVACY.md's text; boundary: run-304846327112 rm-158 owns outgoing-report redaction (--share-safe) — this item owns at-rest local disclosure and purge parity

### Absolute-time report scoping (--since/--until)
- id: `rm-087` | track: customer-experience | priority: 78.0 | status: candidate
- signals: capability.missing:crates/agenttrace-cli/src/main.rs (flag-definition greps for since/until/from/to/after/before/start/end return 0; the only time-adjacent selectors are --latest and the statusline windows), user_need.ccusage_parity (ccusage README :150 `--since 2026-04-25 --until 2026-05-16`, :178 "Date Filtering" feature bullet; fetched live 2026-10-01, archived as research-5b2d04a1/ccusage-readme3.md), docs/ideation/2026-09-02-agenttrace-extensions-ideation.md:382 (idea 13 "Absolute-time scoping", never converted)
- acceptance: --since/--until (plus a --last N[d|w] shorthand if cheap) are accepted by overview/sessions/waste/compare/governance actions in every output format; boundary semantics documented (inclusive start, exclusive end, or stated otherwise) and pinned by fixtures across the boundary; the timezone rule is stated once and reused; scoping composes with rm-085's i18n threading so scoped reports render in the selected language
- evidence: live 2026-10-01 (research 5b2d04a1 RC-2): default --overview on this host aggregates all 5691 sessions back to months with no in-tool way to ask "this month"/"last week"; boundary: rm-040 owns the today-range UTC-day bug inside insights — this item owns the user-facing scoping surface only. (cross-ref corrected 2026-10-01 by run 4b9cc093 roadmap phase: the original rm-018 was campaign-local numbering of the minting lineage; in this file's lineage the today-range bug is rm-040.)

### Shell completions and man page via clap_complete/clap_mangen
- id: `rm-088` | track: customer-experience | priority: 70.0 | status: candidate
- signals: capability.missing (agenttrace --help has no completions/man surface — grep completion = 0; greps for completion|fish|zsh|bash_completion over homebrew/, install.sh, npm/, winget/ = 0; root Cargo.toml carries clap 4.5 with no clap_complete), docs/ideation/2026-09-02-agenttrace-extensions-ideation.md:492 (idea 22 "Shell completions and man page via clap_complete", filed under "Cheap, immediate", never converted)
- acceptance: `agenttrace completions <shell>` emits bash/zsh/fish completions and a man page renders via clap_mangen; generation runs in CI so artifacts cannot drift from --help; outputs are wired into the existing homebrew/winget/npm channels where each has a convention; a CI step exercises generation end to end
- evidence: research 5b2d04a1 RC-3 (greps at HEAD 9d88b36); clap_complete 4.x is a drop-in addition beside the existing clap 4.5 dependency

### Make -m truthful on --compare and --test-match
- id: `rm-089` | track: correctness | priority: 66.0 | status: candidate
- signals: correctness.inert_flag:crates/agenttrace-core/src/pricing.rs::L238-L259 (render_test_match iterates a hardcoded 10-model list; no model parameter exists), correctness.inert_flag:crates/agenttrace-cli/src/main.rs::L226-L227 (the --test-match dispatch passes nothing), correctness.label_only:crates/agenttrace-core/src/reports.rs::L1278-L1321 (report_compare_with_language uses `model` only in the header line; the table body is computed without it, so -m neither filters to nor prices against the named model)
- acceptance: -m on --compare either filters sessions to the named model or prices the comparison against it, with the chosen semantics documented; -m on --test-match targets the named model's lookup (the flag's evident purpose) or the combination exits with an explicit unsupported-flag error; tests pin both actions with a -m argument present
- evidence: live 2026-10-01 (assess 5d6eb40c F3, this worktree @ 9d88b36): `--test-match -m "gpt-5*"` and `-m claude-3-5-haiku-20241022` print identical fixed lists; `--compare` vs `--compare -m claude-sonnet-4` differ only in the header's model label (/tmp/at-probe/{c1,c2}.txt)

### Run-acceptance (trust disposition) ledger — watch item, promotion-gated
- id: `rm-090` | track: customer-experience | priority: 40.0 | status: candidate
- signals: user_need.upstream_discussion_255 ("Where should trace review end and trust review begin for coding-agent runs?", opened 2026-05-31, one comment, links the 0-star camirian/agent-evidence-recorder pushed 2026-09-28; fetched live 2026-10-01, archived as research-5b2d04a1/disc.html), capability.gap (governance reports are read-only diagnostics; no surface records a reviewer accept/reject disposition on a run)
- acceptance: promotion gate — a second independent demand signal must be recorded before any implementation spend; if promoted: `agenttrace --attest <session> --verdict ok|reject --note ...` appends to a local journal reusing the statusline journal mechanics and the terminal-sanitization rules already hardened this cycle, and --delivery-evidence plus governance surfaces render recorded dispositions with reviewer and timestamp
- evidence: research 5b2d04a1 RC-4 with its tempering note verbatim (single user, four months old, no momentum, 0-star linked repo) — held at watch priority deliberately; this is the only open user signal in the upstream Discussions channel naming agenttrace's exact niche

<!-- New items 2026-09-30 (run 30484632 assess f69de232 + research d22fc100). RENUMBERED at the commit gate (2026-09-30, run 30484632 commit e0e516be): minted as campaign-local rm-017..rm-023, which collided with sibling campaigns that landed/pushed while this cycle ran — origin/master ad503da carries rm-001..rm-019, PR #17 carries rm-017..rm-024, PR #18 carries rm-017..rm-025 and rm-033 (each with DIFFERENT content under those numbers). Mapping: rm-017→rm-026, rm-018→rm-027, rm-019→rm-028, rm-020→rm-029, rm-021→rm-030, rm-022→rm-031, rm-023→rm-032; all campaign artifacts (batch selection, stewardship request, cycle-8 record) citing rm-017..rm-023 mean these renumbered IDs. rm-026..rm-028 and rm-002 flip to done in this commit. NOTE: rm-002 (pytest for make-adversarial-sqlite.py) is also implemented by open PR #17's fe8b316 — content overlap to reconcile at integration. -->

<!-- RENUMBER EXECUTED AT INTEGRATION (2026-10-01, conflict case 5af7cbb6, merge of 334b5a8/PR #19 into 1806e18): this block's rm-026..rm-032 collided with landed master — run 88feec46 (merged 9d88b36) minted its own rm-024..rm-027, so rm-026 (trend windows) and rm-027 (display-width math) were doubly-defined in the merged ROADMAP: exactly the double-mint class the master-side renumber note of run 0a279c44 anticipated ("open PR #19 ... to be renumbered at its own integration"). Mapping: rm-026→rm-155, rm-027→rm-156, rm-028→rm-157, rm-029→rm-158, rm-030→rm-159, rm-031→rm-160, rm-032→rm-161 — next free contiguous block above every landed id (this tree max rm-090) and every recorded claim (run-266b6e2b rm-091..rm-099; sibling-worktree walls rm-126..rm-136, rm-138..rm-140, rm-141..rm-154). Block-internal cross-refs swept (rm-031's ride note now cites rm-158) and live pointers swept in deny.toml:6, scripts/ci/check-locked-cargo.sh:4, .github/workflows/scorecard.yml:3, .github/workflows/release.yml:215 — the two workflow comments still carried the pre-renumber campaign-local ids rm-017/rm-018, missed by the campaign's own commit-gate sweep — plus docs/solutions/security-issues/rustsec-advisory-shipped-via-ungated-lockfile.md:55 and docs/solutions/process-issues/2026-09-30-unverified-assumption-compounded-across-governance-artifacts.md:103. Dated stewardship records (docs/stewardship/2026-09-30-cycle8-implementation-record.md and spool artifacts) retain this block's prior ids per the 0a279c44 convention; translate via this mapping. Signal anchors re-verified against master's drift at this integration: the PRIVACY.md caveat now lives at :5 (PRIVACY.md rewritten by run 792ef47b, content intact), and the CLI crate path is crates/agenttrace-cli/src/main.rs (package name `agenttrace`). Post-edit duplicate probe (backtick-normalized): empty. -->

### Truth-correct the private-remote claims and enable the Scorecard lane
- id: `rm-155` | track: reliability | priority: 86.0 | status: done
- signals: security.doc_false:deny.toml::L6 (header asserts the remote is private), security.doc_false:docs/solutions/security-issues/rustsec-advisory-shipped-via-ungated-lockfile.md::L55 (same claim), ROADMAP.md rm-010 cycle-1 note (corrected in place by the cycle-2 truth-correct above); live basis: gh repo view codeo1io/agenttrace --json isPrivate -> false (2026-09-30)
- acceptance: deny.toml:6 and the solutions doc's private-remote sentence corrected to the public truth; an OpenSSF Scorecard workflow (pinned action SHAs) runs and publishes results on the now-public repo; the rm-010 deferral note is the only historical record of the old rationale
- evidence: grep -rn 'private' deny.toml docs/solutions/ returns no false remote-visibility claims; a completed Scorecard run with published artifact; full suite green; conductor validation digest validation:v1:<sha> recorded in the shipping PR

### Pin the winget-create image in the release path
- id: `rm-156` | track: reliability | priority: 84.0 | status: done
- signals: security.unpinned_runner:.github/workflows/release.yml::L225-228 (docker run ghcr.io/microsoft/winget-create:latest — the only non-SHA-pinned runner in the token-bearing release lane); upstream fix available: luoyuctl/agenttrace master 3123f81 "submit WinGet with pinned wingetcreate.exe on Windows" (PR #292, merged 2026-09-30T05:21Z)
- acceptance: the WinGet submit step uses a digest-pinned image (or the pinned wingetcreate.exe path upstream adopted); no :latest image references remain in any workflow; release lane green on the next tagged run
- evidence: grep -n 'latest' .github/workflows/*.yml returns no container pulls; diff against upstream 3123f81 recorded; full suite green; conductor validation digest validation:v1:<sha> recorded in the shipping PR

### Enforce lockfile integrity across CI and release builds
- id: `rm-157` | track: reliability | priority: 82.0 | status: done
- signals: reliability.ungated_build:.github/workflows/release.yml::L34+L98 (cargo test / cargo build --release without --locked), reliability.ungated_build:.github/workflows/ci.yml::L64-108 (fmt/clippy/test/build, none --locked); grep -rn -- '--locked' .github/workflows/ -> no matches; assess AF-3 + research RC-12 converge: the rm-009/rm-010 reviewed-lockfile guarantee is convention-only — a drifted Cargo.toml silently regenerates the lock at build time
- acceptance: every cargo build/test invocation in ci.yml and release.yml carries --locked (fmt/clippy unaffected); a lockfile-drift guard fails when Cargo.toml edits are not accompanied by a committed lock refresh; workflows verified by a grep-based CI assertion or equivalent test
- evidence: grep for cargo build/test without --locked returns nothing; a deliberate drift experiment fails the guard; CI green; conductor validation digest validation:v1:<sha> recorded in the shipping PR

### Ship a share-safe report mode
- id: `rm-158` | track: customer-experience | priority: 78.0 | status: candidate
- signals: privacy.share_leak:PRIVACY.md::L5 (the product itself warns "Review reports before sharing them, because they can contain filenames, command names, model names, token counts, costs, and excerpts derived from local session logs" — the mitigation is manual); privacy.share_leak:crates/agenttrace-core/src/search.rs::L203 (evidence snippets embed session content into shareable output); research RC-11: no redaction capability on the 32-idea wall
- acceptance: a --share-safe flag (or equivalent mode) renders every output format (json/markdown/html/console) with deterministic redaction — paths hashed or basename-only, model/provider names optionally masked, no message/tool excerpts; PRIVACY.md's manual-review caveat is replaced by the mode's guarantee; golden tests pin redaction across all formats
- evidence: golden redaction tests for all four render paths; grep for content-bearing fields in --share-safe output fixtures returns nothing; full suite green; conductor validation digest validation:v1:<sha> recorded in the shipping PR

### Extend baseline gates to quality metrics
- id: `rm-159` | track: reliability | priority: 70.0 | status: candidate
- signals: reliability.gate_gap:crates/agenttrace-cli/src/main.rs::L105-109 (--baseline-max-cost/duration/token-delta-pct exist for cost-class metrics only; loop-rate, waste share, and health-score regressions are ungateable); research RC-13: composes with, not duplicates, 5th-pass idea #7 (shareable baseline config) and RE-5 (absolute-threshold CI gate)
- acceptance: baseline gates accept waste/loop/health metrics with stored baselines and delta-percent flags alongside the cost family; a regression vs stored baseline exits nonzero with a named metric; golden tests cover one passing and one regressed baseline per metric class
- evidence: CLI help shows the extended flag family; golden gate tests green; full suite green; conductor validation digest validation:v1:<sha> recorded in the shipping PR

### Portable redacted session bundles (export/import)
- id: `rm-160` | track: customer-experience | priority: 65.0 | status: candidate
- signals: capability.missing:crates/agenttrace-cli/src/main.rs (no export/import surface — grep for export subcommands returns nothing); research RC-15: a single-file redacted bundle unblocks 5th-pass idea #5's format canary (blocked on donated samples), CI reproduction, and archival; rides rm-158's redaction primitive
- acceptance: `agenttrace export` writes a single-file bundle (sessions + pricing snapshot + manifest, compressed) with redaction applied, and `import`/analysis consumes it without the original local stores; round-trip test proves bundle-in equals store-derived output; a canary fixture generated from a real bundle passes the format tracker
- evidence: round-trip + redaction golden tests; a canary run on a generated fixture; full suite green; conductor validation digest validation:v1:<sha> recorded in the shipping PR

### Open the agenttrace-core library lane
- id: `rm-161` | track: developer-experience | priority: 55.0 | status: candidate
- signals: capability.closed:crates/agenttrace-core/Cargo.toml::L13 (publish = false); crates.io fetch 2026-09-30: no agenttrace-core crate registered (name free); research RC-14: the parser corpus is the fork's moat, pass-5 defensible-center analysis (diagnosis depth) supports broadcasting it as an embeddable library
- acceptance: publish = false dropped (or scoped to a stable subset crate); semver + API-stability policy documented in CONTRIBUTING; docs.rs renders the crate; the public API surface carries no TUI/runtime dependencies
- evidence: published crate resolves on crates.io; docs.rs build green; a downstream smoke consumer builds against the published API; full suite green; conductor validation digest validation:v1:<sha> recorded in the shipping PR

<!-- rm-173..rm-193 appended by run 4b9cc093d3584579b7e65a8b7dfeb8b5 roadmap phase (campaign repository-maintenance ad3ce401e5c645f6ae6d0afd57e1c0cd cycle 1, attempt f71c3a0eada641c5a8fc1210cd32573d) at HEAD ec8acdc, worktree conductor/run-4b9cc093d358, porcelain-clean at dispatch.
ID discipline: this tree's wall was rm-161 (62 unique ids, duplicate probe clean). New ids start at rm-173, RESERVING rm-162..rm-172 for the staged-but-unlanded integration block (worktree integration-58cb435705b8-c7dca75d): its staged ROADMAP still carries campaign-local rm-012..rm-022 duplicating landed ids and must renumber at landing — 11 items fit rm-162..rm-172 exactly. Verified live 2026-10-01: fork master still ec8acdc (git ls-remote), no sibling worktree claims above rm-161, duplicate probe re-run clean after this append.
Sources: assess ledger 5290a1cd (spool assess-5290a1cd999a40c3a6cc66dd109e0dae/2026-10-01-assessment.md — fresh A1-A7 at ec8acdc plus standing F1/F2/F3/F4/F5/F6/F7, K1/K3/K4, N1/N3/N4/N5 re-verified) and research notes f006888b (spool research-f006888b8daa4871aa16852ea4a0dfb4/2026-10-01-research-notes.md — RC-1..RC-4 keep their provisional ids rm-173..rm-176). One item minted from direct verification by this phase: rm-187 (pytest suite unwired from CI; flagged first by sibling assess 3c3e933d, re-verified at ec8acdc by grep over .github/workflows + scripts/ci).
Dedup decisions (no double-mint): A1 = rm-040's defect (live-anchor addendum appended there, both consumers pinned); A7 recorded as a premise correction on rm-050 (ureq caps at 10 MiB — read bounded, failure silent-class); N5 folded as a rider into rm-085 (inline bilingual pairs vs i18n.rs map); A6 refutes research candidate K2 (FIFO admission works at ec8acdc — K2 was never minted, no item to close; ledger-only). Stale cross-ref swept: rm-087's evidence cited rm-018 for the today-range bug (campaign-local numbering of its minting lineage); corrected to rm-040. Carried-unminted debt from three prior assess ledgers (4b079a74/ec96be09, 980eb773/29c7340b, e741e661/ae44ccd0 at 1806e18/e182) cleared into rm-177..rm-193. -->

### Port upstream PR #295 (WinGet manifest schema violation)
- id: `rm-173` | track: upstream-sync | priority: 78.0 | status: candidate
- signals: upstream luoyuctl/agenttrace moved 41ac231c→52ab2cd8 ahead-by-1 on 2026-10-01T06:08:29Z via merged PR #295 (+3/-2 on scripts/release/render-channels.sh); generated manifest files lack yaml-language-server schema headers (L88/L96/L120 in the PR view) and zip Archive installers emit PortableCommandAlias (L130/L134), which the WinGet schema rejects for Archive installers; live proof: microsoft/winget-pkgs PR 444788 open/unmerged on the broken manifest; this fork's render-channels.sh carries the same two PortableCommandAlias lines with zero test pins; patch archived at research-f006888b/evidence/upstream-pr295-render-channels.patch
- acceptance: render-channels.sh emits the yaml-language-server schema directive header on every generated manifest file; zip-Archive installers stop emitting PortableCommandAlias (or the installer type moves to a schema-legal form); a fixture render (dry run against a pinned channel file) byte-compares against a schema-valid golden, pinned by an in-repo test so the fix cannot silently regress
- evidence: landing PR shows before/after rendered manifest for one channel plus the golden-fixture test green; upstream PR #295 is the reference implementation, not a blind copy — cite it in the landing commit

### Antigravity SQLite conversation-store adapter
- id: `rm-174` | track: compatibility | priority: 74.0 | status: candidate
- signals: Antigravity agents persist sessions as SQLite under five roots (~/.gemini/antigravity/conversations, ~/.antigravity/conversations, ~/Library/Application Support/Antigravity/conversations, %USERPROFILE%\.antigravity\conversations, $XDG_DATA_HOME/antigravity/conversations) with ANTIGRAVITY_DATA_DIR override; files are .db under conversations/ rather than JSONL (upstream ccusage paths.rs snapshot archived at research-f006888b/evidence/ccusage-antigravity-paths.rs); zero antigravity references in this repo; the sqlite discovery walk (sqlite_sessions.rs::L91-137) does not recognize the family
- acceptance: discovery recognizes all five roots with env overrides honored; SQLite admission scoped to the family (conversations/**/*.db) or probe-based admission that never hangs on non-SQLite files; unmapped/unreadable SQLite skips with skip accounting (rm-039 unknown≠zero), never a hard failure; at least one golden .db fixture round-trips to priced sessions — fixture comparisons at schema+row dump level only, never byte-equality (fleet constraint: SQLite headers embed library-version bytes); doctor reports the family with resolved roots when present and notes absence otherwise
- evidence: fixture corpus committed under tests (schema+row dump golden); live corpus probe recorded in the landing PR if a real Antigravity corpus is available (none found this cycle — state that explicitly rather than shipping speculative schema parsing)

### Claude cross-session duplicate-request collapse
- id: `rm-175` | track: correctness | priority: 72.0 | status: candidate
- signals: ccusage #1765 (fixes #1762, researched this cycle): identical request payloads copied across sessions are counted once per copy, inflating usage; agenttrace's parse path has zero dedup surface (verified by grep at ec8acdc — no content fingerprint or dedup keys anywhere in the parse pipeline); effect lands in session counts, message counts, and total cost
- acceptance: parse path computes a structural fingerprint for request-bearing records where upstream provides no identity; collapse applies only to byte-identical payloads (no fuzzy matching), first occurrence wins, session attribution stays with the original; the collapse count is surfaced (report line or doctor), never silent; golden fixture containing the known duplicated payload shows before/after counts; negative case pinned: genuinely distinct near-identical records do NOT collapse
- evidence: before/after fixture counts + negative case in the landing PR; upstream #1765's taxonomy cited for the duplicate class

### Grok Build CLI session family
- id: `rm-176` | track: compatibility | priority: 62.0 | status: candidate
- signals: Grok CLI persists updates.jsonl (+ summary.json) under ~/.grok/sessions/**/ with GROK_HOME override (upstream snapshot archived at research-f006888b/evidence/ccusage-grok-paths.rs); zero grok references in this repo; format details unpinned — no local corpus found this cycle
- acceptance: discovery recognizes the grok roots honoring GROK_HOME; a family stub with graceful no-op when absent and skip accounting when present is an ACCEPTABLE landing state if documented needs-corpus (explicit note in doctor + README) because no real corpus exists to pin a schema; if a corpus is obtained: schema pinned from real samples, parser + pricing wired, golden fixture tests; no speculative schema parsing without a captured sample
- evidence: landing PR states which of the two states landed and why; discovery tests for root resolution honoring GROK_HOME either way

### Hermes tool-failure counters are structurally zero
- id: `rm-177` | track: correctness | priority: 90.0 | status: implemented
- signals: sqlite_sessions.rs::L195-196 reads tool_calls_total AND tool_calls_ok from the SAME column (idx 5); tool_calls_fail is never assigned anywhere in the loader; --max-tool-fail-rate can therefore never trip; fleet ledger ec96be09 additionally counted messages.effect_disposition at 186,167 live tool rows referenced zero times in the repo (rm-039 unknown≠zero violation); carried unminted through three prior assess ledgers (4b079a74/ec96be09, 980eb773/29c7340b, e741e661/ae44ccd0) — mint debt cleared here
- acceptance: total/ok/fail each derive from their own persisted columns or from an enforced total=ok+fail identity at load time; golden sqlite fixture whose ok/fail/total counts all differ pins the mapping (schema+row dump comparison, not byte-equality); live demo that --max-tool-fail-rate trips on a fixture containing failing tool calls (impossible today); missing/unknown counters render UNPROVABLE per rm-039 rather than 0; effect_disposition either consumed or dropped with a migration note
- evidence: before (gate cannot trip) / after (trips) runs in the landing PR; regression test green; the same-column defect shown in a before diff hunk

### Windows: zero USERPROFILE fallback while shipping Windows artifacts
- id: `rm-178` | track: compatibility | priority: 88.0 | status: candidate
- signals: discovery.rs::L52-54 yields an empty home list when HOME is unset — native Windows provides USERPROFILE, not HOME; zero USERPROFILE fallback workspace-wide (doctor.rs::L266, sqlite_sessions.rs::L65, pricing.rs::L1160, session_cache.rs::L1087 degrade path); README:72 ships install.ps1 and release.yml:61-66 publishes Windows targets; CI is ubuntu-only; net effect: silent zero-session no-op on the platform the project explicitly ships for
- acceptance: one shared home-resolution helper (HOME then USERPROFILE, or a dirs-style platform resolver) used at all four cited sites; env-driven unit tests pin the Windows-shaped environment (HOME unset, USERPROFILE set) per site; discovery/doctor/sqlite loaders locate fixture session homes under a USERPROFILE-shaped root; a windows CI lane exercises discovery on a fixture corpus, or the shipping docs carry an explicit supported-platform statement — landing at least one of the two, with the other tracked
- evidence: env-driven tests green; CI lane run or documented decision note in the landing PR

### Statusline compaction: fixed temp path, registry-blind orphan
- id: `rm-179` | track: reliability | priority: 80.0 | status: implemented
- signals: statusline.rs::L318 compacts to the FIXED path jsonl.compact — unique_temp_path already exists (session_cache.rs::L355) but is unused here; concurrent statusline hooks clobber each other's compaction and a crash orphans the temp (~5 MiB journal content observed) OUTSIDE both the rm-086 cache_artifact_paths registry (session_cache.rs::L216-227) and the legacy sweep; PRIVACY.md's "exactly four artifact classes" enumeration is falsifiable by that orphan
- acceptance: compaction writes via unique temp + atomic rename; compaction temps are covered by the rm-086 registry or the sweep so no artifact class exists outside the enumerated set; test proves two concurrent compactions lose no entries; PRIVACY.md artifact enumeration parity test extended to include the compaction temp class
- evidence: concurrency test + registry-parity test green in the landing PR

### Session-cache save has no cross-process lock
- id: `rm-180` | track: reliability | priority: 78.0 | status: candidate
- signals: session_cache.rs::L815-861 performs read-modify-write + rename with no cross-process lock; the common collision is a statusline hook writing while an interactive run saves — one side's entries are silently lost
- acceptance: save path takes an exclusive advisory lock (adjacent lock file or sqlite-level); lock contention within a small bound degrades to a documented skip consistent with the statusline latency contract, never a hang; test spawns N concurrent saves each adding a distinct entry and asserts all N persist
- evidence: concurrency test through the public API + contention-degradation path pinned by test, in the landing PR

### Cached costs carry no pricing-snapshot identity
- id: `rm-181` | track: data-freshness | priority: 76.0 | status: candidate
- signals: session_cache.rs::L1106-1111 is_fresh keys on {size, mtime} of the session file only; SCHEMA_VERSION=20 is the sole invalidator, so after --update-pricing pulls a changed catalog, cached per-session costs are re-labeled with the new snapshot date but still computed from old prices (research b88e6baa demonstrated a $0.0105-stale vs $1.2340-labeled divergence; an earlier review finding named it but minted no id)
- acceptance: cache entries record a pricing-snapshot fingerprint (snapshot date + content hash — accessors already exist from rm-021) and is_fresh requires fingerprint match; live demo: same corpus repriced under two snapshots yields two different totals, each labeled with its true snapshot; invalidation pinned by test; catalog changes must not require a SCHEMA_VERSION bump
- evidence: two-snapshot reprice demo + invalidation test in the landing PR

### TUI test suite is non-hermetic without the cache-dir guard
- id: `rm-182` | track: reliability | priority: 74.0 | status: implemented
- signals: cargo test -p agenttrace-tui --lib WITHOUT AGENTTRACE_SESSION_CACHE_DIR fails 44/45 at ec8acdc (RC=101; tests.rs::L1972 reads the HOST's real statusline journal); with_session_cache_dir_for_test unlocks process-global env mutation; CI only runs the guarded form, so the unguarded breakage ships unnoticed
- acceptance: the suite is green with AND without the env guard, on clean HOME and dirty host HOME alike; the offending test constructs its journal fixture explicitly instead of resolving the real home; no test mutates process-global env without isolation; a CI lane (or documented local command) runs the unguarded form
- evidence: both invocation forms RC=0 in the landing PR; this cycle's failing form already recorded at /tmp/assess-5290a1cd/tui-unguarded.log for before/after comparison

### Pricing overrides bypass the finite-rate gate
- id: `rm-183` | track: correctness | priority: 72.0 | status: implemented
- signals: apply_pricing_overrides (pricing.rs::L311-316) extends the catalog with zero validation; the is_finite gate lives only in convert_litellm (pricing.rs::L462-466); parse_pricing_overrides (pricing.rs::L383-393) accepts any f64 — NaN/inf/negative override entries corrupt cost math silently; boundary: rm-050 owns the download body cap, this item owns override-entry validation
- acceptance: override entries validated at load: non-finite and negative rates rejected with a named error naming the override file and entry key; valid overrides still apply (existing override tests stay green); tests feed 1e400 (parses to inf), NaN, and negative fixtures and assert the named rejection error
- evidence: validation tests + a live --update-pricing run against a hostile overrides file showing the named error, in the landing PR

### Unbounded, uncached git subprocesses
- id: `rm-184` | track: reliability | priority: 70.0 | status: candidate
- signals: upstream.rs::L184-186 spawns git fetch with no timeout while the npm probe two functions away bounds curl with --max-time 15 (L317); the log/diff/rev-list calls in the same file are equally unbounded; governance.rs::L780-784 runs git -C root log --all per project root, uncached, per report invocation (callers: CLI main.rs::L306, TUI app.rs::L1569) — a stalled git hangs the tool indefinitely; retraction recorded: the earlier claim that git aliases could shadow builtins is FALSE (verified live: alias.log='!echo' did not execute) — only the unbounded/uncached defects stand
- acceptance: every spawned git subprocess runs under a documented timeout and degrades to a named error, never a hang; governance log results cached per (root, head) for the report run; timeout error paths pinned by tests with a stub git; boundary: rm-048 owns FETCH_HEAD trust disclosure — this item owns execution bounds and caching only
- evidence: stub-git timeout tests + a cache-hit test asserting one log invocation per root per run in the landing PR

### Terminal output perimeter: bidi and control characters unsanitized
- id: `rm-185` | track: security | priority: 68.0 | status: candidate
- signals: live probe at ec8acdc: a session named with U+202E (RTL override) renders raw in --overview (incident timeline) and --sessions (table); "model<script>" prints raw in text mode (the HTML path escapes — rm-035's surface); the sanitization class shipped for statusline (rm-034) is not applied to the terminal perimeter; contract nuance from the rm-034 fix: the sanitizer is control-bytes-only and printable CSI tails legitimately survive — so this fix must target C0/C1 control bytes plus explicit bidi-override codepoints, not printable CSI
- acceptance: terminal-facing renderers (text reports, --sessions/--overview output, TUI) pass free-text fields through one shared sanitizer neutralizing C0/C1 controls and the bidi-override class (U+202E family) — stripped or visibly escaped; byte-level test asserts no raw ESC/bidi-override bytes in captured stdout for the hostile fixture; the TUI consumes the same shared function (no third copy); if the rm-034 contract wording changes, update that item's note
- evidence: hostile-fixture stdout byte assertions + golden render outputs in the landing PR

### OpenCode: hardcoded $HOME/.local/share ignores XDG_DATA_HOME
- id: `rm-186` | track: compatibility | priority: 66.0 | status: candidate
- signals: sqlite_sessions.rs::L116-121 hardcodes $HOME/.local/share/opencode/...; live probe (research b88e6baa K3): with XDG_DATA_HOME set and a corpus present there, opencode rows fail to appear and the CLI exits rc=1 with no doctor row
- acceptance: XDG_DATA_HOME honored when set, $HOME/.local/share fallback preserved; doctor reports the resolved opencode path and flags a missing dir; tests pin both env shapes (XDG set with fixture corpus → rows appear; XDG unset → fallback)
- evidence: env-driven tests + the live probe flipped (rows appear, doctor row present) in the landing PR

### Adversarial-sqlite pytest suite is wired into zero CI lanes
- id: `rm-187` | track: reliability | priority: 66.0 | status: candidate
- signals: scripts/fixtures/test_make_adversarial_sqlite.py (10 tests, landed at ec8acdc with the rm-155..161 block) has no pytest reference in .github/workflows or scripts/ci (verified by grep this phase; first flagged by sibling assess 3c3e933d) — the suite can rot green-in-theory while never executing
- acceptance: a CI lane (or a step in an existing lane) runs the pytest suite and fails the build on test failure, with the python + pytest dependency pinned or provisioned deterministically; the lane proves itself by intentionally breaking one fixture expectation on a throwaway branch (link in the landing PR description), then reverting
- evidence: lane run green in the landing PR; the intentional-break proof run linked

### ~20 user-facing flags render blank --help descriptions
- id: `rm-188` | track: developer-experience | priority: 64.0 | status: implemented
- signals: at ec8acdc these flags print blank descriptions in --help: --compare --audit --recommend --mcp-governance --context-trends --delivery-evidence --overview --sessions --diagnostics --inspect -o --latest --waste --list-models --update-pricing --test-match --demo --search --clear-cache --preserve-history --include-history; root cause: undocumented #[arg(long)] fields in crates/agenttrace-cli/src/main.rs (help="" greps zero — missing doc comments, not empty strings); README additionally documents none of --waste/--compare/--test-match
- acceptance: every #[arg(long)] field in the CLI carries a doc comment or explicit help rendering non-blank help; a test or gate asserts --help output contains no flag entry with an empty description (golden --help snapshot or parse-and-assert); README documents --waste, --compare, and --test-match
- evidence: golden-help test green + README diff in the landing PR

### Parser holds two whole-file copies and the walk has no size gate
- id: `rm-189` | track: performance | priority: 62.0 | status: candidate
- signals: parser.rs::L23-24 does fs::read then str::from_utf8 — two whole-file RAM copies per file; the discovery walk admits any file size (no gate); 5.2 GB real corpora exist on the author's host, so pathological entries multiply memory pressure across the walk
- acceptance: the parse path holds one buffer (read-into-Vec + in-place UTF-8 validation, or streaming classification for cheap paths) with no full duplicate; the discovery walk applies a documented size gate above which files are skipped WITH skip accounting (rm-047's torn-line disclosure pattern), never silently; peak-RSS before/after on a named fixture corpus recorded in the landing PR (/usr/bin/time -v or equivalent)
- evidence: gate-accounting unit tests + the recorded RSS numbers

### History entry ids derive from an unstable hash
- id: `rm-190` | track: reliability | priority: 60.0 | status: candidate
- signals: history.rs::L104-107 derives entry ids from DefaultHasher; demonstrated instability across toolchains (same path hashed to 8b048e65 on one rustc and 76885271 on another, research b88e6baa K4) — a toolchain upgrade silently orphans prior history records (pins, exclusions, delivery evidence)
- acceptance: history ids derive from a pinned, versioned hash (documented algorithm, e.g. FNV-1a or sha over the canonical path) with the algorithm name persisted in the history file; existing ids are never rewritten (or remapped once with a recorded migration); golden-id test pins fixture-path ids so any implementation change that would renumber fails loudly
- evidence: golden-id fixture test + migration note (if any) in the landing PR

### Source-label maps triplicated across surfaces
- id: `rm-191` | track: developer-experience | priority: 58.0 | status: candidate
- signals: source→display-label maps live in three places at ec8acdc — reports.rs::L1918-1921, tui/shared.rs::L243, doctor.rs::L279; adding a source family requires three coordinated edits and the maps can drift (a fourth drift site existed at 1806e182 in presentation.rs and was consolidated then)
- acceptance: one shared mapping in agenttrace-core (e.g. source_label(source) -> &'static str) consumed by all three sites; parity test asserts every source enum variant renders the same label across surfaces; single point of addition for new sources enforced by construction or by grep gate
- evidence: parity test green + grep showing one remaining map in the landing PR

### pi-family discovery follows symlinks silently
- id: `rm-192` | track: security | priority: 56.0 | status: candidate
- signals: discovery.rs::L167-207 admits pi-family children via is_dir(), which follows symlinks; exposure is bounded (only <child>/sessions is probed) but a symlinked home can alias or loop roots and the follow behavior is undocumented
- acceptance: symlinked children under pi-family roots are skipped or explicitly annotated (symlink_metadata class check) with the decision documented; a cyclic/self-referential symlink fixture terminates; doctor reports skipped-symlink count when nonzero; non-symlink roots unchanged (regression fixture)
- evidence: symlink fixtures including a cycle + termination test in the landing PR

### check-locked-cargo gate pattern too narrow
- id: `rm-193` | track: reliability | priority: 54.0 | status: candidate
- signals: scripts/ci/check-locked-cargo.sh:31 greps for `cargo (test|build|clippy)` — misses toolchain-prefixed forms (`cargo +nightly test`), `cargo doc`, `cargo install --path`, `cargo publish`; zero live offenders at ec8acdc (swept over .github/workflows + scripts) so this is prevention-only; rider extending the landed rm-157 gate
- acceptance: pattern matches toolchain-prefixed invocations and the dependency-resolving subcommand set (test|build|clippy|run|doc|install|publish|tree) or documents an explicit exclusion list with rationale; negative fixture tests: workflow snippets with `cargo +nightly test` and `cargo doc` FAIL the gate; the gate stays green on the live tree
- evidence: negative-fixture runs + live sweep green in the landing PR

<!-- managed by hermes-roadmap render; do not edit by hand -->
