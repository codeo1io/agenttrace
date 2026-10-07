# Cycle-3 compound record — run 90f54fafb294441abf7d7eb68c678398

- **Run:** 90f54fafb294441abf7d7eb68c678398 (repository-maintenance ac38807421014b1497aaeabf153aea07, cycle 3)
- **Base:** 89911442173d31f4eb21a1968bfd51ad46e32ed1 (merge landing b1ff12f8: rm-541..544 + rm-540 trunk), branch `conductor/run-90f54fafb294`
- **Compound attempt:** 6a95505861e2460c88fd8b5d3f5b704a (2026-10-07, after full_tests 7e64cb2a)
- **Status at compound:** implemented, pre-review. This record consumes RECORDED pre-review
  evidence only — no test/validation command was executed at compound, per the compound contract.

## Prior-attempt forensics (this run)

Four attempts were transport-reaped mid-run; every one left zero durable work to adopt
(each was redone from scratch, each redo verified the tree was un-drifted first):

| Phase | Dead attempt | Trail |
|---|---|---|
| roadmap | a6180cb7 | result JSON absent; events = progress pings only; worktree clean; reap 12:01:34Z |
| prioritize | edaae731 | result absent; events = turn_start + 1 ping; reaped ~26s in |
| stewardship | cec5a161 | schema-valid result JSON WRITTEN but reaped before Conductor consumed it (session_reaped + delegate_turn_completed status=failed @1791312682); the 34f95330 re-establishment consumed its JSON as evidence, not as a fold |
| targeted_tests | 66320068 | result absent; events = turn_start + 2 pings; reaped ~88s in; tree matched the implement envelope exactly |

