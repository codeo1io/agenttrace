# Prevention rule: conductor validation evidence is a verbatim contract

**Class:** workflow — conductor validation phases (targeted_tests, full_tests) and their typed
result envelopes. Not a product-code defect.

**Observed:** 2026-10-08/09, run e9dbcbc2428d4bbeb7e5a3d541c81d1a (repository-maintenance
1754b828247449b8b59aa46b775b5d72 cycle 1), base 65f9f63.

**Cost:**

1. A `full_tests` attempt was **fold-rejected after completion** on a command-string mismatch
   ("command mismatch (declared python3 …local_validation_)") *while the suite itself was green* —
   the local validation gate's own envelopes on file show `cargo test` rc0 in that worktree. A whole
   phase retry (full-suite re-run on a loaded host) was burned for a transcription defect in the
   typed `validation_evidence.command` field.
2. The same run's `targeted_tests` work order reported `changed_testable_surfaces=[]` /
   `required_scope=none` for a large `crates/`-tree delta — the surface classifier's
   EXECUTABLE_PREFIXES list covers only top-level `src/ | lib/ | tests/ | scripts/ | bench/`
   `.github/workflows`, so a `crates/<crate>/...` change classifies as `non-executable`. The phase
   would have validated nothing had the real surfaces not been derived manually from the implement
   envelope + `git diff`.
3. A mistyped `cargo test -p agenttrace-cli` filter (directory name instead of package name)
   returns rc101 "package ID specification did not match any packages" and runs **zero tests** —
   an easy false green if the exit path alone is read.

## What happened

The fold gate consumes the **structured PhaseResult fields**, not the gate's result envelope and
not the console transcript. `validation_evidence.command` must equal the dispatch
`full_command`/`targeted_command` **byte-for-byte**; `scope` and `outcome` must be the literal
tokens; `validation_digest` must be the work-order digest verbatim (or, for fix/test-bearing
phases that changed executable surfaces, the digest the validation command prints at emission
time). Each of those was independently violated or nearly violated this cycle by retyping instead
of copying.

## Prevention rules

1. **Copy, never retype, the command.** Take `full_command`/`targeted_command` from the work
   order's validation block character-for-character. Before writing the result JSON, verify the
   equality programmatically (parse the written JSON, compare against the dispatch string) — this
   run's accepted retry did exactly that check.
2. **Treat `changed_testable_surfaces=[]` on a `crates/<crate>/` delta as classifier blindness,
   not as "nothing to test."** Derive the real testable surfaces from the implement phase's
   envelope + `git diff --stat`, and say so in the phase record.
3. **`cargo -p` filters take package names, not directory names.** `grep -m1 '^name'
   <crate>/Cargo.toml` first. rc101 with zero result lines is a **no-op**, never a pass; always
   confirm the expected `test result:` lines actually appear in the log.
4. **Know which digest is which.** The local validation gate's envelope digest is derived with
   base=unknown and is never the engine token. The engine digest is
   `validation_digest(<HEAD full sha>, worktree)`; `crates/**` and `docs/**` deltas are
   digest-immobile, so for crate-only deltas the dispatch digest stays the correct verbatim
   declaration — re-derive (don't assume) whenever `scripts/`, the root manifest, the lockfile, or
   `.github/` moved.

**Related:** the string-anchored-edits prevention rule (this directory) — the same
copy-don't-reconstruct discipline for patch anchors; fleet precedent bbde21568cd4 (empty
dispatch full_command ⇒ ci.yml lane mirror is authoritative); this run's cycle record
`docs/stewardship/2026-10-09-cycle1-compound-record-rune9dbcbc2428d.md`.
