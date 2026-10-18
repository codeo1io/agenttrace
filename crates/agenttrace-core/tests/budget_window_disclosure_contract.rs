//! rm-1985: the weekly budget window's day basis is UTC — `utc_day`
//! (statusline.rs) buckets capture samples by UTC calendar day and the
//! window is "today ..= today − 6d" ON THAT BASIS, so a user at
//! UTC+13/+14 can legitimately see a window boundary up to two local
//! calendar days away from their local "today" (the same day-basis
//! seam the daily series on the statusline report rides). The window is
//! not local-time and was never documented as either — this contract
//! pins the disclosure so it never has to be guessed:
//! - the `--budget` view's text render names the UTC day basis;
//! - the `--budget` view's JSON render carries a machine-readable
//!   `day_basis` field (`utc_calendar_days`);
//! - README's `--budget` row names the UTC day basis in prose.
//!
//! Red-first record (2026-10-18, attempt 7167f1ac): all three pins
//! FAILED on the pre-change tree — the header read `trailing 7
//! calendar days` with no basis, the JSON arm carried no day-basis
//! field, and README's row said "trailing 7 CALENDAR days ending
//! today" with no basis clause.
//! (Mint: roadmap band 93067dab rm-1985; selection e229cc98 member 2;
//! stewardship ddb09160.)

use std::sync::{Mutex, MutexGuard, OnceLock};

use agenttrace_core::render_budget_view;
use chrono::Utc;
use serde_json::{json, Value};

/// Tests that touch `AGENTTRACE_SESSION_CACHE_DIR` must hold this lock:
/// the env var is process-global (mirrors statusline_budget_truth.rs).
fn env_lock() -> MutexGuard<'static, ()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(|e| e.into_inner())
}

struct EnvRestore(Option<String>);

impl EnvRestore {
    fn set(dir: &std::path::Path) -> Self {
        let prior = std::env::var("AGENTTRACE_SESSION_CACHE_DIR").ok();
        std::fs::create_dir_all(dir).expect("scratch cache dir");
        std::env::set_var("AGENTTRACE_SESSION_CACHE_DIR", dir);
        EnvRestore(prior)
    }
}

impl Drop for EnvRestore {
    fn drop(&mut self) {
        match self.0.take() {
            Some(v) => std::env::set_var("AGENTTRACE_SESSION_CACHE_DIR", v),
            None => std::env::remove_var("AGENTTRACE_SESSION_CACHE_DIR"),
        }
    }
}

/// One healthy same-day pair so the budget window carries a foldable
/// sample (mirrors statusline_budget_truth.rs's `healthy_capture`).
fn planted_journal() -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("at-budget-basis-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("scratch cache dir");
    let journal = dir.join("statusline.jsonl");
    let now = Utc::now().timestamp();
    let a = json!({
        "captured_at": now,
        "payload": {
            "session_id": "basis-sess",
            "model": { "display_name": "PoC" },
            "cost": { "total_cost_usd": 2.0 },
        },
    });
    let b = json!({
        "captured_at": now,
        "payload": {
            "session_id": "basis-sess",
            "model": { "display_name": "PoC" },
            "cost": { "total_cost_usd": 2.5 },
        },
    });
    std::fs::write(&journal, format!("{a}\n{b}\n")).expect("plant journal");
    dir
}

#[test]
fn budget_view_text_names_the_utc_day_basis() {
    let _guard = env_lock();
    let dir = planted_journal();
    let _restore = EnvRestore::set(&dir);
    let text = render_budget_view("text", Some(10.0)).expect("budget view renders");
    std::fs::remove_dir_all(&dir).ok();

    assert!(
        text.contains("trailing 7 calendar days, UTC day boundaries"),
        "--budget text must name the UTC day basis of the weekly window\n{text}"
    );
}

#[test]
fn budget_view_json_carries_the_day_basis_field() {
    let _guard = env_lock();
    let dir = planted_journal();
    let _restore = EnvRestore::set(&dir);
    let raw = render_budget_view("json", Some(10.0)).expect("json arm renders");
    std::fs::remove_dir_all(&dir).ok();

    let parsed: Value = serde_json::from_str(&raw).expect("json arm is valid JSON");
    assert_eq!(
        parsed["day_basis"], "utc_calendar_days",
        "--budget json must disclose the weekly window's day basis\n{raw}"
    );
}

#[test]
fn readme_budget_row_names_the_utc_day_basis() {
    let readme = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../../README.md"))
        .expect("README readable from the crate");
    let row = readme
        .lines()
        .find(|l| l.starts_with("| `--budget`"))
        .expect("README carries a --budget flag-table row");
    assert!(
        row.contains("UTC day boundaries"),
        "README's --budget row must name the UTC day basis of the window\n{row}"
    );
}
