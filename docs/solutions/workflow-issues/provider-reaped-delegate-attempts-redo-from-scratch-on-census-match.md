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

## Addendum (2026-10-08, run c0f141d1 cycle 2 — six more instances; both directions of rule 4 exercised)

This cycle reaped SIX attempts across one run (research ×2: `84667b59`, `acd61f50`; implement `8778c9d1`; targeted `85eb990f`; full_tests `f4df3f3f`; compound `caa859e5`) and validated the taxonomy while extending it:

- **Zero-durable census match → redo** (rules 1–3): research ×2 and compound `caa859e5` (23s, 2 pings, porcelain identical to the prior phase's handoff) — redone from scratch, nothing adopted.
- **Drift-present ADOPT — new branch, the complement of rule 3:** implement `8778c9d1` left NO typed artifact but uncommitted drift on exactly the batch's files with mtimes inside its window. Drift on exactly the batch's files is CANDIDATE work, never proof: the successor read the full diff line-by-line against the stewardship contract before adopting — and still found real work the dead attempt had missed (an E0659 name collision that only fires at compile). Adopt only after review against the contract, and re-run every gate the dead attempt never reached.
- **Typed-complete but re-dispatched:** targeted `85eb990f` wrote a full PhaseResult JSON yet the phase was re-dispatched; the successor re-ran every leg fresh rather than adopt a sibling's result — the cheapest correct disposition when the battery is minutes, not hours.
- **Survivor-runner ADOPT:** full_tests `f4df3f3f` was reaped 67s AFTER its backgrounded lane runner had finished all 23 lanes rc0. Adoption legs: script identity, log genuineness (logs embed the worktree path), independent lane re-derivation, tree immobility, counts. Precedents: `cd85a837`, `9a4d37af`, `2152ef02`.

Rule 4's verify-before-adopt held as the discriminator in all six dispositions. Record: `docs/stewardship/2026-10-08-cycle2-compound-record-runc0f141d1.md`.

## Related

- Original statement of the porcelain-census discriminator: `docs/stewardship/2026-10-06-cycle2-compound-record-runb1ff12f8.md` ("Attempts and forensics").
- Four-instance corroboration and the compound-time re-verification for attempt `70651ebb`: `docs/stewardship/2026-10-07-cycle1-compound-record-run460d0633.md`.
- Sibling class (fold rejected for a missing attestation, not infra death): the KTD13 changed-surfaces repairs recorded in runs `b1ff12f8` and `3c24960c` — different failure, same "verify before adopting" rule.
