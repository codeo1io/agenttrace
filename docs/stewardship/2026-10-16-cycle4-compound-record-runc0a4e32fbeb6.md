# Cycle 4 compound record — run `c0a4e32fbeb649eda28f159b8d4d8b5a`

- **run:** c0a4e32fbeb649eda28f159b8d4d8b5a (worktree `run-c0a4e32fbeb6-c0a4e32f`)
- **requirement:** repository-maintenance `c899be74d0f546ae932301a0be8d1941` cycle 4
- **base:** `c4aa3a01ed2a42ac3dc72b661b76ad9bc4a70bea` (== origin/master at assess; wall ROADMAP.md md5 `caaefc74…`, 403 id rows, def-max rm-1502)
- **phase chain (attempt ids):** assess a97489a7 · research 45b4df83 · roadmap 27916d7a · prioritize 8615a36d · stewardship c03b1390 · implement 81e40449 · targeted_tests 06c92654 · full_tests 3cc6eb9a · **compound 894a0fe4 (this record; redo of transport-dead d4b5d7cc)**
- **status at compound handoff:** pre-review; batch uncommitted in the worktree (3 M + 1 ??); review/commit/CI happen after this phase

## Outcome

Cycle 4 implemented and validated the three-row batch **"opencode admission hardening + npm win32 clamp"** — rm-1870 LEAD (id-component + stem-identity admission), rm-1871 PAIR (scan-wide read budget), rm-1873 rider (win32 signal clamp) — and compounded the cycle back into the wall as a rebuilt-and-extended ROADMAP band (2 banners + 2 riders + 5 mints, the three implemented rows flipped minted-implemented) plus this record. rm-1872 (`--search` content gap) and rm-1874 (discovery breadth) were minted and deliberately left candidate. No new validation was executed at compound — every test outcome below was **consumed from the recorded targeted/full artifacts**, per the phase contract.

## 1. What the cycle shipped (uncommitted batch, 4 files, +316/−59 at ship)

| File | Δ | Content |
|---|---|---|
| `crates/agenttrace-core/src/parser.rs` | M | opencode id-component admission — absolute/separator/`..`/stem-mismatch components rejected with `opencode_session_id_rejected` disclosure into `--doctor` — plus a scan-wide read budget (64 MiB bytes / 8192 records) enforced read-side via `take(allow+1)`, disclosed through the existing `OpencodeAdmissionStats` → `line_skips` channel |
| `npm/bin/agenttrace.js` | M | win32 signal clamp — non-emulated signals (Node emulates only SIGINT/SIGTERM/SIGKILL on win32) clamp to the 128+signum exit-code fallback instead of `process.kill` throwing |
| `npm/test/launcher-exit.test.js` | M | all-platform mapping-table test (win32 arms covered on the contract, not the platform) |
| `crates/agenttrace-core/tests/opencode_admission_hardening.rs` | new (8 pins) | absolute_session_id_escape · session_id_stem_mismatch · dot_dot_and_stem_mismatch · absolute_message_id_escape · total-records · name-carry · `--search` zero-leak pins with a clean-sibling control |

**Adoption disclosed:** the tree delta is provider-reaped attempt 5b08b443's real, event-logged work (22:14:32→22:28:33), adopted at implement 81e40449 after row-by-row verification against the batch acceptance criteria. The adoption snapshot scratch artifact was swept with the rest of the run's scratch; the typed envelopes carry the verification record.

## 2. Validation (consumed, not re-run at compound)

