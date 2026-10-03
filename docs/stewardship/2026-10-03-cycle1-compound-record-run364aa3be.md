# Cycle-1 compound record — run 364aa3be (repository-maintenance dde7875c)

- **Phase:** compound (attempt `b111d2b8`), 2026-10-03
- **Worktree:** `run-364aa3be00f6-364aa3be` at `fd5532f` — HEAD never moved; every batch file
  stays uncommitted awaiting the commit gate, per the phase contract
- **Inputs consumed (pre-review cycle evidence only — nothing re-run at compound):**
  assess `c22757c9` (report swept from /tmp before compound; durable facts preserved in the
  prioritization doc §0 and the minted rm-298..rm-303 signals), research `c52c2cc3` (same
  sweep; preserved via the rm-006/rm-007/rm-053 evidence-refresh lines landed by the roadmap
  phase), roadmap `cd4c0a4d`, prioritize `e03c70cf` + audit `b8cfcdc2`
  (docs/stewardship/2026-10-03-cycle1-prioritization.md), stewardship `9f7c4806`,
  implement `cfee1fa4` (docs/stewardship/2026-10-03-cycle1-implementation-record.md),
  targeted_tests, full_tests `5f2abd53` (result JSON in the delegate spool; logs and gate
  output tree under /tmp/at-full-5f2abd53, still present at compound time).

## 1. Batch outcome — "The cache obeys its own contract, and the pipe stays pure"

| Item | Status | Recorded outcome (pre-review) |
| --- | --- | --- |
| rm-298 byte-true session-cache bound + first dirs bound (lead, M) | implemented (flipped by implement, folded in wall) | red-first fixture 16,910 B vs 13,112 B ceiling (129%); live corpus gate **67,107,729 B ≤ 67,108,864 B cap** (pre-fix 72,262,482 B = 108.9%); 4,375/5,293 entries kept oldest-first; dirs 2,856 listings / 4,091,271 B ≤ 8 MiB sub-budget |
| rm-301 stdout purity for `-f json` under side-effect flags (S) | implemented | red-first stdout began `"Session cache cleared.\n{"`; post-fix pipe parses as one JSON document, announcements on stderr, human path unchanged |
| rm-303 CHANGELOG section for every merged tag (XS) | implemented | v0.8.1 backfilled (annotated-retroactively); per-tag gate arm red/green on synthetic fixture (rc1→rc0), rc0 on the reconciled tree |

**Full-validation envelope (full_tests `5f2abd53`, consumed as recorded evidence):** fmt rc0 ·
clippy `--all-targets --all-features -D warnings` rc0 · `TMPDIR=/tmp cargo test --all-features`
= **297 passed / 0 failed across 13 test binaries** (cli 20+11+2+9, core 128+7+72+2, tui 46) ·
release build rc0 (binary 12,660,840 B) · **all 12 scripts/ci gates rc0** with
`AGENTTRACE_BIN=$PWD/target/release/agenttrace` — 9 under isolated scratch HOME, and the three
operator-machine gates (check-rust-real-cli-smoke, check-rust-tui-real-smoke,
check-rust-release-local) rc0 under the real HOME (release-local 10m15s, internally re-runs
four contract gates + real-cli). Porcelain fingerprint `ead97f8b2cb4bc0621df09250bab6864`
(8 M + 2 ??, 575 insertions / 45 deletions) identical from implement end through targeted,
full, and compound — no executable surface moved after implement, so the engine validation
digest is copy-verbatim per the emission-time rule (nothing to re-derive at compound).

## 2. Roadmap bookkeeping done at this compound

- **Cycle compound block** added to the ROADMAP "Open items" preamble (newest-first, above the
  2026-10-01 run a9e73293 block): batch, recorded outcomes incl. the full envelope, cycle
  context pointers, cycle-2 leads, mint addendum, watch items.
