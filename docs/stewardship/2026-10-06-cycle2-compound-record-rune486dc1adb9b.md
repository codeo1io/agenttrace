# Cycle-2 compound record — run e486dc1adb9b (agenttrace)

Date: 2026-10-06 · Phase: compound (attempt 1cf661be, after provider-dead 2d11ea18 — event log holds only status pings then `session_reap(failed)`, no typed artifact, tree identical to full_tests' end state at dispatch; phase redone from scratch) · Lane: run worktree `run-e486dc1adb9b-e486dc1a` @ 1511547 (the run's dispatched base). All cycle work is uncommitted on the run's working tree by design; review and shipping happen after compound per the phase contract, and the next cycle's assessment carries them forward.

## Cycle outcome (pre-review evidence chain, consumed — NOT re-run)

Batch **"pi journal accounting truthfulness, part 2"**: rm-253 (LEAD, assess 2e85675 F1 — pi v3 MESSAGE-level `usage.cost.total` silently ignored: journal-recorded $0.0075 reported as $0.0165 catalog list, 2.2x, no disclosure) + the rm-408 pi zero-arm residual rider (assess F3 — present-but-zero pi usage blocks vanished with no disclosure). Selected at prioritize 5678aeda over a 97-candidate open wall with collision risk decisive — every other fresh candidate was claimed by at least one unlanded sibling implementation; stewardship 1179b8468 froze the batch as a structured request without choosing Git topology.

Attempt trail (dead-attempt forensics): implement 79fc974a provider-dead before any code — typed artifact absent, event log empty of work, tree held only the roadmap delta; nothing adoptable, phase redone from scratch as dc9e95fd. Compound's own prior attempt 2d11ea18 died identically.

- **implement dc9e95fd** — RED-first captures proven pre-fix: pi_journal_contract 7 passed/2 failed (`both_pi_arms` got 0.0075 vs 0.015; catalog flip got upstream 0 vs 0.0075), pi_usage_tree_accounting 15/3, core lib 2/1 (`inserts_recognized_zero_classes` panicked), and the schema tripwire discovery_contract RED pre-pin-update (`left: Some(25) right: Some(24)` — the pins catch the bump). Delta: parser.rs +78/−11 (message-arm recorded cost + recognized-zero pi extraction), session_cache.rs +11/−1 (SESSION_CACHE_SCHEMA_VERSION 24→25, rm-230 convention; SQLite snapshot stays 7), discovery_contract.rs +9/−8, pi_journal_contract.rs +77/−9 (catalog pin flipped to recorded-wins BY INTENT + new both-arms pin), pi_usage_tree_accounting.rs +176/−0, tui tests.rs +8/−6 (warm-cache fixture at 25), docs/guides/governance-reports.md (schema-25 sentence), CHANGELOG (2 Fixed bullets).
- **targeted ca60fa5f** — verification-only turn, porcelain identical before/after: core lib 175/0, discovery_contract 78/0, pi_journal_contract 9/0, pi_usage_tree_accounting 18/0, tui 47/0, fmt clean, clippy 0 warnings; digest re-derived with the engine's own validation_policy code.
- **full 6431940f** — ci.yml full+deny lanes taken VERBATIM from the repo's own CI (work order full_command empty; the repo's CI is the authority): ALL 21 gates rc0, cargo test --locked 456/0 across the 3 packages, cargo-deny all-ok (corrected flag order `cargo deny --all-features check`), docs-commands under a PRIVATE AGENTTRACE_CI_OUT (shared-`/tmp/agenttrace-ci` hazard), real-cli-smoke sampled_files=20, check-plugin-version rc0 after the byte-identical 17-line no-changelog-section marker block (third carrier: 250cfd64 → 4a688257 → e486dc1a).
- **Live PoCs (stand as recorded)** — fx1 by_model $0.0165→$0.0075 + provenance.Cost `calculated_from_tokens_with_recorded_cost` + upstream_cost_usd 0.0075; fx2 --doctor zero_usage {sessions:0,events:0}→{sessions:1,events:1} with sample; fx5 saturation control rc0 unchanged. NOTE: `/tmp` PoC corpora are swept between phases (observed fleet-wide) — re-derive fx1/fx2 from the shapes recorded here + the shipped `pi-pi-v3/journal.jsonl` fixture before citing them again.
- **Digest lineage** — `validation:v1:0ca09f019eb888c756ff8e98926c120bbfb2067d89136e1652957f6b62522158` declared VERBATIM at targeted and full, re-derived byte-identical with the engine's own code at both folds: the 9-file delta is crates/** + .md only, never executable-classified, so the digest could not move between folds. This compound adds ROADMAP.md text + this untracked docs/stewardship .md — .md-only again; the declaration stands by construction.

## Roadmap accounting

- Roadmap 7ad6960c minted rm-530/rm-531 (+23/−1; five dated appends on rm-253/rm-239/rm-408/rm-417/rm-306; provenance comment at ROADMAP.md:1493).
- Implement flipped rm-253 candidate→implemented with its EXECUTED bullet; compound appended the c2 banner + the rm-408 residual-rider EXECUTED line.
- rm-530 (service-tier pricing, 78.0) and rm-531 (Cursor recorded cost, 56.0) stay candidates — next-cycle leads; rm-530 deliberately so (needs a full-cycle budget, not a rider slot).
- Done-flip for rm-253 reserved for the commit gate; rm-408 stays done (residual arm closed pre-review — the row is not reopened).
- Post-compound census (this tree): 181 def rows (no mints at compound), 0 duplicate ids, managed render footer untouched.

## Id landscape (live sweep at compound — never a recorded map)

