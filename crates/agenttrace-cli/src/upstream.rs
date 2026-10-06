//! `agenttrace upstream` — fork-vs-upstream drift, made visible (rm-024).
//!
//! The fork's maintenance loop used to re-derive upstream drift by hand
//! every cycle. This command mirrors the standing shell report
//! `scripts/upstream-delta` (rm-012) in Rust so the same numbers are one
//! flag away for any user of the fork:
//!
//! - ahead/behind against the `upstream` remote's default branch,
//! - the last sync point (merge-base) with date and subject,
//! - diverged file count and the unported upstream commits grouped by
//!   the areas they touch (parser / diagnostics / CI / ...),
//! - release-channel drift (upstream tags the fork does not contain),
//! - npm distribution state for `@zack78/agenttrace`.
//!
//! Offline by contract: every number comes from local remote-tracking
//! refs (`git merge-base`, `git rev-list`, `git tag --merged`), never
//! from the network. `--fetch` (`agenttrace --fetch upstream`) refreshes
//! the remote-tracking refs first via `git fetch` and additionally probes
//! the npm registry; both network touches happen only behind that
//! explicit flag. No new crate dependencies: `git` (and optionally
//! `curl`) run as subprocesses, so the CLI crate stays network-free.

use anyhow::bail;
use serde_json::json;
use std::collections::BTreeMap;
use std::fs;
use std::io::Read;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant, SystemTime};

/// rm-450 (run b1ff12f8, cycle 2): every `git` subprocess this module
/// spawns runs under a stated deadline, mirroring governance.rs's
/// GIT_PROBE_TIMEOUT pattern (spawn, drain both pipes on helper
/// threads, poll `try_wait`, kill at the deadline). Local probes take
/// the governance ten-second local class; the opt-in network `git
/// fetch` takes the thirty-second network class the pricing download
/// already uses, because fetching upstream refs can legitimately take
/// longer than a local rev-parse. A subprocess that overruns its
/// deadline is a terminal error naming the deadline and the git
/// operation — never a silent partial report and never a hang.
const UPSTREAM_GIT_TIMEOUT: Duration = Duration::from_secs(10);
const UPSTREAM_FETCH_TIMEOUT: Duration = Duration::from_secs(30);

/// Why a bounded subprocess did not produce its output. `Timeout` is
/// distinguished from `Failed`/`Spawn` so the fail-fast prerequisite
/// probes in `collect_status` cannot misreport a wedged git as "not
/// inside a git repository": only a probe that actually ran and failed
/// counts as a missing prerequisite.
#[derive(Debug)]
enum GitRunError {
    /// The deadline passed and the child was killed. `op` is the
    /// human-named operation (e.g. `git fetch upstream --quiet`).
    Timeout { op: String, bound: Duration },
    /// The child ran to completion and exited non-zero.
    Failed { op: String, message: String },
    /// The child could not be spawned at all (e.g. no git on PATH).
    Spawn { op: String, message: String },
}

impl std::fmt::Display for GitRunError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GitRunError::Timeout { op, bound } => {
                write!(
                    f,
                    "{op} timed out after {}s and was killed",
                    bound.as_secs()
                )
            }
            GitRunError::Failed { op, message } => write!(f, "{op} failed: {message}"),
            GitRunError::Spawn { op, message } => write!(f, "failed to spawn {op}: {message}"),
        }
    }
}

/// Runs a subprocess under a wall-clock deadline and returns
/// `(status, stdout, stderr)`. Both pipes are drained on helper threads
/// because `read_to_end` blocks: draining on the polling thread could
/// deadlock against a child whose pipe buffer fills, turning a fast
/// child into a spurious timeout (the governance.rs
/// `wait_child_bounded` comment states the same contract). On deadline
/// the child is killed and the drains are joined so no thread
/// outlives the call.
fn run_bounded(
    program: &str,
    args: &[&str],
    bound: Duration,
) -> Result<(std::process::ExitStatus, String, String), GitRunError> {
    let op = format!("{program} {}", args.join(" "));
    let mut child = Command::new(program)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|err| GitRunError::Spawn {
            op: op.clone(),
            message: err.to_string(),
        })?;
    let stdout = spawn_drain(child.stdout.take());
    let stderr = spawn_drain(child.stderr.take());
    let deadline = Instant::now() + bound;
    let status = loop {
        if let Ok(Some(status)) = child.try_wait() {
            break status;
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            let _ = stdout.join();
            let _ = stderr.join();
            return Err(GitRunError::Timeout { op, bound });
        }
        std::thread::sleep(Duration::from_millis(25));
    };
    let stdout = drained_text(stdout);
    let stderr = drained_text(stderr);
    if !status.success() {
        return Err(GitRunError::Failed {
            op,
            message: snippet(&stderr),
        });
    }
    Ok((status, stdout, stderr))
}

