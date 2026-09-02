---
type: stewardship-record
id: cycle6-implementation
cycle: 6
batch: coverage-cost-and-capability-tell-the-truth
date: 2026-09-03
base_commit: 696206f
record_kind: ce-handoff/v1
status: implemented-uncommitted
---

# Cycle 6 implementation record — CU-17..CU-22

Executed in the working tree on top of HEAD `696206f` (local master, 4
commits ahead of origin/master; tracking repointed to the `fork` remote as
part of CU-17 — environment state only, never committed). Nothing
committed, pushed, or PR'd; the Conductor owns topology per the
stewardship contract. The ce-* compound-engineering router is not
installed in this environment (only `agent-reach`); phases were executed
directly, as disclosed in every cycle-6 artifact so far.

## Verification summary (re-run after the last code edit)

- `cargo test --workspace`: **212 passed, 0 failed** (baseline 203; +9 new
  tests, all in-tree, listed per CU below).
- `cargo fmt --check`: clean.
- `cargo clippy -p agenttrace-core -p agenttrace-tui -p agenttrace
  -- -D warnings` (the CI invocation): clean.
- `scripts/ci/check-no-self-hosted.sh`: exit 0 clean, exit 1 with
  file:line output when re-contaminated (both polarities re-verified after
  the guard was wired into ci.yml).
- `--doctor` (live corpus): 1416 session files, 722 cache entries,
  cache size 9,342,161 bytes, and the new disclosure line
  `cache size 9342161 bytes, hard bounds: entries<=20000, bytes<=67108864
  (oldest-source entries evicted first)`. Doctor directory list now shows
  `Gemini CLI`, `Antigravity CLI`, `Antigravity CLI conversations`.
- `--demo --overview -f json`: 3 sessions, cost audit intact.
- Fixture audit (`testdata/gemini-thoughts-checkpoint.json`):
  `tokens {"input":80,"output":60,"reasoning":40,...,"total":140}`,
  `reasoning_share_pct 66.6667`, `pricing_status catalog_estimate`,
  `estimated_cost_usd 0.0002`.
- CI-equivalent guard scripts on the release binary, all exit 0:
  `check-rust-real-cli-smoke.sh` (testdata corpus, 20 sampled files),
  `check-docs-commands.sh` (including the pinned schema-17 claims in the
  governance guide), `check-output-contract.sh` (doctor.json valid JSON
  with the new fields), `check-deterministic-output.sh`,
  `check-report-semantics.sh`, `check-release-surfaces.sh`.

## What shipped

### CU-17 — upstream-portable CI (CRITICAL, lead)

- All five `runs-on: self-hosted` occurrences reverted to
  `ubuntu-latest`: ci.yml:21, dependency-review.yml:16, release.yml:13,
  release.yml:40, release.yml:117 (commit 6632014 contamination; the
  hazard was proven live by upstream PR #282 being closed unmerged for
  exactly this).
- New guard `scripts/ci/check-no-self-hosted.sh` (grep
  `runs-on:.*self-hosted` across `.github/workflows/`, file:line output on
  failure). The pattern is narrow enough that the guard's own ci.yml step
  (`scripts/ci/check-no-self-hosted.sh`) cannot self-trigger.
- Wired into CI as a named step (`Portable CI runners only`) right after
  `cargo fmt --check`, and covered by the existing `bash -n scripts/ci/*.sh`
  syntax sweep.
- Local tracking repointed so plain `git push` targets the fork, not
  upstream: `git config branch.master.remote fork`
  (fork = codeo1io/agenttrace; origin = luoyuctl/agenttrace). This is
  local-environment state per AGENTS.md rule 3 — it must never be
  committed; `must_remain_separate` rule 1 of the stewardship request.

### CU-18 — Gemini CLI `~/.gemini/tmp` discovery root

- `known_session_dirs()` (discovery.rs:51) gains the `Gemini CLI` root at
  `~/.gemini/tmp`, restoring the README's "reads … Gemini CLI …" claim
  (README.md:31) as a live capability; chats/checkpoints gating already
  existed (`is_gemini_temp_session_file`, `max_session_dir_depth`).
