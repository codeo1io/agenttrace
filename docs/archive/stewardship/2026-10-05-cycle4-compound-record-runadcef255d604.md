# Cycle-4 compound record — run adcef255d604 (repository-maintenance 6a10ae64)

Compound attempt 584fb9a8, 2026-10-05T16:48Z (host UTC; local calendar 2026-10-06),
tree 30f531a (dispatched base, unchanged), worktree `run-adcef255d604-adcef255`.
Pre-review cycle evidence only — assessment (a3958943, re-dispatch after the
cycle-3 completion; inherited evidence re-verified live rather than trusted),
research (934e1cbf), roadmap (f153840f), prioritization (d7451682), stewardship
(f1a87b0f), implement (774fb402), targeted_tests (061a6d46), full_tests
(9cc6cab7). **No validation was re-run in this phase** — the targeted and
full-suite outcomes are consumed as recorded evidence, per the compound
contract.

Prior-attempt forensics for THIS phase: attempt 5da09688 died twice on
transport (event log: two turns, 9 then 0 progress messages, both reaped).
Verified no durable work before redoing: typed result absent from the delegate
spool, no scratch directory, and worktree porcelain = exactly the 5-file
implement delta with `ROADMAP.md` at +32/−0 (the implement-phase end state).
The phase was therefore redone from scratch, and is declared so here and in
the wall's compound note.

## Cycle outcome in one paragraph

Batch "CLI entry-surface honesty" landed uncommitted at base 30f531a as a
5-file delta: rm-503 implemented (`-` reads one session stream from stdin
through `parse_stdin_bytes`, the shared decode tail of `parse_file` past the
byte read — encoding-guard wording identical to the file lane, `<stdin>`
display label, byte-identical `--overview` output pinned, empty stdin =
the empty-session error class, stdin sessions ephemeral and cache-inert);
rm-505 implemented (`statusline`/`upstream` keyword `--help`/`-h` render
per-keyword help at rc0; any other dropped flag after a keyword stays rc2
but keyword-scoped with a help pointer — never the "positional session path"
mislabel; both pinned by tests; the first statusline help draft wrongly said
"reads the journal" and was corrected against the statusline.rs contract
before validation). rm-504 (schema_version on every `-f json` document)
stays candidate — this run's own mint, deliberately unselected, recommended
below as the cycle-5 lead. Recorded validation: targeted 061a6d46 green on
every focused leg; full 9cc6cab7 green on the `.github/workflows/ci.yml`
lint+full+deny steps run verbatim (command authority — `validation.full_command`
shipped empty): **388 passed / 0 failed across 15 suites**, release build rc0,
all gate scripts rc0 with `AGENTTRACE_BIN` pinned, `ruby -c` OK, `npm test`
4/4, `cargo deny check` ok; digest
`validation:v1:3e4e182b72d59bb8b7fceb6c58a2a32b25a19eae7a7a3882ed1ff629fc4777d7`
at base `30f531a20cd612d2c7e41f4331eb4e4d215fda74` declared identically at
both validation turns. Status accounting: **170 ids = 103 candidate /
32 done / 35 implemented** (rm-503/rm-505 flipped candidate→implemented by
the implement phase with full evidence rows; done-flips reserved to the
commit gate, rm-012 precedent).

## Prevention rules (repo-durable)

**PR-1 — A new input surface enters through the EXISTING decode tail, never
a fork of it.** rm-503 did not add a second parser: `parse_stdin_bytes` is
the shared tail of `parse_file` (UTF-16-BOM bail, zstd-magic bail, UTF-8
validation, `parse_raw_session`), so the stdin lane cannot drift from the
file lane on encoding guards, and the byte-identity pin
(`stdin_dash_overview_is_byte_identical_to_the_same_file`) enforces the
whole-output equivalence. Any future byte source (URLs, sockets, `-d` fed
from a pipe) must route through the same tail, and the pinning pattern is
the byte-identity assert against the file lane — not a new golden file per
surface.

