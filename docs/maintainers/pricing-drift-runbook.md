# Pricing Drift Runbook (rm-1957)

How to respond when the weekly `pricing-drift` scheduled workflow fires. The
gate compares the bundled offline catalog
(`crates/agenttrace-core/src/pricing_snapshot.json`) against the live LiteLLM
catalog on the builder keep-filter basis (costed chat models, no glob keys).
Full mechanics: `scripts/pricing/drift-check.sh --help`.

## Reading the report

`drift-report.txt` (workflow artifact) or `scripts/pricing/drift-check.sh --json`:

| Field | Meaning |
| --- | --- |
| `new_costed` | live models the bundle does not price yet — normal growth, ride the bump PR |
| `rate_changed` / `deprecation_new` / `context_window_changed` / `provider_changed` | normal drift classes, ride the bump PR |
| `retired` | bundled models live no longer prices — **requires adjudication** |
| `retired_documented` / `retired_undocumented` | the rm-1957 split of `retired` by manifest evidence |
| `removal_guard` | SET iff `retired_undocumented > 0` — blocks the auto-PR |
| `documented_drops.retired` | the manifest notes (date + reason) sanctioning each documented key |

## Adjudicating a retirement (the only path that clears the guard)

The manual-drop manifest `scripts/pricing/snapshot-drops.txt` (rm-588) is the
one sanctioned way a price leaves the bundle. For each `retired` key you have
evidence is really gone upstream (deprecation notice, rename with successor,
provider EOL):

1. Add one line to `scripts/pricing/snapshot-drops.txt`:

   ```
   YYYY-MM-DD <exact-bundle-key> <human reason...>
   ```

   The key must match the bundle key byte-for-byte (not a normalized form).
   Date the note on or before the bundle's `_snapshot.date` — a future-dated
   note has not sanctioned the current bundle and the guard stays SET for that
   key until a refresh stamped on/after the note's date.

2. A note is an *adjudication*, not a removal: the key leaves the bundle only
   when `scripts/pricing/update-snapshot.sh` runs next (it refuses to run with
   malformed notes — fail-closed — and prints `manual drop honored (<date>)`
   for each fired note).

3. Prefer the union's default (retain the stale rate) over a note unless you
   have real evidence the model is gone: a stale rate with a
   `deprecation_date` still discloses; a missing entry silently falls back to
   default pricing. Keep the list SHORT and justified — each entry is a model
   users may still be billed for.

## Why documented retirements still report as drift

A documented retirement keeps `retired >= 1` and `actionable_drift: true`
(rc 1) until the refresh drops the key: the bundle is stale, and the bump PR
is the vehicle that refreshes it. The guard is the *silent-unpricing*
tripwire, not a staleness tripwire — adjudicated removals ride the PR;
undocumented ones block it (`rm-176`: catalog growth stays additive).

## First-fire walk (the 2026-10-20 window)

1. `scripts/pricing/drift-check.sh --json` — expect `new_costed > 0` (growth)
   and a large `retired` (the 2026-10-08 bundle predates upstream churn).
2. Triage `retired` into evidence-backed removals (notes) and suspects. Keys
   still priced under a successor name get a note citing the rename; keys with
   no trail stay undocumented and keep the guard SET — do not paper over them.
3. Refresh (`scripts/pricing/update-snapshot.sh`), bump
   `PRICING_SNAPSHOT_DATE` in `crates/agenttrace-core/src/pricing.rs` to the
   new `_snapshot.date`, and let the census pins move with it
   (`pricing_snapshot_hygiene`, `cache_rate_truth_contract`).
4. Re-run `drift-check.sh` — `retired_undocumented: 0` and `removal_guard:
   false` is the green state that re-arms the auto-PR lane.

## Selftest

`scripts/pricing/drift-check.sh --selftest` — ten offline arms; arms 9–10 pin
the manifest lane (documented drop clears the guard while staying actionable;
future-dated notes keep it SET; malformed notes fail CLOSED as rc 2).
