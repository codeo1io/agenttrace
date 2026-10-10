# Prevention rule: roadmap status censuses — read full status lines and recorded adjudications, never first tokens

- **Class:** workflow / roadmap-wall status hygiene — candidate censuses, batch selection, and stale-anchor repair
- **Observed:** 2026-10-10, run b5b85faa61c9 (repository-maintenance ec4f7625 cycle 1): three independent defeat modes in one cycle — rm-618 caught at stewardship (f755bf53), rm-709 caught at implement (97201ea46), and 11 rows caught at prioritize (5843e27b) carrying inline `candidate → implemented` transitions (rm-540 exemplar, implemented 2026-10-05). Fleet corroboration: run c23d7ee8b598's prioritize re-derived the full-status-line rule independently.
- **Cost:** a first-token census overcounts candidates (this wall: 11 phantom candidates at prioritize) and risks double-selecting finished work; conversely, a landed banner's obligations can make live work LOOK finished (the rm-619 false-flip class, run cfe69077 independent_review 271d084c — a compound patch flipped it to implemented on a false justification; reverted spool-side before any landed tree). Both directions re-open or bury real work.

## What happened

Three distinct shapes of the same defect — the wall's *status token* disagrees with the wall's own *recorded adjudications*:

1. **Inline transitions.** A row's status field reads `candidate → implemented (2026-10-05, run …)`. The first token says candidate; the row is finished. First-token filters count it as live work.
2. **Landed-banner recorded-not-actioned obligations.** The integration banner at the top of the wall carries `GATE OBLIGATIONS RECORDED, NOT ACTIONED …: done-flips for rm-617/rm-897/rm-898/rm-899 …, the rm-618 stale annotation (fix landed as rm-602 … — do not re-implement)`. Those flips were never executed on the rows, so rm-618 still read `candidate` while its fix had been live for days (landed as rm-602, verified at `qwen_usage` parser.rs:3221-3285: `select_alias` + `subtract_cached_input_v2`).
3. **A row's own dated adjudication outliving its status token.** rm-709's block carried `ADJUDICATED AT INTEGRATION (2026-10-08, conflict case 2ac5bbe1): resolved as FOLD … nothing further is scheduled under rm-709` while its status token still read first-token `candidate`. A census that reads only status tokens selected it at 86.0; implementing it would have re-shipped work already landed under rm-584.

## Prevention rules

1. **Census scans read FULL status lines**, not first tokens. Count transition rows (`candidate → …`) as finished unless the transition's own citation fails verification; report them as a separate class.
2. **Before implementing any selected row, read the row's ENTIRE block** (id line through the next `###` heading), including dated riders and adjudication notes, **and grep the wall's banners** for recorded-not-actioned obligations naming that row or its title.
3. **Repair a stale anchor by flipping to the status its own recorded adjudication dictates** (`done`, citing the fold/supersession), never to `implemented` — flipping a superseded row to implemented re-opens finished work, and flipping on a banner line alone (the rm-619 class) can mark UNfinished work done. Either direction must cite its provenance in the flip text itself so the next census does not re-flag it.
4. **Adjudications outrank status tokens; code outranks both.** When a row claims its acceptance already landed (rm-618 → rm-602), verify the named symbols exist in the tree at the pinned lines before removing it from a batch — the stewardship phase's `qwen_usage` re-verification is the pattern.

## See also

- `docs/solutions/workflow-issues/adopting-provider-dead-attempts-uncommitted-code-reverify-hunks-reds-and-narrative.md` — the narrative re-verification discipline this rule's flip citations rely on.
- `docs/solutions/workflow-issues/provider-reaped-delegate-attempts-redo-from-scratch-on-census-match.md` — dispatch-side forensics for reaped attempts.
- ROADMAP.md rm-618/rm-709 riders (run b5b85faa61c9, 2026-10-10) — the live instances this rule was distilled from.