**PR-2 — Help text is a claim about code: verify it against the contract
the code enforces, and correct the draft in place with attribution.** The
first statusline keyword-help draft said the command "reads the journal";
the code reads a Claude Code statusline JSON from **stdin**, renders one
line, and **appends** to the capture journal (`--statusline-report` is the
reader). The implement pass caught this by checking the draft against
statusline.rs's contract and corrected it before validation. New help
surfaces should ship with a pinning test that asserts help and behavior
name the same things — the keyword tests pin routes and exit codes the same
way the help text describes them.

**PR-3 — An error message must name the construct the user actually
invoked.** `statusline`/`upstream` dispatch off the positional PATH slot, so
a dropped flag after them used to inherit "flag … follows the positional
session path" — a documented keyword mislabeled as a session path, with no
help route anywhere. The fix went into the ONE interception point such
keywords share (the rm-247 dropped-flag guard, `go_flag_compatible_args`),
not into a new dispatch path. Rule for future keyword commands (more host
integrations will want this shape): register the keyword's help route and
scoped error in that guard; never let a positional-slot dispatch leak
session-path error wording.

**PR-4 — Explicit-path loads stay cache-inert; that invariant is what makes
the new e2e spawns safe.** The stdin lane joins file loads in never touching
the session cache (the cache lives only in the discovery lane's
`load_sessions_with_options`), and the four new entrypoints tests spawn the
CLI unsandboxed — safe ONLY because of that invariant. The wider fleet has
observed real-cache races from discovery-lane spawns (documented by the
csv_export.rs sandbox convention). If an explicit-path load ever gains cache
awareness (e.g. stdin memoization), the entrypoints tests must adopt the
per-test sandbox pattern first; the invariant is now load-bearing for test
hygiene, not just for the ephemerality disclosure in `--help`.

## Process lessons (conductor-fleet facing)

- **Prior-attempt forensics ran three times this cycle with three different
  verdicts — the durable trail decides, never the envelope's absence alone.**
  (1) Assess was a re-dispatch of the identical attempt id after the cycle-3
  completion: the prior result still sat in `delegate/`, so the pass
  re-verified its core evidence live (384/0 re-run, gates re-run) instead of
  trusting it, then swept fresh surface. (2) Implement found its own prior
  in-flight delta in the worktree (the earlier envelope died emission-side on
  a provider API-key error): every hunk was re-verified against the
  acceptance criteria, two defects were corrected, the disposition completed.
  (3) THIS compound's prior attempt (5da09688) died transport-side twice
  leaving zero durable work (verified: no result JSON, no scratch dir,
  porcelain = implement end state) — redone from scratch and declared.
- **Empty `validation.full_command` → the repo's CI workflow is the command
  authority.** Second recorded occurrence in this repo family. The
  `.github/workflows/ci.yml` lint+full+deny job steps were executed verbatim
  (19 steps, `AGENTTRACE_CI_OUT` redirected out of the repo root) — 388/0
  across 15 suites plus every gate script, which is strictly wider than any
  hand-composed battery. `cargo-deny` 0.20.2's CLI rejects the action's
  `--all-features` flag, but the workspace declares zero `[features]`, so
  plain `cargo deny check` is identical-in-effect.
- **The contention sweep paid for itself twice.** Sibling 6aaf51aa's cycle-3
  prioritize verbatim CEDED rm-503/504/505 to this run (recorded in
  prioritize d7451682), and the compound-time host-wide re-sweep confirms
  the batch is still title-unique across every sibling worktree — no
  double-implementation anywhere, one numeric-only collision (below).
- **Ceiling churn is still the norm.** This batch's base 30f531a is now far
  behind the live fleet ceiling (sibling worktrees at 700a67c…1511547+).
  The commit gate must re-verify the wall's file:line anchors
  (main.rs:965/:182/:190, parser.rs:24) at merge time; the wall's def-row
  census at mint time said 167→170, while on-disk fleet walls now reach
  rm-515 and fleet records rm-519.

## Next-cycle context (concrete, for the cycle-5 assessment)

