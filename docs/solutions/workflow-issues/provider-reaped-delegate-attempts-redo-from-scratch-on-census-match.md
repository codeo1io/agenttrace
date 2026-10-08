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

## Related

- Original statement of the porcelain-census discriminator: `docs/stewardship/2026-10-06-cycle2-compound-record-runb1ff12f8.md` ("Attempts and forensics").
- Four-instance corroboration and the compound-time re-verification for attempt `70651ebb`: `docs/stewardship/2026-10-07-cycle1-compound-record-run460d0633.md`.
- Sibling class (fold rejected for a missing attestation, not infra death): the KTD13 changed-surfaces repairs recorded in runs `b1ff12f8` and `3c24960c` — different failure, same "verify before adopting" rule.

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
