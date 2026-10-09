# Cycle 1 implementation record — run 84c45ccdbbeb (attempt 109d3b4f, 2026-10-05)

Base e1d31c4 (worktree `run-84c45ccdbbeb-84c45ccd`). Batch selected at prioritize
(`delegate/622579db0c5946099327f18c2a70a4f4-prioritize.md`), contracted at stewardship
(`delegate/076b8b0370164715add58bc3520c2116.json`): **Provable pricing-catalog freshness** —
rm-176 LEAD + assess-A1/rm-006 desync rider + rm-513 posture rider. No Rust code touched; two
new files plus append-only bookkeeping.

## Changed surfaces (for the validation fold)

| surface | change |
|---|---|
| `scripts/pricing/drift-check.sh` | NEW (executable-classified: bash + python3; shellcheck-clean) |
| `.github/workflows/pricing-drift.yml` | NEW (weekly cron + dispatch; ruby-YAML-validated) |
| `ROADMAP.md` | rm-176 `candidate → implemented` + cycle-1 evidence line; rm-006 implemented fold (status stays candidate for the --overview arm); rm-513 posture fold — 3 line-edits + 3 appended lines, append-only apart from the one status flip |
| `CHANGELOG.md` | one `### Added` bullet under `[Unreleased]` |
| `docs/stewardship/2026-10-05-cycle1-implementation-record-run84c45ccd.md` | NEW (this record) |

Untouched by design (read-only surfaces): `crates/agenttrace-core/src/pricing.rs`, 
`crates/agenttrace-core/src/pricing_snapshot.json`, `scripts/pricing/update-snapshot.sh`, 
`.github/workflows/ci.yml`, every core/TUI source (all held by sibling lanes or the canonical
mid-merge e1e2c7e).

## What landed

### LEAD rm-176 — automation core

