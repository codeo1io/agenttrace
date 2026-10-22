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

/// rm-1954 completion (run 5cf06bab cycle 9, batch "host-state truth
/// hardening"): the standing poison-proof fixture for every wired
/// catalog-lane suite, factored from the landed
/// `mutated_machine_cache_does_not_flip_the_census`
/// (regional_pricing_contract.rs) so each suite pins its OWN
/// representative catalog assertion the same deterministic way, with
/// no reliance on ambient host state:
///
///   guarded   — poison seeded at every default machine-cache
///               location under a scratch HOME, session cache
///               relocated to a fresh empty scratch: the probe must
///               PASS (the bundled snapshot serves the catalog).
///   unguarded — the same poisoned HOME, no relocation: the probe
///               must FAIL (cache preference is real product
///               behavior; if this leg ever turns green the threat
///               model changed — re-adjudicate the fixture, never
///               delete the isolation guard).
///
/// `probe_filter` names the sentinel-gated `#[test]` in the caller's
/// binary that carries the suite's representative assertion;
/// `poisoned_cache_json` is machine-cache bytes (a mutated copy of
/// the bundled snapshot) that deterministically flips that assertion
/// when the cache lane is allowed to serve it.
///
/// `#[allow(dead_code)]`: only the four rm-1954-completion suites
/// call this; `pricing_snapshot_hygiene` and
/// `regional_pricing_contract` compile `mod support` too (for the
/// guard) but keep their own landed fixtures.
#[allow(dead_code)]
pub fn assert_poison_proof_standing(probe_filter: &str, poisoned_cache_json: &str) {
    let tmp = std::env::temp_dir();
    let poison_home = tmp.join(format!(
        "at-poison-home-{}-{}",
        std::process::id(),
        probe_filter
    ));
    let clean_session = tmp.join(format!(
        "at-clean-session-{}-{}",
        std::process::id(),
        probe_filter
    ));
    for dir in [&poison_home, &clean_session] {
        let _ = std::fs::remove_dir_all(dir);
    }
    // Seed every platform default machine-cache location under the
    // scratch HOME (the cache lane resolves ~/.cache on Linux,
    // ~/Library/Caches on macOS — seed both so the fixture is
    // portable either way, exactly as the landed fixture does).
    for rel in [".cache/agenttrace", "Library/Caches/agenttrace"] {
        let dir = poison_home.join(rel);
        std::fs::create_dir_all(&dir).expect("poison cache dir");
        std::fs::write(dir.join("pricing.json"), poisoned_cache_json).expect("seed poison");
    }
    std::fs::create_dir_all(&clean_session).expect("clean session scratch");

    let run_probe = |session_cache_dir: Option<&std::path::PathBuf>| {
        let mut cmd = std::process::Command::new(std::env::current_exe().expect("test exe"));
        cmd.arg("--exact")
            .arg(probe_filter)
            .env("AGENTTRACE_TEST_PRICING_PROBE", "1")
            .env("HOME", &poison_home)
            .env_remove("XDG_CACHE_HOME")
            .env_remove("AGENTTRACE_PRICING_FILE");
        match session_cache_dir {
            Some(dir) => cmd.env("AGENTTRACE_SESSION_CACHE_DIR", dir),
            None => cmd.env_remove("AGENTTRACE_SESSION_CACHE_DIR"),
        };
        cmd.output().expect("spawn probe")
    };

    let guarded = run_probe(Some(&clean_session));
    assert!(
        guarded.status.success(),
        "guarded catalog must ignore a poisoned default machine cache:\n{}",
        String::from_utf8_lossy(&guarded.stderr)
    );

    let unguarded = run_probe(None);
    assert!(
        !unguarded.status.success(),
        "the poison mechanism must stay real: the unguarded probe passed, so \
         the cache-preference contract changed — re-adjudicate this fixture"
    );

    let _ = std::fs::remove_dir_all(&poison_home);
    let _ = std::fs::remove_dir_all(&clean_session);
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
