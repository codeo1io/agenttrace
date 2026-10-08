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
    fn second_writer_never_wins_in_shared_process_state() {
        // rm-800 (minted campaign-locally as rm-753 by run bbde21568cd4;
        // rebound at integration 2026-10-08, conflict case
        // 615546e27c2b4f2d86e02b511f23ddd1 — the rm-753 numeral is held
        // by the landed rm-753 SQLite row-drop row): the process-global
        // table cannot be reset from a test, so the deterministic
        // first-writer pin lives in the
        // fresh-process target (tests/runtime_config_contract.rs),
        // where this file is the table's only writer and "did the
        // first writer install" is always true. What CAN be pinned
        // here -- in any process state and any test order, and unlike
        // the tautologies this replaces, able to fail -- is that a
        // second set() is rejected and a rejected writer never
        // changes what get() reports.
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
        };
        let second = RuntimeConfigOverrides {
            history_dir: Some(PathBuf::from("/second-history")),
            pricing_file: None,
        };
        let _first_installed = set(first);
        let before = get();
        let second_installed = set(second);
        let after = get();
        assert!(
            !second_installed,
            "write-once: a second set() must be reported as rejected"
        );
        assert_eq!(
            after, before,
            "a rejected writer must not alter what get() reports"
        );
    }
}