- **Recommended lead: rm-504 — schema_version on every `-f json` document.**
  This run minted it and live-verified its signals (top-level key dumps show
  no version discriminator on any document kind; `--baseline` consumes
  prior-run artifacts cross-version with no failure mode). It is title-clean
  across the fleet, and the now-implemented rm-503 stdin lane supplies the
  byte-source its acceptance wants: stream a fixture through `-` and assert
  `schema_version` on every document kind without a temp file.
- **Riders / candidate material for the next roadmap phase** (all
  evidence-on-record, none minted this cycle): the two assess INFOs left
  un-minted in the cycle-4 mint comment — `check-example-workflows.sh:12`
  hardcodes the single example workflow path (a second example ships
  unchecked), and doctor's sqlite provider rows key canonical DB paths only
  while counts include `~/.hermes/profiles/*/state.db` + `opencode*.db`
  (sqlite_sessions.rs:103-136), so a profiles-only host reads "missing".
- **Watch-trigger items with live evidence this cycle:** rm-006/rm-176 —
  LiteLLM main at 4,473 entries vs the bundled 2,755 pinned 2026-09-13
  (both automation triggers this family names — count delta + provider-marker
  delta — fire); rm-007/rm-044 — upstream master frozen AT v0.10.0 (compare
  b886850…master identical) with dependabot #309 carrying an 8-crate bump
  manifest to align with when the dependency wave lands; rm-017 —
  @zack78/agenttrace 0.10.0 is dist-tags latest (install-lane refresh should
  pin the 0.10.0 release assets); rm-229 stays closed (OTel
  semantic-conventions-genai still zero tags, pushed 2026-10-05T00:22Z).
- **Collision zones to respect (verified live on this host at compound
  time):** (1) NUMERIC — sibling 96762b67's wall carries its own `rm-505`
  ("CSV statement injection through unsanitized group keys", security/82.0,
  implemented-uncommitted @700a67c); title-disjoint from this run's
  keyword-help rm-505; rebind BY TITLE at integration. (2) FILE —
  sibling d6432dd5's uncommitted 'hardened reporting surfaces' lane touches
  three of this batch's four code files (main.rs, entrypoints.rs, core
  lib.rs @700a67c); 250cfd64's uncommitted lane touches entrypoints.rs.
  Sequence these merges; do not blind-fold. (3) ID SPACE — on-disk fleet
  walls reach rm-515 (84c45ccd @e1d31c4), fleet records rm-519; next mint
  ≥ rm-520 after a fresh def-row census.

## Carry-forward for the review/commit gates (do not double-implement)

- **Status flips:** rm-503 and rm-505 flip implemented → done at the commit
  gate (rm-012 precedent); their rows carry the full evidence chains
  (signals → implement → targeted → full → digest). rm-504 stays candidate.
- **CHANGELOG** entries land with the merge per lineage convention — none
  were minted at compound.
- **Digest lineage:** `validation:v1:3e4e182b…` at base 30f531a…74 was
  declared identically by targeted_tests and full_tests; the implement delta
  is code-only (crates/**), which sits outside the digest's executable-surface
  set, so the declared token is expected to hold at emission.
- **Renumber:** ids are campaign-local until merge; the rm-505 numeric
  collision with 96762b67's lane is recorded on the row and in the wall's
  compound note — rebind by TITLE.
- **Byte-copies for the commit gate:** spool scratch
  `584fb9a8…-scratch/{compound-roadmap.patch, ROADMAP.after.md,
  compound-record.md}` (authoritative) mirrored at
  `/tmp/at-compound-584fb9a8/`.
- Review and shipping outcomes are explicitly out of this phase's scope;
  the next cycle's assessment carries them forward.

## Artifacts (this phase)

- `ROADMAP.md` — compound annotations only: the `cycle 4 compound` header
  note (outcome, recorded validation, cycle-5 lead, commit-gate seams,
  next-free-id frontier), compound evidence lines on the rm-503 and rm-505
  rows, the cycle-5 lead-recommendation line on the rm-504 row, and the
  mint-comment compound addendum (fleet frontier). Status accounting
  unchanged by THIS phase: 170 ids, 103 candidate / 32 done /
  35 implemented.
- This record.
- Spool byte-copies as listed above.
