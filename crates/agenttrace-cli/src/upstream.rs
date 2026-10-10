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

/// rm-543 (run b1ff12f8, cycle 2): every `git` subprocess this module
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
    /// The bound elapsed and the call gave up. Either the child was
    /// still alive at the deadline (it was killed), or it had already
    /// exited but a forked helper kept a pipe write-end open past the
    /// bound (its drains were abandoned, bytes unrecoverable). `op` is
    /// the human-named operation (e.g. `git fetch upstream --quiet`).
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
                    "{op} timed out after {}s and its output was not recovered",
                    bound.as_secs()
                )
            }
            GitRunError::Failed { op, message } => write!(f, "{op} failed: {message}"),
            GitRunError::Spawn { op, message } => write!(f, "failed to spawn {op}: {message}"),
        }
    }
}

/// Runs a subprocess under a wall-clock deadline and returns
/// `(status, stdout, stderr)`, honoring the deadline on every path.
/// Both pipes are drained on helper threads because `read_to_end`
/// blocks: draining on the polling thread could deadlock against a
/// child whose pipe buffer fills, turning a fast child into a spurious
/// timeout (the governance.rs `wait_child_bounded` comment states the
/// same contract). The child spawns as its own process group leader
/// (`process_group(0)`, the rm-583 arm), so a child still alive at the
/// deadline is killed GROUP-WIDE and reported as
/// [`GitRunError::Timeout`]: the grandchildren that inherit a pipe
/// write-end (git-remote-https, a credential helper, an exec'd hook)
/// die with the group instead of holding the drains hostage, and a
/// grandchild that escaped the group (`setsid`) still cannot wedge the
/// call — the return happens without joining the drains, which finish
/// (or die) on their own when the last write-end closes, exactly the
/// bound-over-drain contract governance's `git_commits` established. A
/// child that has already exited gets only the remainder of the
/// deadline for its drains: if its bytes cannot be recovered in time
/// the same named timeout surfaces (the rm-731 fast-exit arm; the
/// 2026-10-07 assess PoC measured 25,106 ms on a 10 s bound here).
/// Drain threads that cannot finish are abandoned — each dies when its
/// pipe finally EOFs or with the process — never waited on unboundedly.
fn run_bounded(
    program: &str,
    args: &[&str],
    bound: Duration,
) -> Result<(std::process::ExitStatus, String, String), GitRunError> {
    let op = format!("{program} {}", args.join(" "));
    let mut command = Command::new(program);
    command
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    // rm-583: own process group so a deadline kill can take the whole
    // tree down in one signal — killing only the direct child leaves
    // grandchildren holding the pipe write-ends.
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        command.process_group(0);
    }
    let mut child = command.spawn().map_err(|err| GitRunError::Spawn {
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
            // rm-583: the deadline contract outranks drain-thread hygiene,
            // and the kill takes the whole process group — the child became
            // its own group leader at spawn, so a forked helper that
            // inherited a pipe write-end dies with it instead of holding
            // the drains hostage. Even a grandchild that escaped the group
            // (setsid) cannot wedge the call: return WITHOUT joining the
            // drains (the governance.rs wait_child_bounded precedent) and
            // let the two drain threads die when the orphaned helper
            // finally closes the pipes (or with the process, which exits on
            // this error path).
            kill_process_tree(&mut child);
            let _ = child.wait();
            return Err(GitRunError::Timeout { op, bound });
        }
        std::thread::sleep(Duration::from_millis(25));
    };
    // rm-731: the child is gone, but a forked helper may still hold a
    // pipe write-end, so the drains get only the remainder of the bound
    // — the old unbounded joins blocked here for as long as the helper
    // kept the pipe open (25,106 ms on a 10 s bound in the assess PoC).
    let stdout = recv_drain_bounded(stdout, deadline);
    let stderr = recv_drain_bounded(stderr, deadline);
    let (stdout, stderr) = match (stdout, stderr) {
        (Ok(stdout), Ok(stderr)) => (stdout, stderr),
        _ => {
            // The helper's bytes are unrecoverable within the bound: the
            // drains are abandoned (each dies when its pipe finally EOFs
            // or with the process) — the rm-583 timeout-arm precedent,
            // extended to the happy path — and the named timeout keeps
            // the deadline promise honest.
            return Err(GitRunError::Timeout { op, bound });
        }
    };
    let stdout = String::from_utf8_lossy(&stdout).to_string();
    let stderr = String::from_utf8_lossy(&stderr).to_string();
    if !status.success() {
        return Err(GitRunError::Failed {
            op,
            message: snippet(&stderr),
        });
    }
    Ok((status, stdout, stderr))
}

