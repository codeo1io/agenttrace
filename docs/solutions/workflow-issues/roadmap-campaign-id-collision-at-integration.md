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

## Detection history

- Found 2026-09-30 by adversarial assessment pass 12 (F1, `docs/reviews/2026-09-30-adversarial-repository-assessment-pass12.md`).
- Executed 2026-09-30, roadmap phase of run `0a279c44`: `rm-012..023` → `rm-034..045`, cross-refs updated, mapping recorded in ROADMAP.md under "RENUMBER EXECUTED". Post-edit duplicate probe: empty; id inventory `rm-001..027 + rm-034..054` all unique.
- Re-hit 2026-10-01, integration of PR #19 (`334b5a8`, conflict cases 55e6e239 → 5af7cbb6): the incoming block used backticked `` - id: `rm-026` `` while the landed block used bare `- id: rm-026`, so the probe as originally written compared the two matches as unequal strings and could not see the duplicate at all. The probe above now normalizes backticks (`sed 's/`//g'`) before `sort | uniq -d`. Renumber executed in the integration commit per rule 2: `rm-026..032` → `rm-155..161`, mapping recorded in ROADMAP.md.
