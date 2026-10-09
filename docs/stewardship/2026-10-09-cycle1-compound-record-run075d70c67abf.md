# Cycle-1 compound record — run 075d70c67abf (repository-maintenance bae59872)

Pre-review compound of cycle 1, attempt e775e6fa58b24c55ae7b33026870a5a4 (2026-10-09), after
transport-dead prior attempt 87e7a1b070cf4fd4bbbcbb5b8549b491. Worktree
`run-075d70c67abf-075d70c6` at base `6013a35e4d4b25d3c8fbcdd5beb2f7ef594a55b8`; the batch rides
UNCOMMITTED (9 batch surfaces + this record). Review and shipping happen after this step; the next
cycle's assessment carries them forward. No test execution at compound — every validation number
below is the recorded outcome of a prior gate, consumed verbatim.

## The batch: 'honest, fast doctor'

Selected by prioritize b6d64efe (designation banner on the wall); stewardship contract bb053f69;
implemented once by 68f4ab29 (fold-rejected on an attestation defect), re-issued without code edits
by 339a76fe; targeted 0d0f53ad; full 56ff4b1d.

| Unit | What landed (mechanism) | Live PoC | Red-first pin |
|---|---|---|---|
| rm-367 STAGE (a) — lead | `doctor_dir_report`'s file-session parse routes through the caching finder (`cached_session` hit, else parse + `store_session`); `doctor_directories` persists via `save_session_cache` when dirty. Stages b-d stay OPEN on the row. | six-file `-d` fixture: cold 0 entries/6 re-parsed → rerun 6/6 reusable, 368 ms→312 ms; 48-file corpus: cold 0.337 s/48 re-parsed → warm 0.328→0.307 s, 48 reusable/0 re-parsed | `tests/doctor_honesty_contract.rs::doctor_persists_fresh_parses_for_the_immediate_rerun` (RED at HEAD); doctor fs-tests + rm-596 demo-contract control re-sandboxed to planted-HOME markers so the new cache writes cannot leak into ambient HOME |
| rm-904 — honesty | `disclose_unlinked_subagent_count` runs `attribute_subagents` over the doctor's scanned sessions (file + sqlite + `--demo` corpus) and inserts `unlinked_subagents` into the disclosures map 0-count-clean; TUI `apply_loaded_sessions` LANDS `report.unlinked_subagents` into `LoadState`, `load_summary_line` appends ', N unlinked subagents (未关联子代理)' when non-zero | `--doctor -d <orphan fixture>` → `Disclosed facts: unlinked_subagents=1`; clean corpus silent | `doctor_discloses_orphaned_subagents_zero_count_clean` (core) + `load_summary_discloses_unlinked_subagents_zero_count_clean` (tui `src/tests.rs:2547`) |
| rm-903 — rider, decision (a) | `update-snapshot.sh` skips LiteLLM keys containing `'*'` at the mode-filter seam; concrete-region commitment-tier keys STAY by documented decision (exact-matchable); option (b) glob lookup declined | filter extracted verbatim, run on synthetic fixture → `wrote 2 chat models`; python assertions: wildcard skipped / concrete-region kept / ride-alongs kept / non-chat+zero-cost+non-dict skipped / header date+models=2 | `tests/pricing_snapshot_hygiene.rs::vendored_pricing_bundle_carries_no_wildcard_keys` (vendored bundle already clean — no re-vendoring) |

CHANGELOG duty was authored at implement: three `honest, fast doctor` entries sit under
Unreleased/Fixed in-tree (verified present at compound).

## Recorded validation outcomes (NOT re-run at compound)

- Targeted 0d0f53ad (engine named one executable changed surface, `scripts/pricing/update-snapshot.sh`):
  `bash -n` + `shellcheck` clean; the shipped-filter fixture probe above; core lib 268/0,
  doctor_honesty 2/0, doctor_demo 2/0, pricing_snapshot_hygiene 1/0, subagent_attribution 4/0,
  demo_contract 14/0; tui 55/0; `cargo clippy --all-targets` 0 warning/error lines; `cargo fmt
  --check` clean.
- Full 56ff4b1d (empty `full_command` → ci.yml `full`+`deny` lanes verbatim, the
  614624d7/7eae74ea/3ec6cec0/2023f222 convention): ALL 24 lanes rc=0 first run — fmt; clippy
  (crate-scoped + workspace `--all-targets`) `-D warnings`; `cargo test --locked` 754 passed / 0
  failed across 41 `test result: ok` lines, machine census 754 == 750 tracked + 4 untracked
  `#[test]`s; MSRV 1.89 floor check rc0; release build rc0; entrypoints rc0; all four
  `scripts/ci/check-*.sh` rc0. Logs: `/tmp/at-full-75d/`.
- Digest: `validation:v1:248a3f22b2833b7c2bb9bcb2c5a30d7f7613d020820d073b23b4bf11c4c59d04`
  re-derived == dispatch VERBATIM at BOTH gates (engine changed_surfaces = the 9 surfaces,
  executable = `[scripts/pricing/update-snapshot.sh]`). Compound adds zero executable surfaces
  (md-only) — digest-imobile per the classifier's sh/py/ps1/yml scope; not re-derived here per the
  phase contract.

## Phase-by-phase attempt ledger

