//! rm-1061 (run 62250c06, batch "corpus-scope truth"): `-d` is
//! repeatable on report lanes — the union of all roots is scanned
//! with a session file that appears in more than one root counted
//! exactly once. Red evidence: at base ecf84dc this CLI exited 2
//! with "the argument '--dir <DIR>' cannot be used multiple times".

use std::process::{Command, Stdio};

fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_agenttrace")
}

fn claude_journal(tag: &str, input: u64, output_tokens: u64) -> String {
    let user = format!(
        r#"{{"type":"user","timestamp":"2026-10-04T01:00:00Z","cwd":"/tmp/{tag}","message":{{"role":"user","content":"check the billing please"}}}}"#
    );
    let assistant = format!(
        r#"{{"type":"assistant","timestamp":"2026-10-04T01:00:05Z","message":{{"id":"msg_{tag}","model":"claude-sonnet-4-5-20250929","role":"assistant","content":[{{"type":"text","text":"on it"}}],"usage":{{"input_tokens":{input},"output_tokens":{output_tokens},"cache_creation_input_tokens":2,"cache_read_input_tokens":3}}}}}}"#
    );
    format!("{user}\n{assistant}\n")
}

fn scratch(tag: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("agenttrace-mrd-{}-{tag}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("mkdir scratch");
    dir
}

fn run_json(args: &[&str], cache: &std::path::Path) -> (i32, String) {
    let output = Command::new(bin())
        .args(args)
        .env("AGENTTRACE_SESSION_CACHE_DIR", cache)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("spawn agenttrace");
    (
        output.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&output.stdout).to_string(),
    )
}

#[test]
fn repeatable_dir_flags_scan_the_union() {
    let root_a = scratch("root-a");
    let root_b = scratch("root-b");
    let union_dir = scratch("union");
    let cache = scratch("cache-union");
    let shared = claude_journal("shared", 100, 50);
    std::fs::write(
        root_a.join("s-a-only.jsonl"),
        claude_journal("a-only", 10, 5),
    )
    .unwrap();
    std::fs::write(root_a.join("s-shared.jsonl"), &shared).unwrap();
    std::fs::write(root_b.join("s-shared.jsonl"), &shared).unwrap();
    std::fs::write(
        root_b.join("s-b-only.jsonl"),
        claude_journal("b-only", 7, 3),
    )
    .unwrap();
    std::fs::write(
        union_dir.join("s-a-only.jsonl"),
        claude_journal("a-only", 10, 5),
    )
    .unwrap();
    std::fs::write(union_dir.join("s-shared.jsonl"), &shared).unwrap();
    std::fs::write(
        union_dir.join("s-b-only.jsonl"),
        claude_journal("b-only", 7, 3),
    )
    .unwrap();

    let (code, out) = run_json(
        &[
            "-d",
            root_a.to_str().unwrap(),
            "-d",
            root_b.to_str().unwrap(),
            "--overview",
            "-f",
            "json",
        ],
        &cache,
    );
    assert_eq!(code, 0, "stderr/stdout: {out}");
    let parsed: serde_json::Value = serde_json::from_str(&out).expect("overview json");
    let sessions = parsed["summary"]["total_sessions"].as_u64().unwrap_or(0);
    assert_eq!(sessions, 3, "shared file must count once: {parsed}");

    let (single_code, single_out) = run_json(
        &[
            "-d",
            union_dir.to_str().unwrap(),
            "--overview",
            "-f",
            "json",
        ],
        &cache,
    );
    assert_eq!(single_code, 0);
    let single: serde_json::Value = serde_json::from_str(&single_out).expect("overview json");
    assert_eq!(
        single["summary"]["total_sessions"].as_u64().unwrap_or(0),
        3,
        "union fixture must hold three distinct sessions"
    );
    // token/cost parity: identical files ⇒ identical totals
    for key in ["total_sessions", "total_tokens", "total_cost"] {
        assert_eq!(
            parsed["summary"][key], single["summary"][key],
            "rm-1061: multi-root totals must equal the single-root scan of the union"
        );
    }
}

#[test]
fn single_dir_lane_still_works() {
    let root = scratch("solo");
    let cache = scratch("cache-solo");
    std::fs::write(root.join("only.jsonl"), claude_journal("only", 4, 2)).unwrap();
    let (code, out) = run_json(
        &["-d", root.to_str().unwrap(), "--overview", "-f", "json"],
        &cache,
    );
    assert_eq!(code, 0);
    let parsed: serde_json::Value = serde_json::from_str(&out).expect("overview json");
    assert_eq!(parsed["summary"]["total_sessions"].as_u64().unwrap_or(0), 1);
}
