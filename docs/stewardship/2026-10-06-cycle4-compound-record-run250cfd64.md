# Cycle 4 compound record — run 250cfd64 (repository-maintenance b099a3a5)

Date: 2026-10-06 · Base: 700a67cc98f903c95a77682fd8c51bc581c23089 · Compound attempt: dcdef640 (pre-review)

## Batch

Theme: *count every session exactly once; keep shipped operator docs honest.*

| id | title | role | state after compound |
|---|---|---|---|
| rm-511 | Deduplicate symlinked/hardlinked session files at the discovery collection boundary | lead | implemented (done-flip reserved for the commit gate) |
| rm-512 | Sweep `skills/*/SKILL.md` agenttrace invocations into `scripts/ci/check-docs-commands.sh` | rider | implemented (done-flip reserved for the commit gate) |

Uncommitted worktree delta carried to the commit gate (post-review-fix b6d507b7): 6 modified
files, +454/−12, plus this untracked record —
`crates/agenttrace-core/src/discovery.rs` (SessionFileTargets/SameFileIdentity threaded through
collect_session_files, collect_session_files_cached, walk_session_files_cached [replay + store
legs], walk_session_files file arm), `crates/agenttrace-core/tests/discovery_contract.rs`
(6 alias tests), `crates/agenttrace-cli/tests/entrypoints.rs`
(cli_alias_paths_report_one_session), `scripts/ci/check-docs-commands.sh` (skills sweep leg),
`ROADMAP.md` (wall + this compound), `CHANGELOG.md` (fleet `no-changelog-section` marker block
plus the rm-511 Unreleased Fixed bullet — see below).

## Attempt trail

implement e86ace2e (work complete; fold-rejected for exactly one defect — missing KTD13
`validation_evidence.changed_surfaces`) → implement ebd2bb22 (attestation repair; delta
byte-identical) → targeted_tests 985b92b2 (the first targeted dispatch died of host ENOSPC after
the work but before the phase_result write — no durable result; redone) → full_tests b10bf831 →
compound dcdef640 (this record).

## Recorded validation outcomes (consumed here as evidence; NOT re-run at compound)

- **Targeted (985b92b2):** `discovery_contract` 84/0 (6 new alias tests); `entrypoints` 26/0;
  docs gate rc0 — the skills leg is silent-on-success, so execution was proven via `bash -x`
  xtrace plus a 6/6 flag-existence sweep against `--help`; fmt rc0; clippy `--all-targets -D
  warnings` rc0 on both changed crates. Live PoC on `/tmp/at-assess-f376d372/poc/sym`:
  1 session / $0.0011 / `Session files: 1` / search `(1)` (assess pre-fix: 2 / $0.0022 / 2 / (2)).
