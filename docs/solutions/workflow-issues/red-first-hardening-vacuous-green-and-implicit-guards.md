# Red-first hardening fixes: vacuous-green races and implicit platform guards

Learned 2026-10-09, run a2abf1a8 (repository-maintenance cycle 1, batch rm-818..rm-822,
implement attempt dd10a972). Companion record: `docs/stewardship/2026-10-09-cycle1-compound-record-runa2abf1a8af3b.md`.

Three traps recur whenever the change under test is a *hardening* fix (the code already
"works" on happy paths; the test must prove the hostile path changed). Each trap produces a
test that is green for the wrong reason — and each has a cheap structural counter.

## 1. Vacuous-green race harnesses

**Trap.** A red-first race harness that loops `open → write → assert-not-in-victim` looks
conclusive, but any error path that exits the loop early converts "the attack was blocked"
into "the test stopped trying". In run a2abf1a8 the O_NOFOLLOW constant was fat-fingered as
`0o40000` (O_DIRECT, not O_NOFOLLOW's `0o400000`); every append failed with EINVAL, the
loop broke on write error, and the harness passed against the broken code.

**Rule.**
- Race harnesses must **panic on every error**, never `break`/`continue` on failure:
  `writeln!(f, …).unwrap_or_else(|e| panic!("append under race failed: {e}"))`.
- Prove the harness exercised the happy path at least once (assert a minimum success
  count), so "everything failed" cannot masquerade as "everything passed".
- When the kernel constant must be hand-written (no libc dep), assert it against a
  known-good pair in a comment and pin it with a test that exercises the *success* path
  (a normal append must still succeed with the flag set).

**Reference.** `journal_append_never_follows_a_link_swapped_mid_open`
(crates/agenttrace-core/src/session_cache.rs) and its in-test note naming the O_DIRECT
incident.

## 2. Implicit platform guards (behavior passes pre-fix)

**Trap.** A behavioral test can pass against unfixed code because a dependency already
neutralizes the hostile input by accident. The agenttrace TUI pushed unsanitized
transcript strings into ratatui spans, but the locked ratatui **skips zero-width symbols
when writing spans** — ESC/BEL/C1 are all zero-width, so a "walk the TestBackend buffer
for control bytes" test was green *before* the sanitization fix. The fix was still
correct (the safety was one ratatui upgrade from vanishing), but the test proved nothing
about the fix.

**Rule.**
- When a behavioral test cannot be made to fail pre-fix, do not ship it as the red-first
  proof. Add a **structural source pin**: grep-count assertions that the flagged call
  sites route through the guard (e.g. "explorer.rs contains ≥5 `crate::sanitize_line_segment`
  call sites; lib.rs re-exports the family"), which is red on exactly the pre-fix tree.
- Keep the behavioral walk anyway, as the regression net for the *combination* "routing
  reverted × implicit guard removed" — and document in the test that it passes pre-fix
  under the current dependency version.
- Write down the implicit dependency (ours: ratatui zero-width skip) in the test comment;
  it is the thing an upgrade can silently change.

**Reference.** `poisoned_journal_cannot_drive_the_terminal_past_the_frame` +
`tui_render_sources_route_the_shared_sanitizer` (crates/agenttrace-tui/src/tests.rs).

## 3. Unorderable concurrency assertions

**Trap.** Asserting equivalence between an *observer's* view of shared state and the
*subject's* behavior is unsound when the state can change between the two reads. We first
asserted "render marks a torn tail iff my snapshot ends torn" — false when the journal
completes (or tears) between my read and the render's read. The soak gate (10 full-suite
runs) caught it as a ~40% flake.

**Rule.**
- Split the property: a **deterministic phase** where the writer is paused mid-write
  (body flushed without its newline, held open) carries the marker assertion; a **stress
  phase** where a live writer races the reader carries only panic-freedom and
  no-corruption assertions (always-acceptable outcomes), never iff-equivalence.
- Treat a soak-gate flake in a *new* concurrency test as the test's bug first, not the
  fix's regression — but re-derive the soak from the fixed tree before concluding that
  (ours: 10/10 green after restructuring).

**Reference.** `concurrent_appends_render_without_panic_and_mark_torn_tails`
(crates/agenttrace-core/src/statusline.rs), phase-1/phase-2 structure with the TOCTOU
note.

## Process corollary

The acceptance line in the roadmap row is the contract, not the batch summary. In this
cycle a percent-rounding change was briefly built under a CSV-marker row's id because the
batch summary had drifted from the row; re-reading `acceptance:` mid-implement caught it
and the wrong work was fully reverted (grep-clean) before the real work landed. Read the
row before you write the code.
