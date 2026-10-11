# Prevention rule: a reaped implement attempt may leave the fix IN THE TREE — adopt then complete, never redo blindly

- **Class:** workflow / conductor delegate attempt recovery (infra-death forensics — the implement-phase extension of the provider-reaped family)
- **Observed:** run `652a6461` (repository-maintenance `d44cb461` cycle 3), implement attempt `2ebf78f0`, provider-reaped at 10m56s (event log: turn_started → progress pings → `session_reaped` failed; no typed envelope, no scratch dir). Same family, smaller artifact: run `ff0068ca`'s dead implement attempt left exactly one verified-adoptable test; run `c762c2b8`'s reaped prioritize `b2987416` left a selection memo.
- **Cost:** two symmetric failure modes. Redoing an implement whose code already sits in the worktree burns a full turn and REPLACES reviewed-quality work with fresh code; blindly keeping the drift ships unreviewed work that was never semantically verified against the selection dossier — and may not even be the fix.

## What happened

The three-part durable-trail shape documented in `provider-reaped-delegate-attempts-redo-from-scratch-on-census-match.md` (no typed result / truncated event log / zero tree drift) has a FOURTH shape at the implement phase: **the tree drift IS the artifact.** The reaped attempt wrote its complete implementation into the worktree as uncommitted modification of exactly the batch's target files, then died before writing the envelope, the scratch, the tests, the fixtures, or the ROADMAP flips.

The discriminator is the porcelain census against the prior phase's handoff: a MISMATCH is no longer proof of contamination — it is the candidate artifact. In the observed instance the drift was confined to the three files the selection dossier named (discovery.rs / history.rs / subagents.rs), which is itself evidence of provenance: drift reproducing the batch's exact surface set is the dead attempt's work; drift outside those files is contamination to inspect separately.

## Prevention rules (adopt-then-complete protocol)

1. **Map the drift onto the batch surfaces before any verdict.** Enumerate the selection dossier's target files and the drift set; drift ⊆ batch surfaces (plus the expected new files) is a candidate adoption, anything else is inspected as foreign work first.
2. **Read the diff line-by-line against the selection dossier's acceptance criteria.** Adoption is a semantic judgment, not a file-identity match: the adopted code was re-derived against the row's acceptance clauses, the PoC recipes, and the seam notes before being trusted.
3. **RED-CONFIRM the adoption.** Revert each fix leg and watch the corresponding pin fail; restore byte-identical. An adopted diff whose legs cannot be made to fail the tests that claim to pin them is not proven to be the fix — and the completing attempt writes those pins itself (the dead attempt never wrote tests).
4. **Repair mechanical nits in place, then COMPLETE the record-keeping the reap cut short:** E2E regression tests over real fixtures, the fixture corpus itself, the CHANGELOG bullet, the ROADMAP flips, the honest comments. The completing attempt owns the record-keeping, and says so.
5. **Declare ADOPT-then-COMPLETE with the forensics legs** — event log shape, envelope absence, drift census vs the prior handoff, the line-by-line verification, the red-confirmation — in the completing phase's result AND on the row's EXECUTED bullet, so the next reader can tell adopted work from fresh work.

## Related

- Base rule family (six rules incl. "an absent envelope is not evidence no work happened; a present artifact is not proof it is valid"): `provider-reaped-delegate-attempts-redo-from-scratch-on-census-match.md` — same directory.
- Precedent instances of adoptable non-envelope artifacts: `b2987416`'s selection memo (adopted with amendments, run `c762c2b8`), the single adopted test in run `ff0068ca`'s implement redo.
- Cycle record: `docs/stewardship/2026-10-08-cycle3-compound-record-run652a6461a204.md`.
