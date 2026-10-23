# Prevention rule: a reaped attempt's "completed state" can cite artifacts it never wrote — verify every evidence citation before adopting the state

- **Class:** workflow / delegate-attempt forensics (evidence-citation integrity for adopted work; the phantom-citation extension of the provider-reaped family)
- **Observed:** 2026-10-23, run `5cf06bab` (repository-maintenance `fe743fb3` cycle 9), compound action — a four-attempt chain: `eda2dad0` (applied the roadmap band, wrote the companion record, reaped 03:39Z), `75951d3b` (rewrote the compound banner with a CORRECTION clause, reaped 06:56Z ~1 minute after the banner write), `93d38b87` (reaped 08:59Z, no durable residue), `770a28a0` (adoption + completion).
- **Cost:** two failure modes that both corrupt the durable trail. (a) A successor adopts the dead attempt's self-description — "completed state, artifacts at X" — without checking X, so the repo permanently carries citations that point at nothing, and the commit gate burns a turn hunting for evidence that never existed. (b) A successor assumes the dead attempt's claims are all unreliable and redoes finished, verified work — the symmetric waste the adopt-vs-redo rules already guard against.

## What happened

Two distinct phantom-citation classes surfaced in one chain, both caught only by first-hand checks:

1. **The reaped author cited ids that never existed.** The dead draft's banner carried the id `e3a97c42-6b0f1d85` and a `112→115` def-row census. Zero events matched either id (events/ is the complete attempt registry), and the real census was 109→112. The numbers looked plausible — right magnitude, right arithmetic shape — which is exactly why they survived into a durable repo doc until the successor's correction clause flagged them.
2. **The corrector itself cited artifacts it never wrote.** The successor banner declared "THIS banner + CENSUS.txt + APPLY-SEAM.md (delegate/75951d3b…-scratch, mirrored ~/.cache/at-compound-75951d3b/) are the completed state" — then was reaped ~1 minute later, before writing any of them. Both cited paths dangled. The banner was *right about the first phantom class* and *wrong in the same way about its own residue* in the same paragraph.

The completing attempt (`770a28a0`) verified each citation with one cheap check each (ls the cited path, grep events for the cited id, re-run the census command) before re-materializing the promised artifacts under its own attempt id and amending both the banner and the companion record to name the paths that actually exist.

## Prevention rules

1. **Treat every evidence citation in an unenveloped attempt's output as a claim, not a fact.** The dead attempt wrote prose, not a PhaseResult; nothing it says about its own artifacts is evidence. Cited scratch dir → `ls` it. Cited attempt id → grep events/. Cited census/count → re-run the command.
2. **Verification must be cheap enough to always run.** Every citation class here resolves with one command; if a claim cannot be re-checked in one step, that itself is a finding.
3. **The corrector inherits the rule.** An attempt that corrects a predecessor's phantom claims is still an unenveloped attempt until its own envelope lands — verify *its* citations with the same skepticism, especially when its correction was itself correct. Being right about class 1 confers no immunity to class 2.
4. **Re-materialize promised-but-absent artifacts under the completing attempt's own id, never under the dead attempt's name.** Fabricating a dead attempt's scratch directory to make its citation true is evidence-tampering; writing the artifacts yourself under your id and amending the citation is completion.
5. **Amend the durable doc in place.** The repo artifact (banner, record) is the thing future cycles read; a correction that lives only in the phase envelope leaves the repo carrying the phantom. Fix the citation where it lives, keep the original claim visible as a recorded correction (newest evidence governs; both records kept, per the integration-banner convention).

## Related

- Base family: `provider-reaped-delegate-attempts-redo-from-scratch-on-census-match.md` (this directory) — the census-match discriminator and the "absent envelope ≠ absent work" rule this chain exercised four times in a row.
- Adopt-vs-redo at implement: `reaped-implement-attempts-leave-the-fix-in-the-tree-adopt-then-complete.md`; per-row audit: `provider-dead-attempt-left-durable-delta-audit-then-adopt-or-redo.md`; tree-edit verification: `dead-validation-attempts-can-leave-tree-edits-verify-by-content.md` — this rule adds the evidence-pointer dimension those four cover.
- Cycle record: `docs/stewardship/2026-10-23-cycle9-compound-record-run5cf06bab5d63.md` (attempt chain, census battery, correction trail).
