# Cycle 3 compound record — run 254b2417 (repository-maintenance 4cee1a0e)

Recorded 2026-10-06 by the compound phase (attempt 527789a187cd4cc5ada7943b99e1aa68) from
pre-review cycle evidence only: assess da490392, research ef54aef2, roadmap 2a268cb4,
prioritize 796d23ff, stewardship 0aae65cc, implement fb2c1ab7, targeted_tests e3e403e1,
full_tests d8ec71db. No validation was re-run at compound (phase contract); every outcome below
is the recorded pre-review outcome of the cited phase.

## Batch

- **rm-551 (LEAD, correctness 88.0)** — copilot checkpoint/shutdown reconciliation rewritten
  from snapshot-level to per-model freshest map (`struct CopilotModelSnapshot`,
  crates/agenttrace-core/src/parser.rs): a checkpoint/shutdown REPLACES values only for the
  models it names and PRESERVES every other model's last-known values; one meta event per
  observed model emitted post-loop with the last-naming timestamp. Shipped the
  documented-copilot-detection-fallback arm for the truncated-`session.start` shape (fixture
  `no-session-start.jsonl`); the disclosure-counter arm was not taken. Session-cache schema
  26 → 27 for the fold's slot history. Live PoC deltas on the built binary: `rot.jsonl`
  50,000/5,000 (one model dropped) → 1,050,000/105,000 both models, credit 0.02 held;
  `partial-shutdown.jsonl` 800,000/80,000 (o4-mini dropped) → 1,200,000/120,000, credit
  0.0095 held. NEW tests/copilot_checkpoint_reconciliation.rs (4 tests).
- **rm-552 (rider, dependencies 55.0)** — `cargo update -p lru` 0.18.1 → 0.18.5
  (RUSTSEC-2026-0253 UAF in range at 0.18.1 via ratatui 0.30.2); Cargo.lock diff exactly
  2 lru-isolated lines; OSV re-query at 0.18.5 → 0 vulns; CHANGELOG Unreleased line.
- **K1 (mechanical rider, no wall row)** — the 4 clippy `-D warnings` sites red at base
  6b03087 (parser.rs:240:34, main.rs:680:37/:686:37/:689:66) fixed as `needless_borrow`
  class; subject later minted as rm-592 on the 614624d7 wall and recorded in-flight in
  sibling cb38b958's lane — title-reconcile the remediation at landing.

## Recorded outcomes (pre-review)

- **targeted_tests e3e403e1** — 362/0 across nine focused suites. Verified per-suite:
  core lib 181, copilot_checkpoint_reconciliation 4, discovery_contract 81,
  zero_usage_contract 4, attribution_dimensions 8, parser_property_invariants 3, tui 47,
  cli entrypoints 30, launch_guards 4. (The implement summary's per-suite notes drifted on
  three counts — attribution "3"→8, parser_property "4"→3, zero_usage "2/3"→4; these are the
  verified numbers.) Dispatch digest `validation:v1:7d172c525350fbc3488453aa947e6ff62f019deb144ca0c5c5dd98545166d6e9`
  re-derived current with the engine's own `validation_digest` code before and after.
- **full_tests d8ec71db** — the work order's `full_command` was EMPTY (the crates/**
  classifier artifact: no path in this layout classifies, so the engine derives no command);
  the authoritative suite was executed from `.github/workflows/ci.yml` (the `full` job +
  the independent `deny` job + the lint-lane extras check-locked-cargo / check-install-ref-drift):
  21 lanes rc0, cargo test 487/0 across 24 binaries, release build rc0 (2m09s),
  ruby/npm/manifests/plugin-version(no tag fetch, PR-5)/script-syntax all rc0,
  `cargo deny --all-features check` rc0. check-rust-tui-real-smoke not applicable
  (ci.yml gates it on repo `vars.AGENTTRACE_TUI_REAL_DIR`, unset).

## Red-first catch and prevention rule PR-A

`scripts/ci/check-docs-commands.sh` FAILED before any fix: the governance-guide schema
sentence (docs/guides/governance-reports.md:72) still said "schema 26" after the 26 → 27
bump — the implement phase moved both discovery-contract pins and the TUI warm-cache fixture
but missed the guide sentence. Second recorded occurrence of this class (1st: run 99d1c79c
cycle-3, 25 → 26). One-word fix applied at the full gate; the gate is green in envelope
lane 12.

**PR-A (prevention):** the schema-bump same-unit set is enforced today ONLY by the CI lane.
A future S-effort batch should add a cargo-test pin (discovery_contract.rs kin) asserting the
guide's schema sentence matches `SESSION_CACHE_SCHEMA_VERSION`, so this lag fails at
TARGETED scope instead of surfacing only in CI. Recorded as a dated rider on rm-230's row.

## Prevention rules carried forward

- **PR-B (fleet numeral contest):** the rm-551 numeral is TRIPLE-HELD fleet-wide (this run,
  4ffc4fbb's pi-compaction fold, run-7f9c6d24's claude_code stream fold). Merge by TITLE,
  renumber by landing order (880a7b9e discipline); never assume an unlanded numeral is safe
  to mint without a live census.
- **PR-C (schema slot contest):** the fleet mainline records schema 27 already present at
  ee67b22 (4ffc4fbb review). If the merge head carries 27, this run's bump re-bases 27 → 28
  or accepts the ceiling under the one-invalidation rule; the whole same-unit set moves
  together (constant + slot-history comment + both discovery pins + TUI fixture + guide
  sentence).
- **PR-D (prior-attempt forensics):** targeted_tests attempt 6b462582 died provider-dead
  ~30s in (session_reaped) with zero durable work; the tree was verified byte-identical to
  the implement end-state before redoing the phase. Provider-death is a transport artifact —
  always verify the durable trail (typed result + event log + tree identity) before redoing.

## Next-cycle leads (concrete)

1. **rm-383 OSC-52 acceptance reopen** (strongest): live corpora already staged in
   delegate/da490392…-scratch/poc/ (osc.jsonl / osc.md / osc.html). Sanitize at the render
   boundary — text-overview group keys + markdown/html cells — not per-format callers.
2. **rm-230 PR-A rider** above (effort S).
3. **rm-553 Cowork discovery**: needs a macOS probe or a recorded defer-with-evidence;
   no Cowork store exists on this Linux host (checked, negative).
4. Watch-only: ccusage PR #1824 still OPEN (our fix is ahead of upstream; if it merges,
   fold its subtract_usage remainder semantics — cost-only `unknown` rows — as a rider on
   rm-551); K2 keyword-precedence stays sibling 2d92ee95's campaign-local rm-539/540 claim.

## Worktree census at compound end

12 paths: 9 modified (CHANGELOG.md, Cargo.lock, ROADMAP.md, docs/guides/governance-reports.md,
crates/agenttrace-cli/src/main.rs, crates/agenttrace-core/src/parser.rs,
crates/agenttrace-core/src/session_cache.rs, crates/agenttrace-core/tests/discovery_contract.rs,
crates/agenttrace-tui/src/tests.rs) + 3 untracked (crates/agenttrace-core/tests/copilot_checkpoint_reconciliation.rs,
crates/agenttrace-core/tests/fixtures/copilot-checkpoints/, this record). Done-flips for
rm-551/rm-552 remain reserved to the commit gate (rm-012 convention).
