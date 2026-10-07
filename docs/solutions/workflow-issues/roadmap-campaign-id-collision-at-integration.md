# Prevention rule: campaign roadmap ids collide when integration skips the renumber

- **Class:** workflow / multi-campaign roadmap maintenance
- **Observed:** 2026-09-30, campaign `47e4432e` cycle 1 (run `0a279c44`), at merge `9d88b36`
- **Cost:** twelve doubly-defined ids (`rm-012..rm-023`) shipped to `master`, discoverable only by duplicate-id grep; fixing it required renumbering a landed block (`rm-012..023` → `rm-034..045`), updating block-internal cross-refs, and re-verifying that no executable surface cited the old ids.

## What happened

Campaigns append roadmap items with locally-chosen sequential ids (`rm-001…`). Two campaigns ran concurrently from overlapping bases (88feec46 from base `7bb4dcb`, later this campaign from `9d88b36`), and each minted `rm-012..rm-023` for unrelated content. The integration merge landed the 88feec46 block **without applying the renumber that its own collision-record prescribed** — the record said "next free id above all landed and claimed," but nothing at integration time checked.

## Prevention rules

1. **Campaign appends must reserve above the known ceiling, not the local ledger.** New ids start above every landed id *and* every published in-flight claim (other campaign blocks, open PRs carrying campaign-local ids). Record the claim list in the append comment (this is what the `rm-046+` block does).
2. **Integration is the only place that can catch it — so integration must check.** Before folding a campaign branch, run the duplicate-id probe over the merged ROADMAP:

   ```sh
   grep -oE '^- id: (`)?rm-[0-9]+' ROADMAP.md | sed 's/`//g' | sort | uniq -d
   ```

   Empty output = safe. Any output = apply the campaign's own collision-record renumber **in the integration commit**, never post-hoc.
3. **Cross-refs live with their block.** Block-internal references use the block's ids; renumbering must sweep them in the same edit. Executable surfaces (scripts, CI, CHANGELOG) must never cite campaign-local `rm-NNN` ids — that is what made the renumber docs-only here (verified by grep before executing it).
4. **Write the collision record as if it will be executed by someone else.** The 88feec46 record's instruction was correct and sufficient — the failure was that no gate consumed it. Rules without a checker are comments.
5. **Size the wall with BOTH id formats before choosing a band.** The fleet's def-line census counts only backticked `` - id: `rm-NNN` `` rows; this repository also carries old-format bare `- id: rm-NNN` entries that such a grep renders invisible — 14 of them (rm-003..008, rm-020..027), 107 counted vs 121 real (research 802e0a09, 2026-10-04). A band chosen off the grep-only count can mint inside territory the invisible entries already own once anything renumbers them forward. Count with the normalized form — `grep -oE '^- id: (`)?rm-[0-9]+' | sed 's/`//g'` — the same normalization rule 2's re-hit taught, applied at census time as well as probe time.
6. **Sweep IN-FLIGHT claims across ALL three claim homes with the numeric-sorted backtick-optional pattern.** The landed wall is only one home. Uncommitted sibling worktree diffs (`git -C <worktree> diff -- ROADMAP.md` → `^\+- id: `?rm-`), delegate-spool scratch patches and postimage `.md`s, and `/tmp` research artifacts all carry live claims that no landed-wall census can see. Two wrong sweeps observed 2026-10-07 (run `de600e73` roadmap `4c34ecaa`): a lexicographic `sort` read run-order noise as the frontier, and a plain `rm-[0-9]+` grep was structurally blind to the backticked row format (rule 5's blindness, resurfacing on the claim side). The correct sweep is `grep -oE '^\+- id: `?rm-[0-9]+' | grep -oE '[0-9]+$' | sort -n` per home, max across homes, THEN choose the band above every landed AND in-flight numeral. Compound phases re-census and record the full claim map — the fresh map, not a remembered "next free", is the mint authority.

## Detection history

- Found 2026-09-30 by adversarial assessment pass 12 (F1, `docs/reviews/2026-09-30-adversarial-repository-assessment-pass12.md`).
- Executed 2026-09-30, roadmap phase of run `0a279c44`: `rm-012..023` → `rm-034..045`, cross-refs updated, mapping recorded in ROADMAP.md under "RENUMBER EXECUTED". Post-edit duplicate probe: empty; id inventory `rm-001..027 + rm-034..054` all unique.
- Re-hit 2026-10-01, integration of PR #19 (`334b5a8`, conflict cases 55e6e239 → 5af7cbb6): the incoming block used backticked `` - id: `rm-026` `` while the landed block used bare `- id: rm-026`, so the probe as originally written compared the two matches as unequal strings and could not see the duplicate at all. The probe above now normalizes backticks (`sed 's/`//g'`) before `sort | uniq -d`. Renumber executed in the integration commit per rule 2: `rm-026..032` → `rm-155..161`, mapping recorded in ROADMAP.md.
- Re-surfaced 2026-10-04, run `41263f584891` research (802e0a09): the census under-counted the wall by exactly the 14 old-format bare ids (107 vs 121), the same format blindness the 2026-10-01 re-hit exposed in the probe — but at census/band-selection time, before any merge existed to catch it. Rule 5 added at that run's compound (attempt 6628fb27) so band selection counts both formats from the start.
- Re-surfaced 2026-10-07, run `de600e73` roadmap (`4c34ecaa`): two sweeps before the correct one — a lexicographic sort mistook run-order noise for the frontier, and a plain `rm-NNN` grep was blind to every backticked claim row; the backtick-optional numeric pattern over all three claim homes found worktree max rm-686 (run-2f02ecaf) + spool patch max rm-692 (runs 5abd9bb7/8e983cf5) where the first two found fragments. Rule 6 added at that run's compound (`4ec574da`), whose fresh claim map topped at rm-779 (14954d7a) with a live double-mint rm-771..775 (aa41d9b5 vs e88f2da2, disjoint subjects) — evidence that the in-flight frontier routinely sits far above the landed wall and moves between phases of a single cycle.
