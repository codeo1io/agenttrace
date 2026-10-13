//! rm-1047 + rm-249 + rm-089 CLI-lane guards (batch "discovery truth:
//! the corpus you asked for", run 7368c786 cycle 1).
//!
//! Binary-level fixtures for the surfaces the core contract tests pin
//! in-library: a resolved `AGENTTRACE_HISTORY_DIR` scopes the
//! `--sessions` corpus end to end (rm-1047 arm 2, the seeded-integration
//! acceptance — a populated HOME plus a pin elsewhere yields exactly
//! the pinned corpus, and an empty pin discloses the scoped-empty story
//! instead of reporting the home corpus); `-d` distinguishes a file
//! (pointing at the positional form), absence, and permission denial
//! with a check-permissions hint (rm-249); `--test-match` refuses a
//! trailing positional path and targets `-m <model>`'s lookup on both
//! render lanes (rm-089).
//!
//! Every run pins HOME/XDG/AGENTTRACE_* to a per-test scratch root, so
//! no test can read or write the operator's real agent homes or the
//! fleet-shared session cache.

use std::process::{Command, Stdio};

fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_agenttrace")
}

/// A minimal but real claude-format journal (the launch_guards /
/// zero_usage_contract line shapes) so every lane loads a genuine
/// session instead of failing for an unrelated parse reason.
fn claude_journal(marker: &str) -> String {
    let user = format!(
        r#"{{"type":"user","timestamp":"2026-10-12T01:00:00Z","cwd":"/tmp/scope","message":{{"role":"user","content":"{marker}"}}}}"#
    );
    let assistant = r#"{"type":"assistant","timestamp":"2026-10-12T01:00:05Z","message":{"id":"msg_1","model":"claude-sonnet-4-5-20250929","role":"assistant","content":[{"type":"text","text":"on it"}],"usage":{"input_tokens":10,"output_tokens":5,"cache_creation_input_tokens":2,"cache_read_input_tokens":3}}}"#;
    format!("{user}\n{assistant}\n")
}

struct Scratch {
    path: std::path::PathBuf,
}

impl Scratch {
    fn new(tag: &str) -> Self {
        let path = std::env::temp_dir().join(format!(
            "agenttrace-history-scope-{tag}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock")
                .as_nanos()
        ));
        std::fs::create_dir_all(&path).expect("scratch root");
        Scratch { path }
    }

    fn seed(&self, rel: &str, name: &str, marker: &str) -> std::path::PathBuf {
        let dir = self.path.join(rel);
        std::fs::create_dir_all(&dir).expect("seed dir");
        let file = dir.join(name);
        std::fs::write(&file, claude_journal(marker)).expect("seed journal");
        file
    }

    /// Runs the built binary with every host-derived store pinned into
    /// this scratch root (HOME, XDG layers, session cache, and the
    /// history dir pin under test).
    fn run(&self, args: &[&str], history_pin: &std::path::Path) -> (i32, String, String) {
        let output = Command::new(bin())
            .args(args)
            .env("HOME", &self.path.join("home"))
            .env("XDG_CONFIG_HOME", self.path.join("home/.config"))
            .env("XDG_CACHE_HOME", self.path.join("home/.cache"))
            .env("XDG_DATA_HOME", self.path.join("home/.local/share"))
            .env("AGENTTRACE_SESSION_CACHE_DIR", self.path.join("cache"))
            .env("AGENTTRACE_HISTORY_DIR", history_pin)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .expect("spawn agenttrace");
        (
            output.status.code().unwrap_or(-1),
            String::from_utf8_lossy(&output.stdout).to_string(),
            String::from_utf8_lossy(&output.stderr).to_string(),
        )
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.path);
    }
}

