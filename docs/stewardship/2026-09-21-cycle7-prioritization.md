# Cycle-7 prioritization (phase: prioritize)

Run `4e6ff52433d44aff92a85afa14400a58`, attempt
`96bf435a70ad440eab1bab105f8bbe55`, 2026-09-21. Inputs: roadmap state
after this run's assess/research/roadmap passes (`ROADMAP.md`,
+106/−9), the cycle-7 assess findings (file/line-pinned), the research
pass's first upstream drift event (`a34dea2` #283 TUI, `6848aa1` #284
Oh My Pi, both merged upstream 2026-09-06/11 past merge-base
`e005952`), the cycle-5 independent review
(`docs/reviews/2026-09-03-cycle5-independent-review.md`, F5-1..F5-7),
and the live in-flight tree. Router note: the compound-engineering
router remains an empty stub in this environment (consistent with every
prior phase this run); work proceeded directly with file evidence.

**In-flight tree accounting (new this cycle).** The working tree at
HEAD `df3b621` carries an uncommitted remediation batch implementing
F5-1..F5-5 of the cycle-5 review plus a cache-isolation flake fix
(run-`5d025d55` open observation): `main.rs` (F5-1 sampling disclosure
`:262-267` now names the `--sort/--order` view, F5-3 `--sample`
rejection `:161-175`, F5-4 shared coverage line `:617-626`),
`insights.rs`/`data_health_scoped` (F5-2 source-unit counting),
`session_cache.rs` (F5-5 headerless-entry eviction, oldest-first),
plus tests in `entrypoints.rs`, `discovery_contract.rs`, `session_cache.rs`
tests, and `agenttrace-tui/src/tests.rs`. Verified this pass:
`cargo test --workspace --quiet` exits 0 (suite green with the batch
in). Stewardship gap found: the review's disposition ("file F5-1/F5-2
as cycle-6 hardening items") was never executed — `ROADMAP.md` contains
zero `F5-*` references; the batch also has no implementation record.

Method: every open item scored on impact (correctness/truthfulness/
user-visible behavior), risk of staying open, effort re-verified
against current source, dependencies, and strategic value. Selected =
highest total that forms one coherent, end-to-end completable batch.

## Scoring matrix (top of the queue)

| Item | Impact | Risk open | Effort | Deps | Strategic | Score |
|---|---|---|---|---|---|---|
| Land in-flight F5-1..F5-5 batch + records | 4 | 5 (stalled debt, proven) | S (impl done; verify/record) | none | 4 | **17** |
| Oh My Pi drift port (`6848aa1`) | 4 (live pi sessions fail to parse) | 4 (parser-superset parity) | S (upstream reference exists) | none | 4 (drift intake before next fork PR) | **16** |
| Saturating arithmetic remainder (cycle-7 assess) | 3 | 3 (debug panics/release wraps) | S | none | 2 | **11** |
| `flag_takes_value` boolean shim | 3 (silent arg swallowing) | 3 | S | none | 2 | **11** |
| Cargo publish metadata | 2 | 2 | XS | none | 2 | **7** |
| Windows `HOME`/`USERPROFILE` resolver (P3-1 remainder) | 5 | 4 | L (4 resolvers + per-OS matrix) | Windows env | 4 | 15* |
| Hermes `tool_calls_ok` fabrication | 3 | 3 | M | `state.db` schema research | 2 | 8* |
| Fork dependency-review false red | 3 | 3 | S-M | workflow edit + observed fork PR | 4 | 10* |
| Candidate 51: pricing snapshot age | 2 | 2 | S | none | 2 | 6* |
| Dependabot folds #278/#279/#259 | 2 | 2 | XS-S | none | 2 | 6* |
| Candidates 53/54 probes (ACP, VS Code logs) | 3 | 1 | M | research spike | 4 | 8* |
| Candidate 3: budgets/pace | 4 | 3 | M-L | history infra (present) | 4 | 11* |
| Pass-9 TUI reload race | 1 | 1 | S-M | none | 1 | 3 (ride-along) |

(*) scored but not selected — rationale below.

## Selected batch — cycle 7: "finish the truth-telling debt, hold
parser parity with upstream"

