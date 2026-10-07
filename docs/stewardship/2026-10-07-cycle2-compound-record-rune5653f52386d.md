# Cycle-2 compound record — run e5653f52386d (repository-maintenance 546703d4, cycle 2)

| field | value |
|---|---|
| run | e5653f52386d40b0bff8a1ed0b47ea4a |
| compound attempt | 643857ae93014452a87da2f17f3ffac3 |
| worktree | conductor-worktrees/agenttrace-80c75f65b7/run-e5653f52386d-e5653f52 |
| base | 89911442173d31f4eb21a1968bfd51ad46e32ed1 (HEAD throughout; never committed) |
| batch | "usage-accounting truth (upstream #312 port, fork-adjudicated)" = rm-616 (LEAD) + rm-617 + lru rider |
| status | implemented, uncommitted, PRE-REVIEW (review/commit/push/pr/ci remain future phases) |
| ids minted at compound | ZERO (campaign-local rm-616..rm-621 stand; renumber BY TITLE at integration) |
| test execution at compound | NONE (compound contract — recorded outcomes consumed as evidence) |

## 1. Phase ledger (attempts, outcomes, dead-attempt forensics)

| phase | attempt | outcome |
|---|---|---|
| assess | a8b0c008 | DEAD — provider reap, 25 events, no payload, nothing adoptable → redone |
| assess | 9797ba43 | SUCCEEDED — 9 findings, 5 live PoCs, envelope 463/0 + clippy rc0 at 8991144 |
| research | ec0d006c | SUCCEEDED — upstream fetched live (tip still 15ed07f2, 8th frozen check); 3 net-new + 12 folds |
| roadmap | d470f545 | SUCCEEDED — minted rm-616..rm-621 campaign-local; 212→218 def rows |
| prioritize | efdc9df9 | SUCCEEDED — selected the rm-616 LEAD + rm-617 batch (twin map vs sibling 91833f02 recorded) |
| stewardship | 07faf4f8 | DEAD — 4-event reap, no PhaseResult → redone |
| stewardship | 8b3d6343 | SUCCEEDED — batch described; five parser arms + schema pins verified live pre-dispatch |
| implement | 58ae14b8 | SUCCEEDED — 8-file uncommitted delta, +480/-152, 20 in-code adjudication anchors |
| targeted_tests | 68648a6c | SUCCEEDED — core 325/0 + tui 47/0 + static rc0; digest replicated; run-only turn |
| full_tests | c4724f98 | SUCCEEDED — 21/21 ci.yml-mirrored lanes rc0; run-only turn (tree census byte-identical) |

**Dead-attempt forensics (the zero-durable reap arm, corroborating the fleet rule):**
assess a8b0c008 (event log ends mid-turn, family provider, no findings payload) and stewardship
07faf4f8 (exactly 4 events: turn_started → progress tick → session_reaped/failed →
turn_completed) both left NO typed artifact and NO tree drift. Both phases were redone from
scratch and adopted nothing. Signature for adopt-vs-redo: typed result JSON absent + pings-only
event log + porcelain unchanged ⇒ redo, never adopt.

## 2. What was implemented (pre-review, from implement 58ae14b8)

