# agenttrace — Roadmap

> Autonomously maintained by the roadmap sync (reliability-first). Items cite reproducible codebase signals; acceptance is proven by cited evidence.

**Vision**: A reliable, customer-friendly repository advanced by evidence-cited roadmap cycles owned by the autonomy loop

**Pillars**: reliability work outranks customer-experience work; every roadmap item cites reproducible codebase signals; acceptance is proven by cited evidence, never claimed

## Fleet context

- dependents (changes here affect): (host), agent, maestro
- graph: evidence-derived (imports/refs/deploy surfaces); advisory

## Open items

### Add test coverage for 1 untested module(s)
- id: `rm-002` | track: reliability | priority: 83.0 | status: done
- signals: reliability.no_tests:scripts/fixtures/make-adversarial-sqlite.py
- acceptance: Every module in ['scripts/fixtures/make-adversarial-sqlite.py'] has a corresponding test file with at least one passing test
- evidence: full suite green (python -m pytest -q) at HEAD; conductor validation digest validation:v1:<sha> recorded in the shipping PR
- compound c1 2026-09-30: implemented in the campaign worktree (uncommitted, pre-review) as scripts/fixtures/test_make_adversarial_sqlite.py — first pytest convention in this repo, 5 tests; recorded outcomes: targeted gate 5 passed, full workspace suite 251/0. Flipped to done at the commit gate 2026-09-30: independent review APPROVED (five-lens, zero fixes), full workspace 251/0, targeted 5 passed, digest validation:v1:bde0b51bf06c9c706fef41075750 at base 90a4ef5, shipped in this commit

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

