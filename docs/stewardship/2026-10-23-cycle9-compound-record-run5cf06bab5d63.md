# Cycle 9 compound record — run 5cf06bab5d63 (repository-maintenance fe743fb3)

Date: 2026-10-23 · phase: compound · attempt chain: eda2dad0 (band + this record, reaped 03:39Z) →
75951d3b (corrected banner, reaped 06:56Z) → 93d38b87 (reaped 08:59Z, no durable residue) →
770a28a0 (adoption + completion) · wall 6b8c5c0a · worktree HEAD eae2237f (conductor-salvage) +
uncommitted completion.

## What cycle 9 selected and executed

Batch `host-state truth hardening` (prioritize 1d5fcc90, stewardship 6c4e589d), implemented by
declared adoption of salvage commit eae2237f plus a 4-file completion (implement da911101):

- **U1 — rm-1954 completion**: `with_isolated_pricing_env` (crates/agenttrace-core/tests/support/mod.rs:113)
  wired into the four last "dark" catalog-census suites — `svg_card_contract` (10 arms),
  `cache_rate_truth_contract`, `first_party_pricing_extract_contract` (8/9), `token_conformance`.
  Module rule ("Every census assertion in the catalog-lane suites must run inside this guard") is now
  fully satisfied at this tree. Default-lane `~/.cache/agenttrace/pricing.json` (4,245-model LiteLLM
  cache) can no longer silently replace the bundled 3,131-key snapshot in these suites.
- **U2 — .git-pointer read cap**: `insights.rs` `worktree_main_root()` (`find_git_root` :702 /
  `worktree_main_root` :723) previously slurped the `.git` pointer file with an uncapped
  `read_to_string`; it is now capped like the parser (`file_read_cap_bytes` parser.rs:103 precedent)
  and an over-cap `.git` pointer is treated as a non-pointer, not a slurp — regression-pinned by lib
  test `an_over_cap_git_pointer_file_is_a_non_pointer_not_a_slurp`.

## Validation record (consumed as recorded evidence; NOT rerun at compound)

- Targeted (307589b5, private cold CARGO_TARGET_DIR): catalog lanes 42/0 (13+3+11+4+2+9 across the six
  suites), core lib 487/0, fmt rc0, clippy `-D warnings` rc0, changelog-structure 396 entries rc0,
  docs-links rc0; engine validation_digest MATCH at base 6b8c5c0a, delta.surfaces=7, delta.testable=[].
- Full (3357c1db): ci.yml:134 verbatim `cargo test --locked -p agenttrace-core -p agenttrace-tui
  -p agenttrace` RC=0 — 95 suites, 1486 passed / 0 failed; fmt rc0; clippy 102 crates rc0; MSRV
  `cargo +1.89 check` rc0; 9 gate scripts rc0; conformance 13 cases / 1 xfail rc0; digest match pre
  and post, porcelain byte-stable through the phase.

## Roadmap state after this compound

- Cycle-9 band (roadmap b433f1d9, amended patch 7f99f1be) APPLIED: `rm-2341` upstream-sync (fork 6
  commits behind frozen upstream 15ed07f2), `rm-2342` ci-runner-migration, `rm-2343` v0.10.2-tag
  minted CANDIDATE. Census verified at completion: hat def rows 109→112, loose `^- id:` 487→490,
  0 duplicate ids, def-max `rm-2343`, next free `rm-2344`, managed footer last. Chain 89a696d9
  (pristine, 3,834 lines) → 1f1de902 (band postimage, 3,868; round-trip re-proven by 770a28a0:
  git apply band.patch on the pristine → byte-exact) → f41caee6 (compound postimage, 3,869 =
  postimage + this cycle's one-line compound banner). Pure insertion only (+35 vs HEAD: band +34,
  banner +1, zero deletions); APPLY-SEAM.md / CENSUS.txt live at delegate/770a28a0…-scratch,
  mirrored ~/.cache/at-compound-770a28a0/.
- The pre-existing "band NOT YET APPLIED at cycle-9 road wall" line and the cycle-9
  "unlanded flywheel" fog marker in the banner head are wall-time snapshots, superseded by the
  cycle-9 compound banner (newest-first top, first blockquote under `## Open items`). Correction
  (770a28a0): this record's earlier draft cited banner id `e3a97c42-6b0f1d85` and a `112→115`
  def-row census — both phantom (zero matching attempts in events/; the hat census is 109→112),
  first flagged by the banner's CORRECTION clause and fixed here.

