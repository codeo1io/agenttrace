# Cycle 2 compound record — run `e40916d7ef9249459d2650236663720b`

- **run:** e40916d7ef9249459d2650236663720b (worktree `run-e40916d7ef92-e40916d7`)
- **requirement:** repository-maintenance `0799db9e8f0b4fe29fb1d8bd1f82f540` cycle 2
- **base:** `c4aa3a01ed2a42ac3dc72b661b76ad9bc4a70bea` (origin/master at dispatch `5b94464`; wall pre-band 403 id rows, def-max rm-1502; post-band 406, def-max rm-1869)
- **phase chain (attempt ids):** assess d5c29839 · research f6076d45 · roadmap 5c2d233e · prioritize 5abc5d3c · stewardship aaa18b02 · implement 755108730 · targeted_tests b4c75ef5 · full_tests b028c792 · **compound c4a7cee5 (this record)**
- **status at compound handoff:** pre-review; batch + band uncommitted in the worktree; review/commit/CI happen after this phase

## Outcome

Cycle 2 implemented and validated the three-row batch **"admission-truth-completion"** (LEAD rm-1867 + rm-1868 + rm-1869), flipped all three mints candidate→implemented on the wall with per-row EXECUTED + VALIDATED arms, and compounded the cycle into this record plus two prevention docs. No validation was executed at compound — every test outcome below was **consumed from the recorded targeted/full artifacts**, per the phase contract.

## 1. What the cycle shipped (uncommitted batch at compound handoff: 6 tracked files + 1 new, +309/−11 tracked + 40 new lines — numeral corrected at review-fix a7603402 from a drifted +304)