- Landed ceiling: origin/master = **8991144** (212 def rows, max rm-545). b1ff12f8's landing moved the ceiling 1511547 → 8991144 *under this run* — every merge-forward seam below stems from that.
- Campaign-local: our wall max rm-531. rm-530 is double-minted UNLANDED (66e75e39's rm-530 auto-review pricing ≠ our rm-530 service-tier pricing) — renumber by TITLE at integration (880a7b9e discipline). rm-531 is unique across walls + landed.
- In-flight agenttrace walls reach rm-550 (9873fc06) → **fleet next free rm-551**. The spool verb-sweep's rm-553..rm-599 "mints" are the dashboard project's foreign id-space (pnpm rows; verified by repo-hint) and consume no agenttrace numerals.
- rm-529 is itself double-claimed between walls (66e75e39 candidate vs 933058dc implemented) — it is the F4-overflow row our do-not-implement note references; flagged so the integration gate cannot miss it.

## Integration seams for the commit gate (merge-forward mandatory)

1. SESSION_CACHE_SCHEMA_VERSION rebases 24→25 ⇒ **27→28** (the ceiling consumed 25/26/27 via landed lanes); discovery_contract + tui pins rebase; re-prove the tripwire RED-first at the merged tree.
2. Both batch defects remain LIVE at ceiling 8991144 (parser.rs:1850 message arm has no recorded-cost read and landed rm-253 is still `candidate`; `oh_my_pi_usage` is still >0-gated) — the batch lands as net-new value with zero landed overlap.
3. File drift base→ceiling: parser.rs +411/−26, session_cache.rs +120/−1, discovery_contract.rs +101/−8, tui tests.rs +14/−6, governance-reports.md +35/−4, CHANGELOG +33/−1, ROADMAP +327/−4; `pi_journal_contract.rs` and `pi_usage_tree_accounting.rs` are IN-SYNC base→ceiling (clean hunks). Audit the oh_my_pi region of parser.rs hunk-by-hunk at merge-forward.
4. Landed rm-544 ("Honor -f json for the waste report") is an unrelated lane — ROADMAP/CHANGELOG text unions only.

## Prevention rules (per-record lettering, PR-A..PR-E)

- **PR-A — Dead-attempt forensics before adoption, again.** Both of this cycle's dead attempts (implement 79fc974a, compound 2d11ea18) died of provider infra with status-pings-only event logs. Typed-artifact absence ≠ no work, so ALWAYS read the event log AND diff the tree against the previous phase's recorded end state before redoing. Both times here the tree was identical → redo from scratch with a clean conscience (same shape as 555a174d's f3fcafd5 precedent).
- **PR-B — The classifier artifact must not be read as "nothing changed".** Engine `changed_testable_surfaces=[] / required_scope=none` for an agenttrace delta is the known classify_surface gap for crates/** layouts. At targeted scope, validate the REAL git delta (9 files) against the suite selection and say so in the result; never let the empty derivation shrink the lane.
- **PR-C — validation_digest needs the FULL base sha.** The abbreviated form silently produces a DIFFERENT digest (wrong-length input is not an error). This run's targeted + full folds both re-derived byte-identical only after switching to the 40-char sha.
- **PR-D — The schema-bump tripwire is load-bearing.** The 24→25 bump was caught RED by discovery_contract's pins BEFORE they were updated (`left: Some(25) right: Some(24)`) — the fourth consecutive cycle this pattern has held (22→23, 23→24, 24→25). Any accounting-behavior change must bump the schema, update the pins, and move the tui warm-cache fixture in the SAME delta, RED first.
- **PR-E — Per-tag CHANGELOG gate in tag-complete shared gitdirs.** check-plugin-version's no-changelog-section arm reds once historical tags become visible in the shared gitdir (wholesale tag fetch); remedy = the byte-identical marker block appended by full_tests (carriers 250cfd64 → 4a688257 → e486dc1a). After ANY tag-state change in the shared gitdir, re-check `grep -c 'no-changelog-section' CHANGELOG.md` and re-run the gate (never delete tags; never hand-edit CHANGELOG to un-red).

## Cycle-3 leads (recorded, not chosen)

- rm-530 service-tier-aware pricing + tier disclosure (78.0) — LiteLLM live census: 116/4,472 models carry *_flex/_priority/_batches with up-to-4x spread; ccusage #1813 and codeburn #1616 both shipped tier-aware pricing this week.
- rm-406 unknown-kind census — designated FIRST RIDER if the implement-time collision re-sweep still shows no sibling claim (stale lead-rec from 41263f58 f79ee937; no implementation found this cycle).
- rm-306 DSH ingestion — demand-gate FIRED (451,964 npm DLs/week; reorders ahead of Devin under the row's own rule); corpus-gated at landing (fleet-host corpus sighting or recorded decline).
- rm-531 Cursor recorded-vs-estimated cost — same corpus gate (codeburn #1545: 45x/16x undercount for estimates over chargedCents).
- rm-421 compaction episodes (90.0) — XL scope; sibling-named next-lead on the 16bbd3ae lane.
- DO NOT implement without integration adjudication: rm-239 group-key sanitization (quadruple-claimed unlanded), F4 by_task_type i64 overflow (owned by this campaign's unlanded rm-446 + 933058dc's rm-529 + b1ff12f8's uncommitted fix), rm-417 residual gate arm (bbe84274's rider likely covers it).

## Watch items (context-only, unchanged this compound)

Upstream luoyuctl v0.10.1 tip; the #297..#315 wave fully pre-claimed; #312 drops Gemini CLI (upstream-port/hazard lane decides). Bare `agenttrace` npm name still free. pi-mono v1.0.3 (journal format unchanged). OTel semconv v1.44.0 mainline docs/gen-ai confirms rm-229's premise.
