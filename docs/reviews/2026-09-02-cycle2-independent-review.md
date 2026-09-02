---
artifact_contract: "ce-review/v1"
created_at: "2026-09-02T12:10:00Z"
title: "Independent adversarial review — cycle 2 batch CU-1..CU-7 (trustworthy numbers on untrusted databases, and a launch that doesn't crash)"
summary: "Verdict: pass_with_findings. All seven change units are present and the implement/targeted/full-test evidence reproduces end to end (debug and release 169/169, fmt/clippy clean, 8/10 check scripts with the two failures being only the pre-existing absent-expect gap). Two medium residual defects found inside the batch's own new surface: the Hermes SQLite path still silently drops whole sessions on non-numeric typed values (the exact defect class fixed in-phase for OpenCode), and partial stored-total rows zero out derived columns while provenance overclaims. Neither blocks the cycle goals; both belong in the immediate follow-up queue."
keywords: ["agenttrace", "independent-review", "cycle-2", "sqlite-hardening", "stored-totals", "adversarial"]
run: "9fcc0661af474e2783a0dee7541f6ddb"
attempt: "73bced96a15448ed833387d53e18bcaf"
repo_root_sha: "e0059522b4fc74d53824f0e7ea7e4ac94d1465bb"
tree_state: "dirty (cycle-1 + cycle-2 uncommitted; nothing committed/pushed, per delegation policy)"
---

# Independent adversarial review — cycle 2 (CU-1..CU-7)

Reviewed against: the cycle goals as stated by
`docs/stewardship/2026-09-02-cycle2-prioritization-update.md` and the two
stewardship requests; `ROADMAP.md` acceptance criteria for the batch items
(H1/SQLite remainder, P4-1 TUI launch, P5-3 snapshot-date pin, P5-5
`.hermes/` ignore); the definition of done in the prioritization update;
security boundaries (read-only SQLite, offline-by-default, no new
dependencies, `.hermes/` never committed); durability/recovery (SQLite
snapshot cache schema v5, WAL/SHM fingerprinting, atomic writes, cache
clearing); and the test evidence claimed by the implement, targeted_tests,
and full_tests phases.

Method disclosure (standing gap, consistent with every prior phase of this
run): no ce-* SKILL.md is installed in this delegate environment; the
review ran in-thread using the repo's own adversarial-review conventions
(passes 1–5). Every finding below was **reproduced or re-derived from the
current tree this phase**; nothing is transcribed from earlier artifacts.

## Verdict

**pass_with_findings.**

The batch does what the cycle selected: the only HIGH on the board (SQLite
arithmetic) is closed with committed reproducers, the falsified cycle-1
closure claims (P5-1/P5-2/P5-3/P5-5) are repaired, and every load-bearing
piece of the implement/full-test evidence reproduces on this machine. Two
medium defects remain **inside the newly delivered surface** (M1, M2
below) plus four low/info items. None is a crash, a negative number, a
network touch, or a cache-durability regression, and none falsifies a
change unit's acceptance criterion as written — but M1 is the same defect
class the batch caught in itself mid-implementation and closed only on the
OpenCode side, so it should be first in the follow-up queue, not deferred
to a distant lane.

## Evidence re-verification (all run this phase, dirty tree at e005952)

