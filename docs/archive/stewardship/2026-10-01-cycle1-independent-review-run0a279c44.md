---
schema: ce-handoff/v1
created_at: '2026-10-01T00:00:00Z'
title: 'Cycle 1 independent review — campaign 47e4432e, run 0a279c44 (rm-046 + rm-047)'
summary: Independent adversarial review of the cycle-1 batch "Trustworthy token accounting on hostile journals" against roadmap acceptance criteria, security boundaries, durability, and test evidence.
keywords: [agenttrace, repository-maintenance, cycle-1, independent-review, rm-046, rm-047]
cwd: /home/agent/.hermes/conductor-worktrees/agenttrace-80c75f65b7/run-0a279c440c10-0a279c44
repo_root_sha: 9d88b36750a991bd1436dbbc91b4579c39003067
branch: conductor/run-0a279c440c10
verdict: NEEDS_CHANGES
---

# Independent review — cycle 1 batch (rm-046 + rm-047), run 0a279c44 attempt 8b2b8ecb

**VERDICT: NEEDS_CHANGES** — narrowly. The core implementation is correct, fully
re-verified live by this review, and the roadmap/bookkeeping/docs are coherent.
Two P3 completion gaps (one introduced by the batch, one pre-existing but a
direct threat to the remaining gates on this host) and one P4 doc nit should
land before shipping. No re-implementation is needed; the fix list is ~10 lines.

## What this review independently verified (all live, at HEAD 9d88b36)

### rm-046 — saturate token accumulation
- All three merge sites named by the acceptance are covered: `add_usage`
  (parser.rs:3507–3513) and `add_usage_value` (parser.rs:3515–3521) now
  `saturating_add`; `add_opencode_tokens` (parser.rs:3494–3505) routes through
  `add_usage_value`, so it inherits saturation.
- Every other accumulation layer was already saturating at this HEAD:
  session-level event loop (lib.rs:722–765), `total_tokens`
  (lib.rs:1298–1304), cross-session overview sum (reports.rs:1603–1607, with a
  comment anticipating exactly this clamp world), the sqlite ingestion path
  (sqlite_sessions.rs:291, :590–593), and `number_as_i64`
  (parser.rs:3894–3907) clamps u64/f64 inputs at i64 bounds. **No residual
  overflow hole found anywhere in the token pipeline.**
- Live re-repro on the preserved adversarial fixture
  (`/tmp/at-adv/opencode/.../session.json`, `input: 5 + i64::MAX`), fresh
  rebuild of both profiles: debug RC=0 (pre-batch: panic at parser.rs:3472-old),
  release RC=0, both profiles report `tokens.input = 9223372036854775807`,
  `output = 200000`, `cost.estimated = 27670116110567.33` (finite, clamped by
  construction), and `--sessions -f json` emits
  `provenance.Cost = "calculated_from_tokens_clamped"`. The marker condition
  (lib.rs:850–862) covers all five token classes including reasoning.
- Regression test `token_accumulation_saturates_instead_of_wrapping`
  (parser.rs:4573) and `clamped_token_totals_flag_the_cost_provenance`
  (lib.rs:1824) both green.

### rm-047 — codex head-classification rescue
- `json_key_present` (parser.rs:2310–2324) anchor logic is sound: the raw
  needle cannot occur inside a valid JSON string value (its quotes would need
  escaping), and the `{`/`,` preceding-byte anchor additionally defeats corrupt
  quoting — both covered by tests (`codex_ignorable_probe_rescues_...`,
  parser.rs:4597+, incl. escaped-marker and corrupt-quoting cases).
- Whole-line rescue is not a performance regression: the old code already did a
  whole-line `contains` for the payload adjacency negative; the new scan runs
  only when the head classifies the line as `event_msg` (short-circuit `&&`),
  matching the implementation record's design note.
- Live re-repro on `/tmp/at-adv/codex/rollout.jsonl` (marker beyond the
  160-byte window): `--sessions -f json` → `tokens 100/40` (old probe dropped
  the usage line) and `line_skips = {"codex_ignorable_line": 3}`.
- Skip accounting reuses the existing `Metrics.line_skips`
  (lib.rs:357–358, `skip_serializing_if` empty) with `#[serde(default)]` on the
  cached side (session_cache.rs:185–190): old caches deserialize, new key
  round-trips (:958/:995), reports render the map generically (reports.rs:43,
  :587+). Zero schema/cache churn claim verified. Single caller of the
  retyped `parse_codex_rollout_jsonl` updated (parser.rs:120).

