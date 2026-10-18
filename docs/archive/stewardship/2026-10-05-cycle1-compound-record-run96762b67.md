# Cycle-1 compound record — run 96762b67 (agenttrace)

Compound fold of repository-maintenance cycle 1 (repository-maintenance:8dc71851, worktree
run-96762b67ccd9-96762b67, base at dispatch 33cc9bb, HEAD 700a67c from prioritize onward —
the 4-commit drift 33cc9bb→700a67c was audited at prioritize and proven defect-surface-neutral
for the batch). Batch: **rm-505** (LEAD, security 82.0 — CSV statement hostile-cell hardening,
rm-409 residual) + **rm-444** (RIDER, reliability 45.0 — check-docs-commands.sh doctor leg
scoped off operator HOME). Written at compound attempt 5c62a3c9, 2026-10-05 real UTC (host
clock; the implement re-verify attempt's `2026-10-06` stamp is fleet clock drift per the
recorded convention — host UTC is the date authority).

All outcomes below are **pre-review** evidence transcribed from the recorded phases —
NO test execution at compound, per the compound contract. Review and shipping outcomes
happen after this phase; the next cycle's assessment carries them.

## Cycle outcome (pre-review evidence chain)

- assess 871cae94 (fresh adversarial re-derivation at 33cc9bb, post-b01edcd landed surface
  audited in depth) → F1a raw OSC-52 clipboard-write bytes in the by_model CSV row, F1b
  tab-prefixed `=HYPERLINK` cell, F1c CRLF negative (single RFC-4180-quoted multiline cell),
  F2 leading-space formula bypass into `--sessions -f csv` and the default TSV; envelope
  `cargo test --workspace` 410/0, fmt/clippy/docs/example gates rc0.
- research 1103a6a4 (live network sweep; detail in spool 1103a6a4…-research-notes.md) →
  upstream v0.10.0 (2026-10-04) with PRs #301–#307; R1 dual session_id/sessionId misroute
  live-verified locally (single.json → SOURCE=qwen_code); ccusage #1780/#1782 OpenCode-v2
  fork semantics; crates.io/LiteLLM/OTel re-census.
- roadmap 04dc4ed2 → minted the collision-proof band rm-505..rm-509 above every landed id
  and every session-recorded in-flight claim (2d37535d rm-446, the rm-448..459 lineage,
  frontier rm-503); folds as dated appends on rm-042/rm-044/rm-045/rm-386/rm-006.
