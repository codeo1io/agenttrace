# Cycle 2 compound record — run ff0068ca31634edc9fcd2629ee064c87

- **Campaign:** repository-maintenance `0314111d36224913b946d183b593bb63`, cycle 2
- **Worktree:** `run-ff0068ca3163-ff0068ca` @ base `8991144` (HEAD never moved; all work uncommitted by phase contract)
- **Batch:** "honest failure boundaries" — `rm-596` (SQLite row-drop + unreadable-DB honesty, lead) + `rm-610` (CLI error cause chains) + `rm-611` (CJK display-width underline)
- **Compound attempt:** `1d77734312fa4c6ca459d4ec4326c989` (after full_tests `53c9af4d`)

## Attempts lineage

| phase | live attempt | dead predecessor | disposition |
|---|---|---|---|
| assess | `7371fe9f` | — | 14 live adversarial probes, porcelain 0 throughout |
| research | `54beb4e3` | `25c00a9e` (provider, 08:14–08:21Z) | dead = zero durable (no envelope, no scratch) → **redone from scratch**, nothing adopted |
| roadmap | `06692f2b` | `b10f0af7` (4-event reap, no envelope) | dead = zero durable → **redone**; surfaced only by this compound's event sweep (the live phase's evidence never mentioned it) |
| prioritize | `60534aec` | — | re-bound `rm-597`→`rm-610`, `rm-598`→`rm-611` after the mid-cycle landing collision (see prevention doc) |
| stewardship | `e8f0cbbf` | — | contract authored, no Git topology chosen |
| implement | `e2fabc59` | `850e5c06` (provider, 14:32–14:34Z) | dead left ONE durable artifact (the rm-610 cause-chain pin test in entrypoints.rs) → forensically verified (event log start/2×progress/reap/complete; no result envelope) and **ADOPTED single-artifact** into the redo; the live attempt's start-of-session porcelain was torn/stale (showed only `M ROADMAP.md`) — trust the event log + stability watch, not a snapshot |
| targeted_tests | `43397de4` | — | 476/0 across 20 result lines; fmt --check rc0; clippy --workspace --all-targets 0 findings; no executable surface changed that turn |
| full_tests | `53c9af4d` | `dee401b7` (provider, 23s, msg_count 2) | dead = zero durable → **redone from scratch**; 21 CI lanes rc0 |

Four provider-dead attempts in one cycle, only one of which left adoptable work. The discriminator held each time: **read the event log, enumerate durable artifacts (envelope, scratch, /tmp, tree delta), verify leg-by-leg, adopt only what verifies.** A missing envelope is not evidence nothing happened; a present delta is not proof it is valid.

## Recorded validation outcomes (pre-review; consumed at compound, NOT re-run)

- **targeted (`43397de4`):** `cargo test -p agenttrace-core -p agenttrace-tui -p agenttrace` = 476 passed / 0 failed over 20 `test result: ok` lines (core lib 187/0, discovery_contract 81/0, **sqlite_hostile_disclosure 5/0** new suite, entrypoints 35/0, csv_export 7/0, upstream 9/0, launch_guards 4/0, warm_cache_pricing 2/0, tui 48/0, cli lib 43/0, + doc-test targets); `cargo fmt --all -- --check` rc0; `cargo clippy --workspace --all-targets` 0 findings.
- **full (`53c9af4d`):** dispatch `validation.full_command` was EMPTY → the ci.yml lint+full+deny jobs ran lane-for-lane locally in CI step order (fleet precedents db6b76260a11/0597e89a, 84c45ccd/a97f5d12, 18b1e331 gate-verbatim at this base): **21 executed lanes, all rc=0** — cargo test 22 suites 476/0, clippy (CI `-p` form), release build, all 8 release-binary artifact gates, ruby/npm/manifests/plugin-version/syntax/locked-cargo, `cargo deny --all-features check`, supplementary `clippy --workspace --all-targets`. Two CI-condition-gated steps documented-skipped (tag-fetch pre-gate; TUI real-dir smoke). Lane 06 (output-contract) took 566s: the unbounded live-homes `--doctor` scan (rchar 737 MB → 5.7 GB, steady — the documented non-hang pattern), which incidentally exercised the NEW doctor failure-counting against real multi-tenant homes and emitted valid JSON.
- **Digest lineage:** dispatch token `validation:v1:a72443d225dc8d687e34505eb328bd65608884abc60d932d59f83678062b7b0c` declared VERBATIM by both validation turns — both were run-only turns over a byte-identical tree (porcelain 9 M + 4 ?? in and out; `git diff` sha256 `949009fea743c08f…` stable); replica re-derived with `hermes_conductor.validation_policy.validation_digest` over the programmatically captured base `89911442173d31f4eb21a1968bfd51ad46e32ed1` = MATCH. Compound touches ROADMAP.md / CHANGELOG.md / docs only — non-executable, no re-validation owed and none performed.

