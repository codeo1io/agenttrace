# Cycle 7 batch selection — 2026-09-14

Decision record for run 125bf93302aa4e308cb0739b67f16f33, phase prioritize.
Method: in-thread `ce-prioritize`-style weighted scoring (impact, risk,
effort, dependencies, strategic value) over the open items in both ROADMAP.md
lanes at HEAD df3b621, consuming the pass-11 assessment and research-pass-9
artifacts without redoing them. The installed compound-engineering router is
not loadable as pi skills in this environment (established in the research
phase; disclosed per the ideation-doc precedent — `docs/ideation/…ideation.md`).

Inputs: `docs/reviews/2026-09-14-adversarial-repository-assessment-pass11.md`
(A11-1..A11-9, all reproduced live at df3b621), `docs/research/
2026-09-14-extensions-research-pass9.md` (candidates 53–59 + external
evidence), and the roadmap's refreshed next-cycle shortlist
(ROADMAP.md:1646–1659). Baseline health is green (assess phase: 213/213
tests, clippy/fmt clean, 10/10 CI check scripts).

## Position

**Selected batch: "trustworthy capture" — fix the ways agenttrace silently
drops or mis-signals data, and open the one capture channel that fills its
two documented blind spots.**

Core items (each closes a roadmap hardening-lane or capability-lane entry;
acceptance and evidence expectations live in ROADMAP.md and are not restated
here):

1. **Go-flag shim `--no-baseline-gate` misregistration** (pass-11 A11-5;
   ROADMAP.md:957). Cheapest user-visible correctness win: the documented CI
   recipe (`docs/guides/ci-integration.md:124-127`) is broken by the shim
   today with a misleading error. Effort S.
2. **Symlinked session-root discovery** (ROADMAP.md:1018; Codex `#42135`).
   Live silent data loss: Codex 0.153.0+ officially supports symlinked
   session roots and agenttrace skips them whole (`discovery.rs:379-391`,
   `file_type().is_dir()` is false for symlinks). Effort S–M with the
   loop-guard bound the roadmap demands.
3. **Statusline capture mode — `agenttrace statusline`** (candidate 53;
   ROADMAP.md:1526). The strategic headline: Claude Code's statusline JSON
   (v2.1.251+) carries `rate_limits.five_hour/seven_day.used_percentage` +
   `resets_at` and the authoritative `prompt_cache` block — exactly the two
   inputs the roadmap repeatedly records as unseen (candidates 3, 9, 10, 17).
   Never-fail-the-host subcommand, bounded local JSONL tee, session-keyed
   ingestion with overlap dedup, limit-pressure and miss-cause surfaces in
   reports/TUI. Effort M–L.
4. **Cache-bound accounting over the deduplicated key union** (A11-2;
   ROADMAP.md:977). Eviction correctness in both directions (byte bound
   double-counts; entry bound under-drops). Latent today (15.5 MB vs 64 MiB)
   but the same file 53's captures will rotate through. Effort S–M.
5. **Installer mode and checksum parity** (A11-3; ROADMAP.md:994). chmod
   0755 (0711 today, verified live) plus published SHA-256 verification the
   way npm's installer already does, with checksum emission added to the
   release workflow. Effort S–M.
6. **TUI delivery-worker error surfacing** (A11-4; ROADMAP.md:1006).
   `Disconnected` currently renders as "no evidence". Small, and it makes
   item 4's subsystem observable. Effort S.

Ride-alongs (droppable without breaking cycle coherence, in drop order):