fn spawn_drain<R: Read + Send + 'static>(pipe: Option<R>) -> std::thread::JoinHandle<Vec<u8>> {
    std::thread::spawn(move || {
        let mut bytes = Vec::new();
        if let Some(mut stream) = pipe {
            let _ = stream.read_to_end(&mut bytes);
        }
        bytes
    })
}

fn drained_text(handle: std::thread::JoinHandle<Vec<u8>>) -> String {
    String::from_utf8_lossy(&handle.join().unwrap_or_default()).to_string()
}

const DEFAULT_REMOTE: &str = "upstream";
const DEFAULT_REF: &str = "master";
/// Distribution channel for this fork (README install line). The repo
/// itself ships no npm manifest; the registry is the source of truth.
const NPM_PACKAGE: &str = "@zack78/agenttrace";
const NPM_REGISTRY_URL: &str = "https://registry.npmjs.org/@zack78%2fagenttrace/latest";
/// Text view caps the unported commit list; JSON carries all of them.
const MAX_UNPORTED_LISTED: usize = 20;

pub struct UpstreamCommit {
    pub sha: String,
    pub subject: String,
    pub files: Vec<String>,
}

pub struct UpstreamStatus {
    pub remote: String,
    pub ref_name: String,
    pub remote_url: String,
    pub local_head: String,
    pub local_branch: Option<String>,
    pub merge_base: String,
    pub base_date: String,
    pub base_subject: String,
    pub ahead: u64,
    pub behind: u64,
    pub diverged_files: u64,
    pub new_releases: Vec<String>,
    pub unported: Vec<UpstreamCommit>,
    pub areas: BTreeMap<String, u64>,
    pub refs_age_seconds: Option<u64>,
    pub fetched: bool,
    /// `registry@X` after a successful `--fetch` probe, `unavailable`
    /// when the probe failed, `unknown (offline)` otherwise.
    pub npm_state: String,
}

/// Entry point for the `agenttrace upstream` host command. Returns the
/// rendered report; failures (no repository, no remote, no common
/// history) propagate to the CLI error handler and exit 1.
pub fn status_report(format: &str, fetch: bool) -> anyhow::Result<String> {
    let status = collect_status(fetch)?;
    Ok(match format {
        "json" => status.to_json()?,
        _ => status.to_text(),
    })
}

fn remote_name() -> String {
    std::env::var("UPSTREAM_REMOTE").unwrap_or_else(|_| DEFAULT_REMOTE.to_string())
}

fn ref_name() -> String {
    std::env::var("UPSTREAM_REF").unwrap_or_else(|_| DEFAULT_REF.to_string())
}

