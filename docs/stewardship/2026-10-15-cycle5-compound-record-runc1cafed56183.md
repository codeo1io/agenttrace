# Cycle 5 compound record — run `c1cafed561834ed19e6e05fa154e1be3`

- **run:** c1cafed561834ed19e6e05fa154e1be3 (worktree `run-c1cafed56183-c1cafed5`)
- **requirement:** repository-maintenance `d304684478ee47508a0618625a687fb7` cycle 5
- **base:** `3e90993dc94350498be08c51eb62fa0da3831f95` (== origin/master at dispatch; wall ROADMAP.md md5 `1d59adda…`, 392 id rows, def-max rm-1373)
- **phase chain (attempt ids):** assess 68fdc77d · research e1e09468 · roadmap a5fa40b9 · prioritize 93789f24 · stewardship ff47386a · implement 668492c3 · targeted_tests fd91a652 · full_tests 1a9de0d3 · **compound 7676255e (this record)**
- **status at compound handoff:** pre-review; batch uncommitted in the worktree; review/commit/CI happen after this phase

## Outcome

Cycle 5 implemented and validated the two-row batch **"bounded opencode admission + npm exit fidelity"** (LEAD rm-1437 + rm-1438), and compounded the cycle back into the wall as a rebuilt-and-extended ROADMAP.md band (banners + 7 riders + 3 mints, +32/−0) plus this record. No new validation was executed at compound — every test outcome below was **consumed from the recorded targeted/full artifacts**, per the phase contract.

## 1. What the cycle shipped (uncommitted batch, 5 files, +439/−14)

| File | Δ | Content |
|---|---|---|
| `crates/agenttrace-core/src/parser.rs` | +96 | `OPENCODE_TRANSCRIPT_READ_CAP_BYTES = 10 MiB`; `read_opencode_records` stat-caps each message/part transcript **before** reading (`metadata().len()` — over-cap files are never admitted into memory); `OpencodeAdmissionStats {capped_files, capped_bytes}` accumulated across both call lanes (message dir + per-message part dir) and threaded into `session.metrics.line_skips` as `opencode_transcript_files_capped` / `opencode_transcript_bytes_capped`; empty-gate relaxations admit fully-capped sessions with their disclosure instead of `unsupported session format` |
| `npm/bin/agenttrace.js` | +37 | numeric exit codes pass through verbatim; signal deaths re-raise to the wrapper's own pid (`process.kill(process.pid, result.signal)`) with a 128+signum fallback map |
| `crates/agenttrace-core/tests/opencode_bounded_admission.rs` | new (3 pins) | `under_cap_corpus_discloses_nothing_and_keeps_outputs`, `over_cap_message_transcript_is_skipped_and_disclosed`, `over_cap_part_transcript_is_disclosed_and_lanes_accumulate` |
| `npm/test/launcher-exit.test.js` | new (4 pins) | numeric passthrough, zero passthrough, SIGINT re-raise, SIGTERM re-raise |
| `CHANGELOG.md` | +17 | two entries, merged into the pre-existing `### Fixed` (see lesson L4) |

**Stewardship correction encoded in the batch:** the original assess phrasing claimed the npm launcher "coerces exit codes". Re-verified against the code at stewardship time: `process.exit(result.status ?? 1)` passes numeric rc through unchanged — the real gap is the **signal path** (`result.signal` discarded). The batch, its tests, and rm-1438's signals line all encode the corrected gap. (Honest-red proof recorded at implement: HEAD launcher vs SIGINT stub → `status:1 signal:null`; fixed → signal re-raised.)

## 2. Validation (consumed, not re-run at compound)

- **targeted (fd91a652):** fmt rc0; clippy core rc0 0 warnings; core 50 suites **825/0** incl. all 3 admission pins; npm **8/8**; batch diff md5 verified byte-identical before/after.
- **full (1a9de0d3, ci.yml lanes verbatim):** `--locked` tests core/tui/cli **63 suites 1121/0**; clippy `--locked` `--all-targets -D warnings` **0 warnings**; MSRV `+1.89 check` rc0; conformance runner **13 cases asserted (1 xfail-pinned)**; **7 smoke gates rc0** (real-bin/real-tui incl. expect); npm lane 8/8; actionlint **pristine-A/B** — `git archive 3e90993` + actionlint reproduces SC2001 at `ci.yml:192:9` rc1, i.e. base-owned, corroborated by the red "Lint and repo contracts" check-run on sibling landing 8bc0587. Validation digest `694c7e1f…` proven **3× identical** across the changelog fix and the whole battery.
- **standing reds at base, all delta-untouched:** actionlint SC2001 `ci.yml:192`; check-install-runtime test D (`^ref=v0.9.0$` vs `install.sh` REF=v0.10.1 — the rm-051 arc); cargo-deny rustls 0.23.45 advisories RUSTSEC-2026-0346..0349 (fix owned by a sibling lock bump).