- Contract tests: synthetic `$HOME/.gemini/tmp/sess-1/chats/chat.json` and
  `.../checkpoints/cp.json` are discovered, parse as `gemini_cli`, and a
  `thoughtsTokenCount` in `tokenUsage` reaches the CU-20 breakdown
  (`tokens_output` 60, `tokens_reasoning` 40).
- New fixture `testdata/gemini-thoughts-checkpoint.json` (Gemini CLI
  checkpoint shape with `thoughtsTokenCount`) pins the end-to-end path for
  assess/review phases to point at.

### CU-19 — Antigravity conversations root (conditioned; met)

- `known_session_dirs()` gains `Antigravity CLI conversations` at
  `~/.gemini/antigravity-cli/conversations`.
- **Resolved open question — the store is not JSON.** Research against
  gstack #1977 and the agy-reader reference implementation (clone at
  /tmp/agy-check, `internal/daemon/types.go`; verified against agy
  1.1.23): conversations live in an undocumented SQLite `.db`
  (`user_version=1`, 7 tables) with protobuf `.pb` blobs; the JSON
  `<uuid>.trajectory.json` sidecars in the same directory are the
  documented reader surface (real JSON, agenttrace-parseable, steps carry
  no usage fields).
- New parser `parse_antigravity_trajectory` (parser.rs, dispatched in the
  single-object branch of `parse_raw_session`): sniffs
  `CORTEX_STEP_TYPE_*` + `metadata.createdAt` (both required — a bare
  `steps` array is not evidence), maps USER_INPUT → user event,
  PLANNER_RESPONSE → assistant event with `thinking` → reasoning and
  `toolCalls[].argumentsJson` parsed into tool-call args,
  RUN_COMMAND → tool event (non-zero `exitCode` → failed call),
  ERROR_MESSAGE → error tool event, VIEW_FILE → tool event with
  line range. INVOKE_SUBAGENT children live in their own trajectory
  files; SYSTEM_MESSAGE/CHECKPOINT/GENERIC carry no transcript content.
- Admission stays JSON-only: `.db`/`.pb` are rejected by the existing
  `is_session_file_name` extension filter, and the contract test asserts
  both the sidecar parses (`source_tool=antigravity_cli`,
  reasoning_blocks=1) and the store/blobs never enter discovery.
- The acceptance condition held: shape documented from the official
  reader implementation, no fabrication of a SQLite parser. **Residual
  filed, not faked**: full `.db`/`.pb` store support remains future work
  (needs schema reverse-engineering plus a real corpus; see
  provisional_future_work).

### CU-20 — thinking tokens billed as output, reported as reasoning

- All three usage sites fold thinking aliases
  (`thoughtsTokenCount`, `thinkingTokenCount`, `thinking_tokens`,
  `reasoning_tokens`) into the output count and insert a
  `reasoning_tokens` breakdown key: `qwen_usage` (parser.rs),
  `gemini_usage` (parser.rs:3718 area), and the table-driven
  `usage_from_value` (parser.rs:4072 area). Zero thinking → no key, so
  legacy corpora keep byte-identical usage maps.
