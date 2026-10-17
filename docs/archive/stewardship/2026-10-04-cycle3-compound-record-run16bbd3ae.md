# Cycle-3 compound record — run 16bbd3ae (repository-maintenance 1f8fb6e3, cycle 3)

Date: 2026-10-04 · Compound attempt: 2f9d31b7065c4a3a89a496f3a26a12cc · Base: c032f33 (== origin/master at dispatch) · Theme: **truthful measurement + parser-contract tripwire**

State at compound: uncommitted 8-path implement delta (4 M + 4 intent-to-add: M CHANGELOG.md, M ROADMAP.md, M `crates/agenttrace-core/src/diagnostics.rs`, M `crates/agenttrace-core/src/reports.rs`; intent-to-add `crates/agenttrace-core/tests/pi_journal_contract.rs` + 3 fixtures `tests/fixtures/{pi-pi-v3,pi-oh-my-pi,pi-senpi}/journal.jsonl`) + this record. No test/validation command was executed at compound — every gate below is consumed as recorded evidence from the targeted and full folds, per the compound contract.

## Cycle outcome (all-green chain, pre-review)

| phase | attempt | outcome |
|---|---|---|
| assess | 428195d6 | 6 findings (A1–A6), 3 live release-binary PoCs at c032f33; baseline 357/0, fmt+clippy clean |
| research | 3de8e5b1 | pass-11: upstream quiet (tip 52ab2cd, 9 CI-only commits), pi 1.0.0→1.0.2 in 4 days on a PRIVATE repo, ccusage #1821 merged pre-release |
| roadmap | 99c06527 | minted rm-420..423 past the live claim landscape (agenttrace ceiling rm-419), +30/-0 |
| prioritize | b680d4a2 | batch = rm-420 (LEAD) + rm-423 + rm-004 residual arm; 9-item ordered deferral queue |
| stewardship | 58a283ff | batch charter (units, seams, fixtures, fold guidance) |
| implement | b11a1f3a | full batch landed uncommitted: 3 code sites + 3 golden tests (diagnostics), pin extension (reports), 7-test contract suite + 3 fixtures, CHANGELOG 2 Fixed + 1 Added |
| targeted_tests | bbcd21a4 | scope `targeted`: core 254/0, fmt rc0, clippy rc0, named greps ok; offline gate dry-run `''` PASS |
| full_tests | 2493686a | ci.yml **full+deny mirrored step-for-step** (empty `full_command` per fleet precedent 71a7d5db/a0407d88): 19 rc0 + 1 skipped-by-condition, 367/0 + entrypoints 25/0, `cargo deny --all-features check` all-ok; porcelain SHA `585dac2f…` identical before/after |
| compound | 2f9d31b7 | this record + ROADMAP gates addenda (below) |

Digest, declared identically at both validation turns (nothing executable changed between them):
`validation:v1:c11a345df7986450bf53c31a22e2818b48515102a9f7f83bd06555721c8e9a25`

## Roadmap accounting (this compound's edits)

- `rm-420` — compound gates addendum appended (implementation detail, pin-extension form, PoC flip 2.0/false → 31.0/true, twin-mint pairing note). Status stays `implemented`.
- `rm-423` — compound gates addendum appended (7/7 suite, fixture provenance, `modelId` wire-key dead-handler pin, **REMAINING SCOPE: version-marker report arm not implemented**). Status stays `implemented`.
- `rm-004` — cycle-3 arm gates line appended under the existing CLOSED arm note.
- `rm-421` — `note 2026-10-04 (compound c3)`: recorded as **next cycle's LEAD** per prioritize b680d4a2's explicit deferral, with the durable fixture mirror (below) and the composition hints (rm-251 adjacency; 6557b823 rm-413 `type:"summary"` fixture sharing).
- No status flipped past `implemented`; done-flips reserved for the commit gate (house precedent). CHANGELOG already carries the batch's 2 Fixed + 1 Added from implement — no compound change.

## Prevention rules (fleet-lettered; continue from 6557b823's PR-G..K)