fn collect_status(fetch: bool) -> anyhow::Result<UpstreamStatus> {
    let remote = remote_name();
    let ref_name = ref_name();

    // Fail fast with the same hints scripts/upstream-delta gives. rm-450:
    // only a probe that RAN and failed counts as a missing prerequisite —
    // a probe that timed out is terminal (named deadline), so a wedged
    // git can never masquerade as "not inside a git repository".
    match git_bounded(&["rev-parse", "--show-toplevel"]) {
        Err(err @ GitRunError::Timeout { .. }) => bail!("{err}"),
        Err(_) => bail!("not inside a git repository"),
        Ok(_) => {}
    }
    let remote_url = match git_bounded(&["remote", "get-url", &remote]) {
        Err(err @ GitRunError::Timeout { .. }) => bail!("{err}"),
        Err(_) => bail!("no '{remote}' remote configured (git remote add {remote} <url>)"),
        Ok(url) => url,
    };
    if fetch {
        fetch_remote(&remote)?;
    }
    let tracking = format!("refs/remotes/{remote}/{ref_name}");
    match git_bounded(&["rev-parse", "--verify", &tracking]) {
        Err(err @ GitRunError::Timeout { .. }) => bail!("{err}"),
        Err(_) => bail!("no remote-tracking ref {tracking}; run agenttrace --fetch upstream"),
        Ok(_) => {}
    }

    let local_head = git(&["rev-parse", "HEAD"])?;
    let local_branch = match git(&["rev-parse", "--abbrev-ref", "HEAD"]) {
        Ok(branch) if branch != "HEAD" => Some(branch),
        _ => None,
    };
    let merge_base = match git_bounded(&["merge-base", "HEAD", &tracking]) {
        Err(err @ GitRunError::Timeout { .. }) => bail!("{err}"),
        Err(_) => bail!("no common history with {remote}/{ref_name}"),
        Ok(base) => base,
    };

    let ahead = git(&["rev-list", "--count", &format!("{merge_base}..HEAD")])?
        .parse::<u64>()
        .unwrap_or(0);
    let behind = git(&["rev-list", "--count", &format!("HEAD..{tracking}")])?
        .parse::<u64>()
        .unwrap_or(0);
    let base_date = git(&["show", "-s", "--format=%cs", &merge_base])?;
    let base_subject = git(&["show", "-s", "--format=%s", &merge_base])?;
    let diverged_files = git(&["diff", "--name-only", &format!("{tracking}...HEAD")])?
        .lines()
        .filter(|line| !line.trim().is_empty())
        .count() as u64;
    let new_releases = git(&["tag", "--merged", &tracking, "--no-merged", "HEAD"])?
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(str::to_string)
        .collect::<Vec<_>>();
    // git log lists newest first; the report shows oldest first, like
    // scripts/upstream-delta's `git log --reverse`.
    let mut unported = parse_log_with_files(&git(&[
        "log",
        "--name-only",
        "--format=%H%x1f%s",
        &format!("HEAD..{tracking}"),
    ])?)?;
    unported.reverse();

    let mut areas: BTreeMap<String, u64> = BTreeMap::new();
    let mut seen_files = std::collections::BTreeSet::new();
    for commit in &unported {
        for file in &commit.files {
            if seen_files.insert(file.clone()) {
                *areas.entry(classify_area(file).to_string()).or_insert(0) += 1;
            }
        }
    }

    let npm_state = if fetch {
        match npm_registry_version() {
            Some(version) => format!("{NPM_PACKAGE}@{version} (registry)"),
            None => "unavailable (registry probe failed or curl absent)".to_string(),
        }
    } else {
        "unknown (offline; agenttrace --fetch upstream)".to_string()
    };

    Ok(UpstreamStatus {
        remote,
        ref_name,
        remote_url,
        local_head,
        local_branch,
        merge_base,
        base_date,
        base_subject,
        ahead,
        behind,
        diverged_files,
        new_releases,
        unported,
        areas,
        refs_age_seconds: refs_age_seconds(&tracking),
        fetched: fetch,
        npm_state,
    })
}

fn fetch_remote(remote: &str) -> anyhow::Result<()> {
    // rm-450: `--fetch` is this module's one opt-in network action and it
    // used to wait on `Command::output()` without a bound — a wedged
    // fetch (hung remote, stale mount) hung the whole CLI. It now runs
    // under UPSTREAM_FETCH_TIMEOUT; a timeout is terminal and names the
    // deadline, the operation, and the remote.
    match run_bounded("git", &["fetch", remote, "--quiet"], UPSTREAM_FETCH_TIMEOUT) {
        Ok(_) => Ok(()),
        Err(GitRunError::Timeout { op, bound }) => bail!(
            "{op} timed out after {}s (UPSTREAM_FETCH_TIMEOUT): the fetch was killed; \
             retry when the remote is reachable — no drift numbers are reported from a partial fetch",
            bound.as_secs()
        ),
        Err(other) => bail!("{other}"),
    }
}

/// Runs git bounded by UPSTREAM_GIT_TIMEOUT and returns trimmed stdout;
/// errors carry the command and a stderr snippet so a failed probe is
/// diagnosable from the report, and a timeout names its deadline.
fn git_bounded(args: &[&str]) -> Result<String, GitRunError> {
    let (_, stdout, _) = run_bounded("git", args, UPSTREAM_GIT_TIMEOUT)?;
    Ok(stdout.trim_end().to_string())
}

