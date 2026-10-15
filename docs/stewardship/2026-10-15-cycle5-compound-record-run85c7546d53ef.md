# Cycle 5 compound record — run 85c7546d53ef (repository-maintenance 205f84bb)

- **Date:** 2026-10-15 · **Compound attempt:** ae7fc7f4 · **Base:** 167bf2b (worktree; origin/master then 3e90993, now d2a02af)
- **Batch at compound entry:** 6 M + 4 ?? (the adopted release-integrity batch, full-validated, UNCOMMITTED — commit gate pending)
- **Compound delivery:** spool patch `delegate/ae7fc7f4…-scratch/compound-delta-ae7fc7f4.patch` chained on the pristine worktree ROADMAP (md5 42cbc4c4 → postimage 3b780c1d, +3 lines insert-only, 377→377 def rows, zero mints, zero status flips; round-trip byte-exact via both `patch -p1` and `git apply`), mirrored to `~/.cache/at-compound-ae7fc7f4/`; docs land in-tree (this file + the prevention rule).
- **Phase contract honored:** no tests executed this phase — targeted and full-battery outcomes were consumed as recorded evidence.

## Phases this cycle (each survived a dead provider-side predecessor)

| Phase | Attempt | Outcome |
|---|---|---|
| assess | f540641f (redo of bff38171†) | 14 findings; suite 1075/3 red ONLY under TMPDIR-inside-a-repo hosts; three-way TMPDIR proof; `--since` probe; hostile sweep 19×19 zero panics |
| research | 0bd06a82 | CC 2.1.299; codeburn #1756–1759; models.dev 17398e58 (229 providers/8,834 models, 0 negative costs); LiteLLM tier matrix stable 2 passes; codex rust-v0.163.0-alpha.11; gemini 0.64.0 ACP usage; pi 1.2.0; npm bare-name 404 (2nd pass) |
| roadmap | 50db6611 (redo of 24c9af39†) | band built against wall 8bc0587/1d59adda (2539 lines, 392 rows): 1 banner + rm-087/rm-381/rm-382 riders + mint rm-1444 — **reaped from spool, never landed** (see Lost work) |
| prioritize | b5ae1c84 | batch = "release-channel integrity": LEAD rm-169 (73.0) + rm-386 (71.0); 109-worktree seam census |
| stewardship | 8095e0c5 | surfaces pinned by read: unguarded `gh release create` at release.yml:167; npm-arm guard pre-existing; msrv facts |
| implement | 72f14637 (adopted 002d1c8b†'s 10-file batch) | rm-169 + rm-386 implemented, hardened (SIGPIPE race), red-first proven |
| targeted_tests | 585c169f | bash -n 6/6; YAML 4/4; 9 adjacent gates rc0; selftest 10/10; porcelain ae1d3d71 in==out |
| full_tests | 06af0baf (redo of ef99dcad†) | 29 lanes green; tests 1078/0/0 (neutral TMPDIR); clippy -D rc0; +1.89 check rc0; release build rc0 188 s; digest validation:v1:2f3dceff == dispatch; 4 pre-existing reds |
| compound | ae7fc7f4 (adopted 650d975b†'s drafted builder + cache mirror) | this record |

† bff38171, 24c9af39, 002d1c8b, ef99dcad, 650d975b — heartbeats-only event logs, typed artifacts absent. 002d1c8b left its complete 10-file batch in the worktree (adopted after surface-by-surface audit against the stewardship pins); 650d975b left a drafted band builder + `~/.cache` wall mirror (prose adopted with corrections: gates ride the ci.yml gates block beside release-pins, not a "static-gates"-named job; its "next mint 1504"/"rm-1444 taken by Docker Sandbox" claims were not reproducible and were replaced by fresh-census discipline).

## THE BATCH (uncommitted, 6 M + 4 ??)

- M `.github/workflows/ci.yml` — selftest + msrv-floor lanes wired beside release-pins/changelog-structure (ci.yml:99/:104)
- M `.github/workflows/release.yml` — existence probe (`gh release view`) before create; attestation gated; publish delegated to `scripts/release/publish-release.sh`
- M `README.md` (:584) + M `Cargo.toml` (:13-23) — MSRV floor documented
- M `scripts/release/render-channels.sh`, M `scripts/release/publish-homebrew.sh` — guard/echo harmonization (no-diff skip semantics preserved)
- ?? `scripts/release/publish-release.sh` — create-with-all-assets / upload-ONLY-missing-assets / no-op exit 0
- ?? `scripts/release/idempotency-selftest.sh` — 3 publish branches + E1–E5 guard wiring vs a PATH-shimmed gh
- ?? `scripts/ci/check-msrv-floor.sh` + ?? `scripts/ci/msrv-floor.txt` — dep-greatest floor 1.88.0 (time 0.3.53, time-core 0.1.9, ratatui 0.30.2 family; cargo_lock_md5 e1061f1b); E0658 `std::fs::File::lock` rides behind rm-166; verdict keep `rust-version` 1.89

## Validation outcomes consumed (NOT re-run this phase)

- **targeted 585c169f** (log `~/.cache/at-targeted-585c169f/battery.txt`): bash -n 6/6; workflow YAML parse 4/4; `check-{release-surfaces,release-pins,cargo-manifests,locked-cargo,changelog-structure,example-workflows,npm-files,install-ref-drift,msrv-floor}.sh` 9/9 rc0; selftest 10/10 rc0 (stability re-proof post-SIGPIPE-hardening).
- **full 06af0baf** (record `delegate/06af0baf…-scratch/full-validation-record.md`; clean-room `/var/tmp/at-ft-06af/`): 29 green lanes — `cargo test --locked` trio 59 suites **1078/0/0** (the 3 TMPDIR-sensitive tests pass under neutral TMPDIR), clippy `--all-targets -D warnings` rc0, `cargo +1.89 check --locked` rc0, release build rc0 (188 s, bin mtime == completion), actionlint run; digest `validation:v1:2f3dceff…` re-derived == dispatch.
- **4 pre-existing reds, all byte-proven at clean HEAD and absent from the batch diff:** (1) actionlint SC2001 `ci.yml:206` (line `changed: /` byte-identical at HEAD:204 and origin/master:204); (2) `check-plugin-version.sh` v0.0.4 CHANGELOG marker; (3) `check-install-runtime` test D (rm-051) — pinned npm ref live; (4) `cargo deny` 4× rustls RUSTSEC-2026-0346..0349 (0.23.46 published 2026-10-14 clears all four).

## Findings and follow-ups for the next cycle

1. **TMPDIR hermeticity (assess F1, re-mint arm):** `upstream.rs:261` + `insights.rs:1159/:1278` flip under TMPDIR-inside-a-repo hosts — three-way proof: TMPDIR=delegate FAIL / /tmp PASS / `GIT_CEILING_DIRECTORIES` PASS. Unfixed at frontier. Fold the reaped roadmap band's rm-087 (`--since` README truth) + rm-381/rm-382 (decode semantics) riders into the re-mint pass.
2. **rustls 0.23.46 one-bump deny-green** — cures red (4) outright.
3. **CC 2.1.299 org-identity seam:** codeburn #1756/#1757 (2026-10-13) + auth-rules `forceLoginOrgUUID` → multi-org quota rings unblocked (rm-535/rm-560 fog; cession recorded in the roadmap envelope).
4. **codex schema-forward tables:** spawn `thread_spawn_edges`/`sections`/`dynamic_tools` (log lines 3180..3219, 1 parse hit); rust-v0.163.0-alpha.11 shipped 2026-10-14.
5. **gemini 0.64.0 ACP `PromptResponse.usage` bridge** (2026-10-13).
6. **npm bare-name `agenttrace` registry 404** on two consecutive research passes → cheap namespace protection ($100 hold).
7. **`docs/workflow-issues/shared-cargo-target-dir-shadowing.md` never landed** (run f084f50e's worktree was reaped before its commit gate) — re-author from the fleet rule: private `CARGO_TARGET_DIR` for every battery; strings-probe stale-bin phantom failures.

## Prevention rules minted this cycle

- `docs/solutions/workflow-issues/never-assert-through-grep-pipes-sigpipe-141-races-in-harnesses.md` — SIGPIPE 141 races: 6/30 gate runs red on correct output → 0/90 with here-string asserts.
- **Mirror rule (fleet process):** every spool artifact gets a `~/.cache` mirror immediately — spool janitors reap `delegate/*-scratch` daily, and the one phase that skipped the mirror (roadmap 50db6611) is the one whose work was lost.

## Lost work — the reaped cycle-5 roadmap band

The roadmap phase's band (banner + rm-087/rm-381/rm-382 riders + mint rm-1444 "upstream.rs outside-git contract test flips under TMPDIR-inside-a-repo hosts", built against wall 8bc0587/1d59adda, round-trip proven) was reaped from the spool before application and never landed. Its mint rm-1444 is absent from the live wall (census 2026-10-15 07:56Z: 400 rows, def-max rm-1502, 0 dups; footer stale at rm-1377; spool fog references ≥ rm-1521). **Recovery:** full band content survives in the roadmap PhaseResult (`delegate/50db6611…json`) — re-mint at the next roadmap phase ONLY after a fresh live census, reconciling by title (rm-708 convention).

## Commit-gate seam notes

- Stage EXPLICITLY: the 6 M + 4 ?? batch + these 2 compound docs (??) — `commit -am` drops untracked files.
- Apply the ROADMAP compound patch **by ANCHOR** on the live wall (not raw context): banner before the first Open-items banner; riders after each row's CURRENT last rider — the wall's rm-169/rm-386 rows carry riders this base lacks.
- Implement authored NO Unreleased CHANGELOG bullets; `changelog-structure` is green without — decide at the gate whether to add release-notes bullets for rm-169/rm-386.
- Standing reds to disclose at the gate: the 4 pre-existing reds above.

## Prior cycle carry-in

`~/.cache/at-assess-5dd9fe76` (cycle-3 assess corpus) and `/var/tmp/at-ft-06af/` (full-battery clean-room, 6.4 G) remain for reuse; `/var/tmp` hygiene per fleet disk posture.
