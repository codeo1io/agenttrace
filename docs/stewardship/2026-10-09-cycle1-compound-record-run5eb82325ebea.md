# Cycle 1 compound record — run 5eb82325ebea4a34b41289d7bdb5dcc0 (repository-maintenance `8f368563856e46fcb526aa5430f974ac`)

Cycle 1, compound attempt `c8f556e8b6324bd9b9b1dfb1c23a1293`, 2026-10-09. Worktree
`run-5eb82325ebea-5eb82325` at base `5ed9ebc`; porcelain at compound = exactly the implement
batch's 17 paths (11 modified + 6 untracked, +591/−160 tracked, +508 untracked lines). This
phase touched nothing in-tree: the roadmap/record delta ships as the spool patch chain —
`delegate/d631be06…-scratch/roadmap-delta-d631be06.patch` (mint band) first, then
`delegate/c8f556e8…-scratch/roadmap-delta-c8f556e8-compound.patch` (this phase; it also
creates this record). Phase contract honored: **no test or validation command was executed at
compound** — every outcome below is consumed from the recorded implement (b20465be +
re-attestation c8233f19), targeted_tests (e0f5df6f), and full_tests (876abf64) results.

## Prior-attempt forensics this cycle (three dead attempts, zero lost work)

- **assess 5747ba6e** — reaped at message_count 3 (4 event lines: delegate_turn_started →
  progress(msg 3) → session_reaped(failed) → completed(failed)), no typed artifact, no scratch
  dir → redo-from-scratch declared and performed at f3529893.
- **research 94a78653** — provider-dead at ~4.5 min (heartbeats only, 3 × ~90 s), no typed
  artifact, no scratch dir → redo-from-scratch declared and performed at ef1ca6d9.
- **implement b20465be** — NOT a reap: the batch completed and was fold-rejected SOLELY for a
  missing KTD13 changed-surfaces attestation ("the engine-derived tree delta holds 1 changed
  executable surface(s)"). The delta was frozen (mtimes 19:36:48–19:59:12, zero writes for ~90
  min before the re-attestation turn); c8233f19 verified it end-to-end and re-emitted with
  `changed_surfaces` = all 17 paths. Nothing was redone — the correct disposition for a
  complete-but-unattested delta (a partial or stale delta gets redone).

## Shared-worktree foreign-write anomaly (disclosed, adopted)

Two completion-stage files (usage_occurrence_contract.rs, tui tests.rs) carried the batch's
intended edits at 19:36:48/19:38:52 — BEFORE the attempt's first owned write (19:47:31) —
evidence of a parallel/earlier attempt sharing this run worktree. Content was verified
line-by-line against the batch plan (schema-41 oracle + TUI planted fixture) before adoption;
reverting would have re-created the planted-fixture red. Adopted with disclosure.

## What compounded (pre-review; done-flips stay reserved to the commit gate, rm-012 convention)

Batch 'intake → attribution → disclosure truthfulness' — selected at prioritize 342c3a38,
described at stewardship 4b085767, implemented at b20465be (re-attested c8233f19):

- **rm-863 (LEAD)** reasoning-fold producer semantics — `ReasoningKeySemantics{Sibling,Subset}`
  at parser.rs:5871/:5878/:5891, applied at usage_from_value_with_keys + qwen_usage +
  gemini_usage: sibling aliases stay additive; flat OpenAI-compatible reasoning/thinking
  tokens set the breakdown without adding (the generic arm seeds only when the doc records
  no output-class member; the structured arms read output via first_number, so a
  present-but-zero member is also seeded — pinned by
  subset_seeding_is_pinned_at_the_present_but_zero_corner, review b0446d6c F4). PoC 400 vs pre-fix
  550 (+37.5%), sibling control 550, discovery pin 96/0. Full EXECUTED bullet on the row.
- **rm-860** duplicate-spelling normalization — EVENT_FIELD_SPELLING_PAIRS (all 11 pairs,
  lib.rs:332) + normalize_duplicate_field_spellings (:356), canonical-winner rule, both JSONL
  and JSON-array arms as strict-parse-failure retry; disclosed informational
  duplicate_spelling_normalized, never a line_skip. PoC 1/1 parsed with the counter, no
  event_schema drop.
- **rm-861** model-label attribution — session_from_events label scan restricted to
  authoritative rows (lib.rs:946-952 scan; :868 was the pre-batch signal site — corrected by
  review b0446d6c F2); non-authoritative rows may seed off 'default', never
  relabel; ignored relabels disclosed model_label_non_assistant_ignored. PoC keeps
  By-Model claude-opus-4-1-20250805.
