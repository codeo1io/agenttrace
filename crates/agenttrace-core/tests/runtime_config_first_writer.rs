//! rm-684 (2026-10-07): the REAL first-writer contract for the
//! runtime-config overrides table. `set()` claims a process-global
//! `OnceLock`, so "first set() → true, second set() → false with the
//! first writer's values retained" is only deterministic in a process
//! where nothing else has claimed it first. An integration test IS
//! that process (its own test binary), which the in-crate unit test
//! cannot be — the in-crate shape previously degenerated to
//! `assert_eq!(current, current)`, which asserted nothing.

use std::path::PathBuf;

use agenttrace_core::{runtime_config_overrides, set_runtime_config, RuntimeConfigOverrides};

#[test]
fn first_set_installs_and_second_set_is_rejected() {
    let first = RuntimeConfigOverrides {
        history_dir: Some(PathBuf::from("/first-history")),
        pricing_file: Some(PathBuf::from("/first-pricing.json")),
    };
    let second = RuntimeConfigOverrides {
        history_dir: Some(PathBuf::from("/second-history")),
        pricing_file: Some(PathBuf::from("/second-pricing.json")),
    };
    assert!(
        set_runtime_config(first.clone()),
        "the first writer in a fresh process must install"
    );
    assert!(
        !set_runtime_config(second),
        "a second writer must be rejected, not overwrite"
    );
    let current = runtime_config_overrides();
    // The first writer's values are retained whole — both fields come
    // from the same writer, never a mix.
    assert_eq!(current.history_dir, first.history_dir);
    assert_eq!(current.pricing_file, first.pricing_file);
    assert_ne!(current.history_dir, Some(PathBuf::from("/second-history")));
    // The handle is stable across repeated reads.
    assert_eq!(runtime_config_overrides(), current);
}