`scripts/pricing/drift-check.sh` — drift between live LiteLLM
(`https://raw.githubusercontent.com/BerriAI/litellm/main/model_prices_and_context_window.json`,
same source `update-snapshot.sh` fetches) and the bundled `pricing_snapshot.json`,
**strictly on the builder keep-filter basis** (`mode=="chat"` AND `input_cost_per_token>0` OR
`output_cost_per_token>0`) and **only on the fields the bundle pins** (input/output per-token
costs, `cache_creation_input_token_cost`, `cache_read_input_token_cost`, `litellm_provider`,
`max_input_tokens` when int>0, `deprecation_date`). Classes: `new_costed`, `retired`
(sets `removal_guard`), `rate_changed` (4 cost fields), `deprecation_new` (actionable) and
`context_window_changed`/`provider_changed` (informational). Exit contract: 0 clean, 1 actionable
drift / desync / removal, 2 infra. `--json` machine report; `--live <file>` offline mode;
`--selftest` four embedded offline arms; locates the const by grep (survives #313 root-slim).

`.github/workflows/pricing-drift.yml` — Mondays 06:00 UTC + `workflow_dispatch`;
`contents: write`, `pull-requests: write`. rc 0 → green summary. `removal_guard` or `desync`
→ `::error::` and job failure — never an auto-PR (additive-growth guard: no silent price
removals). Actionable drift → regenerate via `update-snapshot.sh`, bump the
`PRICING_SNAPSHOT_DATE` const to the stamped date, **re-run the gate and require rc 0** before
pushing `chore/pricing-snapshot-<date>` and opening the PR (drift report + post-regeneration
report + counts in the body; falls back to a warning if PR creation lacks permissions).
ci.yml untouched.

### RIDER A — assess A1 + rm-006

Desync gate inside drift-check: `pricing.rs` const vs `_snapshot.date` must match; mismatch =
rc 1 with a `DESYNC (assess A1)` line. The manual "then update PRICING_SNAPSHOT_DATE" step
`update-snapshot.sh:3-5` documents is now enforced fail-closed. rm-006's count-ledger duty is
automated: bundle-vs-live costed-chat counts in every run output and PR body.

### RIDER B — rm-513 arm-(1) posture (the three-postures table)

| posture | mechanism | verdict | why |
|---|---|---|---|
| **A. Bundled + scheduled PR-bump (chosen)** | offline-first bundle; weekly drift gate opens reviewable refresh PRs | **KEEP** | byte-deterministic reports; doctor census/vintage disclosure depends on the bundle; every money-table change is reviewed; no hot-path network |
| B. Upstream runtime-fetch | LiteLLM raw URL + 24h `CACHE_MAX_AGE` + `OnceLock` catalog (`upstream pricing.rs`, 1,187 lines) | REJECT for the merge | silent stale-cost windows up to 24h; report/doctor output stops being byte-stable; the fork already has runtime refresh as the opt-in `--update-pricing` cache layer — hybridizing architectures doubles the merge hazard rm-513 exists to reduce |
| C. ccusage-style hourly auto-push | hourly cron commits to the default branch | REJECT | unreviewed mutation of the money table; hourly churn (~0.05%/day LiteLLM drift makes weekly adequately sized — recorded rm-176 refresh 2026-10-03) |

**What would change the verdict:** posture A loses if the fork adopts session-aware pricing
overrides that must reflect live rates within hours (not currently on the wall); posture B gains
only if upstream's runtime path becomes load-bearing for features the fork ports wholesale
(watch the wave merge — if #307's pricing.rs internals are ported, revisit). models.dev as a
second source composes with A (LiteLLM primary; models.dev fills `cache_read`/`cache_write` and
`limit.context`/`limit.output` gaps with documented precedence) — designed, not wired this cycle.

**Merge instruction carried from this decision:** reject upstream `pricing.rs` wholesale at the
wave merge; keep fork side; `drift-check.sh` already tolerates the #313 renames.

## Verification matrix (all executed 2026-10-05, evidence on this host)

| check | command | result |
|---|---|---|
| syntax + lint | `bash -n`; `shellcheck scripts/pricing/drift-check.sh` | clean |
| selftest | `scripts/pricing/drift-check.sh --selftest` | 4 arms + control PASS, rc 0 |
| real parity-by-counts | `--live /tmp/at-research-caa1350a/litellm.json --json` | 3099/3099 kept; **true drift found**: rate_changed=11 (openrouter deepseek/qwen/kimi/llama), deprecation_new=2 (azure_ai jamba-instruct, kimi-k2-thinking), context=4 informational, retired=0, guard clear, desync false → rc 1 (correct: the 2026-10-04 bundle is genuinely behind) |
| red arm R1 removal | live copy minus `gpt-4o` | rc 1, `retired: 1`, `removal_guard: true`, model named |
| red arm R2 new-costed | live copy plus `fake/new-model` | rc 1, `new_costed: 1` |
| red arm R3 infra | `--pricing-rs` nonexistent | rc 2, clear infra message |
| red arm R4 desync | const forced `2026-01-01` | rc 1, `DESYNC (assess A1)` line |
| workflow YAML | `ruby -ryaml` parse + 10-point structural assertions | valid; single action `actions/checkout@3d3c42e5aac5ba805825da76410c181273ba90b1` (targeted_tests run a2d985b2: pinned from `@v4` to the same full SHA as ci.yml/release.yml — repo convention per rm-405 discipline); no ci.yml reference |
| phantom-diff diagnosis | bundle vs live field census | builder pins no `max_output_tokens` → first draft's 2,802 "context changes" were comparison artifacts; fixed to pinned-fields-only |
| non-interference | porcelain + `git diff --stat` | only the 5 intended surfaces; zero Rust/core files |

## Boundaries and honest disclosures

- rm-176 stays `implemented` with two arms explicitly deferred: models.dev wiring (designed,
  posture table above) and the first automated green run (happens on the GitHub scheduler after
  this lands; the local matrix above is the cycle evidence).
- rm-006 stays `candidate`: its `--overview` pricing-confidence arm sits on the reports surface
  held by d6432dd5's unlanded batch.
- The bundle was NOT regenerated this cycle (out of scope by contract; the gate's true positive
  is left for the scheduler's first post-merge run — by design, not omission).
- PRs authored by the default GITHUB_TOKEN do not trigger CI (GitHub limitation); disclosed in
  the workflow header and PR body, with the gate's own post-regeneration verification as the
  review artifact.
- Known pre-existing fact surfaced (not fixed here, pricing.rs is contested): the builder drops
  `max_output_tokens` from the bundle, so the fork's context-window disclosures are
  input-window-only. Recorded for the pricing.rs owners (4a688257 / wave merge).

## Review-fix addendum (2026-10-06, independent_review:fix attempt 825ca17f)

The adversarial review (attempt 6d90c387, verdict NEEDS_CHANGES: 2 medium + 3 low) found
the core sound but shipped five defects; all five are fixed. Deltas against THIS record:

- The selftest is now **six** arms, not four — this record's "four embedded offline arms" rows
  describe the pre-fix script. Arm 5 pins fail-closed-on-absent-inputs (rc 2), arm 6 pins the
  declaration-anchored const read.
- The desync pair is **fail-closed on absence** (rc 2), not merely on mismatch: a pricing.rs
  without a parseable `const PRICING_SNAPSHOT_DATE: &str = "<date>"` declaration or a bundle
  without `_snapshot.date` is an infra failure, never "clean" (review F1 — pre-fix red arms
  proved green rc 0 with `<none>`, the exact hole a half-applied wave merge would leave open
  forever).
- The const date is read from the **declaration** (the same shape the workflow's const-bump sed
  targets), immune to dated comments that merely mention the token (review F3 — pre-fix, a
  dated `///` comment above the const produced a false DESYNC).
- `update-snapshot.sh` is no longer untouched: it takes an OPTIONAL pre-fetched-catalog
  argument (default fetch behavior unchanged, byte-identical output proven) so the pricing-drift
  job fetches live exactly ONCE for gate + regenerate + verify (review F4 TOCTOU); the PR branch
  is now `chore/pricing-snapshot-<date>-<run-id>.<attempt>` (review F5 same-day collision).
- `CHANGELOG.md` carried a duplicated content line introduced by the full_tests marker-block
  append; deduped (review F2). The marker block itself is untouched and check-plugin-version
  stays rc 0.

Revalidation at targeted scope (dispatch-authoritative): bash -n + shellcheck clean on both
scripts; `--selftest` rc 0, all six arms; the review's three red arms standalone (missing const
→ rc 2, missing meta → rc 2, dated comment → date read 2026-10-04, no DESYNC); live gate vs a
fresh 2026-10-06 fetch rc 1 true positive identical in shape to the review's own run (3099
bundled / 3101 live, new_costed 2 `*.zai.glm-5.3`, rate_changed 11, deprecation_new 2, guard
clear); sandboxed regen byte-deterministic (3101 @ 2026-10-06) with post-bump verify rc 0 on
the same single fetch. Emission digest moved to
`validation:v1:68faba1d0cbfaf375287d715d43089405a4ed1b6caf7b5c46abff71e48340cdf`
(update-snapshot.sh is now a changed testable surface). Full fix dossier:
`docs/stewardship/2026-10-06-cycle1-compound-record-run84c45ccd.md` § Review fix.

Second-round addendum (2026-10-06, independent_review cac54820 NEEDS_CHANGES 1M+3L → a53b451b,
all four resolved): the nonexistent `pricing` label was dropped from `gh pr create` (label
absent from the repo — deterministic first-run PR failure); `--json` stdout is now PURE JSON
(summary → stderr under `--json`; no-`--json` stdout unchanged, so the workflow's regen
counts-grep is untouched); malformed catalog shapes (list-typed costs, `_snapshot` as a
string) fail closed at rc 2 with EMPTY stdout — parse failures are never actionable drift —
pinned by selftest arm 7 (seven arms now); the CHANGELOG's stale "four arms" corrected.
Emission digest moves to
`validation:v1:395fef176f5805c15e5824b128c2e0ed08ca448bd9ebe3eb4679deca4b2b159c`.
Full dossier: `docs/stewardship/2026-10-06-cycle1-compound-record-run84c45ccd.md` § Review fix 2.