- **MINTED rm-377** — "Gate scripts must not default to a shared /tmp binary"
  (check-deterministic-output.sh:4, check-output-contract.sh:4, check-report-semantics.sh:4;
  sibling-claimed check-docs-commands.sh:4 deliberately excluded as run 1cb610835006's
  uncommitted rm-369, with a fold-at-integration note). Minted from THIS cycle's full-validation
  outcome: the 12-gate green run was only meaningful because full_tests pinned AGENTTRACE_BIN
  to the worktree binary — the unset-env default validates whatever stale binary last landed at
  /tmp/agenttrace on this shared host class (fleet-observed clobber 2026-10-03).
- **Wall census after this compound:** 114 `- id:` defs = **76 candidate / 21 done /
  17 implemented** (was 78/21/14 at mint; implement flipped rm-298/301/303; compound added
  rm-377). Duplicate-id probe empty; render footer intact and last.
- Statuses of rm-298/301/303 were already flipped by the implement phase with dated
  "CYCLE-1 IMPLEMENTED pre-review" evidence folds — verified, not redone.

## 3. Prevention rules and reusable lessons (run 364aa3be cycle 1)

- **R1 — assert on the artifact, never on the model of the artifact.** The 64 MiB "hard bound"
  was enforced over an estimator that counted entry VALUES only, while the writer also emits
  per-path keys, JSON punctuation, top-level fields, and the whole dirs map (108.9% of cap on
  the real corpus). The durable fix is a byte-true projection pinned by a test that measures
  the WRITTEN FILE (`serialized_doc_size_predicts_the_written_file_exactly`,
  `byte_bound_covers_the_written_document_not_a_model`). Any future "bound/contract" item must
  ship a written-artifact assertion, not an estimator assertion.
- **R2 — machine-readable output is a single document end to end.** Side-effect announcements
  are format-aware (`announce` fn pointer: stderr under `-f json`, stdout otherwise, human path
  byte-identical). The regression test must PARSE the whole stdout stream (serde_json), not
  substring-match it.
- **R3 — gate defaults must be repo-relative, never /tmp-relative, on shared hosts.**
  `${AGENTTRACE_BIN:-/tmp/agenttrace}` silently validates a stale or foreign binary (fleet
  clobber observed mid-RED-staging). Minted as rm-377; until it lands, every gate invocation on
  this host class must pin AGENTTRACE_BIN explicitly (full_tests 5f2abd53 did).
- **R4 — the full envelope needs a HOME split, not one HOME.** Nine gates are hermetic under an
  isolated scratch HOME (also the doctor-latency workaround), but the three real-corpus gates
  read `$HOME/.pi/agent/sessions` BY DESIGN (`AGENTTRACE_REAL_CLI_DIR` override exists,
  check-rust-real-cli-smoke.sh:16-28) and cannot pass under the scratch HOME — their isolated-
  HOME rc1s are environmental, proven by real-HOME rc0 reruns. Budget release-local ~10-15 min
  under the real HOME.
- **R5 — /tmp evidence has a lifetime; durable evidence must live in the repo or spool.**
  The assess and research scratch trees (/tmp/at-assess-c22757c9, /tmp/at-research-c52c2cc3)
  were swept before compound, while the implement/full scratch (/tmp/at-impl-cfee1fa4-*,
  /tmp/at-full-5f2abd53) survived. Consequence recorded for cycle 2: the prioritize doc's
  "PoC fixtures already on disk" note for rm-300 (collide.jsonl / nameless.jsonl) is STALE —
  cycle 2 must re-derive both fixtures from the shapes preserved in rm-300's signals lines.
- **R6 — adopt-with-audit, redo only on infra-death.** Two prior-attempt shapes occurred this
  cycle: roadmap attempt 1ef3db41 (events log ends 'failed', no typed result, clean tree →
  REDONE from scratch, declared in the mint note) vs prioritize attempt e03c70cf (typed result
  JSON present, session succeeded → every verifiable claim re-checked fresh and the selection
  ADOPTED unchanged with a §7 audit addendum). The discriminator is the presence of a durable
  completed artifact, not the re-dispatch itself.
