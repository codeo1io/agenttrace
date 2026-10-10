# Prevention rule: provider-reaped delegate attempts — redo from scratch on census match, adopt nothing unverified

- **Class:** workflow / conductor delegate attempt recovery (infra-death forensics)
- **Observed:** recurring fleet-wide; four instances in one cycle on 2026-10-06/07, run `460d0633` (repository-maintenance `347d61bc` cycle 1): assess `13c0b61c`, roadmap `bd772c10`, stewardship `5c42758d`, compound `70651ebb`. Prior cycles hit the same shape (run `b1ff12f8`: `39bffb00`, `54eda020`; run `adcef255d604`: `5da09688`; run `32192d92`: `98139ad7`).
- **Cost:** each dead attempt burns a full turn budget and forces a redo; the two failure modes that cost MORE than the redo are (a) hunting for phantom work that never happened, and (b) adopting a present-but-invalid artifact as if it were the dead attempt's result.

## What happened

A delegate attempt dies mid-turn of an infrastructure/provider failure (session reaped, transport cut). The durable trail is always the same three-part shape:

1. **No typed result:** `delegate/<attempt-id>.json` is absent (never written, or swept).
2. **A truncated event log:** `events/<attempt-id>.jsonl` shows `delegate_turn_started` → a few `delegate_turn_progress` pings (message counts only) → `session_reaped` with `exit_reason: failed` → `delegate_turn_completed` status failed. Sometimes a `phase_result_repair` round appears and also fails to materialize the artifact.
3. **Zero tree drift:** `git status --porcelain` (with `--untracked-files=all`) is byte-identical to the prior phase's recorded handoff census.

In every observed instance the progress pings carried no evidence of completed work — but the work order's forensics duty is not to trust that coincidence, it is to verify it.

## Prevention rules

1. **Read the event log before anything else.** It is cheap and always present. Turn-start + pings + `session_reaped failed` with no result-writing event means the attempt almost certainly produced nothing — now prove it.
2. **Run the porcelain census against the prior phase's handoff.** An md5 (or plain diff) of `git status --porcelain --untracked-files=all` matching the previous phase's recorded census proves the dead attempt changed no tracked or untracked file. Include untracked files: a half-written scratch file inside the repo is exactly the drift you are looking for.
3. **Run the mtime probe.** `find <worktree> -newer events/<attempt-id>.jsonl -type f ! -path '*/.git/*' ! -path '*/target/*'` — anything newer than the death was written after it and is not the dead attempt's work; an empty result plus the census match closes the case. Scratch dirs under the spool (`delegate/<attempt>-scratch/`) and `/tmp` should be checked for existence too.
4. **An absent envelope is not evidence no work happened; a present artifact is not proof it is valid.** If a typed result IS present, verify it against the current work order before adopting: identity (run/action/attempt lineage), test-selection provenance, tree census count, worktree cleanliness at dispatch. Only then adopt — and declare the adoption with its verification legs in the successor's result.
5. **Declare the disposition explicitly.** Every successor attempt states "redone from scratch, nothing adopted" (or "adopted, legs: …") with the evidence lines. A phase result that silently ignores its dead predecessor leaves the next reader unable to tell phantom work from real work.
6. **Consume raw corpora as inputs only.** A dead attempt may leave probe I/O or corpora with no conclusions (e.g. `/tmp/at-assess-460d`). Salvage them as fresh inputs if useful, but never adopt them as findings — conclusions must come from a live attempt's own verification.

## Addendum (2026-10-08, run `ec762a618372`) — reap AFTER work: the census-MISMATCH branch (adopt after lineage verification)

The rules above assume the dead attempt changed nothing. This cycle hit the opposite shape twice, and the disposition inverts: a reaped attempt that finished real work leaves **durable drift plus its own scratch logs**, and a from-scratch redo would throw away completed, verifiable work and risk double-applying a half-landed delta.

- **Stewardship variant (attempt `515fc393`):** the typed result artifact was COMPLETE on disk (`delegate/<attempt>.json`) even though the engine envelope recorded the reap. Disposition: adopt the typed artifact after verifying identity (run/action/attempt lineage) against the successor's work order — exactly rule 4 above.
- **Implement variant (attempt `ccaedd53`, died of a provider 429 after 523s):** NO typed artifact, but the run worktree carried a full contract-matching delta (5 modified files + 2 untracked test paths) AND the attempt's scratch directory carried its gate logs. The logs were decisive: `usage_truth_lanes` 8/8 green, core+tui rc0 — but `FMT_RC=1` and `CLIPPY_RC=101`, proving the reap interrupted the completion pass, not the work. Disposition: **adopt the delta, then close the gaps the reap interrupted** (here: clippy `iter_cloned_collect`, fmt drift, and a missed `SESSION_CACHE_SCHEMA_VERSION` bump the dead attempt never reached).

Rules for the mismatch branch:

1. **A census MISMATCH is not phantom work.** When the porcelain differs from the prior handoff, the dead attempt wrote real files; the question moves from "did anything happen" to "is what happened valid and complete".
2. **Verify lineage before adopting drift:** worktree HEAD == the phase's recorded base; the drift's surfaces == the phase contract's surfaces (stewardship request / work order); no unrelated files touched.
3. **Read the dead attempt's scratch logs to establish the completion frontier** — what was green, what was red, what was never run. Adopting a delta whose gates were red without re-running them adopts the failure too. The successor re-runs every gate itself regardless; the logs only tell it where to look.
4. **Declare the adoption with its verification legs** (reap proof from `results/<attempt>.json` + `events/`, lineage, contract match, gate state) and **close the interrupted work explicitly** — the successor's delta description must name what it completed beyond the adopted core, or the record misattributes the dead attempt's gaps as its own clean state.