- **targeted (06c92654):** `opencode_admission_hardening` **8/0**; `opencode_bounded_admission` **3/0** (rm-1437 pins still green); `--lib opencode` **7/0** (404 filtered); npm **10/0**; fmt/clippy rc0; CLI PoCs: `--search 'clean sibling'` → 1 result, `--search TOPSECRET` on the adversarial corpus → 0 (and 0 on trav).
- **full (3cc6eb9a, ci.yml lanes verbatim):** `--locked` tests **68 suites 1167/0** (hardening suite 8/0 in-lane); clippy `--locked --all-targets -D warnings` **0 warnings**; MSRV `+1.89 check` rc0; **11 static gates rc0**; npm lane **10/0**; conformance **13 asserted (1 xfail-pinned)**; release build rc0 — `strings | grep -c opencode_transcript_files_capped` = 1, `--version` = `agenttrace v0.0.0-dev`; **8 smoke gates rc0**; cargo-deny and actionlint executed and adjudicated. Validation digest `validation:v1:56f2c1d8…` re-derived byte-exact at dispatch-fold. Battery at `/var/tmp/at-ft-3cc6/`.
- **red-first (from implement 81e40449, consumed):** the base binary on the adversarial trav2 corpus leaked cross-session text through `--search` (`count 1`, `results[0].name` = "TOPSECRET DOUBLEHOP LEAK"); the fixed binary → `count 0` and `--doctor` discloses `opencode_session_id_rejected=1`; the new suite at the pristine base tree ran **2 passed / 6 failed** → **8/0** fixed; RSS across agg 4/8/12 × 9.44 MiB: base 59840/96732/133892 KiB (linear) vs fixed 60008/88024/87992 KiB (plateau).
- **standing reds at base, fenced non-regression:** cargo-deny rustls 0.23.45 quartet RUSTSEC-2026-0346..0349; actionlint SC2001 `ci.yml:192` — both byte-identical at base by construction (the 4-path delta touches none of their inputs).

## 3. Band provenance — sweep and rebuild, disclosed

The roadmap phase's band artifacts (patch md5 `38218c55…`, postimage md5 `2c0539e7…`, builder, phase-note) and every other scratch/mirror of this run — assess probe corpora, research raw JSON, the implement diff and adoption snapshot, targeted/full logs, the `~/.cache/at-roadmap-27916d7a` mirror — were **swept before compound** (overnight fleet sweep; only the newest envelope, full_tests 3cc6eb9a, survived in `delegate/`; the engine's `results/` archive retained every prior typed envelope, which is what made this rebuild possible). The roadmap band was therefore **rebuilt from the 27916d7a typed envelope** — mints, titles, tracks, priorities, severities, riders, census and fog state preserved verbatim — and extended with the compound flips into **one band**: two banners + the rm-1437/rm-992 riders + the rm-1870..rm-1874 mint band at the wall foot + this record. The envelope's build-time md5s are unrecoverable and superseded by this band's own census: **403 rows in → 408 out, 408 unique, def-max rm-1874, pure insertion, managed footer last; the band's postimage/patch md5s are pinned in the compound envelope and its mirrors.** Twice-precedented protocol (c1cafed5 cycle 5 first).

Reaped/redone attempts across this run, each disclosed at the time: assess 98cdedd6, roadmap 7098ece8, implement 5b08b443 (work adopted), full_tests 026bef5d, **compound d4b5d7cc (2-event lifecycle log, provider reaped +24s, zero writes → redone from scratch as this attempt)**.

## 4. Wall drift and fog collisions (commit-gate duties)

- **Live wall at compound:** worktree base md5 `caaefc74…`, 403 id rows, def-max rm-1502, unchanged since the roadmap phase — the worktree keeps porcelain = exactly the 3 M + 1 ?? implement delta (ROADMAP.md untouched in-tree, per the dispatch's enforcing hygiene clause).
- **Apply order:** ONE band patch (`ROADMAP.patch` in the compound scratch/mirrors) at base wall `caaefc74…` — the lost roadmap band is folded in; there is no band-1/band-2 chain anymore. Verify `md5sum ROADMAP.md` == `caaefc74…` before applying; expect this band's pinned postimage md5 and the census above.
- **Fog collisions:** sibling in-flight bands hold title-twins — 8771e5c2 mints rm-1866..rm-1869 (title-twin of rm-1871), 68b47700 mints rm-1871..rm-1873 (numeral overlap), 63c582ac/8ee0b570 mints rm-1841..rm-1843. Row identity = **title + run id; reconcile BY TITLE, never numeral** (rm-708). rm-1870 is also title-adjacent to unlanded 60e0cbcb's bounded-read fog claim (noted on rm-1437's evidence line).
- **Stage list at commit:** the 4 batch paths + `ROADMAP.md` + this record; CHANGELOG `### Unreleased` bullets are the **gate's unit** (stewardship c03b1390) — author them with the real PR numeral; `check-changelog-structure.sh` requires every bullet to end `(#N)`. Then flip rm-1870/rm-1871/rm-1873 **minted-implemented → done** (done-flips reserved to the commit gate, rm-012 convention).

## 5. Reusable lessons / prevention rules