- `Metrics` gains `tokens_reasoning: i64` (lib.rs) accumulated
  saturating from event usage, mirrored through the session cache
  (`GoMetrics.TokensReasoning`, `serde(default)` so v17 caches without
  the field deserialize to 0 — additive field, no semantics inversion,
  schema stays 17; the docs-commands guard's pinned `session cache is
  schema 17` claim stays true).
- Governance: `TokenBreakdown.reasoning` accumulates per (provider,
  model) row and per session audit; `ModelCostAudit.reasoning_share_pct`
  (Option; None when no thinking reported) exposes the share of billed
  output tokens. Cost math unchanged beyond the fold: folding into
  output_tokens means pricing already bills thinking at the output rate —
  which is the truth the API documents (Gemini: candidatesTokenCount
  excludes thoughts).
- `--audit` JSON gains `tokens.reasoning` and `reasoning_share_pct`
  automatically (cost_audit is serialized directly); the
  governance-reports guide now documents the fold and the share.
- Tests: per-site unit test (all three functions), end-to-end checkpoint
  metrics test, audit row/share test, cache round-trip + legacy-JSON test,
  discovery assertion in the CU-18 test.

### CU-21 — `--sample` names the active view

- Both disclosure strings (main.rs:251 and :294, governance-class and
  `--compare`) now read
  `sampled first {N} of {total} sessions in the --sort {sort} --order
  {order} view (--sample {N})` — the default is `--sort recent --order
  desc`, so default output still says "recent … desc", and the old
  "sampled newest" claim is gone everywhere it could be a lie
  (e.g. `--sort cost --order asc`).
- Guide updated: "audits the first N sessions of the active
  `--sort`/`--order` view (newest-first by default) and names that view in
  the exclusion reason".
- Test: the entrypoints sampling test now asserts the reason names both
  `--sort cost` and `--order asc` under a non-default view.

### CU-22 — cache byte ceiling (stretch; completed, not rolled over)

- `MAX_SESSION_CACHE_BYTES = 64 MiB` (pub, so doctor can state it) and
  `enforce_byte_bound`: after the entry-count bound, save estimates the
  serialized size as the sum of per-entry JSON lengths (both parsed
  `entries` and raw `raw_entries`), and evicts oldest-source-fingerprint
  entries first — the same policy as `enforce_entry_bound` — until the
  estimate fits. Dropped counts set `dirty` so the eviction persists.
- Sizing rationale (recorded for review): the operator snapshot is
  9,342,161 bytes at 722 entries (≈12.9 KB/entry average, but entries
  carry full tool-arg maps, so heavy corpora skew far higher). At the
  20,000-entry count bound, a corpus of merely 3.4 KB/entry sessions
  already exceeds 64 MiB — the byte bound binds first exactly where the
  count bound is blind.
- `--doctor` discloses it: new `cache_size_bytes` and `cache_limits`
  report fields plus a text line naming both bounds and the eviction
  policy. Output-contract impact: doctor.json only needs to be valid
  JSON (check-output-contract.sh) — no golden pin broke.
- Tests: byte-bound eviction order/idempotence/cleanliness unit test;
  doctor field + rendered-line assertions in the discovery contract.

## Truthfulness checks performed

- Default-view sample reason on the demo corpus:
  `sampled first 2 of 3 sessions in the --sort cost --order asc view
  (--sample 2)` (exactly the `--sort cost --order asc` invocation).
- Doctor on the real corpus reports the real 9.3 MB size and both bounds;
  nothing evicts today (9.3 MB < 64 MiB), and the bound is a floor for
  growth, not a current behavior change.
- Reasoning share on the fixture: 40/60 = 66.6667% — the audit's own
  numbers reproduce the fold.
- Corpus without thinking models: `reasoning_share_pct` is null and usage
  maps gain no key (asserted in tests) — no spurious churn in existing
  outputs; all 203 baseline tests pass unmodified.

## Process notes (mishaps, for the record)

- A `git checkout .github/workflows/ci.yml` during guard negative-testing
  restored the contaminated ci.yml from HEAD and wiped the CU-17 fix; the
  sed revert was re-applied and the final state re-verified (guard clean,
  diff shows the five lines).
- A python line-surgery move of the two new cache tests corrupted
  session_cache.rs structure (tests spliced mid-function, then the logic
  span deleted). Recovered deterministically: rebuilt the file from
  `git show HEAD:...` plus the surviving prefix, re-applied the five
  intended CU-20/CU-22 edits via anchored replacements with count
  asserts, re-appended the tests inside `mod tests`. Verified by the
  clean `git diff` (171 insertions / 4 deletions in that file, only
  intended hunks) and the full suite.
- The trajectory parser dispatch initially landed in the JSONL-only
  branch of `parse_raw_session`; sidecars are single JSON objects, so the
  first test run failed with "unsupported session format". Moved to the
  parsed-value branch; test now pins the dispatch.

## Out of scope / deferred (unchanged from the prioritization)

- Full Antigravity SQLite store (`.db` + `.pb`) support — filed residual.
- ROADMAP.md closures for CU-17..CU-22: roadmap-phase work (this record
  plus the diff is the acceptance evidence), per cycle convention.
- CHANGELOG entries: cycle-5's implement commit did not touch CHANGELOG
  either; release-notes naming happens with the roadmap/commit phase.
- Everything deferred in the prioritization doc (candidate 51 + parse
  size cap + installer checksums; git-root sandbox fused with N10;
  Hermes tool-failure schema work; candidates 3/19/2; P3-1; TUI reload
  race; operator-gated C8 upstream PR — CU-17 is its precondition).
