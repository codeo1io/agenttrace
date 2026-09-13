# Cycle-7 stewardship request (phase: stewardship)

Run `125bf93302aa4e308cb0739b67f16f33`, attempt `e32952a357334017a6007bc40da7e711`,
2026-09-14. This document is the human-readable companion to the structured
`stewardship_request` in the PhaseResult; the JSON fields are authoritative.
Router note: the `ce-*` compound-engineering router is not loadable as pi
skills in this environment (disclosed in every phase of this run, per the
ideation-doc precedent). No Git topology is chosen here — branch/worktree
planning and dirty-state reconciliation belong to Conductor.

## Repository candidate

- `/work/projects/agenttrace` — canonical campaign checkout. HEAD `df3b621`,
  **not clean**: 7 modified files + 17 untracked paths of validated but
  uncommitted prior-cycle work (inventoried below).
- `/home/agent/.hermes/conductor-worktrees/agenttrace-80c75f65b7/run-125bf93302aa-125bf933`
  — this run's worktree at the same HEAD `df3b621`, carrying only this run's
  phase artifacts (`ROADMAP.md` +210, three untracked docs).

Both point at the same commit; the campaign's phases to date executed in the
worktree. Which tree cycle-7 implements in (and how the canonical's
uncommitted state is reconciled) is Conductor's split, informed by the
overlap map below.

## Change units (decisions with rationale)

The six core units + two ride-alongs selected by the prioritize phase
(`docs/decisions/2026-09-14-cycle-7-batch-selection.md`). Surface cites
re-verified live this pass (worktree at df3b621).

- **Shim `--no-baseline-gate` misregistration fix** (pass-11 A11-5 ≡ the
  canonical tree's uncommitted cycle-4-review F2 pin — independently found
  twice). Surfaces: `crates/agenttrace-cli/src/main.rs:707-712`
  (`flag_takes_value`), `:726` (the entry), `:99-100` (bool definition);
  tests in `crates/agenttrace-cli/tests/entrypoints.rs`; recipe that must
  run as documented: `docs/guides/ci-integration.md:124-127`.
- **Symlinked session-root discovery** (Codex `#42135`; live silent data
  loss). Surfaces: `crates/agenttrace-core/src/discovery.rs:379-391`
  (`fs::read_dir` + `file_type().is_dir()`), the `known_session_dirs`
  walk; doctor listing at `crates/agenttrace-core/src/doctor.rs:285`
  (Providers section gains symlink disclosure); contract tests in
  `crates/agenttrace-core/tests/discovery_contract.rs`.
- **Statusline capture mode `agenttrace statusline`** (candidate 53; the
  strategic headline). Surfaces: new subcommand wiring in
  `crates/agenttrace-cli/src/main.rs` (arg region ~:99-160) plus a new
  module (suggested `crates/agenttrace-cli/src/statusline.rs`); capture
  store + `session_id`-keyed ingestion with overlap dedup in
  `crates/agenttrace-core/src/` (discovery/parser seam); report fields in
  `crates/agenttrace-core/src/reports.rs`; TUI surfaces in
  `crates/agenttrace-tui/src/app.rs`; new guide under `docs/guides/` +
  `README.md`. Schema contract: code.claude.com/docs/en/statusline
  (v2.1.251+).
- **Cache-bound accounting over the deduplicated key union** (A11-2).
  Surfaces: `crates/agenttrace-core/src/session_cache.rs:549-564`
  (`cached_entry`), `:617-648` (`enforce_entry_bound`), `:652-695`
  (`enforce_byte_bound`); bounds declared at `:24-32`; tests in-file.
  **Overlaps the canonical tree's uncommitted F5-5 hunks in the same two
  functions** (see overlap map) — this unit must build on, not revert,
  that state.
- **Installer mode and checksum parity** (A11-3). Surfaces:
  `install.sh:53-54` (chmod), `:66` (mv); checksum emission in
  `.github/workflows/release.yml` (asset matrix ~:48-66, steps from :68);
  parity reference `npm/scripts/install.js`.
- **TUI delivery-worker error surfacing** (A11-4). Surfaces:
  `crates/agenttrace-tui/src/app.rs:1513-1521` (spawn swallowing the
  `Result`), `:1536-1553` (`poll_governance_delivery` treating
  `Disconnected` as completion).
- *Ride-along R1* — pricing snapshot refresh + age disclosure:
  `crates/agenttrace-core/src/pricing_snapshot.json` (regenerate via
  `scripts/pricing/update-snapshot.sh`), `pricing.rs:12-15` (vendored
  snapshot docs), age disclosure in `crates/agenttrace-core/src/doctor.rs`.