| Claim (source phase) | Re-verified | Result |
|---|---|---|
| debug suite 169/169 (implement) | `cargo test --workspace` | **169 passed, 0 failed** |
| release suite 169/169 (implement) | `cargo test --workspace --release` | **169 passed, 0 failed** |
| fmt/clippy clean | `cargo fmt --check`; `cargo clippy --workspace --all-targets` | clean / **0 warnings** |
| 8/10 check scripts, failures only `expect` (implement/full_tests) | all ten with `AGENTTRACE_BIN=release`, `AGENTTRACE_CI_OUT=/tmp/at-review/ci-out` | **8 pass**; `check-rust-tui-real-smoke.sh` + `check-rust-release-local.sh` fail only at `command -v expect` (absent; pre-existing P4-5) |
| P5-1 overflow no longer panics | overflow.db through every report action, debug binary | `--overview/--audit/--context-trends/--recommend/--mcp-governance/--latest` all **exit 0**, no stderr; `tokens_input` pins at `i64::MAX`, cost finite positive |
| P5-2 u64 wrap saturates | wrap.db → `--latest -f json` (release) | `"input": 9223372036854775807` (was `-1`) |
| CU-2 stored totals win + delta | seeded DB (stored 1000/240/30/20, cost 0.5) | stored win, `stored_totals_delta: 720`; `data_health.stored_totals_sessions: 1`, `stored_totals_delta_tokens: 1290` on the message-less variant |
| CU-3 unknown time visible | `time_created = 0` DB under `--range 7d` (release) | visible; `unknown_time_sessions: 1`; huge `time_created = i64::MAX` degrades to unknown-time, not a fabricated date |
| CU-4 `--version` wins | `--lang fr --version` | **exit 0**, banner |
| CU-5 non-tty TUI | `--demo < /dev/null` | **exit 1**, `Error: stdout is not a terminal; ... use \`agenttrace --overview\`` (was 101 + backtrace) |
| CU-6 date pin red/green | sandboxed tree copy, const `2026-09-02`→`2025-01-01` | test **FAILED** (red) on mismatch; green on the real tree — the roadmap's "red/green run" evidence expectation, which no prior phase had actually executed, is now discharged |
| CU-7 `.hermes/` ignored | `git status --short` | 31 entries, **no `.hermes/`**; `.gitignore:13` carries the entry |
| cache durability v4→v5 | hand-downgraded snapshot to `schema_version: 4` | rejected, **regenerated as v5**, stored totals re-derived identically; WAL/SHM fingerprint invalidation covered by `session_cache.rs` unit tests |
| DB-mutation fuzz 0 panics | fresh harness `/tmp/at-review/db_fuzz.py`: 12 random corrupt DBs (both opencode and hermes shapes; values incl. `2**63-1`, `-(2**63)`, `2**64-1`, TEXT `'999'`/`'abc'`/`'1e5'`/`'18446744073709551615'`, REAL `1.5e19`, `None`, `""`) × 5 report actions, release binary | **60 runs, 0 bad** (no exit ∉ {0,1,2}, no panic/backtrace, no negative token fields in output) |
| Hermes negative columns clamped | seeded `state.db`, all four token columns negative | reported tokens all `0` ✓ |

Security-boundary spot checks: SQLite opened `READ_ONLY|NO_MUTEX`
(`sqlite_sessions.rs:113-117`, unchanged); no new dependencies
(`Cargo.toml` untouched; CU-5 uses std `IsTerminal`, MSRV 1.80 ≥ 1.70);
`pricing.rs` changes are test-only; no network path added; `.hermes/`
(prior-campaign plan with push instructions) can no longer ride into a
commit by accident.

## Findings

### M1 (medium) — Hermes ingestion still silently drops whole sessions on non-numeric typed values

`crates/agenttrace-core/src/sqlite_sessions.rs:184-197` reads every
Hermes column through strict `row.get::<_, T>()` conversions, and
`:206` discards failed rows with `rows.filter_map(Result::ok)`. A single
non-conforming value — TEXT `started_at`/`ended_at` (REAL affinity keeps
non-numeric text as TEXT), TEXT `'many'` in `message_count`
(INTEGER affinity likewise), or a non-numeric TEXT token column — fails
the row conversion and the **entire session disappears** from every view
with no `data_health` signal (it never reaches the parsed layer).

Reproduced this phase (debug binary, fresh `$HOME`, no cache):
`/tmp/at-review/mk_hermes_corrupt.py` variants —
`started_at='2026-01-01T00:00:00Z'` → `Error: No session files found`;
`ended_at='later'` → same; `message_count='many'` → same.
(`/tmp/at-review/hermes-text-start/`, `…/hermes2-text-endedat/`,
`…/hermes2-text-msgcount/`.)

