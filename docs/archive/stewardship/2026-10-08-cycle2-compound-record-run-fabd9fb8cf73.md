# cycle 2 compound record — run fabd9fb8cf73 (repository-maintenance 1ee0282008)

2026-10-08 · compound attempt 92367d36 · worktree run-fabd9fb8cf73 @ HEAD dc6564494edd99defa9b77fd55d0c48c3107c5e9
Pre-review evidence only (assessment → roadmap → prioritize → stewardship → implement → targeted → full). Review and shipping outcomes are recorded after this step and carried forward by the next cycle's assessment.

## batch envelope

'hostile-value truthfulness in parser usage folds' — one change-unit, one crate/file-family:

- **rm-831 (lead)** — Antigravity fold saturation (per-slot `saturating_add`, rm-046 contract) + negative insert gate in `usage_from_value_with_keys` (`>= 0`, alias-rescue) + workbuddy rm-600 clamp re-seed. Status `implemented`; done-flip reserved to the commit gate.
- **rm-721 (rider)** — sum-time non-finite copilot per-model credit guard with named disclosure (`copilot_credit_nonfinite` counter). Row already `done`; rider appended + validated.

Delta: 6 modified + 1 new fixture (+331/−5): `crates/agenttrace-core/src/parser.rs`, 3 test files, `CHANGELOG.md`, `ROADMAP.md` (mint band + row addenda), `tests/fixtures/copilot-checkpoints/agent-host-nonfinite-credit.jsonl` (untracked — **stage explicitly at the commit gate**).

## consumed outcomes (recorded, not re-run here)

| gate | outcome | digest |
|---|---|---|
| targeted 8a2121cb | 603/0 (core 462/0 + cli 141/0; fmt rc0; clippy `-D warnings` clean) | `validation:v1:d68dd6f920e99821903ad023492fa91483c56dd69657d0255a4b868c4cc83569` (VERBATIM) |
| full e5d41f54 | ci-mirror push suite **23/23 lanes rc0** 11:29:48Z; trio 654/0 = pristine 650 @ dc65644 + exactly 4 new; deny 4-checks ok; MSRV `cargo +1.89 check --locked` (floor read from Cargo.toml at run time); tree byte-immobile (whole-diff sha `b7c9ddf3fd244b0da2a6f497a708eaece4bda927`) | same digest re-declared at full scope |

Attempt trail: implement 4ca0aa26 provider-dead at message_count 2 (envelope absent, tree at dispatch state) → redone as d8b72dad. Targeted/full had no dead attempts.

## prevention rules (reusable lessons)

- **PR-1 — shared-path gates need a caller-contract sweep.** The negative insert gate in the *shared* extractor (`usage_from_value_with_keys`) silently broke workbuddy's *lane-local, stricter* rm-600 contract (clamp-to-zero-with-disclosure became absent-class). The colliding test lived **outside the batch's file-family**, so the red-first battery missed it; only the full trio caught it (`left: None, right: Some(0)`). Rule: when gating a shared parse path, enumerate every caller (`grep` call sites) and run each caller's suite before declaring done.
- **PR-2 — finite-at-insert ≠ finite-at-aggregate.** IEEE poison can be born at SUM time from perfectly finite inputs (1.5e308 × 2 → +inf). Guards belong at BOTH the insert gate and the aggregation site. Never use sign comparisons as finiteness gates: `inf > 0.0` is `true`.
- **PR-3 — insert-side `>= 0` vs read-side `> 0` is deliberate.** Explicit-zero populations (rm-408 client-reported zeros) attach through zero inserts; read-side consumers filter `> 0` at consumption. Aligning one to the other deletes a documented population.
- **PR-4 — refused values are NAMED, never vanished.** Hostile values that are dropped must surface on the disclosure channel (`disclosure_counters` via the bare-meta carrier) — the honest form of a refusal, mirroring the un-counted entry-type family (rm-450/rm-600/rm-616).
- **PR-5 — MSRV floor is read at run time.** The floor moved 1.88 → 1.89 at this base. Runners/lanes that derive the floor from `Cargo.toml` (as ci.yml does) never drift; hard-coded floors do.

## next-cycle context

1. **rm-833 is now UNBLOCKED** — it folds the `copilot_credit_nonfinite` disclosure counter, which landed this cycle. Prime candidate for cycle 3.
2. **rm-832** (repo-identity project grouping) unchanged; design-bearing.
3. **Pricing refresh + tier arm** (rm-006/rm-164) — lane-sized but needs a dedicated cycle; the 7d2af282 research dossier carries the fresh claude-haiku-5-5 pricing snapshot.
4. **Not minted, deferred with reasons**: serde 1-ULP float parse → unlanded spool rm-803 (7c90ea78 patch) + title-twin rm-825; ENV_LOCK private state → rm-804 (same patch); MCP currency → cross-wall rm-823. Implement ONCE by title at integration.

## fences

- DONE-flips for this batch are the commit gate's (rm-012 convention).
- Review-fix rows (if any) go below the cycle-2 compound banner in ROADMAP.md, newest-first.
- Sibling claims (rm-803/rm-804/rm-823) are NOT adopted by this cycle; reconcile by title post-rebase.
