//! Operator cache-directory resolution, shared by every cache
//! consumer (session cache `sessions.json`, doctor cache report,
//! pricing snapshot, statusline capture journal).
//!
//! Consolidated from the four byte-identical per-module copies
//! (doctor / pricing / session_cache / statusline) — salvage 9b2a7a1
//! adopted by intent at run 79fe78c0 (repository-maintenance
//! c151f66b cycle 3); one decision table now pins the ladder for all
//! of them. The landed copies had since gained the rm-248 rider
//! (a set-but-empty OR RELATIVE `HOME` is unset-equivalent), which
//! this shared form preserves.

use std::ffi::OsStr;
use std::path::PathBuf;

/// Resolve the operator cache directory from explicit inputs — the
/// same rules as [`user_cache_dir`] without touching process
/// environment state, so the empty-env fallbacks are unit-testable
/// without mutating (unsafe) globals (the `resolve_user_config_path`
/// pattern from rm-683). `macos` is injected because the `cfg!`
/// copies were compile-time dead code on every non-macOS host — the
/// only way this crate's Linux CI legs can pin the macOS arm at all.
///
/// Decision order (salvage 9b2a7a1 U2; assess 8b46476c F2/F4):
///
/// 1. rm-248: an ABSOLUTE, non-empty `$XDG_CACHE_HOME` wins on EVERY
///    OS — it precedes the macOS arm so the landed rm-596
///    `doctor_demo_contract` sandbox (`HOME` + absolute
///    `XDG_CACHE_HOME`, asserting no `$HOME` text in the report) is
///    satisfiable on macOS too. A relative or empty value is treated
///    as unset: the XDG spec only defines absolute values, and
///    honoring a relative one scattered a per-cwd cache (observed
///    live: `XDG_CACHE_HOME=rel-cache agenttrace --overview` wrote
///    ./rel-cache/agenttrace from a scratch cwd).
/// 2. On macOS a usable `HOME` selects `$HOME/Library/Caches`.
/// 3. Otherwise a usable `HOME` selects `$HOME/.cache`.
/// 4. Otherwise the process temp dir (always absolute).
///
/// A usable `HOME` is set-but-non-empty AND absolute (the rm-248
/// rider, 2026-10-12 run 64e89da50d71, plus statusline cycle-3
/// review F1 attempt ae97110b): a set-but-empty OR RELATIVE `HOME`
/// is treated as unset on BOTH home arms. Without the guard, `HOME=`
/// or `HOME=rel-home` joined the RELATIVE `.cache` /
/// `Library/Caches`, so the cache resolved against the CWD and a
/// `.cache/agenttrace/statusline.jsonl` planted in the working
/// directory was read into the budget view (live PoC: `$999.00 ...
/// OVER by $989.00` from a planted journal; per-cwd session-cache
/// plants at /tmp/at-probe-8275/homeempty/).
#[cfg(test)]
pub(crate) use resolve_user_cache_dir as resolve_user_cache_dir_for_test;

pub(crate) fn resolve_user_cache_dir(
    home: Option<&OsStr>,
    xdg_cache_home: Option<&OsStr>,
    macos: bool,
) -> PathBuf {
    let usable_home = home
        .filter(|value| !value.is_empty())
        .filter(|value| std::path::Path::new(value).is_absolute());
    if let Some(cache) = xdg_cache_home
        .filter(|value| !value.is_empty())
        .filter(|value| std::path::Path::new(value).is_absolute())
    {
        return PathBuf::from(cache);
    }
    if macos {
        if let Some(home) = usable_home {
            return PathBuf::from(home).join("Library").join("Caches");
        }
    }
    if let Some(home) = usable_home {
        return PathBuf::from(home).join(".cache");
    }
    std::env::temp_dir()
}