This is the same defect class the implementation record describes fixing
in-phase for OpenCode ("a failed `get::<Option<i64>>` previously dropped
the entire session silently") — the lenient `sqlite_value_as_i64/_as_f64`
readers at `sqlite_sessions.rs:727-747` were applied only to
`opencode_sqlite_session_rows`. It also undercuts CU-3's own goal ("no
silent data loss in the same file and the same fixture family"). Fix is
small: route the Hermes reads through the same lenient helpers
(non-numeric → unknown-time / zero-clamp fallback).

### M2 (medium) — partial stored-total rows zero out derived columns and overclaim provenance

`apply_opencode_stored_totals`
(`crates/agenttrace-core/src/sqlite_sessions.rs:264-301`) triggers when
**any** of the five token columns is present (`:265-269`), then displaces
**all four** metric fields, substituting `0` for columns that are NULL
(`:281-296`, `unwrap_or(0)`).

Reproduced: opencode session row with only `tokens_reasoning = 77`
(everything else NULL) and a message carrying
`input:400/output:150/reasoning:10/cache 5/5` reports
`tokens: {input: 0, output: 77, cache_read: 0, cache_write: 0}` while
`cost.estimated` stays **derived from the 400-input message** (0.0036) —
tokens and cost now contradict each other, and
`provenance.tokens = "stored_session_totals"` describes data that was not
stored. `/tmp/at-review/oc-only-reasoning/`.

Per-column fallback (use stored when `Some`, keep derived when `None`)
would preserve the displacement semantics for fully-populated rows —
which is what upstream actually writes — while not destroying derived
information on partial/corrupt rows. The `stored_totals_delta` signal
does fire (−493 here), but a wrong number plus a visible delta is still a
wrong number.

### L3 (low) — stored `cost` is discarded when no token column is present

The early return at `sqlite_sessions.rs:270-272` exits before the
cost branch at `:303-308`, so a row with `cost = 0.5` and all token
columns NULL keeps the derived cost (reproduced:
`/tmp/at-review/oc-only-cost/` reports 0.0036). CU-2's request text is
"prefer OpenCode's authoritative session-level `cost` **and** five token
columns when present"; the cost preference silently doesn't apply on its
own. (Interaction with M2: fixing M2 per-column also fixes this — `cost`
should be applied whenever it is finite and non-negative.)

### L4 (low) — roadmap H1 acceptance not fully discharged by *committed* tests

`ROADMAP.md:127-149` requires "debug and release assertions over
`--sessions`, `--waste`, `--audit`, and `--context-trends` output" on the
adversarial corpus. The committed guards
(`discovery_contract.rs:46-98`) assert the **load path** only; the record's
release runs covered `--latest`/`--range`. I verified all report surfaces
exit 0 on `overflow.db` in debug this phase (see table), so this is a
coverage gap, not a live defect — but the acceptance line as written is
not met by the tree's tests. A small loop test over the four report
actions on both fixtures would close it.

### I5 (info) — two literal DoD checks read differently than intended

- The prioritization DoD says `grep "fn number_as_i64"
  sqlite_sessions.rs` → 0; `sqlite_sessions.rs:749` still defines a
  3-line adapter (different signature) delegating to the hardened
  `parser.rs:3582` twin — exactly what the stewardship request prescribed
  ("route through the hardened `parser.rs:3582` twin (adapter for the
  `Option<&Value>` signature)"). Substantively deduplicated; literally
  still one match.
- "all ten check scripts green" is environmentally impossible here
  (`expect` absent, pre-existing P4-5); 8/10 with the two failures solely
  at `command -v expect`, as the record states.

### I6 (info) — ROADMAP.md still describes the batch items as open defects

By design (separation hint 5: phase documentation ≠ implementation),
`ROADMAP.md:150-199` still narrates P4-1/P5-3/P5-5 and the H1 status
change as live. The final validation/commit gate should fold closure
status back into the roadmap so the next cycle doesn't re-select closed
items.

### I7 (info) — the unknown-time predicate now exists in three copies

`sqlite_sessions.rs:222-232` (`session_within_since`),
`discovery.rs:250-257` (retain closure), and `insights.rs:238-246`
(`session_matches_time_range`) implement the same "unknown start stays
visible" rule, and the first two use bare `parse_from_rfc3339` while
`insights` uses `parse_ts` (which additionally accepts naive datetime
forms, `lib.rs:1195-1206`). All three currently keep unknown-time
sessions (verified consistent), so no visibility bug today — but this is
exactly the divergence pattern the existing P3-3/P3-8 roadmap item
tracks; fold it in when that item is executed.

## What was checked and found sound

- **CU-1 arithmetic**: 9 saturating sites in `sqlite_sessions.rs`
  (incl. `:537-540` accumulators), `waste.rs:182` `saturating_sub`,
  `lib.rs:1091-1098` `total_tokens` saturating, `reports.rs` cross-session
  sum folded with `saturating_add` (+ regression test), Hermes negative
  columns clamped live, `governance.rs` confidence downgrades on any
  negative component (`cost_audit`, negative_components counter).
  No remaining plain `+=` token accumulator in the file.
- **CU-2 provenance/delta plumbing**: `Metrics.stored_totals_delta`
  round-trips the snapshot cache (`GoMetrics.StoredTotalsDelta`, schema
  v5, `session_cache.rs:1277`-family test pins v5 and rejects v4);
  non-finite/negative stored cost clamped away (verified with `9e999`,
  `-3.0`); REAL token values saturate (`1.8446744073709552e19` →
  `i64::MAX`); older schemas keep the derived path (committed test).
- **CU-3**: SQL predicates keep `null`/`≤0` rows on both sources; the
  post-load filters retain unparseable starts; `data_health` counts
  `unknown_time_sessions`; unknown-time sessions are excluded from
  earliest/latest anchors rather than fabricating a date.
- **CU-5**: guard sits before `ratatui::init()` in `run_with_app`
  (`app.rs:71-83`), the only `ratatui::init` call site; error exits 1 via
  the normal `main` error path; both callers covered; test exercises the
  real binary with piped stdout.
- **CU-6/CU-7**: as in the verification table.
- **Flake/cascade fixes from targeted_tests** (not part of CU-* but
  riding the same tree): `bump_dir_mtime` now loops until the directory
  mtime actually advances and panics loudly otherwise
  (`discovery_contract.rs:1931-1956`); the env-lock guards are scoped to
  drop before `resume_unwind` and tolerate poisoning
  (`:1875-1920`, `:1973-1979`). Release suite green this phase (the
  configuration that used to flake ~1/15).
- **CHANGELOG accuracy**: the corrected entry states the first pass
  missed the SQLite path and what this pass changes; behavior-change
  bullets match observed behavior (stored totals, unknown-time bucket,
  non-tty TUI error, `--version` precedence).

## Reproducers (kept for the follow-up implementer)

- `/tmp/at-review/mk_hermes_corrupt.py` + `/tmp/at-review/hermes-*`,
  `/tmp/at-review/hermes2-*` — M1
- `/tmp/at-review/mk_oc_partial.py` + `/tmp/at-review/oc-{only-reasoning,
  only-cost,zero-tokens,huge-real-cost,neg-cost,huge-real-tokens,huge-time}`
  — M2, L3, and the clean-saturation checks
- `/tmp/at-review/db_fuzz.py` — independent DB-mutation fuzz (60 runs)
- `/tmp/at-review/cache-mig/` — v4→v5 snapshot regeneration
- `/tmp/at-review/tree/` — sandboxed tree copy used for the CU-6 red run
  (const patched to `2025-01-01`; real tree untouched)

## Recommended follow-up ordering

1. M1 + M2 + L3 (one file, one function family; extends the committed
   fixture corpus to the Hermes shape and a partial-stored-totals case).
2. L4 (four report actions × two fixtures, committed as a test).
3. I6/I7 fold into the existing roadmap lanes at the next stewardship
   pass.