- prioritize 6b56f31a → LEAD rm-505 + RIDER rm-444; full dirty-sibling collision census
  (csv_export.rs + check-docs-commands.sh uncontested; parser.rs is_qwen_* racing 2d37535d —
  excluded from batch; bbe84274's same-day "rm-506 (security 80.0)" LEAD mint shares our
  rm-505's subject family — merge by title at integration).
- stewardship ab78d527 → contract re-established after an engine re-dispatch (prior attempt
  92cbe0e3 was valid; this one re-derived every surface live at 700a67c); two change units,
  one repo, one worktree.
- implement 85477a54 → built the full batch delta, fold-rejected for a missing KTD13
  `changed_surfaces` attestation ONLY (zero code defect); 730cf5fb re-verified every claim
  live and added the test-hermeticity rider (csv_export e2e now sandboxes
  HOME/XDG_CACHE_HOME/AGENTTRACE_SESSION_CACHE_DIR per test thread — it previously spawned
  CLIs against the operator's REAL ~/.cache/agenttrace/sessions.json, observed racing live
  agenttrace processes twice, class-proven with an 80× junk-hammer vs 3× 7/0 sandboxed
  control; operator cache sha-proven untouched after the fix), fold-rejected on the same
  attestation-only ground; e981f43a ADOPTED the uncommitted delta (lineage verified: HEAD
  700a67c, porcelain exactly the 4 batch files, foreign stash `run555-roadmap-delta`
  untouched) and attested — folded clean.
- targeted_tests c5ce66f6 → targeted_command run VERBATIM; the runner reported "unable to
  prove narrow test scope" for the scripts/ shell surface and fell back to the authoritative
  gate (`cargo test` rc0, workers 4) — plus direct execution of the changed surface
  (check-docs-commands.sh rc0, 1.502s; doctor.json mode='custom directory' proving the
  rm-444 scoping live). Zero surfaces changed that turn.
- full_tests d4a4fda9 → dispatch full_command executed VERBATIM; gate admitted (workers=1,
  host-busy), exit 0, 17/17 `test result: ok`, **414 passed / 0 failed** across 17 targets
  (cli 83 incl. the 7-test csv_export suite, core 284, tui 47, doc-tests 0). The FIRST
  full_tests attempt was fold-rejected for an evidence-contract defect ONLY — the declared
  command did not match the dispatch full_command verbatim; the suite itself was green both
  times. Digest token proven current by re-derivation with the engine's own
  `hermes_conductor.validation_policy.validation_digest` at base 700a67c (exact match).

## Prevention rules (reusable, fleet-scoped)

- **PR-1 — KTD13 attestation on EVERY implement fold, first time, every time.** Two of this
  cycle's three implement attempts (85477a54, 730cf5fb) were fold-rejected for a missing
  `validation_evidence.changed_surfaces` declaration with zero code defect. Declare ALL
  repo-relative paths actually changed — source, TESTS, and scripts (a `scripts/**` path
  classifies executable) — with no provenance prose. This is the second consecutive cycle
  to pay it (555a174d's 349ab712, recorded there as PR-1); the rule is now standing fleet
  law, not a reminder.
- **PR-2 — Attestation-only fold rejects: ADOPT, verify lineage, re-attest — never redo.**
  When the fold gate rejects an implement attempt on evidence shape alone, the delta is
  already in the worktree. Before adopting: `git rev-parse HEAD`, exact porcelain-set match,
  foreign-stash inventory; then re-attest with the declared surfaces. e981f43a folded a
  delta two prior attempts had built, with zero code redone. Corollary: a transport-lost
  attempt is not a lost delta — the tree, not the session, holds the work.
- **PR-3 — Tests that spawn the real binary must sandbox the operator's world per test
  thread.** The csv_export e2e suite spawned CLIs against the REAL ~/.cache/agenttrace —
  racing concurrent agenttrace processes (two observed flakes, class-proven) and writing
  fixture entries into operator state. The fix (HOME/XDG_CACHE_HOME/AGENTTRACE_SESSION_CACHE_DIR
  per thread, the rm-301 entrypoints.rs convention) is now the pattern for every
  spawn-the-binary test. The WIDER class is open and recorded on rm-505's row:
  entrypoints.rs spawns CLIs unsandboxed 35/36 times — a standing next-cycle lead.
- **PR-4 — Fleet-shared fixed /tmp paths are multi-tenant hazards.** Two independent hits
  this cycle: the deterministic gate's default bin `/tmp/agenttrace` vanished between two
  sessions, flipping check-deterministic-output.sh to rc=1 until AGENTTRACE_BIN pinned the
  repo build (rm-444 row note: extend the rm-369 repo-bin default to that gate); and the
  docs gate's fixed AGENTTRACE_CI_OUT is shared across concurrent runs (sibling-run
  concatenation observed on this host). Rule: a gate or test that defaults to a fixed
  /tmp path on a fleet host must either mktemp its own or pin the repo-local artifact.
- **PR-5 — The full-suite evidence contract is literal.** validation_evidence.command must
  equal the dispatch `full_command` CHARACTER-FOR-CHARACTER (this cycle's first full_tests
  fold-reject was exactly this defect, with a green suite behind it), and the gate's own
  result envelope digest (`digest_base: "unknown"`) is NOT the engine's dispatch token —
  when nothing executable changed, declare the dispatch digest verbatim and prove currency
  by re-deriving it with the engine's own validation_policy code at base=HEAD before
  declaring. Both checks are two commands and they close the two failure modes this
  fleet has actually hit.

## Residuals and next-cycle context

- **rm-506** (qwen dual-key rejection, 78.0) — smallest lead CONDITIONAL: 2d37535d's
  uncommitted lane already implements it (campaign-local rm-445 at base ea5c41e). Next
  cycle claims it ONLY if that lane is swept without landing; otherwise merge by title at
  integration. Do not touch parser.rs is_qwen_* until then.
- **rm-507** (OpenCode v2 fork accounting, 74.0) — deferred lead; needs the golden v2
  fixture db from ccusage #1780's schema; sequencing rider: lands on sqlite_sessions.rs,
  same module as the rm-044 rusqlite jump — sequence or co-land.
- **rm-085** (--lang truthfulness) — designated lead AFTER a fresh sibling sweep on
  reports.rs/main.rs (≥4 dirty lanes at prioritize; still byte-identical no-op live).
- **rm-367** stage (a) — if the e602bb69 doctor.rs lane resolves/sweeps.
- Wider-class leads recorded on the rows: entrypoints.rs unsandboxed spawns (rm-505 note);
  check-deterministic-output.sh /tmp/agenttrace default (rm-444 note).
- Deferred-by-design this cycle (prioritize table): rm-421, rm-251, rm-195, rm-239, rm-384,
  rm-240, rm-232, rm-053, rm-164, rm-036, rm-175 — dispositions with reasons in spool
  6b56f31a's prioritize-notes.
- Watch items (research, unchanged): upstream v0.10.0 PRs #301–#307 now have full reference
  implementations for rm-042 (daily/blocks), rm-508 (subagents), rm-509 (self-updater);
  OTel semconv-genai still untagged (rm-229 stays parked); upstream #310 superseded
  locally by rm-450's lineage; #103 provenance — local is AHEAD (rm-054+rm-408).
- Fleet numbering at compound: this campaign's band rm-505..509 is campaign-local until
  merge — renumber by TITLE at integration per the 880a7b9e/5af7cbb6 discipline. Known
  concurrent-mint hazard: bbe84274's same-day "rm-506 (security 80.0)" LEAD (subject family
  = our rm-505 + reports.rs arms; no code delta on its worktree at census). Any future mint
  re-runs the live claim census at mint time — the rm-XXX id space is the only global
  registry and in-flight bands above this tree (2d37535d rm-446, the rm-448..459 lineage,
  frontier rm-503) were recorded but not on-disk at the last census.