Ported from upstream cb625d7 (upstream #312, read live via `git show`), with per-arm fork
adjudication documented in-code (20 rm-616/rm-617 anchors; 12 on the codex lane in parser.rs).

- **rm-616 arm 1 (claude)**: `usage_by_message BTreeMap<message-id, event-index>` replaces the
  JSON-embedding dedup key — one meta usage event per message id, MAX-per-field across streamed
  rows (parser.rs ~:3160). Kills the ~1.9× input over-count on streamed journals.
- **rm-616 arm 2 (qwen)**: `first_number` over input aliases (no more `sum_numbers` mirror
  double-count) + cache-read netted out of input per the codex decomposition (~:2426).
- **rm-616 arm 3 (opencode)**: `add_opencode_tokens` folds `tokens.reasoning` into output,
  matching the sqlite path (~:4205).
- **rm-616 arm 4 (kimi)**: `parse_kimi_value` single-counts `usage` XOR `metadata.usage`
  (top-level preferred, nested fallback, ~:3389).
- **rm-616 arm 5 (schema)**: SESSION_CACHE_SCHEMA_VERSION 27→28 with the four-pin chain aligned
  (session_cache.rs:8 + dated convention comment; discovery_contract.rs :991/:1163 `Some(28)`;
  agenttrace-tui tests :1634/:1647; governance-reports.md:72 — the sentence the docs gate greps).
  SQLite snapshot schema stays 7.
- **rm-617 (codex)**: `CodexTotals{prev, seen: BTreeSet}` counts `last_token_usage` once per
  DISTINCT cumulative with positive-delta fallback vs the immediately-previous cumulative; the
  rm-162/#286 high-water port (`token_usage_high_water`) is DELETED. `reasoning_output_tokens`
  is a breakdown, never an addition. Gemini keep-and-own documented at `parse_gemini_value`
  head (:3757; upstream deleted their gemini parser — the fork solo-owns the lane).
- **lru rider**: Cargo.lock-only 0.18.1→0.18.5 (RUSTSEC-2026-0253; hashbrown 0.16.0 in-lock
  transitive refresh; lru has no Cargo.toml declaration — transitive only).

**Tests**: 6 new red-first pins (claude streamed fold; qwen mirrored-aliases-count-once +
cache-net; opencode reasoning-as-output; kimi usage XOR metadata incl. metadata-only fallback;
codex reasoning-breakdown pin {in 10/out 100/reasoning 40}→110; codex distinct-snapshot dedupe
with delta fallback) + `codex_usage_sums_saturate` rewritten to pass-through + the rewind test
folded into the dedupe pin. **Pin moves BY DESIGN** (adjudicated semantics, not regressions):
codex_compaction_verdict 1700/800/620→2300/800/600 (real remote compaction RESETS the
cumulative — the old high-water silently refused fresh post-reset usage; the old test comment
itself called it "its own follow-up class"); discovery_contract codex output 190→160, qwen
stream input 120→110 (net-of-cache); in-parser compaction pin → 870 in/630 cache/500 out;
unpaired-record shape 1910→1720.

## 3. Recorded validation outcomes (consumed at compound; NOT re-run)

- **implement 58ae14b8**: workspace 468 passed / 0 failed across 21 suites (baseline 463,
  +5 net); `cargo fmt --check` rc0; `cargo clippy --offline --workspace --all-targets --
  -D warnings` rc0; `scripts/ci/check-docs-commands.sh` rc0 (after release build). Live PoC
  replays from the assess corpora: F3 codex TOKENS 150→110 (true); F4 qwen Input 200→40 /
  Cache read 60 netted / Output 50; F5 codex3 hit-rate 200.0 unchanged (net basis pre/post);
  NEW claude streamed-rows e2e → TOKENS 80; HOME-isolated schema-28 journal write + warm serve
  verified live.
- **targeted_tests 68648a6c** (scope declared 'targeted' per the fold contract — the dispatch
  block's `changed_testable_surfaces=[]` / `required_scope=none` is the known classifier
  artifact for the crates/** layout, and a real delta exists): agenttrace-core 325/0 across 13
  suites; agenttrace-tui 47/0; clippy/fmt/docs rc0; live re-checks codex 110, qwen 40/50/60,
  claude 80. Digest `validation:v1:a72443d225dc8d687e34505eb328bd65608884abc60d932d59f83678062b7b0c`
  replicated byte-exact from the full-40-char base sha.
- **full_tests c4724f98**: dispatch `full_command` EMPTY → `.github/workflows/ci.yml`
  (lint+full+deny jobs) mirrored lane-for-lane in CI step order; **21/21 executed lanes rc0**
  (~13.5 min; heavy cargo lanes through local_validation_gate.py at host load1 ~22). Step-3
  cargo test 468/0 across 21 suites; entrypoints 31/0; cargo-deny advisories/bans/licenses/
  sources all ok; documented skips = the CI-condition-gated `git fetch --tags` pre-gate
  (delegates never fetch; check-plugin-version itself ran green) + TUI real smoke (repo var
  unset). Run-only turn proven: pre/post tree census byte-identical (HEAD unchanged, same 8 M
  files, `git diff` sha256 698f1cb0be5d202c24c85dc9286852129eb2b989c041311c87f4b726f70573e9),
  digest re-derived live == dispatch digest.

**Digest lineage**: `validation:v1:a72443d2…62b7b0c` declared VERBATIM by targeted_tests and
full_tests; nothing executable moved after implement; compound touches ROADMAP.md and
docs/stewardship/ only (non-executable), so the digest is unchanged by this phase.

## 4. Reusable lessons / prevention rules (this cycle's deltas)

1. **Upstream-port adjudication pattern** (worked end-to-end here, reusable for rm-618):
   read the upstream commit live (`git show`) → map each arm onto the fork's live state →
   adjudicate per arm (adopt / keep-with-rationale / replace) → document every decision as an
   in-code anchor citing the row id → land red-first fixtures pinning the assess PoC numbers
   BEFORE the fix logic. The red-first pins made the "pin moves BY DESIGN" defensible at
   review time instead of a judgment call.
2. **Pin-move discipline**: when a semantics fix legitimately moves existing test pins, the
   moved numbers must be explained in the test itself (the compaction ladder table in
   codex_compaction_verdict.rs is the model) — otherwise review reads a regression.
3. **Zero-durable reap forensics** (2 more instances, now corroborated fleet-wide): typed
   result JSON absent + pings-only event log + porcelain unchanged ⇒ redo the phase from
   scratch; never adopt, never remint from the corpse.
4. **Empty `validation.full_command`** is the standing dispatch shape for this repo → command
   authority is ci.yml lint+full+deny mirrored lane-for-lane; the deny lane must be invoked as
   `cargo deny --all-features check` locally (0.20.2 flag order).
5. **crates/** classifier artifact**: dispatch blocks derive zero changed surfaces for
   under-crates deltas → the fold contract requires declaring 'targeted' (never 'none') when a
   real delta exists.
6. **/tmp lifecycle**: repro sandboxes are NOT durable across phases — /tmp/at-assess-e5653
   and /tmp/at-research-e5653 were reaped between full_tests and compound. Durable replay
   lives in the landed red-first tests; anything a future phase must replay should be turned
   into a test or copied under the delegate spool, not left in /tmp.

## 5. Cycle-3 context (concrete candidates, next maintenance cycle)

- **rm-618 LEAD** — copilot OTEL attribute-name deltas + chat-span usage summation from
  upstream #312, boundary-disciplined against landed rm-485's credit lane (cost =
  max(refined token estimate, credit_usd) must stay untouched); fork otel.rs pinned names need
  the diff read at implement.
- **NEW un-minted candidate (implement finding, reach-corrected by independent_review 3a0e)**:
  `parse_raw_session` runs the generic gemini value probe BEFORE `parse_kimi_value` (parser.rs:205)
  and `gemini_usage` (parser.rs:4560) returns Some for ANY object — every alias read defaults
  to 0 — so `parse_gemini_value` claims ANY doc carrying top-level usage|usageMetadata|
  tokenUsage regardless of alias match: the review probe showed a kimi session.json reporting
  SOURCE gemini_cli with TOKENS 0. The kimi red-first test pins `parse_kimi_value` directly
  because of this. Mint as dispatcher family-detection-before-generic-probes at the next
  roadmap phase.
- **rm-619** — order-robust custom_tool_call failure accounting (CLEAR 460d0633's in-flight
  arms first — sibling claim hazard).
- **rm-620** — per-session cost explanation lens; **rm-621** — Command Code watch (stays
  watch-only until a real corpus lands; do not implement on speculation).
- **Folded evidence riding existing rows** (no new mints): assess F1 (waste ESC leak) + F2
  (-o clobbers an input journal) corroboration at d80f6a25's rm-533..536 merge gate; F5
  (hit-rate mixed denominators) on rm-339's clamp arm; F7 rider on rm-543; F8/F9 riders on rm-542.
- **Integration seams for the commit gate** (carried from prioritize/implement): twin-collapse
  against sibling 91833f02's uncommitted rm-616..rm-619 band (their rm-617 codex-double-add /
  rm-618 qwen-sum / rm-619 opencode-reasoning are TITLE-TWINS of this batch's arms) —
  renumber BY TITLE, never by id; schema-28 slot re-check vs any 36c9140f-lineage rm-529
  claim; CHANGELOG riders land at commit (five-arm Fixed entries + lru 0.18.5/RUSTSEC-2026-0253
  note on rm-044); origin/master movement since base requires re-anchoring file:line citations
  at merge (row anchors were live-verified at 8991144).

## 6. Verification & hygiene (at compound)

- ROADMAP.md: 218 def rows (unchanged), zero duplicate ids, managed footer still the last
  line; compound layer is pure additions on the uncommitted band (numstat +57/-0 over the
  roadmap band's +52); rm-616 (:1819) and rm-617 (:1828) flipped candidate→implemented with
  EXECUTED bullets; rm-618..rm-621 untouched as candidates.
- Status accounting after flips: 132 candidate / 37 done / 49 implemented (was 134/37/47).
- Banner discipline: compound banner sits at the top of the "Open items" block (newest-first)
  with the `>` blank separator preserved before the cycle-2 roadmap banner.
- Worktree porcelain at compound end: the implement batch's 8 M files + this record
  (untracked docs/stewardship file) + ROADMAP.md carrying both bands. No tracked file outside
  the batch was touched; zero test/validation commands executed this phase.

## 7. Review-fix addendum (independent_review 3a0e → fix 9f7e21d2, 2026-10-07)

The independent review (verdict NEEDS_CHANGES, report /tmp/at-review-3a0e/independent-review-report.md)
found one blocker plus three riders. All four addressed this turn:

- **F1 HIGH — FIXED red-first** (parser.rs, rm-616's claude arm): the fold registered
  `usage_by_message[id] = events.len()` before the conditional `usage_from_value` push, so a
  first streamed row with present-but-unparseable usage pointed the slot at whatever landed
  next — the row's own assistant event (later usage rows folded into a non-meta event and the
  analyzer's meta-only usage fold silently dropped them: probe Input 0 vs true 75) or one
  past the end of a still-empty list (a later usage row panicked `index out of bounds` at
  parser.rs:3186). Fix: register the slot only for a just-pushed meta event (events.len()-1),
  matching upstream #312's ordering. Red-first pins `claude_fold_never_indexes_one_past_the_last_event`
  and `claude_usage_always_lands_on_a_meta_event` failed pre-fix at exactly the probed shapes
  (panic at parser.rs:3186:54; meta count left 0 / right 1) and pass post-fix; the original
  streamed pin stays green (happy path unchanged).
- **F2 LOW — disclosure re-anchored** (this record §5 + the ROADMAP banner): gemini_usage
  returns Some for ANY object, so parse_gemini_value claims any usage-bearing doc regardless
  of alias match (probe: kimi session.json → SOURCE gemini_cli, TOKENS 0). Code fix deferred
  to the next cycle's mint, per the original disposition.
- **F3 LOW — comment re-anchor** (discovery_contract.rs, both Some(28) sites): the stale
  'schema-27' prose above the pins now names v28.
- **F4 LOW — CHANGELOG authored in-batch** (CHANGELOG.md Unreleased/Fixed): the five-arm
  Fixed entry + the lru 0.18.5/RUSTSEC-2026-0253 rider; the commit gate carries them
  verbatim, eliminating the drop risk.

Validation this turn (targeted scope, dispatch `required_scope=none` is the crates/**
classifier artifact and executable surfaces DID change): fmt rc0, clippy --workspace
--all-targets -D warnings rc0, cargo test --workspace 470/0 (468 + the two new pins), and
the reviewer's live probes replayed post-fix from the surviving /tmp/at-review-3a0e corpora
(panic corpus clean, misfold corpus TOKENS 75, kimi probe unchanged — pre-existing F2 —,
v27 schema-regen probe unchanged, claude streamed e2e TOKENS 80 unchanged).