- **rm-862 (rider)** doctor vocabulary split — DoctorReport separates the loss tier from
  informational disclosures across all three fold lanes (doctor.rs:1020 'Dropped lines (parse
  loss):' vs :1038 'Disclosed facts (informational):'); resolved beyond the doc-only
  acceptance with an executable change, retest escalated to full.
- **Same-unit rider set** — SESSION_CACHE_SCHEMA_VERSION 38→41 at implement, re-based to 43 by
  the cycle-1 review fix (review b0446d6c F1: the origin ceiling moved while the batch was
  in review — 41 at the ac14e52c integration 9c3c599, 42 at the rm-880/rm-406 journal-truth
  integration run 14954d7a 181630e; never-downgrade rm-230 rule, planned as 39 at mint)
  and to 50 by the review-fix rebase (attempt 7166bfbb: the landed ceiling still stood at
  42 at eafbb592, but sibling run 33b7b7b5's in-flight batch minted 43 — a 43-vs-43
  collision — and run 2a5cdb9e held 49; max observed + 1): const+ladder,
  usage_occurrence_contract oracle, TUI planted fixture, governance-guide sentence, and the
  check-docs-commands.sh digit-prefix regex port ('schema 4([^0-9]|$)', :81) so the truthful
  'schema 50' sentence cannot false-fail the docs lane (the gate's primary schema check
  extracts the const from source and requires the guide to state it verbatim).

## Recorded outcomes (pre-review; consumed, NOT re-run)

- **implement**: targeted core 284/0 (lib 243 + intake_attribution_truthfulness 13 +
  otel_export 4 + parse_torn_tail_contract 4 + pi_usage_tree_accounting 15 +
  usage_occurrence_contract 5), tui 52/0, fmt clean, clippy --workspace --all-targets -D
  warnings clean, docs-gate rc0 (after a release rebuild — target/ had been wiped inter-phase);
  red-first proven on all four arms.
- **targeted_tests e0f5df6f**: dispatched targeted_command verbatim, rc0 — the impacted
  runner printed "unable to prove narrow test scope; falling back to authoritative full
  validation" (a shell-script surface has no narrow lane) and ran the full gate: 38 suites
  696/0.
- **full_tests 876abf64**: full_command VERBATIM, rc0 — 38/38 suites ok, Σ 696/0/0 in 1021.5 s;
  validation digest `validation:v1:f202a556…` declared verbatim and re-derived byte-identical
  by importing the engine's own validation_policy at base 5ed9ebc (the gate envelope's
  `validation:v1:cf61706a…` / digest_base=unknown is the KNOWN envelope/engine mismatch, not
  the fold anchor).
- **Growth accounting**: 696 = the 683 assess-era baseline + exactly the 13 new
  intake_attribution_truthfulness tests; zero test deletions.

## Cycle-1 review fix (attempt 53c6d3ac, 2026-10-09, over review b0446d6c)

Review verdict was NEEDS_CHANGES: one HIGH + four LOW findings; all five fixed this turn
tree-side at the same 17-path uncommitted delta (porcelain identical before/after the
retest — same 17 paths, HEAD unmoved at 5ed9ebc):

- **F1 (HIGH)** schema-41 collision — the origin ceiling moved twice during the review window
  (41: ac14e52c `9c3c599` 2026-10-08 21:24:30Z; 42: rm-880/rm-406 journal-truth, run 14954d7a
  `181630e` 2026-10-09 05:51:01Z, ls-remote-verified tip). The same-unit set re-based 41→43:
  session_cache.rs const + ladder comment, usage_occurrence_contract oracle (+ message),
  TUI planted fixture (+ ladder comment), governance-guide sentence (the docs gate compares
  dynamically and follows). RED-FIRST: with only the constant bumped,
  warm_cache_replays_cold_disclosure_and_schema_38_invalidates_v32 FAILED (41≠43) and tui
  ctrl_r_force_reload_clears_session_cache_before_loading FAILED (cache_hits 0≠1 — planted
  fixture stale); green after the sweep.
- **F2 (LOW)** rm-861 addendum cited the pre-batch :868 scan site — corrected to the live
  :946-952 in the ROADMAP addenda and this record.
- **F3 (LOW)** governance-reports.md now states the seed-off-default exception beside the
  authority rule (a non-authoritative row may still SEED the label while it reads `default`,
  so lanes that stamp the model only on tool rows keep their attribution).
- **F4 (LOW)** SUBSET seeding seam pinned at the present-but-zero corner by the new
  subset_seeding_is_pinned_at_the_present_but_zero_corner (parser.rs): the generic arm's
  entry().or_insert leaves an explicitly reported zero at 0, while the qwen/gemini arms seed
  it (first_number collapses absent to 0). Red-first on both arms by mutation (never-seed on
  the qwen arm; unconditional overwrite on the generic arm), each restored md5-identical.