### Bookkeeping / docs / evidence
- ROADMAP.md: 48 ids, duplicate probe empty; renumber mapping rm-012..023 →
  rm-034..045 complete with dated record; block-internal cross-refs updated;
  **the new cycle notes' id references all resolve correctly** — `rm-020
  (per-model pricing anchor)` in the pre-review blockquote and `rm-017's
  distribution move` in rm-051's note refer to the first-block items at
  ROADMAP.md:174 (`rm-020`, per-model ledger) and the install-surface item
  `rm-017` (distribution), not to stale campaign-local ids.
- Status discipline: rm-046/rm-047 flipped only to `implemented` with
  "flip to done at the shipping gate" — matches campaign-family precedent.
- Gate envelopes result-430870-326193046.json / result-493778-326216181.json:
  both `outcome=completed`, `returncode=0`, recorded against this worktree.
- `cargo fmt --all --check` clean; `cargo clippy --workspace --all-targets`
  zero warnings; doc-tests 0/0.
- The four compound docs (implementation record, assessment pass 12, research
  pass 10 with its own dated grounding-lineage correction, id-collision
  prevention rule) are internally consistent with the tree and with each
  other; pass-12 dispositions map 1:1 onto roadmap items.

## Findings

### F1 · P3 (medium) — new provenance value has no TUI label arm (introduced by this batch)
`crates/agenttrace-tui/src/i18n.rs:172` — `provenance_label` matches
`"calculated_from_tokens"` but not the new `"calculated_from_tokens_clamped"`
(produced at lib.rs:850–862), so the `_ =>` arm (i18n.rs:183) renders clamped
sessions as **"source unknown" / “来源未知”** in the TUI session-detail cost
line (explorer.rs:2214). The marker exists precisely to disclose clamping; the
repo's only interactive provenance surface inverts that disclosure into an
unknown-source label. JSON/report surfaces are unaffected. Fix: one match arm
(+ zh string), optionally one label test.

### F2 · P3 (medium) — pre-existing flaky test makes full-suite evidence nondeterministic on this host
`crates/agenttrace-tui/src/tests.rs:1954` —
`efficiency_panel_renders_statusline_limits_and_cache_causes` asserts "no
journal means no statusline block" without pinning `AGENTTRACE_SESSION_CACHE_DIR`;
the Efficiency panel lazily loads the ambient journal at app.rs:1549–1551.
This host has a real 2.4 MB journal at `~/.cache/agenttrace/statusline.jsonl`.
Proven by A/B (this review): test filtered to run alone, unpinned → **0/10
pass**; pinned to an empty cache dir → **10/10 pass**. In the full suite it
passes only when the parallel `ctrl_r_force_reload_clears_session_cache_before_loading`
(tests.rs:1605, env-scoping helper at tests.rs:1642/:1806–1815) happens to hold
the process-global env override over the render window. Consequence: full
`cargo test` failed in **2 of 2** of this review's full-suite runs on this host
(3 total failures across all invocations); the recorded 268/0 gates were
fortunate timings, and the upcoming commit/push gates on this shared host can
fail nondeterministically. Not caused by the batch (agenttrace-tui has an empty
diff; the mechanism predates it), but it is exactly the "test evidence"
exposure this review must flag. Fix: pin the env in the test exactly as
tests.rs:1642 does.

### F3 · P4 (low) — acceptance text names a test that doesn't exist under that name
`ROADMAP.md:342` — rm-046's acceptance says "regression test
`token_accumulation_saturates_not_wraps` pins the fixture"; the landed test is
`token_accumulation_saturates_instead_of_wrapping` (parser.rs:4573). Digest-neutral
wording fix at the commit gate (or rename the test).

## Required change list (for the re-implement/commit loop)

1. i18n.rs `provenance_label`: add `"calculated_from_tokens_clamped"` arm
   (EN + zh), e.g. "calculated from clamped tokens" / “根据 token 上限重算”.
2. tests.rs:1954: wrap the render in `with_session_cache_dir_for_test` (or set
   the env) so the test owns its journal state.
3. ROADMAP.md:342: correct the test name (or rename the test).

None of these touch the batch's red→green evidence; the core implementation
itself is approvable as-is.
