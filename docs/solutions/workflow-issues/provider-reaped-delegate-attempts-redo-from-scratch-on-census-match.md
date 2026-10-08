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

## 2026-10-08 extension — reaped attempts that DID finish (two adoption classes)

Run `749cd29820f3` (repository-maintenance dda534730 cycle 1) hit four provider deaths and
proved the "zero tree drift" assumption above is a COMMON case, not a law. Two of the four
left complete, verified, adoptable work:

1. **Tree drift (implement 44a88914 → adopted by a0ef8a86).** Reaped at 24 messages with
   no envelope, but the worktree carried the entire implemented batch (~+460 lines over 6
   code/doc files). Adoption gate = the work-order lineage (run/action/base identity),
   verified first-hand before any adoption; the tree trail is only the CANDIDATE, never the
   proof (same rule as 73fe8e1e's 05563e2a case).
2. **Detached survivor runner (full_tests cd85a837 → adopted by 2152ef02).** Reaped
   05:58:36Z — but its backgrounded lane runner had already finished ALL 23 ci.yml lanes
   rc=0 at 05:56Z, two minutes BEFORE the reap decision window. Third fleet instance of the
   pattern (9a4d37af original; f4df3f3f same-day on run c0f141d1). Consequences:
   - Hour-scale batteries run DETACHED with logs + CI_OUT under `/tmp`, named for the
     attempt — never inside the repo, never only in the dying session's stdout.
   - A successor phase SWEEPS for completed survivors before re-running hours of lanes.
   - Adoption needs legs, not vibes: script identity (cd's into THIS worktree), independent
     lane re-derivation from ci.yml, tree immobility (porcelain + mtimes predate the
     runner), log genuineness (embedded paths, gate outputs), digest re-derivation.

The heartbeat-only case still holds: this run's compound 6394648a (3 messages, pings only)
and prioritize 51984e11 (message 4) were correctly redone from scratch — rules 1–3 above
remain the fast path, and this extension adds two checks to run after they come back
clean-or-dirty: **tree drift as candidate work, detached /tmp artifacts as candidate
results.** Recorded in PR-Y of docs/stewardship/2026-10-08-cycle1-compound-record-run749cd29820f3.md.
