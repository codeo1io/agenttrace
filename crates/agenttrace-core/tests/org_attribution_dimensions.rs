//! rm-1397 (Claude org/account-aware attribution, first cut): the
//! account dimension. Claude Code 2.1.299+ exposes the running
//! account's identity — an organization slug and an email address —
//! on journal lines (`$.session.identity`, gateway `attribution`
//! blocks, inline `identity` objects), and keeps per-account session
//! homes under `<config>/profiles/<name>/{projects,transcripts}`.
//! These tests pin the parse lane (identity folded onto Session beside
//! the rm-585 branch), the `by_org` overview dimension (org slug key,
//! email fallback — ZERO-NOISE: zero rows and byte-identical output
//! when the corpus carries no identity) on every output surface, the
//! structural profile-root discovery, and the doctor's informational
//! multi-account census.

use agenttrace_core::{
    build_doctor_report, compute_overview, find_session_files, known_session_dirs, parse_file,
    report_overview_html, report_overview_json, report_overview_markdown, report_overview_text,
    Session,
};
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

const TURNS: &str = concat!(
    r#"{"type":"user","timestamp":"2026-10-14T10:00:00Z","sessionId":"s1","cwd":"/work/app","gitBranch":"main","message":{"role":"user","content":"hi"}}"#,
    "\n",
    r#"{"type":"assistant","timestamp":"2026-10-14T10:00:05Z","sessionId":"s1","message":{"id":"msg_01","model":"claude-sonnet-4-20250514","content":[{"type":"text","text":"hello"}],"usage":{"input_tokens":1000,"output_tokens":50,"cache_read_input_tokens":200}}}"#,
    "\n",
);

/// A claude-code journal whose SECOND line carries the identity (CC
/// 2.1.299 `$.session.identity` shape) — identity can ride any line,
/// not just the first.
const JOURNAL_SESSION_IDENTITY: &str = concat!(
    r#"{"type":"user","timestamp":"2026-10-14T10:00:00Z","sessionId":"s1","cwd":"/work/app","gitBranch":"main","message":{"role":"user","content":"hi"}}"#,
    "\n",
    r#"{"type":"assistant","timestamp":"2026-10-14T10:00:05Z","sessionId":"s1","session":{"identity":{"organizationSlug":"acme-corp","emailAddress":"dev@acme-corp.example"}},"message":{"id":"msg_01","model":"claude-sonnet-4-20250514","content":[{"type":"text","text":"hello"}],"usage":{"input_tokens":1000,"output_tokens":50,"cache_read_input_tokens":200}}}"#,
    "\n",
);

/// Gateway-managed account: the same fields under a top-level
/// `attribution` block.
const JOURNAL_ATTRIBUTION_BLOCK: &str = concat!(
    r#"{"type":"user","timestamp":"2026-10-14T11:00:00Z","sessionId":"s2","cwd":"/work/app","gitBranch":"feature/x","attribution":{"organization":"globex","email":"ops@globex.example"},"message":{"role":"user","content":"hi"}}"#,
    "\n",
    r#"{"type":"assistant","timestamp":"2026-10-14T11:00:05Z","sessionId":"s2","message":{"id":"msg_02","model":"claude-sonnet-4-20250514","content":[{"type":"text","text":"hello"}],"usage":{"input_tokens":500,"output_tokens":25}}}"#,
    "\n",
);

/// Inline top-level `identity` object, email only (no org slug): the
/// email becomes the bucket key.
const JOURNAL_INLINE_IDENTITY_EMAIL_ONLY: &str = concat!(
    r#"{"type":"user","timestamp":"2026-10-14T12:00:00Z","sessionId":"s3","cwd":"/work/app","gitBranch":"main","identity":{"email":"contractor@inde.example"},"message":{"role":"user","content":"hi"}}"#,
    "\n",
    r#"{"type":"assistant","timestamp":"2026-10-14T12:00:05Z","sessionId":"s3","message":{"id":"msg_03","model":"claude-sonnet-4-20250514","content":[{"type":"text","text":"hello"}],"usage":{"input_tokens":200,"output_tokens":10}}}"#,
    "\n",
);

fn scratch_dir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("agenttrace-org-{}-{}", tag, std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("scratch dir");
    dir
}

fn write_journal(dir: impl AsRef<Path>, file: &str, contents: &str) -> PathBuf {
    let dir = dir.as_ref();
    fs::create_dir_all(dir).expect("journal dir");
    let path = dir.join(file);
    fs::write(&path, contents).expect("journal fixture");
    path
}

