---
artifact_contract: "ce-handoff/v1"
created_at: "2026-09-02T12:55:00Z"
title: "Cycle 3 implementation record — trustworthy strings on untrusted input"
summary: "Implements the five change units selected for cycle 3 (P6-1 lead, P6-4, riders P6-2/P6-3/C34) with red-to-green evidence per unit and the full verification matrix (179/179 debug+release, fmt/clippy clean, all runnable check scripts green)."
keywords: ["agenttrace", "cycle-3", "implementation-record", "utf-16", "cjk", "stewardship"]
cwd: "/work/projects/agenttrace"
repository: "luoyectl/agenttrace"
repo_root_sha: "e0059522b4fc74d53824f0e7ea7e4ac94d1465bb"
branch: "master"
head: "e005952"
---

# Cycle 3 implementation record

Run `5d025d55b1194dd1a4dd8784146dfeeb`, phase implement, attempt
`2d3a1075300d41cbb18cdcc2dcca6b2a`. Batch per the stewardship request
(`docs/stewardship/2026-09-02-cycle-3-stewardship-request.md`) and the
cycle-3 prioritization doc. Nothing committed or pushed (outside this
phase's permissions).

Routing: **ce-work** (narrowest installed skill for executing a planned
batch end to end); this run executed in the main working tree — the only
worktree (`git worktree list`), which carries the uncommitted cycle-1/2
state the stewardship request required preserving. No subagent surface;
all work performed in-thread.

## Changed files

| File | Change |
|---|---|
| `crates/agenttrace-core/src/parser.rs` | CU-1: new `hex_escape_u16` reads escape hex from bytes; `repair_lone_surrogates` no longer slices `&str` at computed byte indexes (both crash sites `:3785`/`:3791` removed); 2 unit tests |
| `testdata/generated/adversarial/unicode-escape.jsonl` | CU-1 corpus: 7 lines — both pass-6 reproducers, lone+pair repair case, `\uzzzz`, valid BMP escape, truncated escape, valid pair |
| `crates/agenttrace-core/tests/discovery_contract.rs` | CU-1/CU-5: hostile-file and directory-scan contract tests; opencode placeholder-title fixture test |
| `crates/agenttrace-core/src/lib.rs` | CU-2: `estimate_tokens_from_text` (CJK-aware) replaces bytes/4 at both estimate sites; `reasoning_chars`/`reasoning_lens` count characters; CU-5: `display_title_from_text` extracted and shared; `MetricProvenance.naming`; 3 unit tests |
| `crates/agenttrace-core/src/sqlite_sessions.rs` | CU-5: `first_user_text` aggregation field, `capture_opencode_user_text` (part⋈message join), placeholder-title gate with `provider:placeholder`/`provider_title`/`message_derived`/`session_id` provenance |
| `crates/agenttrace-core/src/session_cache.rs` | CU-4: `unique_temp_path` (pid + counter) for both persist sites; concurrent-persist test (8 writers) |
| `crates/agenttrace-cli/src/main.rs` | CU-3: `--version` early-return hoisted above `validate_primary_action`/`validate_gate_thresholds` |
| `crates/agenttrace-cli/tests/entrypoints.rs` | CU-3: `--overview --version` and reversed order pinned (exit 0, version banner) |
| `CHANGELOG.md` | Cycle-3 entries; the `--version` precedence claim is now true for action validation too |

## Red → green per unit

- **CU-1 (P6-1, HIGH).** Fixture + contract tests committed first; the
  pre-fix run failed with the exact production crash —
  `panicked at crates/agenttrace-core/src/parser.rs:3785:28` — in the unit
  test, and both contract tests failed. After the fix all four pass, and
  the release binary re-run on the pass-6 reproducers exits **1 with
  `Error: unsupported session format`** (a clean degraded error, the same
  path any unrecognized file takes) where it previously exited **101 with
  a panic**; `--doctor`, `--waste`, `--sessions`, `--diagnostics`,
  `--latest`, and directory scans over a directory containing the hostile
  files all exit **0**. Valid surrogate pairs still repair correctly
  (asserted end to end: lone `\ud800` → U+FFFD while `\ud83d\ude00`
  survives as 😀).
- **CU-2 (P6-4).** Decision recorded in code: characters are intended at
  all three sites. ASCII keeps 4-chars/token; each non-ASCII character
  counts one token (`"中文测试中文测试"` → 8 tokens, was 6; tolerance
  ±25% of one-token-per-CJK-character, stated in the test).
  `reasoning_chars` now reports characters (4 for `"中文测试"`, was 12)
  with the unit pinned by test and a doc comment on the field. The
  estimator stays named `estimated_from_text` in
  `provenance.tokens` (existing i18n key preserved).
- **CU-3 (P6-2).** Hoist chosen over rewording: `--version` now returns
  before both validators, making `CHANGELOG.md`'s claim true rather than
  weakening it. Pinned by CLI test in both flag orders.
- **CU-4 (P6-3).** Unique temp suffix (`<name>.tmp.<pid>.<seq>`) with the
  same atomic rename; 8-thread concurrent persist test asserts all writes
  succeed, the snapshot loads, and no temp files survive.
- **CU-5 (C34).** Placeholder pattern `New session - ` gated; naming falls
  back to first-user-message text recovered from the `part`⋈`message`
  join (opencode stores user prose in `part`, verified against the live
  database); real provider titles still win; four-value naming provenance
  with `provider:placeholder` disclosing the gate. Fixture test covers
  placeholder/real/empty titles. Live-db nuance: at implementation time
  only 12/227 local sessions still carry the placeholder (215 now have
  provider summaries titled "Simple OK Response", which correctly win) —
  the gate matters for exactly the sessions the provider does not
  summarize.

## Verification matrix (all this run)

- `cargo test --workspace`: **179/179** (was 169; +10 new tests).
- `cargo test --workspace --release`: **179/179**.
- `cargo fmt --all` applied; `cargo fmt --all --check` clean.
- `cargo clippy --workspace --all-targets`: 0 warnings.
- Release binary rebuilt; pass-6 reproducers re-run end to end: no exit
  101 anywhere; hostile-file error is the standard "unsupported session
  format"; directory scans and every previously-crashing action exit 0.
- `scripts/ci/check-*.sh`: cargo-manifests, plugin-version,
  deterministic-output, docs-commands, output-contract,
  report-semantics, release-surfaces, rust-real-cli-smoke all exit 0.
  `check-rust-tui-real-smoke.sh` and `check-rust-release-local.sh`
  require `expect`, which this host lacks (pre-existing environment
  limitation, unchanged from cycle 2; the non-TTY launch guard itself is
  covered by `launch_guards.rs` tests).
- No `final_validation`/commit/push/pr/ci actions performed.

## Deferred to later phases

ROADMAP.md updates (retiring the closed items into a "Completed cycle 3"
record) and the campaign's final validation — both belong to their own
phases per the work order.
