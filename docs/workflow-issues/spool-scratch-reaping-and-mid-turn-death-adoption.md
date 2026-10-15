# Spool-scratch reaping and mid-turn provider death — rebuild bands from typed envelopes, adopt tree work by verification

**Class:** workflow-issue (fleet operations) · **Minted:** 2026-10-15, run ff7615be9401405b9750eb14c1e7c9a6 cycle 3 compound (attempt 7e1b3a2265db4517a31d52f1440382c6, repository-maintenance 7f0a7958) · **Evidence:** forensics from this run's roadmap (545dbe6d) and compound (dc6cafbb) phases · **Companion to:** `provider-dead-attempts-may-leave-real-worktree-diffs.md` (same family, observed again this run — a third and fourth instance)

## Hazard 1 — spool scratch dirs are reaped mid-run

The roadmap phase emitted its band spool-side (patch + postimage + builder + evidence notes in `delegate/545dbe6d…-scratch/`, mirrored to `/tmp/at-roadmap-545dbe6d/`). By the next phase both the scratch dir and the /tmp mirror were gone; the phase's **typed envelope** (`delegate/545dbe6d….json`) was the only surviving authority — and it is sufficient: its summary lists every rider id + line anchor + gist.

**Prevention:** mirror band artifacts to `/home/agent/.cache/at-<phase>-<id>/` AT EMISSION TIME (janitor-safe; this run's implement/full_tests phases did, the roadmap phase did not — the difference decided survivability).

**Rebuild recipe (proven this run by dc6cafbb, verified by 7e1b3a22):** take the rider list from the typed envelope only — never fleet memory, never a sibling banner, never "what the band probably said". Wall-lineage divergence strikes *inside* one repo family: rows that exist on the wall may simply carry no riders (this run: rm-493/rm-248 are wall rows; the band's 11 riders sit on rm-498 among others). Re-assert after rebuild: insert-only vs the wall, def-row count in/out, zero dup ids, managed footer last, every rider inside the span of the id it names.

## Hazard 2 — mid-turn provider deaths leave real, adoptable work

Boot-deaths (3-4 events, no scratch, no tree change) are distinguishable from mid-turn deaths: this run's compound action lost 50e2dc4a at 4 events (nothing durable) and dc6cafbb at message_count 17 / 25 events with a full scratch dir and an in-tree ROADMAP fold — but no typed envelope. An absent envelope is a transport artifact, NOT evidence no work happened.

**Adoption legs (all must hold, all cheap):**
1. **Identity lineage** — the dead attempt's event log must carry THIS work order's run/action/phase ids.
2. **Batch integrity** — `git diff -- ':(exclude)<adopted-file>'` md5 equals the prior phase's authoritative figure (here: the full_tests-recorded 05feaa6f… proving the dead attempt touched nothing but the artifact under adoption).
3. **Round-trip** — scratch patch `git apply --check` on the pristine wall, then apply → cmp postimage → cmp worktree, all byte-exact.
4. **Fidelity census** — added riders vs the authoritative envelope (ids, anchors, gists); placement census per rider; def-row/dup/footer invariants.
5. **Complete, don't just bless** — the dead attempt had unfinished deliverables (record doc, seam doc absent; banner citing them already folded). Adoption includes finishing them and declaring the split of authorship.

**Prose-vs-code check before banking:** verify every technical enumeration in adopted text against the tree (this run: a rider claimed U+00AD in the implemented spoofing set; `is_invisible_spoofing_control` at statusline.rs:431-433 excludes it, test-pinned). Also record REFUTED suspicions — a believed enumeration erratum was disproven by grep (the text was already correct); writing that down stops a later cycle from re-litigating it.

## Hazard 3 — -U1 rider patches are offset-fragile on diverged walls

`git diff -U1` is the right seam format (small context = fewer collisions), but the single context line of a rider hunk is often the row-generic `- evidence: campaign-recorded in agenttrace ROADMAP.md` — shared by every row. On a wall that advanced by insertions, `patch -p1 -F3` folds rc0 yet can land a rider in a *neighbouring* row's span (observed: rm-401's rider landing at offset 8 inside rm-384's span on wall e130845c).

**Rule:** after ANY fuzz fold, run the placement census and re-anchor mislanded riders to their owning row's last bullet (walk from the `- id: \`rm-N\`` line to the next line starting with `#`; mind the wall-tail/EOF case). Never trust rc0 alone.
