---
ce-handoff: v1
cwd: /home/agent/.hermes/conductor-worktrees/agenttrace-80c75f65b7/run-cf7556984e5d-cf755698
repository: /work/projects/agenttrace
repo_root_sha: 5ef66c045f53e38b812bac94309207cb6fa31e5f
branch: conductor/run-cf7556984e5d
head: 5ef66c045f53e38b812bac94309207cb6fa31e5f
---

# Cycle 2 implementation record — Deterministic attribution + terminal trust

- **Date**: 2026-10-03
- **Cycle**: 2, campaign `6a10ae64` (batch: rm-230 lead + rm-231 / rm-232 / rm-240 riders)
- **Base**: `5ef66c0` (pre-existing uncommitted in worktree at start: `ROADMAP.md` +67 roadmap-phase mint, preserved and extended)
- **Worktree**: `conductor-worktrees/agenttrace-80c75f65b7/run-cf7556984e5d-cf755698`
- **Selection**: `delegate/19c92f82…-scratch/prioritization.md` (73 open items scored; sibling-wall collision census)
- **Status**: implemented + validated pre-review; 10-file uncommitted worktree state (commit/push/PR deliberately out of scope of this phase)

## What shipped

| # | Item | Title | State |
|---|------|-------|-------|
| 1 | rm-231 | Hermetic project-decode and grouping tests (no host-ancestor state) | implemented, suite-green proven on hostile host |
| 2 | rm-230 | Deterministic project-dir decode (host-state-independent attribution) | implemented, PoC triple re-run live |
| 3 | rm-232 | Sanitize terminal-control bytes in CLI text renderers (OSC/CSI injection) | implemented, OSC-52 fixture swept clean |
| 4 | rm-240 | Isolate the release-local gate script (shared CI_OUT wipe + hardcoded bin path) | implemented, concurrency-proven |
| — | (out-of-band) | `docs/guides/governance-reports.md:66` snapshot schema 6→7 | one-word truth-fix by targeted_tests (gate-unblock; not batch scope) |

## rm-231 — Hermetic project-decode and grouping tests

Root cause: `projects_group_worktrees_and_decode_agent_dirs`
(insights.rs:543-568) built fixtures under `std::env::temp_dir()` and
round-tripped them through the decoder, which probed the live filesystem —
so a host directory shadow-matching an intermediate component
(`/home/agent/.hermes/tmp/delegate/agenttrace` on hermes delegate hosts)
flipped attribution and failed the assertion at :563. 110/1 suite-red under
the delegate TMPDIR, green under `/tmp`; cascaded into
`check-rust-release-local.sh` rc101 (its cargo-test step died before its own
checks).

Changes: all decode/grouping fixtures moved to `unique_decode_root()`
(temp_dir + `at-<pid>-<tid>-<seq>-<label>`, per-process `AtomicUsize`); the
grouping test rewritten hermetically on that root; new
`decode_ignores_shadow_directories_at_plausible_ancestors` PLANTS the host
shadow sibling and asserts attribution is unaffected.

Verification (pre-review): core lib 116/0 with the shadow physically present
under the hostile TMPDIR; 116/0 under `TMPDIR=/tmp`; full workspace 285/0 in
the same hostile condition; rc101 cascade gone.

## rm-230 — Deterministic project-dir decode

Root cause: `decode_agent_project_dir` (insights.rs:205-238) reversed the
non-injective `'-'`-encoded project dirname by greedily probing the LIVE
filesystem and joining the first `is_dir()` hit — `my-repo ≡ my/repo ≡
my.repo ≡ my_repo` all encode identically, so an unrelated host decoy dir
re-attributed sessions. Assess PoC triple: decoy filter rc0 / true filter
rc1 / decoy-removed rc0 — same session, attribution flipped by host state.