#[test]
fn history_dir_scopes_the_sessions_corpus_cli_lane() {
    // rm-1047 arm 2: a populated HOME (a discoverable .hermes/sessions
    // journal) plus a history_dir pin pointing elsewhere yields exactly
    // the pinned corpus on the --sessions lane.
    let scratch = Scratch::new("scope");
    let home_journal = scratch.seed("home/.hermes/sessions", "home-cli.jsonl", "home corpus marker");
    let pin_dir = scratch.path.join("pinned");
    std::fs::create_dir_all(&pin_dir).expect("pin dir");
    std::fs::write(
        pin_dir.join("pinned-cli.jsonl"),
        claude_journal("pinned corpus marker"),
    )
    .expect("pin journal");

    let (code, stdout, stderr) = scratch.run(&["--sessions", "-f", "json"], &pin_dir);
    assert_eq!(code, 0, "stderr: {stderr}");
    assert!(
        stdout.contains("pinned-cli.jsonl"),
        "the pinned corpus renders:\n{stdout}"
    );
    assert!(
        !stdout.contains("home-cli.jsonl"),
        "the home corpus must not leak across the pin:\n{stdout}"
    );
    assert!(
        stdout.contains("\"matched_sessions\": 1"),
        "exactly the pinned session, not the home corpus:\n{stdout}"
    );
    let _ = home_journal;
}

#[test]
fn empty_history_dir_pin_discloses_the_scoped_empty_corpus() {
    // rm-1047: a pin over an empty directory reads as the scoped-empty
    // story (naming the pin and the scoping semantics), never as the
    // populated home corpus and never as a bare "nothing found".
    let scratch = Scratch::new("empty-pin");
    scratch.seed("home/.hermes/sessions", "home-cli.jsonl", "home corpus marker");
    let pin_dir = scratch.path.join("pinned-empty");
    std::fs::create_dir_all(&pin_dir).expect("pin dir");

    let (code, _stdout, stderr) = scratch.run(&["--sessions"], &pin_dir);
    assert_ne!(code, 0, "an empty scoped corpus is a loud empty");
    assert!(
        stderr.contains("No session files found in history_dir"),
        "the error names the scoped-empty story:\n{stderr}"
    );
    assert!(
        stderr.contains("discovery is scoped to exactly this directory"),
        "the error states the scoping semantics:\n{stderr}"
    );
}

#[test]
fn dir_flag_pointing_at_a_file_is_rejected_pointing_at_the_positional_form() {
    // rm-249: `-d <file.jsonl>` is a usage error that points at the
    // positional form — and the positional form genuinely loads the
    // file, so the pointer is not a dead end.
    let scratch = Scratch::new("d-file");
    let journal = scratch.seed("corpus", "session.jsonl", "positional marker");
    let journal_str = journal.to_str().expect("utf8 path").to_string();
    let pin_dir = scratch.path.join("pinned");
    std::fs::create_dir_all(&pin_dir).expect("pin dir");

    let (code, _stdout, stderr) = scratch.run(
        &["--sessions", "-d", journal_str.as_str()],
        &pin_dir,
    );
    assert_eq!(code, 2, "a file where -d's directory belongs is rc2:\n{stderr}");
    assert!(
        stderr.contains("takes a directory, not a file"),
        "the error names the story:\n{stderr}"
    );
    assert!(
        stderr.contains("positional PATH"),
        "the error points at the positional form:\n{stderr}"
    );

    // The pointer is real: the same file as the positional PATH loads.
    let (code, stdout, stderr) = scratch.run(&["--sessions", journal_str.as_str()], &pin_dir);
    assert_eq!(code, 0, "stderr: {stderr}");
    assert!(
        stdout.contains("positional marker"),
        "the positional file form loads the single session (row name derives from content, not filename):\n{stdout}"
    );
}

