# Cycle 2 compound record — run a1f6c1f8d706438ebffd6537702516db

repository-maintenance:88313799a395486ba2621e86ad168e00 · cycle 2 · compound attempt fede811cc4d440f6a6dad10111b95414 · 2026-10-15T00:5xZ

Worktree: run-a1f6c1f8d706 @ base 794b6e5 (porcelain 0 at assess; cycle added exactly: M crates/agenttrace-cli/src/main.rs, ?? scripts/ci/check-battery-adjudicate.sh, M ROADMAP.md (this band), ?? this record). Wall at compound: origin/master 8bc0587 (ROADMAP.md md5 1d59addaa633d721268bc6c222cdf03d, 392 def rows, def-max rm-1373) — frozen since 10-14 14:0xZ across every fetch of this run.

## Cycle chain (evidence-dense, one line each)

- **assess 41fa46ac** @794b6e5: battery green except ONE workspace red later adjudicated environmental (pi_cache_write_1h_blocks_disclosed, 11/11 green isolated); 14 findings F1–F14 incl. mode-word bail PoC and us-gov twin $4.95/$5.40 correctness. Prior assess attempt 7c3caf73 reaped with zero durable work (session_reap_failed 'No module named openai') — redone from scratch, declaration recorded.
- **research 57e68931**: five first-hand deltas D1–D5 (codex alpha.11 + daemon-updater mains; claude-code 2.1.299 flip — idle compaction preempts 1h cache, $.session.identity(), append-queue door, 2.1.296 Sonnet 5.5 cache-read $0.20→$0.10/M; models.dev third same-day md5 +16 models/+1 provider with 5-5 rate cells; codex 7-DB state family 6/7 live on host; hermes state.db 4.19 GB). All watch legs untripped (OTel tag, gemini 0.65 stable, amp 401, upstream fork 23rd zero-delta).
- **roadmap a8a7dc77**: band on wall 8bc0587 — riders rm-992/rm-715/rm-1053/rm-535 + mints adjudicator/mode-word; patch verified (apply --check, patch -F3, compositions with both in-flight sibling bands clean). Prior roadmap attempt 872cfdac reaped (provider reap, 6 event lines, zero artifacts) — redone from scratch. Scratch later swept <6h; band content rebuilt in THIS compound band from the typed artifact (forensics below).
- **prioritize 17ae7020**: selected batch 'assess-truth closeout' (adjudicator LEAD 42.0 + mode-word 34.0); found the rm-1505/rm-1506 same-minute numeral collision (51fff5de first-lander, title-disjoint subjects) — renumber disposition recorded.
- **stewardship 09c690d1**: typed stewardship_request; surfaces pinned at wall (main.rs:3037-3061 bail :3059, call :1961, flag cluster :201+, unit matrix :5089-5105); must_remain_separate matrix (5 pairs) incl. new-script ⊥ ci.yml and ⊥ check-install-runtime.sh.
- **implement 3a02719b (retry of 7a46192f after a fold rejection on attestation, NOT a code defect)**: batch built and green — mode_word_hint + bail arm + red-first pin (pre-fix failure message byte-matched the assess PoC); adjudicator script with typed verdicts. Two self-inflicted bugs found and fixed live: cargo `--exact` must follow `--` (arg-parse errors masquerade as defect verdicts — run_attempt swallows stderr by design); environmental-requires-ALL-green semantics (fail-once-then-pass is flaky by definition).
- **targeted_tests 0f86c43d**: tree byte-identical (diff md5 2a85a189…, script 199ffe71…); dispatch digest re-derived by the engine's own validation_policy and matched VERBATIM; bin suite 110/0; engine envelope (PhaseResult, '').
- **full_tests 4a144368**: ci.yml lanes verbatim (full_command empty → workflow authority): fmt rc0, clippy 3pkgs --all-targets -D rc0, workspace tests **61 targets 1090/0 rc0** (the assess environmental red did NOT recur — consistent with its adjudication), MSRV +1.89 rc0, 9 gates + ruby + npm 4/4 + token_conformance 39/0 + binary conformance (13+1xfail) + release build + 5 smoke gates + fixture smoke rc0; release-bin PoC of the hint. Standing reds (pre-existing at base, delta-untouched, disclosed): check-install-runtime test D, check-plugin-version v0.0.4. Not runnable locally: actionlint (no binary; .github/ zero-diff), expect-based TUI smoke.