fn parse_or_panic(path: &Path) -> Session {
    parse_file(path).unwrap_or_else(|error| panic!("parse {}: {error}", path.display()))
}

#[test]
fn session_identity_line_folds_onto_session_beside_branch() {
    let root = scratch_dir("session-identity");
    let path = write_journal(&root, "session.jsonl", JOURNAL_SESSION_IDENTITY);
    let session = parse_or_panic(&path);
    assert_eq!(
        session.org, "acme-corp",
        "$.session.identity.organizationSlug must fold onto Session.org"
    );
    assert_eq!(
        session.email, "dev@acme-corp.example",
        "$.session.identity.emailAddress must fold onto Session.email"
    );
    assert_eq!(
        session.branch, "main",
        "branch capture stays intact (rm-585)"
    );
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn gateway_attribution_block_and_inline_identity_shapes_parse() {
    let root = scratch_dir("identity-shapes");
    let attribution = parse_or_panic(&write_journal(
        &root,
        "attribution.jsonl",
        JOURNAL_ATTRIBUTION_BLOCK,
    ));
    assert_eq!(
        attribution.org, "globex",
        "attribution.organization is an accepted org spelling"
    );
    assert_eq!(
        attribution.email, "ops@globex.example",
        "attribution.email is an accepted email spelling"
    );
    let inline = parse_or_panic(&write_journal(
        &root,
        "inline.jsonl",
        JOURNAL_INLINE_IDENTITY_EMAIL_ONLY,
    ));
    assert!(
        inline.org.is_empty(),
        "email-only identity leaves org empty (email fallback buckets it)"
    );
    assert_eq!(
        inline.email, "contractor@inde.example",
        "inline identity.email parses"
    );
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn identity_free_journals_render_zero_org_rows_and_unchanged_output() {
    // rm-1397 acceptance clauses 2+4: the by-org dimension is optional
    // and default-OFF — an identity-free corpus renders ZERO org rows
    // on every surface and the output stays byte-identical to the
    // pre-rm-1397 shape: no "unknown" bucket, no By Org section on
    // text/markdown/html, and no by_org key in the JSON payload.
    let root = scratch_dir("identity-free");
    let session = parse_or_panic(&write_journal(&root, "plain.jsonl", TURNS));
    assert!(session.org.is_empty() && session.email.is_empty());
    let overview = compute_overview(std::slice::from_ref(&session));
    assert!(
        overview.by_org.is_empty(),
        "identity-less sessions must contribute no org row: {:?}",
        overview.by_org.keys().collect::<Vec<_>>()
    );
    let text = report_overview_text(&overview, std::slice::from_ref(&session));
    assert!(
        !text.contains("By Org"),
        "text surface must stay unchanged: {text}"
    );
    let markdown = report_overview_markdown(&overview, std::slice::from_ref(&session));
    assert!(!markdown.contains("## By org"));
    let html = report_overview_html(&overview, std::slice::from_ref(&session));
    assert!(!html.contains("<h2>By org</h2>"));
    let json_text = report_overview_json(&overview, std::slice::from_ref(&session));
    assert!(
        !json_text.contains("\"by_org\""),
        "the by_org key must be omitted when the dimension is empty"
    );
    let _ = fs::remove_dir_all(&root);
}

/// The three-session corpus: one org-slug account, one gateway org,
/// one email-only account, plus one identity-less session.
fn mixed_corpus(root: &Path) -> Vec<Session> {
    let mut sessions = vec![
        parse_or_panic(&write_journal(root, "acme.jsonl", JOURNAL_SESSION_IDENTITY)),
        parse_or_panic(&write_journal(
            root,
            "globex.jsonl",
            JOURNAL_ATTRIBUTION_BLOCK,
        )),
        parse_or_panic(&write_journal(
            root,
            "contractor.jsonl",
            JOURNAL_INLINE_IDENTITY_EMAIL_ONLY,
        )),
        parse_or_panic(&write_journal(root, "plain.jsonl", TURNS)),
    ];
    sessions.sort_by(|a, b| a.path.cmp(&b.path));
    sessions
}

#[test]
fn overview_by_org_buckets_org_slug_and_email_fallback_only() {
    let root = scratch_dir("by-org");
    let sessions = mixed_corpus(&root);
    let overview = compute_overview(&sessions);
    let mut keys: Vec<&str> = overview.by_org.keys().map(String::as_str).collect();
    keys.sort_unstable();
    assert_eq!(
        keys,
        vec!["acme-corp", "contractor@inde.example", "globex"],
        "org slug keys and email fallback for slug-less accounts; the identity-less session contributes no row"
    );
    let bucket_sessions: usize = overview.by_org.values().map(|g| g.sessions).sum();
    assert_eq!(
        bucket_sessions,
        sessions.len() - 1,
        "every identity-bearing session lands in exactly one org bucket"
    );
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn two_org_corpus_yields_exactly_two_buckets() {
    // rm-1397 acceptance clause 3 (red-first two-org fixture -> two
    // buckets): a corpus of exactly two distinct org accounts splits
    // into exactly two buckets — neither account collapses into the
    // other, which was the whole defect (one host, two corporate
    // accounts, one silently co-mingled spend bucket).
    let root = scratch_dir("two-org");
    let mut sessions = vec![
        parse_or_panic(&write_journal(
            &root,
            "acme.jsonl",
            JOURNAL_SESSION_IDENTITY,
        )),
        parse_or_panic(&write_journal(
            &root,
            "globex.jsonl",
            JOURNAL_ATTRIBUTION_BLOCK,
        )),
    ];
    sessions.sort_by(|a, b| a.path.cmp(&b.path));
    let overview = compute_overview(&sessions);
    let keys: Vec<&str> = overview.by_org.keys().map(String::as_str).collect();
    assert_eq!(
        keys,
        vec!["acme-corp", "globex"],
        "two distinct org accounts -> exactly two buckets"
    );
    assert!(overview.by_org.values().all(|group| group.sessions == 1));
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn by_org_renders_on_every_output_surface() {
    // rm-585's every-surface pin (by_branch_renders_on_every_surface)
    // applied to the account dimension: text, markdown, JSON and HTML
    // must all carry the org dimension with the full bucket labels.
    let root = scratch_dir("every-surface");
    let sessions = mixed_corpus(&root);
    let overview = compute_overview(&sessions);

    let json =
        serde_json::from_str::<serde_json::Value>(&report_overview_json(&overview, &sessions))
            .expect("valid json");
    let orgs = json["by_org"].as_array().expect("by_org array");
    let names: Vec<&str> = orgs.iter().filter_map(|o| o["name"].as_str()).collect();
    for expected in ["acme-corp", "globex", "contractor@inde.example"] {
        assert!(
            names.contains(&expected),
            "JSON by_org must carry {expected}: {names:?}"
        );
    }

    let text = report_overview_text(&overview, &sessions);
    assert!(text.contains("By Org"), "text surface must render By Org");
    assert!(
        text.contains("acme-corp"),
        "text surface carries the org label"
    );

    let markdown = report_overview_markdown(&overview, &sessions);
    assert!(
        markdown.contains("## By org"),
        "markdown surface must render the By org section"
    );
    assert!(markdown.contains("| acme-corp |"));

    let html = report_overview_html(&overview, &sessions);
    assert!(
        html.contains("<h2>By org</h2>"),
        "html surface must render By org"
    );
    assert!(html.contains("acme-corp"));
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn profile_homes_register_structurally_and_discover_their_journals() {
    // The CLAUDE_CODE_PROFILE layout: every child of
    // <config>/profiles registers its projects/ and transcripts/
    // subdirectories; a second corporate account is scanned without an
    // allowlist; a profiles/ root that does not exist registers
    // nothing.
    let root = scratch_dir("profiles");
    let home = root.join("home");
    let profile_projects = home.join(".claude").join("profiles").join("acme-work");
    write_journal(
        profile_projects.join("projects"),
        "work.jsonl",
        JOURNAL_SESSION_IDENTITY,
    );
    write_journal(
        profile_projects.join("transcripts"),
        "work-transcript.jsonl",
        TURNS,
    );
    // Non-session profile child: must not register anything.
    fs::create_dir_all(
        home.join(".claude")
            .join("profiles")
            .join("not-sessions")
            .join("notes"),
    )
    .expect("non-session profile child");

    with_home(&home, || {
        let registry: Vec<(String, PathBuf)> = known_session_dirs()
            .into_iter()
            .map(|dir| (dir.name, dir.path))
            .collect();
        let paths: HashSet<String> = registry
            .iter()
            .map(|(_, path)| path.to_string_lossy().to_string())
            .collect();
        assert!(
            paths.contains(
                &home
                    .join(".claude")
                    .join("profiles")
                    .join("acme-work")
                    .join("projects")
                    .to_string_lossy()
                    .to_string()
            ),
            "profile projects root must register: {paths:?}"
        );
        assert!(
            paths.contains(
                &home
                    .join(".claude")
                    .join("profiles")
                    .join("acme-work")
                    .join("transcripts")
                    .to_string_lossy()
                    .to_string()
            ),
            "profile transcripts root must register: {paths:?}"
        );
        assert!(
            !paths.iter().any(|p| p.contains("not-sessions")),
            "a profile child without projects/transcripts registers nothing: {paths:?}"
        );
        let names: Vec<&str> = registry.iter().map(|(name, _)| name.as_str()).collect();
        assert!(
            names.contains(&"Claude Code profile acme-work (projects)"),
            "registry names the profile: {names:?}"
        );

        let discovered = find_session_files(None);
        let discovered: HashSet<String> = discovered
            .iter()
            .map(|p| p.to_string_lossy().to_string())
            .collect();
        assert!(
            discovered.contains(
                &profile_projects
                    .join("projects")
                    .join("work.jsonl")
                    .to_string_lossy()
                    .to_string()
            ),
            "default discovery must find the profile journal: {discovered:?}"
        );
    });
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn doctor_discloses_a_multi_account_census_but_stays_clean_on_one_account() {
    // The informational-tier census: TWO OR MORE distinct accounts in
    // one corpus disclose `claude_identity_accounts`; a single-account
    // or identity-less corpus renders identically to before
    // (0/1-count-clean).
    let root = scratch_dir("doctor-census");
    let acme = write_journal(root.join("acme"), "a.jsonl", JOURNAL_SESSION_IDENTITY);
    let globex = write_journal(root.join("globex"), "g.jsonl", JOURNAL_ATTRIBUTION_BLOCK);

    let mixed = build_doctor_report(Some(&root), false);
    assert_eq!(
        mixed.disclosures.get("claude_identity_accounts"),
        Some(&2),
        "a corpus mixing two accounts must disclose the census: {:?}",
        mixed.disclosures
    );

    // Single-account corpus: no key.
    let single_root = scratch_dir("doctor-census-single");
    let _ = write_journal(&single_root, "a.jsonl", JOURNAL_SESSION_IDENTITY);
    let single = build_doctor_report(Some(&single_root), false);
    assert!(
        !single.disclosures.contains_key("claude_identity_accounts"),
        "a single-account corpus must stay 0/1-count-clean: {:?}",
        single.disclosures
    );

    // The parsed sessions themselves must carry identity (census is
    // derived from the same fold).
    let acme_session = parse_or_panic(&acme);
    assert_eq!(acme_session.org, "acme-corp");
    let globex_session = parse_or_panic(&globex);
    assert_eq!(globex_session.org, "globex");
    let _ = fs::remove_dir_all(&root);
    let _ = fs::remove_dir_all(&single_root);
}

// ---- env-surgery helpers (pi_family_discovery.rs pattern) ----

fn with_home(home: &Path, f: impl FnOnce()) {
    let result = {
        let _guard = lock_env();
        let previous_home = std::env::var_os("HOME");
        let previous_config = std::env::var_os("CLAUDE_CONFIG_DIR");
        let previous_xdg_config = std::env::var_os("XDG_CONFIG_HOME");
        let previous_xdg_cache = std::env::var_os("XDG_CACHE_HOME");
        let previous_xdg_data = std::env::var_os("XDG_DATA_HOME");
        let previous_cache = std::env::var_os("AGENTTRACE_SESSION_CACHE_DIR");
        std::env::set_var("HOME", home);
        std::env::remove_var("CLAUDE_CONFIG_DIR");
        std::env::set_var("XDG_CONFIG_HOME", home.join(".config"));
        std::env::set_var("XDG_CACHE_HOME", home.join(".cache"));
        std::env::set_var("XDG_DATA_HOME", home.join(".local/share"));
        std::env::set_var("AGENTTRACE_SESSION_CACHE_DIR", home.join("cache"));
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f));
        restore_env("HOME", previous_home);
        restore_env("CLAUDE_CONFIG_DIR", previous_config);
        restore_env("XDG_CONFIG_HOME", previous_xdg_config);
        restore_env("XDG_CACHE_HOME", previous_xdg_cache);
        restore_env("XDG_DATA_HOME", previous_xdg_data);
        restore_env("AGENTTRACE_SESSION_CACHE_DIR", previous_cache);
        result
    };
    if let Err(payload) = result {
        std::panic::resume_unwind(payload);
    }
}

fn restore_env(key: &str, previous: Option<std::ffi::OsString>) {
    match previous {
        Some(value) => std::env::set_var(key, value),
        None => std::env::remove_var(key),
    }
}

fn lock_env() -> std::sync::MutexGuard<'static, ()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    match LOCK.get_or_init(|| Mutex::new(())).lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    }
}