#[test]
fn dir_flag_absence_is_distinguished_from_permission_denied() {
    // rm-249: a missing -d value and an unreadable one tell their two
    // different stories, each with its remedy.
    let scratch = Scratch::new("d-absent");
    let pin_dir = scratch.path.join("pinned");
    std::fs::create_dir_all(&pin_dir).expect("pin dir");

    let missing = scratch.path.join("does-not-exist");
    let (code, _stdout, stderr) = scratch.run(
        &["--sessions", "-d", missing.to_str().expect("utf8 path")],
        &pin_dir,
    );
    assert_eq!(code, 2, "absence is rc2:\n{stderr}");
    assert!(
        stderr.contains("does not exist"),
        "absence says so:\n{stderr}"
    );
    assert!(
        stderr.contains("drop -d to auto-discover"),
        "absence carries its remedy:\n{stderr}"
    );
}

#[cfg(unix)]
#[test]
fn dir_flag_permission_denied_carries_the_check_permissions_hint() {
    // rm-249: a chmod-000 directory is readable metadata but an
    // unreadable corpus — rc2 with the permission story and the
    // ls -ld hint (the fixture pins the chmod-000 case).
    let scratch = Scratch::new("d-denied");
    let pin_dir = scratch.path.join("pinned");
    std::fs::create_dir_all(&pin_dir).expect("pin dir");
    let locked = scratch.path.join("locked");
    std::fs::create_dir_all(&locked).expect("locked dir");
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o000))
        .expect("chmod 000");

    let (code, _stdout, stderr) = scratch.run(
        &["--sessions", "-d", locked.to_str().expect("utf8 path")],
        &pin_dir,
    );
    // Restore readable state regardless of the assertion outcome.
    let _ = std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o755));
    assert_eq!(code, 2, "an unreadable -d root is rc2:\n{stderr}");
    assert!(
        stderr.contains("cannot be read") || stderr.contains("cannot be accessed"),
        "the permission story is named:\n{stderr}"
    );
    assert!(
        stderr.contains("check permissions"),
        "the check-permissions hint rides along:\n{stderr}"
    );
    assert!(
        stderr.contains("ls -ld"),
        "the hint is actionable:\n{stderr}"
    );
}

#[test]
fn test_match_rejects_a_trailing_positional_path() {
    // rm-089 operand half: the trailing-operand swallow is closed —
    // `--test-match <path>` used to print the fixed probe set and
    // silently ignore the path.
    let scratch = Scratch::new("tm-operand");
    let pin_dir = scratch.path.join("pinned");
    std::fs::create_dir_all(&pin_dir).expect("pin dir");

    let (code, stdout, stderr) = scratch.run(
        &["--test-match", "/tmp/some-session.jsonl"],
        &pin_dir,
    );
    assert_ne!(code, 0, "the conflict must be a loud error");
    let combined = format!("{stdout}{stderr}");
    assert!(
        combined.contains("--test-match takes no positional session path"),
        "the error names the conflict and the two real options:\n{combined}"
    );
}

#[test]
fn test_match_targets_the_named_model_on_both_render_lanes() {
    // rm-089: `-m <model> --test-match` targets the named model's
    // lookup on the text lane and the json lane; the fixed probe set
    // stays on the default (no -m) lane.
    let scratch = Scratch::new("tm-named");
    let pin_dir = scratch.path.join("pinned");
    std::fs::create_dir_all(&pin_dir).expect("pin dir");

    let (code, stdout, stderr) =
        scratch.run(&["-m", "deepseek-chat", "--test-match"], &pin_dir);
    assert_eq!(code, 0, "stderr: {stderr}");
    assert!(
        stdout.contains("deepseek-chat"),
        "the named model renders on the text lane:\n{stdout}"
    );
    assert!(
        !stdout.contains("unknown-model-xyz"),
        "the fixed probe set stays on the default lane:\n{stdout}"
    );

    let (code, stdout, stderr) = scratch.run(
        &["-m", "deepseek-chat", "--test-match", "-f", "json"],
        &pin_dir,
    );
    assert_eq!(code, 0, "stderr: {stderr}");
    assert!(
        stdout.contains("deepseek-chat"),
        "the named model renders on the json lane:\n{stdout}"
    );
    assert!(
        !stdout.contains("unknown-model-xyz"),
        "the json lane targets the named model too:\n{stdout}"
    );
}