- **F5 (LOW)** inert `#[allow(clippy::too_many_arguments)]` removed from the
  DoctorDisclosureFold struct (doctor_directories dropped to 7 params with rm-862's bundling);
  clippy -D warnings still rc0, so nothing was being masked.

Retest (dispatched targeted_command VERBATIM → full fallback): 38 suites 697/0 (+1 = the F4
pin test; growth accounting 696→697), fmt clean, clippy clean, docs-gate rc0 after a release
rebuild; emission-time digest re-derived `validation:v1:155774ee…` via the engine's own
validation_policy at base 5ed9ebc. Prior-attempt forensics for this action: 96af27d0
(provider-dead ~18 min in, 18 events) left NO durable worktree writes (all 17 paths' mtimes
predate its 05:31–05:49Z window; the only newer mtimes are the reviewer's red-swap restores
at 02:30Z) and no result envelope/scratch — redo from scratch, performed here.

### Review-fix rebase over the moved in-flight wall (attempt 7166bfbb, 2026-10-09)

The fold rejected 53c6d3ac solely on retest-evidence scope (declared `targeted`; KTD5/KTD6
require `full` over the run's uncovered risk-relevant delta — the targeted runner's fallback
did execute the full gate, but the declared scope is what the fold reads). This attempt
verified all five fixes of review b0446d6c still in-tree (fresh greps, no re-derivation
needed), re-censused the schema wall at gate time, and re-ran validation at FULL scope:

- Live census: origin/master tip eafbb592 (ls-remote-verified; the file's last landing is
  181630e) carries 42 — the landed ceiling is STILL 42 — but the in-flight wall moved past
  this batch: sibling run 33b7b7b5's uncommitted tree minted 43 (43-vs-43 collision: the
  same number would carry two different cached-totals formats) and run 2a5cdb9e's tree
  holds 49. Max observed + 1 → same-unit set re-based 43→50 (rm-230 never-downgrade).
- Sites re-based together: session_cache.rs const + ladder comment,
  usage_occurrence_contract oracle literal + ladder message, TUI planted fixture
  (schema_version 50) + ladder comment, governance-guide sentence + ladder prose. The docs
  gate follows the const dynamically and passed rc0 after a release rebuild.
- RED-FIRST re-proven at 43→50: const-only bump → oracle
  warm_cache_replays_cold_disclosure_and_schema_38_invalidates_v32 FAILED (left 50,
  right 43) and tui ctrl_r_force_reload_clears_session_cache_before_loading FAILED
  (cache_hits 0≠1, planted fixture stale); green after the sweep (oracle 5/0, tui 52/0).
- RETEST AT FULL SCOPE: dispatched full_command VERBATIM (envelope-string sha256
  a0e9a315ab1c514f2eddf87e7c3a7ea5621ee1f6ad828b25faeec1d62cd9db91) →
  local_validation_gate 'cargo test' rc0: 38 suites ok / 0 failed, 697 passed / 0 failed
  (machine-summed). fmt clean, clippy --workspace --all-targets -D warnings rc0, release
  build rc0, docs gate rc0.
- Emission-time digest re-derived via the engine's validation_policy at base 5ed9ebc:
  `validation:v1:155774ee…` — byte-identical to the dispatch token (this attempt changed
  only .rs/.md surfaces; nothing executable-classified moved).

## Status flips and id landscape

- Flips: rm-860/861/862/863 candidate → implemented, dated EXECUTED bullets at each row's true
  end. Postimage census: 319 def rows = 167 candidate / 82 implemented / 70 done (mint census
  was 171/78/70). ZERO mints at compound.
- Numeral seam: fresh compound-time census (2026-10-09 01:3xZ) — rm-86[0-3] def-row claims
  exist ONLY in the d631be06 postimage (whole-spool def-row grep + every agenttrace worktree
  ROADMAP.md); sibling unlanded frontier through rm-859 (d0ecd354/81b107b7 band, base 65f9f63).
  Reconcile BY TITLE at every merge, landed-first precedence (880a7b9e convention); next free
  rm-864 after a fresh live census.

## Lessons / prevention rules

