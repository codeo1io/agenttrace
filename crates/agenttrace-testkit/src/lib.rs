//! rm-2201: per-invocation-unique temp paths for test fixtures.
//!
//! Generalizes the two in-tree precedents into one shared helper so no
//! test needs a fixed-name path under a shared tempdir again:
//!
//! * `otel_export.rs`'s rm-1065 `fixture_dir()` — pid + per-process
//!   atomic counter, which closed the class for that file alone;
//! * `discovery_contract.rs`'s `temp_root(prefix)` — pid only, which
//!   still let two invocations of the same test binary collide whenever
//!   the harness reran one binary's tests twice in one process.
//!
//! The pid separates concurrent *processes*; the monotonically
//! increasing counter separates *calls* (tests, and successive writes)
//! within one process. Fixed-name fixtures under a shared tempdir are
//! the documented cross-process flake class (`rm-2201`, repro: two
//! concurrent invocations of the same test binary, 13/30 pairs failed
//! on the pristine scheme); see
//! `docs/maintainers/test-flake-prevention.md` Rule 12.
//!
//! Dev-only crate: `publish = false`, referenced exclusively from
//! `[dev-dependencies]`; it ships in no production artifact.

use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

static SEQ: AtomicU64 = AtomicU64::new(0);

/// Returns `<TMPDIR>/<prefix>-<pid>-<seq>`, fresh on every call.
///
/// The directory is NOT created — callers keep their own
/// `create_dir_all`/`remove_dir_all` discipline; uniqueness alone is
/// the guarantee (a fresh name cannot collide with any other call, so
/// a leading `remove_dir_all` is a defensive no-op rather than a
/// mid-flight deletion of a sibling process's fixture).
pub fn unique_temp_path(prefix: &str) -> PathBuf {
    let seq = SEQ.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!("{prefix}-{}-{}", std::process::id(), seq))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unique_temp_path_is_unique_per_call() {
        // rm-1065 pin, generalized: the pid alone separates processes;
        // the per-call suffix must separate tests (and successive
        // writes) within one process.
        let a = unique_temp_path("at-testkit");
        let b = unique_temp_path("at-testkit");
        assert_ne!(a, b, "unique_temp_path() must return a fresh path per call");
    }

    #[test]
    fn unique_temp_path_preserves_the_callers_prefix() {
        // The prefix keeps fixtures greppable/attributable in /tmp
        // listings even after the pid+seq suffix is appended.
        let p = unique_temp_path("agenttrace-rm2201-sample");
        let name = p
            .file_name()
            .expect("path has a file name")
            .to_string_lossy()
            .into_owned();
        assert!(
            name.starts_with("agenttrace-rm2201-sample-"),
            "prefix must lead the generated name: {name}"
        );
        assert!(
            name.contains(&format!("-{}-", std::process::id())),
            "pid must appear in the generated name: {name}"
        );
    }

    #[test]
    fn unique_temp_path_distinct_prefixes_never_collide() {
        let a = unique_temp_path("agenttrace-rm2201-alpha");
        let b = unique_temp_path("agenttrace-rm2201-beta");
        assert_ne!(a, b);
    }
}
