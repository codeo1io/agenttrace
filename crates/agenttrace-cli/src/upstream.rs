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
//! explicit flag. No new crate dependencies: `git` runs as a subprocess,
//! and the registry probe goes through agenttrace-core's in-process
//! HTTP client (rm-081) — no external `curl` — so the CLI crate adds no
//! network machinery of its own.

use anyhow::{anyhow, bail};
use serde_json::json;
use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::time::{Duration, SystemTime};

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

    // Fail fast with the same hints scripts/upstream-delta gives. The
    // preflight separates "git could not even be spawned" from
    // "spawned fine, cwd is not a work tree" (rm-081): a valid
    // checkout with git absent from PATH used to be misreported as
    // "not inside a git repository".
    repository_preflight()?;
    let remote_url = match git(&["remote", "get-url", &remote]) {
        Ok(url) => url,
        Err(_) => bail!("no '{remote}' remote configured (git remote add {remote} <url>)"),
    };
    if fetch {
        fetch_remote(&remote)?;
    }
    let tracking = format!("refs/remotes/{remote}/{ref_name}");
    if git(&["rev-parse", "--verify", &tracking]).is_err() {
        bail!("no remote-tracking ref {tracking}; run agenttrace --fetch upstream");
    }

    let local_head = git(&["rev-parse", "HEAD"])?;
    let local_branch = match git(&["rev-parse", "--abbrev-ref", "HEAD"]) {
        Ok(branch) if branch != "HEAD" => Some(branch),
        _ => None,
    };
    let merge_base = match git(&["merge-base", "HEAD", &tracking]) {
        Ok(base) => base,
        Err(_) => bail!("no common history with {remote}/{ref_name}"),
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
            // rm-081: the probe is in-process now, so "curl absent" is
            // no longer a failure mode. Distinguishing WHY an in-process
            // probe failed stays rm-052's lane (npm state split).
            None => "unavailable (registry probe failed)".to_string(),
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
    let output = Command::new("git")
        .args(["fetch", remote, "--quiet"])
        .output()
        .map_err(|err| anyhow!("failed to spawn git fetch: {err}"))?;
    if !output.status.success() {
        bail!(
            "git fetch {remote} failed: {}",
            snippet(&String::from_utf8_lossy(&output.stderr))
        );
    }
    Ok(())
}

/// Outcome of the repository preflight probe.
#[derive(Debug, PartialEq)]
enum RepoProbe {
    /// The git binary could not be spawned at all (absent from PATH,
    /// not executable). Distinct from a genuine non-repository cwd —
    /// conflating the two misdiagnosed valid checkouts (rm-081).
    GitUnavailable(String),
    /// git ran and reported this cwd is not inside a work tree.
    NotARepository,
    /// Inside a work tree; the payload is `rev-parse --show-toplevel`.
    WorkTree(String),
}

fn repo_probe(spawn: Result<std::process::Output, std::io::Error>) -> RepoProbe {
    match spawn {
        Err(err) => RepoProbe::GitUnavailable(err.to_string()),
        Ok(output) if !output.status.success() => RepoProbe::NotARepository,
        Ok(output) => {
            RepoProbe::WorkTree(String::from_utf8_lossy(&output.stdout).trim().to_string())
        }
    }
}

/// rm-081: the repository preflight behind `agenttrace upstream`.
/// "git absent from PATH" gets its own actionable diagnosis instead of
/// the old blanket "not inside a git repository".
fn repository_preflight() -> anyhow::Result<()> {
    let probe = repo_probe(
        Command::new("git")
            .args(["rev-parse", "--show-toplevel"])
            .output(),
    );
    match probe {
        RepoProbe::WorkTree(_) => Ok(()),
        RepoProbe::GitUnavailable(err) => bail!(
            "git is not available ({err}); 'agenttrace upstream' drives the git CLI — install git or extend PATH"
        ),
        RepoProbe::NotARepository => bail!("not inside a git repository"),
    }
}

