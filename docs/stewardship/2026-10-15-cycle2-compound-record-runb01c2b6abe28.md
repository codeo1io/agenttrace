# Cycle 2 compound record — run b01c2b6abe28407593bdf7cd0ec37221

Repository: agenttrace (`/work/projects/agenttrace`, fleet worktree `run-b01c2b6abe28-b01c2b6a`)
Chain: `repository-maintenance:1bf8dd62723248ada047573cdd5154cd:cycle:2`
Base: `8bc0587` (== `origin/master` at prioritize time, `git ls-remote` verified)
Compound attempt: `b63cad57ad6f4da78fd11b437a54de8a` (2026-10-15, after full_tests `9aa0dbc3`)
Scope: **pre-review cycle evidence only** — no tests were executed at compound; every
outcome below is consumed from the recorded targeted/full batteries.

## 1. Phase ledger (attempt → outcome)

| Phase | Attempt | Outcome |
|---|---|---|
| assess | `2f5ccf7a` | green; prior attempt `9bea886b` provider-reaped, heartbeats-only log, nothing durable → redone from scratch |
| research | `4f1bfbfc` | 7 evidence-backed candidates, raw/ dumps (crates.io, LiteLLM live 4,084 keys, npm, opencode v2 sql.ts, ccusage) |
| roadmap | `b3f93f10` | band delivered spool-side (banner + 8 riders + mint rm-1509); **scratch + both mirrors reaped before compound** → regenerated here from the typed envelope |
| prioritize | `7b53e047` | batch selected: `advisory closure + OpenCode v2 delivery-evidence readiness` (A1/A2/A3) |
| stewardship | `fa967946` | structured stewardship_request, file:line surfaces verified live |
| implement | `ed261c20` | 9 M + 2 ?? in-tree; adopted ~60% of the ENOSPC-reaped prior attempt after audit, redid `Cargo.lock` from scratch for first-hand provenance |
| targeted_tests | `852d7227` | focused battery green, one lane per invocation under host load ~34 |
| full_tests | `9aa0dbc3` | 64 suites 1137/0; fmt/clippy/MSRV/conformance/smoke/deny lanes green; 2 standing reds proven delta-untouched via pristine control |
| compound | `b63cad57` | this record; prior attempt `19fb2e56` provider-reaped, heartbeats-only, nothing durable → redone from scratch |

## 2. The batch (implemented, uncommitted — the commit gate lands it)

- **A1 — advisory closure (rides rm-018).** `cargo update -p rustls` 0.23.45 → 0.23.46,
  exactly one package update — the rustls `Cargo.lock` version+checksum pair plus its
  dependency-edge lines (the `rustix`/`errno` `windows-sys` target edges flip; no other
  package version or checksum changes); rustls is transitive-only (zero manifest hits → zero manifest edits).
  Closes RUSTSEC-2026-0346..0349 (advisory-db dated 2026-10-14, patched >=0.23.46 all four;
  0.23.46 release-confirmed `max_stable` on crates.io). Proof: `cargo deny check advisories`
  ok at targeted, `cargo deny --all-features check` advisories/bans/licenses/sources all-ok
  at full. Row stays `candidate` — only the advisory arm closed, not the
  Scorecard/coverage/git-cliff acceptance.
- **A2 — OpenCode v2 summary_diffs (rm-1509, minted this cycle, flipped candidate →
  implemented at compound).** Read side + report surface: presence-guarded read
  (`sqlite_sessions.rs`), `Metrics` fields + decode (`lib.rs`), session-cache schema 65
  (`usage_occurrence_contract` ladder rung), SQLite snapshot schema 13 (core `--lib` oracle
  pins 13, round-trips `summary_diffs`/`summary_diffs_dropped`/`summary_diffs_divergence`,
  rejects schema 12), `--delivery-evidence` surfaces per-file counts with divergence
  disclosed on its own line, never reconciled (`governance.rs`), ±column fixtures under
  `crates/agenttrace-core/tests/fixtures/opencode-summary-diffs/`, CHANGELOG entry,
  docs schema prose swept (`docs/guides/governance-reports.md` 64→65 / 12→13).
  **Residual vs acceptance** (next-cycle starter 1): doctor census arm, diagnostics-counter
  wiring, per-file PATHS granularity.
- **A3 — upstream test temp root (rides rm-241).** `outside_a_git_repository_fails`
  (`upstream.rs:247-266`) derives its scratch root from `non_repo_temp_parent()`
  (`upstream.rs:256`: probes `std::env::temp_dir()`, then `/var/tmp`, then `/tmp`, and takes
  the first root with no `.git` at or above it — no env override exists). 13/13 under BOTH
  `TMPDIR=/tmp/at-tt-852d7227` and the
  dot-nested assess failure-repro shape. The `insights.rs:1159/:1278` lanes remain
  TMPDIR-sensitive — contained by the batteries' dot-free TMPDIR mapping, class not closed.

