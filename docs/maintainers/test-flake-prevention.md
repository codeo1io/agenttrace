# Test Flake Prevention Rules

Source of truth for keeping `cargo test --workspace` deterministic. Each rule
was paid for by a real incident; the case studies cite it. When a new flake is
root-caused, add the rule here before closing it.

## Rule 1 — Env-mutating tests serialize on the shared lock

Any test that sets or unsets process-global environment variables
(`AGENTTRACE_SESSION_CACHE_DIR`, `HOME`, `XDG_CACHE_HOME`, `XDG_CONFIG_HOME`,
…) must hold `agenttrace_core`'s `test_env::lock_env()` mutex for the entire
mutation window, and must save-and-restore the previous value rather than
unset it. Integration tests use their own `lock_env()`; do not add a second
lock inside a crate — one lock per process, and everything that touches the
same variables takes it.

Case study (cycle 7): `journal_roundtrip_tolerates_a_torn_tail_line` failed
roughly 1 run in 6 at the full-workspace scope. Three helpers — pricing's
`with_isolated_cache_env`, the session-cache stale-listings test, and the new
statusline journal test — all mutate the same variables with no
serialization. The fix (`#[cfg(test)] pub(crate) mod test_env` in
`crates/agenttrace-core/src/lib.rs`) made the flake disappear across 8
consecutive full runs, and the class re-appeared the moment a fourth suite
joined the race — which is exactly why the rule is a lock, not a comment.

## Rule 2 — Test-isolation paths are keyed by thread, not just process

`cargo test` runs tests as parallel threads of one binary. A `#[cfg(test)]`
temp base keyed by `std::process::id()` alone is shared mutable state across
those threads. Key it with `std::thread::current().id()` as well (see
`language_preference_path()` in `crates/agenttrace-tui/src/app.rs`), or
serialize access under a lock per Rule 1.

Case study (cycle 7): the TUI language-preference file lived under
`agenttrace-tui-test-{pid}/…`. A test toggling the language to Chinese wrote
`zh` while another thread's `App::new_loading` read the same file, so
`ctrl_r_force_reload_clears_session_cache_before_loading` asserted on an
English status string that had rendered in Chinese — roughly 1 run in 50,
and only under CPU load. Thread-keying the base closed it (18/18 loaded
runs green).

Rule 2's keying must also reach FIXED test roots (rm-429, 2026-10-04): a
root like `temp_dir().join("agenttrace-<item>")` with no pid/thread
component is shared mutable state across every `cargo test` invocation on
the host, not just this binary's threads. Live case: two conductor lanes
ran `cargo test` concurrently on one shared delegate host; the second
suite's `baseline_delta_pct_flags_reject_nan_and_negative_values`
deleted-and-recreated the first suite's `agenttrace-rm346b-baseline/`
mid-run, and the identical-control child failed `rc0 expected` with
`No such file or directory` on a clean HEAD — a pure false regression.
Every `temp_dir().join("agenttrace…")` in tests carries pid+thread
components (`agenttrace-<item>-{pid}-{thread:?}` — see the rm-301 root in
`crates/agenttrace-cli/tests/entrypoints.rs`); a grep probe for unsuffixed
fixed roots belongs in the same review pass as the doc-commands gate.

## Rule 3 — TUI background-worker tests draw before polling

TUI background workers (governance delivery, session loading) spawn when
their panel first renders — `ensure_governance` runs inside
`terminal.draw`, not on the view switch. A test that switches views and
immediately polls for worker output races the spawn. Always draw once, then
poll. Waits use a deadline plus `poll_pending_load`-style drain loops, never
fixed sleeps.

## Rule 4 — Reproduce flakes under artificial load, then read the code

Widen the race window instead of rerunning blind: run four CPU burners
(`while :; do :; done &`) alongside the test loop, capture each run to its
own log, and grep for `FAILED`. A 1-in-50 flake reproduces within ~10 loaded
runs. Then read the panicked assertion and walk both racing code paths to the
root cause — the first plausible suspect (here: a pre-existing `utf16` test)
was wrong, and the real cause was only visible in the shared-path code.

## Rule 5 — Evidence assertions require both sides and honest fixtures

Assertions about time-ordered evidence must require observations on both
sides of the boundary (a limit crossing is claimed only with usage before the
reset and the first observation after), and contract fixtures must be
schema-faithful to the producer's documented payload while being disclosed as
fixtures, not recordings — see `docs/guides/statusline-capture.md` for the
worked example. A test that passes by under-claiming is preferable to one
that passes by inventing evidence.

## Rule 6 — Pin the code's full env read-set, not the helper's hand-me-down list

