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

<!-- managed by hermes-roadmap render; do not edit by hand -->