## 3. Recorded validation outcomes (consumed, not re-run)

- **targeted `852d7227`**: core `--lib` 404/0 · `opencode_summary_diffs` 8/0 ×3 consecutive ·
  `usage_occurrence_contract` 5/0 · `opencode_agent_summary` 3/0 · `discovery_contract`
  104/0 · `upstream` 13/0 under both TMPDIR shapes · fmt rc0 · clippy 0 warnings ·
  `deny advisories` ok · changelog-structure + locked-cargo gates rc0.
- **full `9aa0dbc3`**: 64 suites **1137/0** (assess baseline 63/1129 — delta = the new
  8-test suite) · fmt rc0 · clippy `--all-targets -D warnings` rc0 (94 s) · MSRV 1.89 check
  rc0 in its own target dir · conformance + smoke lanes rc0 · `cargo deny --all-features
  check` all-ok. Dispatch `full_command` was empty → suite = the 7 `ci.yml` jobs mapped
  job-faithfully (fleet precedent).
- **Standing reds, delta-untouched, pristine-control-proven** (both rc=1 at the clean
  8bc0587 control too, `pristine-controls.log`):
  1. `scripts/ci/check-install-runtime.sh` test D — "install receipt must record the
     default pinned ref" (rm-051 install-ref family; wall row rm-017 owns the arm).
  2. `actionlint` SC2001 on the `ci.yml` gate-surface-audit `echo | sed` surface —
     no wall row; banner-noted only.

## 4. Reusable lessons / prevention rules

- **L1 — reap-class triage.** A dead attempt's durable trail determines the response:
  heartbeats-only event log (this run's assess `9bea886b` and compound `19fb2e56`) →
  nothing to adopt, redo from scratch; dirty-tree delta with no envelope (implement's
  ENOSPC prior) → audit the delta line-by-line, adopt the sound parts, redo the rest with
  first-hand provenance. Never redo blindly before checking the tree diff.
- **L2 — transitive-only advisory closure.** Grep the manifests before any surgery: no
  `rustls` hit anywhere → `cargo update -p rustls` is the entire change surface, and it
  moved exactly ONE lock entry with 56 deps unchanged. The closure proof is `cargo deny`
  green (advisories lane), not the version bump itself.
- **L3 — TMPDIR dot-nesting is contained, not closed.** A dot-nested TMPDIR (any path
  inside a git repo) breaks tests that derive scratch roots from `temp_dir()`. A3
  env-guarded the CLI lane; `insights.rs:1159/:1278` remain sensitive. Every battery must
  map a dot-free TMPDIR (or set the env guard) until rm-241's acceptance (per-test
  exclusively-owned roots, all three lanes) lands.
- **L4 — shared-fixture races need per-test unique temp homes.** The ±column suite raced
  `remove_dir_all` vs plant/load on a shared temp-home fixture path; the atomic-counter
  unique suffix fixed it (3 consecutive green). Same family as the otel/csv fixture-dir
  races recorded on the wall.
- **L5 — one heavy lane per cargo invocation.** Host load sat ~34; per-lane invocations
  kept failures attributable and avoided the load-spike trap.
- **L6 — empty `full_command` → job-faithful local ci.yml mapping.** Run the 7 ci.yml
  jobs locally with the workflow's env mapping (AGENTTRACE_BIN, AGENTTRACE_CI_OUT,
  dot-free TMPDIR, scoped HOME, MSRV in its own target dir) and map action-form syntax to
  the local CLI (the `cargo deny check --all-features` action form is rejected by the local
  deny CLI — documented mapping, not a skip).
- **L7 — standing-red adjudication requires the pristine control.** Rerun the red gate at
  the pristine base before attributing it to the batch; both this cycle's reds reproduced
  at 8bc0587 untouched.
