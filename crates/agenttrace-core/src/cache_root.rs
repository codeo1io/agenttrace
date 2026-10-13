//! Shared cache-root resolution (rm-1018 consolidation + review
//! 95d74221 F2 empty-env guard + the macOS doctor-demo CI red at
//! ecf84dc, run 38189118617).
//!
//! Four byte-identical `user_cache_dir()` copies used to live in
//! doctor.rs / session_cache.rs / pricing.rs / statusline.rs, and they
//! had already drifted: statusline alone guarded a set-but-empty
//! `HOME` (the planted-journal class pinned by review 95d74221 F2),
//! and ALL of them returned `HOME/Library/Caches` on macOS BEFORE
//! honoring an absolute `$XDG_CACHE_HOME` — so the sandboxed-home
//! doctor contract held on Linux only, and the macOS CI lane leaked
//! the operator home into the demo report every run. One pure
//! decision function owns the ladder now; every artifact-root
//! constructor resolves through it, and the macOS arms are pinned by
//! tests on any host via the injected platform enum.

use std::ffi::OsStr;
use std::path::PathBuf;

/// Injected platform for [`resolve_user_cache_dir`]: the ladder's only
/// platform input, so the macOS arms are unit-testable on any host.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CachePlatform {
    /// `HOME/Library/Caches` is the platform default; an absolute
    /// `$XDG_CACHE_HOME` still wins over it.
    MacOs,
    /// `$HOME/.cache` (the XDG spec default).
    UnixLike,
}

/// Pure resolution ladder — no environment reads, no globals:
///
/// 1. an ABSOLUTE, non-empty `$XDG_CACHE_HOME` wins on EVERY platform,
///    macOS included (the pre-consolidation macOS arm honored it after
///    the platform default, which is what reddened the macOS
///    doctor-demo lane);
/// 2. the platform default — `HOME/Library/Caches` on macOS, else
///    `$HOME/.cache`;
/// 3. a set-but-empty or unset `HOME` counts as unset on both arms, so
///    no leg can join the RELATIVE `Library/Caches` / `.cache` against
///    the CWD (review 95d74221 F2: `HOME=` used to read a planted
///    `.cache/agenttrace/statusline.jsonl` into the budget view);
/// 4. the ladder always ends absolute: the process temp dir.
pub(crate) fn resolve_user_cache_dir(
    home: Option<&OsStr>,
    xdg_cache_home: Option<&OsStr>,
    platform: CachePlatform,
) -> PathBuf {
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
    let home = home.filter(|value| !value.is_empty());
    match platform {
        CachePlatform::MacOs => home
            .map(|home| PathBuf::from(home).join("Library").join("Caches"))
            .unwrap_or_else(std::env::temp_dir),
        CachePlatform::UnixLike => home
            .map(|home| PathBuf::from(home).join(".cache"))
            .unwrap_or_else(std::env::temp_dir),
    }
}

/// The environment-reading wrapper every artifact-root constructor
/// calls. `AGENTTRACE_SESSION_CACHE_DIR` stays a per-artifact concern
/// (each constructor joins its own file name under it), so this ladder
/// starts at `$XDG_CACHE_HOME`.
pub(crate) fn user_cache_dir() -> PathBuf {
    resolve_user_cache_dir(
        std::env::var_os("HOME").as_deref(),
        std::env::var_os("XDG_CACHE_HOME").as_deref(),
        if cfg!(target_os = "macos") {
            CachePlatform::MacOs
        } else {
            CachePlatform::UnixLike
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::OsStr;

    /// The macOS doctor-demo CI red (run 38189118617): an ABSOLUTE
    /// `$XDG_CACHE_HOME` must win over `HOME/Library/Caches` on macOS.
    /// Pre-consolidation every copy returned the platform default
    /// first, so the sandboxed-home contract held on Linux only.
    #[test]
    fn absolute_xdg_cache_home_wins_on_macos_too() {
        assert_eq!(
            resolve_user_cache_dir(
                Some(OsStr::new("/Users/at")),
                Some(OsStr::new("/var/folders/xdg-cache")),
                CachePlatform::MacOs,
            ),
            PathBuf::from("/var/folders/xdg-cache")
        );
        // And with no HOME at all: the override is still the answer.
        assert_eq!(
            resolve_user_cache_dir(None, Some(OsStr::new("/xdg-cache")), CachePlatform::MacOs),
            PathBuf::from("/xdg-cache")
        );
    }

    /// Review 95d74221 F2 (red-first `HOME=""` fixture, now pinned for
    /// every platform): a set-but-empty `HOME` must count as unset so
    /// no arm joins the RELATIVE `Library/Caches` / `.cache` against
    /// the CWD where a planted cache could be read.
    #[test]
    fn set_but_empty_home_is_unset_on_both_platforms() {
        for platform in [CachePlatform::MacOs, CachePlatform::UnixLike] {
            let empty_home = resolve_user_cache_dir(Some(OsStr::new("")), None, platform);
            assert!(
                empty_home.is_absolute(),
                "{platform:?}: empty HOME must never yield a relative cache dir ({empty_home:?})"
            );
            assert_eq!(
                empty_home,
                std::env::temp_dir(),
                "{platform:?}: empty HOME falls to the temp-dir ladder end"
            );
        }
        // Empty HOME with an absolute override: the override wins.
        assert_eq!(
            resolve_user_cache_dir(
                Some(OsStr::new("")),
                Some(OsStr::new("/xdg-cache")),
                CachePlatform::MacOs,
            ),
            PathBuf::from("/xdg-cache")
        );
        // Empty HOME with only a RELATIVE override: still no relative
        // answer.
        for platform in [CachePlatform::MacOs, CachePlatform::UnixLike] {
            let relative_only = resolve_user_cache_dir(
                Some(OsStr::new("")),
                Some(OsStr::new("rel-cache")),
                platform,
            );
            assert!(
                relative_only.is_absolute(),
                "{platform:?}: {relative_only:?} must stay absolute"
            );
        }
    }

    #[test]
    fn unset_envs_end_at_the_temp_dir_on_both_platforms() {
        for platform in [CachePlatform::MacOs, CachePlatform::UnixLike] {
            assert_eq!(
                resolve_user_cache_dir(None, None, platform),
                std::env::temp_dir()
            );
        }
    }

    #[test]
    fn platform_defaults_apply_when_no_override_is_usable() {
        assert_eq!(
            resolve_user_cache_dir(Some(OsStr::new("/Users/at")), None, CachePlatform::MacOs),
            PathBuf::from("/Users/at").join("Library").join("Caches")
        );
        assert_eq!(
            resolve_user_cache_dir(Some(OsStr::new("/home/at")), None, CachePlatform::UnixLike),
            PathBuf::from("/home/at").join(".cache")
        );
    }

    /// rm-248: a RELATIVE (or set-but-empty) `$XDG_CACHE_HOME` behaves
    /// exactly like unset on BOTH platforms.
    #[test]
    fn relative_or_empty_xdg_behaves_like_unset_on_both_platforms() {
        for platform in [CachePlatform::MacOs, CachePlatform::UnixLike] {
            let home = match platform {
                CachePlatform::MacOs => "/Users/at",
                CachePlatform::UnixLike => "/home/at",
            };
            let expected = resolve_user_cache_dir(Some(OsStr::new(home)), None, platform);
            for bad in ["rel-cache", ""] {
                assert_eq!(
                    resolve_user_cache_dir(Some(OsStr::new(home)), Some(OsStr::new(bad)), platform),
                    expected,
                    "{platform:?}: XDG_CACHE_HOME={bad:?} must behave exactly like unset"
                );
            }
        }
    }
}