- **PR-L — pin-scan self-match, second canonical form.** The workspace-scan pin at reports.rs:3042+ rejects inlined index arithmetic via *runtime-concatenated* probes; 6557b823's PR-H used `concat!` needles. Both anti-self-match forms are now in-tree — any future scan pin copies one of them verbatim; a plain literal needle self-matches through `include_str!` and passes vacuously. (Corroborates PR-H with the second occurrence.)
- **PR-M — private-upstream contract fixtures are DERIVED-REDACTED, never synthetic-only, never raw.** The pi-v3 fixture was derived from a real live 1.0.2 journal: shape/keys/alias families preserved, values synthetic. Record derivation provenance in the fixture header. Pin KNOWN divergences deliberately (the `model_change` `model`-vs-`modelId` dead handler) so an upstream fix must consciously update the contract rather than silently change behavior.
- **PR-N — twin "one definition" implementations collide at the PINS.** When two lanes independently implement the same unification rule (this run's rm-420 ≡ 6557b823's rm-411, different bases, both uncommitted), the code union is trivial but both lanes extend the SAME pin test region — the integration gate must union the two pins into one canonical needle set, not carry both; superseded deltas (f22ff7d1) are dropped, never double-applied. Merge by TITLE per fleet protocol.
- **PR-O — golden-test robustness rules (batch-earned).** (1) Assert `cost > 0`, never exact cost values — pricing catalog drift moves exact assertions. (2) Meta-tests that name drift from a `catch_unwind` payload must `downcast_ref::<String>` AND `downcast_ref::<&str>` — payloads arrive as either. (3) Assert pin-test targets as behavioral outcomes on golden corpora (19×2s+1×31s → 31.0), not as index arithmetic.
- **PR-J corroborated (evidence mirroring).** This cycle re-hit the /tmp sweep risk and pre-empted it: the three PoC fixtures are mirrored into this run's durable spool scratch (`2f9d31b7…-scratch/fx-mirror/`, sha256 `edf2b830…` p95probe / `03de7359…` omp s.jsonl / `0422cb86…` stepprobe) and the full-suite logs were mirrored by the full_tests fold itself. Evidence must be mirrored at the PRODUCING phase; consumers of next-cycle leads (rm-421) read from spool, not /tmp.

## Residuals and open scope (banked for later cycles)

1. **rm-423 version-marker report arm** (XS rider): doctor/gate disclosure of journal markers observed vs `KNOWN_PI_JOURNAL_VERSIONS` — the suite pins the contract; the disclosure surface remains unbuilt.
2. **rm-245 remaining scope — TUI attribution panels** (assess A4 note on the row): by_provider/by_task_type/top_cost_drivers still unreachable from the default TUI surface.
3. **rm-298/rm-299 thrash axis** (assess A5 note): over-bound oldest-mtime eviction re-parses ~918 entries/run at the recorded live gate; needs its own measurement cycle.
4. **rm-017/rm-018 RC-3 refreshes**: `check-deterministic-output.sh` still lacks #294's `generated_at` strip (one-line CI hunk, rides any hygiene pass); Codecov + git-cliff lanes remain unmirrored.
5. Watch items (research RC-4, parked): codex 0.160 GA quiet on rollout/session keywords; OTel semconv still untagged (rm-229 stays parked); npm bare-`agenttrace` identity window still open (rm-195 decision gate).

## Next-cycle context (authoritative order from prioritize b680d4a2)

1. **rm-421 (90.0) LEAD** — cross-parser compaction episodes; omp fixture mirrored (see PR-J); compose claude arm with 6557b823 rm-413's `type:"summary"` pin (one real fixture serves both); watch 2c2db6f5's staged rm-346→rm-372 claim on the codex arm.
2. **rm-422 (60.0)** — idle-vs-in-flight anomaly classes; design-first (user-visible health semantics).
3. rm-245 TUI attribution; 4. rm-251 (90.0, verify-first, share parser.rs churn carefully with rm-421); 5. rm-298/299 eviction; 6. rm-195 (decision); 7. rm-231 (compose with 71f666e8's numerator lane, don't race); 8. rm-196/rm-239 (after sibling pricing/sanitize lanes land); 9. rm-017/rm-018 refresh notes.

## Integration handoff (for the review + commit gates)

- Expected changed-file set: the 8-path implement delta (4 M + 4 intent-to-add) + NEW `docs/stewardship/2026-10-04-cycle3-compound-record-run16bbd3ae.md` (this file). The review-fix fold (bb5bb645, see addendum below) amended ROADMAP.md, CHANGELOG.md, and this record in place, and extended the executable delta with one fixture line + one contract test — nothing else.
- Twin-mint map: **rm-420 ≡ 6557b823 rm-411** — union pins into one canonical form (PR-N); their lane additionally carries tui `shared.rs:284` routing + `percentile` pub(crate)→pub + rm-412 doctor `-d` guard; ours carries the 31s-tail goldens, the rm-004 trace_steps arm, and rm-423 (no counterpart there). f22ff7d1 superseded. **rm-423** has no sibling counterpart; **rm-421** twin claims exist at 6557b823 rm-413 / fb22927c — those lanes' definitions land or renumber per the 880a7b9e discipline.
- Rows' "conductor validation digest … implement gate" placeholders are now satisfied in the compound addenda with the pre-review digest; per 6557b823 precedent, if the integrated tree differs the shipping PR records the re-derived digest at its own gate.
- Review/shipping outcomes land AFTER this record by design; the next cycle's assessment carries them forward.