## Commit-gate note (for the phase after review)

ONE commit for the whole batch: 4 modified (ROADMAP.md, crates/agenttrace-cli/src/csv_export.rs,
crates/agenttrace-cli/tests/csv_export.rs, scripts/ci/check-docs-commands.sh) + CHANGELOG.md
+ this stewardship record. Review and shipping outcomes are deliberately absent here.

## Evidence index

- Assess: spool 871cae94f1074a4f85a5416460feb5e4 (+ /tmp/at-assess-871cae94/ PoC corpora).
- Research: spool 1103a6a481a44112bb1e4b822112bdb7-research-notes.md (+ result JSON).
- Roadmap: spool 04dc4ed28017458aa1d5e8b919c8da6b.json.
- Prioritize: spool 6b56f31ac79648878c73bc0f2b6b8967-prioritize-notes.md (+ result JSON).
- Stewardship: spool ab78d527977a4477b6a8463902b9c5d0 (+ -stewardship-notes.md).
- Implement: spool 85477a54 / 730cf5fb (rejected, attestation-only) and e981f43a (folded);
  logs /tmp/at-impl-85477a54/ and /tmp/at-impl-e981f43a/.
- Targeted: spool c5ce66f6 envelope copy /tmp/at-tt-c5ce66f6/gate-envelope.json + docs-gate.log.
- Full: spool d4a4fda9bab748edb6ba5df618f2cd79 + /tmp/at-full-d4a4fda9/full-tests.log
  (sha256 f37710eb…) + gate envelope
  ~/.hermes/local-validation-gate/results/result-1581557-372635032.json.

## Provenance (adoption note)

Record drafted by compound attempt 5c62a3c9 (provider-dead mid-turn; typed artifact never
written). Adopted and verified by attempt ce1b158a: every factual claim re-checked against
the phase spool artifacts (assess 871cae94, research 1103a6a4, roadmap 04dc4ed2, prioritize
6b56f31a, stewardship ab78d527, implement 85477a54 / 730cf5fb / e981f43a, targeted c5ce66f6,
full 0fb655ef / d4a4fda9). One factual repair: "base at dispatch 33cc9cc" corrected to
33cc9bb — 33cc9cc does not resolve in this repository's object db; it was a digest typo
(the commit is 33cc9bb, parent b01edcd; the same typo was repaired in 5 places in the
uncommitted ROADMAP.md roadmap-phase text). ROADMAP.md compound banner, per-row VALIDATED
appends on rm-505 and rm-444, and the CHANGELOG Unreleased Fixed rider for rm-505 were
landed by ce1b158a; no test was executed at compound.
