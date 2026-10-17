# Cycle-1 compound record — run 12f4e5fb (repository-maintenance c151f66b, agenttrace)

- run: 12f4e5fb389d40188a3f60d27678474e | compound attempt: a21c8141d20a4904898d6a5ecf9dd1f7 | 2026-10-10
- base: 97f06d4384fa95182e93dccc41c177996e5a461a (worktree run-12f4e5fb389d-12f4e5fb; porcelain 0 throughout)
- scope: PRE-REVIEW compounding only — review/shipping outcomes land with the review
  phase and next cycle's assessment carries them forward.

## Phase ledger (all attempts green at write time)

| phase | attempt | one-line outcome |
|---|---|---|
| assess | 48871b27 | 5 fresh findings F1-F5 (live PoCs); baselines green 818/0 |
| research | a67e7d7b | 7 lanes; upstream quiescent; C1 port-source #303 + C2 wire evidence |
| roadmap | 1550f891 | minted rm-920..923 + 6 folds; spool patch proven roundtrip-clean |
| prioritize | 952b8091 | selected 'statusline journal robustness' (rm-921 lead + 922/923) |
| stewardship | 204ccbf8 | stewardship_request JSON; 3 change-units; 5 separation pairs |
| implement | af090895 | 6-file patch +649/-54, red-first proven, worktree restored |
| targeted_tests | deefe660 | focused suites + statics + digest reproduction, all green |
| full_tests | 354e982b | VERBATIM gate `cargo test` 47 suites 825/0 with batch applied |

## Batch outcome (pre-review)

'statusline journal robustness' — rm-921 (lead) + rm-922 + rm-923, delivered as ONE
spool patch, status flipped to `implemented (pre-review)` in the compound roadmap delta:

- rm-921: crossings parse-once precompute + partition_point sweep; sparse n=2000
  release 8.29s -> 0.04s; boundary picks max-before/min-after by captured_at
  (out-of-order 20.0 -> 10.0); benign outputs byte-stable.
- rm-922: bounded read with disclosed `read_truncated` (planted oversize journal now
  disclosed 124527/10485741 instead of whole-read); invalid UTF-8 lossy-recovers per
  line — bad-byte journal 0 -> 3 captures, retention un-wedged.
- rm-923: MCP stdio 1 MiB per-message cap with JSON-RPC error (mirrors the statusline
  host's disclosed-refusal pattern); red on unfixed (panic 'refused, not answered').
- Full gate: verbatim `local_validation_gate.py --shell-command 'cargo test'` exit 0,
  47/47 suites ok, 825 passed / 0 failed (baseline 818 + 7 new tests).
- rm-920 deliberately DEFERRED (hard dependency: linux-only CI cannot compile-verify a
  Windows file_index guard — bundle with the cross-OS matrix port next cycle).

## Validation-digest facts (for the fold and commit gates)

- Work-order digest `validation:v1:9f50a141c33fb0c07de2738ad4aa2032a322522e48e91c4bbe250454d2b27a20`
  re-derived IDENTICALLY via the import route (hermes_conductor.validation_policy,
  module .../8678da8f.../src) on the batch-applied worktree — the 6-file .rs/.md batch
  touches nothing in the classifier-executable set, so the digest is invariant.
- The gate wrapper's own result envelope carries a DIFFERENT internal digest
  (`validation:v1:71599313...`, `digest_base: unknown`) — wrapper-internal with
  unresolved base; do not substitute it for the work-order digest (envelope preserved:
  delegate/354e982b...-scratch/gate-envelope.json).

## Lessons (durable; feed the next cycle's assessment)

1. **Sparse-value journals are a quadratic-probe class.** The cost driver was not the
   scan loop but `rate_limit()` re-cloning + `from_value`-parsing the payload per
   probe step (n distinct resets x n captures). Prevention rule: any per-key probe
   over a journal parses each capture's state ONCE into a precomputed vector, then
   binary-searches (partition_point) the (captured_at, position)-sorted sweep.
2. **A cap enforced on one path is not a cap.** STATUSLINE_CAPTURE_MAX_BYTES bound
   only the append path; reads were unbounded, and a single invalid UTF-8 byte
   silently zeroed insights AND permanently blocked compaction (retention wedged).
   Prevention: bound symmetric (append + read), and decode failures degrade to
   DISCLOSED recovery (lossy per-line) with a surfaced stat, never silent zero.
3. **Sibling surfaces should share input-bound doctrine.** The statusline host's
   1 MiB disclosed-refusal pattern predates the MCP transport's boundlessness;
   when adding an input path, grep for the sibling bound and mirror it (cap value +
   refusal semantics + doc disclosure in one unit).
4. **Assessment PoCs are regression-test raw material.** The assess-phase generator
   (statusline_poc.py) became implement's red-first harness (sparse shape + benign
   control + boundary shapes). Rule: adversarial PoC generators from assess live in
   the run's scratch spool with timings pinned, so implement can reuse them verbatim.