## Review-fix addendum (2026-10-05, attempt bb5bb645 — review 21a6c6a5 NEEDS_CHANGES → all findings resolved)

The independent review verified the code delta, gates, digest chain, and redaction clean; NEEDS_CHANGES rested on one factual error in the compound artifacts plus low-severity scoping items. Dispositions (file:line per the review):

1. **MEDIUM ROADMAP.md:1201** — phantom golden name `tool_p95_uses_house_percentile_on_the_31s_tail`, provenance traced by the review to targeted_tests bbcd21a4's evidence block misquoting its own (truthful, still-on-disk) log → FIXED: the row now cites the real goldens `tool_latencies_p95_uses_house_percentile_at_mod_20_boundary` (diagnostics.rs:1261) and `p95_gap_uses_house_percentile_at_mod_20_boundary` (:1303). The rm-004 gates line already carried the real `trace_steps_agree_with_tool_latencies_beyond_one_hour` (:1313).
2. **LOW ROADMAP.md:1201** — "cross-surface agreement on one session" implied an overview/diagnostics value-equality golden → FIXED: reworded to by-construction (overview `latency_p95` is gap-based reports.rs:94, tool p95 call-based; both route `crate::percentile`; both guarded by the extended pin).
3. **LOW CHANGELOG.md:7** — unscoped "cannot slip in unscanned" → FIXED: scoped to the core crate; the three TUI trunc copies (this tree presentation.rs:3740/:3756, shared.rs:294) named as riding twin lane rm-411 (run 6557b823), which routes them and widens `percentile` to `pub`.
4. **LOW ROADMAP.md:1201** — twin-mint note under-enumerated the sibling's TUI sites → FIXED: three sites enumerated (their tree shared.rs:293 + presentation.rs:3739/:3753) plus the `pub(crate)→pub` widening that makes them reachable.
5. **LOW tests/pi_journal_contract.rs:27** — the known-divergence doc promised the suite changes with any branch_summary/compaction routing change, but no fixture carried such an entry → FIXED by pinning (the review's primary option): the omp fixture gained a `branch_summary` line and NEW test `branch_summary_surfaces_as_assistant_turn_known_divergence` asserts `assistant_turns == 2` (one real assistant message + one system entry) with `user_messages == 1`; the suite header now names the pin. Suite 7 → 8 tests. rm-421's reroute now trips this deliberately.
6. **LOW** rm-423 version-marker report arm — no change: already disclosed as REMAINING SCOPE on the row and above; carried to the commit gate as a known residual.
7. **LOW** "6-file implement delta" count label vs 8 enumerated paths → FIXED to "8-path".

Re-validation at this fold (fresh, targeted): `cargo test -p agenttrace-core` → **255/0** (was 254/0; +1 pin test), `cargo fmt --all --check` rc0 (after rustfmt normalization of the new test), `cargo clippy -p agenttrace-core --all-targets -- -D warnings` rc0. Executable delta changed this turn (tests/pi_journal_contract.rs + fixtures/pi-oh-my-pi/journal.jsonl) — and the post-fix digest re-derivation returns the SAME token `validation:v1:c11a345d…` as before the fix: the engine digest hashes committed state plus true untracked files, and this run's whole delta (implement + review fix) is uncommitted, so the token legitimately does not move until the commit gate. The declared digest in the bb5bb645 phase result is that re-derived, verified-current value. Production code untouched.

## Verification (static — no tests executed at compound)

`grep -c '^### ' ROADMAP.md` sections; `grep -o 'id: \`rm-' ROADMAP.md | wc -l` id rows (old+new formats); duplicate-id check `grep -o 'id: \`rm-[0-9]*\`' | sort | uniq -d` (expect empty); footer `tail -1 ROADMAP.md` == managed-render marker; `git status --porcelain` == the 6-file delta + this record; `git diff --numstat ROADMAP.md` reflects roadmap(+30)+compound appends. Full validation evidence lives in the bbcd21a4/2493686a spool JSONs and the mirrored logs under `delegate/2493686a…-scratch/logs/`.