Plus one fold-rejection (not a reap): implement **ced06cb6** completed the batch but its
fold was rejected SOLELY for the missing KTD13 `changed_surfaces` attestation; reattempt
**ca58403d** re-verified everything first-hand, touched nothing, and added the attestation
(declaring the adopted delta's 10-path surface set). No code defect in either.

## What shipped this cycle (pre-review, uncommitted)

**rm-623 — 'agenttrace mcp' local read-only MCP server** (wall priority 63, solo batch per
prioritize 8fd19c1c). Hand-rolled newline-delimited JSON-RPC 2.0 stdio server
(`crates/agenttrace-cli/src/mcp.rs`), ZERO new dependencies (decision recorded in the lane:
an MCP SDK adoption would be its own change-unit). Tools `usage_overview` +
`by_model_breakdown` render via the SAME core stack as the CLI (compute_overview /
report_overview_json_with_context / data_health_scoped — no second renderer, no core edits).
Keyword host dispatch beside statusline/upstream (rm-505 guard + keyword_help_text extended),
e2e `tests/mcp_server.rs` (handshake, both tools, every error arm, range scoping, empty
corpus isError, hermetic seeded HOME), `docs/guides/mcp-server.md` + README/README.zh-CN rows
+ a live-binary census block in `scripts/ci/check-docs-commands.sh` (keyword help + node-
verified `tools/list`), CHANGELOG Added bullet.

Worktree delta at compound: 7 modified + 3 untracked (see the compound c3 banner in
ROADMAP.md for the full path list); 155(+)/12(-) tracked before compound's own ROADMAP/docs
additions. No commits — the commit gate owns landing.

## Recorded validation outcomes (NOT re-run at compound)

| Phase | Attempt | Recorded outcome |
|---|---|---|
| implement | ced06cb6 → ca58403d | cargo test -p agenttrace 100/0 (7 suites); fmt rc0; clippy -p agenttrace --all-targets --all-features --locked -D warnings rc0; docs gate rc0 (incl. live census block, node-verified tools/list = `by_model_breakdown,usage_overview`); plugin gate rc0; live wire probe: initialize + tools/list + tools/call(by_model_breakdown, range=all) + bogus-tool error arm all correct over the real binary |
| targeted_tests | 82f678f8 | docs gate rc0; cargo build --bin agenttrace --locked rc0; cargo test -p agenttrace --locked 100/0; fmt rc0; clippy rc0; census artifacts produced (/tmp/agenttrace-ci/docs/mcp-help.txt, mcp-tools.json 1078 B) |
| full_tests | 7e64cb2a | fmt rc0; clippy --workspace --all-targets --all-features --locked -D warnings rc0 (0 diagnostics); cargo test --workspace --all-features 467/0 / 0 ignored across 20 test targets + 2 doc-tests; cargo deny --all-features check rc0 (advisories/bans/licenses/sources ok); docs gate rc0; plugin gate rc0 (plugin.json v0.9.0 == CHANGELOG v0.9.0, advances past merged tag) |

Digest lineage: `validation:v1:95b8aa32a158ca1857e8298f58e9dfc342adb1ba51937fa016c27faf4fa453ef`
declared VERBATIM at both validation folds; porcelain census 7 M + 3 ?? stable
implement → targeted → full → compound (no executable surface moved after implement;
compound touches ROADMAP.md + docs/ only — non-executable under the engine classifier,
per docs/solutions/process-issues/validation-digest-base-and-coverage-reconciliation.md).

### Disclosed gap (not-run lanes)

`validation.full_command` shipped EMPTY for this dispatch; full_tests ran the assess-phase
six-lane envelope (fmt / workspace clippy / workspace test / deny / docs gate / plugin gate)
— NOT the full ci.yml step mirror. The ci.yml-only lanes (npm test, ruby -c, release build,
check-deterministic-output, check-output-contract, check-report-semantics, real-cli-smoke,
check-locked-cargo) were not run this cycle. All are orthogonal to this delta (no workflow,
manifest, npm, release-surface change), but the commit gate or the next full checkout should
run the ci.yml mirror VERBATIM (standing precedent: 9cc6cab7's full_tests).

## Id accounting (live sweep at compound, 2026-10-07)

- ZERO ids minted at compound. This run's campaign-local band: rm-622 / rm-623 / rm-624
  (roadmap 3c860e0e @ 8991144).
- Landed ceiling MOVED during the run: origin/master 8991144 → 09cb224 → **61570ea**
  ("merge: autonomously integrate run de96d4cc…"), landed wall max **rm-682** — the
  integration robot landed many sibling bands (incl. the rm-616..621, rm-625+, rm-640s,
  rm-65x bands) while this run was in flight.
- Unlanded worktree diff ceilings (live): rm-686 (run-2f02ecaf), rm-655 (c90a0f00),
  rm-650 (f6a7e7f3), rm-646 (9a4d37af), rm-631 (aa31de94), rm-627 (a0d0d353), rm-624 (this
  run), plus lower bands. Spool records reach rm-698 (8e983cf5's roadmap postimage).
- Consequence: this run's mint band **rm-622..rm-624 WILL renumber by TITLE at integration**
  (202c4d1d dated-annotation convention — the mint note at the band records the mapping;
  these dated campaign lines keep the campaign-local numerals). Future mints re-census live;
  current fleet frontier ≥ rm-699.
- Wall accounting after the compound flip: 201 one-line def rows → **48 implemented /
  130 candidate / 37 done** status fields (rm-623 candidate→implemented with the compound
  evidence addendum; done flips reserved to the commit gate, rm-012 precedent).

## Commit-gate seams (re-verify live at landing)

1. Rebase onto 61570ea+ (or whatever origin/master is at gate time) — the base 8991144 is
   far behind; ROADMAP.md is the fleet-hottest file: resolve by KEEPING BOTH bands and
   renumbering this run's rows by title past the landed ceiling.
2. Hot code files: `crates/agenttrace-cli/src/main.rs` and `tests/entrypoints.rs` are touched
   by landed sibling lanes — re-anchor the `mod mcp;` declaration (:25 at 8991144), the
   `Some("mcp")` dispatch arm, the rm-505 dropped-flag guard literals, and `keyword_help_text`.
3. `scripts/ci/check-docs-commands.sh` (+24-line census block) is the single
   engine-executable surface in the delta — re-run the docs gate after rebase.
4. CHANGELOG entry rides the delta (1 Added under Unreleased, already written).
5. Re-run at gate: touched-crate lanes + the ci.yml-only gates listed under "Disclosed gap".

## Next-cycle context (cycle 4)

- **Lead candidate: rm-624** (plan/quota awareness, 58) — designated by prioritize 8fd19c1c
  once the MCP surface exists; it now does. Byte-identity A/B lock + golden pace fixtures
  per its acceptance.
- **rm-622** (agent-format expansion, 61) splits into per-format rows at the selecting
  roadmap phase; first lane Zed (both decode lanes already in-tree).
- Riders/watch (research 4db229a3): upstream luoyuctl frozen (tip 15ed07f2/#318 ureq-3
  cargo-group; radar #237/#236/#103 unchanged) — watch #318's port lane for the 32 MiB
  pricing-download-cap invariant; LiteLLM byte-flat at 4480 rows (0 intraday drift);
  OTel GenAI semconv still 0 tags (rm-229 trigger untripped); cc 2.1.291 changelog-only.
- Not-selectable ledger (verified at prioritize 8fd19c1c): F1/F2 green on 9873fc06's
  unlanded rm-545 lane; C1 green on d65f72c7's unlanded rm-583; C2 = 91833f02's rm-616;
  K3 = 84be17b3's rm-599; N1/N4 = aa31de94's rm-628/rm-630; K2 landed at origin (6048acc).
  Several of these lanes have since LANDED via 61570ea — next assessment re-verifies which
  claims remain live before any re-selection.

## Prevention rules (fleet-lettered; continuing from PR-P..PR-S in
docs/stewardship/2026-10-05-cycle1-compound-record-run27253dd5.md)

- **PR-T — the KTD13 changed_surfaces attestation, stated once as a standing rule.** The
  fold-rejection-for-missing-attestation class is now the fleet's single most-repeated fold
  defect (fac4fcc5; 85477a54→730cf5fb; 7bd9e41c; this run's ced06cb6→ca58403d). Rule: every
  implement/test fold populates `validation_evidence.changed_surfaces` from the engine-derived
  delta classification. On an attestation-repair reattempt that adopts prior-phase work and
  itself touches nothing, declare the ADOPTED delta's surfaces (ca58403d declared the full
  10-path set), never the empty this-turn set — empty is correct ONLY when no surface in the
  run's whole delta is attributable.
- **PR-U — empty `full_command` gets a named envelope plus a not-run disclosure.** When
  validation.full_command ships EMPTY, command authority falls to the repository's
  established battery, but the fold must NAME the envelope it ran and DISCLOSE every
  established lane it did not (this record: assess six-lane envelope; ci.yml-only lanes
  disclosed as not-run-orthogonal). A later phase needing the full mirror runs the ci.yml
  steps VERBATIM (9cc6cab7 precedent). An unnamed envelope is how a subset silently becomes
  "the full suite".
- **PR-V — every new CI-gate census block writes a named artifact.** Silent-on-success grep
  blocks (fail-only output) cannot prove from rc0 alone that the new checks RAN rather than
  skipped. The rm-623 mcp census block writes `/tmp/agenttrace-ci/docs/mcp-help.txt` and
  `mcp-tools.json` — those artifacts are what made targeted_tests' rc0 provable (the gate log
  itself prints nothing for the block). Follow the pattern for every future gate extension;
  node use in such blocks follows 8 existing gates + the pre-existing `:37` use in the same
  script (no new runtime dependency was introduced by rm-623).

## Verification appendix (how to check this record)

- `git -C <worktree> diff --stat` → the 7-file tracked delta + ROADMAP compound additions;
  `git status --porcelain=v1` → 7 M + 3 ?? + this file (?? docs/stewardship/…).
- `grep -n 'compound c3 (2026-10-07' ROADMAP.md` → the banner (newest-first among dated
  banners, above compound c2 b1ff12f8); `grep -n 'status: implemented' ROADMAP.md | wc -l`
  → 48; duplicate-id check and managed-footer-last check as ever.
- Validation logs (sweep-volatile /tmp): /tmp/at-targeted-82f6/ and /tmp/at-full-7e64/.
