# Cycle 2 compound record — run b1ff12f8 (2026-10-06, pre-review)

- **Run:** `b1ff12f83f8e46b2b8db5089caf62f6c` (repository-maintenance `4cee1a0e216c47d7bed0146d819beb90`, cycle 2)
- **Base:** HEAD `ea5c41e47b04a65aa8717f7097a12534e33628d6` (unchanged all cycle; everything below is an uncommitted worktree delta in `run-b1ff12f83f8e-b1ff12f8`)
- **Batch:** "Trustworthy accounting on hostile and evolved journals" — rm-448 (lead) → rm-451 → rm-450 → rm-449 (implement order; prioritize attempt 728c9223)
- **Status at compound:** implemented, pre-review. Review and shipping happen after this step; the next cycle's assessment carries them forward.

## Attempts and forensics

| Phase | Attempt | Note |
| --- | --- | --- |
| assess | ae486e43 | adversarial sandbox /tmp/at-assess-b1ff (overflow PoC, --waste format matrix, unbounded subprocess walk) |
| research | 3bd0c4c3 | wire-shape census (codex custom-tools), upstream drift, dual-id testdata, pricing freshness, pi corroboration |
| roadmap | ff7fa5b6 | minted rm-448..rm-452 (+35 ROADMAP lines), id-frontier sweep |
| prioritize | 728c9223 | batch selection doc in spool scratch |
| stewardship | 0f77094a | change-unit contract; forensics on dead attempt 39bffb00 (reaped, no envelope, no scratch) |
| implement | fac4fcc5 → repaired fold | KTD13 attestation repair: the fold defect was SOLELY a missing `validation_evidence.changed_surfaces`; implementation judged complete, not redone |
| targeted_tests | 9d729425 | 367/0 across the touched crates; digest re-derived == dispatch token |
| full_tests | 54eda020 (dead) → 52b44afc | 54eda020 transport-reaped mid-turn (event log: turn-start + 2×reap-fail + session_reaped, no work events; tree census md5 `a8994a01…` identical to the targeted_tests census, proving zero drift) → nothing adoptable, redone from scratch by 52b44afc |

**Dead-attempt pattern (recurring):** attempts 39bffb00 (stewardship) and 54eda020 (full_tests) both died of infrastructure/provider failures with no result envelope and no tree drift. The reliable discriminator is the porcelain census: an md5 of `git status --porcelain` matching the prior phase's recorded census proves the dead attempt changed nothing, so the phase is redone from scratch rather than hunted for phantom work. A present artifact is not proof of validity either — verify identity and census before adopting.

## What was implemented (uncommitted delta, 15 files +927/-81 + fixture dir)

- **rm-448 saturating by_task_type accumulator** — `crates/agenttrace-core/src/lib.rs` :1241-1256 (`saturating_add` over the `.max(0)` per-session clamp, rm-448 comment citing the governance.rs `add_context_session` precedent), analyze() clamp comment :733-737 extended to by_model/by_task_type; red-first regression `tests/attribution_dimensions.rs` (+94): two i64::MAX-pinned sessions in ONE bucket → no debug panic, release bucket = i64::MAX (not 0), summary-consistent.
- **rm-451 waste report honors `-f json`** — `waste.rs` `waste_report_json` (schema `agenttrace.waste.v1`, rm-004 allocated-cost caveat carried), `main.rs` waste-arm format match (json→data; text→renderer; markdown/html stay guard-rejected), re-export in lib.rs; format-matrix test `entrypoints.rs` (+83) pins rc + machine-parseability per text/json/markdown × waste cell; schema documented in `docs/guides/governance-reports.md`.
- **rm-450 bounded upstream subprocess waits** — `upstream.rs` (+275): `UPSTREAM_GIT_TIMEOUT` 10s local-probe class / `UPSTREAM_FETCH_TIMEOUT` 30s network-fetch class, bounded spawn/drain/poll/kill runner, `GitRunError{Timeout,Failed,Spawn}` with terminal errors naming the deadline and git operation; `docs/guides/upstream-status.md` documents both bounds; `scripts/upstream-delta` carries its dev-tool exemption note (operator-supervised, CI-capped).
- **rm-449 codex custom-tools parsing** — `parser.rs` (+310): custom_tool_call/custom_tool_call_output (latency pairing, payload.status failure surfacing, tool_usage feed), standalone reasoning (reasoning_blocks/chars), world_state ignorable counter, `codex_unmatched_response_item:<type>` disclosure for future wire growth (rm-401 widened-channel precedent); `SESSION_CACHE_SCHEMA_VERSION` 22→23 (rm-230 regenerate-on-semantics-change convention) with pins updated in discovery_contract.rs, tui tests.rs, and the governance-reports.md sentence the docs gate greps; fixture `tests/fixtures/codex-custom-tools/rollout.jsonl` (real 2026-09-26 rollout, sha256 `c32074ff8a709aa2415cd0d1bcef1acbf0d95ef77830a2569cf277b3ae3f05e2`, README-pinned) + contract tests (+165).