| File | Δ | Content |
|---|---|---|
| `crates/agenttrace-core/src/parser.rs` | +30/−1 | admission gate class-checks the same follow-symlinks metadata the read cap uses, **before any open** — `!meta.file_type().is_file()` skips FIFO/socket/device transcripts; `OpencodeAdmissionStats.non_regular_files` accumulates both lanes and folds into `line_skips` as `opencode_transcript_non_regular` beside the capped counters; stat-**failure** semantics unchanged (that disclosure stays fog rm-1866's ceded arm) |
| `crates/agenttrace-core/src/doctor.rs` | +76/−7 | `line_skips` rendering splits by key class: `admission_cap_counter()` moves `*_bytes_capped`/`*_files_capped`/`*_non_regular` out of "Dropped lines (parse loss)" into their own `Admission caps (skipped before read):` row; byte totals unit-aware via `format_admission_bytes()` (10 MiB corpus renders `10.0 MiB`, not 10486375); parse-loss row only when line-denominated keys remain; `--overview -f json` `data_health` keys unchanged |
| `crates/agenttrace-core/tests/opencode_bounded_admission.rs` | +131/−1 | 3 new pins: `non_regular_transcripts_are_skipped_and_disclosed` (unix-socket fixture, corpus-fingerprint equality), `fifo_transcript_cannot_hang_the_reader` (mkfifo + background writer holds the pipe open so a regressed guard fails **red** under the suite timeout, not hang), `admission_caps_render_in_their_own_doctor_row` |
| `npm/package.json` | +2/−2 | engines `>=18`→`>=20`; `scripts.test` glob (Node≥21-native) → explicit 4-file enumeration |
| `npm/test/test-discovery.test.js` | new, 40 lines | self-listing guard: every listed file exists, all 4 files covered, no glob metacharacter can return — zero-match green can never recur |
| `CHANGELOG.md` | +34 | three entries under `[Unreleased]` |
| `ROADMAP.md` | +36 | carried cycle-2 band (+31 at roadmap) + compound flips/banner (+5) |

README:97 carried "requires Node.js 18+" — this record's compound-time "zero Node claims (grep-proven at implement and targeted)" statement was FALSE (the greps ran cwd-relative from `npm/`, where `README.md` resolves to `npm/README.md`, which carries no Node claim); corrected at review-fix a7603402 (review F1): README synced to "Node.js 20+".

## 2. Validation (consumed, not re-run at compound)

- **targeted (b4c75ef5):** admission + doctor suites **11/0** (opencode_bounded_admission 6 incl. all 3 new pins, doctor_honesty_contract 2, doctor_demo_contract 3); core lib **411/0**; enumerated `node --test` **9/9** and `npm test` via the rewritten entry **9/9 rc0** on v22.22.0; `cargo fmt --check` rc0; clippy core `-D warnings` rc0.
- **full (b028c792):** `--locked --workspace` **67 suites / 1162 passed / 0 failed** — assess-phase baseline 1159 → delta is **exactly** the batch's 3 new tests, zero regressions; fmt rc0; clippy `--workspace --all-targets -D warnings` rc0; npm lane 9/9.
- **digest:** `validation:v1:56f2c1d80b5df3c7a82a61416f91666ea8d7caff45213f1c358a2f6a66abea97` stable dispatch→targeted→full (re-derived at both phases via `validation_policy`, `env -u PYTHONPATH`).
- **standing reds at base, all delta-untouched:** msrv clippy 1.89 version-lints; cargo-deny rustls 0.23.45 (RUSTSEC-2026-0346..0349); actionlint SC2001 `ci.yml:192`; check-install-runtime rm-051 ref-pin.
- **floor gap — CLOSED at review-fix a7603402:** floor-exact probe on official node v20.20.2 (nodejs.org dist): enumerated `node --test` 9/9 rc0 and `npm test` through the entry 9/9 rc0, zero experimental warnings. No CI node lane exists yet (optional hardening, starter #7).

## 3. Band provenance and the evidence sweep

The roadmap band was written **into the worktree tree** (uncommitted) and survived intact through implement, targeted, full and compound — md5 `9a3b2244c851c913be35c538aea18ad1` verified identical at every phase boundary. By compound time the reaper had swept **every scratch mirror AND the typed envelopes of the four read-only phases** (assess d5c29839, research f6076d45, prioritize 5abc5d3c, roadmap 5c2d233e); the surviving record is the run's dispatch work-order summaries, this in-tree band, and the implement/targeted/full envelopes + their scratch logs. Nothing required rebuilding — the band was the durable artifact (cycle-5's L1 lesson, now proven the hard way: the artifact that lives in-tree is the one that survives).

Four reaped/redone attempts this run, each disclosed at its phase: assess `a9c0ef4c` (4-event provider reap), roadmap `2c8c55dd` (15-event provider reap; mtime forensics proved no in-tree delta), targeted_tests `885a6b76` (44 s provider reap), compound `3e9d7132` (**new death mode: truncated by provider output limit, banking failed truthfully** — 12-event log, no artifact, no tree delta; redone from scratch as c4a7cee5). Plus implement's resolved stability incident (below, L7).

## 4. Wall drift and fog duties (commit gate)

- **Live wall at compound:** origin/master `5b94464` (fetch at prioritize, unchanged) — 403 id rows, def-max rm-1502; our worktree band carries 406 / rm-1869. Numerals rm-1867..rm-1869 are absent from the live wall; reconcile newest-landed-first.
- **Fog re-census at compound (2026-10-16T01:3xZ):** spool def-row ceiling is **rm-1900** (sibling e43883b3 postimage cf7cb6b0 mints rm-1899/rm-1900; ac7b5e36 band rm-1898). The band's mint footer "next free rm-1870" is **stale** — numerals 1870..1900 are all fog-claimed. Next mint needs a fresh census, starts ≥rm-1901 (`rm-1999` spool hits = known test fixture).
- **Title-twin to reconcile by title:** rm-1869's independent twin (6b28edba assess c9154c24 C4 @ d2a02af, unlanded at mint time). RC-1 antigravity stays ceded to fog rm-1864 (double-minted); F4 stays ceded to fog rm-1866.
- **Stage list at commit:** the 6 batch files + `npm/test/test-discovery.test.js` + `README.md` (review-fix F1) + `ROADMAP.md` + this record + the two solutions docs below (post-review-fix tree; the review-fix deltas ride the same stage list); then flip rm-1867/rm-1868/rm-1869 to done (done-flips reserved to the commit gate per the rm-012 convention).

## 5. Reusable lessons / prevention rules

- **L1 — the band is the durable artifact.** Scratch dirs, `/tmp`, `~/.cache` AND this time even read-only-phase typed envelopes were swept within hours. Only what lives in-tree (or in the implement/test envelopes) survives; phase summaries must carry enough content to rebuild.
- **L2 — dead-attempt triage, plus a new death mode.** Lifecycle-only event log + clean tree = redo from scratch and disclose (4 instances this run). New mode: provider **output-limit truncation** — bank the phase-result JSON before composing long prose.
- **L3 — premise corrections must be recorded, not silently absorbed.** RC-6's "hidden tier" premise was corrected at research (cap class renders; the hidden class is F4/rm-1866) and the correction is written into rm-1868's row, so the wall can't later "re-fix" a non-bug.
- **L4 — counters carry their denomination.** Every disclosure counter is byte-/count-/line-denominated; renderers branch by unit class, JSON consumers never sum mixed units (generalizes rm-1868).
- **L5 — a size gate does not admit file class.** Any admission gate built on `metadata().len()` still lets FIFOs/devices through (`len()==0` passes); class-check on the **same** metadata call, before open (generalizes rm-1867).
- **L6 — a green lane must prove a nonzero test count.** → `docs/solutions/process-issues/test-lanes-must-fail-when-discovery-finds-zero-tests.md` (this cycle's rm-1869, generalized).
- **L7 — frozen-mtime stale-view incidents: compilation is the authoritative reader.** → `docs/solutions/workflow-issues/frozen-mtime-stale-views-treat-compilation-as-the-authoritative-reader.md` (2nd fleet instance of the transient-revert hazard class).
- **L8 — baseline-delta test accounting.** Record the full-suite baseline before the batch and require the delta to be exactly the batch's enumerable new tests (1159→1162 = +3 = the 3 new pins). Cheapest regression proof there is.

## 6. Next-cycle starters

1. **RC-1 antigravity model wave (HIGH, ceded to fog rm-1864)** — re-verify the landed state first (both sibling selections were in flight), then port ccusage `fe761657`'s `1417 → gemini-4-argon` mapping + the ≥6 models.dev provider entries; corroborating PoCs already on record (zero argon keys in the bundled snapshot; lookup falls back unpriced).
2. **Assess F4 unreadable/invalid-JSON silence (fog rm-1866, in flight)** — chmod-000 and NOT-JSON transcripts both rc0 with `Messages: 0 user` and no dropped-lines tier in `-d`; verify the sibling landing before re-implementing.
3. **rm-018 rustls 0.23.46** — one-line lock bump; PR #22 carries it; advisory-DB mtime check before attributing.
4. **rm-176 pricing drift** — 1,461 absent keys (35.4% of live LiteLLM, widening); snapshot refresh under a never-drop union contract.
5. **rm-493 semconv v2 daily wave** — re-verify all 18 `gen_ai.*` attribute names before re-pinning at first tag.
6. **rm-382 / fog rm-1524 upstream.rs TMPDIR non-hermeticity** — proven again this cycle (ambient-TMPDIR assess run failed only at `upstream.rs:261`).
7. **rm-1869 CI node lane** — the floor-exact probe is DONE (review-fix a7603402, official node v20.20.2, 9/9 rc0 both invocation paths); what remains is a CI lane so the floor stays proven on every push (ci.yml currently has zero setup-node/npm matches).

## 7. Review fix (2026-10-16, attempt `a76034028ced40d28dccea1a2d12f71c`)

Independent review 6eb418b6 returned NEEDS_CHANGES with five findings; all five resolved:

- **F1 (high)** `README.md:97` synced `Node.js 18+` → `20+`; the false "zero Node claims anywhere — grep-proven" claims here (§1), in the ROADMAP compound banner and rm-1869's EXECUTED arm are corrected in place; README.md added to the §4 stage list.
- **F2 (medium)** `npm/test/test-discovery.test.js` guard upgraded from `listed.length >= 4` to readdir set-equality with `test/*.test.js` — a future un-enumerated test file now fails the lane on every Node version (the old glob auto-covered new files on Node ≥ 21).
- **F3 (medium)** `admission_counters_carry_raw_integers_into_the_json_lane` added to `opencode_bounded_admission.rs`: `data_health.line_skips` carries `opencode_transcript_bytes_capped` as a raw JSON number (serde_json wire pin), confidence degrades low, and the doctor's raw fold equals the json integer.
- **F4 (low)** floor-exact probe executed on official node v20.20.2: enumerated `node --test` 9/9 rc0 and `npm test` 9/9 rc0, zero experimental warnings; the EOL-based floor rationale replaced by the probe result (Node 20 past-EOL noted for the commit gate; raising to >=22 is a policy call, not an arm requirement).
- **F5 (low)** §1 numeral corrected +304 → +309 (compound-snapshot truth).

Review-fix validation (targeted, per dispatch validation block): cargo suites admission 7/0 + doctor_honesty 2/0 + doctor_demo 3/0, fmt rc0, clippy core `-D warnings` rc0, `npm test` 9/9 on v22.22.0 (post-F2 guard), floor-exact 9/9 on v20.20.2, changelog gate rc0; digest re-derived `validation:v1:56f2c1d8…` (testable-surface set unchanged).

*review-fix attempt a76034028ced40d28dccea1a2d12f71c · 2026-10-16 · targeted retest only (dispatch required_scope: none)*

*compound attempt c4a7cee50e6948b1941a0ab6c47f808d · 2026-10-16 · no tests executed at this phase*