Changes: rewritten as a budget-bounded deterministic DFS (1024 `is_dir`
probes; longest-run-first; separator variants `-`, `.`, `_` deduped per run —
duplicates were a real bug caught in verification: dash-free runs spell
identically in all three variants and recursed 3^k into duplicate
completions, mislabeling unique decodes Ambiguous; empty/dash components
consumed without movement); first complete decode equals the old greedy
walk's result whenever that walk completed, and still finds a decode where
greedy dead-ends; cwd-first resolution unchanged. New
`project_decode_status` → `ProjectDecodeStatus {NotConsulted, Resolved,
Ambiguous{path,shadowed}, Unresolved{encoded}}`; `DoctorReport` carries a
`project_decode` JSON block plus "Project attribution: N resolved, N
ambiguous, N unresolved" text with sanitized samples and a targeted
recommendation. Regression tests pin the assess PoC triple
(`decode_prefers_the_longest_verified_path_over_a_decoy_split`,
`decode_ignores_shadow_directories_at_plausible_ancestors`).

Verification (pre-review): live PoC triple re-run on the release binary —
true filter rc0 WITH the decoy present (was rc1), decoy filter rc1, still
rc0 after decoy removal; `--doctor` names the decoy as a shadowed
alternative and clears to "1 resolved" after removal.

Known budget-shaped bound (disclosed in code comments): probe budget 1024
and max 8 disclosed alternative decodes keep the DFS cheap on pathological
ambiguity; truncation is deterministic (fixed search order) but `--doctor`
ambiguity lists can be incomplete on adversarial corpora.

## rm-232 — Sanitize control bytes in CLI text renderers

Root cause: `sanitize_line_segment` existed at statusline.rs (landed rm-034,
`--statusline-report` only) but no CLI render path used it; `--search`/`--latest`
`-f text` printed journal-derived strings raw — od-verified
`\x1b]52;c;aGVsbG8\x07` (clipboard-write OSC) emitted verbatim, 1 sequence each.

Changes: `sanitize_line_segment` promoted to the crate API (lib.rs re-export)
and wired into `report_search_text` fields (name/cwd/source_tool/model/matches,
search.rs), reports.rs text surfaces (top-tools `model_used`, anomaly
`detail_for_language`, overview session values), and CLI
`render_session_list` TSV cells. Contract: control bytes → U+FFFD, printable
CSI tails legitimately survive; the TSV test allows the tab separator as the
only control byte.

Post-review correction (independent_review ff2edfcf F1/F2, fixed in
review-fix f62926a4): this section originally credited the governance-family
plain renderer `render_plain_value` (main.rs String arm) as wired — it was
NOT in the implement diff. A crafted mcp_server_name carried a raw OSC-52
into `--mcp-governance -f text` (live PoC, grep ESC=1). It is wired now:
every string leaf routes through sanitize_line_segment, pinned by the new
governance_text_render_sanitizes_report_string_leaves unit test; post-fix
re-probe grep ESC=0.

Verification (pre-review): OSC-52 fixture re-run — `--search`/`--latest`
text show ESC=0 BEL=0 FFFD=1 (was a raw clipboard-write sequence).

## rm-240 — Isolate the release-local gate script

Root cause: `check-rust-release-local.sh` did `rm -rf "$AGENTTRACE_CI_OUT"`
(deleting SHARED gate output mid-suite when unset/shared) and hardcoded
`$repo_root/target/release/agenttrace`, ignoring `AGENTTRACE_BIN` unlike the
other 11 gates.

Changes: writes only under `mktemp -d "$AGENTTRACE_CI_OUT/release-local.XXXXXX"`
(no parent wipe) and honors `AGENTTRACE_BIN` with the repo release binary as
fallback.

Verification (pre-review): static (bash -n, shellcheck rc0); functional probe
— seeded sibling-gate.log + release-local.AAAAAA/old-artifact.txt survived a
full rc0 gate run alongside the new unique dir; full gate end-to-end rc0 with
isolated HOME/XDG, `AGENTTRACE_BIN` at a distinct path (final line names it),
REAL dirs=testdata, TUI smoke not skipped — 11 `test result:` lines all ok.