/// Runs git and returns trimmed stdout; errors carry the command and a
/// stderr snippet so a failed probe is diagnosable from the report.
fn git(args: &[&str]) -> anyhow::Result<String> {
    git_bounded(args).map_err(|err| match err {
        GitRunError::Timeout { op, bound } => anyhow::anyhow!(
            "{op} timed out after {}s (UPSTREAM_GIT_TIMEOUT): the probe was killed — \
             the upstream report is aborted, never rendered from a partial repository",
            bound.as_secs()
        ),
        other => anyhow::anyhow!("{other}"),
    })
}

fn snippet(text: &str) -> String {
    let first = text
        .lines()
        .find(|line| !line.trim().is_empty())
        .unwrap_or("");
    let mut out = first.chars().take(160).collect::<String>();
    if out.len() < first.len() {
        out.push('…');
    }
    out
}

/// Parses `git log --name-only --format=%H%x1f%s`: a line containing the
/// unit separator starts a commit record; every other non-empty line is
/// a file that commit touched.
fn parse_log_with_files(raw: &str) -> anyhow::Result<Vec<UpstreamCommit>> {
    let mut commits = Vec::new();
    for line in raw.lines() {
        let line = line.trim_end();
        if let Some((sha, subject)) = line.split_once('\u{1f}') {
            commits.push(UpstreamCommit {
                sha: sha.to_string(),
                subject: subject.to_string(),
                files: Vec::new(),
            });
        } else if !line.trim().is_empty() {
            match commits.last_mut() {
                Some(commit) => commit.files.push(line.to_string()),
                // A file line before any commit header cannot happen for
                // this format; refuse to silently misattribute it.
                None => bail!("unexpected git log output before first commit: {line}"),
            }
        }
    }
    Ok(commits)
}

/// Groups a changed path into the reporting areas used by the upstream
/// delta view (rm-024 names parser / diagnostics / CI explicitly).
fn classify_area(path: &str) -> &'static str {
    let path = path.trim_start_matches("./");
    if path.starts_with("crates/agenttrace-core/src/parser")
        || path.starts_with("crates/agenttrace-core/src/sqlite_sessions")
    {
        "parser"
    } else if path.starts_with("crates/agenttrace-core/src/diagnostics")
        || path.starts_with("crates/agenttrace-core/src/waste")
    {
        "diagnostics"
    } else if path.starts_with("crates/agenttrace-core/src/pricing") {
        "pricing"
    } else if path.starts_with("crates/agenttrace-core/src/reports") {
        "reports"
    } else if path.starts_with("crates/agenttrace-core") {
        "core"
    } else if path.starts_with("crates/agenttrace-tui") {
        "tui"
    } else if path.starts_with("crates/agenttrace-cli") {
        "cli"
    } else if path.starts_with(".github")
        || path == "deny.toml"
        || path == "Cargo.lock"
        || path.starts_with("scripts/ci")
        || path.starts_with("scripts/release")
    {
        "ci/release"
    } else if path == "package.json" || path == ".npmrc" || path.starts_with("scripts/npm") {
        "npm/install"
    } else if path.starts_with("docs/") || path.ends_with(".md") || path.starts_with("testdata/") {
        "docs/testdata"
    } else {
        "other"
    }
}

/// Age of the local remote-tracking data, so the offline view can say
/// how stale its own numbers are. Prefers the loose tracking ref's
/// mtime, falls back to FETCH_HEAD, and reports nothing when neither
/// exists (e.g. fully packed refs).
fn refs_age_seconds(tracking: &str) -> Option<u64> {
    let ref_path = git(&["rev-parse", "--git-path", tracking]).ok()?;
    if let Some(age) = mtime_age(&PathBuf::from(ref_path)) {
        return Some(age);
    }
    let fetch_head = git(&["rev-parse", "--git-path", "FETCH_HEAD"]).ok()?;
    mtime_age(&PathBuf::from(fetch_head))
}

fn mtime_age(path: &std::path::Path) -> Option<u64> {
    let modified = fs::metadata(path).and_then(|meta| meta.modified()).ok()?;
    SystemTime::now()
        .duration_since(modified)
        .ok()
        .map(|age| age.as_secs())
}

