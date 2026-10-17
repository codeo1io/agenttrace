# Cycle 2 compound record — run 7517204088bd (repository-maintenance ba50ee5c)

- **Run**: 7517204088bd4e008bba3775e7d843fa · cycle 2 · base e130845 (porcelain 0 at assess dispatch)
- **Worktree**: `conductor-worktrees/agenttrace-80c75f65b7/run-7517204088bd-75172040`
- **Compound**: 2260c1eafcfd4ccba0ed9160161fe86c (2026-10-16), pre-review — consumes recorded
  pre-review cycle evidence only; no tests were re-run at compound (phase contract).
- **Prior compound attempts**: 24c54124 provider-reaped at ~44s (4-line lifecycle event log, no
  typed artifact, no scratch, no tree delta) — nothing durable, phase redone from scratch.
  ee5743b8 authored this entire band chain + this record and WROTE its typed envelope, then was
  session-reaped minutes later (events/ee5743b8: `session_reaped` exit_reason failed) — the
  engine recorded the attempt failed and re-dispatched; 2260c1e adopted its durable artifacts
  (chain of custody verified by md5), re-verified every guard, and regenerated the chain with
  corrected attribution (stage-1 byte-identical; stage-2 differs only in the two attribution lines).
- **Scope identity** (campaign ba50ee5c cycle 1): "nothing silently swallowed" — rm-947 + rm-921.

## 1. Cycle ledger

| phase | attempt | durable outcome |
|---|---|---|
| assess | 635ecf6a | static pass clean (8 sanitizer modules show prior hardening only); live hostile-corpus battery → 7 findings F1-F7; PoCs in `-scratch/corpus/` (since reaped; facts preserved in the envelope) |
| research | 548d4fb1 | 14 sources re-fetched, 33 raw artifacts mirrored durably at `/home/agent/.cache/at-research-548d/raw/`; RC-1 LEAD = rustls 0.23.46 published, all four RUSTSEC-2026-0346..0349 carry `fixed:0.23.46` |
| roadmap | b548b491 | spool band: 1 banner + 8 riders, ZERO mints/flips, +9/-0 pure insert; scratch since REAPED — regenerated at compound (below) |
| prioritize | d9cdb742 | selected batch: rm-451 READ_CAP arm (the last unbounded read lane); fog ledger adjudicates every other candidate |
| stewardship | d4d143d9 | typed batch request; all surfaces verified on-disk; must-remain-separate pairs pinned |
| implement | b73f6e34 | rm-451 as UNCOMMITTED 8-file delta (+426/-25) — adopted a reaped attempt's authored-but-never-compiled corpse (9f806027), verified hunk-by-hunk, then completed it |
| targeted_tests | cf4d3bab | fresh private target: core lib 413/0 + 5 contract suites 120/0 + entrypoints 60/0 + clippy/fmt rc0; dispatch digest declared verbatim |
| full_tests | ceacdb55 | fresh private target: workspace 66 suites 1158/0 + release build + conformance 13 + 15 gates + npm + MSRV rc0; 3 standing reds A/B-proven pre-existing at pristine base |
| compound | 2260c1e (retry of reaped ee5743b8) | two-stage spool band chain regenerated + verified (26/26 guards PASS); this record updated; rm-451 flipped candidate→implemented; zero mints |

