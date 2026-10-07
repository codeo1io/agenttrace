# Cycle-3 compound record — run 17319815eab4 (repository-maintenance 52493714 cycle 3)

Compound attempt 41d85c53ed2e4300b429addc97b91001, 2026-10-07. Consumes ONLY pre-review
cycle evidence; NO test execution at compound per the phase contract. Worktree base
09cb224ae469a0c5499a86d434665ff6b928d708 (HEAD immobile); this record is spool-side per
the enforcing-hygiene precedent — landing it as
`docs/stewardship/2026-10-07-cycle3-compound-record-run17319815eab4.md` is a commit-gate
rider decision.

## 1. Batch and delivery state

Batch "cycle-3 CLI action-contract residuals" (prioritize ecd58ce9, stewardship 45fe89a1):
- **Unit 1 — rm-301 acceptance extension** (assess 7276 A1/A2): `--clear-cache` /
  `--update-pricing` early exits silently swallowed co-requested action arms
  (doctor / list-models / test-match / statusline-report). Root cause = predicate drift
  (`has_session_action` main.rs:1672 vs `has_post_pricing_action` :1648).
- **Unit 2 — rm-656** (this cycle's roadmap mint, A5): `--doctor -f json -o FILE`
  artifacts dropped the config-layer disclosure payload the terminal path prints.

Implemented (5cc91240) as an UNCOMMITTED 2-file worktree delta @ 09cb224:
`crates/agenttrace-cli/src/main.rs` (new `has_followup_action` behind both early-exit
guards :281/:298, doctor `-o` top-level `config_disclosure` embed, 20-flag unit matrix)
and `crates/agenttrace-cli/tests/entrypoints.rs` (+217 lines, the 2 red-first goldens).
Review/shipping outcomes deliberately absent here — the next cycle's assessment carries
them (phase contract).

## 2. Recorded outcomes (consumed as evidence at compound, NOT re-run)

| Phase | Outcome |
|---|---|
| implement 5cc91240 | red-first proof: both goldens FAILED at 09cb224 pre-edit (entrypoints :1743 `side_effect_early_exits_do_not_swallow_coadjacent_actions`, :1848 `doctor_json_artifact_carries_config_disclosure`); then `-p agenttrace` 112/0; fmt + clippy `--all-targets -D warnings` rc0; live PoCs recorded |
| targeted 8fad99c0 | fmt rc0; clippy `-p agenttrace --all-targets -D warnings` rc0; 112/0 (bin 55 incl. the 20-flag matrix, csv_export 7, entrypoints 35 incl. both goldens, launch_guards 4, upstream 9, warm_cache_pricing 2); digest `validation:v1:611748d0…` engine-replicated PRE == POST battery (validation-only turn proven: mtimes unchanged) |
| full b38c951d | ci.yml full+lint+deny lanes as command authority (full_command shipped empty): fmt rc0; clippy `--locked --all-targets` 3-crate rc0; `cargo test --locked` **519/0 across 25 test binaries** (516 at assess + the batch's 3); release build rc0 3m34s (13,531,976 B, RETAINED); **14/14 scripts/ci gates rc0** (incl. TUI real PTY smoke + aggregate `check-rust-release-local.sh` which re-ran the 25-suite pass); ruby/npm/manifests/plugin-version/locked-cargo/install-ref-drift/`bash -n`+`sh -n` rc0; `cargo deny` advisories+bans+licenses+sources ok |
| digest lineage | `validation:v1:611748d0286e02e62ff25352166fb778bf89cbec1775b951d8d114c617d1ec95` declared VERBATIM at both validation turns, each independently re-derived live == dispatch token (the 2-file delta is `crates/**`, digest-immobile under the engine classifier; ROADMAP edits at compound cannot move it either) |

Zero regressions found at full scope; zero fixes needed after implement.

## 3. Dead-attempt forensics (four provider-side deaths this cycle, one salvage)

- **implement 927300c8** — reaped 01:25:21 after 9 messages, typed result absent, BUT its
  +217-line red-first test layer in `tests/entrypoints.rs` landed at 01:29:09 (~4 min
  POST-reap: the reap-does-not-kill-in-flight-work pattern). Adopted line-by-line with 2
  corrections by 5cc91240. **Rule: sweep the tree + spool for post-reap-mtime artifacts
  before redoing any dead implement phase.**
- **implement 016c89a2** — 58s, 1 message, nothing durable. Census-clean death.
- **research f4222084** — 23s, 8 event-log messages, zero artifacts. Redone from scratch
  (5207b).
- **stewardship 7d2e3349** — 34s, 2 messages, zero artifacts. Redone from scratch
  (45fe89a1).

## 4. Prevention rules (this cycle's additions)

- **PR-A (dead-attempt salvage):** never discard a dead attempt's tree state without an
  mtime-ordered sweep; reaps lag in-flight writes by minutes. Adoption requires
  line-by-line verification + declared corrections (this cycle: 2 corrections to the
  adopted test layer).
- **PR-B (command authority when full_command is empty):** the repository's CI workflow
  file is the authoritative suite (full+lint+deny lanes); run them verbatim with
  `AGENTTRACE_BIN` pinned to the freshly built release binary. Third+ confirmation in
  this campaign family.
- **PR-C (gate output routing):** route `AGENTTRACE_CI_OUT` (and any gate artifact dir)
  to `/tmp` — gates write into it and would dirty the enforced porcelain census of the
  worktree.
- **PR-D (target/ lifecycle):** the shared target dir is swept between phases; the
  release binary (3m34s cold build) is needed by 4+ gates AND the commit gate —
  full_tests builds and retains it; downstream phases must not sweep.
- **PR-E (cargo-deny local repro):** local cargo-deny 0.20.2 accepts
  `cargo deny --all-features check` (flag BEFORE the subcommand) and rejects the
  ci.yml-action argument order `check --all-features` (rc2 "unexpected argument").
  Plain `cargo deny check` covers all four policy categories; fleet memory #16687
  re-confirmed fresh this cycle.
- **PR-F (spool-chain delivery):** roadmap/compound wall deltas in enforced-hygiene
  campaigns ship as stacked spool patches over the prior phase's postimage; verify the
  whole chain from HEAD byte-identically (each `git apply` rc0 + `cmp`) and prove
  insertion non-idempotence (re-`--check` must fail).

## 5. Wall accounting

- rm-656 flipped candidate → **implemented** (+ EXECUTED bullet).
- rm-301 (implemented since cycle 1, landed 9eec1194): extension EXECUTED bullet appended
  under the cycle-3 note. Done-flips stay reserved for the commit gate (rm-012).
- **ZERO ids minted at compound** (def rows 229 → 229, live-verified by build_compound.py
  guards: count, uniqueness, footer-last, anchor-uniqueness). agenttrace-space next free
  stays **rm-657**.
- Banner prepended newest-first at wall line 16 (under `## Open items` line 14), above
  the cycle-3 roadmap banner.

## 6. Chain verification (reproducible)

```
HEAD:ROADMAP.md (09cb224)
  -> git apply roadmap-4cadcb45.patch      # rc0, == 4cadcb45 postimage (cmp)
  -> git apply ROADMAP.compound.patch      # --check rc0, apply rc0, == ROADMAP.final.md (cmp)
                                             re-apply --check FAILS (one-shot insertions)
```
Delta: +5/−1 (banner + separator blank + 2 EXECUTED bullets + rm-656 status-line rewrite).
`ROADMAP.final.md` sha256 `ae4c5993f2cea0cb025128ad41f097c4b3cc90c1de356006f41aa1963746550b`;
`ROADMAP.compound.patch` sha256 `4ebbc5afb47e825f92fc01e9895b10de59aa4b61c6c5176c8b8f329e7dfb47fb`.

## 7. Next-cycle context (from prioritize ecd58ce9's deferral table — do not re-litigate)

1. **rm-560** (codex per-ChatGPT-account attribution) — the dedicated-cycle LEAD. Its
   landed acceptance demands a wild-corpus survey at the target base BEFORE
   implementation; mechanism already pinned on the row (ccusage #1818:
   `token_count.rate_limits` weekly `resets_at` minute-fingerprint mod 7d +
   `session_meta.creator_account_id`; fork has 0 parser reads of either field —
   statusline.rs:95 already parses the primitive).
2. **rm-341 A4** (`-o` double-emit, ~20 sites) — WAIT for 2880c10f's unlanded rm-652 to
   land, then extend (compose seam main.rs:218-221).
3. **A3 integration note** — re-run the copilot-reset PoC at ≥ 518170a7 to record the
   landed keep-newest-by-checkpoint guard's flip (this base 09cb224 predates it).
4. **rm-377** — gated on its own upstream-parity request.
5. **rm-006/rm-176** — LiteLLM table byte-stable today (live sha `53479dd3…` identical to
   the morning pull; +8/−2 drift recorded on the row); next refresh is a cadence call.
6. **Un-minted rider notes from full b38c951d** (process observations, no repo defect
   proven): ci.yml:263 deny-lane `--all-features` argument is not locally reproducible in
   that position (PR-E); `check-rust-tui-real-smoke.sh` auto-discovers real session data
   from the invoking environment (this host: `/home/agent/.pi/agent/sessions`, sampled
   2/222) while CI gates it on the `AGENTTRACE_TUI_REAL_DIR` repo var.
7. The next cycle's assessment carries this batch's review/shipping outcomes (phase
   contract).

## 8. Commit-gate seams

- Stage exactly the 2 modified files (`crates/agenttrace-cli/src/main.rs`,
  `crates/agenttrace-cli/tests/entrypoints.rs`); porcelain is otherwise clean, zero
  untracked.
- Land the roadmap delta: apply `roadmap-4cadcb45.patch` then
  `ROADMAP.compound.patch` (or write `ROADMAP.final.md` wholesale) — the chain is
  verified above.
- CHANGELOG: 1 Fixed bullet under Unreleased for the five swallowed action combos
  (rm-301) + 1 Fixed bullet for doctor `-o` disclosure parity (rm-656) — lands with the
  merge per lineage convention, NOT minted at compound.
- Release binary already built and retained (`target/release/agenttrace`, 13,531,976 B,
  current with HEAD + delta) — binary-dependent gates need no rebuild.
- Integration reconciles ids BY TITLE (880a7b9e/5af7cbb6 discipline); origin/master
  518170a7 (238 defs, ceiling rm-603) does not yet know rm-656.
- Done-flips for rm-301's extension and rm-656 are reserved to the commit gate
  (rm-012 precedent).
