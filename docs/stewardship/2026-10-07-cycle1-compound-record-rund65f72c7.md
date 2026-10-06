# Cycle 1 compound record — run d65f72c7 (repository-maintenance 71aba0ac, cycle 1)

Date: 2026-10-07 · compound attempt 4d87d33d (after targeted_tests c947 and full_tests c62a4c96)
Base: 8991144 (HEAD, campaign worktree; porcelain at compound = the 5-file implement delta + this record)
Scope: pre-review compounding only — no test execution at compound; all outcomes below are the
recorded envelopes and proof chains of the implement/targeted/full phases, consumed as evidence.

## Batch: "honest failure boundaries"

| row | subject | files | outcome |
| --- | --- | --- | --- |
| rm-583 (LEAD, reliability 82.0) | upstream fetch timeout hangs when a grandchild holds the pipes | crates/agenttrace-cli/src/upstream.rs (+37/−2) | implemented pre-review |
| rm-584 (correctness 70.0) | codex journals: two silent-drop holes in the disclosure contract | crates/agenttrace-core/src/parser.rs (+88/−8) | implemented pre-review |
| rm-587 (security 45.0) | deny lane allows unsound advisories — "advisories ok" is a false clean | deny.toml (+14/−1) | implemented pre-review |
| rm-590 (developer-experience 28.0) | positional path lane folds not-a-regular-file into does-not-exist | crates/agenttrace-cli/src/main.rs (+10/−0) | implemented pre-review |

Uncommitted delta total +216/−11 including ROADMAP.md +67 (the roadmap-phase mint; compound adds the
flips/notes/banner on top). Unselected this cycle, left candidate with next-cycle context:
rm-585 (lead candidate, 60.0), rm-586, rm-588, rm-589.

## Recorded validation outcomes (NOT re-run at compound)

- targeted c947 — prescribed gate VERBATIM (`local_validation_gate.py --shell-command 'cargo test'`):
  GATE_RC=0, 21 suites, 465 passed / 0 failed (463 assess baseline + 2 batch pins); envelope
  `result-700465-378282466.json`; statics rc0: fmt, clippy --workspace --all-targets -D warnings,
  cargo deny --all-features check advisories.
- full c62a4c96 — `validation.full_command` VERBATIM: rc0, 21/21 `test result: ok`, 465/0, both pins
  confirmed in-pass (gate.log:49 / :208); envelope `result-1435878-380465441.json`.
- Per-item proof chains (implement e130765f, logs under /tmp/at-impl-e130/):
  - rm-583: holder PoC rc=1 @~30s named UPSTREAM_FETCH_TIMEOUT (baseline rc=124 @50s external kill);
    control shim unchanged.
  - rm-584: live diagnostics on the assess corpus now
    `{codex_missing_type:1, codex_non_object_line:number:1, codex_non_object_line:string:1}` vs
    `line_skips: {}` pre-fix.
  - rm-587: deny rc1 `error[unsound] RUSTSEC-2026-0253` under `unsound = "transitive"` + `ignore = []`;
    rc0 `advisories ok` with the dated carve-out.
  - rm-590: `/dev/null` → "positional path exists but is not a regular session journal file…";
    missing/dir arms unchanged.

## Digest lineage