Dead attempts this cycle (all provider-side): 544355c1 (assess, 61 heartbeats),
5d0fb445 (roadmap, 4-line log), 075695e5 (implement, 24s), 3f318292 (full_tests, 4-line log),
24c54124 (compound, 4-line log) — these five left nothing durable. TWO exceptions carried real
work across the reap: 9f806027 (implement — its worktree delta WAS real and was adopted at
implement) and ee5743b8 (compound — its typed envelope + full scratch kit + this record were
durable and were adopted by the 2260c1e retry; the reap landed AFTER the envelope write, so only
the turn's completion was lost, not the work).

## 2. What shipped (pre-review): rm-451 "named-file READ_CAP"

`FILE_READ_CAP_BYTES = 256MiB` on the bare `std::fs::read` in `parse_file` (parser.rs:29) — the
claude/codex/pi named-file + positional lane, closing the unbounded members of the cap family
present at THIS base (stdin 256MiB rm-503 · zstd 64MiB · statusline 10MiB rm-898; the opencode
10MiB rm-1437 cap is a SIBLING-lineage arm — ac85f7b, not an ancestor of base e130845 — so
`read_opencode_records` (parser.rs:6593) stays unbounded HERE, a disclosed residual).
Independent review (2026-10-17, 1f153416, F1) also closed the cline task-dir arm, which had
bypassed the new cap via `read_json_value`'s bare `read_to_string`: member reads now share
`read_session_file_bounded` with the typed `ReadCapRefused` marker propagated through
`parse_file` (no parse-around; sparse-fixture unit pin + CLI pins on both lanes).

- `AGENTTRACE_FILE_CAP_BYTES` best-effort override (None/garbage/<=0 → default), pure resolver.
- Typed `ReadCapRefused{path,size,cap}` marker; metadata fast-refusal; `take(cap+1)` TOCTOU guard;
  error-context wording preserved → zero behavior change for normal files.
- `LoadReport.read_cap_refused` (parse_failures subclass — never double-counted) and
  `DoctorDirReport.read_cap_refused` in doctor text + JSON/`-o` lanes; stderr advisory names the
  read-cap share + knob; README section; CHANGELOG entry.
- 5 new tests (3 entrypoints, 2 parser units). No schema bump — no versioned schema surface exists
  (doctor JSON has no `schema_version`; `LoadReport` is never serialized).

**Peak-RSS acceptance arm** (python `os.wait4` ru_maxrss, 300MiB NUL corpus, debug binary):

| scenario | pre (assess F2) | post |
|---|---|---|
| positional, default cap | 613 MiB, rc1 "unsupported session format" | **11.5 MiB**, rc1 named knob-bearing refusal |
| positional, knob 320 MiB | — | 613.7 MiB admitted (reproduces pre — the bound, not the file kind, gates memory) |
| `-d` scan, 1 good + 1 huge | — | rc0 at 24.0 MiB with loud advisory |
| stdin (rm-503 lane) | unchanged | unchanged (named refusal) |

**Recorded validation** (consumed as recorded, not re-run):
targeted cf4d3bab — core lib 413/0 (both new unit tests by name), 5 contract suites 120/0
(discovery_contract, doctor_honesty_contract, doctor_demo_contract, parse_torn_tail_contract,
parser_property_invariants), entrypoints 60/0 (3 new CLI tests by name), clippy `-D` rc0, fmt rc0;
digest `validation:v1:04bee40f02cae6e46d476a8abc59aa409ec7cbc2c3c760d678e452f55cb37ef5` declared
verbatim (that turn changed zero executable surfaces; dispatch digest re-derived live == declared).
full ceacdb55 — workspace 66 suites 1158/0 (rm-451 tests present by name), fmt/clippy/release
rc0, token_conformance 2/0, conformance 13 cases, 15 gate scripts rc0 incl. scoped-HOME binary
gates, npm test rc0, MSRV +1.89 rc0. Standing reds A/B-proven PRE-EXISTING at pristine base
(git-archive base-ab re-runs): cargo-deny rustls x4 (cure = 0.23.46 bump), actionlint SC2001
ci.yml:192, check-install-runtime test D — none share a file with the delta.
**Review fix e9f50573 (completing reaped bcf16691, review 1f153416)**: affected battery fresh at
/var/tmp/at-fix-e9f5 — core lib **414/0** (incl. the new `cline_arm_over_cap_refuses_loudly_both_paths`
unit), 5 contract suites **120/0**, entrypoints **61/0** (incl. the new `cline_task_member_over_cap_*`
CLI pin), clippy trio `-D` rc0 (after closing the 9 doc-indent lints bcf16691 died with — the new
parser.rs doc block's line-leading plus-sign), fmt rc0; live 300MiB cline-member RSS probe
(os.wait4, debug binary): positional default-cap **12.6MiB** rc1 named refusal / knob 320MiB
**614.4MiB** admitted (the bound, not the file kind, gates memory) / `--sessions -d` **23.0MiB**
rc0 with the 1-of-2 advisory — vs the review's 313/540MiB pre-fix peaks. Digest re-derived live
== `validation:v1:04bee40f…` (the fix touches .rs/.md only; the digest covers
executable-classified surfaces, which are unchanged).

**Implement-phase adoption forensics** (reusable): no PhaseResult existed to adopt; spool
events/9f806027 (17 lines, reaped 19:32:13 on 2026-10-15, no artifact/scratch) matched the
worktree delta's file mtimes (18:42→19:31) — the tree held a reaped attempt's authored-but-never-
compiled corpse. Adopted hunk-by-hunk against the row acceptance, then completed: fixed the
corpse's doctor cache-hit laziness regression (parse_file hoisted above cached_session — warm
--doctor would re-parse everything; restored miss-lane-only, warm run "1 of 2 re-parsed"), import
ordering, missing CHANGELOG Unreleased/Added entry.

## 3. Roadmap status accounting (this compound)

Band chain delivered spool-side (the wall is never edited by delegates on this lane); the commit
gate lands it:

- **base** `613186493ac71a84c0f70384f20cc416` (= the roadmap phase's recorded seam)
  → **stage-1** `intermediate.md` `1cfbda618b00cd73526ee3b410d862b3` (regenerated roadmap band:
  banner + 8 riders, +9/-0 — byte-identical across BOTH compound attempts: the cross-attempt
  determinism proof, guarded)
  → **stage-2** `ROADMAP.final.md` `b3b98fbd7e0a1766c82d92394c34ffdc` (compound banner +
  rm-451 VALIDATED rider + status flip, +3/-1; ee5743b8's pre-retry final was
  `69925fb3e8846b3f0763b238ac0cc1c5` — the two differ ONLY in the two attribution lines,
  guarded). Authoritative chain + patches: `delegate/2260c1eafcfd4ccba0ed9160161fe86c-scratch/`.
  **SUPERSEDED at review-fix bcf16691 (2026-10-17, review 1f153416 F2/F3)**: both 2260c1ea
  postimage pins were first reproduced byte-exact via transcript-replay of the swept builder,
  then re-issued as the amended chain — stage-1 `b52b133659b2c4b8e1635a51fd0ee87c` (rm-451
  rider F2 wording only) → stage-2 `f77be96d67e2db1a73a469421dcade76` (banner F3 ghost strike
  + review-fix clause + VALIDATED-rider clause; exactly 3 lines vs the replay final, guarded).
  **SUPERSEDED AGAIN at review-fix e9f50573 (completing reaped bcf16691)**: bcf16691 authored
  the code + filtered pins (unit 3/0 by name, CLI 1/0, fmt rc0) but was provider-reaped with
  clippy RED (9 doc-indent lints from the new parser.rs doc block) — its band claimed
  "revalidated full-scope at bcf16691", which was FALSE at its reap. e9f50573 adopted the
  corpse, closed the parser.rs:56 line-leading plus-sign doc-indent micro, revalidated the
  affected battery fresh, and TRUTH-AMENDED the chain: stage-1 stays byte-identical
  (`b52b1336…`); stage-2 becomes `8cd46dab3c88838a08ead2977681b7b2`, differing from
  bcf16691's final `f77be96d…` by exactly the 2 validation-attribution lines (guarded; no
  "full-scope" claim survives). The commit gate applies the e9f50573 chain:
  `delegate/e9f50573baf8404b937fe937c536e418-scratch/` (28/28 guards PASS there;
  bcf16691's scratch retained as the superseded provenance link).
- rm-451 flipped **candidate → implemented** with its dated VALIDATED rider (compound convention);
  **done reserved to the commit gate** (rm-012). Remaining multi-cycle arms enumerated on the row:
  streaming/reverse-scan DOM rework + the min(16, parallelism) discovery fan-out memory.
- ZERO mints at compound: 397 def rows in == out, 0 dups, def-max rm-1502, footer last; status
  192 in_progress / 156 candidate / 44→45 implemented / 4 done. Next mint ≥ rm-1864 under fresh
  census (roadmap envelope's ceiling adoption; rm-1864 itself withdrawn — fog rm-1522 title-twin).
- **Band regeneration disclosure**: the roadmap phase's spool band (postimage fe09331b…) was
  swept by the spool reaper before compound. Stage-1 is rebuilt content-faithfully from the
  surviving typed envelope `results/b548b491….json`; its insertion anchors reproduce the
  envelope's recorded coordinates exactly (riders @ 300/416/639/719/739/845/1616/2413, banner@16).
  fe09331b… is unrecoverable and must never be cited as the postimage.
- Patches are -U1-context (never -U0: zero-context hunks relocate silently under `git apply` —
  the fd610df5 lesson) and round-tripped in a throwaway git repo with BYTE comparison at both
  stages; `git apply --check` stage-1 passes at the live wall.

## 4. Reusable lessons / prevention rules (this cycle)

1. **Reaped-corpse adoption ladder.** On any dead attempt: event-log shape first (heartbeat-only
   vs 4-line lifecycle vs corpse-with-delta). A tree delta whose mtimes match a reaped attempt's
   window is CANDIDATE WORK, not drift — adopt hunk-by-hunk against the owning row's acceptance,
   then complete it (this cycle: 9f806027's corpse became rm-451 minus 3 defects). Complement
   (92ea7949 c4): a lifecycle-only log + FULL scratch kit + tree delta = salvage-adoption
   candidate; verify md5 anchors, round-trip, census, adopt+disclose; do not blind-redo.
2. **Warm-path laziness is a first-class acceptance arm for any read-path change.** Hoisting a
   read above a cached path silently re-parses the world on warm `--doctor`; the canary is the
   re-parse count ("2 of 2" → "1 of 2"). Check it before declaring a cap done.
3. **Private build targets must live under `/var/tmp`, never `/home/agent/.cache` or `/tmp`.**
   This cycle's full battery had TWO infra incidents cured by relocation: a sibling battery's
   `.cache` target vanished mid-run (disk sweep), and a determinism suite collided on shared
   `/tmp` (csv_export "session directory does not exist") — cure: per-battery `TMPDIR` subdir +
   `/var/tmp` target. Fleet corroboration: #18697 (CARGO_TARGET_DIR trap), the rc=101 root-FS
   trap cured at 92ea7949.
4. **Digest discipline for validation-only turns.** A turn that changes zero executable surfaces
   declares the dispatch digest VERBATIM after re-deriving it live (classifier quirk: Cargo.lock
   is digest-invisible — #18706). Declaring a fresh digest for an unchanged tree breaks the fold
   gate, not the other way round.
5. **Spool scratch is reaped aggressively; envelopes are the source of record.** Any band whose
   value is its bytes must carry its postimage md5 + census figures + insertion anchors IN THE
   ENVELOPE so a later phase can rebuild it content-faithfully (this compound did exactly that).
   Mirrors under `/home/agent/.cache/at-*` survive reaps — prefer them for raw evidence.
6. **Zero-context patches relocate silently.** Pure-insert bands still get -U1 context; round-trip
   with `cmp`, never exit-code alone (fd610df5 lesson, re-cited because this chain ships patches).

## 5. Concrete candidates for the next maintenance cycle

1. **LEAD — rustls 0.23.45 → 0.23.46** (rm-018 rider): cures the cargo-deny x4 standing red
   (RUSTSEC-2026-0346..0349 all `fixed:0.23.46`); `cargo update -p rustls`; ureq 3.4.2 is the
   sole edge. Fork PR #22 targets it — **merge, don't reimplement**. Cheapest verified lead.
2. **rm-933 joiners-widening arm** (rider): U+200C/U+200D into the non-directional-Cf
   enumeration; red-first od-proven fixture (`73 2d 7a 77 e2 80 8d 6a` raw in TSV+CSV name cells)
   + both-lane pin on the widened arm.
3. **Fog cessions to title-reconcile at integration** (if still unlanded, next assess
   re-adjudicates): rm-1516/rm-1518 (zstd per-frame loop, 1e4f6b95) + the parser.rs:50 vs
   CHANGELOG.md:52 zstd-origin drift riding it; rm-1522 (--inspect rank/overflow, 7288595f);
   1e4f6b95-rider/rm-700 (named-file cap family — now partially superseded by rm-451).
4. **discovery.rs:659 `opencode_session_fork_parent` uncapped `read_to_string`** (92ea7949 N5,
   un-minted): same family as rm-451, deliberately separate row; natural next cap arm.
5. **rm-451 residual arms** (multi-cycle, enumerated on the row): streaming/reverse-scan DOM
   rework; the min(16, parallelism) discovery fan-out memory (a per-worker cap × workers bound).
6. **Long-horizon**: pricing snapshot refresh + provenance stamps (%Y-%m-%d + content-hash,
   rm-176 rider; ~2026-10-19 predicted-red horizon, owned by the weekly automation — drill
   refreshes REVERTED, never landed); quota-truth enrichment via ccusage #1889 (rm-201 rider).

## 6. Commit-gate seams

1. `git apply roadmap.regen.patch` then `git apply ROADMAP.compound.patch` from
   `delegate/e9f50573baf8404b937fe937c536e418-scratch/` (truth-amended review-fix chain —
   stage-1 identical to bcf16691's, stage-2 `8cd46dab…`; supersedes the retained bcf16691
   scratch and the swept 2260c1ea/ee5743b8 scratch dirs; base md5 must match the wall at apply
   time — see its APPLY-SEAM.md for the full replay + truth-fix provenance).
2. Stage the 8-file implement delta + this record (untracked) + the band.
3. done-flip rm-451 (rm-012 convention). The CHANGELOG :9/:17 duplicate-bullet micro from the
   earlier seam is RETIRED as a stale-review ghost (the structure gate's live census sees 313
   entries, no duplicate section kinds; :9/:17 were rm-1036-era line numbers at base 82e23b9).
4. Fog title-reconcile by title, not numeral (rm-708 convention).

## 7. How to verify this record

- `md5sum ROADMAP.md` in the worktree == `613186493ac71a84c0f70384f20cc416` (wall untouched).
- `git status --porcelain` == the 8-file implement delta + this file (untracked).
- Spool scratch `delegate/e9f50573baf8404b937fe937c536e418-scratch/` (truth-amended review
  fix): `python3 build_compound_fix.py` → 28/28 guards PASS — bcf16691 chain pins intact
  (b52b1336…/f77be96d…) + stage-1 byte-identical to bcf16691's + divergence from bcf16691's
  final confined to the 2 validation-attribution truth lines + the inherited 2260c1ea
  replay-pin chain-of-custody (1cfbda61…/b3b98fbd… in /tmp/replay-bcf/) + byte round-trips +
  wall `git apply --check` rc0; throwaway round-trip repos under `/tmp/at-compound-e9f50/`;
  live RSS probe log at its `logs/rss-probe-cline.log`.
- Recorded outcomes: `results/cf4d3bab….json`, `results/ceacdb55….json`, and the review-fix
  envelope `results/e9f50573….json` (typed); raw research evidence at
  `/home/agent/.cache/at-research-548d/raw/` (33 artifacts).
