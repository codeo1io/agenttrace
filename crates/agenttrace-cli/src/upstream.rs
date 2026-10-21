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

use agenttrace_core::sanitize_line_segment;
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
/// rm-052's git lane — the sibling bound of NPM_REGISTRY_MAX_BYTES. A
/// hostile or MITM'd remote relays unbounded sideband ("remote:")
/// text through git's pipes, so the drains are byte-bounded at OUR end
/// of the pipe and the subprocess tree is killed the moment it pushes
/// past the ceiling — the same discipline the pricing probe applies to
/// a hostile registry. Probe-sized by design: rev-parse and ls-remote
/// emit bytes, not kilobytes.
const UPSTREAM_GIT_MAX_BYTES: u64 = 256 * 1024;

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
    /// rm-052's git lane: the child pushed more than `cap_bytes` through
    /// one of its pipes. The tree was killed the moment the drain saw the
    /// overrun and the excess bytes were never buffered — a remote that
    /// floods the sideband channel cannot grow agenttrace's memory, only
    /// fail the upstream report loudly.
    OverCap { op: String, cap_bytes: u64 },
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
            GitRunError::OverCap { op, cap_bytes } => {
                write!(
                    f,
                    "{op} pushed more than {cap_bytes} bytes through its pipes and was \
                     killed (UPSTREAM_GIT_MAX_BYTES)"
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
    max_bytes: u64,
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
    let mut stdout_rx = spawn_drain(child.stdout.take(), max_bytes);
    let mut stderr_rx = spawn_drain(child.stderr.take(), max_bytes);
    let deadline = Instant::now() + bound;
    // rm-052: completed-but-under-cap outcomes consumed by the in-loop
    // check are stashed so the post-loop recv never blocks on an already
    // drained pipe.
    let mut stdout_stash: Option<DrainOutcome> = None;
    let mut stderr_stash: Option<DrainOutcome> = None;
    let status = loop {
        if let Ok(Some(status)) = child.try_wait() {
            break status;
        }
        // rm-052: the npm probe's discipline — the tree is killed the
        // moment a drain sees the overrun, not after the child's natural
        // exit. A finished-under-cap drain parks its bytes in the stash.
        for (rx, stash) in [
            (&mut stdout_rx, &mut stdout_stash),
            (&mut stderr_rx, &mut stderr_stash),
        ] {
            if stash.is_none() {
                if let Ok(outcome) = rx.try_recv() {
                    if outcome.exceeded {
                        kill_process_tree(&mut child);
                        let _ = child.wait();
                        return Err(GitRunError::OverCap {
                            op,
                            cap_bytes: max_bytes,
                        });
                    }
                    *stash = Some(outcome);
                }
            }
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
    let stdout = match stdout_stash {
        Some(outcome) => Ok(outcome),
        None => recv_drain_bounded(stdout_rx, deadline),
    };
    let stderr = match stderr_stash {
        Some(outcome) => Ok(outcome),
        None => recv_drain_bounded(stderr_rx, deadline),
    };
    let (stdout, stderr) = match (stdout, stderr) {
        (Ok(stdout), Ok(stderr)) => {
            if stdout.exceeded || stderr.exceeded {
                return Err(GitRunError::OverCap {
                    op,
                    cap_bytes: max_bytes,
                });
            }
            (stdout.bytes, stderr.bytes)
        }
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

/// One drain's outcome. `exceeded` means the writer pushed past the byte
/// ceiling: `bytes` holds what was read up to (and at most one chunk
/// past) the cap, and the caller must treat the overrun as terminal.
struct DrainOutcome {
    bytes: Vec<u8>,
    exceeded: bool,
}

fn spawn_drain<R: Read + Send + 'static>(
    pipe: Option<R>,
    cap: u64,
) -> std::sync::mpsc::Receiver<DrainOutcome> {
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let mut bytes = Vec::new();
        let mut exceeded = false;
        if let Some(mut stream) = pipe {
            // rm-052: chunked reads, never read_to_end — the drain stops
            // buffering at the cap so a flooding writer cannot grow this
            // Vec unboundedly, and the send is abandonable exactly as
            // before (a dropped receiver lets this thread die when its
            // pipe finally EOFs or with the process).
            let mut chunk = vec![0u8; 16 * 1024];
            loop {
                match stream.read(&mut chunk) {
                    Ok(0) => break,
                    Ok(n) => {
                        bytes.extend_from_slice(&chunk[..n]);
                        if bytes.len() as u64 > cap {
                            exceeded = true;
                            break;
                        }
                    }
                    Err(_) => break,
                }
            }
        }
        // On an abandoned drain the receiver is gone — the send errors
        // and this thread exits as soon as its pipe finally EOFs.
        let _ = tx.send(DrainOutcome { bytes, exceeded });
    });
    rx
}

/// Bounded drain join (rm-731): the reader's bytes if they arrive before
/// the run's deadline, `Err` otherwise — the reader thread is abandoned
/// on `Err`, mirroring the rm-583 timeout arm's dropped joins.
fn recv_drain_bounded<T>(
    rx: std::sync::mpsc::Receiver<T>,
    deadline: Instant,
) -> Result<T, std::sync::mpsc::RecvTimeoutError> {
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
/// uses, so the disclosure cannot drift from the behavior it describes
/// (rm-950 added the two opencode vendor-watch probes to this list).
/// rm-979 review rider: the URL is remote-derived — a tracked repo's git
/// config can carry hostile bytes — and stderr is a terminal render too, so
/// it goes through the same control-byte sanitizer as the report body.
/// `sanitize_line_segment` is the identity on every legitimate URL (RFC
/// 3986 admits no control bytes), so the disclosure stays truthful.
fn fetch_disclosure_line(remote_url: &str) -> String {
    format!(
        "network: --fetch upstream is opt-in — `git fetch {remote_url}` plus three HTTPS metadata requests: {NPM_REGISTRY_URL} (published version), {OPENCODE_NPM_URL} and {OPENCODE_RELEASES_URL} (opencode vendor watch; see PRIVACY.md)",
        remote_url = sanitize_line_segment(remote_url)
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
    /// opencode vendor-watch channel state (rm-950, cycle 3): the release
    /// tag from the anomalyco/opencode releases API, with an explicit
    /// `npm channel retired` label once the registry probe 404s — a dead
    /// channel is a state, not a probe failure.
    pub opencode_state: String,
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

/// Stewardship 7e19719f unit-2 (argv hardening, CWE-88): UPSTREAM_REMOTE
/// and UPSTREAM_REF are the only env-derived bytes reaching this module's
/// git argv, and `git fetch` honors `--upload-pack=<cmd>` — so a
/// leading-dash value is argument injection, not a bad remote name.
/// Refuse both at parse, BEFORE any argv is constructed, naming the env
/// var and the value verbatim so the operator sees which knob to fix.
/// Legal values are unaffected: git's own check-ref-format rejects
/// '-'-leading remote and ref names, so no legal value starts with '-'.
fn validate_remote_and_ref(remote: &str, ref_name: &str) -> anyhow::Result<()> {
    for (var, value) in [("UPSTREAM_REMOTE", remote), ("UPSTREAM_REF", ref_name)] {
        if value.starts_with('-') {
            bail!(
                "{var}={value:?} is refused: a leading-dash remote/ref would be \
                 parsed as a git option (argument injection, e.g. --upload-pack) \
                 before any git invocation — no legal git remote/ref name starts \
                 with '-'; set a non-dash value"
            );
        }
    }
    Ok(())
}

fn collect_status(fetch: bool) -> anyhow::Result<UpstreamStatus> {
    let remote = remote_name();
    let ref_name = ref_name();

    // Unit-2 (argv hardening): refuse a hostile remote/ref BEFORE any
    // argv is constructed — ahead of even the show-toplevel probe, so
    // no git subprocess ever spawns on a leading-dash value.
    validate_remote_and_ref(&remote, &ref_name)?;

    // Fail fast with the same hints scripts/upstream-delta gives.
    // rm-543 + rm-579 routing lives in `git_probe`: only a probe that
    // RAN and failed counts as a missing prerequisite, so these hints
    // stay truthful under every failure mode.
    git_probe(
        &["rev-parse", "--show-toplevel"],
        "not inside a git repository",
    )?;
    let remote_url = git_probe(
        // Unit-2: the `--` keeps the env-derived remote in the positional
        // slot (honored by `git remote get-url`; verified live against git
        // 2.34, standard parse-options behavior) — defense in depth under
        // the parse-time refusal in validate_remote_and_ref.
        &["remote", "get-url", "--", &remote],
        &format!("no '{remote}' remote configured (git remote add {remote} <url>)"),
    )?;
    if fetch {
        // rm-404: PRIVACY.md promises fully-offline-by-default, so every opt-in
        // network touch announces itself on stderr before any request goes out.
        eprintln!("{}", fetch_disclosure_line(&remote_url));
        fetch_remote(&remote)?;
    }
    let tracking = format!("refs/remotes/{remote}/{ref_name}");
    git_probe(
        &["rev-parse", "--verify", &tracking],
        &format!("no remote-tracking ref {tracking}; run agenttrace --fetch upstream"),
    )?;

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

    // rm-950: the opencode vendor watch anchors on the releases API of the
    // renamed home (anomalyco/opencode) and reports the retired npm channel
    // explicitly rather than as a probe failure.
    let opencode_state = if fetch {
        opencode_channel_state()
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
        opencode_state,
    })
}

fn fetch_remote(remote: &str) -> anyhow::Result<()> {
    // rm-543: `--fetch` is this module's one opt-in network action and it
    // used to wait on `Command::output()` without a bound — a wedged
    // fetch (hung remote, stale mount) hung the whole CLI. It now runs
    // under UPSTREAM_FETCH_TIMEOUT; a timeout is terminal and names the
    // deadline, the operation, and the remote.
    // Unit-2: `--` before the env-derived remote keeps it positional
    // (`git fetch` assigns the first post-`--` argument to the
    // repository — verified live against a two-remote fixture on git
    // 2.34; stable parse-options behavior) so an option-shaped value
    // can never be interpreted, on top of the parse-time refusal in
    // validate_remote_and_ref.
    match run_bounded(
        "git",
        &["fetch", "--quiet", "--", remote],
        UPSTREAM_FETCH_TIMEOUT,
        UPSTREAM_GIT_MAX_BYTES,
    ) {
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
    let (_, stdout, _) = run_bounded("git", args, UPSTREAM_GIT_TIMEOUT, UPSTREAM_GIT_MAX_BYTES)?;
    Ok(stdout.trim_end().to_string())
}

/// rm-543/rm-579: run one fail-fast prerequisite probe and collapse
/// its failure into the caller's missing-prerequisite hint — routing
/// the failure classes distinctly so the hint stays truthful. Only a
/// probe that RAN and failed (non-zero exit) is a missing
/// prerequisite. A probe that timed out is terminal and names its
/// deadline (rm-543), so a wedged git can never masquerade as "not
/// inside a git repository". A probe that could not even start git
/// (binary absent from PATH, exec failed) is an environment failure,
/// not a repository fact: it must name the spawn instead of collapsing
/// into the hint, so a PATH-less shell is not misdiagnosed as a
/// missing repository/remote/ref (rm-579).
fn git_probe(args: &[&str], missing: &str) -> anyhow::Result<String> {
    match git_bounded(args) {
        Ok(output) => Ok(output),
        Err(err @ GitRunError::Timeout { .. }) => bail!("{err}"),
        Err(err @ GitRunError::Spawn { .. }) => {
            bail!("{err} — git could not be started; is git installed and on PATH?")
        }
        Err(_) => bail!("{missing}"),
    }
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

/// rm-052: why a curl probe in this module produced no body — the npm
/// registry probe (published version) and the opencode vendor watch
/// (rm-950) share these. They stay distinct so `agenttrace --fetch
/// upstream` no longer labels an absent probe tool the same as a probe
/// failure.
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

/// rm-052 discipline for every curl probe in this file (the npm registry
/// probe and rm-950's opencode vendor watch): null stdin, piped stdout
/// read through `read_capped` at OUR end of the pipe (a hostile or
/// MITM'd endpoint cannot grow agenttrace's memory), process-group
/// ownership (rm-583) so the byte-ceiling kill takes the whole
/// subprocess tree down in one signal, `--max-filesize`'s exit 63
/// mapped to `ResponseOverflow` alongside the capped read (curl
/// 7.81.0 honors `--max-filesize` only for size-declaring responses —
/// review ff418327 F2), and only a NotFound spawn meaning "curl is
/// not installed". `args` must carry the wall bound (`--max-time`)
/// and, when the body is wanted, a `--max-filesize` matching `cap`.
fn curl_output_bounded(args: &[String], cap: u64) -> Result<Vec<u8>, NpmProbeFailure> {
    let mut command = Command::new("curl");
    command
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        // stderr is deliberately null: probes label by exit code and
        // body, never by curl's stderr text.
        .stderr(Stdio::null());
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
        Some(pipe) => read_capped(pipe, cap).map_err(|_| NpmProbeFailure::ProbeFailed)?,
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
    Ok(body)
}

/// npm registry probe behind `--fetch` only. Degrades to an `Err` label
/// (never fails the report) when curl is absent or the registry answers
/// badly — with the failure cause kept distinct (rm-052) and the body
/// read byte-bounded at OUR end of the pipe via the shared
/// `curl_output_bounded` runner (`--max-filesize` surfaces as curl exit
/// 63, mapped to ResponseOverflow; the capped read catches the
/// size-less responses curl's flag cannot see).
fn npm_registry_version() -> Result<String, NpmProbeFailure> {
    let body = curl_output_bounded(&npm_registry_curl_args(), NPM_REGISTRY_MAX_BYTES)?;
    parse_npm_registry_version(&String::from_utf8_lossy(&body)).ok_or(NpmProbeFailure::Unparseable)
}

/// opencode vendor-watch authority (rm-950, cycle 3). The npm channel the
/// watch originally anchored on is retired (registry.npmjs.org/opencode and
/// /@opencode-ai/opencode both 404); the releases API of the renamed home,
/// github.com/anomalyco/opencode, is the authority for latest.
const OPENCODE_RELEASES_URL: &str =
    "https://api.github.com/repos/anomalyco/opencode/releases/latest";
const OPENCODE_NPM_URL: &str = "https://registry.npmjs.org/opencode";

/// Ceiling for the opencode releases-API read (rm-950), the sibling bound
/// of NPM_REGISTRY_MAX_BYTES: a `releases/latest` document is a few KB, and
/// the ceiling is enforced on OUR side of the pipe through the shared
/// `curl_output_bounded` runner, so a hostile or MITM'd endpoint cannot
/// grow agenttrace's memory through the vendor-watch probe.
const OPENCODE_RELEASES_MAX_BYTES: u64 = 256 * 1024;

/// curl argv for the opencode releases probe, split out so tests can pin
/// the bounding flags without shelling out (same pin style as
/// `npm_registry_curl_args`, rm-052).
fn opencode_releases_curl_args(url: &str) -> Vec<String> {
    [
        "-fsSL".to_string(),
        "--max-time".to_string(),
        "15".to_string(),
        "--max-filesize".to_string(),
        OPENCODE_RELEASES_MAX_BYTES.to_string(),
        url.to_string(),
    ]
    .into()
}

/// HTTP status of `url` via curl, `None` when curl is absent or the
/// request itself fails (offline). A 404 here means "channel retired",
/// not "probe broke". Runs under the shared rm-052 discipline: the body
/// goes to `/dev/null`, so the piped stdout is the `%{http_code}` render
/// only — still read through `read_capped` at OUR end of the pipe, never
/// an unbounded `.output()` capture.
fn http_status_via_curl(url: &str) -> Option<u32> {
    let args: Vec<String> = [
        "-sSL",
        "-o",
        "/dev/null",
        "-w",
        "%{http_code}",
        "--max-time",
        "15",
        url,
    ]
    .iter()
    .map(|arg| arg.to_string())
    .collect();
    let body = curl_output_bounded(&args, NPM_REGISTRY_MAX_BYTES).ok()?;
    String::from_utf8_lossy(&body).trim().parse::<u32>().ok()
}

/// `tag_name` from a GitHub releases/latest JSON body.
fn parse_release_tag(body: &str) -> Option<String> {
    let value = serde_json::from_str::<serde_json::Value>(body).ok()?;
    value.get("tag_name")?.as_str().map(str::to_string)
}

/// Pure classification so the retired-channel contract is testable
/// offline: `Some(404)` from the npm registry + a live tag from the
/// releases API renders `tag (npm channel retired; authority:
/// anomalyco/opencode releases)` — never a bare failure.
fn classify_opencode_channel(registry_status: Option<u32>, release_tag: Option<&str>) -> String {
    let retired = registry_status == Some(404);
    match (retired, release_tag) {
        (true, Some(tag)) => {
            format!("{tag} (npm channel retired; authority: anomalyco/opencode releases)")
        }
        (true, None) => "channel retired; releases authority unreachable (network?)".to_string(),
        (false, Some(tag)) => tag.to_string(),
        (false, None) => "unknown (offline; agenttrace --fetch upstream)".to_string(),
    }
}

fn opencode_channel_state() -> String {
    classify_opencode_channel(
        http_status_via_curl(OPENCODE_NPM_URL),
        github_latest_release_tag(OPENCODE_RELEASES_URL).as_deref(),
    )
}

fn github_latest_release_tag(url: &str) -> Option<String> {
    // rm-052 discipline via the shared runner: wall bound, byte cap on
    // OUR side of the pipe, process-group kill on overflow — a raw
    // `.output()` here would re-open the unbounded-body lane the npm
    // probe closed (the probes were originally written against a tree
    // before that discipline landed; integration re-binds them to it).
    let body = curl_output_bounded(
        &opencode_releases_curl_args(url),
        OPENCODE_RELEASES_MAX_BYTES,
    )
    .ok()?;
    parse_release_tag(&String::from_utf8_lossy(&body))
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
    /// Text view. rm-979 (run 1ecd791b cycle 1, minted
    /// campaign-locally as rm-594, rebound at integration — the
    /// landed rm-594 is the journal-derived counter-key cap row):
    /// every free-form string that originates
    /// OUTSIDE this machine — the remote URL, the merge-base subject,
    /// each unported commit subject, and the registry-probed npm state
    /// (plus rm-950's releases-probed opencode state — its tag comes
    /// from an HTTP JSON body, not a git ref, so it is exactly the
    /// class this boundary exists for) — passes through `sanitize_line_segment` at this render
    /// boundary like every other hostile-bytes text sink in the CLI
    /// (rm-034/rm-383 family: control bytes become U+FFFD, so a
    /// hostile upstream commit cannot carry an OSC-52 clipboard write
    /// or CSI sequence onto the operator's terminal). Strings git
    /// itself constrains stay raw: `remote`/`ref_name` come from
    /// operator config (`UPSTREAM_REMOTE`/`UPSTREAM_REF`), `base_date`
    /// is `--format=%cs`, release tags are ref names (git's ref format
    /// forbids control bytes), and areas come from the static
    /// `classify_area` table. The JSON view deliberately skips the
    /// sanitizer — serde escaping already encodes control bytes
    /// losslessly (`\u001b`) — and its keys are append-only, per the
    /// documented schema.
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
            self.remote,
            self.ref_name,
            sanitize_line_segment(&self.remote_url)
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
            sanitize_line_segment(&self.base_subject)
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
                sanitize_line_segment(&commit.subject)
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
            npm_state = sanitize_line_segment(&self.npm_state)
        ));
        // rm-950: the vendor-watch state joins the same render boundary
        // discipline — its tag is remote-derived (releases JSON), so it
        // sanitizes like npm_state rather than riding raw onto the
        // terminal (rm-979 family).
        out.push_str(&format!(
            "  opencode:         {}\n",
            sanitize_line_segment(&self.opencode_state)
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
                "opencode_state": self.opencode_state,
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
mod opencode_channel_tests {
    use super::{classify_opencode_channel, opencode_releases_curl_args, parse_release_tag};

    #[test]
    fn opencode_releases_probe_args_are_time_and_byte_bounded() {
        // rm-052 parity pin: the vendor-watch releases probe rides the
        // same bounding discipline as the npm registry probe — 15s wall,
        // 256 KiB `--max-filesize` early abort, and (pinned by the shared
        // `curl_output_bounded` runner its argv feeds) a capped read on
        // OUR side of the pipe with a process-group kill on overflow.
        let args = opencode_releases_curl_args(super::OPENCODE_RELEASES_URL);
        assert_eq!(args[0], "-fsSL");
        assert_eq!(args[1], "--max-time");
        assert_eq!(args[2], "15");
        assert_eq!(args[3], "--max-filesize");
        assert_eq!(args[4], "262144", "256 KiB byte ceiling");
        assert_eq!(
            args.last().map(String::as_str),
            Some(super::OPENCODE_RELEASES_URL)
        );
    }

    #[test]
    fn parse_release_tag_reads_tag_name() {
        let body = r#"{"tag_name": "v1.18.35", "name": "v1.18.35"}"#;
        assert_eq!(parse_release_tag(body).as_deref(), Some("v1.18.35"));
        assert_eq!(parse_release_tag("{\"name\": \"no tag\"}"), None);
        assert_eq!(parse_release_tag("not json"), None);
        assert_eq!(parse_release_tag(""), None);
    }

    #[test]
    fn retired_channel_renders_as_state_not_failure() {
        // rm-950 contract: a 404 registry is "retired", and when the
        // releases authority answers, the tag leads.
        assert_eq!(
            classify_opencode_channel(Some(404), Some("v1.18.35")),
            "v1.18.35 (npm channel retired; authority: anomalyco/opencode releases)"
        );
        assert_eq!(
            classify_opencode_channel(Some(404), None),
            "channel retired; releases authority unreachable (network?)"
        );
    }

    #[test]
    fn live_or_absent_registry_renders_plainly() {
        assert_eq!(
            classify_opencode_channel(Some(200), Some("v1.18.35")),
            "v1.18.35"
        );
        assert_eq!(
            classify_opencode_channel(None, Some("v1.18.35")),
            "v1.18.35"
        );
        assert_eq!(
            classify_opencode_channel(None, None),
            "unknown (offline; agenttrace --fetch upstream)"
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Stewardship 7e19719f unit-2 (argv hardening, CWE-88): UPSTREAM_REMOTE
    /// and UPSTREAM_REF are the only env-derived bytes in this module's git
    /// argv, and `git fetch` honors `--upload-pack=<cmd>` — so a hostile
    /// value is argument injection, not a bad remote name. Both must be
    /// refused at parse, naming the env var AND the value verbatim, BEFORE
    /// any git argv is constructed. Red at base: git's own parser was the
    /// only line of defense (`error: unknown option 'upload-pack=…'` for
    /// `remote get-url`; on the fetch lane the shape is a live option).
    /// One test, sequential arms, env restored eagerly — no other test in
    /// this binary reads these vars (pinned by grep at landing).
    #[test]
    fn leading_dash_upstream_env_is_refused_before_argv_construction() {
        let hostile = "--upload-pack=touch /tmp/at-u2-refused-marker";
        for var in ["UPSTREAM_REMOTE", "UPSTREAM_REF"] {
            std::env::set_var(var, hostile);
            let err = match collect_status(false) {
                Ok(_) => panic!("leading-dash {var} must be refused"),
                Err(err) => err,
            };
            std::env::remove_var(var);
            let msg = format!("{err:#}");
            assert!(
                msg.contains(var),
                "{var} refusal must name the env var: {msg}"
            );
            assert!(
                msg.contains(hostile),
                "{var} refusal must echo the value verbatim: {msg}"
            );
        }
    }

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
        // rm-052's git lane: the byte ceiling is contract, not tuning —
        // the sibling of NPM_REGISTRY_MAX_BYTES (both 256 KiB, both
        // enforced our-side of the pipe).
        assert_eq!(UPSTREAM_GIT_MAX_BYTES, 256 * 1024);
    }

    #[test]
    fn run_bounded_kills_a_child_pushing_past_the_byte_cap() {
        // rm-052's git lane: a writer that floods a pipe past the byte
        // ceiling is killed and surfaced as a named OverCap — the drain
        // stops buffering at the cap instead of reading to EOF, so a
        // hostile remote relaying unbounded sideband text cannot grow
        // agenttrace's memory, only fail the report loudly. 1 MiB of
        // zeros against a 4 KiB cap: the child blocks on the pipe once
        // the drain stops reading, the in-loop check sees the overrun,
        // and the tree is killed well inside the deadline.
        let started = Instant::now();
        let result = run_bounded(
            "sh",
            &["-c", "head -c 1048576 /dev/zero; sleep 30"],
            Duration::from_secs(30),
            4 * 1024,
        );
        match result {
            Err(GitRunError::OverCap { op, cap_bytes }) => {
                assert!(op.contains("head"), "over-cap names the operation: {op}");
                assert_eq!(cap_bytes, 4 * 1024);
            }
            other => panic!("expected OverCap, got {other:?}"),
        }
        assert!(
            started.elapsed() < Duration::from_secs(5),
            "the byte cap, not the child's lifetime, bounds the wait"
        );
    }

    #[test]
    fn run_bounded_returns_output_under_the_byte_cap() {
        // rm-052: the cap must never disturb the happy path — a probe-
        // sized output drains whole and reaches the caller untouched.
        let result = run_bounded(
            "sh",
            &["-c", "printf probe-ok"],
            UPSTREAM_GIT_TIMEOUT,
            UPSTREAM_GIT_MAX_BYTES,
        );
        let (_, stdout, _) = result.expect("probe under the cap must succeed");
        assert_eq!(stdout, "probe-ok");
    }

    #[test]
    fn run_bounded_surfaces_a_stderr_overrun_as_over_cap() {
        // rm-052: the sideband channel is the real vector (git relays
        // "remote:" text on stderr) — an over-cap stderr alone is
        // terminal even when the child later exits successfully.
        let result = run_bounded(
            "sh",
            &["-c", "head -c 1048576 /dev/zero >&2; printf ok"],
            Duration::from_secs(30),
            4 * 1024,
        );
        match result {
            Err(GitRunError::OverCap { op, cap_bytes }) => {
                assert!(op.contains("head"), "over-cap names the operation: {op}");
                assert_eq!(cap_bytes, 4 * 1024);
            }
            other => panic!("expected OverCap, got {other:?}"),
        }
    }

    #[test]
    fn run_bounded_kills_a_slow_child_at_the_deadline() {
        // A child that ignores the deadline is killed and surfaced as a
        // named Timeout — never a hang. `sleep 5` under a 300ms bound
        // must return well before the child's natural lifetime.
        let started = Instant::now();
        let result = run_bounded(
            "sleep",
            &["5"],
            Duration::from_millis(300),
            UPSTREAM_GIT_MAX_BYTES,
        );
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
            UPSTREAM_GIT_MAX_BYTES,
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
            UPSTREAM_GIT_MAX_BYTES,
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
            UPSTREAM_GIT_MAX_BYTES,
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
            UPSTREAM_GIT_MAX_BYTES,
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
        let (_, stdout, _) = run_bounded(
            "git",
            &["--version"],
            UPSTREAM_GIT_TIMEOUT,
            UPSTREAM_GIT_MAX_BYTES,
        )
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
            UPSTREAM_GIT_MAX_BYTES,
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
            UPSTREAM_GIT_MAX_BYTES,
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
            opencode_state:
                "v1.18.35 (npm channel retired; authority: anomalyco/opencode releases)".to_string(),
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
    fn text_render_sanitizes_remote_control_bytes_json_stays_raw() {
        // rm-979, unit replay of the assess PoC (59bc617f): a repo the
        // user tracks as upstream can carry attacker-chosen bytes in
        // its commit subjects and its remote URL. The PoC's two live
        // payloads — a BEL-terminated OSC-52 clipboard write and a CSI
        // SGR hijack — plus an OSC-8 hyperlink variant across subject
        // and remote_url must reach the text view with every control
        // byte neutralized (U+FFFD), while the JSON arm keeps them
        // verbatim behind serde's lossless escaping.
        let mut status = sample_status();
        let osc52 = "pwn\u{1b}]52;c;aGVsbG8=\u{7}OSC52clip";
        let csi = "\u{1b}[31mREDTEXT\u{1b}[0m CSI-sgr hijack";
        let osc8 = "\u{1b}]8;;https://evil.example\u{1b}\\OSC8 link\u{1b}]8;;\u{7}";
        status.remote_url = format!("https://github.com/{osc8}/agenttrace.git");
        status.base_subject = format!("Merge pull request {osc52}");
        status.unported[0].subject = csi.to_string();
        status.unported.push(UpstreamCommit {
            sha: "36cded1a".to_string(),
            subject: osc8.to_string(),
            files: Vec::new(),
        });

        let text = status.to_text();
        assert!(
            text.chars().all(|c| !c.is_control() || c == '\n'),
            "text view must carry no control bytes besides newlines"
        );
        assert!(!text.contains(osc52) && !text.contains(csi) && !text.contains(osc8));
        assert!(text.contains('\u{FFFD}'), "control bytes become U+FFFD");
        // Printable context survives verbatim around the neutralized
        // bytes — the sanitizer is control-byte-scoped, not textual.
        assert!(text.contains("upstream:         upstream/master"));
        assert!(text.contains("Merge pull request"));
        assert!(text.contains("REDTEXT"));
        assert!(text.contains("OSC8 link"));

        // JSON arm unchanged: raw payload bytes round-trip verbatim.
        let raw = status.to_json().expect("json");
        let value: serde_json::Value = serde_json::from_str(&raw).expect("parse json");
        assert_eq!(value["remote_url"], status.remote_url.as_str());
        assert_eq!(value["last_sync"]["subject"], status.base_subject.as_str());
        assert_eq!(value["unported_commits"][0]["subject"], csi);
        assert_eq!(value["unported_commits"][1]["subject"], osc8);
    }

    #[test]
    fn benign_text_rendering_is_byte_stable() {
        // rm-979 byte-stability contract: sanitize_line_segment is the
        // identity on control-free strings, so a benign repo renders
        // exactly the bytes it rendered before the boundary existed —
        // the docs/guides/upstream-status.md sample output stays true.
        let status = sample_status();
        let text = status.to_text();
        assert!(!text.contains('\u{FFFD}'));
        assert!(text.contains(&format!("({})", status.remote_url)));
        assert!(text.contains(&status.base_subject));
        assert!(text.contains(&status.unported[0].subject));
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
        let privacy = std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../docs/PRIVACY.md"
        ))
        .expect("PRIVACY.md readable");

        // The npm registry probe URL, constructor-derived from the const.
        assert!(
            privacy.contains(NPM_REGISTRY_URL),
            "PRIVACY.md must name the npm registry probe URL {NPM_REGISTRY_URL}"
        );

        // rm-950: the opencode vendor-watch probes are network touches of
        // the same flag — PRIVACY.md must name both of their URLs too.
        assert!(
            privacy.contains(OPENCODE_NPM_URL),
            "PRIVACY.md must name the opencode registry probe URL {OPENCODE_NPM_URL}"
        );
        assert!(
            privacy.contains(OPENCODE_RELEASES_URL),
            "PRIVACY.md must name the opencode releases authority URL {OPENCODE_RELEASES_URL}"
        );
        assert!(
            !privacy.contains("one HTTPS request"),
            "PRIVACY.md must not under-count the --fetch upstream HTTPS touches"
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
        // same constants the fetch path uses — and renders a legitimate
        // (control-free) URL verbatim.
        let line = fetch_disclosure_line("https://github.com/example/upstream.git");
        assert!(line.contains("git fetch"), "disclosure must name git fetch");
        assert!(
            line.contains("git fetch https://github.com/example/upstream.git"),
            "benign URL renders verbatim in the disclosure"
        );
        assert!(
            line.contains(NPM_REGISTRY_URL),
            "disclosure must name the npm probe URL"
        );
        // rm-950: the vendor-watch probes ride the same opt-in flag, so
        // the disclosure names them too — every network touch, always.
        assert!(
            line.contains(OPENCODE_NPM_URL) && line.contains(OPENCODE_RELEASES_URL),
            "disclosure must name both opencode vendor-watch probe URLs"
        );
        assert!(
            !line.contains("one HTTPS request"),
            "disclosure must not under-count the --fetch HTTPS requests"
        );
        assert!(
            line.contains("PRIVACY.md"),
            "disclosure must point at PRIVACY.md"
        );

        // rm-979 review rider: the disclosure goes to stderr, which bypasses
        // every stdout choke point, so hostile URL bytes must die at this
        // boundary too — same class as the report body's render sites.
        let hostile = "\u{1b}]8;;https://evil.example\u{1b}\\remote\u{1b}]8;;\u{7}";
        let hostile_line = fetch_disclosure_line(hostile);
        assert!(
            hostile_line.chars().all(|c| !c.is_control()),
            "disclosure line must carry no control bytes: {hostile_line:?}"
        );
        assert!(
            hostile_line.contains("https://evil.example"),
            "printable URL text survives the sanitizer"
        );
    }
}
