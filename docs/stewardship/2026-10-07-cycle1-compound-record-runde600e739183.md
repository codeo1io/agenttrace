# Cycle 1 compound record — run de600e7391834bafa2967938c7135160 (repository-maintenance fe743fb3)

- **Base:** 61570eaeefb57e12b1d062e5a1c625b3417b86e5 (run worktree `run-de600e739183-de600e73`, family `agenttrace-80c75f65b7`)
- **Phases:** assess `cbeffb67` · research `8d66e93d` · roadmap `4c34ecaa` · prioritize `d4ce541f` · stewardship `298c5bb4` · implement `26e85ea2` · targeted_tests `556c51b9` · full_tests `b41be38c` · compound `4ec574da` (this).
- **Batch:** "Trusted write paths & honest gates" — rm-693 (LEAD 92.0, rank #1 of 144 open rows) + rm-697 + rm-703 + rm-704, selected by prioritize `d4ce541f` from the 144-open-row postimage wall minted this cycle (rm-693..rm-707).
- **Delta at compound time:** 10 M tracked from implement (ROADMAP.md, CHANGELOG.md rider, cli main.rs + tests/entrypoints.rs, core history.rs/pricing.rs/reports.rs/session_cache.rs/statusline.rs + tests/demo_contract.rs) + this compound's ROADMAP annotations (compound banner + 4 VALIDATED addenda) + this record + the collision-doc rule 6. No commits; the worktree is the authoritative carrier for the commit gate.

## Prior-attempt forensics across the cycle (five reaps, one adoption, zero lost work)

| dead attempt | phase | evidence at reap | verdict |
|---|---|---|---|
| `9e89faa3` | stewardship | event log: turn_started → progress msg_count=11 → session_reaped (provider); no typed artifact, no scratch | redo (done by `298c5bb4`) |
| `7af2…` (prefix only survives on the rm-693 row) | implement | left a subset delta (no reports.rs), superseded by the fuller `50cbbae9` delta | superseded |
| `50cbbae9` | implement | provider fault 09:12–09:21Z, no typed artifact — but an 8-file uncommitted delta covering ROADMAP + rm-693 + rm-703 + rm-704 + tests | **ADOPTED** by `26e85ea2` after line-by-line verify against the stewardship surface pins (ROADMAP byte-identical to the verified spool postimage; no other session touched the worktree), then extended to the history + statusline staging sites and rm-697 the dead delta had left unimplemented |
| `e312d2e0` | full_tests | ~25s, message_count=6, no artifact/scratch, no at-* dirs in its window | redo (done by `b41be38c`) |
| `46330765` | compound | ~78s, message_count=2, session_reaped 15:21:31Z; no typed artifact, no scratch dir; worktree mtimes all predate its window | redo (this attempt `4ec574da`) |

Lesson held again (de96d4cc cycle-2 precedent): the event log's reap line + typed-artifact presence + tree mtime census decides adopt-vs-redo BEFORE any rework — an absent envelope is not evidence no work happened, and a present delta is not proof it is valid.

## Recorded outcomes (pre-review; consumed at compound, NOT re-run)

- **Implement `26e85ea2` (with the adoption above):** `write_private_exclusive` (O_EXCL create_new 0600 + seq-bump retry ≤16 + rename-into-place; loud refusal on a fully poisoned window) at every staging site — cli write_output_resolved (-o lane), session_cache save_session_cache + store_sqlite_snapshot_at, pricing write_pricing_cache_at (both arms; closes the 0644 gap → 0600), history save_derived, statusline compact_statusline_capture_under; journal append via open_private_append (create_new append-only + pre-open inspection). Baseline reader rejects negative/non-finite totals with the rm-569-shaped regeneration hint. Quarantine uniques as `history.json.corrupt.{N}` (cap 10_000, disclosed). `create_dir_all(parent)` wrapped with `-o` target+phase context. Red-first proofs: live stash-red rc101 recorded for rm-697 (demo_contract.rs:405) and rm-704 (entrypoints.rs:267); rm-693's six symlink-plant tests and rm-703's corrupt-twice test are red-by-construction on base (no live stash-red run for those two — review rider 1c6f7aa3 absorbed at the commit gate). Live e2e: `-o` write 0600, zero temp litter, rewrite-over-existing keeps 0600. +9/−0 tests.
- **Targeted `556c51b9` (validation-only, zero fixes needed):** tree = exactly the 10 batch files, `git diff` byte-identical to implement's handoff `26e85ea2…-scratch/final-delta.diff` (sha256 `fcfbacf47f97326aec22a8de5d80a05d080ff969562cdf424caeeec5891dc449`; note the earlier dispatch summary's `fcbacf…` was a truncation typo — the full value is authoritative). fmt rc0; clippy `--workspace --all-targets --locked -- -D warnings` rc0/0 warnings; core lib 212/0; demo_contract 14/0; entrypoints 35/0; cli bins 61/0.
- **Full `b41be38c`:** dispatch `full_command` empty → `.github/workflows/ci.yml` at base 61570ea is the command authority (`validation_policy` `if expected_full:` skip), mirrored lane-for-lane, CI step order, commands verbatim: **22/22 executed lanes rc=0** (lane 14 = documented var-gated skip), 03-tests **558/0 across 26 result lines** = assess pristine baseline 549 + batch census +9/−0 EXACT; release build + output-contract + deterministic-output + report-semantics + real-cli-smoke + plugin-version + cargo-deny all green (log anchors: `advisories ok, bans ok, licenses ok, sources ok`; `plugin.json v0.9.0 matches CHANGELOG v0.9.0`). Tree immobile pre/post (git diff sha256 unchanged; porcelain sha256 `b9b9c155…`). Digest `validation:v1:d2b35fb3c8b92a6ffa71f7c51a0a4501766f351640207f73310a81f623a93cbc` declared verbatim AND re-derived live with the engine's `validation_digest()` — MATCH (docs/ROADMAP deltas are digest-immobile under the classifier).
- **Assess `cbeffb67` (baseline inputs):** 549/0 across 26 result lines, fmt rc0, clippy (trio + workspace all-targets) 0 warnings at pristine HEAD; the dfc3b36 landed batch reviewed line-by-line CLEAN; live PoCs NN1 (`-o` raw EACCES context loss), NN1' (`-o /dev/stdout` double-emit 2-vs-1), NN2'/NN3' (baseline sign admission, quarantine overwrite), config probes.
- **Research `8d66e93d`:** upstream QUIET at 15ed07f (open set 5); CodeBurn 11,344 stars frontier; 4 evidence-backed candidates (C1 Antigravity 2.16 quota shape — decode BOTH pre/2.16; C2 Command Code; C3 Mistral Vibe; C4 generation speed) → minted rm-705/706/707 + rm-707-era riders on landed rows.