- **R7 — compound-time numbering must sweep DEF ROWS, not mentions.** A bare `rm-[0-9]+` grep
  over spool roadmap copies over-counts: `# dashboard — Roadmap` copies def-claim rm-4xx/5xx
  (other project's id-space) and agenttrace mint comments MENTION rm-373/374/484 numerals in
  prose. Def-row sweep (`^ *- id:` lines, header-gated on `# agenttrace — Roadmap`) across
  worktree walls + spool + /tmp gave the true agenttrace ceiling rm-376 (6cddf8e8 compound
  postimage) → rm-377 minted. rm-374 is left unspent: two fleet records treat it as claimed but
  no def row was found anywhere — flagged for fleet reconciliation, not spent here.

## 4. Commit-gate handoff

Ship the 8 M + 2 ?? delta exactly as the implementation record's census describes, plus this
compound's ROADMAP additions (compound block + rm-377). Ownership notes that still hold:
session_cache.rs / check-plugin-version.sh / CHANGELOG.md owned outright by this batch this
cycle; crates/agenttrace-cli/src/main.rs shared with 7 live lanes — merge by FUNCTION
(write_stderr / announce block), never by line number; entrypoints.rs additions are whole-test
appends. ROADMAP merge rule: this run's mint (rm-298..rm-303) + folds + compound block + rm-377
ship together; sibling renumbering/collision discipline per the mint header note still applies
at integration. The validation digest is recorded at the commit gate per the shipping-PR
convention (no executable surface changed after implement).

## 5. Prior-attempt forensics (this phase, this attempt id)

A prior incarnation of attempt `b111d2b8` had already run and recorded its completion in
project memory (#15994) — but its durable artifacts did NOT survive: no typed result JSON in
the delegate spool, the events log holds only `delegate_turn_progress` markers (no reap/
completion), and the worktree at this attempt's start was back to the implement end state
(ROADMAP +41 mint-only, no compound record, 8 M + 2 ??). Per rule R6's infra-death arm this
redo was REQUIRED, not adopt-with-audit. Corroboration worth recording: the redo — executed
without consulting the memory until after its edits — independently converged with the lost
attempt's memory on every material point (same rm-377 mint and sibling rm-369 boundary, same
114/76/21/17 census, same R1-R7 rule set, same cycle-2 lead rm-300, same /tmp-sweep correction),
which is evidence both reconstructions drew from the same underlying cycle record rather than
from chance.

## 6. Context for the next maintenance cycle

- **Designated lead: rm-300** (flat-pair id namespace hardening — synthesized-id collision +
  nameless-result pairing gap, priority 68.0, effort M; deferred this cycle only on two-M
  landing discipline). Red-path material: re-derive collide.jsonl (explicit `flat-pair-Bash-0`
  id + id-less sibling call + one result → currently Bash count=2 unmatched=0) and
  nameless.jsonl (id-less result without tool_name → count=1 unmatched=1) per R5.
- **Sequenced:** rm-299 (warm-path stat-fingerprint snapshot) folds into in-flight sibling
  rm-286 at integration, survivor keeps the union of evidence; rm-302 rides unlanded rm-201;
  rm-053 conformance harness gains AgentMeasure's conformance/{pack,vectors,runners} as a
  concrete fixture source (issue #24 message_start usage vector).
- **New:** rm-377 (repo-relative gate-binary defaults; fold the fourth script into sibling
  rm-369 if it lands first).
- **Watch:** upstream quiet since pass-11 (tip 52ab2cd, #295; v0.9.0 latest; residual lanes
  #294/#291); npm identity window open (bare 'agenttrace' free, @zack78/agenttrace flat at 456
  dl/mo); ccusage open #1814 (codex Flex service-tier pricing) rides the same
  token_usage_record surface sibling lanes are parsing; pricing snapshot drift keeps widening
  (rm-006 refresh line).
