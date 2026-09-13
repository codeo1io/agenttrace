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
