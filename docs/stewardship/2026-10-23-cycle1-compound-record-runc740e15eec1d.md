# Cycle 1 compound record — run c740e15eec1d (repository-maintenance 8b195ab412b74350, cycle 1)

- cycle: 2026-10-22 → 2026-10-23 (phases assess → research → roadmap → prioritize → stewardship → implement → targeted_tests → full_tests → compound)
- compound attempt: b39cd52f35f649448b12d6d21d5510b5 (prior compound 517733d4 reaped by a 429 rate-limit provider error at 454s, message_count 5, envelope absent, scratch never created — zero durable work, forensics in the disposition chain below; phase redone from scratch)
- adoption closure (compound 8b08089094e9492da81870d97bc3c02a): b39cd52f was ITSELF reaped at 2236s by the same 429 class AFTER applying the full delta (band + solutions addendum + this record) and writing patch/postimage mirrors — envelope absent, scratch patch present → delta-present reap, ADOPTED after independent re-verification of every leg (table row + verification section below); closure riders appended to all three artifacts
- wall at compound time: salvage commit c21618da `conductor-salvage: run=c740e15eec1d` on branch `conductor/run-c740e15eec1d`, porcelain 0 in/out (the engine's salvage process committed the cycle's implement batch after the run was marked failed — Salvage-Status failed is a transport verdict only, no validation claim; verified this phase: exactly the 8 attested files, +182/-1, ROADMAP.md still pristine md5 2fb20e06)
- roadmap delta: ONE compound band (30-line pure insertion before the managed footer — 28 lines authored by b39cd52f + 2-line adoption-closure rider appended by 8b0808909); zero mints, zero status flips, census 489 def rows in == out, max rm-2057, footer still last
- evidence base: pre-review cycle outcomes only (assessment, research, roadmap, prioritization, implementation, targeted/full test records); review and shipping outcomes land after this step and are carried forward by the next cycle's assessment. NO tests were executed in compound.
- tree delta of THIS phase: `M ROADMAP.md` (+30/-0: 28-line band + 2-line adoption-closure rider), `M docs/solutions/workflow-issues/provider-reaped-delegate-attempts-redo-from-scratch-on-census-match.md` (addendum + closure), `?? docs/stewardship/2026-10-23-cycle1-compound-record-runc740e15eec1d.md` (this file) — deliverable artifacts only, no repo-root scratch.

## Disposition chain (every attempt of the cycle, with verdict)

| phase | attempt | verdict |
|---|---|---|
| assess | aae54c30 | REAPED (provider family) — envelope absent, event log progress-ticks only, session_reaped; zero durable work → REDONE |
| assess | 5e462e30 | done — adversarial sweep at tip 769453c3 (release build, lane-private probes): svg_card golden cached 291/3938 vs bundled 215/4443; cache_rate census 3,941 models / 2,230 read_absent; first_party 7/2; token 1/1; windows E0433 :38 + E0599 :488/:502; `--baseline /dev/zero` 320.9MB peak RSS (insights.rs:724 chain) |
| research | 64dfe24e | REAPED at ~24s (start + 1 tick + session_reaped) — zero durable work → REDONE |
| research | ea808803 | done — upstream still frozen (15ed07f2, exactly 6 past merge-base cb625d73, releases flat v0.10.1 2026-10-05); fork master CI red (runs 313/318/324, 2026-10-21); PR#30 diff 425 lines / 13 files; LiteLLM 4,245 keys (+1,581/-467 vs bundle 3,131; 4 argon keys vs 0); C1-C10 with 48 raw payloads |
| roadmap | 806a1255 | REAPED — empty scratch, declared dead by successor |
| roadmap | b7d84f3c | REAPED at ~25min/25msg but left a COMPLETE deliverable in scratch (band + builder + pristine/postimage + patch + census/roundtrip logs) → ADOPTED |
| roadmap | e3d941f5 | done via ADOPTION — all legs independently re-verified; +7/-0 pure insertion (patch md5 fd6e0ba5, postimage md5 ce35aa48); worktree left porcelain 0; delivery was spool-side pending commit-gate landing |
| prioritize | 051c994d | done — selected 'truthful master test surface at tip' (U1-U4, ZERO prod code) from the unresolved pool; deferred backlog recorded (see Context) |
| stewardship | 6be2f9b9 | ABORTED (provider abort at 34.6s, results/ record says 'This operation was aborted') — zero durable work → REDONE |
| stewardship | 8b9c445f | done — all 12 surface anchors re-verified FIRST-HAND at tip 58227d21 before composing the stewardship request |
| implement | 01fef1e6 | completed the batch; fold rejected SOLELY for a missing KTD13 changed_surfaces attestation (work intact) |
| implement | 7ba2987b | REPAIR, not redo — re-emitted the attestation, re-verified fresh: +182/-1 over 8 files, cross-checks clean (windows-gnu --tests BOTH crates), fmt/clippy rc0 |
| targeted_tests | 2fe3bf9d | done — focused battery ALL GREEN (core 5 suites 41/0, entrypoints 65/0, cli bin 115/0, fmt/clippy rc0, windows-gnu Finished); porcelain md5 f309f380 byte-stable pre/post; engine targeted runner itself printed 'unable to prove narrow scope; fallback=full' → focused battery per phase budget (targeted + changed_surfaces:[] precedent, run e9021a4b) |
| full_tests | a2a16cbb | done — work-order full_command VERBATIM, gate rc=0, 95 suites / 1,487 passed / 0 failed / 0 non-ok; hermetic lane under ~/.cache/at-full-a2a16cbb; dispatch digest declared verbatim |
| compound | 517733d4 | REAPED (429 rate-limit, 454s, message_count 5) — zero durable work → REDONE by b39cd52f |
| compound | b39cd52f | REAPED at 2236.1s (429 rate-limit, results/ record) — DELTA-PRESENT: full delta applied in-tree + patch/postimage mirrors written (spool scratch + `~/.cache/at-compound-b39cd52f/`), envelope absent → ADOPTED |
| (engine) | conductor-salvage | c21618da committed the implement batch post-full_tests — content preservation only |
| compound | 8b0808909 | done via ADOPTION of b39cd52f — identity, roundtrip (`git apply --check -R`), pristine base, census, mirror parity, fresh origin fetch ALL re-verified first-hand; closure riders appended to band + addendum + this record; dangling scratch-dossier citation corrected (rule 8) |

