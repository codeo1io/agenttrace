//! rm-1022 (silent-zero closeout rider): README privacy transparency —
//! the auto-discovered agent roots are a documented contract, not an
//! implementation detail. This suite pins the README (EN and zh-CN)
//! enumeration to discovery's declared roots: every root
//! `known_session_dirs()` registers must appear in both READMEs as a
//! `~`-relative path row, so a newly registered scan surface cannot
//! ship undocumented (the pre-rm-1022 state: discovery silently grew
//! roots — antigravity's five, Copilot OTEL — with no user-facing
//! enumeration anywhere).
//!
//! Determinism: `known_session_dirs()` reads `HOME` plus the
//! relocation env vars (`CODEX_HOME`, `CLAUDE_CONFIG_DIR`,
//! `XDG_CONFIG_HOME`, `XDG_DATA_HOME`, `OPENCODE_DATA_DIR`,
//! `OPENCLAW_STATE_DIR`) and scans
//! pi-family roots structurally. This binary runs under a scratch
//! `HOME` with every relocation var cleared, so the enumeration is
//! exactly the statically declared list — no host state can leak in.
//! The macOS-only `user_config_dir` branch makes the Cline row
//! platform-dependent, so the pin runs where the README's row is
//! written: Linux.

#[cfg(target_os = "linux")]
use agenttrace_core::known_session_dirs;
#[cfg(target_os = "linux")]
use std::fs;
#[cfg(target_os = "linux")]
use std::path::{Path, PathBuf};

#[cfg(target_os = "linux")]
fn readme_path(relative: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../")
        .join(relative)
}

/// `~`-relative form of a root path under the scratch home, matching
/// the README table's rendering (forward slashes, `~` for the home).
#[cfg(target_os = "linux")]
fn tilde_form(home: &Path, path: &Path) -> Option<String> {
    let relative = path.strip_prefix(home).ok()?;
    Some(format!(
        "~/{}",
        relative.to_string_lossy().replace('\\', "/")
    ))
}

#[cfg(target_os = "linux")]
fn scratch_enumeration() -> Vec<(String, String)> {
    let scratch =
        std::env::temp_dir().join(format!("agenttrace-docs-roots-{}", std::process::id()));
    fs::create_dir_all(&scratch).expect("scratch home");
    // Env surgery is confined to THIS test binary (own process).
    std::env::set_var("HOME", &scratch);
    for var in [
        "CODEX_HOME",
        "CLAUDE_CONFIG_DIR",
        "XDG_CONFIG_HOME",
        "XDG_DATA_HOME",
        "OPENCODE_DATA_DIR",
        "OPENCLAW_STATE_DIR",
        "CLINE_SESSION_DATA_DIR",
    ] {
        std::env::remove_var(var);
    }
    let home = scratch.clone();
    let mut roots: Vec<(String, String)> = known_session_dirs()
        .into_iter()
        .filter_map(|dir| tilde_form(&home, &dir.path).map(|path| (dir.name.clone(), path)))
        .collect();
    roots.sort();
    roots.dedup();
    roots
}

#[cfg(target_os = "linux")]
#[test]
fn readmes_enumerate_every_auto_discovered_root() {
    let roots = scratch_enumeration();
    assert!(
        roots.len() >= 20,
        "expected the full static root census, got {}",
        roots.len()
    );
    for readme in ["README.md", "README.zh-CN.md"] {
        let text = fs::read_to_string(readme_path(readme))
            .unwrap_or_else(|error| panic!("read {readme}: {error}"));
        for (name, path) in &roots {
            let row = format!("`{path}`");
            assert!(
                text.contains(&row),
                "{readme} must enumerate auto-discovered root {name} at {path} (rm-1022 transparency contract)"
            );
        }
    }
}

#[cfg(target_os = "linux")]
#[test]
fn readmes_document_the_explicit_dir_escape() {
    // The enumeration must be paired with the escape hatch: a reader
    // who finds a root they did not expect needs to know how to scan
    // something outside the table.
    for readme in ["README.md", "README.zh-CN.md"] {
        let text = fs::read_to_string(readme_path(readme))
            .unwrap_or_else(|error| panic!("read {readme}: {error}"));
        assert!(
            text.contains("-d/--dir") || text.contains("`-d` / `--dir`"),
            "{readme} must point the roots table at the -d/--dir escape"
        );
    }
}