- **Full (b10bf831):** the `ci.yml` full+deny jobs mirrored locally — fmt/clippy rc0;
  17 test targets **417/0** (assess baseline 410 + the batch's 7 tests); release build rc0;
  8 shell gates rc0 (private `AGENTTRACE_CI_OUT`, isolated HOME); ruby / npm test /
  cargo-manifests / locked-cargo / bash -n / cargo-deny all rc0; TUI real-smoke correctly
  skipped (CI-conditional variable unset, identical to CI default). Digest
  `validation:v1:7dc026a5017de172af07ab913beb9e393ba9ec2b6f4a0a6288d648e99cf07ad0` stable across
  the targeted and full folds (the one later tree change is `.md`-only and re-derived unchanged
  via the engine's own `validation_policy`).
- **Full-suite event:** `check-plugin-version.sh` initially failed rc1 (merged fork tags
  v0.7.2–v0.8.0 with no CHANGELOG sections — the documented fleet-wide tag-visibility state;
  provably batch-independent: neither git tags nor CHANGELOG were in the batch delta).
  Remediated with the sanctioned rm-303 `no-changelog-section` marker block, copied
  byte-identically from run-4a68825724aa-4a688257 → gate rc0. This run is the second carrier of
  the identical 18-line block: whichever commit gate lands first carries it; the other dedupes
  trivially; integration carries it forward.

## Residual / for the review gate

- rm-512's acceptance red-green leg (a mutated scratch `SKILL.md` with a bogus flag tripping the
  gate) was not evidenced at compound time — **discharged by review 1f52da3e** (corpus at
  /tmp/at-review-rm512-red trips rc1) and re-verified after the b6d507b7 race fix.

## Lessons (process; also on the cycle-4 compound banner)

1. **KTD13 attestation:** implement folds REQUIRE `validation_evidence.changed_surfaces` naming
   every engine-executable path (`scripts/**` classify executable; `crates/**` do not). A
   completed implement can be fold-rejected on the attestation alone.
2. **Host ENOSPC:** a full targeted dispatch was lost after the work but before the
   `phase_result` write. Check `df` and prune `/tmp/at-*` artifacts at phase start on this host.
3. **Silent-on-success gate legs** need positive execution proof (`bash -x`) when the leg emits
   no output; a bare rc0 cannot distinguish execution from skip.
4. **CI shell gates on fleet hosts:** always a private `AGENTTRACE_CI_OUT` (mktemp) + isolated
   HOME — the default `/tmp/agenttrace-ci` is shared and foreign doctor.json tails corrupt it.
5. **Naming/tooling quirks:** `crates/agenttrace-cli` is package `agenttrace` for `cargo -p`;
   local cargo-deny 0.20.2 has no `--all-features` flag (plain `cargo deny check` is the
   equivalent).

## Next-cycle context

- Upstream #312 **merged** 2026-10-05T10:00:36Z (cb625d73): rm-251/rm-232 deferral is now a
  post-merge reconcile design (upstream cache schema 26 vs fork 23-with-different-content;
  #311 workbuddy-sum vs the 66a7d797-band rm-450 keep-last reconcile owed at integration).
- rm-085 (F2 `--lang` no-op) remains the top unimplemented assess finding; corpora preserved at
  `/tmp/at-assess-f376d372/poc/{langcsv,csvinj,neg}` (subject to /tmp sweep risk — 364aa3be
  precedent: re-derive from recorded shapes if swept).
- Upstream v0.10.0-wave adoption arms: rm-042 (R2), rm-045 (R6); deps bump table: rm-007 (R1).

## Review fix (b6d507b7, 2026-10-06, post-compound)

Review attempt 1f52da3e returned NEEDS_CHANGES; all five findings addressed:

1. **HIGH — racy flag check** (`scripts/ci/check-docs-commands.sh`, skills leg): `printf | grep -q`
   under `set -euo pipefail` could turn a *matching* flag into `flag missing from --help`
   (grep -q exits on first match, printf takes SIGPIPE on the ~6KB help text). Replaced with a
   here-string check plus ERE-metachar escaping for malformed tokens. Verified: gate green x15
   fresh runs; the bogus-flag red leg still trips rc1 (`--no-such-flag-exists` corpus at
   /tmp/at-review-rm512-red).
2. **MEDIUM — CHANGELOG gap:** added the rm-511 Unreleased Fixed bullet (symlink/hardlink
   double-count fix).
3. **MEDIUM — seam omission:** commit-gate seams (this file and the ROADMAP banner) now name
   the untracked stewardship record explicitly — a `git commit -am` would silently drop it.
4. **LOW — stale totals:** banner and this record now carry the post-review-fix totals
   (6 modified +454/−12 + 1 untracked; verified `git diff --numstat` post-fix — a +466 figure
   in an earlier draft was a mis-sum).
5. **LOW — survivor asymmetry:** the uncached walk now admits alias candidates in sorted
   order, mirroring the cached walk, so the surviving alias path is identical cold and warm;
   discovery_contract 84/0 and entrypoints 26/0 rerun green, fmt/clippy rc0.

## Commit-gate seams

7-path delta: the batch's 5 files + `CHANGELOG.md` (marker block + rm-511 Fixed bullet) + this
record itself (untracked — stage it explicitly; `git commit -am` would drop it). `discovery.rs` is shared with three sibling
lanes (2d37535d `matches_filter` / 542a3ee6 LoadReport / 7197db2a admission cap — all outside
the walk arm; region-disjoint). ROADMAP carries this cycle's wall on the 700a67c base — apply
fleet patches in order, then this compounded postimage.
