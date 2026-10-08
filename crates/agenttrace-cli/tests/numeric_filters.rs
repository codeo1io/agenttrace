//! End-to-end pins for the shared numeric-filter dialect (rm-389 remaining
//! scope / rm-786, run 24ec00eb repository-maintenance cycle 1).
//!
//! The CLI gate and matcher route through `agenttrace_core::filters` — the
//! same ONE dialect the TUI uses. Observable contract, pinned here against
//! the in-tree generated corpus (fixtures evolve additively, so assertions
//! use distinctive row fragments instead of exact row counts):
//! - an optional `>=`/`<=`/`>`/`<`/`=` operator followed by a FINITE number;
//! - a bare number means `>=` (the CLI previously rejected `--cost 0`, the
//!   TUI accepted it — now both accept it);
//! - non-finite thresholds (`>=nan`, `<=inf`, `1e400`) and garbage are
//!   invalid filters that exit 1 with a message documenting the dialect
//!   (they previously parsed fine and then silently matched nothing or
//!   everything).

use std::path::Path;
use std::process::Command;

/// Sandboxed cache envs (mirrors csv_export.rs): the CLI must not read or
/// write the operator's real `~/.cache/agenttrace/sessions.json`, and
/// parallel test threads must not race on a shared cache file.
fn run_cli(dir: &Path, extra_args: &[&str]) -> (i32, String, String) {
    let sandbox = std::env::temp_dir().join(format!(
        "at-numeric-filters-{}-{:?}",
        std::process::id(),
        std::thread::current().id()
    ));
    let cache = sandbox.join("cache");
    std::fs::create_dir_all(&cache).expect("create sandbox cache dir");
    let output = Command::new(env!("CARGO_BIN_EXE_agenttrace"))
        .env("HOME", &sandbox)
        .env("XDG_CACHE_HOME", &cache)
        .env("AGENTTRACE_SESSION_CACHE_DIR", &cache)
        .arg("--sessions")
        .arg("-f")
        .arg("text")
        .arg("-d")
        .arg(dir)
        .args(extra_args)
        .output()
        .expect("run agenttrace CLI");
    std::fs::remove_dir_all(&sandbox).ok();
    (
        output.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&output.stdout).into_owned(),
        String::from_utf8_lossy(&output.stderr).into_owned(),
    )
}

fn generated_dir() -> &'static Path {
    Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../testdata/generated"
    ))
}

#[test]
fn cost_filter_accepts_bare_numbers_as_gte_end_to_end() {
    // Bare `0` means `>=0`: it must keep the zero-cost corpus rows that the
    // strictly-greater `>0` form filters out — proof the CLI speaks the
    // same dialect as the TUI, where bare already meant `>=`.
    let (rc_gt, out_gt, _) = run_cli(generated_dir(), &["--cost", ">0"]);
    let (rc_bare, out_bare, _) = run_cli(generated_dir(), &["--cost", "0"]);
    assert_eq!(rc_gt, 0, "strictly-greater cost filter must be valid");
    assert_eq!(
        rc_bare, 0,
        "bare cost filter must now be valid (was exit 1)"
    );

    let zero_cost_row = out_bare
        .lines()
        .find(|line| line.contains("qwen3-coder") && line.contains("\t0.0000\t"))
        .unwrap_or_else(|| panic!("bare `--cost 0` must keep zero-cost rows, got:\n{out_bare}"));
    assert!(!zero_cost_row.is_empty());
    assert!(
        !out_gt.lines().any(|line| line.contains("qwen3-coder")),
        "`--cost >0` must exclude zero-cost rows that the bare form keeps:\n{}",
        out_gt
    );

    // An explicit operator form behaves identically to the bare spelling.
    let (rc_ge, out_ge, _) = run_cli(generated_dir(), &["--cost", ">=0"]);
    assert_eq!(rc_ge, 0);
    assert_eq!(
        out_ge, out_bare,
        "`>=0` and bare `0` must be the same filter"
    );
}

#[test]
fn cost_filter_rejects_non_finite_thresholds_end_to_end() {
    // These parsed fine under the old gate and then matched nothing
    // (`>=nan` via total_cmp) or everything (`<=inf`) — silent nonsense
    // instead of an error.
    for bad in [">=nan", "<=inf", ">=inf", "1e400", "=1e400", "abc", ">="] {
        let (rc, _, err) = run_cli(generated_dir(), &["--cost", bad]);
        assert_eq!(rc, 1, "`--cost {bad}` must exit 1, stderr: {err}");
        assert!(
            err.contains(&format!("invalid --cost filter: {bad}")),
            "error must name the rejected filter {bad:?}: {err}"
        );
        assert!(
            err.contains("bare number means >="),
            "error must document the shared dialect: {err}"
        );
    }
}

#[test]
fn health_filter_shares_the_dialect_end_to_end() {
    // Tier spellings still work; numeric health filters follow the same
    // finite dialect, and the old `>=nan` acceptance is gone.
    let (rc_nan, _, err_nan) = run_cli(generated_dir(), &["--health", ">=nan"]);
    assert_eq!(rc_nan, 1, "`--health >=nan` must exit 1");
    assert!(
        err_nan.contains("invalid --health filter: >=nan"),
        "error must name the rejected filter: {err_nan}"
    );

    let (rc_bare, out_bare, _) = run_cli(generated_dir(), &["--health", ">=0"]);
    assert_eq!(rc_bare, 0);
    assert!(
        out_bare.contains("SESSION\tHEALTH"),
        "a valid numeric health filter must render the sessions table"
    );

    let (rc_tier, out_tier, _) = run_cli(generated_dir(), &["--health", "good"]);
    assert_eq!(rc_tier, 0);
    assert!(
        out_tier.contains("SESSION\tHEALTH"),
        "tier filter still valid"
    );
}
