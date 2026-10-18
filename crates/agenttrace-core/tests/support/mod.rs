//! Shared support for agenttrace-core integration suites.
//!
//! rm-1954 (run ce9309d9 cycle 1, batch "battery-truth hygiene"): the
//! catalog-census suites (`regional_pricing_contract`,
//! `pricing_snapshot_hygiene`) resolve pricing through the PUBLIC lane,
//! which prefers a machine pricing cache at
//! `$AGENTTRACE_SESSION_CACHE_DIR/pricing.json` — or, when that var is
//! unset, the default user-cache location — over the bundled snapshot.
//! The catalog singleton initializes once per process on first lookup,
//! so a machine cache present AT THAT MOMENT silently re-points every
//! census assertion (assess f4cb5f63 F2 PoC: a mutated cache flipped
//! `bundled_catalog_window_sweep_serves_own_rows` with
//! `left: Some(524345)` vs `right: Some(512000)` on a pristine tree).
//!
//! [`with_isolated_pricing_env`] makes the census deterministic: it
//! serializes its users behind one lock (no census test can initialize
//! the catalog under ambient env), points the session cache at a fresh
//! empty scratch (nothing to prefer), relocates `HOME` to a scratch
//! (the default-location lane lands in the scratch too), and clears
//! `XDG_CACHE_HOME` and the operator override file
//! (`AGENTTRACE_PRICING_FILE`) so no ambient lane can reach the
//! catalog. Everything it touches is restored on drop.

use std::ffi::OsString;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, MutexGuard};

/// Serializes every [`with_isolated_pricing_env`] user in this process.
/// The pricing catalog initializes once per process on first lookup;
/// holding this lock across a census body guarantees that moment
/// happens under the isolated environment, whatever test runs first.
static ENV_LOCK: Mutex<()> = Mutex::new(());

/// Per-process sequence for unique scratch roots (rm-1882 discipline).
static SEQ: AtomicU64 = AtomicU64::new(0);

/// Env vars [`PricingEnvGuard`] isolates: name → saved ambient value
/// (`None` means the var was absent and is removed again on restore).
const ISOLATED_VARS: &[&str] = &[
    "AGENTTRACE_SESSION_CACHE_DIR",
    "HOME",
    "XDG_CACHE_HOME",
    "AGENTTRACE_PRICING_FILE",
];

/// RAII guard behind [`with_isolated_pricing_env`].
pub struct PricingEnvGuard {
    scratch: PathBuf,
    saved: Vec<(&'static str, Option<OsString>)>,
    _lock: MutexGuard<'static, ()>,
}

impl PricingEnvGuard {
    fn acquire() -> Self {
        // Poison-tolerant: a panicking census test must not brick the
        // isolation of every later census test in the binary.
        let _lock = ENV_LOCK
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let seq = SEQ.fetch_add(1, Ordering::Relaxed);
        let scratch =
            std::env::temp_dir().join(format!("at-pricing-env-{}-{}", std::process::id(), seq));
        // pid+seq is unique among live processes; clearing first also
        // neutralizes residue from a crashed earlier run of a recycled
        // pid — the scratch must be provably empty when the census
        // starts, or the isolation would be theater.
        let _ = std::fs::remove_dir_all(&scratch);
        std::fs::create_dir_all(scratch.join("cache")).expect("scratch session-cache dir");
        std::fs::create_dir_all(scratch.join("home")).expect("scratch HOME");

        let saved = ISOLATED_VARS
            .iter()
            .map(|name| (*name, std::env::var_os(name)))
            .collect();
        let guard = Self {
            scratch,
            saved,
            _lock,
        };
        guard.apply_isolated_env();
        guard
    }

    fn apply_isolated_env(&self) {
        std::env::set_var("AGENTTRACE_SESSION_CACHE_DIR", self.scratch.join("cache"));
        std::env::set_var("HOME", self.scratch.join("home"));
        std::env::remove_var("XDG_CACHE_HOME");
        std::env::remove_var("AGENTTRACE_PRICING_FILE");
    }

    fn restore_env(&self) {
        for (name, value) in &self.saved {
            match value {
                Some(v) => std::env::set_var(name, v),
                None => std::env::remove_var(name),
            }
        }
    }
}

impl Drop for PricingEnvGuard {
    fn drop(&mut self) {
        self.restore_env();
        let _ = std::fs::remove_dir_all(&self.scratch);
    }
}

/// Run `body` with the machine pricing-cache lane deterministically
/// absent (see the module docs). Every census assertion in the
/// catalog-lane suites must run inside this guard; assertions outside
/// it are env-coupled battery truth.
pub fn with_isolated_pricing_env<T>(body: impl FnOnce() -> T) -> T {
    let _guard = PricingEnvGuard::acquire();
    body()
}