- **R1 — Pricing snapshot refresh + age disclosure, local half** (candidate
  51; ROADMAP.md:1510): regenerate the snapshot from the live catalog
  (3,518 → 3,923 keys since 2026-09-02) and surface snapshot age in
  `--doctor`/report provenance. The workflow-plus-PR half stays deferred
  (PRs are outside this cycle's gate).
- **R2 — CI workflow hygiene**: fork-tolerant dependency review
  (`.github/workflows/dependency-review.yml`, pinned action
  `a1d282b36b6f3519aa1f3fc636f609c47dddb294`; unblocks clean merge signals
  for every future PR) and removal or activation of the never-true
  `AGENTTRACE_TUI_REAL_DIR` step (ci.yml:84, pass-11 A11-8). YAML-only,
  no CI execution required to land.

## Why this batch

- **Impact.** Items 1 and 2 ship wrong or lossy behavior to real users
  today, both reproduced live in the assess phase (misleading
  `--baseline requires --overview -f json` exit; symlinked roots skipped).
  Item 3 converts agenttrace's two most-repeated "cannot see this" roadmap
  admissions into a data source no competitor-inventoried pass has yet
  indexed (pass 9 §1.6: the payload is 11 days old and already has three
  dedicated tools).
- **Risk.** Low-to-medium and contained: 1 is a one-line registry fix in a
  class already fixed once (`--sample`, cycle 5) with a standing test
  pattern; 2 is confined to `known_session_dirs()` walk with a bounded
  loop guard; 4 is two functions in `session_cache.rs`; 5 is installer
  shell plus one workflow emission; 6 is one channel type. Item 3's only
  real risk is schema drift, mitigated by pinning fixtures to the
  documented schema (code.claude.com/docs/en/statusline, dateModified
  2026-09-09) and version-gating fields at v2.1.251+.
- **Effort.** Calibrated against cycle 6 (six CUs: CU-17..CU-22): four
  S/S–M hardening fixes plus one M–L feature ≈ cycle-6 load; R1/R2 are
  the explicit stretch valve.
- **Dependencies.** None among the six core items (parallelizable;
  different files: `main.rs`, `discovery.rs`, new `statusline` module,
  `session_cache.rs`, `install.sh`/release workflow, `app.rs`). Item 3 is
  itself the *prerequisite* for the highest-value deferred item: 56's
  tier selection needs per-request observed context sizes and cache
  write/miss volumes, which 53's `prompt_cache` captures provide.
- **Strategic value.** "Never silently lose or mis-signal data" plus
  "capture the rate-limit/cache telemetry competitors only display" is a
  single coherent story for the CHANGELOG, and it compounds: 53 feeds 56,
  2 feeds 54's fixture work, 6 feeds N10's timeout follow-on.

## Why not the alternatives

- **Candidate 56 (tier/service-tier `Price` v2, ROADMAP.md:1572):** the
  accuracy case is strong (operator corpus 94% cache-read; 101/81
  context-tiered base models; ~115 priority tiers), but it is the highest
  regression-risk item in the pool (cost totals must stay bit-identical
  for tier-free corpora) and it would *guess* per-request context tiers
  without 53's captures. Dependency-ordered behind 53; first candidate
  for cycle 8.
- **Candidate 54's usage-record half (#41912):** needs a real 0.153+
  rollout fixture and `codex exec` cross-accounting; its live-bug half
  (symlinks) is exactly what item 2 extracts. Deferred half stays with
  the roadmap entry.
- **Candidates 58 (Hermes plugin channel) and 37:** external-repo
  creation and channel decisions better made once 53 lands (the skill
  will wrap the statusline workflow too).
- **Candidates 55/57/59:** experimental (no semconv tags yet), spec-new
  (2026-07-28 MCP needs its own design), and fixture-blocked
  (Antigravity) respectively.
- **A11-6 platform parity / HOME fallback:** cannot be verified
  end-to-end here (no Windows host) — the cycle-1 bar still holds.
- **A11-7 git timeout (N10) and the untrusted git-root item:** natural
  cycle-8 pair with 6's error channel; adding them now stretches the
  cycle without raising its ceiling.
- **Legacy mid-lane items (`--limit` scoping P3-5, control-character
  filter P3-4-class, local-calendar windows P3-2, SQLite `since`
  push-down):** real but lower-severity polish; none regress data truth
  the way 1/2/4 do.
- **Do nothing:** rejected — 1 and 2 are reproducible defects on
  documented paths.

## Verified facts

- Project (this session or the pass-11 assess phase at df3b621):
  `flag_takes_value` at `main.rs:707` with the misregistered entry at
  `:726` (bool defined `main.rs:99-100`); `discovery.rs:379-391`
  `file_type().is_dir()`; `session_cache.rs:549-564` (`cached_entry`),
  `:617-648`, `:652-695`; `install.sh:53-54,66`; `app.rs:1513-1521`,
  `:1536-1553`; `.github/workflows/dependency-review.yml:1,23`;
  `ci.yml:84` guarded by `AGENTTRACE_TUI_REAL_DIR` which the workflow
  never sets; `pricing.rs:16` snapshot dated 2026-09-02; no statusline
  surface exists (grep-verified in research pass 9).
- External (research pass 9, fetched 2026-09-13/14): statusline schema
  and v2.1.251+ gating (code.claude.com/docs/en/statusline);
  Codex 0.153.0 release notes `#42039/#42135/#41912`; LiteLLM census
  3,923 keys; ecosystem statusline tools per §1.6.

## Assumptions (not verified)

- Effort sizes (S/M/L) are choke-point and precedent estimates, not
  measured implementation time.
- Ride-along R1 assumes internet access at implement time for the
  snapshot fetch (true in every prior cycle).
- R2's fork-tolerance approach (early clean exit vs `continue-on-error`)
  is left to the implementer within the roadmap item's operator-informed
  constraint.

## Conditions / definition of done

The cycle is complete when the roadmap acceptance criteria for all six
core items hold with their evidence expectations recorded (each closes as
a CU entry in Completed, per the standing convention), the behavior
changes are named in the CHANGELOG, `docs/guides/ci-integration.md`'s
baseline recipe runs as written, and the baseline stays green (cargo
test/clippy/fmt, all `scripts/ci/check-*.sh`). Ride-alongs close
optionally; dropping R1/R2 does not fail the cycle.
