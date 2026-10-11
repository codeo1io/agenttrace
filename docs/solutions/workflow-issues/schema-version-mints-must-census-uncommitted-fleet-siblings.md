# Schema-version mints must census uncommitted fleet siblings

**Severity class:** silent cache-schema collision across concurrent
maintenance runs.
**Origin:** run `e944a060a640` cycle 1, the 51 → 53 mint (implement attempt
`2ba22694`), 2026-10-11.
**Related:** `session-cache-schema-bump-sweeps-all-pinned-surfaces.md` (the
same-unit sweep AFTER a rung is chosen — this doc is how the rung is chosen).

## The failure shape

`SESSION_CACHE_SCHEMA_VERSION` was 51 in this tree's committed base and in
every landed ref. A sibling worktree (`run-57c491338818`, batch
`b5b85faa61c9`-family) held an **uncommitted** 52 in its working tree —
invisible to `git log`, invisible to `git ls-remote`, absent from every
landed ref. Minting "52" here (the naive next rung from the committed
ceiling) would have created two different cache schemas both claiming 52:
whichever landed second would silently serve the other's entries — exactly
the stale-totals class the bump exists to prevent, reintroduced by the bump
itself.

## The rule

**Before minting a schema rung, census the ENTIRE fleet — committed refs AND
uncommitted working trees AND spool-side patches — and mint above every
claim.** Concretely:

1. Every run worktree: `grep -h "SESSION_CACHE_SCHEMA_VERSION: i64"
   <worktree>/crates/agenttrace-core/src/session_cache.rs` — uncommitted
   edits show up only here.
2. The canonical checkout, dirty state included (it may carry foreign staged
   work).
3. The conductor delegate spool: patches and postimages may mint rungs for
   batches that have not landed anywhere yet.
4. Mint the lowest rung unique across ALL of the above. Record the census in
   the ladder-rung comment (this cycle's rung names the sibling claim it
   cleared: "this tree held the landed 51, sibling worktree
   run-57c491338818 holds an uncommitted 52 — minted 53 so every rung stays
   unique across the fleet").

Commit-time collision still needs a guard: the commit gate should re-run the
census at staging time — a sibling landing between implement and commit moves
the ceiling again, and the cheapest correct response is re-minting the whole
same-unit sweep (const, ladder, oracle, governance sentence, sweep sites)
rather than negotiating a shared rung.

## The pin that holds it

The census is reproducible from the conductor host:

```sh
for wt in /home/agent/.hermes/conductor-worktrees/agenttrace-*/run-*; do
  grep -h "SESSION_CACHE_SCHEMA_VERSION: i64" \
    "$wt/crates/agenttrace-core/src/session_cache.rs" 2>/dev/null \
    | grep -oE "[0-9]+" | xargs -I{} echo "$(basename "$wt") {}"
done
```

(any run whose minted rung equals another live claim failed this census; the
51 → 53 mint this cycle is the worked example).