Dispatch token `validation:v1:a72443d2…` declared VERBATIM at both validation turns; the tree was
porcelain-stable implement→targeted→full→compound. The whole batch sits outside the engine digest's
re-derivation set for this repo (crates/** derives []/none; ROADMAP/deny drift cannot move it).
The gate's own envelope digest (`validation:v1:7b779eb6…`, digest_base=unknown) is a different
namespace by design and is evidence only — see PR-4.

## Dead-attempt forensics

full_tests attempt 1f9840429ac04aa2a49a2f4c44d305dd: event log shows delegate_turn_started → one
progress tick (message_count 2, ~23s) → session_reaped (provider infra) → completed=failed; the typed
result artifact is absent; no gate ran. Nothing adopted; the phase was redone from scratch. An absent
envelope is not evidence the work never happened — and a present artifact is not proof it is valid;
verify, then adopt or redo (PR-5).

## Id accounting

ZERO ids minted at compound. Wall after compound: 206 def rows, zero duplicate ids, managed footer
last; four rows flipped candidate→implemented (done-flips reserved to the commit gate, rm-012
convention). Fleet ceiling moved mid-cycle past the research snapshot (e94bb1ee minted rm-578..582
after research 5782) — re-census live before the next mint (PR-6).

## Prevention rules (reusable)

- **PR-1 — timeout arms must not join drain threads.** A bounded-subprocess timeout path that kills
  only the direct child and then joins stdout/stderr reader threads deadlocks whenever a grandchild
  inherits the pipe write-ends (`read_to_end` never sees EOF) — the deadline contract is violated by
  its own cleanup. Kill + wait the direct child, return the named error, and DROP the joins, exactly
  as governance.rs `wait_child_bounded` and (now) upstream.rs `run_bounded` do; document the leaked-
  drain tradeoff in a comment. The stronger alternative when a happy-path deadline is wanted:
  spawn into a dedicated process group (`pre_exec` setsid / `CommandExt::process_group(0)`) and kill
  the group. Regression shape: a PATH-shim `git` whose fetch leaves `sleep 300 &` holding the pipes
  must exit rc=1 at the deadline with the named error (pin: `run_bounded_timeout_survives_a_
  grandchild_holding_the_pipes`).
- **PR-2 — every silent-drop parser arm lands in a disclosure counter.** `None => continue` and
  `_ => {}` arms in lenient parsers are invisible data loss; an empty `line_skips` map is a false
  clean, not health. Key counters by shape (`codex_non_object_line:<kind>`, `codex_missing_type`)
  following the existing counters-insert pattern, and pin the golden so disclosed drops are counted,
  never silent (pin: `codex_line_skips_disclose_every_dropped_shape`).
- **PR-3 — a bare "advisories ok" is not audit evidence.** cargo-deny v2 default-accepts
  `informational = "unsound"` advisories, so the lane reported ok while RUSTSEC-2026-0253 (lru
  use-after-free, patched ≥ 0.18.2, lock 0.18.1) sat open. The lane now carries
  `unsound = "transitive"` plus a dated ignore entry for that advisory; the ignore MUST be dropped
  when the lru bump (unlanded rm-552) lands. Future no-advisories-open claims cite the policy, not
  the bare ok. v2 schema fact: `unsound` accepts all|workspace|transitive|none.
- **PR-4 — know your digest namespaces.** The local-validation-gate envelope digest and the engine's
  dispatch-token digest are different namespaces by design (the envelope is a re-derived additive
  record; without `--digest-base-sha` it can never stand in for the token). Declare the dispatch
  token verbatim when no executable surface moved; re-run and re-declare when one did.
- **PR-5 — forensics before redo.** On a dead prior attempt, read the typed artifact AND the event-log
  tail, verify lineage/tree-census/cleanliness, then adopt or redo — and declare which. This cycle:
  redo was correct (reaped 23s in, zero durable work).
- **PR-6 — live census at every mint/selection.** The fleet id-space moves mid-cycle; a recorded map
  is stale by the time it is read. Every mint, selection, or next-cycle designation re-runs the live
  claim sweep (landed wall + sibling worktrees + spool), and rows reconcile BY TITLE at integration.

## Commit-gate checklist (for the landing, not compound)

1. ONE commit: 5 modified (ROADMAP.md, upstream.rs, parser.rs, main.rs, deny.toml) + this record.
2. CHANGELOG Unreleased riders: Fixed bullets for rm-583 / rm-584 / rm-590 (user-visible behavior);
   rm-587 is CI-config-only — follow the rm-444 precedent (no CLI-behavior bullet).
3. Flip rm-583/584/587/590 implemented→done at the commit gate (rm-012 convention).
4. No schema constants moved in this batch (SESSION_CACHE 27 / SQLITE snapshot 7 untouched) — no
   docs-guide sentence alignment owed; verify check-plugin-version.sh rc0 at the landing tree and
   carry the sanctioned no-changelog-section marker block only if it is red.
5. Drop/keep the RUSTSEC-2026-0253 ignore in lockstep with rm-552's lru bump when that lane lands.

## Next-cycle context

Lead candidate rm-585 (spend-by-branch from the unused `gitBranch` wire field; claude-lane-only first
cut, gap disclosed; codeburn v0.9.25 reference UX). rm-586 (#306 time-bucketed reports port; riders:
rm-549 dedupe lesson, rm-574 no-silent-fallback family). rm-588 (pricing refresh union contract —
lands with the next rm-176 refresh; LiteLLM moved 3x on 2026-10-06). rm-589 (--exclude/--session-id
filters; standalone). Re-census the fleet ceiling before selection/mint (PR-6).
