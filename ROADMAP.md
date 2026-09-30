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
- cycle-2 truth-correct (2026-09-30, run 30484632 assess, attempt f69de232): both cycle-1 closure caveats are now FALSE — the remote is PUBLIC (gh repo view codeo1io/agenttrace --json isPrivate -> false, live 2026-09-30) and the deny job HAS run remotely green twice (CI runs 36677489765 pull_request @ b898caa, 36678452544 push @ 7bb4dcb); the Scorecard deferral rationale is void, enablement landed as rm-026

### Report-contract truthfulness batch (search truncation disclosure, rates.total, unknown-reason labels)
- id: `rm-011` | track: reliability | priority: 81.0 | status: candidate
- signals: correctness.contract_gap:crates/agenttrace-core/src/search.rs::L203 (search JSON omits total_matches/truncated; per-session cap 8; --sort/--order unapplied on the search path, main.rs::L409-419), correctness.constant_field:crates/agenttrace-core/src/governance.rs::L97+L104 (rates_per_million_usd.total serialized as constant 0.0 while components.total is computed at L106; serialization sites L326/L434), correctness.fallback_mislabel:crates/agenttrace-tui/src/i18n.rs::L147-L155 (unknown inspect_reason values render as "ok"/"正常" while every sibling fallback says "unknown ...")
- acceptance: search JSON carries total_matches and truncated fields and either applies --sort/--order or exits with an explicit unsupported-flag error; rates_per_million_usd.total is computed from its components or the field is removed across all cost-audit serializations; inspect_reason fallback labels unknown reasons as unknown in every locale
- evidence: golden JSON tests covering search truncation disclosure, rates-total computation, and reason-fallback rendering; full suite green; conductor validation digest validation:v1:<sha> recorded in the shipping PR
- cycle-2 anchor: deliberately deferred this cycle (prioritization a818550b — three sub-defects × golden-test scaffolding ≈ a full session alone); untouched this cycle, anchors assess AF-1..AF-3

<!-- New items 2026-09-30 (run 30484632 assess f69de232 + research d22fc100). RENUMBERED at the commit gate (2026-09-30, run 30484632 commit e0e516be): minted as campaign-local rm-017..rm-023, which collided with sibling campaigns that landed/pushed while this cycle ran — origin/master ad503da carries rm-001..rm-019, PR #17 carries rm-017..rm-024, PR #18 carries rm-017..rm-025 and rm-033 (each with DIFFERENT content under those numbers). Mapping: rm-017→rm-026, rm-018→rm-027, rm-019→rm-028, rm-020→rm-029, rm-021→rm-030, rm-022→rm-031, rm-023→rm-032; all campaign artifacts (batch selection, stewardship request, cycle-8 record) citing rm-017..rm-023 mean these renumbered IDs. rm-026..rm-028 and rm-002 flip to done in this commit. NOTE: rm-002 (pytest for make-adversarial-sqlite.py) is also implemented by open PR #17's fe8b316 — content overlap to reconcile at integration. -->

### Truth-correct the private-remote claims and enable the Scorecard lane
- id: `rm-026` | track: reliability | priority: 86.0 | status: done
- signals: security.doc_false:deny.toml::L6 (header asserts the remote is private), security.doc_false:docs/solutions/security-issues/rustsec-advisory-shipped-via-ungated-lockfile.md::L55 (same claim), ROADMAP.md rm-010 cycle-1 note (corrected in place by the cycle-2 truth-correct above); live basis: gh repo view codeo1io/agenttrace --json isPrivate -> false (2026-09-30)
- acceptance: deny.toml:6 and the solutions doc's private-remote sentence corrected to the public truth; an OpenSSF Scorecard workflow (pinned action SHAs) runs and publishes results on the now-public repo; the rm-010 deferral note is the only historical record of the old rationale
- evidence: grep -rn 'private' deny.toml docs/solutions/ returns no false remote-visibility claims; a completed Scorecard run with published artifact; full suite green; conductor validation digest validation:v1:<sha> recorded in the shipping PR