| Phase | Attempts | Outcome |
|---|---|---|
| assess | ca6169f1 | hunk-level review of landings 987be34 + 8eb6ad4, verdict CLEAN, 4 findings (F1→rm-904, F3→rm-895 sibling, F4→rm-857 sibling, marker guard→rm-822 sibling) |
| research | 55f248e9 | pass 15, delta-scoped, every number first-hand; C86→rm-903 |
| roadmap | e5f06c9e DEAD (provider; 11 progress pings, zero durable) → 5e151fe6 | minted rm-903/rm-904 (348 rows); spool patch chain link 1 |
| prioritize | 5811bc80 REAPED (11 progress + reap; no scratch) → b6d64efe | batch 'honest, fast doctor'; designation banner; spool patch chain link 2 |
| stewardship | bb053f69 | structured stewardship_request, change-unit decisions U1/U2/U3 |
| implement | 68f4ab29 fold-REJECTED (missing `validation_evidence.changed_surfaces` while the engine delta held 1 executable surface) → 339a76fe re-issue, zero code edits | 9-surface uncommitted tree at base 6013a35 |
| targeted_tests | 0d0f53ad | nothing failed; zero fixes needed |
| full_tests | 56ff4b1d | 24/24 lanes rc0 |
| compound | 87e7a1b0 DEAD (transport; 15 event entries, no typed result) → e775e6fa (this record) | see forensics below |

87e7a1b0 forensics (why nothing was adopted): durable trail = `delegate/87e7a1b0…-scratch/work/`
holding `base.md` (md5 0414d306… == exactly the prioritize-designated wall) and a git chain repo
seeded with that single base commit — object census 3 (blob+tree+commit), no second commit, no
patch, no postimage, no doc, no typed result. The phase was redone from scratch; the dead attempt's
base CHOICE (designated wall as chain base) was confirmed correct and reused conceptually.

## Lessons & prevention rules (this cycle)

- **Fold-attestation (live instance):** 68f4ab29 finished a complete 9-surface tree but was
  fold-rejected solely for the missing `changed_surfaces` attestation; the 339a76fe re-issue needed
  zero code edits. Rule already durabled at
  `docs/solutions/workflow-issues/implement-fold-needs-changed-surfaces-attestation.md` — cited, not
  duplicated.
- **Forensics discipline:** an absent envelope is not evidence no work happened, and a present
  scratch is not proof of completion — this run's four dead attempts split exactly along the durable
  trail (three left nothing; one left a complete tree under an attestation defect). Verify the trail
  (typed result, scratch contents, tree drift, object census) before adopt-or-redo.
- **Numeral hygiene:** rm-903's mint text said "32 unmatchable keys" where the true union is 28 (the
  4 wildcard keys ⊂ the 28 commitment-tier keys). A set-diff count stated as a union must be
  re-derived as a UNION before minting; the correction was restated on the row and in the compound
  banner per the designation's instruction.
- **Compound consumes, never re-runs:** recorded targeted/full outcomes are the evidence; zero test
  execution at compound; md-only appends leave the validation digest immobile (classifier covers
  only executable surfaces).

## Next-cycle context

1. **rm-367 stages b-d** — open on the row, staged acceptance arms already written: (b) per-root
   wall-clock budget with honest partial-scan disclosure, (c) stderr progress before first parse,
   (d) count-sampled/size-capped root passes. Natural continuation lead.
2. Alternates (from the designation): rm-011 AFTER re-adjudication (row anchors drifted — live
   `search.rs` carries no `total_matches`), rm-339 (78.0), rm-487, rm-368 (only once 3f6b86bc's
   session_cache.rs fix lands).
3. Contended pool (owned by unlanded sibling claims — reconcile by title at their landing gates):
   rm-895 (reports.rs truncation), rm-857 (waste.rs re-price), rm-822 (csv marker guard).
4. Research watch lanes: @anthropic-ai/claude-code 2.1.295, @openai/codex 0.162.0,
   @google/gemini-cli 0.63.0, @zack78/agenttrace 0.10.1; upstream frozen 8 consecutive checks
   (15ed07f2 @2026-10-06, ahead 191/behind 30); OTel semconv-genai still zero tags; MCP spec latest
   still 2026-07-28 (index dateModified is site-rebuild metadata).
5. Fleet numerals (family census at compound): worktree walls max rm-894; unlanded family spool
   claims through rm-919 (535cbb3f 916; 778583d2 917-918; ed44938c 915-919); this wall's tail
   rm-904; next family free ≥ rm-920.

## Commit-gate seams

1. **Stage explicitly:** `git commit -am` drops untracked paths — stage
   `crates/agenttrace-core/tests/doctor_honesty_contract.rs`,
   `crates/agenttrace-core/tests/pricing_snapshot_hygiene.rs`, and this record, alongside the 7
   modified files.
2. **ROADMAP chain, in order, on the pristine worktree wall** (worktree `ROADMAP.md` stays md5
   `82dcadfd…` until then):
   `delegate/5e151fe6…-scratch/ROADMAP.patch` → `delegate/b6d64efe…-scratch/ROADMAP-designation.patch`
   → `delegate/e775e6fa…-scratch/ROADMAP.compound.patch` (this compound; three-link roundtrip proven
   from the pristine wall, each intermediate byte-identical to its phase postimage; re-apply of the
   compound link rejected).
3. **987be34's owed status seam** (recorded at roadmap): rm-044/rm-651/rm-652/rm-654 wall text still
   reads "done-flip reserved" — action at integration.
4. **Reconcile siblings by title** (rm-903's seam-neighbor rm-901 owns the same script's argv
   full-refresh trap, 862369f8 lineage).

## Artifacts & hashes

Spool scratch (this attempt):
    74eb3e4e67f1b181d2c0f2454ad1be78caba773673008487155d21d3b980ab14  ROADMAP.compound.patch
    ad20705a1257aa50d9545e534e35afd38602af98cb844581f715962303ffa2c7  ROADMAP.compound.postimage.md
    1248a02fe1f6cec200af615235081b92f87014affb7ee92002cc5c0bfb6e343e  build_compound.py


