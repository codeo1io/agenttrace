# Cycle 1 implementation record — repository-maintenance 6d611136 (run cbe30a9c42e5)

Date: 2026-09-30. Campaign: `6d611136fbbc4a6d98abd218e74aa16d` cycle 1. Worktree:
`run-cbe30a9c42e5-cbe30a9c` at HEAD `ce279697b711004ef9c90d36bf1173e686b04b36`
(branch `conductor/run-cbe30a9c42e5`). This record follows the stewardship
ledger format of `docs/stewardship/2026-09-14-cycle7-implementation-record.md`;
numbering continues the upstream convention the merged PR #17 lane set
(`2026-09-30-cycle1-*.md` campaign-local numbering).

## Batch — "Truthful pairing, honest loops, closed channels"

Selected by prioritization 407521b8 from 12 open roadmap items under the
sibling-lane collision map; stewardship request 98492da1 named the surfaces.

| unit | statement | state |
| --- | --- | --- |
| rm-025 | flat-transcript tool_result joins its call via explicit `tool_use_id` with positional fallback | implemented, pre-review/uncommitted |
| rm-028 | loop detection keys on (name, args), not name alone | implemented, pre-review/uncommitted |
| rm-029 | install.sh refuses on missing/malformed `.sha256` sidecar (parity with ps1/npm) | implemented, pre-review/uncommitted |
| rm-030 | source-build fallback pinned via `AGENTTRACE_SOURCE_REF`, never a floating master tip | implemented, pre-review/uncommitted |

Files touched (worktree, uncommitted): `crates/agenttrace-core/src/parser.rs`,
`crates/agenttrace-core/src/diagnostics.rs`,
`crates/agenttrace-core/src/lib.rs`, `install.sh`, `ROADMAP.md`, plus this
record and `docs/solutions/workflow-issues/validate-probe-corpus-before-trusting-failure-signals.md`.

## Verification record (consumed as evidence this compound phase; not re-run here)

- targeted_tests 3fbef9c0: work-order `targeted_command`
  (`local_validation_gate.py --shell-command 'cargo test'`) RC=0; durable
  envelope `/home/agent/.hermes/local-validation-gate/results/result-2611030-325550536.json`.
- full_tests: first attempt d5fbd03d was rejected by the fold gate for a
  structural defect in its delegate record (flat JSON, no `phase_result`
  wrapper) — the run itself was green and was NOT re-run for science; the
  re-dispatch 715a6509 ran the full suite fresh at the same tree: RC=0,
  **248 passed / 0 failed / 0 ignored across 9 suites** (243 baseline + 5 new
  batch tests — review F9 corrected the earlier 241+7 tally: the batch adds
  exactly 5 regression tests), envelope
  `/home/agent/.hermes/local-validation-gate/results/result-3152145-325731231.json`.
- Digest at this tree (validation-policy v1, base = full 40-char HEAD
  `ce279697b711004ef9c90d36bf1173e686b04b36`):
  `validation:v1:669d910daa8cec0a72eb68da55eab39ac03c9e1fafa003f0fc06bb1e962262b9`.

## Cycle ledger

- assess 3f8ddeff — adversarial re-assessment at the post-merge HEAD; 5 fresh
  findings (rm-025..rm-028 seeds, install.sh armv7 ghost target, sidecar
  refusal parity); no-repeats wall re-derived from the 11 assess runs in the
  delegate spool.
- research 5a025a61 — upstream mining (Claude Code 2.1.271..285, ccusage
  v20.0.21..26, codex 0.158..0.161-alpha, LiteLLM catalog) → FR-1..FR-4
  (rm-026, rm-027, rm-032, rm-033).
- roadmap 753451ab — ROADMAP.md extended with rm-025..rm-033 (pure append,
  56 lines, heading/ID invariants verified).
- prioritize 407521b8 — 12 open items scored; batch of 4 selected; deferrals
  recorded (rm-011 third deferral, rm-026/rm-027 collision/effort, rm-031
  strategic fork, rm-033 score).
- stewardship 98492da1 — batch request with 7 live-verified file:line surfaces.
- implement — the four units above with regression tests (gate-first: the
  parity test was extended to refusal semantics and proved RED on the unfixed
  installer before the fix).
- targeted_tests 3fbef9c0, full_tests 715a6509 — as recorded above.
- compound 36f385b6 — this record; ROADMAP status flips; the learning below.

## Learning captured this cycle

`docs/solutions/workflow-issues/validate-probe-corpus-before-trusting-failure-signals.md`
(knowledge track, workflow_issue): the rm-025 verification initially read as
"fix failed" because the hand-crafted probe corpus carried RFC 3339-invalid
timestamps (`…T00:00:4Z`); `parse_time` (diagnostics.rs:933) is strict and its
callers drop unparseable events silently (:741 result-side, :747 call-side),
which converts a correctly-paired corpus into a reported `unmatched` signal
(:535, :760). Rule: lint the corpus before trusting a probe's failure signal;
when a probe and a unit test disagree, the fixture is the first suspect.

## Operator notes (conductor-side, next cycle's gates)

- **Digest base-keying**: `validation_digest(base_sha, repo)` must receive the
  FULL 40-char HEAD; an 8-char prefix hashes differently on identical content
  (669d910d… vs 91c863ff… at this tree). Gate envelopes always record
  `digest_base: "unknown"` — their digests are cross-checks only, never the
  declared value.
- **phase_result schema**: the fold gate requires a top-level `phase_result`
  object wrapper with `status` ∈ {succeeded, failed} (d5fbd03d rejection).
- **Sibling lanes**: run-c8397418 and run-88feec46 hold uncommitted
  parser.rs/pricing.rs work in the usage-accounting cluster, and
  run-304846327112 has pushed branch conductor/run-304846327112
  claiming ROADMAP ids rm-026..rm-032 (commit 334b5a8, PR pending) —
  re-derive the collision map before the next implement phase
  (prioritize 407521b8's map is point-in-time and predates that push).

## Next-cycle candidates (concrete)

1. **rm-032** (static-context attribution) — next-cycle lead candidate by
   score; requires governance.rs `context_trends` extension plus a
   tool-definition-bearing fixture.
2. **rm-011 split** — three sub-defects (search truncation disclosure,
   rates.total, unknown-reason labels) as a batch of their own.
3. **rm-031** decision input — armv7: build lane vs explicit refusal; needs
   stewardship input, and release.yml is contested by open PRs #272/#278.
4. **rm-026/rm-027** after the usage-accounting lanes land — both compose with
   their halves rather than duplicating.

## Integration expectations

- ROADMAP status flips to `done` for rm-025/028/029/030 belong to the commit
  gate, after independent review; the compound phase set `implemented`
  (pre-review, uncommitted) only.
- CONCEPTS.md vocabulary capture deferred: the file does not exist at this
  HEAD but landed on origin/master via PR #17 — creating it here would
  add/add-conflict at integration. Candidate terms when the tree catches up:
  *flat transcript*, *tool_use pairing / unmatched*, *probe corpus*.
- The learning doc adds `docs/solutions/workflow-issues/` — the directory also
  exists on origin/master (two other files); filenames differ, no conflict.