## Reusable lessons / prevention rules

1. **Reap-salvage adoption beats blind redo.** Three consecutive reaped implement attempts
   (f22c8c96, 967ee375, 33fbdbb5) left no typed envelope, but the worktree carried a
   `conductor-salvage:` commit preserving exactly the batch — adopted and declared. Triage recipe:
   read the attempt's `events/*.jsonl`; `message_count >= 3` + scratch/worktree residue = salvageable;
   `message_count <= 3` with zero residue (de8f159e, 103391aa, 653f3c5f) = redo from scratch.
2. **Wall-stale selection must be diffed against tip before units are picked.** The wall 6b8c5c0a is
   4 behind origin/master 769453c3; candidate F9 (`zst_stem_twin_pairs`) existed only at tip and was
   correctly dropped as wall-stale. Always `git diff <wall>..<tip> -- <unit files>` first.
3. **Census-class claims need a hermetic per-phase CARGO_TARGET_DIR.** Shared target dirs and
   default cache lanes have both produced byte-identical wrong censuses on this fleet. Both validation
   phases used private cold lanes (at-targeted-307589b5, at-full-3357c1db).
4. **Engine digest folds base_sha** — the gate envelope's base:unknown digest will never equal the
   dispatch digest; declare the dispatch digest whenever porcelain is byte-stable (full_tests 3357c1db
   proved match at base 6b8c5c0a with the engine-derived value).
5. **Mirror-at-write-time.** The /tmp sweeper and the delegate-spool sweeper both strike mid-phase;
   every scratch artifact of this phase is mirrored to ~/.cache/at-compound-eda2dad0/ at write time.

## Candidates / context for the next maintenance cycle

- **PR#30** owns the master CI-red matrix (windows E0433/E0599 main.rs:5063/5065 +
  count_honesty_admission.rs:38 PermissionsExt; macos-14; release-binary-smoke; dependency-policy;
  test-and-build). Watch merge; cycle-9 batch files are disjoint from its 13 files.
- **rm-2342 runner migration dates**: macos-14 unsupported 2026-11-02 (imminent); ubuntu-latest→26.04
  completes 2026-11-19; ubuntu-22.04 brownouts 3× Saturdays March 2027, fails 2027-04-17. Workflows:
  ci.yml:24/117/176/225, release.yml:59-63, pricing-drift/scorecard/changelog/dependency-review/
  publish-channels.
- **rm-2343 v0.10.2**: annotated tag v0.10.2-pre = e9a1cbec (2026-10-17) sits unpublished; release.yml
  is tag-triggered; all 4 install channels stale at v0.10.1.
- **rm-2341 upstream sync**: origin/master..upstream/master = 6 (upstream luoyuctl frozen at 15ed07f2).
- **Hygiene candidates**: CHANGELOG.md 64 duplicated lines (spans 812-881==884-985 and
  900-969==972-1073); cargo-deny red RUSTSEC-2026-0381/0382; LiteLLM live catalog 4,244 keys vs
  bundled 3,131 (argon 4/0, sonnet-5 37/37, codex 9/2) — refresh via the sanctioned
  drift-check/UPDATE_CARD_GOLDEN channel.

## Evidence map

- Prior phases: typed records at /home/agent/.hermes/conductor-delegate-spool/results/
  {c601c1ab,9bb79e52,da911101,307589b5,3357c1db,6c4e589d}*.json (the delegate/ envelopes named by
  the earlier draft were swept; results/ is the durable store) + roadmap band mirror
  ~/.cache/at-roadmap-b433f1d9/.
- This phase: draft mirror ~/.cache/at-compound-eda2dad0/ (COMPOUND.md, APPLY-SEAM.md, notes — its
  spool scratch was swept) and completed-state artifacts at
  conductor-delegate-spool/770a28a071e74209adaa1c1f2cc4e7db-scratch/, mirrored
  ~/.cache/at-compound-770a28a0/ (CENSUS.txt, APPLY-SEAM.md, COMPOUND.md). The banner's original
  citation of delegate/75951d3b…-scratch / ~/.cache/at-compound-75951d3b/ dangled — that attempt
  was reaped ~1 min after inserting the banner, before writing them; amended by 770a28a0.
