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

<!-- rm-017..rm-024 appended by run 3099db98 roadmap phase (campaign 4a20d61e cycle 1, attempt 69d23ead) at HEAD 90a4ef5. IDs start past the highest sibling-worktree allocation (02993de2 rm-009..016, cc2f32d5 rm-003..008); rm-IDs are campaign-local until merge and ID reconciliation is the commit phase's concern. Sources: this run's assess phase (df972c9e, AR2-1..AR2-4) and research phase (183a4126, PE-1..PE-6, pass 8). -->

### Price multi-model sessions per model

- id: rm-017
- track: correctness
- priority: high
- status: candidate
- signals: correctness.single_model_pricing:crates/agenttrace-core/src/lib.rs::L510, correctness.discarded_model_data:crates/agenttrace-core/src/lib.rs::L147, correctness.misleading_provenance:crates/agenttrace-core/src/lib.rs::L770
- acceptance: Analysis accumulates a per-model token ledger keyed on per-event model_used; session cost is the sum over models of lookup_price(model) x that model's tokens; report --overview, JSON output, and the TUI session detail view render a per-model breakdown (model, tokens in/out, rate source, cost); provenance.cost becomes a per-model enumeration or an explicit mixed-model marker; a golden test pins a two-model fixture's expected per-model costs; the sqlite_sessions.rs multi-model acknowledgment test is updated to assert the new exact behavior
- evidence: Anchors re-verified at HEAD 90a4ef5: lib.rs:509-521 retains one model string per session (per-event model_used captured at :147/:344 is discarded at pricing time), lib.rs:596-601 does one lookup_price, lib.rs:763-772 applies that single price to all tokens and stamps provenance.cost=calculated_from_tokens at :770; sqlite_sessions.rs:857 test admits the analogous imprecision on the SQLite path. Research PE-1 (pass 8, 92% confidence): no competitor prices multi-model sessions per-model (ccusage 18,811 stars, tokenmaxxing both apply one rate), making this both the residual correctness defect (assess AR2-1) and a differentiating capability
- next-cycle anchor (compound c1): top unselected alternate — deferred value-based on merge contention (render surface overlaps 02993de2's staged report_overview_* edits), not on value; becomes the top cycle-2 anchor once that merge lands, with rm-018 riding its per-model ledger

### Stamp pricing provenance into every artifact

- id: rm-018
- track: data-freshness
- priority: medium
- status: candidate
- signals: data_freshness.unattributed_pricing:crates/agenttrace-core/src/pricing.rs::L98, data_freshness.unconsumed_accessors:bundled_snapshot_date/model_count have zero artifact consumers
- acceptance: Every report footer, JSON output, and TUI about/overview surface embeds the pricing snapshot date, model count, catalog content hash, and the override-file fingerprint when AGENTTRACE_PRICING_FILE is active; two artifacts priced against different snapshots are distinguishable without repo access; JSON changes are additive only; a test pins the stamp format
- evidence: pricing.rs:98-105 exposes bundled_snapshot_date() and bundled_snapshot_model_count() with zero artifact consumers at HEAD 90a4ef5 (grep-verified research pass 8; the accessors were added for cycle 7 R1 disclosure and never rendered); research PE-3 (88% confidence, low complexity); compounds with rm-017's rate-source column and with sibling campaign rm-006's doctor drift surfacing (that item surfaces at read time, this one pins into the artifact)

### Sign and attest release artifacts

- id: rm-019
- track: security
- priority: medium
- status: candidate
- signals: supply_chain.hash_only:.github/workflows/release.yml::L103, supply_chain.upstream_hardening:luoyuctl/agenttrace #287-#290 + 30-release channel on 2026-09-30
- acceptance: Release tarballs and installers are cosign-signed; the build workflow emits SLSA provenance attestations; an SBOM ships with each release (cargo-auditable or syft); install.sh and npm/scripts/install.js verify a signature rather than only the sidecar hash; README documents verification including a Windows path; verification failure refuses install
- evidence: release.yml emits SHA-256 sidecars only (sha256sum at :103-106, checksums.txt at :140) with no signing or attestation lane at HEAD 90a4ef5; upstream landed #287 (CodeRabbit+Codecov), #288 (cargo-deny+Scorecard+rustls advisory patch), #289 (git-cliff), #290 (badges) and grew a 30-release/tag channel between same-day probes 03:40Z-06:1xZ on 2026-09-30 (gh api, research pass 8); research PE-2 (80% confidence). Complementary to - not duplicating - the sibling campaign's in-flight cargo-deny/Scorecard batch: that gates dependencies, this attests artifacts

### Materialize an incremental explorer index

- id: rm-020
- track: performance
- priority: medium
- status: candidate
- signals: performance.per_frame_rebuild:crates/agenttrace-tui/src/explorer.rs::L544, performance.duplicate_resolution:Projects comparator resolves each project twice
- acceptance: The explorer's session index (filtered/sorted views, project resolution) is materialized into a SQLite sidecar keyed by session path + mtime; TUI launch appends/updates incrementally instead of enumerating and parsing every file; explorer_indices() remains the in-memory shape over indexed rows; comparator project resolutions are cached; the schema carries a version field for migration; full suite green
- evidence: explorer.rs:544-601 rebuilds filtered+retained+sorted indices per call across ~5 call sites per rendered frame and the Projects comparator calls resolve_project() twice per comparison at HEAD 90a4ef5 (assess AR2-2); rusqlite is already a workspace dependency; research PE-4 (75% confidence, medium-high complexity). This is the architectural fix beneath the per-frame waste and the substrate rm-022 needs; it supersedes the narrower dead-recompute-removal idea on the sibling e7206fe5 roadmap (rm-012 there) - reconcile at merge
- next-cycle anchor (compound c1): deferred on lane contention (explorer.rs claimed by cc2f32d5's presentation/explorer work); also carries the rm-012 supersession decision vs the sibling e7206fe5 roadmap — settle both after the merge landscape clears; re-verify explorer.rs anchors at the cycle-2 tree before planning

### Ship an `agenttrace upstream status` subcommand

- id: rm-021
- track: upstream-sync
- priority: medium
- status: done
- signals: upstream_sync.manual_drift_rederivation:fork maintenance loop re-derives drift by hand each cycle, upstream_sync.channel_volatility:+4 commits and 30 releases in one day; npm package name moved once
- acceptance: `agenttrace upstream status` compares the running build against upstream main, the GitHub release channel, and the npm package - commits ahead/behind, PR-level delta grouped by area (parser/diagnostics/CI), advisory drift, distribution-channel state; offline by default with an explicit fetch flag; output schema documented and stable for scripting
- evidence: Upstream master moved be25c4c9 -> e54831e (#287-#290) and gained 30 releases/tags between same-day probes 03:40Z-06:1xZ on 2026-09-30; npm @zack78/agenttrace stabilized at 0.9.0 after unpublish churn while bare `agenttrace` 404s (all fetched live, research pass 8); the fork's own maintenance loop re-derives this drift by hand each cycle via internal-only scripts; research PE-6 (72% confidence, low-medium complexity)
- compound c1 2026-09-30: implemented in the campaign worktree (uncommitted, pre-review) as `agenttrace upstream` — crates/agenttrace-cli/src/upstream.rs, offline-by-default (local remote-tracking refs) + explicit `--fetch` (git fetch + npm probe), `-f json` append-only schema documented in docs/guides/upstream-status.md; 6 unit + 9 hermetic integration tests; live-verified offline (ahead 37 / behind 8 / 8 unported #285-#292 / 124 diverged files) and via --fetch (npm 0.9.0); recorded outcomes: targeted 5 passed, full suite 251/0. Solution doc: docs/solutions/workflow-issues/fork-upstream-drift-status-subcommand.md. Flipped to done at the commit gate 2026-09-30: independent review APPROVED (five-lens, zero fixes), full workspace 251/0, targeted 5 passed, digest validation:v1:bde0b51bf06c9c706fef41075750 at base 90a4ef5, shipped in this commit

### Watch mode on session files

- id: rm-022
- track: customer-experience
- priority: medium
- status: candidate
- signals: capability_gap.no_live_tail:ccusage monitor mode has no fork equivalent, existing_machinery:TUI already re-scans and re-renders on change inside a full launch
- acceptance: `agenttrace watch [paths]` tails session files on append, re-analyzes only the changed file incrementally, and re-renders the live overview/sessions view (cost so far, waste signals, loop detection); implementation stays file-tail re-analysis of written events and never stream interception, honoring the recorded non-goal; cost stays linear in changed files via rm-020's index or a per-file cache; the notify dependency is evaluated against the static-build profile
- evidence: ccusage's monitor mode is that project's flagship (org repo, 18,811 stars, pushed 2026-09-30T02:50Z); the fork's differentiating diagnostics (waste/loop/governance) have no live-mode equivalent anywhere; the TUI already re-scans and re-renders on change, so the machinery exists but only inside a full-launch directory sweep; research PE-5 (70% confidence, high complexity; scoped to file-tail per the ROADMAP non-goal - live tracing while a model is streaming - as the repeat-check in pass 8 established)

### Exclude untimestamped sessions from trend windows

- id: rm-023
- track: correctness
- priority: low
- status: candidate
- signals: correctness.trend_pollution:crates/agenttrace-core/src/reports.rs::L2075, correctness.unguarded_none:reports path lacks the !session_start.is_empty() guard the explorer path has
- acceptance: Sessions with no parseable timestamp are excluded from health-trend window bucketing (or bucketed into an explicit unknown bucket); direction and regression verdicts no longer change when untimestamped sessions are present; a golden test pins a mixed fixture with and without timestamps
- evidence: reports.rs:2071-2076 sorts (None, Some) => Ordering::Greater, placing untimestamped sessions after all timestamped ones - into the newest trend window; analyze() leaves session_start default-empty when no event timestamp parses (lib.rs) and parse_rfc3339 then yields None; the explorer path guards !session_start.is_empty() for its comparison (explorer.rs) while the reports path does not; assess AR2-3 (attempt df972c9e), anchors re-verified at HEAD 90a4ef5

### Use display-width math for report column budgets

- id: rm-024
- track: correctness
- priority: low
- status: candidate
- signals: correctness.rune_width:crates/agenttrace-core/src/reports.rs::L2447, inconsistency:reports count chars while the TUI uses unicode-width
- acceptance: Width-aware truncation and padding (unicode-width) for label and name columns in text reports; CJK content renders within the budgeted width; parity with the TUI's existing unicode-width approach; golden tests pin CJK-label fixtures in the affected columns
- evidence: reports.rs:2447-2461 truncate_text_runes counts chars (value.chars().count()) not display cells, so CJK labels/names render up to 2x the budgeted width; the TUI already measures with unicode_width::UnicodeWidthStr (filters.rs:379/:411, dependency unicode-width 0.2) - the two rendering surfaces disagree; zh translations ship in the i18n files, making this reachable in normal use; assess AR2-4 (attempt df972c9e), anchors re-verified at HEAD 90a4ef5

<!-- managed by hermes-roadmap render; do not edit by hand -->
