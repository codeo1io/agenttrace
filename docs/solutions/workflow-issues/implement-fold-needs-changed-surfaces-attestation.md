# Implement folds need a changed-surfaces attestation (KTD13)

**Class:** workflow prevention · **First fleet instance recorded:** run 6cb2756a cycle 2 (2026-10-08) · **Prevention owner:** every implement-phase delegate

## Failure shape

A completed implement phase can be REJECTED AFTER THE FACT by the fold gate with:

> implement fold without a changed-surfaces attestation: the engine-derived tree delta holds N changed executable surface(s); declare them in validation_evidence.changed_surfaces (repo-relative paths you actually changed)

The delegate's tree work is fine; the PhaseResult envelope is what failed. The gate classifies the tree delta itself (git diff vs the run's base + untracked) and requires a non-empty `changed_surfaces` list whenever the delta holds at least one **executable-classified** surface.

## Root cause mechanics

- The engine's classifier (`hermes_conductor.validation_policy`) treats anything under `scripts/` (and other executable prefixes) as an **executable** surface; `crates/**.rs`, `*.md` classify **non-executable**. A batch that edits one shell script plus N Rust files yields exactly 1 executable surface.
- The implement fold branch requires: declared-surfaces non-empty when `delta.testable_surfaces` is non-empty, AND every declared EXECUTABLE path must exist in the engine's own delta (unknown-executable check).
- The rejection count ("1 changed executable surface") refers to the engine's classification — match it by deriving, not by guessing.

## Prevention rules

1. **Derive, never hand-type.** Compute the attestation with the engine's own code before emitting the result:
   ```python
   import sys; sys.path.insert(0, "/home/agent/.hermes/releases/hermes-conductor/current/src")
   from hermes_conductor.validation_policy import changed_surfaces, classify_surface, resolve_declared_surface
   base = <git rev-parse HEAD>          # never a hand-copied sha
   delta = changed_surfaces(base, worktree)
   ```
2. **Declare ALL repo-relative changed paths**, not just the executable-looking ones — the gate cross-checks declared executables against its delta and the attestation must be non-empty whenever the delta holds any testable surface.
3. **Validate the envelope before finishing:** parse the written JSON through `hermes_conductor.models.PhaseResult` and run the gate's own branch logic on the parsed object. A result that cannot be re-derived by the engine is a liability, not evidence.
4. On a KTD13 rejection: do NOT redo the phase — verify the tree is byte-intact (mtimes vs the rejected attempt's result write), re-derive with the engine's code, re-emit the envelope with the attestation, and say exactly that in the summary.

## Verified instance

Run 6cb2756a7e014e44be735d0284f29db5, implement attempts 7e7096d3 (rejected: no attestation) → 360949d2 (accepted): tree byte-identical, engine `changed_surfaces()` = 10 surfaces / 1 testable (`scripts/record-real-marketing.sh`), declaring all 10 paths → gate simulation PASS.