Instances so far (fleet): runs `460d0633` (review-fix variant `ca3db0d2`), `6cb2756a` (`2c0c0709`), `ec762a618372` (`515fc393`, `ccaedd53`), `feb16bba` (`a652fa98`, zero-durable), `14954d7a` (`e3779309`, zero-durable), `2023f222` (`84f5f0fd`, zero-durable) — the zero-durable ones take the original redo branch; only drift-bearing or artifact-bearing ones take this branch.

## Related

- Original statement of the porcelain-census discriminator: `docs/stewardship/2026-10-06-cycle2-compound-record-runb1ff12f8.md` ("Attempts and forensics").
- Four-instance corroboration and the compound-time re-verification for attempt `70651ebb`: `docs/stewardship/2026-10-07-cycle1-compound-record-run460d0633.md`.
- Sibling class (fold rejected for a missing attestation, not infra death): the KTD13 changed-surfaces repairs recorded in runs `b1ff12f8` and `3c24960c` — different failure, same "verify before adopting" rule.

## Addendum (2026-10-08, run 9ab0afadaf1d, repository-maintenance 7e0b1c6c cycle 1): the drift-present ADOPT branch

This run recorded the first fleet instance of a dead attempt whose work WAS real:

- **implement attempt `9c12d7c0c9b34d1abcda46aca41cc344`** died provider-side with the standard absent trail (no `delegate/<id>.json`, no `-scratch/`, event log = progress ticks + `session_reaped` failed) — but the porcelain census did NOT match the prior phase's handoff: the worktree carried a full 6-entry implement delta (4 modified core files + an untracked 274-line test) where the handoff recorded only `' M ROADMAP.md'`. The mismatch was the SIGNAL, not a hygiene failure: the delta was verified leg-by-leg against the batch contract (BATCH.md selection + row acceptance arms), found complete in substance but unfinished in polish (4 fmt drifts, 1 clippy warning, no CHANGELOG bullet), and was ADOPTED + COMPLETED by the successor (`48a129fd`) rather than reverted and redone.
- **Rule refinement:** the porcelain-census check (rule 2) discriminates TWO dead-attempt outcomes, not one. Census MATCH ⇒ redo from scratch (the documented default — this run's assess `342ba13e` and research `642a61f4`/`a0168c68`, all verified zero-durable). Census MISMATCH ⇒ the dead attempt left work behind: inspect the drift, map every hunk against the batch contract, and adopt only what verifies leg-by-leg — a present delta is subject to the same "present artifact is not proof it is valid" skepticism as a present envelope (rule 4). Declare the adoption with its verification legs either way; never revert unexamined drift that may be a dead attempt's real work.
- **Why this matters here:** redo-from-scratch on top of unexamined drift would have DOUBLE-IMPLEMENTED the batch against the sibling run-ec762a618372 title-twin lane (its rm-760 is a declared superset of this run's rm-756+rm-757) — the adoption path is also the fleet-coordination path.
## Dated addendum (2026-10-08, run `14954d7abe15`, repository-maintenance `e2b222adc9784aa2af0335184dcd1a6d` cycle 1)

Three further zero-durable instances in ONE run, all handled by rules 1-3 + 5 with no phantom-hunt and no adoption:

- **assess `95792417`:** event log = 19 progress-ping lines, typed artifact absent, no scratch dir, no /tmp output — census MATCH (porcelain empty before/after) ⇒ redone from scratch, declared in the successor's result.
- **implement `e3779309`:** event log = progress pings → `session_reaped` (provider); typed artifact absent; worktree at dispatch was exactly the roadmap-phase handoff (` M ROADMAP.md` +50/0, porcelain verified) with zero untracked code — census MATCH ⇒ redone from scratch, declared.
- **targeted_tests `caeef554`:** event log = exactly 4 lines (started, one ping at 6 messages, `session_reaped` failed, completed failed); typed artifact absent, no scratch dir — census MATCH ⇒ redone from scratch, declared.

Fleet pattern now seven corroborated instances across six distinct actions since 2026-10-06 (assess, roadmap, stewardship, compound, implement, targeted_tests). The artifact-present branch of rule 4 has STILL never fired on a reaped attempt in this fleet — every observed reap has been zero-durable-absent, so the census match (rule 2) closed every case without the mtime probe (rule 3) ever being decisive. Successor results that carry the declared dispositions: assess `0ff2b543`, implement (run 14954d7abe15 phase result), targeted_tests `e63bcd642`. Corroboration: `docs/stewardship/2026-10-08-cycle1-compound-record-run14954d7abe15.md`.

### Dated sub-addendum (2026-10-08, later same run — compound attempt `af809889`)

The artifact-present branch has now FIRED, on this run's compound action — and the notice did not point at it:

- **compound `6bab4ea172d2`:** reaped 03:11:01Z as a provider failure, but ALL compound artifacts were already written (mtimes 03:07:11–03:07:46Z, inside its live window 03:01:47–03:11:01Z) and its typed PhaseResult landed at 03:09:37Z — 84s BEFORE the reap. Work-then-envelope complete; only the dispatch's read of the result failed. Adopted by successor `af809889` after leg-by-leg verification (identity, artifact-mtime census, wall/code/fixture legs).
- **compound `51e8d7bf`:** the retry notice's named attempt — 27s / 2 messages / zero writes (census match against `6bab4ea1`'s end-state) ⇒ nothing to redo. Eighth instance, sixth action unchanged (compound repeats).
- **Rule sharpened:** the forensics notice names the LATEST transport failure, not the author of any durable work. On a reaped action with prior attempts, enumerate EVERY sibling envelope on the same action (spool `delegate/<attempt>.json`) and cross-check artifact mtimes against each attempt's live window BEFORE redoing — a complete envelope written before its reap is adoptable evidence, not a phantom.