- **L8 — reaped roadmap scratch is recoverable from the typed envelope.** The rider list,
  anchors, mint, and banner position all live in the roadmap phase's PhaseResult envelope;
  regenerate from it (this record's band regeneration is the worked example). Recorded
  postimage md5s of reaped artifacts are untrusted/unreproducible — bytes on disk are the
  only authority; disclose the supersession chain explicitly.

## 5. Next-cycle starters (concrete, evidence attached)

1. **rm-1509 residual arms** — doctor census, diagnostics-counter wiring, per-file PATHS
   granularity in `--delivery-evidence`; flip to done only after those arms are evidenced.
2. **rm-051 quick win** — install receipt default pinned ref (fix shape: derive from
   `install.sh`, cf. `check-install-ref-drift.sh:35-38`; standing title-reconcile caveats:
   sibling `88313799`/`07f36b2ffec7` claims test D/E cleared; unlanded `220ece42` band
   rm-989 same subject).
3. **actionlint SC2001 quick win** — ci.yml gate-surface-audit `echo | sed` →
   `${variable//search/replace}`; no wall row today.
4. **rm-164 variant-axis ingestion** — live per-key axes `_flex` 242 / `_priority` 242 /
   `_batches` 392 / `_ultrafast` 8 vs bundled 106/0/0/6 (research `4f1bfbfc` raw/).
5. **rm-176 snapshot refresh** — bundled 3,131 vs live 4,084 = 953 structural drift;
   ledger band rm-1440/901/1443 NOT landed at this base.
6. **Standing watches** (roadmap banner): MCP spec 2026-07-28 frozen; codex `rust-v0.162.1`
   (0.163 alpha-only); claude-code 2.1.299 (+2.1.300-next idle-compaction); fork `15ed07f2`
   frozen (~23rd check); kimi-code 2.2.0 wire-key drift watch (rm-400).

## 6. Roadmap delta (this phase's required artifact)

- Live wall `ROADMAP.md` @ 8bc0587: md5 `1d59addaa633d721268bc6c222cdf03d`, 392 def rows.
- Final postimage: md5 `cdcbc831d53b57ccfefdb63b6223264c`, 2561 lines, 393 def rows
  (2539 + 22 insertions; the 2589 figure quoted in early compound prose was wrong —
  line-count corrected at review round 3), zero duplicate ids, footer still last line,
  `## Closed items` at :2452, mint row at :2446.
- Patch `roadmap-delta-b63cad57.patch` (md5 `1c7bccb6bc2a849f9aa5f3b1c0ad2540`): 12 hunks,
  **22 insertions / 0 deletions — pure insertion** against the live wall (census corrected
  at review-fix `d8e651f8`: `git apply --stat` counts 22 added lines; the earlier 61 figure
  counted context+header lines). Contents: 2 banners
  (compound c2 newest, then the regenerated cycle-2 roadmap banner, both immediately after
  the INTEGRATION block), 8 reconstructed roadmap riders (rm-400/rm-164/rm-176/rm-201/
  rm-507/rm-715/rm-845/rm-018), 4 compound riders (rm-018 advisory-arm closure, rm-241
  CLI-lane containment, rm-017 standing-red re-proof, rm-1509 VALIDATED flip narration),
  1 mint row rm-1509 (`status: implemented` — candidate → implemented at compound;
  done-flip reserved to the commit gate, rm-012 convention).
- Provenance: the b3f93f10 band bytes were regenerated from the surviving typed envelope
  `delegate/b3f93f10bc9d4d0f8a2dbff2db5ab5ec.json` (rider list, anchors, mint, banner
  position); its recorded postimage md5 `7e6b00f5…` is unreproducible (source reaped) and
  superseded by this delivery. Fog census at this delivery: rm-1509 claimed only by this
  run's envelopes; rm-1510..rm-1520 claimed by sibling fleet envelopes; next free rm-1521
  (re-census before any future mint; reconcile by title at integration).

## 7. Commit-gate seams (do not re-derive)

1. `git apply` `roadmap-delta-b63cad57.patch` (scratch:
   `delegate/b63cad57ad6f4da78fd11b437a54de8a-scratch/`) — single patch carries band +
   compound; the reaped b3f93f10 patch must NOT be awaited.
2. `git apply` `stewardship-record-b63cad57.patch` — adds this file.
3. Stage exactly: the implement batch (9 M: `CHANGELOG.md`, `Cargo.lock`,
   `crates/agenttrace-cli/tests/upstream.rs`, `crates/agenttrace-core/src/governance.rs`,
   `crates/agenttrace-core/src/lib.rs`, `crates/agenttrace-core/src/session_cache.rs`,
   `crates/agenttrace-core/src/sqlite_sessions.rs`,
   `crates/agenttrace-core/tests/usage_occurrence_contract.rs`,
   `docs/guides/governance-reports.md`; 2 ??: `crates/agenttrace-core/tests/fixtures/
   opencode-summary-diffs/`, `crates/agenttrace-core/tests/opencode_summary_diffs.rs`)
   + `ROADMAP.md` + this record. Never `git add -A`.
4. Optional gate-time done-flip: rm-1509 → done (only if the gate accepts the VALIDATED
   rider narration as sufficient; otherwise it stays implemented for next cycle's arms).
5. Title-twin checks before push: grep the wall + spool for any sibling rm-1509 /
   summary_diffs claim (live census: none — this run only).