5. **Digest invariance under code-only batches.** A .rs/.md-only batch leaves
   validation_digest unchanged (classifier counts only .py + scripts/ + workflows/);
   re-derive via the import route to CONFIRM rather than assume, and never mistake
   the gate wrapper's internal envelope digest (digest_base unknown) for the
   work-order digest.
6. **Dependency-coherent deferral kept the batch locally provable.** rm-920 was the
   2nd-highest-priority mint but was deferred on a compile-verification dependency
   (linux-only CI); the batch that shipped was the one provable end-to-end on this
   box (red-first, full gate). Rule: when a fix's acceptance requires an OS you
   cannot compile for, bundle it with the CI-matrix row that first compiles that OS.

## Next-cycle leads (ranked, pre-review)

1. **rm-920 + cross-OS CI matrix port (bundle).** Port upstream #303 (36b943f:
   macos-14/windows-latest jobs, actionlint, MSRV 1.85) onto our CI layout; land the
   Windows journal guard in the same batch — first-ever compile+test coverage of every
   cfg(not(unix)) arm. Inputs: this run's assess F1 (statusline.rs:384-417 comment is
   factually wrong about the rename), git show 36b943f.
2. **rm-617 + rm-618 (+ rm-251 R1) accounting batch** (88/87/79): verify-first —
   re-run the recorded PoCs at the next base before scoping; rm-230-class
   invalidation surface (schema/semantics), dedicate the cycle.
3. **rm-232 cross-session dedupe (86)**: row records stale PoCs; verify-first step
   not yet run.
4. **Riders batch (small):** pricing snapshot refresh (live 4,507 vs bundled 3,131,
   55 real cost-field drifts incl. claude-sonnet-5.5 cache-read halved) + compact_str
   0.9.1 -> 0.10.0.
5. **rm-037 spend_limit window**: now wire-confirmed upstream (claude-code 2.1.295
   changelog names the status line as a spend-limit surface); blocked on a local
   post-2.1.295 gateway capture (none on this box as of 2026-10-10).

## Commit-gate seam map (what lands, in order)

1. Code batch: delegate/af090895a6bc4676a3fa3f4468171727-scratch/statusline_robustness.patch
   (6 files, proven apply-clean on 97f06d43).
2. Roadmap wall: delegate/1550f891bb7547b7b90b37412a1f8d89-scratch/ROADMAP.patch
   (rm-920..923 band + 6 folds; proven apply-clean on the pristine wall).
