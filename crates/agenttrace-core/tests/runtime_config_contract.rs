//! rm-800 (minted campaign-locally as rm-753 by run bbde21568cd4; rebound
//! at integration 2026-10-08, conflict case 615546e27c2b4f2d86e02b511f23ddd1
//! — the rm-753 numeral is held by the landed rm-753 SQLite row-drop row):
//! the runtime-override table's write-once contract, pinned in
//! a fresh-process cargo target. This binary owns its process's table
//! exclusively -- this file is the only caller of `set` in it -- so
//! "did the first writer install" is deterministic (always true) and
//! the first-writer-wins assertion can never be silently gated off,
//! which is exactly how the old in-crate test could pass vacuously.
//!
//! Keep this the ONLY `set` caller in this target: a second test here
//! would race for the table and reintroduce the order dependence the
//! move to a fresh process removed.

use agenttrace_core::{
    runtime_config_overrides as get, set_runtime_config as set, RuntimeConfigOverrides,
};
use std::path::PathBuf;

#[test]
fn first_writer_wins_in_a_fresh_process() {
    let first = RuntimeConfigOverrides {
        history_dir: Some(PathBuf::from("/tmp/first")),
        pricing_file: Some(PathBuf::from("/tmp/first.json")),
        // rm-298 capacity arm added the third knob after this target
        // landed; unset here so the write-once assertions below stay
        // about the table mechanism, not any one knob's value.
        session_cache_entries: None,
    };
    let second = RuntimeConfigOverrides {
        history_dir: Some(PathBuf::from("/tmp/second")),
        pricing_file: None,
        session_cache_entries: None,
    };
    // Fresh process, sole writer: the first set() installs.
    assert!(
        set(first.clone()),
        "fresh process: this test is the table's first writer"
    );
    // Write-once: the second set() is rejected. rm-800's red-first
    // mutation (set() reporting every writer as the winner) fails
    // exactly here, while the old tautology test stayed green.
    assert!(
        !set(second),
        "second set() must be rejected: the table is write-once"
    );
    // The winner is what get() reports -- never the rejected writer.
    let current = get();
    assert_eq!(*current, first);
    assert_eq!(current.history_dir, Some(PathBuf::from("/tmp/first")));
    assert_eq!(current.pricing_file, Some(PathBuf::from("/tmp/first.json")));
    // The handle is stable across calls.
    assert_eq!(get(), current);
}
