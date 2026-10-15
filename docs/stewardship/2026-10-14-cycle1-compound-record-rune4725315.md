# Compounding Record — repository-maintenance `967ea98ab53243f2bcd84629342d0a1d` cycle 1

- Run: `e4725315c57d4b129211fed1126ba862` (delegate session, this document written by the
  compound phase, attempt `6222d64395f144398dad2bd79eb9f646`, 2026-10-14)
- Cycle chain: assess `32eb1c13` → research `f7739b6c` → roadmap `8ae91e99` →
  prioritize `7d698fb6` → stewardship `3c4627ac` → implement `18264cbf` →
  targeted_tests `c8b336f7` → full_tests `923b0a85` → compound `6222d643` (this document)
- Base: worktree `8bc0587` == `origin/master` tip (the b5d391ec landing); wall base
  ROADMAP md5 `1d59adda`, 392 def rows, def-max rm-1373.

## 1. Cycle ledger (no janitor losses this chain)

Every phase's authoritative JSON survived in the delegate spool (verified md5s during
compound): assess dossier, research dossier + 17 raw fetches (reaped /tmp copies
re-derivable from the named sources), roadmap band patch + postimage, prioritize memo,
stewardship request, implement batch.diff + test file, targeted/full battery logs.
Durable mirrors: `/home/agent/.cache/at-impl-18264cbf/` (implement delta).

## 2. Shipped batch — "cost-time truth" (rm-1500 LEAD + rm-1501)

Delta: 7 paths, +122/−16, uncommitted in worktree `run-e4725315` for the commit gate —
`lib.rs` (counter `Overview.non_finite_costs` :778; nine skip-while-counting fold gates
incl. by_agent :2558-2566 and the quota-window share :2864; `format_cost` null-arm `—`
:3144-3148), `parser.rs` (`timestamp_millis[_nanos]` `div_euclid`/`rem_euclid`
:7014-7030), `insights.rs` (SourceScope gate :813), `reports.rs` (text `⚠️  Non-finite
costs: N` + JSON field), `doctor.rs` (fold disclosures), `CHANGELOG.md`, and NEW
`tests/cost_non_finite_contract.rs`.

Both rows red-first (4/4 + 2/2 honest failures pre-fix), then green everywhere.

## 3. Recorded validation outcomes (compound re-ran nothing)

- targeted (c8b336f7): `cost_non_finite_contract` 4/0, `doctor_demo_contract` 2/0,
  `doctor_honesty_contract` 3/0, `report_numeric_truthfulness` 3/0, core `--lib` 406/0;
  fmt rc0; clippy `-D warnings` rc0. Tree proven byte-identical to implement's end
  (diff md5 `08fea4d4f1411261a07c55c131a0eb00`).
- full (923b0a85, ci.yml verbatim): fmt rc0; clippy lint-trio `--all-targets -D` rc0;
  `cargo test --locked -p agenttrace-core -p agenttrace-tui -p agenttrace` →
  **64 ok lines, 1135 passed / 0 failed** (62 test binaries + 2 doc suites).
  `cargo deny advisories` red — **pre-existing at HEAD**: RUSTSEC-2026-0346/0347/0348/
  0349 vs ring 0.17.12 (rustls chain); the batch touches zero Cargo files; fix =
  run-966bd5f9's unlanded Cargo.lock bump, a separate unit for the commit gate.

## 4. Compounded artifacts

1. ROADMAP band (roadmap phase): `delegate/8ae91e99…-scratch/ROADMAP.patch`
   (+30/−0; wall `1d59adda` → `49199da8`) — 3 mints rm-1500/rm-1501/rm-1502,
   10 evidence riders, banner.
2. Compound flip layer (this phase): `delegate/8ae91e99…-scratch/ROADMAP.compound.patch`
   (+4/−0, 3 hunks; `49199da8` → `1f1ab384`) — rm-1500 + rm-1501
   `candidate → implemented` with full evidence citations (implemented rows now 42).
   Chain verified byte-exact with `patch -F3` through both layers from a fresh
   `git show origin/master:ROADMAP.md`.