/// Runs git and returns trimmed stdout; errors carry the command and a
/// stderr snippet so a failed probe is diagnosable from the report.
fn git(args: &[&str]) -> anyhow::Result<String> {
    let output = Command::new("git")
        .args(args)
        .output()
        .map_err(|err| anyhow!("failed to spawn git: {err}"))?;
    if !output.status.success() {
        bail!(
            "git {} failed: {}",
            args.join(" "),
            snippet(&String::from_utf8_lossy(&output.stderr))
        );
    }
    Ok(String::from_utf8_lossy(&output.stdout)
        .trim_end()
        .to_string())
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

/// npm registry probe behind `--fetch` only. In-process through the
/// same ureq client the pricing fetch uses (rm-081) — no external curl
/// dependency, same bounded timeout — so a PATH without curl can no
/// longer report the registry as unavailable. Degrades to `None`
/// (never fails the report) when the registry answers badly.
fn npm_registry_version() -> Option<String> {
    let body = agenttrace_core::http_get_text(NPM_REGISTRY_URL, Duration::from_secs(15)).ok()?;
    npm_version_from_body(&body)
}

/// Pure parsing half of the registry probe, split out for hermetic
/// tests: pull the `version` field out of a registry document.
fn npm_version_from_body(body: &str) -> Option<String> {
    let value = serde_json::from_str::<serde_json::Value>(body).ok()?;
    value.get("version")?.as_str().map(str::to_string)
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

    #[test]
    fn npm_version_from_body_parses_registry_documents() {
        // rm-081: the parsing half of the in-process registry probe,
        // hermetic — no network, no curl.
        assert_eq!(
            npm_version_from_body(r#"{"name":"@zack78/agenttrace","version":"0.42.0"}"#),
            Some("0.42.0".to_string())
        );
        assert_eq!(npm_version_from_body("not json"), None);
        assert_eq!(
            npm_version_from_body(r#"{"name":"@zack78/agenttrace"}"#),
            None
        );
        assert_eq!(npm_version_from_body(r#"{"version":42}"#), None);
    }

    #[test]
    fn repo_probe_classifies_spawn_failure_as_git_unavailable() {
        // rm-081 arm 1, fully hermetic: a real spawn failure (the exact
        // io::Error a PATH without git produces) must classify as
        // GitUnavailable, never as NotARepository.
        let missing_binary = Command::new("agenttrace-git-probe-nonexistent")
            .args(["rev-parse", "--show-toplevel"])
            .output();
        match repo_probe(missing_binary) {
            RepoProbe::GitUnavailable(_) => {}
            other => panic!("spawn failure misclassified as {other:?}"),
        }
        // Same classification for a synthetic spawn error (portable).
        assert!(matches!(
            repo_probe(Err(std::io::Error::from_raw_os_error(2))),
            RepoProbe::GitUnavailable(_)
        ));
    }

    #[test]
    fn repo_probe_classifies_git_exit_failure_as_not_a_repository() {
        // rm-081 arm 2: git ran fine and said "not a work tree" — the
        // only outcome that may report "not inside a git repository".
        // Run real git from a pristine temp directory so the exit path
        // is exercised exactly as production sees it.
        let temp =
            std::env::temp_dir().join(format!("agenttrace-repo-probe-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&temp);
        std::fs::create_dir_all(&temp).expect("temp dir");
        let probe = repo_probe(
            Command::new("git")
                .args(["rev-parse", "--show-toplevel"])
                .current_dir(&temp)
                .output(),
        );
        let _ = std::fs::remove_dir_all(&temp);
        match probe {
            RepoProbe::NotARepository => {}
            // Hosts without a git binary skip the live arm — the spawn
            // arm above already covers that classification.
            RepoProbe::GitUnavailable(_) => {}
            RepoProbe::WorkTree(toplevel) => {
                panic!("pristine temp dir {temp:?} classified as work tree {toplevel}")
            }
        }
    }

    #[test]
    #[cfg(unix)]
    fn git_absent_from_path_is_not_reported_as_not_a_repository() {
        // Assess F2 live repro, hermetic: a VALID checkout (this test
        // runs inside one) with git scrubbed from the child's PATH must
        // diagnose "git unavailable" — the old code called this exact
        // state "not inside a git repository". The PATH override is
        // scoped to the spawned probe; this process's env is untouched.
        let probe = Command::new("git")
            .args(["rev-parse", "--show-toplevel"])
            .env("PATH", "/nonexistent-agenttrace-probe")
            .current_dir(std::env::current_dir().expect("cwd"))
            .output();
        match repo_probe(probe) {
            RepoProbe::GitUnavailable(_) => {}
            RepoProbe::WorkTree(toplevel) => panic!(
                "PATH scrub ignored by the platform — probe found git via {toplevel}; \
                 drop this test or use an absolute missing binary"
            ),
            RepoProbe::NotARepository => {
                panic!("valid checkout + absent git misreported as not a repository")
            }
        }
    }
}