## Recorded validation outcomes (pre-review; consumed at compound, NOT re-run here)

- **targeted (9d729425):** 367 passed / 0 failed — core lib 161, discovery_contract 78, attribution_dimensions 9, cli 34+26+4+9, tui 46; `cargo clippy --offline --workspace --all-targets -- -D warnings` clean; `cargo fmt --all --check` clean; `scripts/upstream-delta --json` rc0 (base be25c4c, ahead 90 / behind 27, releases v0.9.1/v0.10.0/v0.10.1); shellcheck at its 2 pre-existing warnings only.
- **full (52b44afc):** the CI `full` ("Test and build") job's step sequence executed verbatim (19 steps, `/tmp/at-full-tests/run-full.sh`, log `full-suite-pass.log`) — ALL PASSED: step-3 `cargo test --locked` 378/0 across 13 test binaries, step-5 entrypoints 26/0, output-contract/deterministic-output/report-semantics/release-surfaces/install-runtime/docs-commands/real-cli-smoke gates green, ruby -c formula, npm test, cargo-manifests, plugin-version, script syntax, locked-cargo; supplementary deny lane (`cargo deny --all-features check`): advisories/bans/licenses/sources all ok.
- **Digest lineage:** dispatch token `validation:v1:0ad68958b52088dea80ac896a513d297dfce1963bd19a1acbbfcf84a6064e39e` declared VERBATIM by both targeted and full; re-derived with the engine's own function at both turn ends; no executable surface moved after implement (CHANGELOG markers and ROADMAP/docs are non-executable).

## Full-suite regression found + fixed (pre-existing at base, not batch-introduced)

`check-plugin-version.sh`'s per-tag arm (rm-303) was RED at ea5c41e on any full checkout: fork origin carries tags v0.7.2..v0.8.0 (all seven ls-remote SHAs are merge-base ancestors of HEAD) whose CHANGELOG sections never existed (heading jump v0.8.1 → v0.7.1), `git log -S 'no-changelog-section'` proves no marker ever touched this lineage, and an earlier lineage's identical remedy (full_tests attempt 96f0f58) never landed. Fixed with the gate's designed remedy: 7 one-line `<!-- no-changelog-section: vX.Y.Z: rationale -->` markers before `## v0.9.0` in CHANGELOG.md. **That marker block is a commit-gate rider on every landing until it merges** — see docs/solutions/workflow-issues/unlanded-ci-gate-remedy-recurs-on-every-full-checkout.md.

## Next-cycle context

- **rm-452** (upstream #305 Claude Code subagent attribution) is the designated cycle-3 lead: orphan-stub PoC corpus persists at `/tmp/at-research-b1ff/corpus` (claude-agent-*.jsonl), port source 4506b7e verified fetchable in-tree, landing coordinated with rm-232's request-id identity key.
- **rm-042 port wave:** upstream v0.10.0 (4aa6f07) ships `--daily/--weekly/--monthly --tz` + 5-hour `--blocks` — the 5-hour blocks collapse rm-042's candidate scope to a port.
- **rm-006 pricing refresh:** live LiteLLM snapshot 2026-10-05 = 4,472 models vs bundled 2,755 pinned 2026-09-13 (`/tmp/litellm-latest.json` persists).
- **Repro sandboxes (persist across delegate sessions):** `/tmp/at-assess-b1ff` (overflow corpus — expected GREEN under this delta; caches `/tmp/at-cache-b1ff`, `/tmp/at-cache-rel`, `/tmp/at-cache-perf`), `/tmp/at-research-b1ff` (custom-tools + orphan-stub + dual-id corpora; cache `/tmp/at-cache-r`), `/tmp/at-full-tests` (full-suite runner + logs), `/tmp/at-full-ci-out` (CI-script artifacts).
- **Watch:** upstream drift base be25c4c (2026-09-30), behind 27 with releases v0.9.1/v0.10.0/v0.10.1 live; every sibling worktree still fails the plugin-version gate until this marker block lands (see the prevention doc).
