# Compounding Record — repository-maintenance `389e223a636a4783baa2d9b64ab4b94b` cycle 1

Base `93acc06` (origin/master tip at dispatch) → landed conductor-salvage `5cc1b95f`
(content-preservation landing, Salvage-Phase `full_tests:full_tests`, no validation claim
inside the commit — the validation evidence lives in the typed envelopes named below).
Compounding written at clean HEAD `5cc1b95f`, pre-review, from cycle evidence only.

## 1. What cycle 1 shipped — "pricing-drift fuse defusal"

The only calendar-fused defect on the wall: `.github/workflows/pricing-drift.yml:23`
(Monday `0 6 * * 1` cron) was red BOTH ways at base — removal_guard:true exited 1 at
:55-57 forever, and clearing the guard without fixing the regen step would have redden
master via the byte-pinned svg goldens on the first GITHUB_TOKEN auto-PR (no CI on those
PRs; the `UPDATE_CARD_GOLDEN=1` arm existed test-side at `svg_card_contract.rs:37` but
was invoked by nothing). The batch (prioritize `d9e79fe5`, stewardship `4391d617`,
implement `e4098c47` after fold-rejected `4d602419`):

1. **466 upstream-retired costed keys adjudicated** — dated per-key notes in
   `scripts/pricing/snapshot-drops.txt` (459 `openrouter/*` namespace purge + 7
   stragglers; 466 dated notes in total, all `2026-10-17`). First sanctioned use of the rm-588
   manual-drop gate; the union-merge "carry retired keys forever" failure mode is now
   curable with evidence.