3. This record: `docs/stewardship/2026-10-14-cycle1-compound-record-rune4725315.md`.
- Land order for the gate: roadmap band + flip layer (one `roadmap:` commit is fine),
   then the code batch (own commit, `(rm-1500, rm-1501, run e4725315)` tag), then this
   doc rides the docs commit. Status vocabulary: cycle-complete-uncommitted = implemented.

## 5. Reusable lessons / prevention rules

- **Edit-call atomic rollback (wrong-file anchor):** one multi-edit call with a single
  oldText anchored in the WRONG file rolls back the whole call silently — re-anchor
  each edit against its own file and re-apply. Cost: one confusing re-run.
- **Clippy package names ≠ dir names:** `crates/agenttrace-cli` builds as package
  `agenttrace`; the CI lint trio is `-p agenttrace-core -p agenttrace-tui -p agenttrace`.
  `-p agenttrace-cli` does not exist.
- **None-scope dispatch digests are block hashes, not tree fingerprints:** when the
  validation block carries `changed_testable_surfaces: []`, the digest is derived from
  the block itself and is identical across such dispatches. Copy VERBATIM only when the
  turn changed nothing executable (held twice this cycle).
- **LiteLLM variant census lens:** the tiered-pricing axis is per-key SUFFIXED FIELDS
  inside model records (`input_cost_per_token_priority/_batches/_flex/_ultrafast` —
  399/242/242/8 models live); top-level key-name suffix scanning finds ZERO. Censuses
  and coverage gates must scan field names (one false census was caught this cycle).
- **GitHub API 301 discipline:** moved repos answer 301 with a JSON body that jq parses
  as `null`; fetch tooling must `-L` and fail loudly (sst/opencode → anomalyco/opencode,
  ryoppippi/ccusage → ccusage/ccusage).
- **Fog seam protocol that worked:** file-level dirty census → per-worktree
  `git diff -U0 | grep '^@@'` hunk census → region-level clearance; declare the
  ceded-adjacent seam explicitly and reconcile by TITLE at the gate (lib.rs:2716/2717
  vs the org runs this cycle).

## 6. Context for the next maintenance cycle

- **rm-1502** (ASCII-only case folding) deferred — document-or-Unicode decision +
  red-first pin; search.rs was contested ×2 in this cycle's fog.
- **rm-451 bounded reads** now has an upstream fix shape to vendor: openai/codex
  `5670b1c` `read_recent_rollout_items` over `ReverseJsonlScanner` + seekable reader
  (bounded reverse scan; `codex-rollout` NOT on crates.io — vendor the pattern);
  claude-code 2.1.299 shipped the same-class memory fixes. Design note: admission
  bound at discovery + reverse/tail scan for recency lanes.
- **models.dev `cost.tiers`** flipped live: 739 models / 780 entries / 27 distinct
  service-class types (top-level `tier` gone). rm-845's refutation clause is stale;
  the tier-matrix second-source decision needs re-running against the live surface.
- **Advisories lane** red at HEAD (4 RUSTSEC vs ring/rustls): adopt run-966bd5f9's
  unlanded Cargo.lock at the gate as its own unit — never bundle with a code batch.
- **Upstream watches:** ccusage v20.0.32 staged as open PR #1885 (k2p6 float fix
  #1883 rides inside); opencode v2.0.29 version-sync landed while releases/latest
  still v1.19.0 — 2.0 cutover shipping, packages restructured to `packages/core`
  (path-keyed discovery must key on the DB schema, not layout); gemini-cli v0.64.0
  STABLE ships append-only delta patching + bounded history windowing (#29568) —
  rm-715's `$patch` arm is now a stable-format requirement; claude-code 2.1.299 idle
  compaction (~54 min idle, before cache expiry) will appear in transcripts as new
  compaction artifacts (rm-422's classification lane).
- **Next free numeral:** rm-1503 after a fresh live census (fog ceiling at this
  compound: rm-1504 seen 13:59Z; reconcile by title, never renumber).