## Status flips and id landscape

- rm-693 / rm-697 / rm-703 / rm-704 carry EXECUTED (implement) + VALIDATED (this compound) addenda; `status: implemented` stands — `done` flips at the commit gate (rm-012 convention). **ZERO ids minted at compound.**
- Status accounting: 250 backticked def rows = **140 candidate / 63 implemented / 47 done** (264 both formats incl. 14 legacy bare rows); duplicate probe (normalized, both formats) empty; managed footer still last.
- In-flight claim map live-censused at compound (worktree uncommitted diffs + spool scratch + /tmp, backtick-optional pattern, numeric sort): rm-685..686 (2f02ecaf) · rm-709..715 (73fe8e1e) · rm-726..728 (32f3b7a1) · rm-760..763 (ec762a61) · rm-764..770 (6a844b9b) · **rm-771..775 (aa41d9b5) vs rm-771..774 (e88f2da2) DOUBLE-MINTED on disjoint subjects — reconcile by title+numeral at integration** · rm-776..779 (14954d7a). Fleet high-water **rm-779 → next free rm-780 after a fresh live claim census** (never reuse this number without re-sweeping — rule 6, below).

## Cycle-2 deferred queue (priority order from prioritize `d4ce541f`)

1. **rm-421 (90.0)** compaction laundering parser.rs:1448 — verify-first. **rm-251 (90.0)** re-emission dedup — parked on upstream #312 post-merge reconcile design. **rm-195 (90.0)** release wrong-owner npm tarball — release/CI lane, commit-gate-adjacent.
2. **rm-705 / rm-706 / rm-707** — fixture-first competitor lane (Command Code `~/.commandcode/projects/`, Mistral Vibe `~/.vibe/logs/session/unified/`, generation-speed tokens/sec metrics); needs synthetic specimens before any parser work; shapes quoted in `/tmp/at-research-8d66/research-dossier.md` (sweep-volatile — this record is the durable pointer).
3. **rm-694** generic-lane usage fold — needs role-semantics design (which roles fold); 4M-token PoC corpus preserved under assess scratch.
4. **rm-698** cache RMW locking — concurrency design choice (lockfile vs merge-on-write) before code.
5. **rm-695** waste red-tier reachability — needs data-driven threshold rationale, not a bare constant change.
6. **rm-699..702** — small correctness wave 2.
7. **Compose, never re-implement:** rm-696 (stdout-alias double-emit) is title-twin with sibling run-9a4d37af's IMPLEMENTED rm-641 (`output_is_stdout_alias`, red-first tests + live PoC) — pair by title at integration. rm-693 is title-twin with the 8fbbf166 campaign's planned `rm-664+` batch (d36c4cd7 earmark) — our landed implementation wins the seam, their band renumbers above the fleet frontier.

## Residuals disclosed on the rows (open arms, no new ids)

- rm-693: `open_private_append`'s inspect→open swap race — a static plant is refused; a live swap TOCTOU is not closed (O_NOFOLLOW needs a libc dep). Deferred with the row.
- rm-703: past the 10_000-generation cap the last quarantine slot is reused — disclosed in code and on the row.

## Commit-gate seams (for the next phase, NOT performed here)

- ONE commit for the whole batch: the 10 implement files + ROADMAP.md compound banner + 4 VALIDATED addenda + CHANGELOG rider (already present) + this record + the collision-doc rule 6. Done-flips for rm-693/697/703/704 at the gate, **by title** (fleet double-mint rm-771..775 discipline).
- Validation digest moves only at the commit gate (classifier digests executable surfaces; the ROADMAP/docs deltas are immobile) — re-declare `validation:v1:d2b35fb3c8b92a…` provenance from the full_tests record, do not re-derive expectations from a dirty intermediate.
- The worktree `git diff` (sha256 `fcfbacf47…`) is the authoritative batch copy; spool/log mirrors under `delegate/*-scratch/` and `/tmp/at-*` are sweep-volatile.

## Prevention rule extended this cycle

`docs/solutions/workflow-issues/roadmap-campaign-id-collision-at-integration.md` **rule 6** (this compound): the in-flight claim sweep must cover ALL claim homes (worktree uncommitted diffs, delegate-spool scratch patches/postimages, /tmp research artifacts) with the numeric-sorted backtick-optional pattern — this cycle's roadmap phase burned two sweeps (lexicographic string-sort noise; plain `rm-NNN` grep blind to backticked rows) before the correct third pass, and the compound-time re-census above found the fleet high-water at rm-779, far past this wall's rm-707 — a band chosen off either wrong sweep would have double-minted inside live sibling territory.

Review and shipping outcomes deliberately absent here — they land after the compound phase; the next cycle's assessment carries them forward.