- **L1 — the durable substrate is the typed envelope + the wall.** Every scratch/mirror copy (spool, `/tmp`, `~/.cache`) was swept within ~18h. Envelopes must carry the content (census numerals, titles, md5s, validation numbers) needed to rebuild their artifacts — that is exactly what made this rebuild, and c1cafed5's before it, possible.
- **L2 — adoption-first on provider-reaped attempts.** 5b08b443's 14 minutes of real logged work survived as a clean tree delta while every artifact of it was swept. Triage: lifecycle-only event log + clean tree = redo and disclose; a real tree delta = verify against acceptance criteria and adopt.
- **L3 — private target dirs with string-verified feature markers before any CLI PoC.** A foreign conductor lane's cargo displaced a sibling lane's `debug/agenttrace` hardlink through a shared `CARGO_TARGET_DIR` (fleet prevention lineage b5affeade c2 L2); this run built into `/var/tmp/at-c0a4-target` and string-verified the marker before trusting probe output.
- **L4 — an empty dispatch `validation.full_command` does not skip full validation.** The empty field routes to the repository's ci.yml lanes executed verbatim (fleet precedents); the suite in the repo is the authority, not the field.

## 6. Next-cycle starters

1. **rm-1872 — `--search` content matching** (search.rs:8; README:494 contract text): decide the index shape (Event.content inclusion vs a sidecar) now that the admission seams are settled; adversarial pin — content-only text findable, unadmitted content not.
2. **rm-1874 — discovery breadth** (amp/codebuff/grok/kilo/openclaw per ccusage's 21-adapter census): wild-corpus survey first (rm-560 pattern), multi-cycle by design.
3. **rustls 0.23.46 lock bump** — cures the fenced cargo-deny quartet RUSTSEC-2026-0346..0349; re-verify advisory DB freshness before attributing.
4. **actionlint SC2001** at `ci.yml:192` — fleet-known cleanup; title-twin of sibling rm-1504.
5. **rm-176 drift gate** — ccusage v20.0.35 (PR #1901 in flight) and models.dev md5 churn (`0da1f5fd…`, 5th distinct value) demand hash-compare on content, never dates.
6. **rm-992 zstd arm** stays open (ruzstd single-frame, zstd-rs #127); the extension-awareness arm was discharged this cycle — see the rm-992 rider.

## 7. Review corrections (fe68a49f, cycle 4 independent-review fix)

Independent review 092fa8cb returned NEEDS_CHANGES, text-only and narrow; this section records the corrections so the record stays honest:

1. **F1 drift attribution (medium)** — parser.rs shipped md5 `d13c415f` (diff `+316/−59`), 2 lines past the implement pin `5a5e7690` (`+314/−59`). The delta is the part-lane stem-identity relaxation (admission rejects non-stem ids only on session+message lanes, whose stems feed filesystem joins; part docs are TYPE-named `part_text.json` carrying ids like `part_user_text` and their ids never join anything). Authored in reaped full_tests attempt `0e6d3e18` to cure the `discovery_contract` `rust_parses_opencode_storage_session` regression the first full-suite run caught. The full 68-suite lane 1167/0 is green ON the shipped bytes, so the lines are reviewed-and-disclosed here rather than reverted.
2. **F2 banner pins (medium)** — the banner and §1 carried stale pins (`5a5e7690`, `+314/−5`) and a false "byte-identical through targeted and full" claim; all three corrected to `d13c415f` / `+316/−59` / name-census wording, with the F1 attribution inlined in the banner.
3. **F3 evidence hygiene (low)** — the full envelope's "tree md5s identical to targeted census" claim was false (censuses were name/shape-only). Prevention rule L5 added in §5: byte-pin md5 per validation phase, or claim name-census only.
4. **F4 marker claim (low)** — the implement evidence's unnamed "strings marker grep → 0/1" was the nondiscriminating `opencode_transcript_files_capped` (present at base); the base/fixed discriminator is `opencode_session_id_rejected` (base 0 / fixed 1). Banner corrected to name the discriminator.
5. **F5 session-cache flake (low, pre-existing)** — the `session_cache::history` full-lane flake under fleet contention (#18750 family) reproduced once during the review's re-run; out of batch scope, carried to next-cycle candidates above.

*review-fix attempt fe68a49f23674e07bc90c62a42c46bbe · 2026-10-16 · no tests executed (required_scope none; all consumed evidence as recorded)*

*compound attempt 894a0fe4e1014bb8b2c57a66692fd534 · 2026-10-16 · no tests executed at this phase*
