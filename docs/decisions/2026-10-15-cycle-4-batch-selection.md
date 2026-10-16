# Cycle 4 batch selection — 2026-10-15

Decision record for run 0915f736aa1c488ebd3b085b73a6d7df (repository-maintenance
e74cb714 cycle 4), phase prioritize (attempt 950bd0bb). Method: in-thread
`ce-plan`-style ranking (impact, risk, effort, dependencies) with an added
**claim-state** column — every candidate was title-swept against the resting
wall AND the unlanded fleet band census (delegate scratch postimages/patches)
before selection. The compound-engineering skills load as methodology, not as
sub-agent loops, in this environment (established cycle 3; disclosed per the
ideation-doc precedent).

Inputs: this run's own assess (adversarial envelope at HEAD 8bc0587 — 17
gates, 16 rc0, two standing reds) and research (ea2041bc dossier: three-source
chain for the effort-suffix mis-price; ecosystem census), both redone from
scratch because prior attempts died provider-side with no typed artifacts.

## Position

**Selected batch: rm-1511 "pricing truth for reasoning-effort-suffixed model
ids"** — the single highest impact-per-effort item that was *unclaimed*.

The claim-state gate was the decisive column, not the impact ranking:

- Every higher-visibility red was sibling-owned in unlanded fleet bands:
  rustls 0.23.46 (kills the 4-advisory deny lane), the install-runtime
  derive-pin fix, the pricing snapshot refresh (LiteLLM 4,116 vs bundled
  3,131), zh-CN parity. Re-implementing any of them double-lands.
- rm-1511 was this run's own fresh mint (fog grep clean), with a recorded
  PoC as ready-made acceptance (0.0105@200k-ctx vs 0.007@1M — a 1.5x output
  over-bill rendered `priced`), and **zero dependencies**: the base keys were
  already in the bundled snapshot, so no regeneration, no svg-golden churn,
  no session-cache catalog-stamp churn. Independence from the snapshot
  refresh band was itself a selection criterion — disjoint surfaces make the
  eventual compound trivial.

## Outcome (pre-review, compound-time record)

Implemented red-first (implement 6d028035): effort arm in `match_variants`
feeding `lookup_price_in`'s pre-existing catalog-first variant resolution
(`matching_catalog_key` — unchanged by the batch; `has_specific_price`
shares the same variant basis, so no divergence existed — corrected by
review fix 1a615b64), `effort_tier_of` re-derivation
for tier-aware lanes, named fallback disclosure
(`data_health.fallback_with_base_available`, bounded to 5, JSON lane),
9-test contract suite (12 after review fix 1a615b64 — antigravity-lane
end-to-end fixture, second-order variant pin, `effort_tier_of`
SKU-suffix limitation pin, unique fixture dirs), CHANGELOG entry. Validated (targeted 7c6d6aa9 battery rc_total=0;
full 412a12c1/caac3078: 64 suites 1138/0, fmt+clippy+12 gates+tui smoke
green; standing reds install-runtime test D and deny x4 unchanged and
owned elsewhere). Digest stable across the run:
`validation:v1:c0abde73…` (re-derived, byte-identical).

Prevention rule minted at `docs/solutions/reliability/
pricing-variants-must-resolve-against-the-catalog-before-builtin-defaults.md`:
a resolver consulting a fallback table through derivable variants must offer
those variants to the authoritative table first, and an available exact base
behind an unknown suffix is a disclosure obligation.

Next-cycle candidates left on the table (with evidence already banked):
upstream ports #314/#317 (now MERGED upstream — fork gap two PRs wide,
rm-448 rider), cline/kiro accounting-parity signals (tokscale v4.19.0
#1399/#1404, rm-203 rider), and the ceded lanes above whose owning bands may
have landed by then — re-census before claiming.

## Review fix (2026-10-16, attempt 1a615b64)

Independent review ed20477f returned NEEDS_CHANGES on the shipped record (the
pricing change itself re-verified correct); this fix landed after it:

- Mechanism narrative corrected across CHANGELOG, both docs, the pricing.rs
  comment, and the compound banner (regenerated spool patch): the catalog-first
  variant arm pre-existed; the defect was the missing effort arm, and
  `has_specific_price` shared the candidate list — the originally claimed
  divergence never existed.
- Acceptance arm (2) scoped honestly to the `--overview -f json`
  `data_health` lane (the originally quoted `priced at default; exact base
  available` annotation rendered nowhere; text/markdown carry the counter).
- Acceptance arm (3) delivered: `antigravity_lane_prices_effort_alias_at_base`
  (rm-947 lane shape) exercises the alias end-to-end through the real parse
  path; second-order variant loop pinned; `effort_tier_of` SKU-suffix
  limitation documented and pinned; fixture dirs made unique per invocation.
