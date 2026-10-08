# Prevention rule: provider-reaped delegate attempts — redo from scratch on census match, adopt nothing unverified

- **Class:** workflow / conductor delegate attempt recovery (infra-death forensics)
- **Observed:** recurring fleet-wide; four instances in one cycle on 2026-10-06/07, run `460d0633` (repository-maintenance `347d61bc` cycle 1): assess `13c0b61c`, roadmap `bd772c10`, stewardship `5c42758d`, compound `70651ebb`. Prior cycles hit the same shape (run `b1ff12f8`: `39bffb00`, `54eda020`; run `adcef255d604`: `5da09688`; run `32192d92`: `98139ad7`). Five more in one cycle on 2026-10-08, run `e88f2da2fbe9` cycle 3 — three of them DELTA-PRESENT adoptions, not zero-drift redos (see the dated Addendum below).
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

## Addendum (2026-10-08, run `e88f2da2fbe9` cycle 3 — five reaps, three delta-present adoptions)

The 2026-10-06/07 four-instance cycle above was the ZERO-DRIFT class: porcelain census matches the prior handoff, nothing to adopt, redo from scratch. Cycle 3 of run `e88f2da2fbe9` (repository-maintenance `546703d4`, base `92149bd`) hit the class five times and produced the other two dispositions in bulk — including the first recorded COMPOUND-phase reap-after-write:

| Phase | Attempt | Durable trail | Disposition |
|---|---|---|---|
| roadmap | `9060b4e3` | `M ROADMAP.md` +38 (band mint), written 15:07:54Z; reaped 15:09:23Z, 89 s later | delta-present ADOPT after leg-by-leg re-verification (redo `65bb1d3a` filed the result) |
| stewardship | `8d760c28` | 4-line event log, no artifacts | zero-durable → redo `1109077635` from scratch |
| implement | `d886218c` | 3-file code delta + `red-first.log` with 4 planted red tests | delta-present ADOPT then COMPLETE — rule 7 below |
| compound | `884f0594` | banner + 3 row flips + cycle record, written 02:29:37Z; reaped 02:31:08Z, 91 s later | delta-present ADOPT by `4c62495c` — rule 8 below |
| compound | `1e7485cb` | 5-line event log, 31 s, no file mtimes near it | zero-durable → redo (the present attempt) |

Two refinements, both minted as numbered rules:

7. **Adopt-then-run-the-reds (delta-present code adoption).** A reaped attempt's planted red tests are the cheapest oracle for whether its delta is complete. `d886218c`'s adopted implement delta was structurally sound but seeded the budget series' first-day baseline with `(NEG_INF, NEG_INF)`, making `first_seen` unreachable — two of its OWN planted reds stayed red against the adopted tree, because the reap had cut the attempt before any green run. The successor ran the dead attempt's reds against the adopted delta, caught the seed defect from them, and completed. An adopted delta is not trusted until the dead attempt's own red tests (or fresh equivalents planted to the same acceptance) run green against it — a green full suite alone is not enough, because the suite was green at the base too.

8. **A referenced artifact is not a written artifact (delta-present documentation adoption).** The reaped compound attempt's banner and cycle record both cited "a dated addendum on this doc" — written 91 s before its death, never started (this file sat unmodified at HEAD). When adopting a dead attempt's documentation delta, sweep its own references for targets that do not exist in the tree yet: the citation list IS the TODO list. Closing this one surfaced a second lesson of the same shape — verify phase-attribution labels against the event logs before certifying a wall (`01e7fe06` was the targeted_tests attempt, not compound; the row now says `targeted`).
