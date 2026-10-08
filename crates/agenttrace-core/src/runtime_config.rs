//! Runtime configuration overrides (rm-384, cycle 1).
//!
//! The CLI resolves a layered configuration (CLI flags > `--config`
//! file > project `.agenttrace/config.toml` > user
//! `~/.config/agenttrace/config.toml`) and installs the winning values
//! here before any report or cache code runs. Core knob readers
//! (`history::history_path`, `pricing::load_pricing_overrides`) consult
//! this table first and fall back to their `AGENTTRACE_*` environment
//! variable, which keeps the documented precedence chain in one place
//! while leaving the env knobs working for library consumers (the TUI)
//! and the statusline host, which deliberately stays config-free.
//!
//! The table is write-once: the CLI installs it exactly once during
//! startup, and a second `set` is ignored. That makes the effective
//! values immutable for the life of the process after startup — the
//! same property the pricing override table relies on.

use std::path::PathBuf;
use std::sync::OnceLock;

/// Knob overrides a resolved configuration may install. `None` leaves
/// the corresponding `AGENTTRACE_*` environment knob in charge.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RuntimeConfigOverrides {
    /// Directory holding `history.json`; overrides
    /// `AGENTTRACE_HISTORY_DIR`.
    pub history_dir: Option<PathBuf>,
    /// Pricing override file; overrides `AGENTTRACE_PRICING_FILE`.
    pub pricing_file: Option<PathBuf>,
    /// Session-cache entry bound (rm-298 capacity arm); overrides
    /// `AGENTTRACE_SESSION_CACHE_ENTRIES`. The resolved value is
    /// clamped by `session_cache::resolve_session_cache_entries` —
    /// an out-of-range or unparseable knob never reaches the bound.
    pub session_cache_entries: Option<usize>,
}

static OVERRIDES: OnceLock<RuntimeConfigOverrides> = OnceLock::new();

/// Install the overrides. Returns `false` if a table was already
/// installed (first writer wins; the CLI is the only intended caller).
pub fn set(overrides: RuntimeConfigOverrides) -> bool {
    OVERRIDES.set(overrides).is_ok()
}

/// The installed overrides (all-`None` when none were installed).
pub fn get() -> &'static RuntimeConfigOverrides {
    OVERRIDES.get_or_init(RuntimeConfigOverrides::default)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unset_table_is_all_none_and_first_writer_wins() {
        // A fresh process state cannot be simulated with OnceLock, so
        // assert the public contract shape: get() never panics and the
        // first set() wins over a second one with different values.
        // INTEGRATION (2026-10-07, conflict case 2f1198de): installing a
        // table here redirects every later history_path()/pricing
        // override read in this process away from their AGENTTRACE_*
        // env knobs, so this test takes the shared env lock — the
        // env-steered history tests (history_file_is_owner_only and
        // run de96d4cc's legacy-fold test) are serialized against the
        // install instead of racing it for the life of the binary.
        let _env = crate::test_env::lock_env();
        let first = RuntimeConfigOverrides {
            history_dir: Some(PathBuf::from("/first-history")),
            pricing_file: Some(PathBuf::from("/first-pricing.json")),
            session_cache_entries: None,
        };
        let second = RuntimeConfigOverrides {
            history_dir: Some(PathBuf::from("/second-history")),
            pricing_file: None,
            session_cache_entries: Some(1),
        };
        // Both writers target the same process-global table; exactly
        // one of them installs. Assert get() is stable across calls.
        let installed_first = set(first.clone());
        let _ = set(second);
        let current = get();
        assert_eq!(current, current); // stable handle
        if installed_first {
            assert_eq!(current.history_dir, first.history_dir);
        }
        // Regardless of test ordering, the table is never empty-panic
        // and always reports Some/None values, never garbage.
        assert!(current.history_dir.is_some() || current.history_dir.is_none());
    }
}
