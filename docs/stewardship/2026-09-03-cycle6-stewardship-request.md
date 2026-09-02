# Cycle-6 stewardship request (phase: stewardship)

Run `a24bcf084cf049208c75d2cb4f3a3755`, attempt `374b5bc31d10409c8003445e1a88c5bd`,
2026-09-03. This document is the human-readable companion to the structured
`stewardship_request` in the PhaseResult; the JSON fields are authoritative.
Router note: no `ce-*` compound-engineering router installed (disclosed, as in
every phase of this run).

## Repository candidate

`/work/projects/agenttrace` — working fork checkout of `luoyuctl/agenttrace`
(remotes: `origin` = upstream, `fork` = `codeo1io/agenttrace`). HEAD
`696206f`; local `master` is 4 commits ahead of `origin/master`. No Git
topology is chosen here — branch/worktree planning belongs to Conductor.

## Change units (decisions with rationale)

Six units, CU-17..CU-22, selected by the prioritize phase
(`2026-09-03-cycle6-prioritization.md`). Surface cites re-verified live this
pass.

- **CU-17 Upstream-portable CI revert** (pass-9 CRITICAL; policy fix).
  Surfaces: `.github/workflows/ci.yml:21`, `.github/workflows/release.yml:13`,
  `:40`, `:117`, `.github/workflows/dependency-review.yml:16` (five
  `runs-on: self-hosted` hits confirmed by grep this pass); new
  `scripts/ci/check-no-self-hosted.sh` (guards upstream-bound branches; joins
  the eleven existing `scripts/ci/check-*.sh` guards); plus one **local,
  uncommitted** action — `git config branch.master.remote fork` (and merge ref)
  so a plain `git push` can never target upstream. Rationale: highest
  risk-reduction per line in the whole backlog (upstream PR #282 was closed
  for exactly this contamination); the revert is policy-required by
  AGENTS.md rule 3 and must never carry other concerns.
- **CU-18 Gemini CLI `~/.gemini/tmp` discovery root** (candidate 50a).
  Surfaces: `crates/agenttrace-core/src/discovery.rs:51`
  (`known_session_dirs()` — twelve `KnownSessionDir` entries today, gemini
  family present only as `…/antigravity-cli/brain`), predicates already in
  place at `discovery.rs:533` (`tmp` under `.gemini` → depth 4) and
  `:584` (`is_gemini_temp_session_file`); fixture work near existing
  `testdata/gemini-checkpoint.json` / `testdata/gemini-current-chat.json`;
  tests in `crates/agenttrace-core/tests/discovery_contract.rs`. Rationale:
  restores a README-claimed capability; format proven to parse in research
  pass 8; cheapest high-impact unit.
- **CU-19 Antigravity `…/antigravity-cli/conversations/` root** (candidate
  50b, conditioned). Surfaces: same `known_session_dirs()` block plus a
  file-name predicate near `discovery.rs:508` (`is_session_file_name`) or a
  sibling of `is_gemini_temp_session_file`; possible parser detector near
  `parser.rs:389` (`parse_antigravity_jsonl`) if the conversations shape
  needs one; new fixture from the officially documented shape. Rationale:
  completes radar #236, but it is the only unit whose input shape is
  documented-not-verified (this host has no `~/.gemini`), so it carries a
  conditioned acceptance and must be severable from CU-18.
- **CU-20 Gemini-family thinking tokens billed as output** (candidate 25).
  Surfaces: the three alias-driven usage extractors — `parser.rs:1802`
  (`qwen_usage`), `parser.rs:3550` (`gemini_usage`), `parser.rs:3885`
  (table-driven) — none reads `thoughtsTokenCount` (grep zero matches this
  pass); reporting seam for the reasoning share near the audit/cost paths in
  `reports.rs` (`cost_audit` wiring, `reports.rs:553-612`) and the audit
  render path; gemini-3.x fixtures; changelog note that baselines shift.
  Rationale: cost truthfulness for every thinking-model session; small,
  contained diff.
- **CU-21 `--sample` disclosure names the active ordering** (pass-9 residual
  of closed F8-1). Surfaces: `crates/agenttrace-cli/src/main.rs:250-252` and
  `:293-295` (both reason strings verified identical this pass: "sampled
  newest {N} of {M}") plus a CLI test. Rationale: two-line honesty fix;
  zero risk.
- **CU-22 Session-cache byte ceiling** (pass-9, F8-3 follow-on; declared
  stretch). Surfaces: `crates/agenttrace-core/src/session_cache.rs:24`
  (`MAX_SESSION_CACHE_ENTRIES`), `:607` (`enforce_entry_bound`),
  `:636-639` (`save_session_cache` pre-save bound); doctor disclosure via
  `render_doctor_report` (`crates/agenttrace-core/src/doctor.rs`, imported
  `main.rs:6`); eviction test beside `:395` (`prune_dead_entries` tests).
  Rationale: completes the cache-bounding story; explicitly droppable
  without touching the other five units.

## Must remain separate (hints to Conductor's split)

1. **CU-17's local git-config repoint vs any committed file change.** The
   tracking repoint is environment state (AGENTS.md rule 3's whole point);
   it must never appear in, or ride with, committed content.
2. **CU-17 (policy/CI revert) vs CU-18..CU-22 (code capability/hardening).**
   Different review audiences (operator policy vs upstream-portable code);
   independently revertable; conflating them recreates the exact
   contamination that killed PR #282.
3. **CU-18 vs CU-19.** Same function, different confidence: CU-19's
   conditioned acceptance (documented-shape fixture, possible provisional
   fallback) must be severable so it cannot drag CU-18's verified root.
4. **CU-22 (stretch) vs CU-17..CU-21.** May roll to cycle 7 intact; keep it
   a self-contained session_cache.rs unit.
5. **Docs (ROADMAP.md status updates, implementation record) vs code units.**
   The roadmap documents the closures and must land with or after the code
   it describes — but as its own reviewable unit, not interleaved into
   functional diffs.

## Dirty state to preserve (inventoried for Conductor)

`git status --short` (verified this pass): `ROADMAP.md` modified (roadmap
phase, +265/−27, uncommitted); untracked: `AGENTS.md`,
`docs/stewardship/2026-09-02-cycle-4-reconciliation.md`,
`docs/stewardship/2026-09-02-reconciliation-record.md`,
`docs/stewardship/2026-09-02-roadmap-cycle2-update.diff`,
`docs/stewardship/2026-09-02-roadmap-cycle3-update.diff`,
`docs/stewardship/2026-09-03-cycle5-adversarial-assessment.md`,
`docs/stewardship/2026-09-03-cycle5-reconciliation.md`,
`docs/stewardship/2026-09-03-cycle6-prioritization.md`,
`docs/stewardship/2026-09-03-research-extensions.md`,
`docs/stewardship/2026-09-03-roadmap-cycle5-update.diff`. All are campaign
records meant to survive branch/worktree operations.

## Out of scope this cycle (per prioritize phase)

Parse-file size cap, installer checksums, git-root sandbox fused with N10,
Hermes tool-failure extraction, candidate 51 fused with 24, strategic lanes
(candidates 3/19/2), P3-1 Windows parity, TUI reload race, and the
operator-gated cleaned upstream PR (C8) — CU-17 is its precondition.
