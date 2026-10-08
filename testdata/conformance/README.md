# Token-accounting conformance pack (rm-053)

A checked-in fixture pack + manifest that pins the **truth of token and
cost totals** across the parser and pricing lanes. It exists to fail on
any *silent double-count, mis-price, or unprovable total* — the five
recurring billing-bug classes the [AgentMeasure 2026-09
audit](https://github.com/roy-tong/AgentMeasure) measured across ~110
usage tools:

| audited class | pack cases | guard |
| --- | --- | --- |
| re-emitted/resumed events double-counted | `007` | usage re-emitted on the same message id counts once; double-count would render 3500/350 instead of 1500/150 |
| cache-tokens priced as input | `005`, `008` | cache write/read price at their own snapshot rates and stay out of net input (`0.0135`, not the `0.0255`/`0.027` misprices); the workbuddy cache-inclusive input basis subtracts to a disclosed zero |
| price-table drift vs vendor consoles | `009`, `012`, `013` | every cost literal is re-derived from the bundled snapshot at run time; the frozen-first-model misprice is xfail-pinned to rm-663 |
| resume/fork lineage loss | `010` | post-compaction growth counts forward once per window (`1450/610`, not the `1000/400` refusal undercount nor the `1800/760` cumulative double count) |
| overflow/clamp handling | `001`–`003` | i64::MAX-scale totals saturate with the `calculated_from_tokens_clamped` marker, never panic or wrap; an unprovable hostile journal fails loudly |

plus two pack extensions for the acceptance's "unprovable total" arm:
`004`/`011` (partial/zero evidence renders as zero *with a disclosure
marker*, never padded by estimates) and `003`/`006` (loud nonzero exit,
never a silent zero-total session).

## Layout

- `manifest.json` — one entry per case: class, fixture, source
  provenance, expected totals. This file is the contract.
- `cases/*.jsonl` — fixtures. `rm046-*.jsonl` are **verbatim imports of
  the landed rm-046 adversarial corpus**
  (`testdata/generated/adversarial/`); the byte-identity is asserted by
  the harness, so the corpus can never be silently regenerated through
  this pack.

## Pricing rule (no second price table)

Expected costs are **totals** derived from the bundled snapshot
(`crates/agenttrace-core/src/pricing_snapshot.json`, recorded derivation
date in the manifest) via the catalog formula in
`crates/agenttrace-core/src/lib.rs` (`round4` of the per-class
token × rate sum, cache classes at their own rates). The pack embeds no
price table of its own: the harness re-derives every non-xfail literal
from `lookup_price`/the builtin default at run time and fails when a
literal drifts more than one `round4` step (`6e-5`) from the live
snapshot — so a snapshot refresh that moves a used model's rates forces
a pack refresh here instead of silently re-blessing old totals.

## Running

```sh
# library half (parser + pricing directly)
cargo test --locked -p agenttrace-core --test token_conformance

# binary half (end-to-end `--sessions -f json` wire shape)
cargo build -p agenttrace && AGENTTRACE_BIN=target/debug/agenttrace scripts/conformance/run.sh
```

CI runs both in the `conformance` job (`.github/workflows/ci.yml`),
which gates every PR — most importantly those touching
`crates/agenttrace-core/src/parser.rs` or `pricing.rs`.

## Refresh rules

- **Snapshot refresh** (rm-006 cadence): if a used model's rates move,
  the re-derivation guard fails the affected cases — recompute the new
  expected totals, update the literal and the recorded derivation date.
- **rm-663 lands** (per-event model stamping): case `012`'s xfail turns
  into an unexpected match and the suite fails with a refresh
  instruction — drop the case's `xfail` block (its `expect.cost_estimated`
  already carries the per-block truth `0.0495`).
- **Adding a case**: drop a fixture in `cases/`, append a manifest entry
  (unique id, class, verifiable totals — derive cost from the bundled
  snapshot, not from a rival price source), and extend the table above.

## Provenance

- `rm046-*.jsonl` — copied verbatim from
  `testdata/generated/adversarial/` (the rm-046 adversarial corpus); the
  harness asserts byte-identity.
- `claude-multi-model-frozen.jsonl`, `claude-opus-control.jsonl` —
  imported from run 6d254aad's assess corpus (`cc-multi` / `cc-opus`,
  the frozen-first-model PoC pair).
- everything else — pack-owned fixtures in the shapes already pinned by
  `crates/agenttrace-core/tests/usage_accounting_truthfulness.rs`
  (rm-601/602/603/554/556), crafted fresh so this pack asserts the
  end-to-end totals rather than re-using unit-harness fixtures.
