# New producer lanes lie silently — admit them against failure classes and typing, not the happy path

Run: 2026-10-15, cycle 6 of run 6d2a1d8db1ca48ef83522a8d5e6e733c
(applies to: every future agenttrace parser-lane admission)

## The pattern

All three cycle-6 rows (rm-1864 antigravity numeric model ids, rm-1865
opencode directory/projectID, rm-1866 opencode read/parse failures) share
one root cause: **a lane was admitted on its happy path, and every input
surface that deviated from the sample corpus degraded silently**:

- a field assumed `Value::String` folded numeric JSON to `None` —
  usage still banked, identity lost, disclosure never fired;
- a failure assumed impossible (`Err → continue`) vanished without a
  counter, and one of them even *changed the lane verdict* (unreadable
  message dir → "unsupported session format");
- a field present in the producer's docs but absent from our decode
  (`directory`) silently collapsed a whole rollup to `unknown`.

None of these fail a test whose fixture was built from the same happy-path
sample. The ecosystem proved each one live: ccusage PR #1894/#1897 show
numeric model ids are the *shipping* format, not a corruption edge case.

## The rule

When admitting or hardening a producer lane, enumerate every input surface
and decide each of these four explicitly — in code comments and tests, not
just intent:

1. **Typing**: for every decoded field, what happens when the JSON value is
   a number, a bool, null, an empty string? String-only decodes must be a
   decision, not a default. Where the ecosystem ships a type we don't
   accept, stringify + disclose rather than fold to `None`.
2. **Failure classes**: for every IO/parse step, classify NotFound (the
   store's own empty-container case → silent) vs every other error (→ a
   per-class lost-data counter in `Metrics::line_skips`). A failure class
   that can change the lane verdict (bail → unsupported) must be pinned by
   a test that proves it no longer does.
3. **Attribution**: fields the producer emits for attribution (cwd,
   directory, projectID, model id) must actually feed the attribution
   surfaces (`by_project`, `by_model`) — decoding them as format gates
   only is a silent lie. Verify with the producer's own fixture, end to
   end (`--overview -f json`), not a unit assertion.
4. **Disclosure routing**: value-preserving honesty counters
   (`disclosure_counters`) vs lost-data counters (`line_skips`) are
   different planes — pick per counter, and say why in a comment
   (precedent: numeric-id sightings are honesty; read/parse losses are
   losses).

## Detection recipe (assessment-time)

- grep the lane for `Value::as_str`-only decodes over producer fields that
  the ecosystem documents as loosely typed;
- grep for `let Ok(..) = .. else { continue/return Vec::new() }` around
  fan-out reads;
- run the CLI live on the repo's own fixtures and diff the rollups
  (`by_project`, `by_model`) against what the fixture's raw JSON claims
  (`"directory": "/repo"` vs `by_project: unknown` was the rm-1865 proof).

## Why this compounds

The same enumeration is the review checklist for lane patches and the
red-first recipe for their tests: each surface gets a fixture for each
class it can take. Cycle-6's three rows landed as one batch precisely
because the rule unified them; the next lane admission (opencode v2
sqlite, antigravity acp.db, whatever follows) should open with this list.