## Compounded into the repository (this turn)

ROADMAP.md in-tree band (+31/−0 pure insertion — 27 band lines + 4 banner-correction lines; rows → 385, def-max rm-1508, postimage md5 c9a06762…, post-correction):
- riders rm-176 / rm-715 / rm-992 / rm-535 (research evidence appends, exact content from the roadmap-phase typed artifact);
- mints **rm-1507** (adjudicator) and **rm-1508** (mode-word hint) — MINTED-IMPLEMENTED with EXECUTED+VALIDATED bullets; done-flips pend review/commit (rm-012);
- compound banner with: wall/wall-md5 at build, numeral-renumber forensics (1505/1506 → 1507/1508 vs 51fff5de first-landed band), reaped-scratch rebuild disclosure, and the rm-1053 rider's wall-side disposition (row absent from this base tree — content preserved here: us-gov twin correct both trees; cc 2.1.299 halves Sonnet 5.5 cache-read to $0.10/M; Haiku 5.5 adaptive-thinking token-count fix).

## Durable lessons (prevention rules)

1. **Cargo filter flags**: `--exact`/`--ignored`/`--include-ignored` are libtest flags — they belong AFTER the `--` separator. Before `--`, cargo rejects them and the rejection is indistinguishable from a test failure if stderr is swallowed. (Fixed in the shipped adjudicator; keep for future harness scripts.)
2. **Adjudication semantics**: "environmental" must mean ALL isolated attempts green; a fail-once-then-pass sequence is FLAKY (mixed), never environmental. Bank flaky as disclosed, never as green.
3. **Same-minute numeral races**: two roadmap bands can census the same next-free numeral minutes apart on a frozen wall. First-lander keeps the numerals; the later band renumbers at the landing gate and cites by TITLE everywhere in between. Cite-by-title made this collision a non-event.
4. **Reaped-attempt forensics**: before redoing a phase, read the event log lines (a 1-line session_reap_failed with zero scratch = nothing adoptable); after landing a spool band, mirror anything the <6h sweep could erase — typed artifacts survive, scratch does not (this cycle rebuilt its band from the typed JSON).
5. **Validation-digest hygiene**: in a validation-only turn, declare the dispatch digest VERBATIM only after re-deriving it with the engine's own `validation_policy.validation_digest(base, worktree)`; also run `spool_outcomes.validate_phase_result` on the actual envelope before writing it. Both are one-liners and caught a digest typo this cycle (full_tests artifact initially had d773f vs 723f — corrected before the fold).

## Candidates / context for the next cycle

- rm-992 urgency UP: codex 0.163 stable cut expected any hour (alpha.8→.11 in ~38h) — next cycle's assess should re-fetch releases FIRST and pin the fixture arm to the shipped schema.
- models.dev snapshot drift family (rm-176/rm-1020): bundled snapshot 2026-10-08 vs three same-day upstream md5s; the 5-5 rate cells corroborate the us-gov twin rate mechanism (rm-1053 wall-side rider).
- Standing reds to close or keep disclosed: check-install-runtime test D (ref-pin family), check-plugin-version v0.0.4 at this base — the failing arm is per-tag CHANGELOG sections, not the version chain (manifest AND CHANGELOG head are both v0.10.1 at base, wall-identical): 49 merged pre-v0.4.0 tags (v0.0.4 + the v0.3.x family) carry neither a `## v` section nor a no-changelog-section marker at base (base has 8 markers, all v0.7.2–v0.8.0), while the WALL already carries 58 markers incl. v0.0.4 + every v0.3.x — healed wall-side; the red is base-tree-only and dissolves at the gate's wall rebase. (Mechanism corrected by attempt 8d59bf4e; the first draft attributed it to a plugin.json version delta.)
- The adjudicator is now the standing-red ritual: feed it any single-test battery red instead of hand-adjudicating (`scripts/ci/check-battery-adjudicate.sh -p <pkg> -t <target> -n <test> --runs 3 --json`).
- Commit-gate duties for THIS cycle (recorded, not executed here): stage exactly the 4 paths (never -A); land the ROADMAP band against the LIVE wall newest-first by content-hunk with mints before the managed footer; verify 51fff5de's band disposition first; CHANGELOG entries for both items (check-changelog-structure gate at landing); flip rm-1507/rm-1508 done after review.

