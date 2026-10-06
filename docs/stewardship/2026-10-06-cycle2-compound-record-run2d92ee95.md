# Cycle 2 compound record — run 2d92ee95, repository-maintenance 52493714

2026-10-06 · compound attempt 748e67e1 · worktree run-2d92ee95f2ed-2d92ee95 @ base 1511547 · pre-review

Batch: **session-cache store integrity** — rm-292 (lead, minted-as-implemented here,
provenance in-row) + rm-041 (flipped implemented) + rm-298-residual (closed in-row).
Implement 39c5c7bc; targeted c3c67eec; full 15ef0829. All validation numbers below are
consumed from those recorded outcomes — nothing was re-run at compound.

## Outcome summary

- rm-292: schema/pricing mismatch now invalidates entries WITH a bounded persisted
  `schema_invalidations` ledger instead of silently discarding the store; dir listings
  survive a schema bump when the walk lane + pricing identity match and drop-and-rewalk
  once when the lane moves; the store self-heals forward. Live PoC: schema flip 26→24
  between invocations keeps `--sessions` identical while the ledger records
  `{observed 24, schema 26, dropped_entries 2, kept_dir_listings 1}`.
- rm-041: lossless `at-bytes:` percent-encoded cache keys + listing members (non-UTF-8
  journals no longer vanish/collide); listing freshness compares directory size (same-tick
  creates); `DIR_LISTING_WALK_VERSION` 3→4.
- rm-298 residual: save/clear serialize under an advisory lock with content-first
  staleness and a fenced rename; 8 concurrent CLI processes over 4 corpora lose zero
  entries (base behavior: last-writer-wins).