/// npm registry probe behind `--fetch` only. Degrades to `None` (never
/// fails the report) when curl is absent or the registry answers badly.
fn npm_registry_version() -> Option<String> {
    let output = Command::new("curl")
        .args(["-fsSL", "--max-time", "15", NPM_REGISTRY_URL])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let body =
        serde_json::from_str::<serde_json::Value>(&String::from_utf8_lossy(&output.stdout)).ok()?;
    body.get("version")?.as_str().map(str::to_string)
}

fn age_phrase(seconds: Option<u64>) -> String {
    match seconds {
        None => "unknown age".to_string(),
        Some(secs) if secs < 60 => format!("{secs}s old"),
        Some(secs) if secs < 3600 => format!("{}min old", secs / 60),
        Some(secs) if secs < 86_400 => format!("{}h old", secs / 3600),
        Some(secs) => format!("{}d old", secs / 86_400),
    }
}

impl UpstreamStatus {
    fn to_text(&self) -> String {
        let mode = if self.fetched {
            "post-fetch"
        } else {
            "offline"
        };
        let mut out = format!(
            "agenttrace upstream status — {mode} view (remote-tracking refs {})\n",
            age_phrase(self.refs_age_seconds)
        );
        out.push_str(&format!(
            "  upstream:         {}/{} ({})\n",
            self.remote, self.ref_name, self.remote_url
        ));
        match &self.local_branch {
            Some(branch) => out.push_str(&format!(
                "  local:            {} @ {}\n",
                branch,
                short(&self.local_head, 12)
            )),
            None => out.push_str(&format!(
                "  local:            detached @ {}\n",
                short(&self.local_head, 12)
            )),
        }
        out.push_str(&format!(
            "  last sync:        {} ({}) — {}\n",
            short(&self.merge_base, 12),
            self.base_date,
            self.base_subject
        ));
        out.push_str(&format!("  ahead:            {} commit(s)\n", self.ahead));
        out.push_str(&format!("  behind:           {} commit(s)\n", self.behind));
        out.push_str(&format!("  diverged files:   {}\n", self.diverged_files));
        if self.new_releases.is_empty() {
            out.push_str("  new releases:     0\n");
        } else {
            out.push_str(&format!(
                "  new releases:     {} ({})\n",
                self.new_releases.len(),
                self.new_releases.join(" ")
            ));
        }
        out.push_str(&format!("  unported commits: {}\n", self.unported.len()));
        for commit in self.unported.iter().take(MAX_UNPORTED_LISTED) {
            out.push_str(&format!(
                "    {} {}\n",
                short(&commit.sha, 8),
                commit.subject
            ));
        }
        if self.unported.len() > MAX_UNPORTED_LISTED {
            out.push_str(&format!(
                "    … and {} more\n",
                self.unported.len() - MAX_UNPORTED_LISTED
            ));
        }
        if self.areas.is_empty() {
            out.push_str("  unported areas:   none\n");
        } else {
            let areas = self
                .areas
                .iter()
                .map(|(area, count)| format!("{area} {count}"))
                .collect::<Vec<_>>()
                .join(", ");
            out.push_str(&format!("  unported areas:   {areas}\n"));
        }
        out.push_str(&format!(
            "  npm:              {npm_state} | running agenttrace v{}\n",
            agenttrace_core::VERSION,
            npm_state = self.npm_state
        ));
        out
    }

    /// Stable JSON schema for scripting; documented in
    /// docs/guides/upstream-status.md. Keys are append-only.
    fn to_json(&self) -> anyhow::Result<String> {
        let value = json!({
            "command": "agenttrace upstream",
            "mode": if self.fetched { "fetched" } else { "offline" },
            "remote": format!("{}/{}", self.remote, self.ref_name),
            "remote_url": self.remote_url,
            "local": {
                "head": self.local_head,
                "branch": self.local_branch,
            },
            "last_sync": {
                "sha": self.merge_base,
                "date": self.base_date,
                "subject": self.base_subject,
            },
            "ahead": self.ahead,
            "behind": self.behind,
            "diverged_files": self.diverged_files,
            "new_upstream_releases": self.new_releases,
            "unported_commits": self.unported.iter().map(|commit| json!({
                "sha": commit.sha,
                "subject": commit.subject,
                "files": commit.files,
            })).collect::<Vec<_>>(),
            "unported_areas": self.areas,
            "refs_age_seconds": self.refs_age_seconds,
            "npm": {
                "package": NPM_PACKAGE,
                "state": self.npm_state,
                "running_version": agenttrace_core::VERSION,
            },
        });
        Ok(serde_json::to_string_pretty(&value)?)
    }
}

