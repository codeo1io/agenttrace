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
    fn set_is_write_once_and_rejected_writers_never_mutate_the_table() {
        // rm-684 (2026-10-07): this lib binary shares the
        // process-global table with every other core test — any test
        // that resolves a knob (history_path, pricing) default-
        // initializes the OnceLock through get()'s get_or_init, so
        // whether this test is the first writer depends on thread
        // order. The DETERMINISTIC first-writer contract lives in its
        // own process at tests/runtime_config_first_writer.rs. What
        // must hold under ANY ordering: at most one set() ever
        // installs, a rejected set() never mutates the table, and
        // get() is a stable handle.
        let first = RuntimeConfigOverrides {
            history_dir: Some(PathBuf::from("/first-history")),
            pricing_file: Some(PathBuf::from("/first-pricing.json")),
        };
        let second = RuntimeConfigOverrides {
            history_dir: Some(PathBuf::from("/second-history")),
            pricing_file: None,
        };
        let installed_first = set(first.clone());
        let installed_second = set(second.clone());
        assert!(
            !(installed_first && installed_second),
            "two writers cannot both install: {installed_first}/{installed_second}"
        );
        let before = get().clone();
        let _ = set(RuntimeConfigOverrides {
            history_dir: Some(PathBuf::from("/third-history")),
            pricing_file: None,
        });
        assert_eq!(get(), &before, "a rejected writer mutated the table");
        let current = get();
        assert_eq!(current, get()); // stable handle
                                    // When this test WAS the first writer, the whole table (not
                                    // just one field) must be the first writer's, and the second
                                    // set() must have been rejected.
        if installed_first {
            assert!(!installed_second);
            assert_eq!(current.history_dir, first.history_dir);
            assert_eq!(current.pricing_file, first.pricing_file);
        }
    }
}
