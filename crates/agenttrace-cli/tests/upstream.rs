//! Integration tests for `agenttrace upstream` (rm-024, cycle 1).
//!
//! Every scenario runs against a hermetic fixture repository pair (a
//! local bare "upstream" plus a fork clone whose remote is named
//! `upstream`), so the tests never touch the network or this repo's own
//! remotes. The `--fetch` path is exercised against the local bare
//! remote — same `git fetch` subprocess, no network.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const GIT_ENV: [(&str, &str); 8] = [
    ("GIT_AUTHOR_NAME", "fixture"),
    ("GIT_AUTHOR_EMAIL", "fixture@example.com"),
    ("GIT_COMMITTER_NAME", "fixture"),
    ("GIT_COMMITTER_EMAIL", "fixture@example.com"),
    ("GIT_AUTHOR_DATE", "2026-09-01T10:00:00Z"),
    ("GIT_COMMITTER_DATE", "2026-09-01T10:00:00Z"),
    ("GIT_CONFIG_GLOBAL", "/dev/null"),
    ("GIT_CONFIG_NOSYSTEM", "1"),
];

fn git(dir: &Path, args: &[&str]) -> String {
    let mut command = Command::new("git");
    command
        .args(["-C", dir.to_str().expect("fixture path is UTF-8")])
        .args(args);
    for (key, value) in GIT_ENV {
        command.env(key, value);
    }
    let output = command.output().expect("run fixture git");
    assert!(
        output.status.success(),
        "git -C {:?} {:?} failed: {}{}",
        dir,
        args,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );
    String::from_utf8_lossy(&output.stdout)
        .trim_end()
        .to_string()
}

fn write(dir: &Path, rel: &str, content: &str) {
    let path = dir.join(rel);
    std::fs::create_dir_all(path.parent().expect("file has a parent")).expect("create dirs");
    std::fs::write(path, content).expect("write fixture file");
}

fn run_cli(cwd: &Path, args: &[&str]) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_agenttrace"));
    command.args(args).current_dir(cwd);
    for (key, value) in GIT_ENV {
        command.env(key, value);
    }
    command.output().expect("run agenttrace CLI")
}

fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).to_string()
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).to_string()
}

struct Fixture {
    #[allow(dead_code)]
    root: PathBuf,
    src: PathBuf,
    bare: PathBuf,
    fork: PathBuf,
}