/// rm-583: kill the child's whole process tree. `process_group(0)` at
/// spawn made the child its own group leader, so signalling the
/// negated pid takes every descendant that stayed in the group — the
/// grandchildren that inherit the pipe write-ends die with it instead
/// of holding the drain threads hostage.
#[cfg(unix)]
fn kill_process_tree(child: &mut std::process::Child) {
    // SAFETY: libc::kill only enqueues a signal; the pid is the child's
    // own process group (it became the leader at spawn, before exec, so
    // no grandchild can have raced into it), and the child has not been
    // reaped yet, so the id cannot be recycled.
    unsafe {
        libc::kill(-(child.id() as libc::pid_t), libc::SIGKILL);
    }
    // Belt for a raced setpgid (killpg ESRCH): the direct child must die
    // regardless of the group outcome.
    let _ = child.kill();
}

/// rm-583, non-unix arm: no process-group primitive exists; killing the
/// direct child is the best available bound and the unjoined drains keep
/// the call from blocking on whatever survives it.
#[cfg(not(unix))]
fn kill_process_tree(child: &mut std::process::Child) {
    let _ = child.kill();
}

fn spawn_drain<R: Read + Send + 'static>(pipe: Option<R>) -> std::sync::mpsc::Receiver<Vec<u8>> {
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let mut bytes = Vec::new();
        if let Some(mut stream) = pipe {
            let _ = stream.read_to_end(&mut bytes);
        }
        // On an abandoned drain the receiver is gone — the send errors
        // and this thread exits as soon as its pipe finally EOFs.
        let _ = tx.send(bytes);
    });
    rx
}

/// Bounded drain join (rm-731): the reader's bytes if they arrive before
/// the run's deadline, `Err` otherwise — the reader thread is abandoned
/// on `Err`, mirroring the rm-583 timeout arm's dropped joins.
fn recv_drain_bounded(
    rx: std::sync::mpsc::Receiver<Vec<u8>>,
    deadline: Instant,
) -> Result<Vec<u8>, std::sync::mpsc::RecvTimeoutError> {
    let now = Instant::now();
    let wait = if deadline > now {
        deadline - now
    } else {
        Duration::ZERO
    };
    rx.recv_timeout(wait)
}

const DEFAULT_REMOTE: &str = "upstream";
const DEFAULT_REF: &str = "master";
/// Distribution channel for this fork (README install line). The repo
/// itself ships no npm manifest; the registry is the source of truth.
const NPM_PACKAGE: &str = "@zack78/agenttrace";
const NPM_REGISTRY_URL: &str = "https://registry.npmjs.org/@zack78%2fagenttrace/latest";

