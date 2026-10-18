# Reaped-attempt plans and sibling dossiers need first-hand anchor and number verification

## Symptom

A dead provider attempt (session_reaped) or a concurrent sibling lane can leave artifacts
that look authoritative — a complete, never-executed build script in its scratch dir, or a
research dossier whose numbers were never re-derived on this base. Executing or citing them
verbatim produces patches that abort mid-apply, mint colliding roadmap numerals, or carry
confabulated facts into walls and changelogs.

## Rule

1. A never-run plan is a hypothesis stack, not a result. Before executing an adopted plan
   script: re-verify EVERY anchor (file:line, def-row existence, md5 pins) against the live
   tree, and re-derive the plan's own targets (rider rows must exist; mint bands must sit
   above the live claim frontier; ceded subjects must stay ceded).
2. Sibling dossiers/postimages are consumed with provenance labels AND first-hand
   re-derivation of every load-bearing number and every anchor they steer (dates, key
   counts, line citations, symbol existence). Two-of-many disproven is the norm, not the
   exception — assume at least one stale claim per foreign artifact.
3. Adoption verification covers PROSE, not just code: mechanism attributions, numeric red
   values, and code-topology citations in any adopted appendix/banner/patch must match
   what the tree's own tooling emits.

## Evidence (run 6b369f2f8a4e cycle 1, agenttrace @ dc65644, 2026-10-09)

- Reaped roadmap attempt 39be0a16 left a complete build-roadmap-delta.py that never ran.
  Adopting its skeleton surfaced three defects BEFORE execution: (a) its rm-006 rider
  targeted a def row that does not exist on this wall (every rm-006 occurrence was banner
  prose — the build would have aborted on its first rider); (b) its kimi mint and rm-691
  rider duplicated 2e7758d3's same-base unlanded band (rm-847) — never re-mint, reconcile
  by title; (c) its pricing riders duplicated fbc4581f's unlanded rm-006/164/176 band.
  Failed sibling roadmap attempts e740de95/a379dfb6 also left scratch (a379dfb6's script
  minted superseded numerals rm-839..842).
- Sibling research dossier 068a3070: 'pricing lineage divergence 2026-09-13 / 2,756 keys'
  was FALSE (pricing.rs:17 = 2026-10-04, bundle 3,100 keys — verified in both worktrees);
  'bare glm-5.3 absent live' was reframed (prefixed keys exist; only the bare key is
  absent, which is the actual defect).
- Citation drift: the roadmap banner's "F-numbering corrected" note renumbered assess
  findings away from findings.md's actual headings; the minted id lines inherited it
  (rm-851 'F2'→F3, rm-852 'F1'→research §1, rm-902 'F4'→F1). Caught only at compound by
  re-reading the authoritative findings file — fixed with dated addenda.

## Prevention checklist

- [ ] For each anchor in the adopted plan: grep/read it on the live tree, count == expected.
- [ ] For each mint numeral: fresh live claim census (worktrees' walls + spool patches +
      postimages) before assigning.
- [ ] For each load-bearing number in a foreign artifact: re-derive with the repo's own
      data before it steers a mint, a changelog bullet, or a wall row.
- [ ] After adoption: diff the plan's PROSE claims (attribution, values, line refs) against
      what the tree actually emits; correct with dated annotations, never silent edits.
