# Cycle 2 record — run 91066ba0 (repository-maintenance bc152ef11, wall adae9a7c)

Phases: assess 5442e75a → research 0b22f8d0 → roadmap 2f058832 → prioritize 8931d7e8 → stewardship 1251ce18 → implement 5803f591 → targeted_tests 0efb9281 → full_tests 0c66cb29 → compound 2281d1ac. Two provider reaps this cycle (assess b723c934, targeted 524c47c4 — both redone from scratch, nothing adoptable); the roadmap band's scratch+envelope were ALSO swept post-delivery, forcing this verbatim-in-substance rebuild (fleet precedent: swept bands rebuild from the work-order record; census/rider-owner facts re-verified).

## Landed this cycle (uncommitted in the run worktree, 10 M + 1 ??, commit gate lands)
- rm-2013 hostile naive-extreme-year timestamps: choke-point year bound (0000..=9999) in parse_ts_form closes the quota_window + governance panics by construction; red-first proved the exact lib.rs:3191 panic first; new contract test file hostile_timestamp_truth.rs (5 tests, both chrono edges).
- rm-2014 installer pinning: AGENTTRACE_VERSION/INSTALL_DIR/NO_MODIFY_PATH honored by both installers, fail-closed measured (exit=1); false README install paragraph corrected in both languages; upstream #317 contract absorbed.
- rm-1574 extension: additive pricing_basis CSV column + TUI inline marker (text-lane vocabulary).

## Validation record (consumed, not re-run here)
- targeted (0efb9281): core lib 463/0 (run2; run1 had a governance drain-cap flake under load 52/14cpu, isolated rerun ok — load-induced, not a regression), hostile 5/0, csv 9/0, parity 2/2; digest e124a3b7 base-pinned.
- full (0c66cb29): ci.yml:134 verbatim `cargo test --locked -p agenttrace-core -p agenttrace-tui -p agenttrace` hermetic detached → 1413/0/0, 91 suites, 5m33s; ALL companions rc0 (fmt/clippy/manifests/release-surfaces/script-syntax/locked-cargo/install-ref-drift/changelog/npm-files/release-pins/homebrew/docs-links/docs-commands-rerun/npm-contract/msrv-1-89); docs-commands needed AGENTTRACE_BIN rerun with the debug binary (environmental).

## Reusable lessons (prevention rules)
1. Hostile-string panics: find the single parse seam before scattering checked arithmetic — one bound at parse_ts closed a class spanning two crates (proven again here).
2. Piped installers cannot take parameters (iex); pin contracts MUST be env-based and fail closed; and when docs contradict code, the docs row itself is a finding (the false PATH/-Version paragraph).
3. Machine surfaces (rm-895): disclosure = additive column, NEVER key suffixes; pin the header in tests.
4. Engine gotchas: dispatch validation block can stamp full_command="" (KTD10) — the authoritative suite is ci.yml:134 verbatim; validation digest must be BASE-pinned (validation_policy.validation_digest(base_sha, repo)) — None folds 'base:unknown' into the hash.
5. Provider reaps + spool sweeps are routine: write the envelope FIRST-class (typed, immediately), mirror bands to ~/.cache at write time (this cycle lost an unmirrored band), and run long suites detached (setsid+nohup).

## Next-cycle candidates (priority order)
1. fog rm-2017 reasoning-token billing at plain output rate (72 live models; sibling b37d6f73 mint, wall b425eb91 — reconcile by title, strongest correctness candidate).
2. fog rm-2011 snapshot refresh wave + non-chat-mode pricing (1,547 new / 467 retired keys; drift cron fires Monday 2026-10-20 06:00 UTC — predicted red until landed; svg-golden arm lives on the sibling wall).
3. rm-2015 qwen /export + dual-output (corpus-first: capture a real /export JSONL before semantics).
4. PR #27 ports still open on this wall's lineage: replay-equality zero-rebase + discarded reconcile stats (fog rm-1541/rm-1542 landed upstream master via c280887c).
5. serde_json 1.0.154 reconcile (fog rm-1895) + antigravity numeric-map wave watch (fog rm-1864; ccusage #1902/#1905/#1897/#1904).