<!-- New items 2026-09-30 (run 88feec46 assess attempt 6c039674 + research attempt b31b51bd): numbered rm-012+ to stay disjoint from sibling campaign cc2f32d5's rm-003..rm-008 and this fork's own rm-001/rm-002/rm-009..rm-011 -->
<!-- ID-collision record (commit gate 2026-09-30, review P2 of attempt c429ef80): campaign-local rm-012..rm-019 COLLIDE with distinct integrated items on origin/master ad503da (run 02993de2 / PR #16 renumber, campaign e7206fe5, landed while this campaign ran). Disposition per the PR #16 precedent: renumber this campaign's block at integration to the next free contiguous range above all landed AND claimed IDs at that time (today: master up to rm-019; open PR #17 claims rm-017..rm-024 stale-low; run cbe30a9c's tree claims rm-025..rm-033) and record the old>new mapping here; all phase artifacts, the independent review, and the commit message use the campaign-local IDs -->

### Sanitize statusline report output (terminal control injection from journal payloads)
- id: `rm-012` | track: reliability | priority: 89.0 | status: done
- signals: security.terminal_injection:crates/agenttrace-core/src/statusline.rs::L615+L628+L636-644 (report path prints journal-derived session_id and miss_cause strings raw; the render path sanitizes the same class at L263-268 sanitize_line_segment; ingestion stores hostile payloads verbatim at L275 append_statusline_capture; live repro 2026-09-30: crafted journal payload emitted raw ANSI SGR + OSC-52 through `agenttrace --statusline-report`)
- acceptance: every string printed by --statusline-report passes the same control-character sanitization as the render path (shared helper), locked by a regression test asserting ESC/OSC sequences appear sanitized in report output; TUI panel stays numeric-only
- evidence: cargo test green including the new injection regression test; live repro re-run shows no raw ESC bytes in report output (cat -v); conductor validation digest validation:v1:<sha> recorded in the shipping PR — compound-c1 (2026-09-30, run 88feec46): implemented uncommitted at 7bb4dcb — all three report print sites route through the shared sanitizer (statusline.rs:619/:632/:645 over sanitize_line_segment:265); regression test statusline_report_sanitizes_journal_derived_strings (statusline.rs:909); targeted+full gates green 238/0, digest validation:v1:9c9a1d4d5e3067b03657b0b4984c45be5b0358487a19497c64241c6224082088; live cat -v re-run shows U+FFFD and zero raw ESC; learning recorded at docs/solutions/security-issues/terminal-injection-through-journal-derived-statusline-strings.md; done-flip reserved for the commit gate — flipped 2026-09-30 (commit gate, run 88feec46 cycle 1)

### Fix Codex token double-counting after compaction (adopt upstream #286 high-water fix)
- id: `rm-013` | track: reliability | priority: 88.0 | status: done
- signals: correctness.double_count:crates/agenttrace-core/src/parser.rs::L2257 (codex_token_count_usage computes token_usage_delta(&total, prev_total) on the raw cumulative total; when Codex rewinds total_token_usage after compaction and climbs back, the rebound is re-counted; upstream fix be25c4c "Track Codex cumulative token high-water mark so rewound totals are not re-counted" merged 2026-09-30 and present in the local git object store; our fork point 6848aa1 predates it)
- acceptance: token accounting tracks a cumulative high-water mark so rewinds never double-count; a fixture with rewind+rebound yields exactly the sum of true per-event deltas; estimated Codex costs are unaffected by compaction events
- evidence: new regression test with a rewound-then-risen total_token_usage fixture asserting single-counted tokens; cargo test green; conductor validation digest validation:v1:<sha> recorded in the shipping PR — compound-c1 (2026-09-30, run 88feec46): be25c4c's high-water hunk transplanted hunk-level (token_usage_high_water at parser.rs:2313 feeding the codex_token_count_usage call at parser.rs:2258), never the 11-file wave; regression test codex_total_usage_rewind_after_compaction_is_single_counted (parser.rs:4443) asserts single-counted 620 input/590 output/480 cache-read vs 870/630 pre-fix, proven by a revert-production-hunk discrimination re-run; targeted+full gates green 238/0, digest validation:v1:9c9a1d4d5e3067b03657b0b4984c45be5b0358487a19497c64241c6224082088; done-flip reserved for the commit gate — flipped 2026-09-30 (commit gate, run 88feec46 cycle 1)

### Parse JSONL once across format probes
- id: `rm-014` | track: reliability | priority: 80.0 | status: candidate
- signals: performance.reparse:crates/agenttrace-core/src/parser.rs (parse_raw_session probes each candidate format by re-parsing the raw JSONL text per probe; upstream be25c4c refactored the same chain to parse objects once and probe the parsed form as part of its +1249/-369 wave)
- acceptance: format detection parses the source text once and shares the parsed objects across all probes; unknown-format files show a single parse pass; existing parser golden behavior unchanged
- evidence: cargo test green (existing parser goldens); instrumentation or benchmark note documenting single-pass detection on a mixed corpus; conductor validation digest validation:v1:<sha> recorded in the shipping PR
- notes: compound-c1 sequencing held — deliberately behind rm-013 so the parse-once refactor could not muddy the rewind fixture's baseline; when implementing, transplant the be25c4c probe-refactor hunks hunk-level (never the whole 11-file wave) and prove with a revert-production-hunk discrimination re-run, the pattern rm-013 used

### Catch up statusline schema: spend_limit window + structured repo identity
- id: `rm-015` | track: customer-experience | priority: 78.0 | status: candidate
- signals: contract.gap:crates/agenttrace-core/src/statusline.rs::L215+L405-502 (only five_hour/seven_day rate-limit windows modeled; live Claude Code statusline docs 2026-09-30 additionally document rate_limits.spend_limit.used_percentage/resets_at, silently dropped), contract.gap:workspace.repo.{host,owner,name}+workspace.git_worktree+added_dirs+session_name+effort.level+exceeds_200k_tokens (documented fields absent from the payload model; repo identity would also feed insights attribution and reduce resolve_project filesystem walks)
- acceptance: spend_limit parsed and rendered alongside the two existing windows and recorded in journal insights when present; workspace.repo identity captured into the journal record and used by insights attribution when available; payload model tolerates unknown future fields without error
- evidence: statusline unit tests with a fixture carrying spend_limit and workspace.repo; round-trip journal entry showing the new fields; cargo test green; conductor validation digest validation:v1:<sha> recorded in the shipping PR
- notes: compound-c1 disposition — named the cycle-2 pair with rm-016 per the prioritize record (run 88feec46 attempt 51899064)

### Memoize session project identity in TUI hot paths
- id: `rm-016` | track: reliability | priority: 77.0 | status: candidate
- signals: performance.hot_walk:crates/agenttrace-tui/src/app.rs::L1427-1429 (filter predicate calls resolve_project -> parent-dir git-root walk per session per refresh), performance.hot_walk:crates/agenttrace-tui/src/explorer.rs::L556-559+L568 (sort comparator and retain walk per element, O(n log n) walks), performance.hot_walk:crates/agenttrace-tui/src/filters.rs::L251-253 (label lookup per render); walk implementation crates/agenttrace-core/src/insights.rs::L148-184 with no cache
- acceptance: resolve_project results memoized per session identity and populated at load; TUI filter/sort/label paths perform zero additional filesystem walks after initial resolution; behavior identical for sessions outside git repos
- evidence: instrumentation or test demonstrating walk count independent of sort/refresh; cargo test green; conductor validation digest validation:v1:<sha> recorded in the shipping PR

### Remove dead self-hosted cache steps from hosted-runner CI lanes
- id: `rm-017` | track: reliability | priority: 74.0 | status: candidate
- signals: ci.dead_step:.github/workflows/ci.yml::L44-63+L69-78 (Restore/Save cargo-target tar steps with self-hosted persistence comments run on ubuntu-latest since 6ba55ea #11 2026-09-22; the 369M target tar is written to a path no later hosted job can restore), docs.drift:.github/workflows/dependency-review.yml::L7 (comment claims ci.yml runs "cargo audit + cargo deny"; only cargo-deny exists)
- acceptance: lint lane uses actions/cache keyed on Cargo.lock (or the dead steps are deleted) so PR wall-time no longer includes a dead 369M tar; dependency-review comment names exactly the gates that exist
- evidence: CI run on the changed workflow shows the lint lane green without the tar step; grep shows no stale self-hosted persistence comments; conductor validation digest validation:v1:<sha> recorded in the shipping PR

### Fix --range today UTC boundary
- id: `rm-018` | track: reliability | priority: 72.0 | status: candidate
- signals: correctness.timezone:crates/agenttrace-core/src/insights.rs::L65-70 (Today range computes UTC midnight boundaries rather than the user's local day)
- acceptance: today range anchors to local midnight with documented timezone handling; a test pinning the boundary around a fixed offset passes
- evidence: new unit test for the local-day boundary; cargo test green; conductor validation digest validation:v1:<sha> recorded in the shipping PR

### Harden discovery-cache keys and listing freshness
- id: `rm-019` | track: reliability | priority: 73.0 | status: candidate
- signals: reliability.lossy_key:crates/agenttrace-core/src/session_cache.rs::L1031-1033 (cache keys built via to_string_lossy: non-UTF-8 paths vanish from cached discovery or collide), reliability.stale_listing:crates/agenttrace-core/src/session_cache.rs::L513 (directory-listing freshness keyed on mtime alone; same-tick creates after store stay invisible until the next mtime change)
- acceptance: cache keys encode paths losslessly (OsStr bytes); listing freshness detects same-tick changes (size/inode or forced rescan); regression tests cover both cases
- evidence: new tests for non-UTF-8 path round-trip and same-tick create visibility; cargo test green; conductor validation digest validation:v1:<sha> recorded in the shipping PR

### Add transcript-derived 5-hour billing block analytics
- id: `rm-020` | track: customer-experience | priority: 76.0 | status: candidate
- signals: user_need.ccusage_blocks (ccusage/ccusage 18,813 stars; blocks report groups usage into 5-hour billing windows with active-block burn rate and projections — fetched live 2026-09-30), capability.gap (our rate-limit visibility exists only via the opt-in statusline journal; CLI-only users have no 5h window view derived from local transcripts)
- acceptance: a blocks command/report groups transcript usage into 5-hour windows aligned to first use, reports per-block cost and token totals by model, and marks the active block with its current burn rate; works with no statusline configured
- evidence: golden test on a synthetic transcript corpus asserting block boundaries and totals; report rendered for a fixture corpus; conductor validation digest validation:v1:<sha> recorded in the shipping PR

### Unify crossterm on 0.29
- id: `rm-021` | track: reliability | priority: 75.0 | status: candidate
- signals: deps.duplicate:cargo tree -i crossterm (0.28.1 direct plus 0.29.0 via ratatui 0.30.2 -> ratatui-crossterm 0.1.2; two copies compiled into every binary; crossterm 0.29.0 is latest per crates.io 2026-09-30)
- acceptance: a single crossterm version in the dependency graph; TUI behavior unchanged (full tui test suite green); no new advisories introduced
- evidence: cargo tree -i crossterm shows one version; cargo test green; cargo-deny advisories ok; conductor validation digest validation:v1:<sha> recorded in the shipping PR

### Refresh ureq and rusqlite majors
- id: `rm-022` | track: reliability | priority: 71.0 | status: candidate
- signals: deps.stale:ureq 2.12.1 (ureq 3.4.2 current per crates.io 2026-09-30; our rustls RUSTSEC exposure rode the ureq 2.x graph), deps.stale:rusqlite 0.32 (0.40.2 current per crates.io 2026-09-30)
- acceptance: pricing transport migrated to ureq 3.x with identical request behavior locked by existing pricing-fetch tests; hermes/opencode sqlite reads migrated to current rusqlite with golden db fixtures passing
- evidence: cargo test green including pricing and sqlite fixture suites; cargo-deny advisories ok; conductor validation digest validation:v1:<sha> recorded in the shipping PR

### Adopt upstream release-engineering wave (crt-static, git-cliff CHANGELOG, republish workflows)
- id: `rm-023` | track: reliability | priority: 70.0 | status: candidate
- signals: release.gap (upstream #285 .cargo/config.toml crt-static MSVC builds merged 2026-09-30 while this fork has no .cargo/config.toml, so Windows users need the VC redistributable; #289 git-cliff automated CHANGELOG addresses the hand-maintained Unreleased section; #291/#292 republish plus pinned wingetcreate harden the distribution channels)
- acceptance: Windows release binaries link the static CRT (no VC redist requirement, verified by dumpbin or a CI artifact check); CHANGELOG generated by git-cliff with existing history preserved as prior context; release republish steps pinned and exercised once
- evidence: release CI artifacts built with crt-static; generated CHANGELOG diff reviewed against current hand-maintained entries; conductor validation digest validation:v1:<sha> recorded in the shipping PR

<!-- New items 2026-10-01 (run b8d7db050713 roadmap phase, repository-maintenance c1b65b397f3940619e1164ac359e2660 cycle 2, attempt 7a959112; prior attempt envelope died on a provider 429 before any file edit — this is the first landing) at HEAD 9d88b36: sources = this run's assess 3eef69c9 (F1-F10) + research b7b82926 (N1-N4, local-corpus format-drift lane + ecosystem refresh). Numbered rm-084+ — above every live claim re-verified at mint time by fleet-wide scan of agenttrace-80c75f65b7/run-*/ROADMAP.md (30 files; ceiling rm-083 in run-e602bb692466's uncommitted block; canonical /work/projects/agenttrace stale at 90a4ef5 holding rm-001..rm-027 only; agenttrace-lineage/agenttrace-phase2-land stale at rm-001..002). rm-IDs are campaign-local until merge; ID reconciliation is the commit phase's concern.
Deliberately NOT minted (dedup discipline — fold-in evidence recorded so it is not lost):
- assess F1 cross-file copied-request double-count in aggregate totals (P2, live-repro'd dir-mode: two fixture sessions sharing message id msg_1 -> ./target/release/agenttrace a.jsonl b.jsonl -f json -> total_tokens 330 vs unique-request truth 180) is CLAIMED by the unmerged integration lane (integration-9c6b6f4e1107 / cbe30a9c) as rm-048 "Cross-session usage dedupe and fork identity", deferred their cycle 1 — do NOT re-mint or implement without reconciling with that lane (two prior roadmap phases, 1766ab4e b0d93067 and e602bb69 d0721c50, already recorded the same boundary). NEW fold-in evidence from this run for rm-048's eventual integration: (a) the aggregation site is reports.rs:1604-1607 (sessions.iter().map(total_tokens).fold(saturating_add)) — the dedupe must engage at load/aggregate, not only in-parser; (b) research b7b82926: codex upstream shipped first-party resume-copies-history on 2026-09-30 (#49600 "Reuse unchanged history snapshots when resuming threads", #49599 "Return authoritative replay history when resuming a thread") — the copy pattern is now tool-native, raising the item's priority; (c) within-file dedupe keys on message.id (parser.rs:2365-2415) which is exactly the key that repeats across fork copies.
- assess F2 hermes sqlite sessions always report 100% tool success (sqlite_sessions.rs:195-196: tool_calls_ok mirrors the SAME column 5 as tool_calls_total, so tool_calls_fail is always 0; downstream the lib.rs:969-975 tool-fail anomaly can never fire and the --max-tool-fail-rate gate is a no-op on hermes DBs) is CLAIMED by run-40208f3d2ca8's "Stop reporting fabricated 100% tool success for hermes sessions" — fold the gate-no-op consequence (lib.rs:1467 + main.rs:1420-1424) into that item's acceptance.
- assess F4 unbounded per-root git log delivery evidence is CLAIMED by run-40208f3d2ca8's rm-076 (governance.rs:780-781 caching/bounding) — fold this run's additional axis: the scan runs via .output() with NO timeout (:779-787), so a hung git on FUSE/NFS hangs the CLI indefinitely; the timeout belongs in rm-076's bounding fix, not a second item.
- assess F9 preserved history unbounded growth is CLAIMED by run-83642957d130's rm-064 (history.rs:37-47 retention cap + eviction) — fold: eviction must also handle dead sessions and the --include-history load path (history.rs:44-57 appends ALL scanned sessions each run; no TTL).
- research N2's catalog/consumption half (Price gains above_1hr rates, tier-selected billing) is CLAIMED by run-c7dca75d30e3 rm-015 "Pricing tier and price-class support" (mirrored in run-aa9c4fd68e5b) — rm-086 below deliberately claims ONLY the transcript-side producer (ephemeral 5m/1h split capture + snapshot key), per the boundary discipline; merge or sequence the two at integration.
- research N4 server-side tool billing (usage.server_tool_use.{web_search_requests,web_fetch_requests} present on 200/212 live corpus messages, all-zero on this host = latent; LiteLLM main carries search_context_cost_per_query $0.01/query on claude-sonnet-4-5 and claude-opus-4-6) extends run-40208f3d2ca8's rm-073 "Codex session-envelope fidelity batch" acceptance: join captured request counts with per-query catalog cost into cost totals + provenance. Same fold target for the queue-operation observation (109/628 corpus lines, operation:dequeue) if latency analytics is ever wanted. No new mint.
- upstream/ecosystem refresh (all live 2026-09-30T16:50-17:10Z): upstream tip still 41ac231 with the fork 8 behind (#287-#294) — #287-290 content already routed (rm-010 done, rm-018 remainder) and 1766ab4e's b0d93067 note already routes #291-294 -> rm-045/rm-018, so no new item; ccusage pricing automation confirmed hourly (12 snapshot commits in one day — strengthens rm-062's cadence case); codex CLI is the fast mover (v0.159.2 -> v0.161.0-alpha.4, 4 releases in 36h — strengthens rm-084/rm-003 drift posture); crates.io maxima unchanged (rm-007/rm-021/rm-022 figures current); tokenmaxzing demand signal now 404 (retire it); ccusage still v20.0.26 (flex/service-tier pricing unshipped). Codex world_state top-level events (92/96 files) remain defensibly skipped — size, state blobs, no item.
- No existing item or status was modified. -->

### Parse Codex custom tool calls (`custom_tool_call`/`custom_tool_call_output`)
- id: `rm-084` | track: compatibility | priority: 92.0 | status: candidate
- signals: compatibility.parser_blind_spot:crates/agenttrace-core/src/parser.rs::L2157-2243 (codex response_item match arms are exactly message / function_call / function_call_output|function_call_result, then _ => {}; grep -c custom_tool_call parser.rs = 0 at HEAD 9d88b36 — verified), drift.corpus (96 live host rollouts: 497 custom_tool_call + 517 custom_tool_call_output + 70 function_call = 87% of tool traffic custom-tool shaped; the 12 newest: 149 custom_tool_call all name:"exec" and ZERO function_call — current codex CLIs run the shell as a custom tool; upstream shipped v0.160/v0.161 alphas 2026-09-30, direction is MORE custom tools)
- acceptance: parser gains custom_tool_call and custom_tool_call_output arms mapped onto the existing tool-call model (name, input/output, ok/fail pairing, call latency); on the preserved 12-fixture codex-sample corpus the live repro flips from data_health.with_tools=0 and recent_sessions[].tools=0 to with_tools=12 and per-session tools>0; a golden fixture test pins custom-tool call counts, failure accounting, and tool-authority classification; the --max-tool-fail-rate gate and governance tool metrics exercise the custom-tool path in a test; standalone reasoning response_items (431 in corpus) are parsed for content or their skip is explicitly documented
- evidence: research b7b82926 live repro (HOME-sandboxed `./target/debug/agenttrace -d codex-sample --overview -f json` -> with_tools=0, tools=0 across 12 sessions containing 149 real exec calls; fixtures + probe JSON preserved in spool research-b7b82926335d434091c12995a67e023f/codex-sample/); fleet wall grep 2026-10-01: custom_tool_call claimed in zero sibling ROADMAPs; adjacent-but-distinct from 40208f3d2ca8's rm-073 (envelope FIELDS vs tool EVENTS — record adjacency at integration)

### Codex capacity telemetry: `token_count.rate_limits` + `info.model_context_window`
- id: `rm-085` | track: customer-experience | priority: 71.0 | status: candidate
- signals: capability.dropped_telemetry:crates/agenttrace-core/src/parser.rs::L2144-2146+L2273+ (codex_token_count_usage reads only the info token buckets; grep -c 'rate_limits|world_state|model_context_window' parser.rs = 0 at HEAD 9d88b36 — verified; the event lines ARE decoded, only these fields are ignored), corpus.saturation (all 620 token_count events across the 96-file live corpus carry payload.rate_limits AND info.model_context_window)
- acceptance: rate-limit window usage and model context-window size from token_count events are captured into derived session state storage-first (no rendering dependency); a per-turn context-pressure series (consumed vs model_context_window, and rate-limit window consumption) renders additively in session detail and JSON; composes with the landed codex high-water token tracking (rm-013 done) rather than re-deriving tokens; golden fixture with rate_limits + context window pins the series; world_state skipping stays documented-defensible (size, state blobs)
- evidence: research b7b82926 corpus scan (structure-only probes of live ~/.codex/sessions); fleet wall grep 2026-10-01: the only sibling rate_limits hits are statusline-schema text (spend_limit), none claim codex telemetry; no competitor ships capacity telemetry (ccusage v20.0.26 surface checked)

### Capture the tiered cache-write split (ephemeral 5m/1h) storage-first
- id: `rm-086` | track: correctness | priority: 73.0 | status: candidate
- signals: correctness.single_tier_sum:crates/agenttrace-core/src/parser.rs::L1573 (cache_write = sum_numbers(obj, &["cacheWrite", "cache_creation_input_tokens"]) on the claude path, codex path :1996 same; grep -c ephemeral parser.rs = 0 — verified at HEAD), drift.latent (live claude-code v2.1.285 corpus: 200/212 assistant messages carry usage.cache_creation.{ephemeral_5m_input_tokens, ephemeral_1h_input_tokens}, all-zero on this host — latent format drift, no live cost repro claimed), catalog.gap (bundled 2026-09-13 snapshot lacks cache_creation_input_token_cost_above_1hr entirely; live LiteLLM 2026-09-30: claude-sonnet-4-5 3.75e-06 -> 6.0e-06 above 1hr, claude-opus-4-6 6.25e-06 -> 1.0e-05)
- acceptance: parser captures the ephemeral 5m/1h split into per-tier counters before any pricing (storage-first discipline — the fields are carried in derived state even where pricing is not yet tier-aware); the bundled snapshot regenerates to carry cache_creation_input_token_cost_above_1hr; where the catalog provides both tiers, cache-write cost bills each tier at its own rate; golden test with a non-zero 5m+1h fixture asserts both tier counters and the two-rate sum; deliberately ONLY the transcript-side producer lane — the Price/tier-selection and consumption side is claimed by c7dca75d rm-015 (mirror in aa9c4fd6); sequence or merge with it at integration
- evidence: research b7b82926 corpus scan + live LiteLLM catalog fetch (probe commands re-runnable from the research ledger); fleet wall grep 2026-10-01: ephemeral* claimed nowhere (only hit is unrelated CI text "ephemeral-runner")

### Preserved history re-attaches to real projects
- id: `rm-087` | track: correctness | priority: 66.0 | status: candidate
- signals: correctness.display_name_as_path:crates/agenttrace-core/src/history.rs::L139 (into_session sets cwd = self.project, the bare DISPLAY NAME), correctness.cwd_dependent_resolution:crates/agenttrace-core/src/insights.rs::L166-199 (resolve_project walks git_root of that name resolved RELATIVE to the invocation cwd — same preserved-history corpus groups differently depending on where the user runs the CLI; root-keyed views always split preserved history from the live sessions of the same project), dead.guard:insights.rs::L179 (the raw.starts_with("history:") branch is unreachable — project_name never yields that prefix)
- acceptance: DerivedSession persists the resolved project ROOT (or a root fingerprint) at preserve time and --include-history resolution uses it, so preserved sessions group under the same project root as live sessions regardless of invocation cwd; the dead "history:" guard is removed or made live with a test; a regression test runs the same crafted history.json from two different cwds (one containing a same-named .git dir, one without) and asserts identical project grouping
- evidence: assess 3eef69c9 F5 live repro (crafted BTreeMap<String, DerivedSession> record with project "agenttrace-probe" resolves as its own project / merges with a same-named cwd repo depending on invocation dir); fleet wall grep 2026-10-01: no sibling claims the preserved-history re-attachment axis ("Cache resolve_project"/"Memoize session project identity" are perf-cost items on the live-session path); adjacency: 40208f3d2ca8's rm-072 re-keys the same history.rs id neighborhood — land with or after it

### Sub-second session-id salt for SQLite-sourced sessions
- id: `rm-088` | track: reliability | priority: 63.0 | status: candidate
- signals: reliability.colliding_key_inputs:crates/agenttrace-core/src/history.rs::L105-111 (session_id = SipHash over (session.path, session_start); for sqlite sessions path is the SHARED db file path, sqlite_sessions.rs:203, and session_start is second-granularity, sqlite_sessions.rs:776 SecondsFormat::Secs), consequence (two sessions starting in the same second inside one DB hash identically -> the history.rs:44-57 BTreeMap insert silently overwrites one preserved record and any id-keyed same-session merge false-merges)
- acceptance: session_id inputs for sqlite-sourced sessions include a per-session discriminator (session uuid or first-message hash) so two same-second sessions in one DB yield distinct ids; a fixture with two same-second sessions preserves BOTH records and never false-merges; boundary recorded: distinct from 40208f3d2ca8's rm-072 (hash ALGORITHM stability across Rust releases) and 83642957d130's rm-055 (alias SPELLING forks) — same history.rs:104-107 neighborhood, three different defects; expect text-level conflict at integration, do not drop the sibling hunks
- evidence: assess 3eef69c9 F3, anchors re-verified at HEAD 9d88b36 this phase (sqlite_sessions.rs:195-196/203/776, history.rs:105-111/44-57); fleet wall grep 2026-10-01: no sibling claims the collision-input axis

### Low-severity correctness and robustness batch (evidence ordering, NaN filters, clipboard timeout)
- id: `rm-089` | track: reliability | priority: 56.0 | status: candidate
- signals: correctness.lexicographic_level_sort:crates/agenttrace-core/src/governance.rs::L745-749 (delivery_evidence records sort_by raw level string — ascending renders medium < none < non_code < strong < weak, i.e. no-evidence entries rank ABOVE strong), correctness.nan_filter:crates/agenttrace-cli/src/main.rs::L1034-1038 (valid_number_filter accepts any f64 the parser yields, and Rust parses "NaN" — `--cost >=NaN` validates then silently matches nothing), reliability.unbounded_clipboard_wait:crates/agenttrace-tui/src/explorer.rs::L1957-1980 (copy_summary spawns xclip/xsel then child.wait() with no timeout — a wedged clipboard helper freezes the TUI thread)
- acceptance: level sorts by an explicit rank map (none/non_code ranked below every evidence grade or grouped separately) locked by a golden ordering test; valid_number_filter rejects non-finite floats (and empty numeric operands) with an explicit CLI error instead of a silent empty result; copy_summary uses a bounded wait with kill-on-expiry so the TUI never blocks on a wedged helper; one test per sub-defect
- evidence: assess 3eef69c9 F6/F7/F8, anchors re-anchored and verified at HEAD 9d88b36 this phase (governance.rs:745-749; main.rs:1034-1038 — the assess ledger's :1016-1021 pointed at the caller block, the function is at :1034; explorer.rs:1957-1980 — the assess ledger's :1727-1753 pointed at the overlay renderer, copy_summary is at :1957); fleet wall grep 2026-10-01: no sibling claims any of the three loci

<!-- managed by hermes-roadmap render; do not edit by hand -->