- Validation: targeted 181/0 core lib (all 8 batch tests) + 78/0 + 47/0 + 13/0, fmt/clippy
  clean, 4× stability loop; full `cargo test --workspace --all-features` 18 suites 433/0
  (base 425 `#[test]` fns + exactly this batch's 8); digest
  `validation:v1:0ca09f019eb888c756ff8e98926c120bbfb2067d89136e1652957f6b62522158`
  stable across folds (uncommitted crates/** + .md delta is never executable-classified).

## Prevention rules (PR-1..PR-5)

**PR-1 — When a cached format's member/key ENCODING changes, bump the replay-gating lane,
not just the schema version.** Schema version gates entry restore; the walk lane gates
listing replay. This cycle: rm-041's lossless re-encoding shipped initially WITHOUT the
lane bump, so legacy lossy-member listings would have replayed verbatim THROUGH the new
rm-292 restore path — hiding the exact non-UTF-8 journals the encoding existed to surface.
The module's own v2→v3 precedent (a schema-22 journal carrying stale v2 listings)
documents the trap; it was still walked into. Red-first tests for lane bumps must seed the
ABSOLUTE legacy lane value, not `LANE - 1`: a const-relative seed self-adjusts when the
constant is pinned back and passes red (caught live this cycle when the red-first proof
did not fail).

**PR-2 — Advisory-file-lock design: staleness must fire BEFORE wait-budget exhaustion,
be content-first, and critical sections must be fenced.** The adopted draft had
wait 2s < stale 10s: a starved waiter fell back to an UNLOCKED save (reopening the
lost-update window the lock existed to close) before the safe steal could fire — its own
concurrency test flaked ~1-in-4 under full-suite load ("writer-2's entry was lost").
Also, a pure-wait lock wedges on ANY stray file at the lock path (proven: WAIT=120s
constants → 10/10 pass but every run exactly ~120.5s). The landed protocol: steal a lock
naming a dead process or holding non-token content IMMEDIATELY; respect a fresh empty
file for 1s (live creator); steal a wedged-but-alive holder past 5s; wait budget 30s >
steal threshold; before the atomic rename, re-check that the token is still ours and
re-serialize around the thief's on-disk state if not (3 attempts, then land anyway —
permanently dropping the process's own entries is worse than a fenced fall-through).

**PR-3 — Provider-dead prior attempts leave real work behind: verify, then adopt, never
blindly redo.** Attempt d9322339 died mid-turn with no typed result, but its 6-file
uncommitted diff sat exactly on this batch. The adoption protocol that worked: read the
event log for run/action lineage; read the full diff; compile it; re-run its suites
first-hand; THEN adopt and adversarially re-review the adopted work as if it were your
own (the re-review found 3 real defects — PR-1/PR-2 above — which the original attempt
would have shipped). Adoption preserved the large majority of the implementation.

**PR-4 — Reconcile test tallies against git-level ground truth before citing deltas.**
The assess headline recorded "18 suites, 370 passed" for base 1511547, but the tree
holds 425 `#[test]` fns at that commit (`git grep -c '#\[test\]' 1511547 -- crates/`).
The full_tests phase proved the discrepancy by counting base fns vs working-tree fns vs
passing tests (425 + 8 batch tests = 433 passing = exact match). Headline tallies in
PhaseResults are convenience artifacts; when a count looks wrong, count fns in git, not
just re-run suites.

**PR-5 — Low numerals need a provenance sweep before mint or re-materialization.**
rm-292 existed in NO wall (not this tree, not HEAD, not origin/master, not any of ~30
live worktrees) yet was selected from a rendered table citing the numeral — its original
home (7dc3899d's unlanded band rm-284..rm-292, 2026-10-02) survives only in sibling
spool censuses with no subject text. The compound re-materialized the numeral on this
wall WITH an in-row provenance bullet and a title-reconciliation instruction (fleet
convention 202c4d1d: integration merges by title, loser renumbers). Without that bullet,
a future cycle would face an orphaned numeral again.

## Merge-gate seams (reserved for commit/merge, NOT done here)

1. `SESSION_CACHE_SCHEMA_VERSION`: this batch lands on 26; origin/master's
   session_cache.rs is already **27**. Reconcile at the gate — the new restore path is
   precisely what makes the bump non-destructive (mismatch → disclosed invalidation,
   listings survive when the walk lane matches). Ceiling walk lane is still 3, so the
   3→4 lane bump lands cleanly.
2. Done-flips for rm-292 and rm-041 + CHANGELOG entries for the batch, at the commit gate
   per convention.
3. ROADMAP delta this cycle: roadmap phase +24, compound +banner +3 row-edits +1 mint.

## Next-cycle leads

- **rm-539** (render-lane terminal-injection superset, priority 86): Cc→Cf character-class
  extension + every format lane + TUI choke points; rm-239's residual ceded to it in-row.
- **rm-540** (discovery partial-failure disclosure, 55): unreadable subtrees vanish rc=0.
- **lru 0.18.1 → 0.18.2** LOW rider (RUSTSEC-2026-0253, transitive via ratatui-core) from
  the roadmap phase's OSV census — ride it at the next deps touch.
- rm-298's thrash axis (16bbd3ae A5: deterministic eviction order re-parses ~918/run at
  over-bound sizes) remains open under rm-298.
- Watch (from roadmap banner): pi 1.0.4 empty-body release; codeburn #1540/#1579 open;
  upstream #312 accounting port (high-water → sum-once) conflicts with rm-035's landed
  acceptance mechanism — re-open the accounting contract when porting.

## Artifacts

- ROADMAP.md delta (this cycle's roadmap + compound edits, uncommitted, for the gate)
- /tmp/at-impl-39c5c7bc (PoC corpora + caches), /tmp/at-targeted-c3c67eec, /tmp/at-full-15ef0829
- Spool: delegate/39c5c7bcef854bcea1628b7fe247451b.json, delegate/c3c67eecb01e470987589ec30392fc86.json,
  delegate/15ef0829b85b44d7b65bcc2666a3c9d3.json, 39c5c7bcef854bcea1628b7fe247451b-scratch/implement-audit.md