impl Fixture {
    /// Base repo (one shared commit) cloned to a bare `upstream.git`
    /// and a `fork` clone whose remote is named `upstream`, mirroring
    /// this repository's own remote layout.
    fn new(tag: &str) -> Fixture {
        let root =
            std::env::temp_dir().join(format!("agenttrace-upstream-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let src = root.join("src");
        std::fs::create_dir_all(&src).expect("create fixture src");
        git(&src, &["init", "-q", "-b", "master"]);
        write(&src, "README.md", "base\n");
        git(&src, &["add", "."]);
        git(&src, &["commit", "-q", "-m", "base: shared history"]);

        let bare = root.join("upstream.git");
        git(
            &src,
            &["clone", "-q", "--bare", ".", &bare.to_string_lossy()],
        );

        let fork = root.join("fork");
        let output = Command::new("git")
            .args([
                "clone",
                "-q",
                "-o",
                "upstream",
                bare.to_str().expect("bare path is UTF-8"),
                fork.to_str().expect("fork path is UTF-8"),
            ])
            .output()
            .expect("clone fork");
        assert!(
            output.status.success(),
            "fork clone failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );

        Fixture {
            root,
            src,
            bare,
            fork,
        }
    }

    fn fork_commit(&self, rel: &str, message: &str) {
        write(&self.fork, rel, "fork change\n");
        git(&self.fork, &["add", "."]);
        git(&self.fork, &["commit", "-q", "-m", message]);
    }

    /// Commits on the upstream side and publishes it to the bare remote
    /// the fork treats as `upstream`.
    fn upstream_commit(&self, rel: &str, message: &str) {
        write(&self.src, rel, "upstream change\n");
        git(&self.src, &["add", "."]);
        git(&self.src, &["commit", "-q", "-m", message]);
        git(
            &self.src,
            &["push", "-q", &self.bare.to_string_lossy(), "master"],
        );
    }

    fn upstream_tag(&self, name: &str) {
        git(&self.src, &["tag", name]);
        git(
            &self.src,
            &["push", "-q", &self.bare.to_string_lossy(), name],
        );
    }

    /// Fixture-side pre-fetch, simulating a fork whose remote-tracking
    /// refs are already current (the offline command's data source).
    fn fetch(&self) {
        git(&self.fork, &["fetch", "-q", "upstream"]);
    }
}

#[test]
fn offline_diverged_report_lists_ahead_behind_and_sync_point() {
    let fixture = Fixture::new("diverged");
    fixture.fork_commit("fork.txt", "fork: local work");
    fixture.upstream_commit("up1.txt", "upstream: first");
    fixture.upstream_commit("up2.txt", "upstream: second");
    fixture.fetch();

    let output = run_cli(&fixture.fork, &["upstream"]);
    assert!(
        output.status.success(),
        "upstream status failed: {}",
        stderr(&output)
    );
    let text = stdout(&output);
    assert!(
        text.starts_with("agenttrace upstream status — offline view"),
        "expected offline banner, got {text:?}"
    );
    assert!(text.contains("  upstream:         upstream/master"));
    assert!(text.contains("  ahead:            1 commit(s)"));
    assert!(text.contains("  behind:           2 commit(s)"));
    assert!(text.contains("  last sync:"));
    assert!(text.contains("base: shared history"));
    assert!(text.contains("  unported commits: 2"));
    assert!(text.contains("upstream: second"));
    assert!(text.contains("npm:              unknown (offline"));
}

#[test]
fn json_schema_counts_match_the_text_report() {
    let fixture = Fixture::new("json");
    fixture.fork_commit("fork.txt", "fork: local work");
    fixture.upstream_commit("up1.txt", "upstream: first");
    fixture.fetch();

    let output = run_cli(&fixture.fork, &["-f", "json", "upstream"]);
    assert!(
        output.status.success(),
        "json status failed: {}",
        stderr(&output)
    );
    let value: serde_json::Value =
        serde_json::from_str(&stdout(&output)).expect("stdout parses as JSON");
    assert_eq!(value["command"], "agenttrace upstream");
    assert_eq!(value["mode"], "offline");
    assert_eq!(value["remote"], "upstream/master");
    assert_eq!(value["ahead"], 1);
    assert_eq!(value["behind"], 1);
    assert_eq!(value["last_sync"]["subject"], "base: shared history");
    assert_eq!(value["unported_commits"].as_array().map(Vec::len), Some(1));
    assert_eq!(value["unported_commits"][0]["subject"], "upstream: first");
    assert_eq!(value["npm"]["package"], "@zack78/agenttrace");
    assert_eq!(
        value["npm"]["state"],
        "unknown (offline; agenttrace --fetch upstream)"
    );
}

#[test]
fn in_sync_clone_reports_zero_drift() {
    let fixture = Fixture::new("insync");
    let output = run_cli(&fixture.fork, &["upstream"]);
    assert!(output.status.success(), "{}", stderr(&output));
    let text = stdout(&output);
    assert!(text.contains("  ahead:            0 commit(s)"));
    assert!(text.contains("  behind:           0 commit(s)"));
    assert!(text.contains("  unported commits: 0"));
    assert!(text.contains("  new releases:     0"));
}

#[test]
fn missing_upstream_remote_fails_with_setup_hint() {
    let root = std::env::temp_dir().join(format!(
        "agenttrace-upstream-noremote-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("create fixture dir");
    git(&root, &["init", "-q", "-b", "master"]);

    let output = run_cli(&root, &["upstream"]);
    assert!(
        !output.status.success(),
        "must fail without an upstream remote"
    );
    assert!(
        stderr(&output).contains("no 'upstream' remote configured"),
        "expected setup hint, got {:?}",
        stderr(&output)
    );
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn outside_a_git_repository_fails() {
    let root =
        std::env::temp_dir().join(format!("agenttrace-upstream-nogit-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("create plain dir");

    let output = run_cli(&root, &["upstream"]);
    assert!(
        !output.status.success(),
        "must fail outside a git repository"
    );
    assert!(
        stderr(&output).contains("not inside a git repository"),
        "expected repository hint, got {:?}",
        stderr(&output)
    );
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn missing_git_binary_is_a_spawn_diagnosis_not_a_missing_repository() {
    // rm-579: run inside a REAL git repository but with a PATH that
    // cannot resolve `git`. The first prerequisite probe cannot even
    // start git — an environment failure that must name the spawn,
    // not collapse into the probe's semantic verdict ("not inside a
    // git repository"). Pinned in both renderings: the human report
    // and `-f json` share the anyhow error channel, so each must
    // carry the spawn diagnosis on stderr with a non-zero exit.
    let fixture = Fixture::new("spawn");

    let run_without_git = |json: bool| {
        let mut command = Command::new(env!("CARGO_BIN_EXE_agenttrace"));
        if json {
            command.arg("-f").arg("json");
        }
        command.arg("upstream").current_dir(&fixture.fork);
        for (key, value) in GIT_ENV {
            command.env(key, value);
        }
        command.env("PATH", "/nonexistent-agenttrace-rm579");
        command.output().expect("run agenttrace CLI")
    };

    for output in [run_without_git(false), run_without_git(true)] {
        assert!(
            !output.status.success(),
            "a PATH-less environment must fail, not report"
        );
        let err = stderr(&output);
        assert!(
            err.contains("failed to spawn git rev-parse --show-toplevel"),
            "expected the spawn diagnosis naming the probe, got {err:?}"
        );
        assert!(
            err.contains("is git installed and on PATH"),
            "expected the PATH hint, got {err:?}"
        );
        assert!(
            !err.contains("not inside a git repository"),
            "a spawn failure must not masquerade as a repository verdict, got {err:?}"
        );
    }
}

#[test]
fn fetch_flag_refreshes_remote_tracking_refs() {
    let fixture = Fixture::new("fetchflag");
    fixture.upstream_commit("up1.txt", "upstream: before clone-visible window");
    fixture.fetch();
    // Publish two more upstream commits but do NOT pre-fetch: only the
    // command's own `git fetch` can make them visible.
    fixture.upstream_commit("up2.txt", "upstream: unfetched second");
    fixture.upstream_commit("up3.txt", "upstream: unfetched third");

    let output = run_cli(&fixture.fork, &["--fetch", "upstream"]);
    assert!(
        output.status.success(),
        "--fetch upstream failed: {}",
        stderr(&output)
    );
    let text = stdout(&output);
    assert!(
        text.starts_with("agenttrace upstream status — post-fetch view"),
        "expected post-fetch banner, got {text:?}"
    );
    assert!(text.contains("  behind:           3 commit(s)"), "{text}");
}

#[test]
fn release_drift_and_area_grouping_appear_in_the_report() {
    let fixture = Fixture::new("areas");
    fixture.fork_commit("fork.txt", "fork: local work");
    fixture.upstream_commit(
        "crates/agenttrace-core/src/parser.rs",
        "upstream: parser fix (#287)",
    );
    fixture.upstream_commit(".github/workflows/ci.yml", "upstream: ci tweak (#288)");
    fixture.upstream_tag("v9.9.9");
    fixture.fetch();

    let output = run_cli(&fixture.fork, &["-f", "json", "upstream"]);
    assert!(output.status.success(), "{}", stderr(&output));
    let value: serde_json::Value =
        serde_json::from_str(&stdout(&output)).expect("stdout parses as JSON");
    let releases = value["new_upstream_releases"]
        .as_array()
        .expect("release list");
    assert!(
        releases.iter().any(|tag| tag == "v9.9.9"),
        "expected v9.9.9 in {releases:?}"
    );
    assert_eq!(value["unported_areas"]["parser"], 1);
    assert_eq!(value["unported_areas"]["ci/release"], 1);
    assert_eq!(
        value["unported_commits"][0]["files"][0],
        "crates/agenttrace-core/src/parser.rs"
    );
}

#[test]
fn upstream_remote_env_override_is_honored() {
    // A clone whose remote is named `origin` (the default) plus
    // UPSTREAM_REMOTE=origin must report against it — same override
    // scripts/upstream-delta supports.
    let fixture = Fixture::new("envremote");
    git(&fixture.fork, &["remote", "rename", "upstream", "origin"]);

    let plain = run_cli(&fixture.fork, &["upstream"]);
    assert!(
        !plain.status.success(),
        "default remote name must not match"
    );
    assert!(stderr(&plain).contains("no 'upstream' remote configured"));

    let mut command = Command::new(env!("CARGO_BIN_EXE_agenttrace"));
    command
        .arg("upstream")
        .current_dir(&fixture.fork)
        .env("UPSTREAM_REMOTE", "origin");
    for (key, value) in GIT_ENV {
        command.env(key, value);
    }
    let output = command.output().expect("run agenttrace CLI");
    assert!(output.status.success(), "{}", stderr(&output));
    let text = stdout(&output);
    assert!(text.contains("  upstream:         origin/master"), "{text}");
    assert!(text.contains("  behind:           0 commit(s)"));
}

#[test]
fn fetch_flag_without_the_upstream_command_is_rejected() {
    // --fetch only means something for `agenttrace upstream`; a stray
    // --fetch must fail loudly instead of silently reporting stale
    // data from another action.
    let fixture = Fixture::new("strayfetch");
    let output = run_cli(&fixture.fork, &["--fetch", "--overview"]);
    assert!(!output.status.success(), "stray --fetch must fail");
    assert!(
        stderr(&output).contains("--fetch applies only to the upstream command"),
        "expected stray-flag hint, got {:?}",
        stderr(&output)
    );
}
