# Cycle 3 compound record — run bbf61fbdc172486790905665257fc591

repository-maintenance 61d344c3cfec4e79925cf71665493a30 · cycle 3 · compound attempt b4fa7fcc · 2026-10-15
Base: 3e90993 (== origin/master at assess/implement time; live master advanced to 16dcc9a while this
band built — 395 rows, def-max rm-1502; all rider/flip rows verified present on BOTH walls, hunks rebase cleanly).

## Batch: 'pricing-truth + gate-truth' (selected at prioritize f7061d38; stewardship 9b24c5b7)

| Subject | Status | Outcome |
|---|---|---|
| rm-1053 exact-key catalog resolution | done-flip pend commit | EXECUTED (attempt 9e257e97, pricing.rs dual-key catalog + raw-first lookup, ~+470 lines incl. 6 pins) + VALIDATED (targeted 405/0, full 1124/0/0, clippy -D rc0, 8 gates rc0, conformance 13/1xfail, behavioral PoC 3.6 vs pre-fix 3.0) |
| rm-1390 vacuous-Unreleased gate hole | minted-implemented (done) | A/B proof: BASE rc0 'sees all 0 entries' → NEW rc1; live gate rc0 on 302 entries |
| rm-726 overflow residual (verify-arm) | rider on done row | Pin `implausible_magnitudes_bill_as_parsed_with_disclosure`: disclosure fires, billing-as-parsed pinned; clamp-still-doesn't-gate-cost residual recorded, unaccepted |

## Retraction (the cycle's process lesson)
rm-1389 ('pricing match provenance fields', minted at 58.0 in the roadmap phase) is a TITLE TWIN of
pre-existing rm-175 (same subject at 82.0). Root cause: head-truncated grep title-check. Deltas folded
into rm-175 with sequencing (implement after rm-1053 — match_status is computed at the exact-key seam).
rm-1389 never landed; numeral stays free. House rule now pinned: title-checks are full-table parses.

## Band provenance
The cycle-3 roadmap phase's envelope AND scratch were reaped before landing (<6h sweep). This compound
band REBUILDS it verbatim from the phase summaries (the typed work-order context carries the full
PhaseResult texts) plus the outcome layer. Disclosed in the band banner. Attribution correction applied
during rebuild: the overflow rider was mis-fanned to rm-600 (workbuddy-arithmetic row) in the unlanded
band; it rides rm-726 here.

## Wall deltas in this band
- Banner [a]-[h] (rebuilt band, batch outcomes, retraction, riders, mints, cessions, negative pins, commit-gate duties)
- 10 dated riders: rm-012, rm-1053, rm-726 (attribution fix), rm-501 (E0433 5→14), rm-930, rm-947, rm-758, rm-776, rm-039, rm-175 (retraction fold)
- Flip: rm-1053 candidate → done (pend commit)
- Mints: rm-1390 (done), rm-1391, rm-1392, rm-1512 (fleet frontier was rm-1511; package-name trap from full_tests)
- Postimage: 396 rows, 0 duplicate ids, def-max rm-1512, managed footer last

## Cessions and negative pins
- Ceded: zh-CN parity rm-1062; serde_json bump rm-1023; install surface fog rm-989/1002/1016 + upstream #313-#318 wave (unmerged, 6 behind); qwen rm-655/656; mcp memoize re-adjudication after sibling 9f23671d lands.
- Negative pins (closed/absent): plugin-version gate rc0 (standing red CLOSED); formula lanes re-proven guarded (19 hostile lanes, zero unmarked introducers); claude-code 2.1.299 no journal-shape change; models.dev schema stable.
- Standing reds re-proven NOT regressions: check-install-runtime test D/E (rm-051 literals vs install.sh REF); cargo-deny advisories rustls 0.23.45 (lock untouched by batch; sibling owns bump).

## Context for the next cycle
1. rm-175 is the sequenced follow-on to rm-1053 (provenance fields at the exact-key seam) — upstream #103's promoted spec.
2. Upstream wave #313-#318 unmerged: #315 deletes the install.cmd TOCTOU surface; port wave closes the install findings family.
3. Antigravity: official migration timeline arrived (rm-947/rm-758) — gemini discovery-root guard is cheap and ripe.
4. Watch: sibling 9f23671d (mcp.rs) and the tier-rates twin b5d391ec (pricing.rs, different seam) — rebase hazards for anything touching those files.
5. E0433 class actively growing (rm-501): 14 ungated unix-only test fns; windows-latest lane stays compile-red until gated.

## Validation consumed (no execution this phase)
Targeted (4ffa8b55): bash -n + gate rc0 + fmt/clippy rc0 + core-lib 405/0 + 7 pins 7/0, digest
validation:v1:618a8229… stable before/after. Full (36ee5dd4): 1124/0/0 across 61 suites, clippy ws
-D rc0, fmt rc0, deny advisories-only red (rustls, standing), release build + behavioral PoC 3.6,
8 binary gates rc0, conformance 13/1xfail, install-runtime rc1 standing (test D, rm-051).
Logs: delegate/{4ffa8b55…,36ee5dd4…}-scratch/ + janitor-proof mirrors ~/.cache/at-tt-4ffa8b5/, ~/.cache/at-full-36ee5dd4/.