## Fleet census at compound (2026-10-06, for the next mint, not for this one)

- **origin/master** `09cb224` — landed def-row ceiling **rm-600** (walls moved three times within this one cycle: `ee67b22` ceiling rm-545 → `05d5016` rm-598 → `09cb224` rm-600; the exact mechanism the new prevention doc names).
- **Uncommitted worktree claims** (normalized def-row maxima): rm-625 (run-9873fc06), rm-619 (run-91833f02), rm-609 (run-0583fb6e), rm-600 (run-17319815), rm-595 (run-460d0633) … → fleet high-water **rm-625**, next free **rm-626 after a fresh live sweep** (dashboard 6xx spool numerals are a separate id space and excluded).
- **Zero ids minted at compound** (house rule). This run's wall delta stays +3 def rows / 2 rebind notes / 1 roadmap banner + this compound's banner.

## Commit-gate checklist (the delta to land as ONE batch commit)

1. Tracked M paths from the implement batch (9): `ROADMAP.md`, `sqlite_sessions.rs`, `doctor.rs`, `discovery.rs`, `lib.rs` (core), `main.rs` (cli), `presentation.rs`, `tests.rs` (tui), `entrypoints.rs` (cli tests); compound adds two doc-only tracked edits: `CHANGELOG.md` (3 Fixed riders) and the `ROADMAP.md` compound banner + flips above (same file, already counted).
2. 4 ??: `tests/fixtures/{opencode-hostile,hermes-hostile,random-bytes}/` (README-pinned sha256s: `06ead30f…`, `f0463565…`, `efea7142…`) + `tests/sqlite_hostile_disclosure.rs` — land WITH the tests that consume them.
3. New at compound: `CHANGELOG.md` 3 Fixed riders; `docs/stewardship/2026-10-06-cycle2-compound-record-runff0068ca3163.md`; `docs/solutions/workflow-issues/mid-cycle-landing-consumes-minted-numerals-rebind-dont-remint.md`; ROADMAP compound banner + candidate→implemented flips (done-flips reserved to the commit gate, rm-012 convention).
4. Seams: `main.rs` is shared with in-flight d65f72c7's `rm-590` (positional path classification) — title-disjoint subjects, sequence rather than blind-merge; the rebind numerals (rm-610/rm-611) are provenance-recorded on the rows and banner.
5. Re-verify: full suite at the commit gate per its own dispatch; the digest there is the commit-time emission.

## Next-cycle leads (recorded, not minted)

1. **Bound the output-contract gate's doctor leg** — the fleet's slowest lane on multi-tenant hosts (566 s / 5.7 GB rchar this cycle) vs 1 s for the rm-444-bounded docs-gate leg; same remedy (`-d` over a fixture corpus). The natural cycle-3 roadmap mint.
2. **Cross-process session-cache lost update** (assess F1; `fd-lock` 4.0.4 current) stays unlanded `rm-580`'s exact claim — corroboration only.
3. **Drift watch items** on the cycle-2 roadmap banner (claude-code 2.1.291, codex 0.160.1) re-arm the usual triggers.
4. Deferred research candidates ride their rows' dated notes (LiteLLM pricing refresh cadence; pi 1.0.3 azure provider rename → `by_provider` alias family).

## Scratch provenance (persists where /tmp survives)

`/tmp/at-assess-ff00` (probes + hostile-home fixtures), `/tmp/at-research-54be` (dossier), `/tmp/at-prioritize-6053`, `/tmp/at-targeted-4339`, `/tmp/at-full-53c9` (lane logs + `full-tests-record.md`), `/tmp/at-impl-e2fab` (live E2E homes). `/tmp` is sweepable — the durable copies of the phase evidence are the delegate envelopes + this record.
