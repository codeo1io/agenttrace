# Cycle-1 compound record — run 66a7d797 (repository-maintenance f89f5223)

Compound attempt 5b37d3d4, 2026-10-05, tree ea5c41e (base; unchanged), worktree
`run-66a7d7974046-66a7d797`. Pre-review cycle evidence only — assessment
(d282b81f; dead-attempt forensics on 7719332c declared), research (a0ab9c1d),
roadmap (add00e29, adopting delivery-reaped f6d7247f), prioritization
(85fc2335), stewardship (1db3e5d4), implement (6861329c, fold-repaired by
7eff1c1c), targeted_tests (b2082236), full_tests (af206e05). **No validation
was re-run in this phase** — the targeted and full-suite outcomes are consumed
as recorded evidence, per the compound contract.

## Cycle outcome in one paragraph

Batch "Provable parser truthfulness" landed uncommitted: rm-450 implemented
(workbuddy `input_tokens` basis disclosed via `workbuddy_input_basis:*`
counters riding the existing `Metrics.line_skips` channel + a reports.rs
provenance note + `SESSION_CACHE_SCHEMA_VERSION` 22→23 so stale caches
regenerate instead of masking the counters; red-first on the mismatch fixture:
silent `tokens_input 0 / Dropped lines 0 / confidence high` → green:
disclosure present + confidence downgraded; arithmetic unchanged pending
upstream #310); rm-452 implemented (seeded property harness
`tests/parser_property_invariants.rs`, zero new dependencies — xorshift64*
generator with `AGENTTRACE_PROPERTY_SEED`/`AGENTTRACE_PROPERTY_CASES` knobs;
three invariant classes; the dual-key class ceded to rm-449); the rm-017
drift-gate rider landed (`scripts/ci/check-install-ref-drift.sh` + ci.yml step,
green live v0.9.0==v0.9.0; rm-017's remaining scope untouched). Validation:
376 passed / 0 failed twice — targeted (engine-seeded impacted-tests command
verbatim → documented fallback) and full (`full_command` verbatim) — fmt/clippy
clean, digest `validation:v1:251dcfc3…` re-derived byte-exact against the
dispatch base. Wall: 155 def rows, 107 candidate / 29 done / 33 implemented.

## Prevention rules (repo-durable, reuse verbatim)

**PR-1 — Any change to a persisted `Metrics`/cache-derived field's meaning or
key set bumps `SESSION_CACHE_SCHEMA_VERSION` in the same delta.** The
workbuddy disclosure counters ride `Metrics.line_skips`; without the 22→23
bump, a pre-existing session cache (built at schema 22 without those keys)
replays old metrics and the new disclosure never surfaces — the fix silently
does not apply to previously scanned corpora (same class as the 20→21 bump
recorded on rm-230's integration). Demonstrated live this cycle: a cache
written pre-bump showed 0 cache hits post-bump and re-parsed with the
disclosure present. Rule: bump is part of the change, not a follow-up; a
test pinning the version (discovery_contract.rs) rides along.

**PR-2 — Offline-registry property testing is a seeded harness, not a skipped
acceptance leg.** crates.io was unreachable this cycle, so rm-452 landed as a
deterministic xorshift64\* harness (env knobs, printed seed) with the proptest
swap recorded as a residual acceptance leg on the row. The invariants
(arbitrary-input non-panic, key-permutation invariance, detector mutual
exclusion) are the durable part; the framework is interchangeable. Rule: when
a dependency is unavailable, implement the property deterministically and
document the knob contract — never defer the invariant itself.

**PR-3 — Suspected-basis defects get disclosure now, arithmetic later.** While
upstream #310 is unresolved, the workbuddy subtraction stays as-is and the
assumption is disclosed on every surface it touches (counter + provenance note
+ confidence downgrade). This extends the rm-304 verify-first pattern to the
class where the *true* semantics are owned by an open upstream decision:
changing arithmetic ahead of that pin would guess, and guessing silently is
the exact defect rm-450 exists to prevent. Flip only when upstream pins (watch
on the row).

## Process lessons (conductor-fleet facing)

- **A fold rejection names its defect — repair exactly that.** The implement
  fold rejected 6861329c for a missing `changed_surfaces` attestation
  (KTD13), not the work. The repair (7eff1c1c) verified the tree
  byte-identical against the rejected attempt's recorded diff sha
  (`1eebd341…`), re-ran only the focused battery, and re-filed with the
  attestation — no re-implementation, no scope drift. The engine's
  `validation_policy.apply_validation_gates` can be imported offline and fed
  a synthetic envelope to reproduce a rejection verbatim before re-filing.
- **Declare the re-derived digest, never a runner-printed one.** Applied the
  standing rule (docs/solutions/process-issues/
  validation-digest-base-and-coverage-reconciliation.md) end-to-end this
  cycle: the standalone gate envelope printed its own unknown-base digest
  (`8ca2443b…`) on every run; the declared value was the dispatch-base
  re-derivation (`251dcfc3…`), proven byte-exact twice. The only
  digest-visible delta files here are `scripts/ci/*.sh` and
  `.github/workflows/*.yml` — a crates-only batch would not have moved it.
- **Adoption beats redo when a reaped session left durable work.** Roadmap
  attempt f6d7247f completed but was reaped at harvest; the re-dispatch
  (add00e29) adopted the surviving wall delta after first-hand re-verification
  (2 citation corrections applied to rm-451) rather than re-minting —
  precedent recorded in the wall's mint comment.
- **Triple-claimed bands are carried, not renumbered mid-cycle.** rm-448..452
  is claimed by three lineages (2d37535d, 7197db2a, b1ff12f8, this run);
  the band stays unrenumbered because later sibling records already cite the
  numerals; integration reconciles BY TITLE (carried-collision precedent
  7aa0c31d × a161bf24).

## Next-cycle context (concrete, for the cycle-2 assessment)

- **Recommended lead: rm-454** — the v0.10.0 upstream port (12 commits past
  the fork's last-known 52ab2cd, incl. `--blocks`/`--daily`/`--weekly`/
  `--monthly`/`--tz` per research a0ab9c1d; evaluate adopting upstream's
  implementation under rm-448 before building fork-local versions, per the
  note already on rm-042).
- **Alternate: rm-453** (rust-i18n-style `--lang` parity lane — the assess
  F2 fold showed `--lang zh` remains a byte-identical no-op), then **rm-456**.
- **Dedicated later cycle: the rm-448 wave** — v0.9.1+v0.10.0 integration,
  the rm-449 dual-key port (~5 lines + fixture; cheapest correctness win,
  but the property harness deliberately excluded that class this cycle to
  avoid colliding with sibling 2d37535d's in-flight lane), and rm-451
  (parse-time admission bound) land together with the rm-007/rm-044
  dependency lanes to avoid post-merge lockfile churn (R9 composition note).
- **Deferred rows already prioritized** (85fc2335): rm-455 / rm-457 / rm-458
  after the wave.
- **Collision zones to respect:** parser.rs qwen lane (2d37535d in flight),
  discovery.rs admission lane (7197db2a in flight), b1ff12f8's queued
  "Trustworthy accounting" batch overlaps rm-448/449/450/451 by title —
  this run's rm-450/rm-452 EXECUTED bullets are the landed-record side of
  that reconciliation.

## Carry-forward for the review/commit gates (do not double-implement)

- **Review must check the schema-bump reasoning** (PR-1): 22→23 is a
  masking-fix, not a format change — entries re-parse once; the tui fixture
  and discovery_contract version pins moved with it, and
  docs/guides/governance-reports.md states schema 23.
  *(Integration note, 2026-10-05, conflict case 3d25a9e9, merge of run
  66a7d797 into the run-6403d975 head: the campaign landed against the
  schema-22 base and bumped 22→23 there; the integrated tree re-based the
  same bump onto the already-advanced ceiling — 24→25 — per the rm-230
  convention, so session_cache.rs, the tui fixture, the
  discovery_contract pins and governance-reports.md all read 25 in the
  merged tree. One invalidation either way; the masking-fix intent is
  unchanged.)*
- **Do not flip rm-450's arithmetic** ahead of upstream #310 (PR-3); the
  row's watch is the flip condition. Done-flips for rm-450/rm-452 stay
  reserved for the commit gate after independent review.
- **Do not "upgrade" the property harness to proptest** inside a review-fix
  pass — it is a dependency decision (registry offline this cycle), recorded
  as rm-452's residual acceptance leg.
- **Integration sequencing:** this run's ROADMAP compound postimage applies
  after the roadmap-phase delta (same file, additive hunks only); the
  implement delta's shared anchors are session_cache.rs:8 (version constant)
  and discovery_contract.rs EOF — check sibling 7197db2a's parked
  discovery_contract additions for the same EOF anchor before merging.
- **Uncommitted state is the deliverable**: 8 M + 4 ?? (diff sha
  `1eebd341…` for tracked), HEAD ea5c41e, porcelain fingerprint recorded in
  the implement/targeted/full records.
- Review outcomes are explicitly out of this phase's scope; the next cycle's
  assessment carries them forward.

## Artifacts (this phase)

- `ROADMAP.md` — compound annotations only: rm-450 + rm-452 flipped
  candidate→implemented with EXECUTED bullets transcribed from recorded
  pre-review evidence; rm-017 rider note (status unchanged, candidate);
  compound c1 comment in the Open-items header. No ids minted; no other rows
  touched. Status accounting: 155 def rows → 107 candidate / 29 done /
  33 implemented.
- This record.