3. THIS compound delta: delegate/a21c8141d20a4904898d6a5ecf9dd1f7-scratch/ROADMAP.compound.patch
   (status flips + evidence riders + rm-920 deferral rider; proven apply-clean ON THE
   POSTIMAGE TREE — apply strictly AFTER #2; sequential apply on the pristine wall
   without #2 will fail at the band hunks by construction).
4. This record file itself (docs/stewardship/2026-10-10-cycle1-compound-record-run12f4e5fb.md).
   Done-flips of rm-921/922/923 to `done` remain reserved to the commit gate AFTER
   review passes (they are pre-review today).

## Integrity pins (this block pins the OTHER artifacts; this record's own hash is
pinned externally in the attempt scratch sha256.txt and the PhaseResult)

- code patch sha256: f21a4e3b1e83ee2b87753e270efca5b71deec06baa1d289336dc2438a358e671
- roadmap-phase patch sha256: c86808e14210ebe43fd913e4b7a9481498d6559b5395e70d9f85ac0aac37fb00
- roadmap-phase postimage md5: 028eecea939b0cef615fc8b747cabaef
- compound patch sha256: 15b4f8d388f168bf3d1b2a7516bbbb75db4551e879d8f27beeaf002864a3b0ab
- compound postimage md5: 86134a476055ce51ae1ee514b6f73b54
- full-suite record with batch applied: 47 suites / 825 passed / 0 failed
  (delegate/354e982b18c74037826180d66daf3ea6-scratch/full-tests.log)

## Integration addendum (2026-10-10, conflict case e2d35c6f)

This record was written pre-review at base `97f06d4` and lands at a later
mainline HEAD (`428f0e9`) through conflict cases a94d0019 (textual) +
e2d35c6f (independent review). What changed relative to its pins:

- **rm-922 disclosure figures (batch-outcome bullet above)** — the
  "124527/10485741" pair was the candidate tree's read-length `bytes`
  accounting. On the merged tree the landed rm-898 rule (review 95d74221
  F7) keeps `bytes` naming the ON-DISK metadata size, so the same planted
  journal discloses `lines 124527 / bytes 10938890 (on-disk) /
  capped_away_bytes 453130 / read_truncated true` — the cut is carried by
  `capped_away_bytes`, never by shrinking `bytes`. rm-922 also COMPOSES
  with rm-898 rather than replacing it: one reader
  (`read_statusline_capture_bounded`) serves both rows, keeping rm-898's
  tail cap, UTF-8 boundary walk and richer text line beside rm-922's
  lossy decode and `read_truncated` flag (CHANGELOG carries the
  composition note).
- **Single-read discipline kept** — the candidate's budget-view text arm
  re-read the journal; the merged tree drops that re-read (rm-684/rm-898
  own the single shared read), and rm-917's calendar-window budget
  semantics (landed at HEAD via run d02291d0efbb, `4433e0e`) are kept
  whole beside the batch.
- **Next-cycle lead #2 is STALE as a lead** — rm-617 + rm-618 (with
  rm-619 and the rm-251 R1 arm) landed at mainline HEAD in the interim
  via run 0a55a397eecc cycle 1 (`24b1917`, merged `9e57ccd`), and
  rm-917/rm-918 landed via run d02291d0efbb (`4433e0e`). The ranked
  leads that still stand: #1 (rm-920 + cross-OS CI matrix port bundle),
  #3 (rm-232, verify-first), #4 (riders: pricing refresh + compact_str),
  #5 (rm-037 spend_limit, blocked on a post-2.1.295 gateway capture).
- **Done-flips** of rm-921/rm-922/rm-923 remain reserved to the commit
  gate (rm-012 convention); independent review attempt 0d27652e
  returned APPROVED before this landing.
- **Verification at this integration** (targeted only; full suite
  deferred to the supervisor's post-resolution gate): workspace
  `cargo check --all-targets --locked` rc0, fmt rc0, clippy
  `-D warnings` rc0, core `--lib -- statusline` 34/0,
  hostile_journal_disclosure 15/0, journal_truth_batch 13/0, cli
  mcp_server 6/0, doctor_honesty_contract + doctor_demo_contract 2/2/0,
  tui 56/0, entrypoints 49/0.