### Pin the winget-create image in the release path
- id: `rm-027` | track: reliability | priority: 84.0 | status: done
- signals: security.unpinned_runner:.github/workflows/release.yml::L225-228 (docker run ghcr.io/microsoft/winget-create:latest — the only non-SHA-pinned runner in the token-bearing release lane); upstream fix available: luoyuctl/agenttrace master 3123f81 "submit WinGet with pinned wingetcreate.exe on Windows" (PR #292, merged 2026-09-30T05:21Z)
- acceptance: the WinGet submit step uses a digest-pinned image (or the pinned wingetcreate.exe path upstream adopted); no :latest image references remain in any workflow; release lane green on the next tagged run
- evidence: grep -n 'latest' .github/workflows/*.yml returns no container pulls; diff against upstream 3123f81 recorded; full suite green; conductor validation digest validation:v1:<sha> recorded in the shipping PR

### Enforce lockfile integrity across CI and release builds
- id: `rm-028` | track: reliability | priority: 82.0 | status: done
- signals: reliability.ungated_build:.github/workflows/release.yml::L34+L98 (cargo test / cargo build --release without --locked), reliability.ungated_build:.github/workflows/ci.yml::L64-108 (fmt/clippy/test/build, none --locked); grep -rn -- '--locked' .github/workflows/ -> no matches; assess AF-3 + research RC-12 converge: the rm-009/rm-010 reviewed-lockfile guarantee is convention-only — a drifted Cargo.toml silently regenerates the lock at build time
- acceptance: every cargo build/test invocation in ci.yml and release.yml carries --locked (fmt/clippy unaffected); a lockfile-drift guard fails when Cargo.toml edits are not accompanied by a committed lock refresh; workflows verified by a grep-based CI assertion or equivalent test
- evidence: grep for cargo build/test without --locked returns nothing; a deliberate drift experiment fails the guard; CI green; conductor validation digest validation:v1:<sha> recorded in the shipping PR

### Ship a share-safe report mode
- id: `rm-029` | track: customer-experience | priority: 78.0 | status: candidate
- signals: privacy.share_leak:PRIVACY.md::L14 (the product itself warns "Review reports before sharing them, because they can contain filenames, command names, model names, token counts, costs, and excerpts derived from local session logs" — the mitigation is manual); privacy.share_leak:crates/agenttrace-core/src/search.rs::L203 (evidence snippets embed session content into shareable output); research RC-11: no redaction capability on the 32-idea wall
- acceptance: a --share-safe flag (or equivalent mode) renders every output format (json/markdown/html/console) with deterministic redaction — paths hashed or basename-only, model/provider names optionally masked, no message/tool excerpts; PRIVACY.md's manual-review caveat is replaced by the mode's guarantee; golden tests pin redaction across all formats
- evidence: golden redaction tests for all four render paths; grep for content-bearing fields in --share-safe output fixtures returns nothing; full suite green; conductor validation digest validation:v1:<sha> recorded in the shipping PR

### Extend baseline gates to quality metrics
- id: `rm-030` | track: reliability | priority: 70.0 | status: candidate
- signals: reliability.gate_gap:crates/agenttrace/src/main.rs::L96-101 (--baseline-max-cost/duration/token-delta-pct exist for cost-class metrics only; loop-rate, waste share, and health-score regressions are ungateable); research RC-13: composes with, not duplicates, 5th-pass idea #7 (shareable baseline config) and RE-5 (absolute-threshold CI gate)
- acceptance: baseline gates accept waste/loop/health metrics with stored baselines and delta-percent flags alongside the cost family; a regression vs stored baseline exits nonzero with a named metric; golden tests cover one passing and one regressed baseline per metric class
- evidence: CLI help shows the extended flag family; golden gate tests green; full suite green; conductor validation digest validation:v1:<sha> recorded in the shipping PR

### Portable redacted session bundles (export/import)
- id: `rm-031` | track: customer-experience | priority: 65.0 | status: candidate
- signals: capability.missing:crates/agenttrace/src/main.rs (no export/import surface — grep for export subcommands returns nothing); research RC-15: a single-file redacted bundle unblocks 5th-pass idea #5's format canary (blocked on donated samples), CI reproduction, and archival; rides rm-029's redaction primitive
- acceptance: `agenttrace export` writes a single-file bundle (sessions + pricing snapshot + manifest, compressed) with redaction applied, and `import`/analysis consumes it without the original local stores; round-trip test proves bundle-in equals store-derived output; a canary fixture generated from a real bundle passes the format tracker
- evidence: round-trip + redaction golden tests; a canary run on a generated fixture; full suite green; conductor validation digest validation:v1:<sha> recorded in the shipping PR

### Open the agenttrace-core library lane
- id: `rm-032` | track: developer-experience | priority: 55.0 | status: candidate
- signals: capability.closed:crates/agenttrace-core/Cargo.toml::L13 (publish = false); crates.io fetch 2026-09-30: no agenttrace-core crate registered (name free); research RC-14: the parser corpus is the fork's moat, pass-5 defensible-center analysis (diagnosis depth) supports broadcasting it as an embeddable library
- acceptance: publish = false dropped (or scoped to a stable subset crate); semver + API-stability policy documented in CONTRIBUTING; docs.rs renders the crate; the public API surface carries no TUI/runtime dependencies
- evidence: published crate resolves on crates.io; docs.rs build green; a downstream smoke consumer builds against the published API; full suite green; conductor validation digest validation:v1:<sha> recorded in the shipping PR

<!-- managed by hermes-roadmap render; do not edit by hand -->
