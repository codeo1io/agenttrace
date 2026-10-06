# Prevention rule: mid-cycle landings consume freshly-minted roadmap ids — rebind, don't re-mint

- **Class:** workflow / multi-campaign roadmap maintenance
- **Observed:** 2026-10-06, campaign `0314111d` cycle 2 (run `ff0068ca`), minted at roadmap `06692f2b`, caught at prioritize `60534aec`
- **Cost:** two of three minted ids (`rm-597`, `rm-598`) were already owned by landed rows one phase later. Caught early the remedy was a rebind (rows + banner provenance, zero re-review); caught at integration it would have cost the full renumber sweep described in `roadmap-campaign-id-collision-at-integration.md`.

## What happened

The roadmap phase censused the claim landscape the correct way — every sibling worktree's wall (33 of them, max uncommitted claim `rm-609`), the delegate spool patches, and a fresh `git fetch origin` (landed ceiling `rm-545` at `ee67b22`) — and minted `rm-596..rm-598` above every visible claim.

But run `250cfd64` landed **between that fetch and the prioritize phase**: origin/master moved `ee67b22` → `05d5016` and its band put `rm-597` ("Deduplicate symlinked/hardlinked session files") and `rm-598` ("Sweep skills/ CLI claims in the docs-commands gate") onto the landed wall. A landed-and-cleaned lane has **no worktree**, so no worktree-band sweep can ever see it; its numerals live only on origin/master. The mint was correct at mint time against everything visible — the wall simply moved underneath it.

Prioritize re-fetched origin as part of its live-state verification, title-checked each minted id against the landed rows, found `rm-596` free and the other two consumed by title-disjoint subjects, and **rebound**: `rm-597`→`rm-610`, `rm-598`→`rm-611`, numerals chosen above every recorded claim (with the `rm-600..604` dashboard-band cession respected), the rebind recorded on each row's notes and on the campaign banner.

The same cycle provides the counterfactual: within it, origin/master's ceiling advanced three times (`rm-545` → `rm-598` → `rm-600` by compound time). Any mint not checked against a same-phase fetch is racing every concurrent landing.

## Prevention rules

1. **A worktree-band census is an under-approximation by construction.** Landed-and-cleaned lanes are invisible to it. The census needs all three inputs — sibling worktrees, delegate-spool patches, and a *fresh* `git fetch origin` wall — and the banner should timestamp the fetch so staleness is detectable later.
2. **Walls move mid-cycle; re-check at every consuming phase.** Any phase that acts on campaign numerals (prioritize selecting a batch, implement annotating rows, integration folding bands) re-fetches and title-checks origin/master first. An id claimed by a landed row with a different title is a **rebind**, never a second mint of the same numeral.
3. **Rebind mechanics (the 202c4d1d convention, now exercised at prioritize too):** the rebound numeral clears every recorded claim; the rebind is recorded once on the row's notes and once on the campaign banner; campaign-local numerals survive inside dated annotations so grep lineage never loses the history; executable surfaces never cite campaign-local ids (that is what keeps a rebind docs-only).
4. **Title-census beats id-census for "is my subject already landed."** Id ceilings move hourly; title overlap is the actual duplication signal. Here the landed `rm-597`/`rm-598` were title-disjoint from the mints — a pure numeral collision, which is why rebind (not fold) was the correct reconciliation. (A title-twin landed row means fold-as-corroboration instead — the 96762b67 rm-506/rm-508 precedent.)

## Detection history

- Found 2026-10-06 at prioritize `60534aec` of run `ff0068ca`: the re-fetch + direct status check of origin/master's rows; root cause confirmed by `ls -d ../run-250cfd64*` → no worktree (the landed lane had been cleaned, so the roadmap census could not have seen it even in principle).
- Rules 1–4 codified at this run's compound (`1d777343`), the same pass that recorded the ceiling moving twice more within the cycle.
- Sibling failure mode, same remedy family: the `1ecd791b`/`460d0633` `rm-594` double-mint (two *concurrently uncommitted* runs minting the same numeral above the same ceiling from mutually visible worktrees) — a concurrent-mint race rather than a no-worktree blind spot; reconciled by title at integration. Both modes reduce to rule 2: the check that matters happens when a numeral is about to be consumed, against a wall fetched in that phase.
