# Cycle 3 compound record — run fd0e91a2 cycle 3

Run fd0e91a2c53d4d22a96e30cb9b063b5d at wall 10c811b (worktree run-fd0e91a2c53d-fd0e91a2; canonical ecf84dc is an
ancestor — the wall carries master forward). Compound consumes pre-review cycle evidence only.

## Pipeline
assess 2bdc99b3 → research c8dd9dba → roadmap 9463c444 → prioritize fdf08332 →
stewardship adf32231 → implement 714aad25 (prior implement attempt b8d57a14 and roadmap
attempt 4075ad11 were provider-reaped with zero durable artifacts; both redone) →
targeted ea4ddfeea0354 → full af2a5776f3094 → compound 4f0cfc37.

## Batch (rm-1570 + rm-1571 + rm-1574) — implemented, validated pre-review
- rm-1570 daybreak pricing: gpt-daybreak-red/blue-latest hand-pinned (LiteLLM 2026-10-17:
  red 1.25e-5/7.5e-5/cw 1.5625e-5/cr 1.25e-6; blue 4e-6/2e-5/5e-6/4e-7); drop-on-upstream-ship.
  Mispricing proof pre-fix: $18.00/M-pair vs true $87.50 (4.86x), cost_basis 'priced', unknown=0.
- rm-1571 org cap: capture cap on org/org_email — sanitize, then the rm-594 shape past
  96 chars (48-visible-prefix + U+2026 + '#' + 12-hex FNV-1a), org_identity_truncated
  counter. NO session-cache rung bump: materialization (into_session) applies the same
  idempotent cap, so warm rows match-or-shrink and SESSION_CACHE_SCHEMA_VERSION stays 68.
  [Review fix 9ce27e5c (F2) rewrote this bullet — it originally claimed a 68→69
  bump and 32-prefix/8-hex that never executed.] Pre-fix PoC: 5 MiB orgIdentifier →
  5,250,754-byte JSON overview, 5,247,893-byte cache write.
- rm-1574 pricing-basis field: distinct per-row pricing basis (exact/fallback/unknown);
  loop-cost 'cost_basis' (waste.rs:256, diagnostics.rs:71) intentionally untouched. Review
  fix 9ce27e5c (F4/F1): by-model JSON key named pricing_basis (not cost_basis), and the
  --diagnostics --inspect JSON session row gained the always-present pricing_basis sibling.

## Validation record (pre-review)
- targeted: cargo test --locked -p agenttrace-core -p agenttrace — 74 suites / 1191 / 0
  (claude_org_identity_contract 6/0; daybreak 1/0; org_identity 3/0; pricing_basis 1/0);
  clippy focused -D warnings 0; fmt clean.
- full (ci.yml:134 verbatim): cargo test --locked -p agenttrace-core -p agenttrace-tui
  -p agenttrace — 76 suites / 1255 / 0; wall baseline 1251 → +4 new, 0 regressions;
  clippy --workspace --all-targets -D warnings 0; fmt clean.
- digest validation:v1:13dc7738045184964544a0b79596f137612955f36d767ebc6ef0a61c1c4ce217 — batch digest-invisible (crates/** + svg + CHANGELOG only);
  re-derived emission-time, byte-equal at targeted and full.

## Prevention rules (reusable)
1. New journal-derived string field ⇒ cap + contract test in the same delta (rm-594/rm-778; rm-1571).
2. Responses-mode SKU class (rm-610 → rm-1570): OpenAI default-SKU changes silently take the
   built-in default price — pin first, contract-test exact rates, drop-on-upstream-ship.
3. Empty-stamp validation blocks (KTD10) ⇒ derive the command verbatim from ci.yml:134
   (3-package form) and run hermetically; never substitute a local script.
4. Spool scratch is ephemeral: mirror to two extra locations immediately; keep replayable
   builders; prefer typed-JSON pins for recovery. Third swept-artifact recovery this fleet-week.

## Next-cycle context
- rm-176 cure STILL UNLANDED on this wall (snapshot-drops 'none', SNAPSHOT_DATE 2026-10-08,
  bundle 3,130); drift rc1 numbers in the rm-176 rider; cron red predicted Monday 2026-10-19
  06:00 UTC. One-commit rule for the cure stands (drops+refresh+date+goldens+workflow-widening).
- Deferred: rm-1572 reconcile warm-path (own the rung bump as a headline cycle), rm-1573 fork
  subagents (fixture-first: capture a real 2.1.302 on/off pair before semantics), rm-1575 TUI
  org column, rm-1576 opencode V2 probe.
- Watch: claude-code 2.1.302 cloud-session cost-restart carriers; OTel attribute-count-limit
  eviction; opencode V2 dev-tag motion; upstream versions as of 2026-10-17: codex 0.163.0,
  claude-code 2.1.302, opencode latest 1.19.3, ccusage 20.0.34, gemini-cli 0.64.0.

## Review fix (2026-10-18, independent_review:fix attempt 9ce27e5c)
Independent review (ffc108d5) returned NEEDS_CHANGES on the code delta and the compound claims;
every finding was fixed in-tree or in this record set:
- F1 (HIGH) inspect basis: --diagnostics/--inspect JSON session row now emits an always-present
  pricing_basis sibling key (exact/fallback/unknown from metrics.model_used, computed at render) —
  reports::session_pricing_basis, exported from lib.rs, asserted in the main.rs diagnostics test.
- F2 (HIGH) fabricated claims: the rm-1571 claims above (rung 68->69 'with sweep', '512B-class,
  32-prefix/8-hex') never executed — corrected here to the executed design (no bump; 96/48/12).
- F3 (MEDIUM) money narrative: CHANGELOG.md and pricing.rs told the defect backwards ($3.70
  overcharge); the truth is $18.00 billed on fallback vs $87.50 at published red rates — a 4.86x
  UNDERstatement. All three sites corrected.
- F4 (MEDIUM) key collision: by-model JSON key renamed cost_basis -> pricing_basis pre-pin
  (loop-cost vocabulary keeps cost_basis); contract test updated and now also asserts the loop-cost
  key never appears on a group row.
- F5 (LOW) acceptance arm: 1M+1M token bills pinned ($87.50 red / $24.00 blue / $18.00 fallback).
- F6 (LOW) deviation recorded: truncation discloses boolean counter + capped form, not the original
  length (length itself is hostile-controlled signal).
- F7 (LOW) spoofing arm: contract test asserts ESC/bidi/BOM/joiner/newline chars are U+FFFD-replaced
  before the cap (U+2028/ZWSP survive by lane design and are not asserted).
Retest at review-fix scope: claude_org_identity_contract 7/0 (spoofing arm added), daybreak lib test
with the three dollar pins, reports pricing_basis lane, CLI diagnostics test, clippy -D 0, fmt clean,
changelog-structure and output-contract gates rc0; validation digest unchanged (13dc7738...) — the
batch stays digest-invisible (crates/** + CHANGELOG only).

## Forensics: the swept band
The original cycle-3 band (patch md5 8c94eb0e9258091aafac7a04737b7bc7, postimage md5
a4f12f62289478fc9ed344310fb846a6, +46/-0, 5 hunks) was reaper-swept with every mirror
(delegate scratch, ~/.cache, /tmp twins) before compound. Events log held heartbeats only.
Rebuilt content-faithful from delegate/9463c444…json + fdf08332…json typed records; numerals,
scores, titles, acceptance criteria and rider content identical to the typed record; prose
reconstructed. New pins are recorded in the compound result JSON. Nothing landed anywhere
before this rebuild — no wall ever contained the swept bytes.
