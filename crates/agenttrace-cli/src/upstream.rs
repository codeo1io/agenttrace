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
//!
//! Formats: `text` and `json` only (rm-507) — every other `-f` value
//! bails loudly instead of silently rendering the text report.

use anyhow::{anyhow, bail};
use serde_json::json;
use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::time::SystemTime;

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
        "text" => status.to_text(),
        // rm-507: no silent text fallback. The CLI dispatch names
        // text|json as the supported set for `agenttrace upstream`; this
        // arm keeps library callers to the same contract instead of
        // quietly rendering text for `-f csv|markdown|html upstream`.
        other => bail!(
            "-f {other} is not supported by `agenttrace upstream`; supported formats: text, json"
        ),
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

    // Fail fast with the same hints scripts/upstream-delta gives.
    if git(&["rev-parse", "--show-toplevel"]).is_err() {
        bail!("not inside a git repository");
    }
    let remote_url = match git(&["remote", "get-url", &remote]) {
        Ok(url) => url,
        Err(_) => bail!("no '{remote}' remote configured (git remote add {remote} <url>)"),
    };
    if fetch {
        // rm-404: PRIVACY.md promises fully-offline-by-default, so every opt-in
        // network touch announces itself on stderr before any request goes out.
        eprintln!("{}", fetch_disclosure_line(&remote_url));
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