## 3. Band provenance — reap and rebuild, disclosed

The roadmap phase's band artifacts (ROADMAP.patch, postimage, APPLY-SEAM, builder) and every other scratch mirror of this run (implement diff, full-validation logs, /tmp research blobs) were **swept before compound**; only the typed phase envelopes survived. The roadmap band was therefore **rebuilt from the a5fa40b9 typed envelope**, which carried the full census, rider set, mint titles and evidence numerals; the envelope's build-time md5s (patch `9bf4f5c2…`, postimage `637a64f3…`) are unrecoverable and superseded by this band's own census: **395 rows / 395 unique / def-max rm-1439 / +32/−0 pure insertion / managed footer last / new wall md5 `b3397225…`**.

Three reaped/redone attempts occurred across this run, each disclosed at the time: assess 8e3290df (4-event reaped log, redone as 68fdc77d), full_tests 75148ab9 (serve-hold transport artifact, redone as 1a9de0d3), compound 28892e8e (6 lifecycle events, no typed artifact, clean tree → **redone from scratch as this attempt**).

## 4. Wall drift and fog collisions (commit-gate duties)

- **Live wall at compound:** origin/master `16dcc9a` — ROADMAP.md md5 `1f1ab384…`, 395 id rows, def-max rm-1502 (vs this band's base 3e90993 / `1d59adda` / 392 / rm-1373). Numerals rm-1437..rm-1439 are **absent from the live wall**; title-level reconciliation is owed hunk-by-hunk, newest-landed-first.
- **Fog collisions:** rm-1438's numeral is triple-claimed and rm-1439's double-claimed by fleet fog bands (bad1a64e mints 1440–1443, 50db6611 mints 1444, 0c44e399 re-mints 1440 — observed at prioritize). Row identity = **title + run id; reconcile by title, never numeral**.
- **Title-twin to fold:** sibling run d130035f's unlanded band mints an SC2001 `ci.yml:192` cure (rm-1504 by its census) — fold by title, keep one row.
- **Stage list at commit:** the 5 batch files + ROADMAP.md + this record; then flip rm-1437/rm-1438 to done (minted-implemented → done; done-flips reserved to the commit gate per the rm-012 convention).

## 5. Reusable lessons / prevention rules

- **L1 — durable channel is the typed envelope only.** Spool `-scratch/`, `/tmp`, and `~/.cache` mirrors were all swept within ~6h. Phase summaries must carry the content (census numerals, titles, md5s) needed to rebuild their artifacts — that is exactly what made this compound's rebuild possible.
- **L2 — dead-attempt triage.** A lifecycle-only event log (≤6 events, no typed artifact) + clean tree = redo from scratch and disclose; do not hunt for phantom work. Third instantiation this run.
- **L3 — verify claims at stewardship before encoding acceptance.** The npm "exit-code coercion" claim was half-wrong; re-reading the cited lines produced the corrected signal-path gap the batch actually fixed.
- **L4 — CHANGELOG structure gate belongs at implement time.** The batch introduced a duplicate `### Fixed` under `[Unreleased]` (#18407 class); full validation had to stop and merge it (digest proven stable across the fix).
- **L5 — pristine-base A/B for standing-red attribution.** `git archive <base>` + rerun proves a red is base-owned before blaming the delta; check-runs on a sibling landing corroborate.
- **L6 — run every CI lane locally or disclose the gap.** Earlier fleet full-tests never fetched actionlint; the SC2001 standing red was invisible until this run fetched it (into a scratch bin dir — never CWD, the binary-in-worktree trap).

## 6. Next-cycle starters

1. **rm-1439 variant-axis snapshot coverage** — declined this cycle with reason (needs a snapshot refresh landing on a churned pricing seam; pricing.rs dirty in sibling batches); gated by rm-176's delta-report demand.
2. **SC2001 cure** at `ci.yml:192` (echo-pipe class; fix-pattern precedent in-tree from the pipefail/flake work; title-twin of sibling rm-1504).
3. **rm-051 ref-pin cure** — `check-install-runtime.sh` test D expects `^ref=v0.9.0$` while `install.sh` is at v0.10.1 (stale test, one-line fix).
4. **rustls lock bump** — RUSTSEC-2026-0346..0349, sibling-owned; re-verify advisory DB mtime before attributing.
5. **rm-1373 breadth lead** — ccusage parity, 8 lanes missing; watch release PR #1885 tagging (fix(opencode) #1883 k2p6).
6. Banner-noted rides to union at integration: fog rows rm-1432 / rm-1427 / rm-1425 / rm-1424.

*compound attempt 7676255ed34140138c6d786f74119c08 · 2026-10-15 · no tests executed at this phase*
