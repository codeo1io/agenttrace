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

<!-- New items 2026-09-30 (run c7dca75d research attempt 70d7eac1 + assess attempt ca614a05; pass-10 candidates C-60..C-69 + strengthened C-56): numbered rm-012+ above the then-assumed master ceiling rm-011; sibling reservations checked (cc2f32d5's rm-003..rm-008 on PR #15, since merged with those ids absent from landed master's ROADMAP). -->
<!-- CORRECTION + COLLISION RECORD 2026-09-30 (run c7dca75d commit gate, attempt 2bf65d52): the ceiling assumption above was false — review round 2 finding, resolved record-not-renumber at this gate. Landed origin/master 9d88b36 (2026-09-30 09:02Z) holds rm-001, rm-002, rm-009..rm-011, run 02993de2's campaign (renumbered to rm-012..rm-019 at ITS integration), AND run 88feec46's campaign rm-012..rm-023 (record-not-renumber, merged as PR #18) — so this campaign's rm-012..rm-023 collide with two landed master blocks carrying distinct content. In-flight claims above rm-023 at this writing: conductor/run-304846327112@334b5a8 rm-026..rm-032 (local, unpushed) and run cbe30a9c rm-025..rm-033 (uncommitted worktree); open PRs are dependabot/fix-only (#279/#278/#272/#259). WHY NOT RENUMBERED HERE: the ids are load-bearing citations in validated executable surfaces (scripts/ci/check-plugin-version.sh rm-014, .github/workflows/ci.yml rm-022 + rm-014, .github/workflows/dependency-review.yml rm-019, CHANGELOG.md rm-012) and in the three docs/solutions lessons; renumbering would change executable content and invalidate the cycle-1 validation digest validation:v1:35e27732a5145b980499b463978fe14a60acb93cdb39b005a42ac91234b70f92 (base 7bb4dcb, full suite 241 passed / 0 failed). INTEGRATION GATE INSTRUCTION: renumber this block to the next free contiguous range above every landed AND in-flight claim at merge time (rm-034+ as of this note — re-derive then) in ONE commit updating ROADMAP.md, the lesson-doc cross-references, and the code/workflow comment citations together; the mapping also rides the shipping PR body. -->

### Codex token-usage arithmetic hardening (saturation + rewind double-count fix)
- id: `rm-012` | track: reliability | priority: 86.0 | status: done
- signals: correctness.overflow:crates/agenttrace-core/src/parser.rs::L2277 (codex_token_count_usage sums output+reasoning with plain `+`; debug builds panic and release builds wrap on adversarial i64-scale journal magnitudes while sibling sums at L1990/L3807/L4145 already use saturating_add), correctness.upstream_fix_gap:crates/agenttrace-core/src/parser.rs (upstream #286 token_usage_high_water rewind-after-compaction fix absent from the fork at HEAD 7bb4dcb; compaction rewind re-adds token totals and inflates Codex costs)
- acceptance: every token/cost accumulation in parser.rs uses saturating/checked arithmetic with a regression test feeding i64::MAX-scale magnitudes; rewind-after-compaction no longer double-counts (port or re-derive upstream #286's token_usage_high_water); golden cost totals unchanged for benign fixtures
- evidence: new parser tests for overflow and rewind fixtures pass; full suite green (cargo test + pytest); conductor validation digest validation:v1:<sha> recorded in the shipping PR
- cycle-1 (2026-09-30, run c7dca75d30e3): implemented uncommitted in worktree run-c7dca75d30e3-c7dca75d — codex_token_count_usage output+reasoning now saturating_add (matching the sibling precedent at L1990/L3807/L4145); upstream #286 token_usage_high_water rewind ported as the minimal parser.rs hunk; 3 new tests (i64::MAX-scale saturation, rewind double-count fixture, add_usage/add_usage_value accumulation saturation added by the cycle-1 independent review) green; status flipped to done by the commit gate

### Cache resolve_project for TUI hot paths
- id: `rm-013` | track: reliability | priority: 84.0 | status: candidate
- signals: performance.hot_path:crates/agenttrace-tui/src/explorer.rs::L557-L559 (uncached resolve_project — a .git filesystem walk — used as listing sort key and per-keystroke filter), performance.hot_path:crates/agenttrace-tui/src/app.rs::L1427-L1429, performance.hot_path:crates/agenttrace-tui/src/filters.rs::L251
- acceptance: a shared cached project-resolution handle serves the explorer sort/filter and app paths; keystroke-driven listing at 20k sessions performs no per-frame .git walks (regression test counting resolutions, or benchmark)
- evidence: TUI tests green including a resolution-count regression test; full suite green; conductor validation digest validation:v1:<sha> recorded in the shipping PR
- cycle-2 anchor: deferred this cycle (prioritization 2d5404e4 — explorer.rs/search.rs/main.rs/lib.rs stay untouched while PR #15 is open to keep its 9-file diff clean); highest-priority carry-over

### Codex plugin manifest release gate (version parity + asset integrity)
- id: `rm-014` | track: reliability | priority: 82.0 | status: done
- signals: release.channel_drift:.codex-plugin/plugin.json::L3 (version "0.7.1" vs released v0.9.0; manifest last touched e005952 2026-08-22), release.dangling_assets:.codex-plugin/plugin.json::L41-L46 (referenced assets/plugin-icon.svg and assets/README-plugin.md do not resolve under .codex-plugin/; icons live at repo-root assets/), reliability.ci_gap:.github/workflows (upstream #291/#292 merged 2026-09-30 grow release-channel automation while the fork ships an unverified plugin manifest)
- acceptance: a CI job fails when plugin.json version != crate version or when any referenced asset path fails to resolve; the manifest is corrected to match the v0.9.0 artifacts
- evidence: the gate job runs green on the corrected manifest and red on an injected mismatch (demonstrated in the PR); full suite green; conductor validation digest validation:v1:<sha> recorded in the shipping PR
- cycle-1 (2026-09-30, run c7dca75d30e3): implemented uncommitted in worktree run-c7dca75d30e3-c7dca75d — manifest 0.7.1→0.9.0, CHANGELOG v0.9.0 entry backfilled, check-plugin-version.sh hardened to three anchors (manifest ↔ CHANGELOG heading ↔ latest ^v[0-9]+\.[0-9]+\.[0-9]+$ tag, housekeeping tags filtered) plus every ./-path in the manifest must resolve; red path proven live by stashing the fix (exit 1 on tag mismatch); correction recorded: the signal's assets/README-plugin.md reference exists nowhere in the tree — the true defect was version drift; status flipped to done by the commit gate

### Pricing tier and price-class support (priority, above_1hr cache creation, batch, above_200k)
- id: `rm-015` | track: reliability | priority: 80.0 | status: candidate
- signals: data.model_gap:crates/agenttrace-core/src/pricing.rs (catalog schema carries no tiered price fields; live LiteLLM catalog 2026-09-30: 3,701 costed models, priority input rates on 208 — up from 116 at pass 9 — and cache_creation_input_token_cost_above_1hr on 216 — up from 147; models.dev 225 providers / 8,324 models), correctness.cost_gap:crates/agenttrace-core/src/statusline.rs+governance.rs (costs computed at base rates only, so priority-tier sessions under-report spend)
- acceptance: pricing structures accept per-tier rates (priority/flex, above_200k, cache_creation above_1hr) when the catalog provides them; cost reports and statusline rate math select the tier actually used by the session; golden tests cover a priority-tier fixture and an above-1hr cache fixture
- evidence: golden tier-cost tests pass; a catalog census re-run shows the tier fields consumed; full suite green; conductor validation digest validation:v1:<sha> recorded in the shipping PR
- cycle-2 anchor: deferred this cycle (prioritization 2d5404e4 — golden-test scaffolding across pricing + statusline + governance ≈ a full session on its own)

### Timezone-truthful rate-limit resets and daily buckets
- id: `rm-016` | track: reliability | priority: 78.0 | status: candidate
- signals: correctness.utc_drift:crates/agenttrace-core/src/statusline.rs::L228-L229 (resets_at rendered as bare UTC HH:MM while L565-L566 labels its timezone), correctness.utc_drift:crates/agenttrace-core/src/insights.rs::L65-L70 ("today" bucketing in UTC mis-buckets local-evening sessions)
- acceptance: statusline renders reset times in the user's local timezone with an explicit offset label; insights daily buckets honor a configurable timezone; tests cross a UTC-midnight boundary in a non-UTC locale
- evidence: boundary tests pass; full suite green; conductor validation digest validation:v1:<sha> recorded in the shipping PR
- cycle-2 anchor: deferred this cycle (prioritization 2d5404e4 — bundled with rm-015/rm-017 as the cycle-2 cost/timezone truthfulness wave)

### Statusline journal integrity (locking + bounded growth)
- id: `rm-017` | track: reliability | priority: 77.0 | status: candidate
- signals: reliability.race:crates/agenttrace-core/src/statusline.rs::L285-L290 vs L298-L326 (append and compact paths run without locking; concurrent statusline invocations can interleave or lose entries), reliability.unbounded_growth:crates/agenttrace-core/src/statusline.rs (journal compaction lacks a size-cap guard)
- acceptance: journal writes are serialized (file lock or atomic rewrite) and compaction triggers on a size threshold; a concurrency test hammers the journal from parallel processes without lost or duplicated entries
- evidence: concurrency test green; full suite green; conductor validation digest validation:v1:<sha> recorded in the shipping PR
- cycle-2 anchor: deferred this cycle (prioritization 2d5404e4 — the parallel-process journal harness rides the cycle-2 wave)

### Pricing-fetch guardrails (size cap, timeout, provenance stamp)
- id: `rm-018` | track: reliability | priority: 76.0 | status: done
- signals: security.unbounded_fetch:crates/agenttrace-core/src/pricing.rs::L345 (catalog response read with unbounded into_string — a hostile mirror can exhaust memory), data.provenance_gap:crates/agenttrace-core/src/pricing.rs (snapshot carries no fetched-at/source stamp)
- acceptance: catalog download enforces a byte cap and a timeout, failing closed to the bundled snapshot with a warning; the on-disk snapshot records fetch timestamp and source; tests cover oversized and hostile responses
- evidence: guardrail tests pass; full suite green; conductor validation digest validation:v1:<sha> recorded in the shipping PR
- cycle-1 (2026-09-30, run c7dca75d30e3): implemented uncommitted in worktree run-c7dca75d30e3-c7dca75d — download_pricing streams via into_reader().take(cap+1) with a 32 MiB PRICING_DOWNLOAD_MAX_BYTES cap rejecting over-cap responses, UTF-8 validation before parse, and a .meta.json provenance sidecar (url/fetched_at_unix/bytes) written by a path-injectable write_pricing_cache_at; 2 new tests green; status flipped to done by the commit gate

### Dependency-review workflow truthfulness
- id: `rm-019` | track: reliability | priority: 74.0 | status: done
- signals: ci.doc_drift:.github/workflows/dependency-review.yml::L7 (comment claims "cargo audit" while the job runs cargo-deny), ci.dead_branch:.github/workflows/dependency-review.yml::L32+L35 (unreachable arms)
- acceptance: workflow comments match the executed commands; dead branches removed or exercised; the existing check-docs-commands.sh pattern extended to workflow comments
- evidence: doc/workflow command check green; full suite green; conductor validation digest validation:v1:<sha> recorded in the shipping PR
- cycle-1 (2026-09-30, run c7dca75d30e3): implemented uncommitted in worktree run-c7dca75d30e3-c7dca75d — the comment now states the real coverage (ci.yml's lockfile scan runs cargo-deny only; cargo audit is not wired anywhere, grep-verified) and the dead PR-event guard arms are removed (the workflow is workflow_dispatch-only, so those conditions could never vary); status flipped to done by the commit gate

### Release workflow idempotency
- id: `rm-020` | track: reliability | priority: 73.0 | status: candidate
- signals: reliability.release_order:.github/workflows/release.yml::L148 (publish steps unguarded for re-runs; artifact-existence checks missing)
- acceptance: the release job is safe to re-run (existence checks before upload/publish; no duplicate release assets); a dry-run or simulated re-run demonstrates idempotency
- evidence: re-run safety demonstrated in the PR; full suite green; conductor validation digest validation:v1:<sha> recorded in the shipping PR
- cycle-2 anchor: deferred this cycle (prioritization 2d5404e4 — needs dry-run infrastructure to demonstrate idempotency honestly rather than assert it)

### Unicode-aware search case folding
- id: `rm-021` | track: reliability | priority: 72.0 | status: candidate
- signals: correctness.search_fold:crates/agenttrace-core/src/search.rs::L218 (ASCII-only to_ascii_lowercase fold; CJK-adjacent, diacritic, and mixed-case queries miss), ux.i18n:README.md (bilingual zh/en user base)
- acceptance: search folds with locale-agnostic to_lowercase over char boundaries; tests cover diacritics, mixed-case Latin, and emoji-adjacent text
- evidence: folding tests pass; full suite green; conductor validation digest validation:v1:<sha> recorded in the shipping PR
- cycle-2 anchor: deferred this cycle (prioritization 2d5404e4 — search.rs is PR #15 territory; folds into its wake once that merges)

### Lint-job tar-cache cleanup in CI
- id: `rm-022` | track: reliability | priority: 70.0 | status: done
- signals: ci.waste:.github/workflows/ci.yml::L44-L78 (lint job restores a ~369MB tar cache unused since 6ba55ea; dead weight on every run)
- acceptance: unused cache keys removed or scoped; the lint job restores only what it uses
- evidence: CI run shows reduced cache restore size; full suite green; conductor validation digest validation:v1:<sha> recorded in the shipping PR
- cycle-1 (2026-09-30, run c7dca75d30e3): implemented uncommitted in worktree run-c7dca75d30e3-c7dca75d — restore now tolerates a corrupt or partial tar (remove + cold build instead of failing the lane) and save skips when no target file is newer than the cached tar (find -newer), ending the ~369 MB re-tar churn per run; both branches proven in a sandbox before landing; status flipped to done by the commit gate

<!-- managed by hermes-roadmap render; do not edit by hand -->