An environment-faking helper is hermetic only for the variables it pins.
Enumerate the read-set from the code under test, not from habit:
`with_home_and_cache` (crates/agenttrace-core/tests/discovery_contract.rs)
pins `HOME`, `XDG_CONFIG_HOME`, `XDG_CACHE_HOME` and
`AGENTTRACE_SESSION_CACHE_DIR`, but opencode discovery also reads
`XDG_DATA_HOME` (crates/agenttrace-core/src/discovery.rs), so those tests
inherit whatever the *runner* exports — green on a bare shell, red under a
CI/delegate runner that sets it, on a tree nobody changed. When a suite's
verdict moves between runners, diff the runner env (`env | sort`) against
the helper's pinned set before suspecting the code; the durable fix is
pinning (or unsetting) the missing variable inside the helper, not a
"run without XDG_DATA_HOME" convention.

Case study (cycle 1, campaign 1f5ad3cf): the implement phase saw two
`discovery_contract` tests fail only under the delegate runner; `git stash`
at HEAD 9d88b36 reproduced the same failures on the untouched base, proving
the cause was inherited environment, not the batch. All subsequent gate
runs prepend `env -u XDG_DATA_HOME` (sandboxed `HOME` per the recorded
recipe); the cycle's full-suite envelope is result-1618525-330496923.json.
Kin to the sibling lane's Rule 6 (absence-asserting tests pin host state):
both are incomplete-hermetic-boundary flakes — this one enters through the
helper's variable list, that one through un-pinned host files. Numbered
Rule 6 campaign-locally; sibling campaign 2962e401 (run aa9c4fd6) holds its
own uncommitted Rule 6 in a parallel worktree — renumber at fold.

## Rule 7 — Fixtures that exercise host-observing code get unique roots, and pin the host state they need absent

When the code under test observes the live filesystem (decode probing,
discovery walks, anything that `stat`s a path it did not create), a fixture
under a runner-shared namespace such as `std::env::temp_dir()` inherits
whatever the runner's *host* keeps there: a host directory that
shadow-matches an intermediate fixture component flips the code's
observable result on a tree nobody changed. Two-part rule: (a) give every
such fixture a namespace no sibling test or host dir can collide with —
`unique_decode_root()` in `crates/agenttrace-core/src/insights.rs`
(temp_dir + `at-<pid>-<tid>-<seq>-<label>`, one per-process `AtomicUsize`
seq) — extending Rule 2's thread-keying from shared *mutation* to host
*probing*; (b) plant the host state you rely on the absence of, so the
suite re-proves immunity every run instead of silently depending on a
clean TMPDIR: the regression test creates the shadow sibling and asserts
attribution is unaffected.

Case study (cycle 2, campaign 6a10ae64, run cf755698): `cargo test
--workspace` was 110/1 FAILED on hermes delegate hosts
(`projects_group_worktrees_and_decode_agent_dirs`, insights.rs:563) and
green under `TMPDIR=/tmp`, because the host keeps
`/home/agent/.hermes/tmp/delegate/agenttrace` and the greedy project-dir
decoder probed that live-FS shadow first — the red was host state, not the
tree. After rm-381 (deterministic budget-bounded decode) plus this rule
(rm-382), the suite is 285/0 with the shadow deliberately planted during
validation, and the rc101 cascade in `check-rust-release-local.sh` (its
cargo-test step died before its own checks) vanished with it. Same
incomplete-hermetic-boundary class as both Rule 6s, entered through fixture
placement rather than env read-sets or absence-assertions. Numbered Rule 7
campaign-locally; sibling lanes hold their own uncommitted rules
(2962e401's Rule 6) — renumber at fold.

## Rule 8 — budget or isolate host-state-reading commands in gates (2026-10-03, cycle-2, run 0d487394)

`agenttrace --doctor -f json` (and any lane that walks the session
store) stalls past two minutes when `HOME` points at the grown real store
(~/.claude ≈253 MB + sibling roots on this delegate host) while
completing rc0 in under 30 s under an isolated `HOME`. This is a host
condition, not a regression: cycle-2 re-proved it at BASE by stashing the
batch and rebuilding 5ef66c0 — the base binary also times out (45 s, rc124
under `timeout`), so a red here indicts the environment, never the batch.
Same family as Rule 1's TMPDIR shadow (ambient host state leaking through
an un-isolated variable) and kin to Rule 6's host-state pinning.

Recipe (recorded in the cycle's full-suite logs): every gate that invokes
`--doctor` (`scripts/ci/check-docs-commands.sh` step 2,
`scripts/ci/check-rust-release-local.sh`) runs under a scratch `HOME` +
`TMPDIR`, a pinned `AGENTTRACE_BIN` (the worktree binary under test), and
a redirected `AGENTTRACE_CI_OUT`; gates needing the real CLI/TUI fixtures
set `AGENTTRACE_REAL_CLI_DIR` / `AGENTTRACE_TUI_REAL_DIR` — an isolated
`HOME` hides the default fixture paths, which cost the cycle's assess
sweep a false-red rerun. Gate budgets set accordingly (>120 s or
isolated). Numbered Rule 7 campaign-locally by run 0d487394
(`docs/stewardship/2026-10-03-cycle2-compounding.md` retains the
campaign-local numeral); renumbered to Rule 8 at its integration merge,
where Rule 7 was already taken by run cf755698's fixture-root rule above —
the cross-campaign collision convention both rules record.

