# serde-skipped report fields must survive the session-cache round trip

**First seen:** 2026-10-15, run abb3b0914e48 cycle 3 (rm-1385) — but the trap
was set 2026-06 when `Metrics.timestamps` gained `#[serde(skip)]`.

## The trap

`Metrics` (crates/agenttrace-core/src/lib.rs) marks per-event timestamps
`#[serde(skip)]` to keep report JSON compact. The session cache serializes
sessions through a compact carrier (`GoSession`/`GoMetrics`) that predates and
does not mirror every report field. Result: `into_metrics` reconstructed
sessions with `timestamps = vec![]` on every warm load.

Nothing crashed and nothing tested it, because:

- cold-path tests always parse from journals (timestamps present);
- the quota report does not require timestamps — `quota_cost_share` silently
  falls back from the precise event-count basis to the coarse span-seconds
  basis when stamps are empty, and the census field was only added later.

So the SAME corpus + command reported **different spend depending on whether
the cache was warm**: $0.0016 cold vs $0.0012 warm on a 22k-file fleet corpus
(−25%); an 80-file PoC flipped the disclosed census basis
`event_count → span_seconds` outright.

## The rule

A `#[serde(skip)]` on a report field is a **wire-format decision, not a
statement that the field is reconstructible**. Every derived view that branches
on that field must survive every persistence layer the session travels through:

1. Enumerate the round trips: report JSON (the skip is FOR this one), session
   cache journal, sqlite snapshot lanes, MCP/TUI carriers.
2. For each trip: either carry the field or make the absence truthful (the
   census-fallback pattern — disclose which basis produced the number).
3. Pin it with a warm/cold parity test that asserts BOTH the number and the
   disclosed basis, not just the number. A number-only pin passes while the
   basis quietly degrades if any consumer later changes its fallback.
4. When the cache carrier gains the field, that is a schema bump — sweep the
   pinned oracle same-unit (see
   `session-cache-schema-bump-sweeps-all-pinned-surfaces.md`).

## How it is pinned now

- `event_timestamps_survive_the_cache_round_trip_exactly` — carrier fidelity
  (incl. a legacy-shape entry that legitimately deserializes to empty).
- `warm_quota_basis_matches_cold_across_the_journal_round_trip` — the parity
  oracle: warm == cold on spend AND census basis, journal-backed.
- E2E shape: run `--quota -f json` cold, then with `AGENTTRACE_SESSION_CACHE_DIR`
  banked, then warm — all three byte-identical on
  `{five_hour.spend_usd, weekly.spend_usd, corpus.attribution}`.

## Generalization

Audit every other `#[serde(skip)]` on a struct that flows through the cache:
if a consumer branches on the skipped field, the same silent-basis-drift bug is
available to it. (grep anchor: `serde(skip)` in lib.rs — at fix time,
`Metrics.timestamps` was the only one on a branch-consumed field.)
