# Cycle 4 compound record — run 37101c744c524d0d95a651b2a29ac39b

- **Campaign**: repository-maintenance 1ee0282008e1410ebb6d4b6c541a8a33, cycle 4, repo `/work/projects/agenttrace`
- **Base**: cab6917 (verified 10 ahead of origin/master 3e90993 at assess/research; origin has since moved — this worktree is the batch's authority)
- **Compound attempt**: 90259256d4be495a852119eee6ce51cd (2026-10-15), prior attempt d266a8ae died infra-reaped @~168s with zero durable trail → redone from scratch (event log 4 lines: started → 7-progress → session_reaped failed/provider → completed failed; typed artifact absent; no scratch dir)
- **Scope guard**: pre-review compound — consumes ONLY pre-review cycle evidence; NO tests executed at compound (targeted 5bf8c6ee and full 0f3f263e outcomes are consumed as recorded results)

## Phase ledger

| phase | attempt | outcome |
|---|---|---|
| assess | 6fdbd38b | battery 61 suites 1094/0/0, clippy -D rc0, fmt rc0; static reds F1 (--search metadata-only) + F3; standing gate reds plugin-version + install-runtime |
| research | 5a6734fb (redo of reap-boot-death 0b5b7fb1) | live npm/models.dev/LiteLLM/GitHub census 2026-10-14T01:29-01:35Z; raw artifacts swept with /tmp, digest survives in envelope |
| roadmap | 06b5995d | mint rm-1415 + 6 riders, spool-side band (postimage md5 e8ea3f5f, 387 rows, round-trip byte-identical) — **scratch later REAPED pre-gate** |
| prioritize | ae20fc51 (redo of reaps 93c3602c @24s, 9fdb9735) | selected rm-1415, three ordered CUs, independent cold-build red repro |
| stewardship | 5f7cd666 | anchors re-verified live at cab6917, porcelain 0; request typed |
| implement | 37c02d90 (ADOPTED reaped 42048792's spec-complete worktree trail) | rm-1415 implemented uncommitted: 4 M + 2 ?? |
| targeted_tests | 5bf8c6ee (redo of reap 815c30f8 @118s, zero durable) | first-run green: contract 6/0, lib 382/0, demo+discovery 14/0+104/0, cli 235/0, fmt/clippy/changelog rc0 |
| full_tests | 0f3f263e | ci.yml lanes verbatim, 24 green lanes, trio 1100/0 (= 1094 + 6 new), MSRV 1.89 rc0, release build rc0 |
| compound | 90259256 (this) | in-tree roadmap landing (rebuilt band) + this record + seam doc |

## Batch and outcome

**rm-1415 'restore the --search full-text transcript contract'** (correctness 65.0, this run's own roadmap mint) — the only fresh defect on this run's evidence chain. `--search` promised full-text transcript search (README.md:464, main.rs:203-206) but `search_session_evidence` scanned derived metadata only (search.rs:153-214, truncate(8) at :213-217); body-only queries silently returned `count:0` rc0.

Fix (implemented, uncommitted): `search_sessions_detailed` → `SearchReport` with `transcript_scan` disclosure; bounded per-session transcript re-read, unconditional for every canonical session (NOT gated on metadata-empty — the metadata arm runs first and transcript evidence merges after it with dedup); 8 MiB/file + 64 MiB total budget; skip+disclosure counters; `snippet_around()` 96-char window; row cap 8 post-merge; stable `search_sessions()` signature; byte-stable JSON lane; rm-988 blank rejection preserved in both lanes; sanitize routed through existing `sanitize_line_segment`; red corpus internalized as `tests/fixtures/search-transcript/{body-only,body-only-2}.jsonl`; 6 contract pins in `tests/search_transcript_contract.rs`.

Recorded validation (consumed, not re-run): targeted first-run green; full ci.yml-verbatim green across 24 lanes with all 4 standing reds on delta-untouched surfaces (install-runtime test D / rm-051; plugin-version v0.0.4 CHANGELOG section; actionlint SC2001 ci.yml:192; cargo-deny rustls RUSTSEC-2026-0346+ — fix owned by sibling Cargo.lock bump). Live flip: `--search zygomorphic` count:0 → count:2 with transcript snippets + `transcript_scan` block; metadata control 'opener' unchanged; blank query still refuses rc1.

## Roadmap landing (delivery change, and why)

The roadmap phase's spool-side band (8-hunk patch + postimage + builder) was **reaped from delegate scratch before any gate could land it** — only the typed envelope 06b5995d survived. This compound **rebuilt the band in-tree from the envelope spec** (the 17680dd2 precedent, run f923a62a cycle 3): mint rm-1415 row with the candidate→implemented flip + VALIDATED rider, and six dated rebuilt riders on rm-992, rm-713, rm-406, rm-201, rm-1020, rm-012. Every rebuilt line is marked `rebuilt in-tree from the typed envelope by compound 90259256`.

Postimage verification: md5 `1fd96e5c4ba0c9c994f533186fe0f5f7`, 387 def rows (+1), def-max rm-1415, 0 duplicate ids, `## Closed items` singular, managed footer remains the last line, +15/-0 pure insertion. Builder (guarded on the pristine base md5 11853c3b): `delegate/90259256…-scratch/build_compound.py`.

**Review-fix amendment (2026-10-15, review-fix e73a2fd4, resolving independent review 94adc6c6 NEEDS_CHANGES findings 1+2):** the rm-1415 VALIDATED rider and the paragraph above originally misdescribed the transcript arm ("only when the metadata arm found nothing" / "gated on metadata-empty" — false: `transcript_evidence()` runs for EVERY canonical session, search.rs:131; "TranscriptScanReport" → actual struct `TranscriptScanSummary`; "100-char window" → actual 96). Corrected in-tree in both files; postimage md5 after the correction is `df1b81d5c483efd72b1d02534759d4b5` (census re-verified: 387 def rows, 0 dup ids, def-max rm-1415, `## Closed items` singular, footer last line, still +15/-0 pure insertion vs base). No executable surface touched.

## Reap/adoption ledger — six provider reaps, zero lost work

| reaped attempt | phase | class | disposition |
|---|---|---|---|
| 0b5b7fb1 | research | zero-durable (1-line boot-death event log) | redone from scratch (5a6734fb) |
| 93c3602c | prioritize | zero-durable (@24s, 4-line log) | redone (9fdb9735) |
| 9fdb9735 | prioritize | zero-durable (@10-progress) | redone (ae20fc51) |
| 42048792 | implement | complete-work-then-reaped (@44.7min: no typed artifact, spec-complete uncommitted worktree delta) | **adopted** by redo 37c02d90 after line-by-line verification vs stewardship request 5f7cd666 |
| 815c30f8 | targeted_tests | zero-durable (@118s) | redone (5bf8c6ee) |
| d266a8ae | compound | zero-durable (@168s) | redone (90259256, this) |

## Prevention rules (reusable)

1. **The durable trail decides, not the event log.** Enumerate typed JSON, scratch dir, event-log line count, and worktree porcelain before redo-or-adopt. A `session_reaped exit=failed` line is availability signal, never a work verdict.
2. **Adoption needs identity + spec match.** A reaped implement's uncommitted delta is adoptable only after verifying it against the stewardship request line-by-line and confirming the tree census; the adoption is then declared in the redo's evidence.
3. **The typed envelope is the roadmap band's source of record.** Spool scratch is reap-vulnerable between phases; landing learnings in-tree at compound removes the chain of custody gap (this cycle exercised it once already).
4. **TMPDIR on sdb, warm fleet targets when tight.** The cli `upstream` suite's StorageFull failures under `/tmp` are environmental (stash A/B at pristine cab6917 proved identical 0/13); `/tmp` hits 100% on this host mid-battery.
5. **Declare test-surface scope from the tree census.** The engine dispatch block's `changed_testable_surfaces=[]` cannot see an uncommitted implement delta; the tree-level digest does.
6. **Zero-doc-edit when prose already states the contract.** Restoring implementation truth beats editing README/--help — the fix made existing wording true (CU-C).
7. **Internalize red corpora in-repo.** A /tmp+spool-only red corpus dies to the reaper; the contract fixture now pins the red durably.

## Next-cycle candidates (concrete starters)

1. **rm-1020 pricing catalog refresh** — bundle 6 days stale at census (3,130 keys cut 2026-10-08 vs LiteLLM live 4,084; live-only 1,420 / bundled-only 466; 160 regional-prefixed live-only; models.dev 228 providers / 8,814 models, md5 3d70bf7f byte-stable same-day). Re-census before implementing; check sibling pricing.rs fences.
2. **rm-992 codex zstd decode arm** — demand-proven: tokscale #1423 merged 2026-10-11; codex 0.163.0-alpha.10 carries #53732 + #53670.
3. **rm-713 gemini trigger FIRED** — stable dist-tag 0.64.0 live at the 2026-10-14 fetch.
4. **Reliability leads**: rm-1054 (dir-scoped cache banking), rm-967 (coverage collapse at corpus scale).
5. **Standing reds** (reconcile-by-title with sibling owners before re-attempt): install-runtime test D (rm-051), plugin-version v0.0.4 CHANGELOG section, actionlint SC2001 ci.yml:192, cargo-deny rustls advisories.
6. **Watch**: Claude Code dist-tags split (latest 2.1.296 / next 2.1.299 / stable 2.1.287; 2.1.299 tarball has no CHANGELOG.md) — version probes must not assume 'latest'; codex stable 0.162.1; luoyuctl fork tip 15ed07f2 (2026-10-06); codeburn 0.9.27 with open issues #1756/#1757.

## Skill disclosure

No `ce-*` router skill installed in this harness (avail = `agent-reach` only) — disclosed per cycle-4 roadmap precedent; compound discipline embodied from fleet precedent (banner/def-row/rider conventions as landed by sibling compounds).

## Commit-gate duties

See `delegate/90259256…-scratch/APPLY-SEAM.md` (spool). Headline: **do not** apply the reaped spool roadmap patch — ROADMAP.md is already modified in-tree; stage it with the code delta.