## Verification (no tests executed this turn — PROHIBITED in compound)

- `git diff --numstat ROADMAP.md` → 31/0 (27 pre-correction + 4 banner-correction lines); postimage md5 `c9a06762af0173445bfa48f7911022b9` (post-correction; pre-correction 2b2929860113578555b738c9f91b0791); `grep -c '^- id: \`rm-' ROADMAP.md` → 385; def-max rm-1508; duplicate-id assert in builder, re-verified empty this attempt.
- Rider placement: `grep -n 'rider (2026-10-15, run a1f6c1f8' ROADMAP.md` → 6 hits at the expected blocks (rm-176 :566, rm-715 :1530, rm-535 :1803, rm-992 :2322, mints :2415/:2421).
- Consumed evidence only: phase artifacts 41fa46ac / 57e68931 / a8a7dc77 / 17ae7020 / 09c690d1 / 3a02719b / 0f86c43d / 4a144368 (typed JSONs + full_tests logs).

## Adoption addendum — attempt 8d59bf4e (2026-10-15T03Z)

The first compound attempt (fede811c) was truncated by a provider output limit mid-turn; the engine logged "banking failed truthfully" and no typed artifact was written — but the filesystem work above was complete and durable. THIS attempt adopted that work after first-hand verification, then corrected two factual slips (both disclosed inline where they occurred).

Adoption verification legs (all first-hand, read-only; NO tests executed — compound prohibition honored):

- Identity: events/fede811c….jsonl carries run a1f6c1f8 / phase compound / attempt fede811c, truncated at 12:06:24Z-equivalent clock with `phase_result_repair_failed` ×2 then reap; the typed JSON was never written → adoption of WORK, not of an envelope.
- Tree ≡ scratch: `git diff HEAD -- ROADMAP.md` ≡ scratch `ROADMAP.band.patch` (md5 c9635671… both); record doc tree copy ≡ scratch copy (md5 c01841b7… both).
- Batch non-interference: main.rs diff md5 2a85a189… and scripts/ci/check-battery-adjudicate.sh md5 199ffe71… byte-identical to the implement phase — compound touched no executable surface (both .md-only edits).
- Band structure re-verified: +27/−0; rider/mint lines at :566/:1530/:1803/:2322/:2415/:2421; 385 def rows post-band; duplicate-id census empty; rm-1053 confirmed absent from the base tree (wall-only rider, carried above).
- Rebuilt-band provenance: the reaped roadmap scratch is confirmed gone (`a8a7dc77…-scratch`: No such file or directory) while the typed artifact a8a7dc77….json survives (12,367B) and contains the band content (rm-1505/rm-1506 first-mints ×7/×10, rm-1053 rider ×4, models.dev/codex anchors) — the "rebuilt from the typed artifact" disclosure holds. (Caveat: assess 41fa46ac typed JSON + scratch are BOTH swept; its evidence survives only in the engine's work-order chain + events/41fa46ac….jsonl.)
- "bin suite 110/0" re-verified from the targeted_tests log: the agenttrace bin target ran 110 tests; package 11 targets / 236 passed / 0 failed.

Corrections made this attempt (minimal-touch, both cited at their sites):

1. Compound-banner base census fixed: 385/rm-1361 → 383 rows / def-max rm-1369 (banner carries the correction inline with the fresh leg list; postimage md5 moves 2b292986… → `c9a06762af0173445bfa48f7911022b9`).
2. Plugin-version standing-red mechanism corrected (Candidates section): CHANGELOG per-tag markers, not a plugin.json version delta.

Fog at THIS emission (fresher than the 00:48Z census the banner cites): the WALL MOVED — origin/master e06cd2b, ROADMAP.md md5 3ab2cb85, 2595 lines, 397 def rows, def-max rm-1502 (worktree now 27 behind). In-flight unlanded sibling claims extend past rm-1508 (fleet fog through rm-1515; one sibling gate plans rm-1503→rm-1507 — same numeral as our rm-1507, title-disjoint). Numerals STAY rm-1507/rm-1508: title-unique and already fog-registered; the COMMIT GATE must re-census against the live wall at fold time and renumber ours if any same-numeral band landed first (house rule: fold BY TITLE, never by numeral).
