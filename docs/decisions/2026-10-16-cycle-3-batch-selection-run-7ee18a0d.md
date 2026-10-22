# Cycle 3 batch selection — 2026-10-16 (run 7ee18a0d)

Decision record for run 7ee18a0dbb6c4dae9ec02a53c373e7ee (repository-maintenance
d50a9f62a6174a9dbbf73e310c799c67, cycle 3), phases prioritize d8c9212f +
stewardship e52a3094/e5f05695, compounded here from the selection memo
(md5 167fc3920632af972f54a0a4ea4b594b; the spool scratch was reaper-swept,
quarantine copy at `quarantine/scratches/20261014T173348Z/d8c9212f…/`).
Base c0da781 = origin/master~1 (porcelain 0); effective wall 3e90993
(ROADMAP.md md5 1d59adda, 392 def rows) plus the two unlanded bands known at
selection time (61c524f0 rm-1425; 5d5d680d rm-1426 — this run's own mint).

## Position

**Selected batch: "locally-defined catalog pricing truth" — LEAD rm-1426
(this run's own mint from roadmap 5d5d680d), with the rm-1052
composition/cross-link arm folded in.**

rm-1052 is already `implemented`; the arm is additive composition through the
same disclosure-counter family (distinguishing recorded-zero from
locally-defined-unpriced), not re-implementation.

### Why rm-1426 won

- Highest-value GENUINELY-UNIMPLEMENTED row on the effective wall. The
  86–96 priority band (rm-381/485/382/421/400/251/230/195/173/383/939/617/551/497)
  is code-complete at this base per the standing fleet adjudication — the
  status board lags code; selecting on status alone mis-selects.
- Fresh mint from this run's own assess→research→roadmap chain, title-checked
  90 minutes earlier; zero competing title claims (grep across all sibling
  selection-memo/stewardship artifacts: no hits for rm-1426).
- Evidence is host-verified, not hypothetical: this host's codex actually runs
  `model=glm-5.2` / `model_provider=cliproxyapi` against a private proxy with
  a 37,781B `model_catalog_json` whose schema carries zero pricing fields —
  locally-defined slugs are unpriceable BY CONSTRUCTION.
- Precise acceptance with red-first fixtures, a named-counter pattern
  (`kimi_usage_alias` precedent), and an in-tree fix-shape precedent
  (`apply_codex_sku_aliases`'s explicit or_insert override).

### Why the alternatives lost

| Alternative | Disposition |
| --- | --- |
| rm-381/485/382/421/… (86–96 band) | fleet-contended / code-complete; board lags code |
| rm-421 CompactionEpisode | real gap, but parser+attribution multi-cycle scale; upstream wave still moving |
| rm-176/rm-845 snapshot refresh | ingestion machinery is multi-cycle; same file as two live drifts |
| rm-195 fork identity | decision-gated on upstream channel events; no forcing event this cycle |
| rm-1373 OpenClaw arm, rm-241 gate-truth | owned by f084f50e/3124ddcb (live implement 20 min before census) |
| rm-1053 exact-key pricing | bbf61fbd drift owns the title (title-distinct from rm-1426) |
| CC multi-org, doctor-formula, CI lanes | owned by e63efa6f / a64735b7+966bd5f9 / 3d480e26 |

## Sub-units (as implemented, implement aa1aa3d9)

- **CU-A fixtures (red-first):** bare `glm-5.2` locally-defined session,
  wrong-carrier prefixed-slug arm, no-private-model zero-delta corpus.
- **CU-B disclosure machinery:** `pricing::is_locally_defined` marking,
  `locally_defined_model_pricing:{slug}` + `locally_defined_priced_cents`
  counters, guard-family agreement (insights / governance / totals).
- **CU-C operator binding:** `AGENTTRACE_PRICING_FILE` slug→rate with override
  provenance; rm-1052 cross-link; CHANGELOG entry.

## Outcome (pre-review, at compound 63b99e35)

Implemented uncommitted in worktree run-7ee18a0dbb6c-7ee18a0d; 5 of 10
surfaces carried into HEAD 1d51188 by conductor-salvage commits
c45d5e1/1d51188, 5 as porcelain. Validated: targeted 430/0 across 11 targets
+ fmt/clippy/changelog-structure rc0 (c0383413); full ci.yml:134 lane
verbatim rc0, 66 suites 1119/0/0 (106a5c83). ROADMAP flip rm-1426
candidate→implemented delivered spool-side at the compound seam
(band 25d35745 + compound patch → postimage 96731cec…; done-flip reserved to
the commit gate, rm-012 convention).

## Verification constraints recorded for future cycles

1. TMPDIR must be pinned outside any git repo and PROVEN so
   (`git -C "$TMPDIR" rev-parse --show-toplevel` must fail) — see
   `docs/solutions/workflow-issues/prove-tmpdir-is-outside-any-git-repo-before-trusting-a-battery.md`.
2. Private `CARGO_TARGET_DIR`; touch-all-src before trusting a full battery
   (stale-binary trap).
3. Reap forensics before any redo; quarantine + md5 round-trip for swept
   artifacts — see
   `docs/solutions/process-issues/dead-attempt-forensics-and-quarantine-recovery.md`.
4. Census HEAD vs porcelain against the implement record: salvage commits can
   land mid-cycle carrying a subset of the batch.