/// The environment-reading wrapper every cache consumer calls.
pub(crate) fn user_cache_dir() -> PathBuf {
    resolve_user_cache_dir(
        std::env::var_os("HOME").as_deref(),
        std::env::var_os("XDG_CACHE_HOME").as_deref(),
        cfg!(target_os = "macos"),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Row {
        name: &'static str,
        home: Option<&'static OsStr>,
        xdg: Option<&'static OsStr>,
        macos: bool,
        expected: PathBuf,
    }

    /// The one decision table for all four consumers, subsuming the
    /// per-module pins the copies each carried. Red-first, proven on
    /// the two embedded historical algorithms (scratch harness
    /// /tmp/impl-36efe4f3/redfirst/proof.rs, run 79fe78c0): against
    /// the immediately-preceding 3e90993 copies (macOS arm FIRST,
    /// absolute-HOME guard already landed) exactly the
    /// `absolute xdg wins on macos` row is red — those copies
    /// returned `$HOME/Library/Caches` before consulting the XDG
    /// arm. Against the older salvage-base 82e23b9 form (unfiltered
    /// HOME) the `relative home …` rows are red instead — that form
    /// joined `HOME=rel-home` into the RELATIVE `.cache` /
    /// `Library/Caches` against the CWD, the bug the rm-248 rider
    /// landed the guard for; this shared form preserves that guard.
    #[cfg(target_os = "linux")]
    // fixtures assume unix fs/path/time semantics (driveless absolutes, /-encoded names, byte-transparent journals); never executed on Windows before the 2026-10-17 E0433 fix
    #[test]
    fn user_cache_dir_decision_table() {
        let empty = Some(OsStr::new(""));
        let home = Some(OsStr::new("/home/at-test"));
        let rel_home = Some(OsStr::new("rel-home"));
        let xdg_abs = Some(OsStr::new("/xdg-cache"));
        let xdg_rel = Some(OsStr::new("rel-cache"));

        let cases: &[Row] = &[
            // XDG arm: absolute + non-empty wins on EVERY OS.
            // RED-FIRST: on macos the pre-consolidation order
            // returned $HOME/Library/Caches here.
            Row {
                name: "absolute xdg wins on macos",
                home,
                xdg: xdg_abs,
                macos: true,
                expected: PathBuf::from("/xdg-cache"),
            },
            Row {
                name: "absolute xdg wins on linux",
                home,
                xdg: xdg_abs,
                macos: false,
                expected: PathBuf::from("/xdg-cache"),
            },
            Row {
                name: "absolute xdg wins with empty home",
                home: empty,
                xdg: xdg_abs,
                macos: false,
                expected: PathBuf::from("/xdg-cache"),
            },
            Row {
                name: "absolute xdg wins with relative home",
                home: rel_home,
                xdg: xdg_abs,
                macos: false,
                expected: PathBuf::from("/xdg-cache"),
            },
            // Relative or empty XDG is treated as unset (rm-248).
            Row {
                name: "relative xdg ignored on macos",
                home,
                xdg: xdg_rel,
                macos: true,
                expected: PathBuf::from("/home/at-test/Library/Caches"),
            },
            Row {
                name: "relative xdg ignored on linux",
                home,
                xdg: xdg_rel,
                macos: false,
                expected: PathBuf::from("/home/at-test/.cache"),
            },
            Row {
                name: "empty xdg ignored on macos",
                home,
                xdg: empty,
                macos: true,
                expected: PathBuf::from("/home/at-test/Library/Caches"),
            },
            Row {
                name: "empty xdg ignored on linux",
                home,
                xdg: empty,
                macos: false,
                expected: PathBuf::from("/home/at-test/.cache"),
            },
            // Unset XDG: the platform home arms.
            Row {
                name: "unset xdg on macos",
                home,
                xdg: None,
                macos: true,
                expected: PathBuf::from("/home/at-test/Library/Caches"),
            },
            Row {
                name: "unset xdg on linux",
                home,
                xdg: None,
                macos: false,
                expected: PathBuf::from("/home/at-test/.cache"),
            },
            // RED-FIRST (salvage-base form): an empty OR RELATIVE
            // HOME must never produce a RELATIVE cache dir — the
            // temp-dir fallback instead (both OS arms; the
            // unfiltered copies joined "" or "rel-home" into
            // ".cache"/"Library/Caches" against the CWD — the rm-248
            // rider pins this rule, and the shared resolver keeps
            // it).
            Row {
                name: "empty home falls to temp dir on macos",
                home: empty,
                xdg: None,
                macos: true,
                expected: std::env::temp_dir(),
            },
            Row {
                name: "empty home falls to temp dir on linux",
                home: empty,
                xdg: None,
                macos: false,
                expected: std::env::temp_dir(),
            },
            Row {
                name: "empty home + relative xdg falls to temp dir",
                home: empty,
                xdg: xdg_rel,
                macos: false,
                expected: std::env::temp_dir(),
            },
            Row {
                name: "relative home falls to temp dir on macos",
                home: rel_home,
                xdg: None,
                macos: true,
                expected: std::env::temp_dir(),
            },
            Row {
                name: "relative home falls to temp dir on linux",
                home: rel_home,
                xdg: None,
                macos: false,
                expected: std::env::temp_dir(),
            },
            Row {
                name: "relative home + relative xdg falls to temp dir",
                home: rel_home,
                xdg: xdg_rel,
                macos: false,
                expected: std::env::temp_dir(),
            },
            // Everything unset: the ladder end, always absolute.
            Row {
                name: "both unset falls to temp dir",
                home: None,
                xdg: None,
                macos: false,
                expected: std::env::temp_dir(),
            },
        ];
        for Row {
            name,
            home,
            xdg,
            macos,
            expected,
        } in cases
        {
            assert_eq!(
                &resolve_user_cache_dir(*home, *xdg, *macos),
                expected,
                "case {name}"
            );
        }
    }

    #[test]
    fn shared_user_cache_resolver_census_pins_one_definition() {
        // rm-1389 census pin (run 1a4cce3a cycle 3, batch "one truth
        // on the money-and-cache path"; relocated here at integration
        // case 4d82812f): exactly one definition of the shared
        // resolver may exist across crates/ — the four hand-synced
        // private copies the consolidations removed (rm-1389 first,
        // then the superseding rm-1536 move into this module) had
        // already drifted twice in three days (the empty-HOME fix
        // landed on three copies while statusline's lagged; the
        // relative-HOME fix then landed on statusline alone), so a
        // re-born private copy is a regression of exactly that
        // family. The needle is assembled at runtime so this test's
        // own source cannot count itself.
        let needle = ["fn ", "resolve_user_cache_dir"].concat();
        let crates_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join("crates");
        let mut definitions = 0usize;
        let mut stack = vec![crates_dir];
        while let Some(dir) = stack.pop() {
            let entries = std::fs::read_dir(&dir)
                .unwrap_or_else(|err| panic!("census cannot read {}: {err}", dir.display()));
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    stack.push(path);
                    continue;
                }
                if path.extension().and_then(|e| e.to_str()) != Some("rs") {
                    continue;
                }
                let source = std::fs::read_to_string(&path)
                    .unwrap_or_else(|err| panic!("census cannot read {}: {err}", path.display()));
                definitions += source.matches(&needle).count();
            }
        }
        assert_eq!(
            definitions, 1,
            "exactly one shared resolver definition may exist (this module); \
             the census found {definitions} across crates/"
        );
    }
}
