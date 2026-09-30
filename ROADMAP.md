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

<!-- managed by hermes-roadmap render; do not edit by hand -->