/// One-line network disclosure for the `--fetch` path (rm-404). Lists every
/// network touch the flag triggers, derived from the same constants the code
/// uses, so the disclosure cannot drift from the behavior it describes.
fn fetch_disclosure_line(remote_url: &str) -> String {
    format!(
        "network: --fetch upstream is opt-in — `git fetch {remote_url}` plus one HTTPS request to {NPM_REGISTRY_URL} (see PRIVACY.md)"
    )
}
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

    // Fail fast with the same hints scripts/upstream-delta gives. rm-543:
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
        // rm-404: PRIVACY.md promises fully-offline-by-default, so every opt-in
        // network touch announces itself on stderr before any request goes out.
        eprintln!("{}", fetch_disclosure_line(&remote_url));
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
            Ok(version) => format!("{NPM_PACKAGE}@{version} (registry)"),
            Err(failure) => failure.label().to_string(),
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
    // rm-543: `--fetch` is this module's one opt-in network action and it
    // used to wait on `Command::output()` without a bound — a wedged
    // fetch (hung remote, stale mount) hung the whole CLI. It now runs
    // under UPSTREAM_FETCH_TIMEOUT; a timeout is terminal and names the
    // deadline, the operation, and the remote.
    match run_bounded("git", &["fetch", remote, "--quiet"], UPSTREAM_FETCH_TIMEOUT) {
        Ok(_) => Ok(()),
        Err(GitRunError::Timeout { op, bound }) => bail!(
            "{op} timed out after {}s (UPSTREAM_FETCH_TIMEOUT): the fetch's output \
             was not recovered within the bound — it may even have completed; \
             retry when the remote is responsive — no drift numbers are reported from a partial fetch",
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
            "{op} timed out after {}s (UPSTREAM_GIT_TIMEOUT): the probe's output was \
             not recovered within the bound — the upstream report is aborted, never \
             rendered from a partial repository",
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

/// Ceiling for the npm registry metadata read (rm-052). The version
/// document is a few KB; 256 KiB leaves orders of magnitude of slack
/// while bounding the body a hostile or MITM'd endpoint can push
/// through the subprocess pipe. The ceiling is enforced on OUR side of
/// the pipe (`read_capped` + process-group kill) because curl 7.81.0
/// honors `--max-filesize` only when the response declares a size —
/// chunked, size-less responses sail past it (review ff418327 F2 live
/// PoC: 1,100,014 bytes delivered under this cap with rc=0).
/// `--max-filesize` stays in the argv as the early abort for
/// size-declaring endpoints (exit 63).
const NPM_REGISTRY_MAX_BYTES: u64 = 256 * 1024;

/// rm-052: why an npm registry probe produced no version. These stay
/// distinct so `agenttrace --fetch upstream` no longer labels an absent
/// probe tool the same as a registry failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum NpmProbeFailure {
    /// curl was not found on PATH — the only spawn error that means the
    /// tooling is absent. Every other spawn failure (permissions,
    /// resources) is a `ProbeFailed`, not tool absence (review ff418327 F5).
    ToolAbsent,
    /// curl could not produce a usable response: network error,
    /// HTTP >= 400 (`-f`), timeout, an unspawnable-for-another-reason
    /// curl, or a read error on the response pipe.
    ProbeFailed,
    /// The response pushed past the byte ceiling — either curl aborted
    /// with exit 63 (size-declaring endpoint) or our own capped read
    /// tripped on a size-less stream.
    ResponseOverflow,
    /// curl succeeded but the metadata carried no usable version.
    Unparseable,
}

impl NpmProbeFailure {
    fn label(self) -> &'static str {
        match self {
            NpmProbeFailure::ToolAbsent => "unavailable (probe tooling absent: curl not on PATH)",
            NpmProbeFailure::ProbeFailed => {
                "unavailable (registry probe failed: network or HTTP error)"
            }
            NpmProbeFailure::ResponseOverflow => {
                "unavailable (registry probe failed: response exceeded byte cap)"
            }
            NpmProbeFailure::Unparseable => "unknown (registry returned unparseable metadata)",
        }
    }
}

/// curl argv for the registry probe, split out so tests can pin the
/// bounding flags without shelling out.
fn npm_registry_curl_args() -> Vec<String> {
    [
        "-fsSL".to_string(),
        "--max-time".to_string(),
        "15".to_string(),
        "--max-filesize".to_string(),
        NPM_REGISTRY_MAX_BYTES.to_string(),
        NPM_REGISTRY_URL.to_string(),
    ]
    .into()
}

/// Extract the `version` field from an npm registry metadata document.
/// Split out so the parser is testable hermetically (no network).
fn parse_npm_registry_version(body: &str) -> Option<String> {
    let doc = serde_json::from_str::<serde_json::Value>(body).ok()?;
    doc.get("version")?.as_str().map(str::to_string)
}

