# Cycle 2 compound record — run 6cb2756a7e014e44be735d0284f29db5

- **Run:** 6cb2756a7e014e44be735d0284f29db5 · repository-maintenance 91d44ce8 · cycle 2 · worktree `run-6cb2756a7e01-6cb2756a` @ base 39bd06b3 (branch conductor/run-6cb2756a7e01)
- **Compound attempt:** 392550ac90cd483d9631b67d669d5982 (2026-10-08), after targeted_tests d8c84263 and full_tests 018ac82d — provider-reaped 04:07–04:17Z after completing the banner, the five candidate→implemented flips with EXECUTED bullets, and both docs below, but before writing its result envelope; verified against the dispatch-recorded outcomes (identity/lineage, tree census, no test execution) and adopted whole by 103de133aab44b738386fe720ccfa87c, which amended this record (adoption note, dead-attempt forensics, numeral-contention seam) and emitted the envelope
- **Batch:** "Subagent attribution honesty & machine-format completeness" — rm-797 (lead) + rm-798/rm-799/rm-801/rm-855 (rm-800 rebounded to rm-855 at the commit gate; prioritize 7c107edb selected this run's own assess findings 35b3b690 F1–F5; stewardship bc45d2fd)
- **Status rule applied:** candidate→implemented flipped HERE with dated EXECUTED bullets; done-flips reserved for the commit gate (rm-012 convention). Zero ids minted at compound.

## Cycle outcome (all pre-review; tree left uncommitted for the commit gate)

10-file worktree delta (+548/−54, zero untracked) at HEAD 39bd06b3:

| Unit | Row | Change |
|---|---|---|
| U1 | rm-797 | `attribute_subagents` rewritten around `LinkKey` = same-file identity (dev/inode; canonical off-Unix) with raw-path fallback — a parent spelled twice (hardlink/symlink across scanned roots) links from every spelling; self-links count unlinked; returns orphan count |
| U2 | rm-798 | `--sessions -f json` wraps rows as `{matched_sessions, returned_sessions, truncated, limit, sessions}`; csv gains a capped-only `# truncated: showing N of M (--limit L)` marker; every format gets the stderr Note; TSV byte-unchanged |
| U3 | rm-799 | orphan count on `LoadReport.unlinked_subagents` + `disclose_unlinked_subagents()` stderr warning on report runs |
| U4 | rm-855 (minted rm-800, rebound at commit gate 0bfffcc0) | csv `SessionCsvRow` + `subagents,subagent_cost,parent_session` (13-column header) + README contract |
| U5 | rm-801 | README flag-row/csv-section contract, `--help` entry, CHANGELOG Unreleased rider; `scripts/record-real-marketing.sh` migrated off the bare-array json (the ONE in-repo consumer, with pre-rm-798 tolerance) |

## Recorded validation outcomes consumed at compound (NOT re-run here)

- **targeted d8c84263** — core lib 218/0 · subagent_attribution 4/0 · discovery_contract 90/0 · cli bin 65/0 · csv_export 8/0 · fmt/clippy clean · docs-gate rc0 (after rebuilding a `target/release` wiped between phases) · shell static (bash -n, shellcheck, node --check) + 3-case behavioral replay of the embedded consumer all green.
- **full 018ac82d** — dispatch full_command EMPTY ⇒ ci.yml full+deny lanes run VERBATIM, sequential: **21/21 lanes rc=0**; tests-trio 600/0 over 31 result lines + entrypoints 42/0 = **642 machine-summed passed / 0 failed / 0 ignored**; cargo deny `advisories ok, bans ok, licenses ok, sources ok`; the known sqlite env-race flake did NOT fire.
- **Digest:** `validation:v1:ff1421a4a015627d445dd06ddf28254e9c9d214be02f98ab932914ebacbb016f` — declared VERBATIM by both validation turns over a byte-identical tree; engine re-derivation MATCH.

## PR-1..6 — what this cycle taught

- **PR-1 (process, KTD13):** an implement fold was rejected solely for a missing `validation_evidence.changed_surfaces` attestation. Derive it with the engine's own `hermes_conductor.validation_policy.changed_surfaces()` (base via `git rev-parse`, never hand-typed) and declare ALL repo-relative changed paths — the engine classifies `scripts/*` as EXECUTABLE, `crates/**.rs` as non-executable, and the gate cross-checks declared executables against its own delta. Prevention doc: `docs/solutions/workflow-issues/implement-fold-needs-changed-surfaces-attestation.md`.
- **PR-2 (assessment honesty):** assess F2's "30 files → 20 rows" PoC was NOT dedup loss — it was the silent DEFAULT `--limit 20` cap. The rm-798 disclosure makes the default cap visible in every format; future assesses must not re-report it as data loss.
- **PR-3 (scratch harness):** a diagnostics echo placed between rc capture and rc recording silently disabled fail-fast in the first full-suite runner (zsh-ism `${((…))}` aborted the lane function after `rc=$?`, returning 0). v1 run VOID; v2 re-ran all 21 lanes with per-lane logs and immediate rc append. Rule: record rc before anything that can fail.
- **PR-4 (environment):** `target/` can be wiped between phases. `check-docs-commands.sh` HARD-fails without `target/release/agenttrace`; the release-pinned TSV e2e leg SOFT-passes (early-return Ok) — a silent coverage downgrade. Before release-dependent gates: verify the binary, else `cargo build --release -p agenttrace` (~2.5 min).
- **PR-5 (evidence hygiene):** the implement-phase record carried a stale discovery_contract count (86); the deterministic count on the tree is 90 (`-- --list`). Machine-sum when a count matters; a hand-carried number can be one cycle stale.
- **PR-6 (attribution honesty by example):** identity-based linking (U1) is the fix class whenever a resolver walks paths that a deduplicator identities — exact-string resolution diverges from identity resolution precisely on hardlinks/symlinks/cross-root copies, and the divergence is invisible until a PoC proves it.

## Dead-attempt forensics (this run — 4 provider deaths + 1 fold rejection, all dispositioned)

- **assess 45537c8f** — provider 429 at 424.8s; its `results/*.json` is a FAILURE record and its `done/*.json` the dispatch envelope (no typed PhaseResult); salvage baseline `/tmp/at-assess-45537/test-baseline.log` (595/0 @ this HEAD) ADOPTED as the pre-batch reference; assess redone from scratch (35b3b690).
- **research 00cd5846** — provider death at +770s mid-collection (event log = status pings only, no typed artifact, no synthesis); RAW catalogs (`/tmp/at-research-00cd5/`: upstream PR list #279–#318, the 173KB #312 diff, CC changelog, opencode releases) verified against live state and adopted where not superseded; synthesis redone (6988a0d6).
- **implement-prior 2c0c0709** — provider death at 10 messages; zero durable work (no typed artifact, no code delta); redone from scratch.
- **implement 7e7096d3** — NOT a death: KTD13 fold rejection (PhaseResult omitted `validation_evidence.changed_surfaces`); tree byte-intact (all 10 changed-file mtimes ≤ its 00:36 result write); re-folded by 360949d2 with the engine-derived attestation — see the prevention doc below.
- **compound 392550ac** — provider reap at 04:17Z AFTER completing all tree work (banner, five flips, EXECUTED bullets, both docs; only ROADMAP.md touched among tracked files) but BEFORE the result envelope (envelope absent; 12-line event log). Verified against this dispatch — identity (run 6cb2756a / action compound:compound), test-selection provenance (EXECUTED bullets match the recorded targeted d8c84263 + full 018ac82d outcomes digit-for-digit), tree census (10 M + 2 ??), no test execution — and adopted whole by 103de133.

## Next-cycle leads (recorded, not minted)

1. `--doctor`'s own lanes do not disclose `unlinked_subagents` (render_doctor_report walks files separately from load_sessions_report) — the natural rm-799 completion.
2. The sqlite env tests' private `ENV_LOCK` race remains the standing flake (sqlite_sessions.rs vs the shared `lock_env` helper in lib tests); it did not fire this cycle — retire by sharing the lock.
3. Research watch items re-arm the drift triggers: CC 2.1.293 signal shapes (serverToolUses/agentId/agentType/effort → rm-037/rm-406); `claude-haiku-5-5` absent from BOTH the bundle and live LiteLLM (→ rm-176/rm-164).
4. Upstream #312's Gemini-drop must be carved out of any port wave (rm-448 decision; repo non-goals #236 forbid doc removal on migration messaging alone).

## Commit-gate seams

- Stage the 2 untracked compound docs EXPLICITLY (this record + the prevention doc) beside the 10-file delta.
- rm-797..rm-801 done-flips + EXECUTED-era dating land in the same commit; the standing +9 research ROADMAP lines (roadmap phase d36014e7) ride the same commit.
- CHANGELOG Unreleased rider already written at implement; no riders owed at compound.
- **Numeral contention (live-verified at this compound, 2026-10-08):** the rm-800 numeral carries a title-disjoint sibling SPOOL claim — sibling wall run-a2abf1a8's census (roadmap 8efd1560 + adoption correction 7d3df557, base 2578751) records `worktree def-max rm-801 (run-6cb2756a), spool claims rm-800..803 + rm-816/817` and minted its own band rm-818..rm-823 (next free rm-824). Whichever commit gate lands SECOND renumbers BY TITLE onto next-free past a fresh live census (house convention — conflict cases 880a7b9e / 202c4d1d, `docs/solutions/workflow-issues/mid-cycle-landing-consumes-minted-numerals-rebind-dont-remint.md`); cite rm-797..rm-801 rows by TITLE, never numeral alone.
- RESOLVED at the cycle-2 commit gate (2026-10-08, 0bfffcc0): fresh census — the landed wall (origin/master) carries a title-disjoint rm-800 (run bbde2156, runtime_config test-tautology, itself a rebound; landed-first precedence) and unlanded spool def-row claims run through rm-854 (972cf938's mint declares NEXT FREE rm-855) — the campaign's machine_format row renumbered rm-800→rm-855 (rebind-don't-remint, 202c4d1d; band now rm-797/rm-798/rm-799/rm-801/rm-855, done-flips landed in the same commit, rm-012).
- **Forward-compose obligation:** sibling assesses pinned this batch's ABSENCE at their bases — a2abf1a8's assess names the `# truncated: showing N of M (--limit L)` marker as “6cb2756a's UNLANDED sessions_csv_bounded lane (a forward-compose reference only)”. Land the marker text verbatim so the sibling reference resolves, and sequence against sibling main.rs/csv_export.rs batches rather than blind-merging.