## Status updates delivered

In the ROADMAP compound band (this cycle, ROADMAP.md:3870):

- EXECUTED riders (flips stay reserved to the commit gate, rm-012 convention): **rm-1537** windows compile cure — acceptance green in-worktree (both crates' `--tests` windows-gnu cross-checks Finished; E0433 :38 + E0599 :488/:502 gone); PR#30's remaining value superseded (post-landing: close #30 + stale #26/#29). **rm-1954 PARTIAL** — the 4 catalog-lane suites (svg_card, cache_rate_truth, first_party_pricing_extract, token_conformance) now wired through the existing `with_isolated_pricing_env` guard (tests/support/mod.rs:113); the STANDING suite-enumeration guard remains open. **rm-1543 partial** — fresh CI run at the new tip owed post-landing.
- Roadmap-band fog note: full re-materialization of the cycle's roadmap band (6 riders, zero mints) with integrity pins, because the byte-verbatim patch is fog-lost (see Errata E-1).
- ZERO mints, ZERO status flips, census 489 in == out, footer last — landing belongs to the commit gate.

## Errata of record

- **E-1 (fog loss, recurrence):** all 8 spool scratch dirs of this run were swept before compound — including both roadmap scratches AND the `~/.cache/at-roadmap-b7d84f3c/` mirror that the roadmap phase had restored from spool one phase earlier. Only `delegate/a2a16cbb…-scratch/` (full_tests, 3 files) survived; it has been rescued to `~/.cache/at-compound-b39cd52f/full-tests-scratch-rescue/`. The roadmap band's byte-verbatim patch (+7/-0) and postimage are gone; integrity pins recorded (patch md5 fd6e0ba5 / sha256 97171cf2, postimage md5 ce35aa48 / sha256 6c8d6f60); the compound band's fog note is the re-materialization of record and the landing re-anchors from it BY TITLE (rm-708 convention).
- **E-2 (canonical fallback):** typed results survived for every attempt (`~/.hermes/conductor-delegate-spool/results/<attempt>.json`) and carry the full phase summaries — forensics order for a swept scratch: envelope → event log → typed result → mirrors. All 10 typed results of this run are now mirrored at `~/.cache/at-compound-b39cd52f/typed-results/`.
- **E-3 (wall drift, verified benign):** the worktree HEAD at compound time is salvage c21618da, not the full_tests-attested `58227d21 + dirty`. Content verified identical to the attested batch (8 files, +182/-1, ROADMAP.md pristine 2fb20e06); validation evidence still describes that exact content.

## Reusable lessons / prevention rules

1. **Reap-storm disposition taxonomy (standing protocol, all variants exercised this cycle):** zero-durable reap → redo from scratch; complete-deliverable scratch → adopt only after independent re-verification of every leg; attestation-only fold rejection → repair, never redo; engine-side `conductor-salvage:` commit → treat as content, not a verdict (re-verify drift maps to attested surfaces; validation claims do not ride along). 429-rate-limit and provider-streaming failures are one transport class.
2. **Fog immunity (2nd recurrence of the 2026-10-18 lesson):** spool scratch AND a mirror restored one phase earlier were both swept within a day. The emitting phase embeds the full band-of-record text in-tree — md5s alone do not survive. Mirror-at-write-time to `~/.cache` and rescue surviving scratch immediately.
3. **Ambient pricing-cache pollution:** a host carrying `~/.cache/agenttrace/pricing.json` (4,245-key LiteLLM fetch, `load_catalog_for_current_env` preference) machine-couples red catalog-lane suites. Every NEW catalog-consuming test goes through `with_isolated_pricing_env` from birth; rm-1954's standing enumeration guard owns enforcement.
4. **Hermetic full-gate recipe:** isolated CARGO_TARGET_DIR + clean XDG_CACHE_HOME under `~/.cache` (never /tmp — that sweeper takes build lanes within minutes), warm `--no-run` first; the gate envelope digest folds an unknown base and can never equal the dispatch digest — declare the dispatch digest verbatim, prove porcelain byte-stability. A shared CARGO_TARGET_DIR can link a freshly built CLI against a stale sibling rlib; cross-binary consistency claims need an isolated target dir.
5. **KTD13:** implement-phase folds require `validation_evidence.changed_surfaces` (machine-readable path list); in this repo only `.github/workflows/*` classify executable, so declare the real path list anyway (cost this cycle: one fold, repaired cheaply).
6. **Verify every inherited number before writing it into a durable artifact** — this phase recomputed the census (489/0 dups/max rm-2057), the salvage stat, and the ci matrix first-hand.

## Context for the next maintenance cycle

1. **LANDING SEAM FIRST (commit gate):** reconcile salvage c21618da + the fog-lost roadmap band against origin/master 87f18ebc (+13 past 58227d21, overlapping pricing.rs / pricing_snapshot.json / svg goldens / reports.rs); then the fresh CI run at the new tip (rm-1543 rider) — windows/macOS lanes should cure via U1-U3 (E0433/E0599 + /dev/full gating + macos-15), unblocking the Require-all-CI aggregator; close PR#30 (superseded) and stale #26/#29.
2. **rm-176 pricing snapshot refresh:** +1,581/-467 drift, 4 argon keys land mechanically; blocked on svg golden churn + schema rung 72→73 pairing; sibling pricing landings at 87f18ebc must reconcile first.
3. **rm-1954 residual:** the standing suite-enumeration guard (auto-wrap every catalog-lane binary; the 4 hand-wired suites are the seed set).
4. **rm-341:** baseline-path .git-size-coupled memory (320.9MB peak RSS PoC, insights.rs:724 chain); acceptance = bounded/streamed .git walk + RSS re-pin.
5. **Deferred-from-prioritize backlog:** discovery.rs `-d` zst-twin double-count (rm-548 vs rm-2056 semantics + cached-lane parity; unblocks once PR#30 closes); mcp.rs:466 initialize -32022 floor (live old-host probe owed); read caps reports.rs:1395 / insights.rs:724; CHANGELOG dedup + gate; attribution sweep.
6. **Fleet context for the next assessment:** review/shipping outcomes of THIS cycle's batch (review verdict, landing, CI truth at the new tip) are the first thing the next assessment must carry forward — they are deliberately absent here (pre-review compound).

## Artifacts and verification

- ROADMAP band: worktree `ROADMAP.md:3870-3899` (+30/-0 pure insertion; `git diff --stat -- ROADMAP.md`); census `grep -c '^- id:'` = 489 pre == post, dup check 0, managed footer still the last line (unique).
- Compound delta + mirrors: `~/.cache/at-compound-b39cd52f/` (compound-delta-b39cd52f.patch md5 598c5a66, ROADMAP.postimage.md, solutions.postimage.md, typed-results/ ×10, full-tests-scratch-rescue/ ×3 — all byte-verified against the worktree by 8b0808909) and spool scratch `delegate/b39cd52f…-scratch/` (the patch ONLY — the "COMPOUND.md dossier, forensics logs" this file originally cited here were never written before the reap; rule-8 correction: the dossier of record is `delegate/8b0808909…-scratch/COMPOUND.md` + `verify-8b0808909.log`, mirrored at `~/.cache/at-compound-8b0808909/`).
- Solutions addendum: `docs/solutions/workflow-issues/provider-reaped-delegate-attempts-redo-from-scratch-on-census-match.md` (rules 9-10, this cycle's evidence).
- Verification commands (all non-test, run this phase): `git rev-parse HEAD` = c21618da; `git status --porcelain` (pre-edit: 0; post-edit: exactly the 3 deliverable paths); `git show --stat c21618da` (8 files +182/-1); `grep -c 'c740e15eec1d\|8b195ab4' ROADMAP.md` = 0 pre-insertion (no prior band); census checks above.
- Adoption verification (8b0808909, all non-test): `git apply --check -R compound-delta-b39cd52f.patch` rc0 (worktree drift byte-exact vs the authored patch, pre-rider); md5 parity scratch patch == mirror patch (598c5a66), mirror postimages == worktree files (ROADMAP f7f49903, solutions 4a7e4f69, this record pre-rider 9ac9ff6c); pristine `git show HEAD:ROADMAP.md` md5 2fb20e06 / 3,870 lines; fresh `git fetch origin` → 87f18ebc; census re-run 489 / 0 dups / max rm-2057 / footer last; svg_card guard recount 10 wrapped + 1 catalog-free (`theme_parser_rejects_unknown_values` :254); `results/b39cd52f….json` (429, 2236.1s) + `results/517733d4….json` (429, 454.0s) — both deaths are the provider transport class, matching every recorded claim.
