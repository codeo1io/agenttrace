---
artifact_contract: "ce-handoff/v1"
created_at: "2026-10-01T21:19:42Z"
title: "Cycle 1 implementation record (campaign 2962e401) — truthful price at the point of use"
summary: "Records the rm-078 core batch end to end from pre-review cycle evidence only: what landed in 9 files, the recorded targeted and full-gate outcomes (consumed, not re-run), the two in-cycle defects found and fixed, the flake-prevention rules compounded from the full-tests incident, and the next-cycle anchors this cycle leaves behind."
keywords: ["agenttrace", "cycle-1", "rm-078", "pricing-provenance", "price-match-status", "dual-url-fetch", "test-hermeticity", "borrowed-hermeticity"]
cwd: "/home/agent/.hermes/conductor-worktrees/agenttrace-80c75f65b7/run-aa9c4fd68e5b-aa9c4fd6"
repository: "codeo1io/agenttrace"
repo_root_sha: "9d88b36750a991bd1436dbbc91b4579c39003067"
branch: "conductor/run-aa9c4fd68e5b"
head: "9d88b36"
---

# Cycle 1 implementation record — run `aa9c4fd6`

Campaign `2962e4019cad409ca9e08c4fe341026d` (repository-maintenance), cycle 1.
Worktree `run-aa9c4fd68e5b-aa9c4fd6` at HEAD `9d88b36` (fork master), branch
`conductor/run-aa9c4fd68e5b`. **Nothing is committed or pushed**: the whole batch
(+643/−62 over 9 files, plus this record and the flake-prevention additions) is
uncommitted and owned by the commit gate, per the delegation policy and the
campaign's roadmap precedent.

Phase lineage this cycle: assess `2d8f159a` · research `8b0c3149` · roadmap
(rm-078 minted, patch `2026-10-01-roadmap-rm-078.patch`) · prioritize `059ad85e`
(decision doc `2026-10-01-cycle1-prioritization.md`, spool) · stewardship
`de8d5015` · implement `45397946` · targeted_tests `855005b3` · full_tests
`16def3676` · compound `9a26b4c5` (this record). Review and shipping outcomes
happen after this step and are deliberately absent here.

## 1. What landed (rm-078 core — uncommitted, pre-review)

1. **Matcher-status plumbing** (`crates/agenttrace-core/src/pricing.rs`):
   `PriceMatchStatus {exact|variant|builtin_variant|default}` +
   `PriceSource {snapshot|override|builtin|default}` +
   `PriceMatchInfo {price, status, matched_key, source}`, threaded out of
   `lookup_price_in` / `matching_catalog_key` — the pair that already computed
   and discarded this status. Prices are byte-identical to before; only the
   provenance became observable.
2. **Dual-URL pricing fetch** (`pricing.rs`): `PRICING_URLS` tries the current
   canonical LiteLLM catalog first and the legacy URL as fallback, naming both
   on double failure. Grounding: LiteLLM renamed the catalog URL overnight
   2026-10-01 (legacy 404, canonical 200, sha256 `a1af9a18…`, live-probed in the
   research phase).
3. **data_health split** (`insights.rs`): `pricing_approximated`
   (builtin-variant pricing) separated from `pricing_default` (true default
   fallback) — previously one conflated `fallback_pricing` bucket.
4. **Priced-via naming on every surface that prints money**: text reports
   (`reports.rs`), `--overview` JSON (`lib.rs`), TUI snapshot (`explorer.rs`),
   governance panel (`presentation.rs`). Discloses only when the matched key
   differs from the displayed model — exact snapshot matches stay terse.
5. **Docs** (`README.md`): pricing-overrides aliases + `AGENTTRACE_PRICING_FILE`.
6. **Hermeticity fix** (`crates/agenttrace-tui/src/tests.rs`): the
   efficiency-panel statusline test pinned to a scratch root + the env lock
   (§4 below) — this is what made the workspace suite deterministic.

Live before/after (binaries from HEAD vs this tree, fixtures in the implement
log): `(model: <synthetic>)` silently builtin-priced → `(model: <synthetic>
(priced via builtin <synthetic>))`; with an override alias → `(priced via
override …)`; `--update-pricing` against a fake HOME loaded 1,519 prices and
wrote a 3,018,676-byte cache (the canonical-URL payload size), leaving the real
host cache untouched.

## 2. Recorded validation outcomes (consumed from the gates — NOT re-run here)

Compound does not execute tests; these are the recorded outcomes of the cycle:

| Gate | Outcome |
|---|---|
| implement targeted (`cargo test -p agenttrace-core`) | 106 lib + 7 demo_contract + 72 discovery_contract, 0 failed |
| implement targeted (tui filters) | 5 passed / 0 failed |
| `cargo fmt --all --check` | clean |
| `cargo clippy -p agenttrace-core -p agenttrace-tui -p agenttrace --all-targets -- -D warnings` | clean (CI lane `ci.yml:67`) |
| full_tests gate (verbatim `local_validation_gate.py --shell-command 'cargo test'`) | **rc=0, 271 passed / 0 failed / 0 ignored** across 10 suites (bin 20, entrypoints 10, launch_guards 2, upstream 9, core lib 106, demo_contract 7, discovery_contract 72, tui lib 45, doc-tests 0+0); envelope `result-1001075-329899709.json` |
| validation digest | `validation:v1:155d1e3d82ccd7bc7369ace33562cface32567490ee3cde875121137d0a6c738` at base `9d88b36`, re-derived over the post-fix tree (test-only files do not move the digest) |

## 3. Defects found and fixed inside the cycle

1. **Implement-phase review of the 429-orphaned code**: one defective new test
   fixed and one missing acceptance pin added before anything was declared green
   (implement attempt `45397946` verification log).
2. **The full-suite verdict was a scheduling race** (full_tests attempt
   `16def3676`): the efficiency-panel test asserted "no journal → no statusline
   block" while the app lazily loads the REAL `~/.cache/agenttrace/statusline.jsonl`
   on first Efficiency render. Alone with real HOME it was deterministically
   red; the full suite passed only when the concurrent `ctrl_r_force_reload`
   test happened to hold `AGENTTRACE_SESSION_CACHE_DIR` on a scratch root. A
   byte-identical tree scored rc=101 (2026-09-30 envelope) and rc=0 (2026-10-01)
   a day apart. Fixed hermetically; the first fix attempt then exposed the
   reverse race (two env-mutating tests corrupting each other bidirectionally),
   closed by serializing the env var — both documented as prevention rules (§4).

## 4. Prevention rules compounded

`docs/maintainers/test-flake-prevention.md` gains:

- **Rule 1, second-binary case study** — core's `lock_env()` cannot protect the
  `agenttrace-tui` test binary; every crate's env-mutating helpers need their
  own per-process lock (`SESSION_CACHE_DIR_TEST_LOCK`), one per binary, no
  third lock inside a crate.
- **Rule 6 (new) — absence-asserting tests pin host state**: borrowed
  hermeticity (passing because a concurrent test redirected the env) is a flake
  with a day-scale flip window; pin the state dir to a scratch root.

Campaign-side lesson (not repo code): the conductor validation digest's surface
classifier excludes test-only `.rs` files, so a test-only fix does not move
`validation_digest` — declare the dispatch digest verbatim after such fixes
(verified by reproduction against the engine's own policy, both gate envelopes
identical pre/post fix).

## 5. Next-cycle context (what cycle 2 inherits)

- **Top anchor — rm-020 "Price multi-model sessions per model"** (ROADMAP :180,
  the reliability item — a second, unrelated `rm-020` blocks-analytics block exists
  further down, see the integrity debt bullet):
  its deferral condition cleared (02993de2 merged) and this cycle built its
  primitive — the per-model ledger should consume `PriceMatchInfo` rather than
  re-derive match status. Anchor note updated in ROADMAP.
- **rm-021 "Stamp pricing provenance into artifacts"**: still sequenced behind
  run-1766ab4e rm-070's #103 `cost_provenance` JSON contract — do not mint
  fields the contract will rename.
- **Sibling-owned territories** (do not re-mint): tiered/price-class support
  (run-c7dca75d), models.dev second catalog (run-1766ab4e rm-071), snapshot
  refresh/drift (run-83642957 rm-062), first-party vendor priority
  (run-40208f3d rm-070). **Integration coordination**: the dual-URL
  `download_pricing` composes with c7dca75d rm-018's uncommitted
  cap/timeout/sidecar guardrails on the same function.
- **ROADMAP integrity debt** (assess finding, still open): rm-002..rm-027 exist
  twice (two campaign generations with overlapping IDs, e.g. two rm-020 blocks);
  settle with a renumber pass at an integration gate before the next research
  wall build.
- **Fleet note**: the statusline-journal race fixed here lives in sibling
  worktrees at older HEADs too — their cycles should take Rules 1/6 rather than
  rediscover them.

## Verification of this record

`git status --porcelain` = the 9-file batch (README, ROADMAP, 4 core files, 3
tui files) + `docs/maintainers/test-flake-prevention.md` modified + this file
untracked; no test command was executed during compound; every outcome cited
above carries its gate artifact (spool logs and gate envelopes listed in the
phase result).