1. **KTD13 prefix-blindness**: the engine's executable-surface classification is prefix-based —
   crates/** classify non-executable, so a Rust-only delta is invisible to the attestation gate
   while a scripts/ rider trips it. Emit the full changed-surfaces attestation unconditionally;
   b20465be's fold rejection was purely this.
2. **Inter-phase target/ wipes** make `cargo build --release -p agenttrace` part of any
   docs-gate budget — check-docs-commands.sh needs the release binary and returned rc1 on a
   wiped target/ twice this cycle (implement, and again observed at full_tests).
3. **The gate envelope digest is not the fold anchor** (digest_base=unknown vs base=HEAD):
   re-derive via hermes_conductor.validation_policy at base=HEAD when in doubt — this cycle it
   confirmed the dispatch token byte-identically.
4. **Schema re-base over the live origin ceiling, RE-VERIFIED AT EVERY GATE** — the ceiling is
   a moving wall in a multi-run fleet: minted 39, re-based to 41 at implement, re-based to 43
   at review because 41 (ac14e52c 9c3c599) and 42 (rm-880/rm-406, run 14954d7a 181630e)
   landed during the review window, and re-based to 50 at the review-fix rebase
   (7166bfbb) because the IN-FLIGHT wall — not just landed origin — passed this batch's
   number (sibling run 33b7b7b5 minted 43; run 2a5cdb9e held 49, landed ceiling 42 at
   eafbb592): the census must sweep sibling worktrees' uncommitted consts, not only the
   landed origin ceiling — the compound census checked rm-numerals but NOT the
   schema constant (review b0446d6c F1). The same-unit literal
   sweep (oracle, fixture, guide sentence, ladder) + the docs-gate regex port must land as ONE
   unit; a partial landing re-creates the planted-fixture red or a false-failing docs lane.
5. **Dead-attempt triage**: verify the events jsonl (message count, typed artifact, scratch
   dir) before redoing; and distinguish reaped attempts (redo) from complete-but-unattested
   emissions (re-attest the frozen delta — do not redo).
6. **Shared-worktree foreign writes**: adopt only after line-by-line verification of the
   content and mtime forensics; disclose. (Third fleet occurrence of shared-worktree anomalies
   this month.)
7. **Assert on counters, not the confidence label**: 'confidence: low' on a hermes_jsonl-lane
   fixture is correct lane classification (usage_present_not_counted, the rm-616 boundary), not
   a regression.
8. **Semantics pre-decided at prioritize** (sibling-vs-subset table came from research PoCs)
   kept implement mechanical with zero mid-flight design churn — keep that division of labor.

## Commit-gate seams

- (a) Land order: d631be06 mint patch → c8f556e8 compound patch (creates this record) →
  53c6d3ac review-fix rider patch (amends this record + two addenda) →
  7166bfbb review-fix-rebase rider (same-unit set 43→50 over the moved in-flight wall,
  amends this record + the rm-863 addendum) →
  done-flips for the 4 rows ONLY after review passes.
- (b) Stage all 17 paths together — the schema-50 same-unit set + the docs-gate regex rider
  land atomically.
- (c) The 6 untracked files need explicit `git add` (commit -am drops them).
- (d) CHANGELOG Unreleased Fixed bullets for the four rows are NOT yet in the delta — ship
  them at commit.
- (e) The ship digest after the review fix is `validation:v1:155774ee…` (re-derived at base
  5ed9ebc over the review-fixed tree via the engine's own validation_policy; the pre-review
  `f202a556…` token described the pre-review tree) — the ROADMAP/record
  chain is md-only and digest-immobile (the .sh rider is already inside the declared tree).
- (f) NEW SEAM (review b0446d6c F1): re-verify the live origin schema ceiling immediately
  before the commit (git log origin/master -- crates/agenttrace-core/src/session_cache.rs +
  git ls-remote, the 5417681937ae cycle-3 convention) and re-base the same-unit set one slot
  if the number is taken — the ceiling moved TWICE during this cycle's review window.

## Next-cycle leads (ranked)

1. Wall tops rm-195 / rm-251 / rm-421 @90.0 — verify-first (the standing trio).
2. rm-617 @88.0 codex reasoning double-add arm — verify its wall-status seam before claiming
   (research ef1ca6d9 treats the fix as landed at its lane; this wall still shows candidate);
   the GENERIC fold is now rm-863's and implemented.
3. rm-239 @87.0 text-asset ledger; rm-618 @87.0 input-arm synonym-sum (disjoint from rm-863's
   output arm — title-twin reconcile with the rm-617 family).
4. rm-232 @86.0; rm-709 @86.0 (4c34ecaa's unlanded band — re-census whether it landed before
   any work).
5. F4/F5/C2/C3 folds ride their owning lanes' landing gates (F4→rm-700, F5→rm-858, C2→rm-455
   rider, C3→rm-588/rm-164 riders — all recorded in the d631be06 banner).
6. Watches carried: pi v1.1.0 additive-only (spec-watch untripped); serde_json 1.0.151 float
   defect still unfixed (pin posture holds); upstream 15ed07f2 ninth consecutive zero-delta.