- *Ride-along R2* — CI workflow hygiene:
  `.github/workflows/dependency-review.yml:1,23` (pinned action
  `a1d282b36b6f3519aa1f3fc636f609c47dddb294`; fork tolerance),
  `.github/workflows/ci.yml:84` (step gated on `AGENTTRACE_TUI_REAL_DIR`,
  which the workflow never sets).

Rationale for the batch as a whole (impact/risk/effort/dependencies/
strategic value) is recorded in the prioritize decision record and is not
restated here.

## Must remain separate (hints to Conductor's split)

- `main.rs` flag-shim registry fix ≠ `session_cache.rs` bound accounting —
  unrelated subsystems (CLI parsing vs cache persistence).
- Statusline capture (new feature surface) ≠ session-cache bound accounting
  (latent eviction bugfix) — different failure modes, different test
  strategies.
- `install.sh` + `release.yml` (shell/workflow channel) ≠ `crates/**`
  (Rust behavior) — different review and verification audiences.
- CI workflow hygiene (R2) ≠ any Rust change-unit — config-only, no code
  coupling.
- Pricing snapshot refresh (R1, data refresh + disclosure) ≠ statusline
  capture — different data domains; R1 must not ride inside the feature.
- The canonical tree's uncommitted 2026-09-08 state (F5-3/F5-4/F5-5 fixes,
  their tests, the cycle-4 F1–F7 roadmap pins) ≠ cycle-7 change units —
  prior validated work to preserve and reconcile on its own lineage, not
  to fold into cycle-7 CUs.

## Dirty state to preserve (inventoried for Conductor)

Canonical `/work/projects/agenttrace` at `df3b621` (verified this pass;
worktree is clean of all of this):

- Modified (7): `ROADMAP.md` (+116: cycle-4 review F1–F7 pins dated
  2026-09-08, including **F2 = the `--no-baseline-gate` misregistration**,
  i.e. the same defect as batch item 1 — dedupe to one roadmap entry citing
  both lineages at the commit gate); `crates/agenttrace-cli/src/main.rs`
  (+49/−8: F5-3 `--sample` loud rejection, F5-4 text/JSON coverage parity
  — hunks at :160-171, :312-339, :586-625, no line overlap with the shim
  registry at :707+); `crates/agenttrace-core/src/session_cache.rs`
  (+152: **F5-5 headerless-entry bound fix in `enforce_entry_bound`/
  `enforce_byte_bound` (:627-641, :680-695) + 131 test lines — direct
  textual overlap with batch item 4's surfaces**);
  `crates/agenttrace-core/src/insights.rs` (+29: DataHealth fields);
  `crates/agenttrace-cli/tests/entrypoints.rs` (+37);
  `crates/agenttrace-core/tests/discovery_contract.rs` (+121, no symlink
  content — batch item 2 is still novel);
  `crates/agenttrace-tui/src/tests.rs` (+105).
- Untracked (17): `AGENTS.md` (root; absent from the worktree),
  `docs/reviews/2026-09-08-cycle5-review-fix.md`,
  `docs/reviews/2026-09-08-tui-language-preference-flake-fix.md`,
  and 14 `docs/stewardship/` records (cycle-2/3/5/6 reconciliations,
  roadmap-update diffs, compound learnings, pr3-reconciliation).

Worktree dirty state (this run's artifacts, all untracked-or-modified as
left by the roadmap phase): `ROADMAP.md` (+210 vs df3b621),
`docs/decisions/2026-09-14-cycle-7-batch-selection.md`,
`docs/research/2026-09-14-extensions-research-pass9.md`,
`docs/reviews/2026-09-14-adversarial-repository-assessment-pass11.md`.

**ROADMAP.md is dirty on both sides of the same base commit** — the two
extensions are textually disjoint (canonical adds around :950-1040 and the
Completed/closing seams; the worktree adds :965-1027-equivalent and
:1526-1611-equivalent blocks plus status notes), but both insert hardening
items after the same anchor region, so a content merge — not a
checkout-theirs — is required, with the duplicate shim entry deduped.

## Out of scope this cycle (per prioritize phase)

Candidate 56 (Price v2 — dependency-ordered behind 53), candidate 54's
usage-record half, candidates 55/57/58/59, A11-6 platform parity (no
Windows host), A11-7 git timeout + untrusted git-root selection (cycle-8
pair with the delivery-error channel), legacy mid-lane polish
(P3-2/P3-5/control-chars/SQLite-since), and the canonical tree's newly
pinned cycle-4 F1/F3–F7 items (not in this cycle's selection; they await
the next prioritize phase alongside the reconciliation).