Theme: every shipped surface's honesty claim becomes verifiably true
(the stalled F5 debt), the parser keeps its superset parity with
upstream's first drift, and the two cycle-7 assess correctness findings
land while they are still one-file changes. All items are locally
test-verifiable, mutually independent, need no schema research, and
touch no CI workflow or API shape (the F5-7 JSON-shape decision stays
recorded and untouched).

- **CU-24 — Land the in-flight F5-1..F5-5 remediation batch.**
  Implementation is already in-tree and green (suite exit 0 verified
  this pass); this cycle supplies what the stalled run lacked: each
  F5-n acceptance from `docs/reviews/2026-09-03-cycle5-independent-review.md:136-236`
  verified against live source, roadmap entries filed (the missing
  disposition), a stewardship implementation record mapping finding →
  fix → test, and the release-notes ride-along. F5-6/F5-7 remain
  recorded decisions (re-confirmed unchanged). Acceptance: all five
  findings closed with named evidence; `grep 'F5-' ROADMAP.md` no
  longer empty; suite green.
- **CU-25 — Oh My Pi leading non-session lines (upstream `6848aa1`).**
  Skip-or-record leading non-`session` objects before the header
  requirement at `parser.rs:1325`/`:1400`, matching upstream semantics;
  drift census naming `a34dea2` (TUI — port or consciously decline)
  and `6848aa1` (ported) in the stewardship log. Acceptance: a fixture
  whose first JSONL object is a non-session line parses to a session;
  unit test pins the skip; census recorded.
- **CU-26 — Saturating arithmetic — parser accounting remainder.**
  `token_usage_delta` (`parser.rs:2304`) moves to saturating
  subtraction with negative deltas clamped or flagged (provider total
  resets); `add_usage` (`:3423`) / `add_usage_value` (`:3429`) move to
  saturating adds; the arithmetic downstream of
  `codex_token_count_usage` (`:2244`) audited. Acceptance: an
  `i64::MAX`-field fixture parses without panic in a debug build; the
  audit list lands in the stewardship record.
- **CU-27 — Go-flag shim boolean misclassification.** Remove
  `--no-baseline-gate` from `flag_takes_value`'s value list
  (`main.rs:753`); add a contract test pinning the shim's value-flag
  set against clap's definitions so the tables cannot drift.
  Acceptance: `--no-baseline-gate --overview` parses as `--overview`
  (today the shim swallows it).
- **CU-28 — Publish metadata ride-along.** `Cargo.toml:15-16`
  `repository`/`homepage` name the publishing remote
  (`codeo1io/agenttrace`) or the pin is recorded with a reason, so
  crates.io/docs.rs links stop pointing at `luoyuctl/agenttrace`.

Batch gate at implement's end: full workspace suite (baseline green,
exit 0, 2026-09-21), `cargo fmt --check`, `cargo clippy
--workspace --all-targets`, and the per-item evidence above.

## Deferred, with rationale

- **Windows resolver (15*)** — the highest-impact open item, but L
  effort across four differently-shaped resolvers plus a per-OS test
  matrix; it deserves a dedicated cycle (cycle-8 headliner) rather
  than sharing one with the F5 landing.
- **Hermes `tool_calls_ok` (8*)** — blocked on `state.db` schema
  research (does any error/status column exist?); starts as a research
  sub-task next cycle, not blind implementation.
- **Fork dependency-review false red (10*)** — the fix is small but
  its acceptance requires editing `.github/workflows` and observing a
  real fork PR's checks; that belongs to a cycle's PR/CI stage, and
  observing takes a live PR.
- **Candidate 51 snapshot age + dependabot folds** — ride-alongs on
  the next pricing/bump touch; no user-visible harm while parked.
- **Candidates 53/54 probes** — research spikes (next research pass),
  not implementation work; both accept dated negative findings.
- **Candidate 3 budgets/pace** — stays sequenced after the hardening
  lanes clear, per the cycle-6 prioritization; unchanged.
- **TUI reload race** — INFO; ride-along on the next TUI touch only.

Selection integrity: the batch closes one MEDIUM live parse failure,
one MEDIUM sampling-honesty defect (F5-1, stalled since 2026-09-03),
three LOW F5 riders, one MEDIUM arithmetic-robustness finding, one
MEDIUM CLI correctness finding, and one metadata defect — while
deferring every item whose acceptance cannot be verified locally this
cycle.
