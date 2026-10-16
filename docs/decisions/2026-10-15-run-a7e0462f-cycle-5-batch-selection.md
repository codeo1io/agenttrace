# Cycle 5 batch selection & outcome — 2026-10-15

Decision record for run a7e0462f7e594d41a8968ac540c3fb4f, phases prioritize →
implement → validation → compound (this doc), repository-maintenance cycle 5 at
base 3e90993dc94350498be08c51eb62fa0da3831f95. Method: in-thread
`ce-prioritize`-style cross-weighting of the composed roadmap band
(a0310bcf postimage, 394 def rows) against this cycle's assess eda5ed0f (5 fresh
+ 6 standing findings, live-probed at HEAD) and research aaa3f632 (9
evidence-backed candidates; upstream frozen at master 15ed07f2 but issues #103
provenance demand + #236/#237 radars are live user-need signals).

## Position

**Selected batch: "rate-row truth + walk hygiene + last green gate"** — fix the
two correctness defects the assessment probed end-to-end, and clear the only
standing gate red so the cycle ships with a fully green battery.

1. **rm-1053 — regional rate collapse + provider attribution divergence**
   (correctness 60.0, LEAD). Assessor pinned the full mechanism:
   `normalize_model` strip_date/strip_version non-idempotence
   (pricing.rs:1340-1410 at 3e90993: normalize_model, strip_date_suffix,
   strip_version_suffix) plus `convert_litellm` first-sorted-wins key
   collapse; the F-A2 probe showed every regional spelling resolving to the
   $3.30/M fold-family winner (au.anthropic, the dedupe winner of the
   `claude-sonnet-4-5-20250929-v1` fold family) — the right dollars for the
   $3.30/M cross-region rows but uniform au.anthropic attribution, +10%
   overbill on the $3.00/M rows, −8.3% underbill on GovCloud — while the
   bundled snapshot carried the full $3.00/$3.30/$3.60 lattice (economics
   corrected at review fix 6f1290d735734e1b979ff2788a977f85; the pre-fix
   draft recorded the winner as $3.00/M). Research corroborated the lattice (19/20 live keys bundled) and the
   demand (upstream issue #103, open since 05-04). Outcome: VALIDATED —
   spell-exact resolution (`resolve_spell_exact`) now precedes every fold;
   provider/source derive from the same resolved row; see
   `docs/solutions/reliability/resolve-exact-rate-rows-before-any-normalization-fold.md`
   (the cycle's durable learning) and the ROADMAP compound rider.
2. **rm-1435 — `-d` walks ingest agenttrace's own at-rest artifacts**
   (correctness 66.0, fresh mint from assess F-A3's walk-hygiene arm). One own
   cache artifact (e.g. `opencode-sqlite.json`) failed `--overview -d $HOME`
   wholesale; the `is_session_file_name` blocklist enumerated reader-side
   guesses instead of the writers' actual outputs. Outcome: VALIDATED — the
   enumeration now lives beside the writers as
   `session_cache::is_agenttrace_artifact_file_name` (session_cache.rs:890),
   consulted at discovery.rs:1093, with a red-first nine-artifact enumeration
   test.
3. **Rider: install-runtime standing red (the "last green gate")** —
   check-install-runtime tests D/E asserted hardcoded `v0.9.0` literals while
   install.sh ships `REF=v0.10.1`, stranding the gate red for two cycles. Outcome:
   CLEARED — the gate now derives the pinned ref from install.sh's own REF line
   (expectation from the shipped artifact, not literals); rc0 at implement,
   targeted, and full batteries. Prevention rule recorded in the compound
   banner (L2).

## Outcome (pre-review evidence, consumed — not re-run at compound)

- Targeted (e3ab7160): core --lib 404/0/0, discovery_contract 105/0/0, pricing
  filter 51/0/0, fmt/clippy rc0, install-runtime rc0, plugin-version rc0;
  validation digest `validation:v1:715b72ac…` re-derived post-battery.
- Full (c1c24b30, after 5 provider-dead full attempts redone from scratch):
  trio rc0 **1124/0/0** across 121 result lines, clippy `-D` rc0 54s, fmt rc0,
  token_conformance 2/0, conformance 13 cases (1 xfail-pinned), MSRV 1.89 rc0,
  release rc0 (14,191,144-byte bin), 5 sandboxed output-contract gates rc0,
  install-runtime rc0 under `AGENTTRACE_BIN=release`, plugin-version GREEN.
- Tree discipline: 6 M / 0 untracked; the diff md5 drifted
  583866…(844 lines) → 050da582…(841 lines) from a docs-only CHANGELOG edit
  (review-fix lineage, mtime 2026-10-15 02:20:50) — executable surfaces
  byte-stable, digest unaffected by design.
- Process repair: implement's fold was rejected once for a missing
  changed-surfaces attestation (KTD13); repaired by 168b98ca after proving the
  predecessor tree byte-intact. Lesson doc already in tree:
  `docs/solutions/workflow-issues/implement-fold-needs-changed-surfaces-attestation.md`.

## Standing reds at full battery

- `actionlint SC2001` `.github/workflows/ci.yml:192` — cron deadline
  2026-10-20 06:00 UTC.
- `cargo-deny` 4× RUSTSEC, all rustls 0.23.45 — **cure intel**: rustls
  0.23.46 (2026-10-14) fixes all four; a lock-only bump clears them (upstream
  PR #22 owns distie). Cheap next-cycle item.

## Lessons (durable)

- **L1** Resolve spell-exact rows before any normalization fold (the LEAD
  item's mechanism — solutions doc above).
- **L2** Derive CI-gate expectations from the shipped artifact, never literals
  (the install-gate red existed only because the gate hardcoded a version the
  release had already moved past).
- **L3** Enumerate at-rest artifact filenames beside their writers, not beside
  the readers (rm-1435).
- **L4** Docs-only review-fix edits shift the batch diff md5 without touching
  executable surfaces — declare file-mtime lineage at validation dispatch so
  byte-drift is attributable (the validation-digest design made this provable
  rather than ambiguous).

## Next-cycle candidates (ranked, from this cycle's evidence)

1. **rm-1436 VS Code Copilot Chat store lane** (research §C5: codeburn bills
   it at 11k★ scale; our discovery census shows zero VS Code lanes).
2. **rm-967 session-cache schema ping-pong** (assess F-A1: 2735 banked entries
   dropped across 4 invalidation bumps in one fleet `sessions.json`; 19
   distinct schema versions across 100 worktrees).
3. **rustls lock-only bump** (clears all 4 cargo-deny reds; one Cargo.lock
   change).
4. **Assess F-A3 stale-session-name pickup** (JSONL-timestamp grouping admits a
   stale `sessions.json` name — unowned; needs a mint).
5. **rm-1054 dir-scoped cache banking** (dir-scoped report runs bank foreign
   sessions into the operator-global cache).
