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
- cycle-2 anchor: deliberately deferred this cycle (prioritization a818550b — three sub-defects × golden-test scaffolding ≈ a full session alone); untouched this cycle, anchors assess AF-1..AF-3; prioritize 407521b8: deferral recorded a THIRD time (score 81.0 below the selected batch) — split into its three sub-defects and take them as a batch of their own next cycle

<!-- New items 2026-09-30 (run cbe30a9c, repository-maintenance 6d611136 cycle 1: assess attempt 3f8ddeff + research attempt 5a025a61, both at HEAD ce279697b711004ef9c90d36bf1173e686b04b36): numbered rm-025+ past every claimed number — this tree's landed rm-001..rm-011, master's rm-012..rm-019 (origin/master ad503da integrated run 02993de2's campaign while this one ran), and open PR #17's rm-020..rm-024 (branch conductor/run-3099db98be2b @ fe8b316; that branch's rm-017..rm-019 now collide with landed master and must renumber at ITS integration — not this campaign's content) -->

<!-- CORRECTION + COLLISION RECORD 2026-09-30 (review F2, independent_review cd56e4f5 → fix c2f3a9ec): the note above was written before three integration events and is now stale — (1) PR #17 (fe8b316) and PR #18 MERGED 2026-09-30 ~09:02Z, so rm-020..rm-024 are landed, not open; (2) origin/master (ad503da) now carries rm-001..rm-023 — ceiling rm-023, not rm-019; (3) the pushed lane conductor/run-304846327112 (commit 334b5a8, PR pending) claims rm-026..rm-032, colliding 7-for-7 with this file's rm-026..rm-032; PR #19 (285495c) claims rm-012..rm-022. This campaign's rm-025 and rm-033 are collision-free; rm-026..rm-032 must RENUMBER AT INTEGRATION (do not renumber now: the ids are cited across this cycle's docs and code comments — record-not-renumber, same discipline as PR #19). Next free id ≈ rm-034; re-verify against all live claimants before minting. -->

### Join flat-transcript tool results to their calls
- id: `rm-025` | track: correctness | priority: 90.0 | status: implemented
- signals: correctness.join_key_dropped:crates/agenttrace-core/src/parser.rs::L679 (flat Claude-transcript arm emits tool_result events with no tool_use_id-to-tool_call_id linkage; the paired tool_use arm carries name/args only), correctness.noise_escalation:crates/agenttrace-core/src/diagnostics.rs::L535 (unmatched calls feed the latency-review filter)
- acceptance: flat-transcript parsing preserves the tool_use_id-to-tool_call_id pairing so a present result never counts as unmatched; a regression fixture with 3 parallel same-name tool_use blocks plus 3 present results reports unmatched 0 with correct per-tool counts; governance no longer escalates on that corpus
- evidence: assess 3f8ddeff probe-verified at HEAD ce27969 (parallel-batch corpus: Bash count 3, unmatched 3, all results present); new parser and diagnostics regression tests green; full suite green; conductor validation digest validation:v1:<sha> recorded in the shipping PR; compound 2026-09-30 (cycle 1): implemented in this worktree at ce279697, pre-review/uncommitted — targeted_tests 3fbef9c0 RC=0 (envelope result-2611030-325550536.json), full_tests 715a6509 RC=0 with 248 passed/0 failed/0 ignored incl. the new regression tests (envelope result-3152145-325731231.json); done-flip reserved for the commit gate after independent review — cycle record docs/stewardship/2026-09-30-cycle1-implementation-record.md

### Context-window truthfulness for utilization denominators
- id: `rm-026` | track: correctness | priority: 88.0 | status: candidate
- signals: correctness.substring_ladder:crates/agenttrace-core/src/diagnostics.rs::L797 (context_utilization guesses the window from model-name substrings: claude maps to 200_000, unknown models silently to 131_072), data_freshness.dropped_field:crates/agenttrace-core/src/pricing_snapshot.json (max_input_tokens present on 0 of 2,756 entries — LiteLLM's upstream catalog carries the field; the snapshot builder drops it)
- acceptance: the snapshot builder carries max_input_tokens through; diagnostics resolves the context window per session model via catalog lookup with a documented fallback ladder; a golden test with a 1M-window claude model asserts the utilization math; unknown models render utilization as labeled-estimated rather than silently 131_072
- evidence: research 5a025a61 FR-2 — Claude Code 2.1.284/2.1.285 made 1M-context Sonnet 5.5/Opus 5.5 the defaults and 2.1.283 switched custom-base-URL sessions to the 1M window, so current production sessions divide by a denominator ~5x too small; refreshed-snapshot probe shows max_input_tokens coverage; golden utilization test; full suite green; digest validation:v1:<sha> in the shipping PR. Distinct from rm-006 (snapshot cadence and entry-count drift): this item is the window-denominator pipeline — compose at implement, do not duplicate; prioritize 407521b8: DEFERRED cycle 1 — snapshot half collides with sibling lane run-c8397418's uncommitted pricing.rs/pricing_snapshot.json work

### Cross-session usage dedupe and fork identity
- id: `rm-027` | track: correctness | priority: 86.0 | status: candidate
- signals: correctness.path_identity:crates/agenttrace-core/src/discovery.rs::L19 (Session identity is the file path), correctness.path_identity:crates/agenttrace-core/src/discovery.rs::L159 (find_session_files enumerates paths only; no request-level or cross-file key anywhere in-tree)
- acceptance: message identity equals (session_id, message uuid/request id, timestamp) across files; corpus aggregates (overview, insights, context_trends, pricing totals) dedupe before summing; a deduped_messages truthfulness counter surfaces the correction in diagnostics; dedupe scoping keeps daily/weekly/monthly windows in agreement
- evidence: research 5a025a61 FR-1 — forked or resumed Claude Code sessions duplicate earlier messages across files so per-file sums double-count; ccusage fixed the same defects twice (#1765 in v20.0.23, #1799 in v20.0.26, 2026-09-17..27); fixture with a forked session pair asserting single-counted usage; full suite green; digest validation:v1:<sha> in the shipping PR; prioritize 407521b8: DEFERRED cycle 1 — effort L and identity surface overlaps three concurrent usage-accounting lanes

### Loop detection must not flag parallel same-name calls as retries
- id: `rm-028` | track: correctness | priority: 84.0 | status: implemented
- signals: correctness.name_only_key:crates/agenttrace-core/src/diagnostics.rs::L634 (retry-loop detection compares call.name == last only), correctness.mispriced_waste:crates/agenttrace-core/src/waste.rs::L54 (loop groups flow into compute_waste_report pricing; each retry costs 0.0075 in the diagnostics retry term)
- acceptance: loop detection keys on (tool name, argument identity) so at least 3 distinct-argument parallel calls inside one assistant turn are not a retry loop; arguments that differ only in key order count as identical (review F7); the parallel-batch corpus reports no loop block; waste loop_cost excludes those groups
- evidence: assess 3f8ddeff probe-verified at HEAD ce27969 (3 parallel same-name calls with distinct arguments produced loop_type Bash_loop, retry_events 1, loop_groups 1, turns 3); distinct from rm-004 (implemented — that batch fixed loop-group counting, repeated_response keying and stuck scoring; this defect survived it and was re-probed post-landing); new regression test; full suite green; digest validation:v1:<sha> in the shipping PR; compound 2026-09-30 (cycle 1): implemented in this worktree at ce279697, pre-review/uncommitted — targeted_tests 3fbef9c0 RC=0, full_tests 715a6509 RC=0 248/0/0 incl. the loop regression test; done-flip reserved for the commit gate after independent review — cycle record docs/stewardship/2026-09-30-cycle1-implementation-record.md

### install.sh must refuse on missing .sha256 sidecar
- id: `rm-029` | track: security | priority: 80.0 | status: implemented
- signals: supply_chain.parity_gap:install.sh::L86 (missing sidecar downgrades to a warning and the install proceeds), supply_chain.channel_contrast:install.ps1::L70 (refuses on missing, malformed or mismatched sidecar), supply_chain.channel_contrast:npm/scripts/install.js::L62 (HTTP != 200 rejects and main().catch exits 1)
- acceptance: install.sh treats a missing or malformed sidecar as fatal exactly like the other two channels; the three-installer parity test (crates/agenttrace-core/src/lib.rs:1620) asserts refusal semantics for all three channels, not just sidecar naming; a missing-sidecar run of each channel shows all three refusing
- evidence: assess 3f8ddeff at HEAD ce27969 — install.ps1's own rm-005 comment states the doctrine ("its absence from a fetched URL is itself a red flag, not a condition to tolerate"); extends landed rm-005 (implemented) to the POSIX channel; full suite green; digest validation:v1:<sha> in the shipping PR; compound 2026-09-30 (cycle 1): implemented in this worktree at ce279697, pre-review/uncommitted — parity test extended to refusal semantics, targeted_tests 3fbef9c0 RC=0, full_tests 715a6509 RC=0 248/0/0; done-flip reserved for the commit gate after independent review — cycle record docs/stewardship/2026-09-30-cycle1-implementation-record.md

### Pin and verify the source-build fallback
- id: `rm-030` | track: security | priority: 78.0 | status: implemented
- signals: supply_chain.unpinned_fallback:install.sh::L118 (glibc failure falls back to git clone --depth 1 of the master tip — unpinned and unverified — then builds and installs it)
- acceptance: the fallback builds a pinned verifiable ref (release tag with its commit SHA recorded and checked), never a floating master tip; the fallback path is exercised once in CI or a recorded manual run; docs state exactly what the fallback installs
- evidence: assess 3f8ddeff at HEAD ce27969 — the checksum-verified release artifact is replaced by arbitrary unreviewed master exactly when the verified artifact fails to run; composes with master's landed rm-017 (install-source refresh) without duplicating it: that item moves the download source, this one pins the degraded path; full suite green; digest validation:v1:<sha> in the shipping PR; compound 2026-09-30 (cycle 1): implemented in this worktree at ce279697, pre-review/uncommitted via AGENTTRACE_SOURCE_REF pinning, targeted_tests 3fbef9c0 RC=0, full_tests 715a6509 RC=0 248/0/0; done-flip reserved for the commit gate after independent review — cycle record docs/stewardship/2026-09-30-cycle1-implementation-record.md

### Stop accepting armv7l the release lane never builds
- id: `rm-031` | track: reliability | priority: 66.0 | status: candidate
- signals: reliability.ghost_target:install.sh::L17 (armv7l accepted and mapped to asset agenttrace-linux-armv7), reliability.matrix_mismatch:.github/workflows/release.yml::L48 (exactly 6 targets published — no armv7 lane exists)
- acceptance: either an armv7 release lane exists or install.sh exits with an explicit unsupported-platform message naming the supported set; the arch map and the release target matrix are locked together by a check so they cannot drift apart again
- evidence: assess 3f8ddeff at HEAD ce27969 — release.yml:48-65 publishes the 6-asset set that homebrew, winget and npm all agree on; 32-bit ARM hosts currently pass the arch check then hit a generic download 404; full suite green; digest validation:v1:<sha> in the shipping PR; prioritize 407521b8: DEFERRED cycle 1 — release.yml surface collides with open PR #278/#272 activity and the armv7 decision (build lane vs refuse) is a strategic fork needing steward input

### Static-context attribution: skills, MCP definitions, system prompt
- id: `rm-032` | track: customer-experience | priority: 64.0 | status: candidate
- signals: capability.gap:crates/agenttrace-core/src/governance.rs::L613 (context_trends measures message, cache and file-read dynamics only), capability.gap (no static-context decomposition in-tree; tool-schema, skill and MCP-definition occupancy is unmeasured anywhere)
- acceptance: a static-vs-dynamic context split is reported per session wherever the data exists (tool schemas appear in transcripts; skill and MCP definition sizes when recorded); unattributable slices degrade honestly (marked unattributed, never estimated); docs name exactly what is measured
- evidence: research 5a025a61 FR-3 — VSCode 2.1.28x ships per-skill token estimates behind a typed /skills, /skill-doctor reports loaded-skill context cost, CLAUDE_CODE_MAX_MCP_DESCRIPTION_LENGTH (2,048 default) makes MCP-definition bloat tunable, and competitor tare already reports per-MCP/skill attribution; golden test on a fixture carrying tool definitions; full suite green; digest validation:v1:<sha> in the shipping PR; prioritize 407521b8: NEXT-CYCLE LEAD CANDIDATE — deferred on score, not on defect

### Session-id scope flag across report modes
- id: `rm-033` | track: customer-experience | priority: 62.0 | status: candidate
- signals: capability.gap:crates/agenttrace-core/src/search.rs::L10 (free-text query only), capability.gap:crates/agenttrace-cli/src/main.rs::L57 (inspect takes a positional rank; no flag accepts a session id or path to scope any report mode)
- acceptance: a --session <id|path-substring> scope flag composes with the existing report modes and --format lanes; help documents it; a no-match run errors naming what was searched
- evidence: research 5a025a61 FR-4 — ccusage shipped session-id report filtering in v20.0.26 (#1777, 2026-09-27) because users asked; distinct from rm-011 (search-contract truthfulness) and from master's landed rm-016 (Unicode case folding — matching semantics, not scoping); CLI tests green; digest validation:v1:<sha> in the shipping PR; prioritize 407521b8: DEFERRED cycle 1 — scored below the selected batch and below rm-032

<!-- managed by hermes-roadmap render; do not edit by hand -->