fn short(sha: &str, len: usize) -> String {
    sha.chars().take(len).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn upstream_subprocess_timeouts_are_pinned_to_the_stated_classes() {
        // rm-450: the two deadline classes are contract, not tuning —
        // local git probes share governance.rs's ten-second local class
        // (GIT_PROBE_TIMEOUT), and the opt-in network fetch takes the
        // thirty-second network class the pricing download uses. If a
        // change here is intentional, update the error strings and
        // docs/guides/upstream-status.md in the same commit.
        assert_eq!(UPSTREAM_GIT_TIMEOUT, Duration::from_secs(10));
        assert_eq!(UPSTREAM_FETCH_TIMEOUT, Duration::from_secs(30));
    }

    #[test]
    fn run_bounded_kills_a_slow_child_at_the_deadline() {
        // A child that ignores the deadline is killed and surfaced as a
        // named Timeout — never a hang. `sleep 5` under a 300ms bound
        // must return well before the child's natural lifetime.
        let started = Instant::now();
        let result = run_bounded("sleep", &["5"], Duration::from_millis(300));
        match result {
            Err(GitRunError::Timeout { op, bound }) => {
                assert!(op.contains("sleep"), "timeout names the operation: {op}");
                assert_eq!(bound, Duration::from_millis(300));
            }
            other => panic!("expected Timeout, got {other:?}"),
        }
        assert!(
            started.elapsed() < Duration::from_secs(3),
            "the deadline, not the child's lifetime, bounds the wait"
        );
    }

    #[test]
    fn run_bounded_returns_output_for_a_fast_child() {
        let (_, stdout, _) = run_bounded("git", &["--version"], UPSTREAM_GIT_TIMEOUT)
            .expect("git --version runs inside the bound");
        assert!(
            stdout.contains("git version"),
            "stdout is drained intact: {stdout}"
        );
    }

    #[test]
    fn run_bounded_distinguishes_failure_from_timeout() {
        // A probe that runs and fails is Failed (snippeted stderr); only
        // a deadline overrun is Timeout — the distinction the fail-fast
        // prerequisite probes rely on.
        match run_bounded(
            "git",
            &["rev-parse", "--verify", "agenttrace-no-such-ref"],
            UPSTREAM_GIT_TIMEOUT,
        ) {
            Err(GitRunError::Failed { op, message }) => {
                assert!(
                    op.contains("rev-parse"),
                    "failure names the operation: {op}"
                );
                assert!(!message.is_empty(), "failure carries a stderr snippet");
            }
            other => panic!("expected Failed, got {other:?}"),
        }
    }

    #[test]
    fn run_bounded_surfaces_a_missing_binary_as_spawn() {
        match run_bounded(
            "agenttrace-definitely-absent-binary",
            &["--version"],
            UPSTREAM_GIT_TIMEOUT,
        ) {
            Err(GitRunError::Spawn { op, message }) => {
                assert!(op.contains("agenttrace-definitely-absent-binary"));
                assert!(!message.is_empty());
            }
            other => panic!("expected Spawn, got {other:?}"),
        }
    }

    #[test]
    fn classify_area_maps_the_named_reporting_areas() {
        assert_eq!(
            classify_area("crates/agenttrace-core/src/parser.rs"),
            "parser"
        );
        assert_eq!(
            classify_area("crates/agenttrace-core/src/sqlite_sessions.rs"),
            "parser"
        );
        assert_eq!(
            classify_area("crates/agenttrace-core/src/diagnostics.rs"),
            "diagnostics"
        );
        assert_eq!(classify_area(".github/workflows/ci.yml"), "ci/release");
        assert_eq!(classify_area("deny.toml"), "ci/release");
        assert_eq!(classify_area("crates/agenttrace-tui/src/app.rs"), "tui");
        assert_eq!(classify_area("README.md"), "docs/testdata");
        assert_eq!(classify_area("some/where/else.bin"), "other");
    }

    #[test]
    fn parse_log_with_files_splits_commits_and_their_files() {
        let raw = "b2222222222222222222222222222222222222222\u{1f}second upstream change\n\ncrates/agenttrace-cli/src/main.rs\n\n11111111111111111111111111111111111111111\u{1f}first upstream change\n\ncrates/agenttrace-core/src/parser.rs\n.github/workflows/ci.yml\n";
        let commits = parse_log_with_files(raw).expect("parse");
        assert_eq!(commits.len(), 2);
        assert_eq!(commits[0].subject, "second upstream change");
        assert_eq!(commits[0].files, vec!["crates/agenttrace-cli/src/main.rs"]);
        assert_eq!(commits[1].subject, "first upstream change");
        assert_eq!(
            commits[1].files,
            vec![
                "crates/agenttrace-core/src/parser.rs",
                ".github/workflows/ci.yml"
            ]
        );
    }

    #[test]
    fn parse_log_with_files_rejects_files_before_any_commit() {
        assert!(parse_log_with_files("stray-file.rs\n").is_err());
    }

    fn sample_status() -> UpstreamStatus {
        let mut areas = BTreeMap::new();
        areas.insert("parser".to_string(), 1);
        areas.insert("ci/release".to_string(), 2);
        UpstreamStatus {
            remote: "upstream".to_string(),
            ref_name: "master".to_string(),
            remote_url: "https://github.com/luoyuctl/agenttrace".to_string(),
            local_head: "90a4ef5abcdef".to_string(),
            local_branch: Some("conductor/run-x".to_string()),
            merge_base: "748d1d2abcdef".to_string(),
            base_date: "2026-09-02".to_string(),
            base_subject: "Merge pull request #280".to_string(),
            ahead: 3,
            behind: 2,
            diverged_files: 12,
            new_releases: vec!["v0.9.1".to_string()],
            unported: vec![UpstreamCommit {
                sha: "e54831e".to_string(),
                subject: "parser: fix edge".to_string(),
                files: vec!["crates/agenttrace-core/src/parser.rs".to_string()],
            }],
            areas,
            refs_age_seconds: Some(3_600),
            fetched: false,
            npm_state: "unknown (offline; agenttrace --fetch upstream)".to_string(),
        }
    }

    #[test]
    fn text_render_reports_every_schema_line() {
        let text = sample_status().to_text();
        assert!(text.starts_with("agenttrace upstream status — offline view"));
        assert!(text.contains("upstream:         upstream/master"));
        assert!(text.contains("ahead:            3 commit(s)"));
        assert!(text.contains("behind:           2 commit(s)"));
        assert!(text.contains("new releases:     1 (v0.9.1)"));
        // BTreeMap renders areas in lexicographic order — part of the
        // stable-schema promise.
        assert!(text.contains("unported areas:   ci/release 2, parser 1"));
        assert!(text.contains("npm:              unknown (offline"));
        assert!(text.contains("running agenttrace v"));
    }

    #[test]
    fn json_render_keeps_the_documented_keys() {
        let raw = sample_status().to_json().expect("json");
        let value: serde_json::Value = serde_json::from_str(&raw).expect("parse json");
        for key in [
            "command",
            "mode",
            "remote",
            "remote_url",
            "local",
            "last_sync",
            "ahead",
            "behind",
            "diverged_files",
            "new_upstream_releases",
            "unported_commits",
            "unported_areas",
            "refs_age_seconds",
            "npm",
        ] {
            assert!(value.get(key).is_some(), "missing schema key {key}");
        }
        assert_eq!(value["mode"], "offline");
        assert_eq!(value["ahead"], 3);
        assert_eq!(value["behind"], 2);
        assert_eq!(value["new_upstream_releases"][0], "v0.9.1");
        assert_eq!(value["unported_areas"]["parser"], 1);
        assert_eq!(value["npm"]["package"], "@zack78/agenttrace");
    }

    #[test]
    fn age_phrase_scales_the_units() {
        assert_eq!(age_phrase(None), "unknown age");
        assert_eq!(age_phrase(Some(45)), "45s old");
        assert_eq!(age_phrase(Some(120)), "2min old");
        assert_eq!(age_phrase(Some(7_200)), "2h old");
        assert_eq!(age_phrase(Some(172_800)), "2d old");
    }
}
