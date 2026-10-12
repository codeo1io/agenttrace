//! Shared user-cache-root resolution (rm-985, run 6e476825 cycle 1).
//!
//! This crate used to carry FOUR private `user_cache_dir()` copies —
//! statusline (the guarded model, review 95d74221 F2) plus three
//! byte-identical-but-UNguarded copies in pricing, doctor, and
//! session_cache. The unguarded copies treated a set-but-empty `HOME`
//! as set, joined `.cache` against the CWD, and wrote a RELATIVE
//! cache (assess PoC: `cd /tmp && HOME= XDG_CACHE_HOME=
//! agenttrace --overview` created `./.cache/agenttrace/sessions.json`,
//! 1939 bytes). The model's rules now live here, ONCE, and every
//! artifact-root constructor in the crate routes through them:
//!
//! - `AGENTTRACE_SESSION_CACHE_DIR` stays a first-class override at
//!   each constructor (it relocates whole roots, not just this dir);
//! - a set-but-empty `HOME` is treated as unset on BOTH arms (XDG
//!   spec v0.8 rule, review 95d74221 F2);
//! - a relative `XDG_CACHE_HOME` is treated as unset (rm-248: the XDG
//!   Base Directory spec only defines ABSOLUTE values);
//! - the ladder ends at the system temp dir, never a relative path.

use std::path::PathBuf;

/// Environment-aware wrapper over [`resolve_user_cache_dir`]: reads
/// `HOME` / `XDG_CACHE_HOME` from the process environment. Every
/// cache-root constructor in the crate calls THIS, never its own copy.
pub(crate) fn user_cache_dir() -> PathBuf {
    resolve_user_cache_dir(
        std::env::var_os("HOME").as_deref(),
        std::env::var_os("XDG_CACHE_HOME").as_deref(),
    )
}

/// Pure resolution core for [`user_cache_dir`]: the same rules without
/// touching process environment state, so the empty-env fallbacks are
/// unit-testable without mutating (unsafe) globals (the
/// `resolve_user_config_path` pattern from rm-683).
///
/// Review 95d74221 F2 (2026-10-07): a set-but-empty `HOME` is now
/// treated as unset on BOTH arms — the same XDG-spec v0.8 rule the
/// user-config guard applies. Without the guard, `HOME=` joined the
/// RELATIVE `Library/Caches` / `.cache`, so the capture journal
/// resolved against the CWD and a `.cache/agenttrace/statusline.jsonl`
/// planted in the working directory was read into the budget view
/// (live PoC: `$999.00 ... OVER by $989.00` from a planted journal).
pub(crate) fn resolve_user_cache_dir(
    home: Option<&std::ffi::OsStr>,
    xdg_cache_home: Option<&std::ffi::OsStr>,
) -> PathBuf {
    if cfg!(target_os = "macos") {
        if let Some(home) = home.filter(|value| !value.is_empty()) {
            return PathBuf::from(home).join("Library").join("Caches");
        }
    }
    // rm-248: the XDG Base Directory spec only defines ABSOLUTE
    // $XDG_CACHE_HOME values; a relative one is treated as unset and
    // falls through to the HOME-based default instead of scattering a
    // per-cwd cache (observed live: `XDG_CACHE_HOME=rel-cache
    // agenttrace --overview` wrote ./rel-cache/agenttrace from a
    // scratch cwd).
    if let Some(cache) = xdg_cache_home
        .filter(|value| !value.is_empty())
        .filter(|value| std::path::Path::new(value).is_absolute())
    {
        return PathBuf::from(cache);
    }
    if let Some(home) = home.filter(|value| !value.is_empty()) {
        return PathBuf::from(home).join(".cache");
    }
    std::env::temp_dir()
}

#[cfg(test)]
mod tests {
    use super::resolve_user_cache_dir;
    use std::ffi::OsStr;
    use std::path::PathBuf;

    #[test]
    fn user_cache_dir_treats_empty_env_as_unset() {
        // Review 95d74221 F2: `HOME=` or `XDG_CACHE_HOME=` must never
        // yield a RELATIVE cache dir — an empty HOME used to join
        // `.cache`/`Library/Caches` against the CWD, where a planted
        // statusline journal loaded into the budget view (live PoC:
        // `$999.00 ... OVER by $989.00`).
        let empty = Some(OsStr::new(""));
        // Both empty: the temp-dir fallback — always absolute.
        let both_empty = resolve_user_cache_dir(empty, empty);
        assert!(both_empty.is_absolute(), "{both_empty:?}");
        // Empty HOME with a set XDG override: the override wins.
        assert_eq!(
            resolve_user_cache_dir(empty, Some(OsStr::new("/xdg-cache"))),
            PathBuf::from("/xdg-cache")
        );
        // Unset both: the same ladder end.
        assert_eq!(resolve_user_cache_dir(None, None), std::env::temp_dir());
        #[cfg(not(target_os = "macos"))]
        {
            // Set HOME with an empty XDG value: the spec default
            // `$HOME/.cache`, not the RELATIVE `.cache` the empty
            // value used to produce.
            assert_eq!(
                resolve_user_cache_dir(Some(OsStr::new("/home/at-test")), empty),
                PathBuf::from("/home/at-test").join(".cache")
            );
            // rm-248: a RELATIVE XDG_CACHE_HOME is treated as unset —
            // the XDG spec only defines absolute values, and honoring
            // a relative one scattered a per-cwd cache
            // (XDG_CACHE_HOME=rel-cache wrote ./rel-cache/agenttrace
            // from a scratch cwd). Same ladder as session_cache /
            // doctor / pricing (`relative_xdg_cache_home_is_ignored_per_spec`).
            assert_eq!(
                resolve_user_cache_dir(
                    Some(OsStr::new("/home/at-test")),
                    Some(OsStr::new("rel-cache"))
                ),
                PathBuf::from("/home/at-test").join(".cache")
            );
            let relative_only = resolve_user_cache_dir(empty, Some(OsStr::new("rel-cache")));
            assert!(relative_only.is_absolute(), "{relative_only:?}");
        }
    }

    #[test]
    fn empty_home_alone_lands_on_the_temp_ladder_end() {
        // rm-985 per-arm pin (the case the three unguarded copies got
        // wrong): HOME set-but-empty with XDG unset-or-empty must end
        // at the system temp dir — NEVER the CWD-relative `.cache`.
        let empty = Some(OsStr::new(""));
        for xdg in [None, empty] {
            assert_eq!(
                resolve_user_cache_dir(empty, xdg),
                std::env::temp_dir(),
                "empty HOME with xdg={xdg:?} must reach the temp fallback"
            );
        }
    }
}