## Out-of-band change and prior-attempt forensics

- The one-word docs truth-fix (`governance-reports.md:66`, schema 6→7) was a
  pre-existing red at base (stale guide vs `SQLITE_SNAPSHOT_SCHEMA_VERSION=7`),
  aborting the changed gate mid-flow under `set -e`; targeted_tests fixed it
  per "fix failures until they pass". The fork already carries schema 7 at
  `fd5532f` (with session-cache schema 21 from the sibling pricing lane —
  this base's session-cache schema 20 is correct here); upstream `52ab2cd`
  still says schema 4. Fold into commit attribution explicitly — not batch
  scope creep.
- Implement attempt `e85d9ef3` was reaped after ~7 minutes with only progress
  pings in its event log, no typed artifact, zero worktree changes → the
  batch was redone from scratch and declared; nothing adopted.

## Validation record (pre-review; consumed at compound, not re-run)

- `TMPDIR=/home/agent/.hermes/tmp/delegate cargo test --workspace` → rc0,
  285 passed / 0 failed across all 3 workspace members (core 116 lib + 7 doc,
  cli 21 + 10 entrypoints + 2 launch_guards + 9 upstream, tui 46/72), with
  the shadow dir `…/delegate/agenttrace` (+stray jsonl) planted during the
  run — the exact host state that produced assess's 110/1 baseline at the
  frozen base.
- `cargo fmt --all -- --check` rc0; `cargo clippy --workspace --all-targets
  -- -D warnings` rc0.
- Composite `check-rust-release-local.sh` rc0 end-to-end (targeted_tests,
  same tree, not rerun by full_tests per use-don't-redo).
- Assess's two gate reds both cleared: docs-schema (truth-fix) and
  release-local rc101 (rm-231/rm-240).
- Digest lineage: the only engine-executable-classified file in the delta is
  `scripts/ci/check-rust-release-local.sh` (scripts/ prefix;
  `.rs`/`.md` are non-executable per the classification rules);
  targeted (c0531a6c) and full (66f0249b) validation declared the
  dispatch-time validation_digest VERBATIM with fold re-derivation unchanged.
- Environmental note for future implement/test phases: the host was
  fork-exhausted during implement (LA ~28-36; flaky rustc/collect2 aborts on
  unrelated crates) — `CARGO_BUILD_JOBS<=4` and one `cargo clean -p rustls`
  cleared it; prefer quieter windows for final_validation.

## Lessons and prevention rules compounded

- **Rule 7** in `docs/maintainers/test-flake-prevention.md`: fixtures that
  exercise host-observing code get unique roots AND plant the host state
  they need absent (paid for by the 110/1 delegate-host suite-red).
- **Base-staleness triage**: a gate red found at a frozen base that is
  already fixed at origin/master HEAD is base-staleness, not a new defect —
  diff the upstream surface before minting; the ff belongs to the merge
  phase (this cycle's docs-schema red was exactly this class).
- **Live-FS probing anti-pattern**: attribution (project filters,
  governance, leaderboards, TUI) must never depend on first-hit
  `is_dir()` walks over uncontrolled directories; when a name encoding is
  non-injective, decode deterministically and DISCLOSE residual ambiguity
  (`--doctor project_decode`) rather than silently picking.
- **Prior-attempt forensics**: a reaped delegate session whose event log
  holds only progress pings is a non-attempt — redo from scratch and
  declare; do not partially adopt.
- **Gate hygiene**: a CI script must never `rm -rf` a directory it does not
  exclusively own, and must honor the same `AGENTTRACE_BIN` contract as its
  siblings.

## Next-cycle candidates (context for cycle 3)

1. **rm-233 config file (recommended lead)** — first-class
   `~/.config/agenttrace/config.toml` + project override + `--config`, with a
   documented precedence chain and `--doctor` disclosure (model it on the new
   `project_decode` disclosure block). Unlocks rm-234 weekly budgets.
   Evidence on disk: `delegate/c8fbb160…-scratch/ccusage-config.html`
   (ccusage 4-method priority chain), flag/env census (50 `#[arg]` flags, 4
   env knobs).
2. **Pricing family rm-196/175/176 as one batch** — LiteLLM pricing bundle
   etag churn two days running (aaa7b007/3026393B on 10-02 → fccab44/3028610B
   on 10-03, bundle dated 09-13).
3. **Rider batch rm-236/238/239** — sqlite LIKE failure markers, limit-zero/
   NaN gate thresholds, zh README parity + case-insensitive `.jsonl`
   admission (all with live PoCs from assess 172562a8).
4. **rm-235 truthful MSRV** — rust-version 1.80 vs edition-2024 deps
   (ratatui-crossterm ≥1.88, zeroize ≥1.85); `cargo +1.80.1` on this host
   can re-prove the floor.
5. Decisions still open: rm-011/237 consumer-contract (truncation
   disclosure shape), rm-053 (consume AgentMeasure pack), rm-194/195/197
   release-lane (needs sibling coordination).
6. Watch: upstream `52ab2cd` 9 commits ahead (inventoried; its
   governance guide still claims schema 4 — worth a fork-carried patch
   upstream eventually); PR #279 dep-bump open; OTel semconv v1.44.0;
   agent-console 786★ (10-02); tanuu5/usage-log + kenar as rm-234 demand
   signals.

## Commit-gate reconciliation notes

- Delta = 10 modified files: `ROADMAP.md` (roadmap mint + compound c2),
  6× `agenttrace-core/src/*.rs`, `agenttrace-cli/src/main.rs`,
  `docs/guides/governance-reports.md`, `scripts/ci/check-rust-release-local.sh`.
- **ID RACE (documented collision class — merge by TITLE, never id)**:
  this wall's `rm-230..240` were minted against the campaign-local body
  (rm-001..212 + this mint) while origin/master `fd5532f` already carries
  fleet `rm-230..238` as COMPLETELY DIFFERENT items (fleet rm-230 there =
  "Join flat-transcript tool results to their calls", implemented) and
  sibling campaigns hold rm-241..243+ in flight. Reconcile per
  `docs/solutions/workflow-issues/roadmap-campaign-id-collision-at-integration.md`;
  re-derive the fleet high-water mark before the next mint.
- ff `5ef66c0 → fd5532f` at merge clears the base-stale docs-schema red;
  batch files have zero hunk overlap with the 4 sibling commits (verified at
  implement), except cite sibling run 364aa3be's main.rs import-list hunks
  by title+signals at integration.

## Evidence map

- Assess: `delegate/172562a8…-scratch/` (report.md, evidence.log,
  gate-check-*.log, poc-misattr-{decoy,true,nodecoy}.json, osc-*.txt)
- Research: `delegate/c8fbb160…-scratch/` (research-report.md,
  ccusage-config.html, new-entrants.log, ecosystem.log, cratesio.log)
- Roadmap: `delegate/543bc0be…-scratch/roadmap-update.diff` (+67 mint)
- Prioritize: `delegate/19c92f82…-scratch/prioritization.md`
- Stewardship: `delegate/6810e9f2…-scratch/stewardship-request.md`
- Implement: `delegate/77f9dd78…-scratch/` (implement-batch.diff 1007 lines,
  poc-doctor-*.txt, osc-*.txt)
- Targeted: `delegate/c0531a6c…-scratch/` (evidence.log, gate-run.log)
- Full: `delegate/66f0249b…-scratch/` (workspace-test.log, fmt.log, clippy.log)
- Fixtures (host /tmp, persist across sessions on this host):
  `/tmp/at-assess-172562a8/env/{misattr,osc,many}`,
  `/tmp/at-impl-77f9dd78`, `/tmp/at-tt-c0531a6c`