/// Read `reader` to EOF, stopping as soon as more than `cap` bytes
/// have been seen (review fix ff418327 F2). Returns the bytes read
/// (at most one chunk past the ceiling) and whether the cap was
/// exceeded. This — not curl's `--max-filesize` — is the byte bound:
/// curl 7.81.0 ignores `--max-filesize` for responses without
/// Content-Length, so a hostile endpoint streaming chunked bodies is
/// bounded only by what WE are willing to buffer.
fn read_capped<R: Read>(mut reader: R, cap: u64) -> std::io::Result<(Vec<u8>, bool)> {
    let mut body = Vec::new();
    let mut chunk = [0u8; 16 * 1024];
    loop {
        let n = reader.read(&mut chunk)?;
        if n == 0 {
            return Ok((body, false));
        }
        body.extend_from_slice(&chunk[..n]);
        if body.len() as u64 > cap {
            return Ok((body, true));
        }
    }
}

/// npm registry probe behind `--fetch` only. Degrades to an `Err` label
/// (never fails the report) when curl is absent or the registry answers
/// badly — with the failure cause kept distinct (rm-052) and the body
/// read byte-bounded at OUR end of the pipe: stdout is piped and read
/// through `read_capped`, and the subprocess tree is killed the moment
/// it pushes past the ceiling (curl's `--max-filesize` alone does NOT
/// bound size-less responses — review ff418327 F2; exit 63 remains the
/// early-abort signal for size-declaring endpoints).
fn npm_registry_version() -> Result<String, NpmProbeFailure> {
    let mut command = Command::new("curl");
    command
        .args(npm_registry_curl_args())
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        // stderr is deliberately null: the probe labels by exit code and
        // body, never by curl's stderr text.
        .stderr(Stdio::null());
    // rm-583 discipline: own process group so the overflow kill takes
    // the whole subprocess tree down in one signal.
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        command.process_group(0);
    }
    let mut child = command.spawn().map_err(|err| {
        // Review fix (ff418327 F5): only NotFound means "curl is not
        // installed" — anything else is a probe failure.
        if err.kind() == std::io::ErrorKind::NotFound {
            NpmProbeFailure::ToolAbsent
        } else {
            NpmProbeFailure::ProbeFailed
        }
    })?;
    let (body, exceeded) = match child.stdout.take() {
        Some(pipe) => {
            read_capped(pipe, NPM_REGISTRY_MAX_BYTES).map_err(|_| NpmProbeFailure::ProbeFailed)?
        }
        // Unreachable with Stdio::piped above; fail as a probe failure,
        // never panic the report.
        None => return Err(NpmProbeFailure::ProbeFailed),
    };
    if exceeded {
        // The endpoint streamed past the ceiling without declaring a
        // size — the exact case curl's --max-filesize cannot see. Kill
        // the tree; the label is the same ResponseOverflow the exit-63
        // path produces.
        kill_process_tree(&mut child);
    }
    let status = child.wait().map_err(|_| NpmProbeFailure::ProbeFailed)?;
    if exceeded || status.code() == Some(63) {
        return Err(NpmProbeFailure::ResponseOverflow);
    }
    if !status.success() {
        return Err(NpmProbeFailure::ProbeFailed);
    }
    parse_npm_registry_version(&String::from_utf8_lossy(&body)).ok_or(NpmProbeFailure::Unparseable)
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
    fn rm052_curl_probe_args_are_time_and_byte_bounded() {
        // rm-052: the registry probe must be bounded in BOTH dimensions —
        // the wall row pins the timeout at 15s and the body read at
        // 256 KiB. --max-filesize surfaces as curl exit 63, mapped to
        // ResponseOverflow (see npm_registry_version).
        let args = npm_registry_curl_args();
        assert_eq!(args[0], "-fsSL");
        assert_eq!(args[1], "--max-time");
        assert_eq!(args[2], "15");
        assert_eq!(args[3], "--max-filesize");
        assert_eq!(args[4], "262144", "256 KiB byte ceiling");
        assert!(args.last().is_some_and(|u| u.starts_with("https://")));
    }

    #[test]
    fn rm052_read_capped_enforces_the_ceiling_without_content_length() {
        // Review fix (ff418327 F2): curl 7.81.0 ignores --max-filesize
        // for responses with no Content-Length, so the byte ceiling is
        // enforced by read_capped at OUR end of the pipe. Hermetic pin:
        // a size-less stream past the ceiling trips the cap after at
        // most one read chunk of slack, and an under-cap body is
        // returned whole.
        let cap = NPM_REGISTRY_MAX_BYTES;
        let oversized = vec![b'x'; cap as usize + 64 * 1024];
        let (body, exceeded) = read_capped(std::io::Cursor::new(oversized), cap).unwrap();
        assert!(exceeded, "a stream past the ceiling must trip the cap");
        assert!(
            body.len() as u64 <= cap + 16 * 1024,
            "at most one 16 KiB read chunk past the ceiling may be buffered"
        );
        let under = vec![b'x'; 1024];
        let (body, exceeded) = read_capped(std::io::Cursor::new(under), cap).unwrap();
        assert!(!exceeded, "an under-cap body must not trip the cap");
        assert_eq!(body.len(), 1024, "an under-cap body is returned whole");
    }

    #[test]
    fn rm052_parse_npm_registry_version_extracts_version_field() {
        assert_eq!(
            parse_npm_registry_version(r#"{"name":"agenttrace","version":"0.10.1"}"#),
            Some("0.10.1".to_string())
        );
        assert_eq!(parse_npm_registry_version("{}"), None);
        assert_eq!(parse_npm_registry_version("not json"), None);
        assert_eq!(parse_npm_registry_version(r#"{"version":42}"#), None);
    }

    #[test]
    fn rm052_probe_failure_labels_are_distinct() {
        // The conflation this row exists to remove: "curl absent" and
        // "registry answered badly" must never share one label.
        let labels = [
            NpmProbeFailure::ToolAbsent.label(),
            NpmProbeFailure::ProbeFailed.label(),
            NpmProbeFailure::ResponseOverflow.label(),
            NpmProbeFailure::Unparseable.label(),
        ];
        assert_eq!(
            labels.len(),
            labels
                .iter()
                .collect::<std::collections::HashSet<_>>()
                .len(),
            "labels must be pairwise distinct"
        );
        assert_eq!(
            NpmProbeFailure::ToolAbsent.label(),
            "unavailable (probe tooling absent: curl not on PATH)"
        );
        assert!(NpmProbeFailure::ProbeFailed
            .label()
            .contains("network or HTTP error"));
        assert!(NpmProbeFailure::ResponseOverflow
            .label()
            .contains("byte cap"));
    }

    #[test]
    fn upstream_subprocess_timeouts_are_pinned_to_the_stated_classes() {
        // rm-543: the two deadline classes are contract, not tuning —
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

    #[cfg(unix)]
    #[test]
    fn run_bounded_kills_pipe_holding_grandchildren_at_the_deadline() {
        // rm-583: the PoC shape (assess pocD.sh) — a hung child that
        // spawned orphans inheriting the pipe write-ends. Killing only
        // the direct child left the orphans holding the drains open,
        // and the unconditional join hung the CLI far past its own
        // bound (a 10s deadline was still running at 25s). The child
        // now leads its own process group and the deadline kill
        // signals the whole group, so the pipes close and the call
        // returns at the bound instead of when the orphans exit.
        let started = Instant::now();
        let result = run_bounded(
            "sh",
            &["-c", "sleep 10 & sleep 10"],
            Duration::from_millis(300),
        );
        assert!(
            matches!(result, Err(GitRunError::Timeout { .. })),
            "expected Timeout, got {result:?}"
        );
        assert!(
            started.elapsed() < Duration::from_secs(3),
            "the group kill, not the orphans' lifetime, bounds the wait"
        );
    }

    #[cfg(unix)]
    #[test]
    fn run_bounded_returns_at_the_deadline_when_a_grandchild_escapes_the_group() {
        // rm-583's second half: a grandchild that escaped the process
        // group (setsid — the ssh/askpass/git-remote fetch shape) still
        // holds the pipe write-ends after the group kill. The bound
        // must outrank the drain: the call returns at the deadline
        // WITHOUT joining the drain threads, the same contract
        // governance's git_commits established (rm-242). The escaped
        // orphan dies on its own timer; it may never hold the CLI.
        let started = Instant::now();
        let result = run_bounded(
            "sh",
            &["-c", "setsid sleep 10 & sleep 10"],
            Duration::from_millis(300),
        );
        assert!(
            matches!(result, Err(GitRunError::Timeout { .. })),
            "expected Timeout, got {result:?}"
        );
        assert!(
            started.elapsed() < Duration::from_secs(3),
            "no join may wait out an escaped grandchild"
        );
    }

    #[test]
    fn run_bounded_timeout_survives_a_grandchild_holding_the_pipes() {
        // rm-583 regression: a killed child's forked helper (what a wedged
        // git-remote-https looks like) inherits the pipe write-ends, so
        // joining the drain threads would block on read_to_end until the
        // grandchild exits — unboundedly. The named timeout must still
        // surface near the bound; the leaked drains are accepted (process
        // exit reaps the orphan holding the pipes).
        let started = Instant::now();
        let result = run_bounded(
            "sh",
            &["-c", "sleep 30 & exec sleep 30"],
            Duration::from_secs(1),
        );
        match result {
            Err(GitRunError::Timeout { op, bound }) => {
                assert!(op.contains("sleep"), "timeout names the operation: {op}");
                assert_eq!(bound, Duration::from_secs(1));
            }
            other => panic!("expected Timeout, got {other:?}"),
        }
        assert!(
            started.elapsed() < Duration::from_secs(10),
            "grandchild-held pipes must not block past the bound, took {:?}",
            started.elapsed()
        );
    }

    #[test]
    fn run_bounded_fast_exit_survives_a_grandchild_holding_the_pipes() {
        // rm-731 regression: the assess PoC's PATH-shim git — a helper
        // holding both pipe write-ends past the child's rc-0 exit — made
        // the fast-exit arm's unbounded drain joins block for the
        // helper's lifetime (25,106 ms on a 10 s bound) with no error.
        // Reduced to its essence here, without the PATH mutation: the
        // shape is `sleep 60 & exec true`, a holder outliving the exited
        // child. The drains must be bounded — the named timeout surfaces
        // at the bound and the call returns, never a hang.
        let started = std::time::Instant::now();
        let result = run_bounded(
            "sh",
            &["-c", "echo probe-ok; sleep 60 & exec true"],
            Duration::from_secs(1),
        );
        match result {
            Err(GitRunError::Timeout { op, bound }) => {
                assert!(op.contains("probe"), "op names the command: {op}");
                assert_eq!(bound, Duration::from_secs(1));
            }
            other => panic!("expected the bounded timeout, got {other:?}"),
        }
        assert!(
            started.elapsed() < Duration::from_secs(5),
            "the fast-exit arm must not wait on the holder past the bound (took {:?})",
            started.elapsed()
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

    /// rm-404: PRIVACY.md must enumerate every opt-in network touch this
    /// module performs, derived from the same constants the code uses
    /// (mirrors rm-086's `privacy_disclosure_lists_every_artifact` pin in
    /// core). If a new network touch lands here, this fails until
    /// PRIVACY.md names it.
    #[test]
    fn privacy_disclosure_lists_every_network_touch() {
        let privacy =
            std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../../PRIVACY.md"))
                .expect("PRIVACY.md readable");

        // The npm registry probe URL, constructor-derived from the const.
        assert!(
            privacy.contains(NPM_REGISTRY_URL),
            "PRIVACY.md must name the npm registry probe URL {NPM_REGISTRY_URL}"
        );

        // The git fetch touch and its trigger flag.
        assert!(
            privacy.contains("git fetch") && privacy.contains("--fetch upstream"),
            "PRIVACY.md must name the `git fetch` touch and its `--fetch upstream` trigger"
        );

        // The false "only exception" phrasing must stay dead.
        assert!(
            !privacy.contains("only exception"),
            "PRIVACY.md must not claim --update-pricing is the only network exception"
        );

        // The runtime disclosure line lists both touches, derived from the
        // same constants the fetch path uses.
        let line = fetch_disclosure_line("https://github.com/example/upstream.git");
        assert!(line.contains("git fetch"), "disclosure must name git fetch");
        assert!(
            line.contains(NPM_REGISTRY_URL),
            "disclosure must name the npm probe URL"
        );
        assert!(
            line.contains("PRIVACY.md"),
            "disclosure must point at PRIVACY.md"
        );
    }
}
