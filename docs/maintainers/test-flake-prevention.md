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

Case study (2026-10-01, campaign 2962e401 cycle 1 — the second-binary form):
`agenttrace-tui`'s helper `with_session_cache_dir_for_test` (`src/tests.rs`)
mutated `AGENTTRACE_SESSION_CACHE_DIR` with no lock — core's `lock_env()`
cannot help across crate boundaries because lib tests run as separate
binaries/processes. With a second user added to the helper, two tests
corrupted each other in both directions within one run: one read the other's
empty scratch root and asserted 0 sessions where it had written 1, while the
other was restored back to the REAL home cache mid-render and asserted on
host state. The fix is the same rule in the second binary: a per-process
`static SESSION_CACHE_DIR_TEST_LOCK: Mutex<()>` acquired inside the helper
and released only after the env restore. 6/6 consecutive full `--lib` runs
green, plus the workspace gate (271 tests) — and any future env-mutating
helper in this crate must go through the same lock, not add a third one.

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

## Rule 6 — Absence-asserting tests pin host state; borrowed hermeticity is a flake

A test that asserts the ABSENCE of output derived from host-stateful files
(`~/.cache/agenttrace/statusline.jsonl`, `sessions.json`, `pricing.json`) is
hermetic only if it points the state-dir env var at a scratch root itself.
Passing because a concurrently running env-mutating test (Rule 1 class)
happened to redirect the process env is **borrowed hermeticity**: the suite's
verdict then depends on thread scheduling alone, and a byte-identical tree
can score red one day and green the next on host state nobody changed.

Case study (2026-10-01, campaign 2962e401 cycle 1):
`efficiency_panel_renders_statusline_limits_and_cache_causes` asserted "no
journal means no statusline block" while the app lazily loads the REAL
journal on the first Efficiency render (`app.rs`, `ensure_governance` →
`load_statusline_insights()`). On a host that runs the statusline hook the
single test was deterministically red (rc=101); the full workspace suite
scored rc=101 and rc=0 on a byte-identical tree a day apart, purely on
whether the concurrent `ctrl_r_force_reload` test held
`AGENTTRACE_SESSION_CACHE_DIR` on its scratch root at render time. Pinned by
wrapping the test in `with_session_cache_dir_for_test` with an empty scratch
root (Rule 1's lock then applies), verified single-test-alone green with the
real home present and 271/271 at the workspace gate.
