# Cycle 2 compound record — run 91833f02 (2026-10-07, pre-review)

- **Run:** `91833f02565b477d9e2e9505062967a4` (repository-maintenance `b5aab57c4a8c4b36a6c71f9f27eb58e4`, cycle 2)
- **Base:** HEAD `89911442173d31f4eb21a1968bfd51ad46e32ed1` (unchanged all cycle; the whole batch is an uncommitted worktree delta in `run-91833f02565b-91833f02`)
- **Batch:** "Generic-lane model/usage truth" — single lead unit **rm-616** (prioritize ba83719b)
- **Status at compound:** implemented, pre-review. Review and shipping happen after this step; the next cycle's assessment carries them forward. Gates recorded, not re-run — compound is validation-inert by contract.

## Attempts and forensics

| Phase | Attempt | Note |
| --- | --- | --- |
| assess | 4502dfdc | adversarial @8991144: F1 Event.model_used alias gap (MED-LOW) PoC-proven; first TUI pty injection probe (render lane clean); battery 463/0 + fmt/clippy/deny rc0 |
| research | c5ec22b8 | upstream 12-commit wave (#305-#318) as differential matrix; R2 codex reasoning double-add / R3 qwen synonym-sum / R4 opencode reasoning-subtree drop → minted rm-617/618/619 |
| roadmap | b750e6ab → **1637d6e8** | b750e6ab provider-reaped mid-turn (event log ends session_reaped/failed, no phase_result, typed artifact absent, zero durable work — porcelain 0 at redispatch) → phase REDONE from scratch by 1637d6e8: minted rm-616..rm-619 (+37/-0 ROADMAP) |
| prioritize | 49a0a5b0 → **ba83719b** | 49a0a5b0 provider-reaped at +28s (4 events, no result, no scratch) → redone by ba83719b: selected single-unit batch rm-616 (correctness 66.0); twins mapped, spool-wide model_used grep (37 hits all adjudicated unrelated) |
| stewardship | f6661378 → **de04b72e** | f6661378's typed artifact is the product of a COMPLETED phase_result_repair turn (repair-completed then transport-reaped) — ADOPTED by de04b72e after full identity/census verification; de04b72e added verification legs only |
| implement | 4bbffa8b | rm-616 all three lanes, red-first (7 failed → 7 passed); left rm-616 status:candidate deliberately (done-flip reserved to commit gate rm-012) |
| targeted_tests | bcf901c4 | validation-only; suite 7/0, core 327/0, fmt/clippy rc0, live PoC PASS incl. native no-regression |
| full_tests | 248750270124458696cc3e19c1ff7501 | 21/21 CI lanes rc0 (empty `full_command` → ci.yml mirror per engine skip-clause + ff0068ca/614624d7/18b1e331 precedent), 470/0 = base 463 + 7 |
| independent_review | f6b98bc3 | VERDICT: NEEDS_CHANGES — 4 findings, PoC-proven: F1 (high) stale v27 session cache masks the fix; F2 (medium) this record's false no-bump rationale; F3 (low) `usage_present_not_counted` suppressed beside meta usage; F4 (low) fixed-name /tmp fixtures + unpinned usage_models join. Read-only, zero writes |
| independent_review:fix | 9e2227b4 | all four fixed (see "Review fix" section); targeted retest green |
| compound | e0f6d7d0 → acdf9b98 → **48fecbe9** | TWO provider-dead predecessors: e0f6d7d0 (+83s, 12 msgs, no envelope, typed artifact absent, zero tree drift — ROADMAP delta still exactly the roadmap phase's +37) and acdf9b98 (+29s, 2 msgs, pings-only). Neither was in the dispatch forensics; BOTH surfaced by the compound-time event-log sweep. Nothing adoptable → this attempt redid the phase from scratch |

**Dead-attempt pattern (5 reaped attempts in one run — new high for this fleet's records):** b750e6ab (roadmap), 49a0a5b0 (prioritize), f6661378 (stewardship, salvageable via repair artifact), e0f6d7d0 + acdf9b98 (compound). The reliable discriminator held every time: porcelain census vs the prior phase's recorded census (md5 of `git status --porcelain`) proves zero drift, so the phase is redone from scratch rather than hunted for phantom work — and the event log, not the dispatch forensics block, is the complete attempt registry (the work order named only acdf9b98; the sweep found e0f6d7d0).

## What was implemented (uncommitted delta: lib.rs +115/−9, 2 new test paths)

**rm-616 Generic-lane model/usage truth** (Event model identity and usage silently dropped to text estimation when `model_used` casing mismatched or usage rode conversation lines):

- `crates/agenttrace-core/src/lib.rs` — `alias = "model_used"` beside `rename = "ModelUsed"` on `Event.model_used` (both wire casings now populate the field); `analyze()` gains a generic-lane arm beside the meta gate (conversation-line usage where `source_tool == "generic"` → `provenance.tokens = reported_by_agent`, text estimation stands down) folding with the meta arm's saturating contract and joining `usage_models` for per-block per-model pricing; two new disclosures ride `Metrics.line_skips` — `model_or_usage_dropped` (parse-side: unparseable/`event_schema`/`non_event` rejects that themselves carried model/usage keys) and `usage_present_not_counted` (analyze-side: usage observed outside its family's counted lane while the session estimated); `parse_jsonl_session` now merges its counters into `metrics.line_skips` instead of overwriting analyze()'s.
- Native families deliberately untouched: every native parser arm emits usage on `role:meta` events (pre-checked at implement), so the generic-scoped fold cannot double-count — pinned by `native_shaped_session_keeps_meta_gate_and_discloses_unfolded_usage`.
- `crates/agenttrace-core/tests/generic_model_usage_truth.rs` — 7 red-first tests (pre-change all 7 failed on exactly the arms they pin: model `default` vs `gpt-5`, tokens 0 vs 2, `None` vs `Some(3)`; post-change 7 passed). Both acceptance arms covered: the md5-pinned `c2.json` golden fixture (`gpt-5` 7/3, was `default` 1/1) and `casing_parity_both_spellings_populate_model_used`.
- `crates/agenttrace-core/tests/fixtures/rm-616-generic/{c2.json,c-generic.json}` — verbatim PoC copies (md5 `bd025fbd3b70b9f581b682fa5520b5d2` / `720b8f3992522f44ab9afd496ee5bce5`, byte-matched at compound against the implement record).

## Recorded validation outcomes (pre-review; consumed at compound, NOT re-run here)

- **targeted (bcf901c4):** validation-only turn, tree byte-identical before/after; `generic_model_usage_truth` 7/0; core 327/0 (lib 184 + 12 integration suites); fmt rc0; clippy `-D warnings` rc0; live PoC PASS — `generic one`/`hi` → `gpt-5` in=7 out=3 `src=generic`, native claude_code 10/5 and 1100/550 unchanged with the injection string preserved.
- **full (248750270124458696cc3e19c1ff7501):** 21/21 CI lanes rc0 in CI step order (fmt, clippy, tests, build-release, entrypoints, output-contract, gates 07-12, real-cli-smoke, lanes 15-20, cargo-deny, workspace clippy); tests 470/0 across 22 suites = base 463 + 7 new; heavy lanes through `local_validation_gate.py` per fleet policy.
- **Digest lineage:** `validation:v1:a72443d225dc8d687e34505eb328bd65608884abc60d932d59f83678062b7b0c` — declared VERBATIM by targeted and full, re-derived engine-side at both turn ends; every changed surface is non-executable (ROADMAP/lib.rs/tests/fixtures), so the digest could not move between folds.
- **No test execution at compound** — this phase documents; validation outcomes above are the record.

## /tmp durability (re-proven this cycle)

`/tmp/at-assess-4502` (the assess PoC corpus) is REAPED — the only surviving verbatim copies of the c2/c-generic PoCs are the in-tree fixtures. Re-proves the b1ff12f8 c1 rule: **golden fixtures go in-tree (md5-pinned) at fix time; /tmp is not a citation surface across phases.** Other cycle scratch (`/tmp/at-prioritize-ba83`, `/tmp/at-full-2487`) should be treated as gone by the next cycle.

## Prevention / reusable lesson

New: `docs/solutions/reliability/a-serde-rename-without-an-alias-is-a-silent-wire-break.md` — a `serde(rename)` without an `alias` is a silent wire break on journal-shaped structs, and a fallback parse lane converts it into silent wrong numbers; the cure is alias + casing-parity test + drop-disclosure counters on the fallback lane.

Vocabulary capture: "counted lane" added to `CONCEPTS.md` (the role/surface on which a family's usage is actually summed — meta events for native families, conversation lines for the generic lane; the term the rm-616 disclosures are phrased in). No other cycle term qualified.

## Review fix (2026-10-07, attempt 9e2227b4)

All four review findings fixed in the same worktree, uncommitted:

- **F1 (high) stale-cache mask:** `SESSION_CACHE_SCHEMA_VERSION` 27→28 (`session_cache.rs` bump-comment chain extended) + the three pin sites aligned with dated chains: tui warm fixture (`agenttrace-tui/src/tests.rs:1648`), governance-guide sentence (`docs/guides/governance-reports.md`, enumeration extended), both `Some(27)`→`Some(28)` literals in `discovery_contract.rs`. Replay of the reviewer's PoC: the same `/tmp/at-review-f6b9/cache-stale` dir warmed by the pre-fix binary now serves **gpt-5 7/3** on both sessions (was `default` 2/2 & 1/1); fresh cache stamps **28**; warm round-trip byte-identical.
- **F2 (medium) false rationale:** the "Session-cache schema" seam below is rewritten (dated) — the compound-time "no bump owed" claim was wrong and is corrected here.
- **F3 (low) suppressed disclosure:** the `!has_reported_usage` guard dropped from the `usage_present_not_counted` branch in `analyze()` — stray conversation-line usage now discloses even beside meta totals (totals stay meta-only; the meta arm and generic fold still cannot fire it). Pinned by `stray_conversation_usage_disclosed_beside_meta_usage` (meta 10/5 + stray 7/3 → totals 10/5, disclosure 1). No native fixture carries conversation-line usage (suite-verified: zero regressions).
- **F4 (low) test hygiene + unpinned join:** `write()` pid-qualifies `/tmp/at-rm616-*` paths and `session()` removes them after parse; the usage_models per-block pricing join is pinned by `model_switch_joins_usage_models_for_per_block_pricing` (two-model generic session → tokens 12/5, `model_used == "multiple"`, `pricing_source == "multiple models (priced per usage block)" — catalog-independent observables).

**Retest (targeted, this turn):** core **329/0** (was 327 + 2 new; lib 184 + suites incl. discovery_contract 81/0 and generic_model_usage_truth 9/0), tui **47/0**, `cargo fmt --all -- --check` rc0, `cargo clippy --workspace --all-targets -D warnings` rc0, `scripts/ci/check-docs-commands.sh` rc0. Digest re-derived at fold (see result envelope) — the executable surfaces changed this turn.

Tooling note recorded for the fleet: one edit-tool "success" on `agenttrace-tui/src/tests.rs` never reached disk (file mtime still the checkout stamp; content unchanged) — the failure surfaced only because the tui suite went red. Verify edits on disk immediately, especially raw-string files; the re-apply went through a byte-exact python replace.

## Next-cycle context

- **Next-cycle lead batch: rm-617 (88.0) + rm-618 (87.0) + rm-619 (70.0)** — the upstream #312 accounting-truth port wave (codex reasoning double-add removal / qwen first_number single-source + cache clamp / opencode tokens.reasoning subtree). Each carries a composition-hazard rider on the wall: **run-7f9c6d24's unlanded rm-551..556 band** ("usage-accounting truthfulness cycle 2", uncommitted, parser.rs +291/−139, schema 26→27) already implements the qwen first_number rewrite and the codex saturating_add removal — re-sweep that band live before implementing; reconcile by TITLE at its landing; rm-619 (opencode) is not evidenced in it but re-verify with a title sweep anyway.
- **R7 rider home:** workbuddy_usage basis disclosure rider → line 1568 of the wall (`rm-450`'s workbuddy lane).
- **R1 already claimed:** reemission double-count sits at wall line 1316 (do not re-mint).
- **Watches carried from research c5ec22b8:** LiteLLM live 4,480 priced rows vs bundled 3,100 (rm-006/rm-176 own rates/cadence); upstream #318 ureq-3 cargo-group is the reference for any transport-limit port (fork must keep the pricing read cap ≥32 MiB so `read_body_capped` stays binding); OTel semconv v1.44.0 with the gen-ai docs moved to the dedicated `semantic-conventions-genai` repo (zero tags — rm-229 watch untripped).
- **Fleet id frontier at compound (live sweep, 53 worktrees):** def-row max on disk = rm-686 (run-2f02ecafb267); fleet records carry spool-side mints past rm-690. The space above rm-619 is CONTESTED — the next mint re-censuses live (worktree walls AND spool scratch, both; the 8e983cf5 attempt died on an mtime-window miss). Landed wall max at this base: rm-545.

## Commit-gate seams (for the gate that lands this)

- ONE commit: ROADMAP.md (roadmap-phase +37, compound delta, rm-616 review-fix note), `crates/agenttrace-core/src/lib.rs`, `crates/agenttrace-core/src/session_cache.rs` (schema 28), `crates/agenttrace-core/tests/generic_model_usage_truth.rs` (9 tests), `crates/agenttrace-core/tests/discovery_contract.rs` (both v28 pins), `crates/agenttrace-core/tests/fixtures/rm-616-generic/`, `crates/agenttrace-tui/src/tests.rs` (warm fixture v28), `docs/guides/governance-reports.md` (schema-28 sentence), this record, the prevention doc, `CONCEPTS.md`.
- rm-616 done-flip (implemented → done) reserved to the gate (rm-012 convention).
- CHANGELOG entries land with the merge per lineage convention (fd5532f/1e1eb66 precedent), not minted at compound. The Unreleased/Fixed entry should name both the alias/fold and the schema-28 invalidation (review fix F1).
- Session-cache schema: **bumped 27→28 at review fix 9e2227b4** — the compound-time claim that no bump was owed was WRONG (review f6b98bc3 F1 PoC: a cache warmed by the pre-fix binary served `default`/1/1 verbatim under matching size/mtime fingerprints — no re-parse, no self-heal). A later sibling bump (e.g. the unlanded 7f9c6d24 band's 26→27) re-bases onto 28 per the rm-230 convention — one invalidation either way, the higher number wins at merge.
- Integration will likely have to rebase this wall band (rm-616..619) numerals past the contested space — renumber by TITLE, keep this record's campaign numerals as dated provenance.

---

## Correction (appended 2026-10-09, run feb16bba cycle 2)

This record's "Retest (targeted, this turn): core 329/0" is faithful
to the lane it was written in, not to the merged tree: at the recorded
integration (86c4eb4, the rm-616 landing) the FULL workspace battery
was RED — `hostile_journal_disclosure::relocated_usage_keys_disclose_instead_of_silent_zero`
failed its len()==2 pin because the landed rm-616 review fix F3 fires
`usage_present_not_counted` beside meta usage (the cycle-2 adversarial
assess measured 605 passed / 1 failed on the merged tree). The tree
stayed red until integration re-pinned that test (e249a10,
2026-10-07 21:47:03Z) to
len()==3 with the beside-meta counter asserted (conflict case
15bdfe5f, `e249a10` on the landed wall). Recorded here as a dated
append rather than an edit of the gate lines above: they say what that
turn saw, and this says what the merged tree actually was.

Cycle 2 (run feb16bba, rm-856 'journal-truth composition repair')
adopts that re-pin at its base (96b528e) so the battery this record
speaks for is green there again, and closes the adjacent gap that
battery could not see: a generic-lane session that also carries usage
on a meta-role line previously folded BOTH lanes into its totals (the
assess PoC: meta 100/50 + assistant 7/3 → total_tokens 160, no
disclosure). With meta usage present the meta arm is now the counted
lane — the native-family precedent
`stray_conversation_usage_disclosed_beside_meta_usage` — the generic
fold stands down, and the stood-down lines disclose via
`usage_present_not_counted`. Session-cache schema 33 → 34 (rm-230
convention) so warm caches regenerate under the corrected totals; the
governance guide's schema sentence moved with it.