2. **Snapshot re-bundled** — `pricing_snapshot.json` @ capture 2026-10-17 (const
   `PRICING_SNAPSHOT_DATE` in `pricing.rs`, footer "LiteLLM snapshot 2026-10-17
   (bundled)"); conformance case 005 literal updated 0.0001 → 0.0 with the derivation
   recorded in the manifest per pack rule 1.
3. **Golden-regen acceptance closed (ROADMAP rm-176 acceptance arm)** — the workflow now
   pins toolchain actions, runs `UPDATE_CARD_GOLDEN=1 cargo test -p agenttrace-core
   --test svg_card_contract` in the regen step, re-runs the byte-pin assertion, and
   stages the three goldens beside bundle+const; PR body carries the no-CI disclosure.
   *(Extended 2026-10-18 by review fix 00e01125 F1: the same step now also verifies the
   four consumer suites — `pricing_snapshot_hygiene`, `cache_rate_truth_contract`,
   `first_party_pricing_extract_contract`, `token_conformance` — against the refreshed
   bundle and refuses to open the PR on red, so a moved census pin is a loud workflow
   failure and a manual-sweep signal, never a red master at the GITHUB_TOKEN merge.)*
4. **Ledger** — CHANGELOG entry + the rm-176 riders (deadline/acceptance-closure/
   executed-refresh) landed with the salvage.

## 2. Evidence chain (all typed envelopes under `~/.hermes/conductor-delegate-spool/delegate/`)

| phase | attempt (dead→redo) | envelope |
|---|---|---|
| assess | `d8e8aec4`→`2081ae39` | 7 findings F1–F7, full suite 1298/0/81 at base |
| research | `110abe8f`→`23033714` | RC1–RC6 (codex zstd-compression, LiteLLM drift 4201/3131, models.dev tiers, CC 2.1.302 idle compaction) |
| roadmap | `1ffff93d`→`88c27c51` | 9-hunk append-only spool band (md5 `7cf255ef…`, base `c7a7651f…`) |
| prioritize | `9ea1516e`→`d9e79fe5` | batch selection + fleet-fog resolution (canonical `ecf84dc`; stranded `d1479d6`/`77e1480`) |
| stewardship | `4391d617` | 9 surfaces + 4 must_remain_separate pairs |
| implement | `347d99e4` (provider-dead)→`4d602419` (fold-rejected, KTD13)→`e4098c47` | 15-path delta, KTD13 replay with controls |
| targeted_tests | `015247bc` | focused battery green; digest `validation:v1:4936e116…` reproduced pre+post |
| full_tests | `f9892823` (provider-dead; delta salvaged as `5cc1b95f`)→`53f1c679` | ci.yml-verbatim workspace suite 1298/0/81, 02:01:56Z→02:08:19Z |

Six of eight first attempts were provider deaths (identity: absent `delegate/<attempt>.json`,
event log ending `session_reaped` exit_reason=failed; implement's true first attempt `347d99e4`
included; census identity corrected 2026-10-18 by review d77fa453 F2, which caught it omitted
and `f9892823` double-counted). Characterization corrected 2026-10-18 by review 00e01125 F2 —
this record's earlier "~30s, zero output" wording was wrong: durations re-derived from the
event logs are 28s (`d8e8aec4`) and 31s (`9ea1516e`) but 218s (`f9892823`), 531s (`110abe8f`),
2524s (`347d99e4`), and 3122s (`1ffff93d`), and durable output was NOT uniformly zero —
`347d99e4` left a quarantine scratch (`gen_drops.py`, `refresh-run.log`,
`removal-analysis.txt` under `quarantine/scratches/20261017T193835Z/`) whose delta the
implement redo adopted, and `f9892823`'s delta was salvaged as landing `5cc1b95f`. Reap
triage must census duration AND scratch/envelope state per attempt, never assume zero-durable
from the reaper exit alone. The redo-from-scratch-on-census-match rule carried the run — see
the 2026-10-18 Addendum
on `docs/solutions/workflow-issues/provider-reaped-delegate-attempts-redo-from-scratch-on-census-match.md`.

## 3. Lessons & prevention rules (reusable)

1. **A test-side regen arm is dead code until the mutating pipeline invokes it.** The
   golden trap was not "goldens are stale" but "the workflow that changes the inputs
   does not run the regen path". Rule: any automation that seds a constant consumed by a
   byte-pinned fixture must run that fixture's regen+verify arm and stage its outputs in
   the same step (`pricing-drift.yml` now does; grep `UPDATE_CARD_GOLDEN` in
   `.github/workflows/` to confirm — it must never return to zero hits).
2. **An empty `full_command` in the work order defers to the repo's own CI definition.**
   Authority chain used: `ci.yml` test lane verbatim (`cargo test --locked -p
   agenttrace-core -p agenttrace-tui -p agenttrace`) — never a local convenience script.
3. **Retired-with-evidence is a first-class drop class.** The rm-588 gate exists precisely
   so the union merge cannot hold 466 dead keys hostage; the adjudication notes
   (openrouter purge class + stragglers) are the reusable template for the next purge.
4. **Spool bands are base-pinned; salvage landings are not band applications.** The
   9-rider band `88c27c51` (base `c7a7651f…`) did NOT land — the salvage carried only the
   +3 in-tree folds. Never replay a stale spool patch onto a moved wall: re-derive riders
   by title (rm-708 convention) against the current file. The band remains recoverable in
   the spool scratch + `~/.cache/at-roadmap-88c27c51/`.
5. **Validation-only phases prove digest currency by reproduction, not by assertion.**
   Copying the dispatch digest verbatim is correct exactly when nothing executable
   changed — demonstrated by running the engine's own `validation_digest()` before and
   after the battery (match both times, `015247bc`).
6. **Salvage landing + later green envelope**: when both exist, cite the envelope as the
   evidence record and the salvage commit as the landing vehicle (rm-176 rider of
   2026-10-18 does this).

## 4. Context for the next maintenance cycle

- **LIVE PROOF POINT — Monday 2026-10-19 06:00 UTC**: first scheduled run of the lane.
  BOTH outcomes are acceptable (reworded 2026-10-18 by review 00e01125 F3 — the original
  "rc0 + auto-PR" conjunction was unsatisfiable: rc0 ⇒ mode=clean ⇒ no PR): (a) gate rc0,
  removal_guard clear, mode=clean, no PR; or (b) gate rc1 actionable with guard+desync clear
  → bump arm regenerates bundle+const+goldens, post-regen gate rc0 + consumer suites green
  in-run, PR opens with the goldens staged. Reopen rm-176 only on rc2, removal_guard SET,
  desync, post-regen gate ≠ rc0, or a red master after a merge; a red consumer suite in the
  workflow is the manual census-sweep signal (review 00e01125 F1), not a lane defect.
  Check `gh run list --workflow pricing-drift.yml`.
- **Queued candidates** (prioritize d9e79fe5, in value order): by_org/by_branch
  cost-finite guard (assess F2 — governance/reports folds, small); insights relative-cwd
  attribution (F4); models.dev `cost.tiers` second pricing source (RC3; bundled snapshot
  has 0 tiers vs 655 live — long-context under-count risk); CC 2.1.302 idle-compaction
  session-shape watch (RC-lane rider on the CC row).
- **Ceded by title**: codex `.jsonl.zst` discovery admission → stranded salvage `d1479d6`
  (verify it or re-derive; ccusage `reader.rs` is competitor parity evidence). Cline
  intake → `77e1480`. Do not re-mint titles.
- **Numeral hygiene**: this cycle minted ZERO new roadmap numerals; def-max on the landed
  wall is unchanged from the 10-17 census (`rm-1874`); fog frontier was `rm-1999`
  (spool-side) at prioritize time — re-census before any mint.
- **Open seam**: the sibling run `1fb751544907` delta shared item-3 surfaces; if it lands
  separately, reconcile by title (its pricing.rs/snapshot/goldens edits vs this cycle's).

## 5. How to verify this record

- `git show --stat 5cc1b95f` (15 paths); `grep -c '^2026-10-17' scripts/pricing/snapshot-drops.txt` → 466;
  `grep PRICING_SNAPSHOT_DATE crates/agenttrace-core/src/pricing.rs` → "2026-10-17";
  `grep UPDATE_CARD_GOLDEN .github/workflows/pricing-drift.yml` → present;
  envelopes under `~/.hermes/conductor-delegate-spool/delegate/` per §2;
  `bash scripts/pricing/drift-check.sh --selftest` → 8/8 arms (recorded in `015247bc` scratch).
