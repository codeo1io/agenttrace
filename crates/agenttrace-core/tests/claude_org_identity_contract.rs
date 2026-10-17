//! rm-1405 reap (run 4f2af62e5b7e, repository-maintenance 4f9d9268
//! cycle 1, "Claude org-dimension truth"): contract tests for the
//! Claude org-identity dimension.
//!
//! The claude-code 2.1.29x+ journals carry a session identity block
//! (`$.session.identity` with `orgIdentifier` / `loginEmail`). The
//! batch captures it on the session's meta event, surfaces it as
//! `Session::org` / `Session::org_email`, rolls it up as a ZERO-NOISE
//! `by_org` facet (only org-carrying sessions bucket; NO "unknown"
//! bucket; no facet key in report JSON and no section on text
//! surfaces when no session carries an org), discloses the capture
//! basis via a `claude_org_identity:*` counter, and preserves it
//! through the session cache (schema rung bump).
//!
//! The multi-org pair shares ONE cwd / project path on purpose: today
//! such sessions merge into indistinguishable attribution; the facet
//! separates them by the only dimension that distinguishes them.

use agenttrace_core::{
    cached_session, compute_overview, parse_raw_session, report_overview_json, store_session,
    SessionCache,
};

const ORG_ALPHA: &str = include_str!("fixtures/claude-org/org-9f2a-example.jsonl");
const ORG_BETA: &str = include_str!("fixtures/claude-org/org-6d1c-example.jsonl");
const NO_ORG: &str = include_str!("fixtures/claude-org/no-org.jsonl");

fn session(name: &str, raw: &str) -> agenttrace_core::Session {
    // parse_raw_session is the real dispatch surface (parse_file →
    // parse_session_bytes → parse_raw_session): the envelope-lane
    // claude_code probe lives there. parse_jsonl_session is the
    // generic lenient fallback, not the claude dispatcher.
    parse_raw_session(name, &format!("/work/projects/shared/{name}.jsonl"), raw)
        .expect("claude_code fixture parses")
}

#[test]
fn multi_org_pair_separates_in_by_org() {
    let alpha = session("org-9f2a-example", ORG_ALPHA);
    let beta = session("org-6d1c-example", ORG_BETA);
    assert_eq!(alpha.org, "org-9f2a-example");
    assert_eq!(alpha.org_email, "alpha@example.com");
    assert_eq!(beta.org, "org-6d1c-example");
    assert_eq!(beta.org_email, "beta@example.com");

    let overview = compute_overview(&[alpha, beta]);
    assert_eq!(
        overview.by_org.len(),
        2,
        "same cwd, different orgs: two buckets, not one merged attribution"
    );
    assert!(overview.by_org.contains_key("org-9f2a-example"));
    assert!(overview.by_org.contains_key("org-6d1c-example"));
    assert!(
        !overview.by_org.contains_key("unknown"),
        "no invented bucket: sessions without org stay out of the facet entirely"
    );
    for group in overview.by_org.values() {
        assert_eq!(group.sessions, 1);
    }
}

#[test]
fn org_identity_disclosed_via_counter() {
    let alpha = session("org-9f2a-example", ORG_ALPHA);
    assert_eq!(
        alpha
            .metrics
            .disclosure_counters
            .get("claude_org_identity:transcript"),
        Some(&1),
        "exactly one transcript-basis counter regardless of how many lines repeat the identity block"
    );
}

#[test]
fn no_org_corpus_is_zero_delta() {
    let plain = session("no-org", NO_ORG);
    assert_eq!(plain.org, "");
    assert!(!plain
        .metrics
        .disclosure_counters
        .contains_key("claude_org_identity:transcript"));

    let overview = compute_overview(std::slice::from_ref(&plain));
    assert!(overview.by_org.is_empty(), "no org data: empty facet");

    let json = report_overview_json(&overview, std::slice::from_ref(&plain));
    assert!(
        !json.contains("by_org"),
        "facet key absent from report JSON without org data"
    );
    assert!(
        !json.contains("claude_org_identity"),
        "no disclosure rows without org data"
    );

    // Mixed corpus: the org-less session stays out of the facet; the
    // org-carrying one does not inherit a merged neighbor.
    let alpha = session("org-9f2a-example", ORG_ALPHA);
    let overview = compute_overview(&[plain, alpha]);
    assert_eq!(overview.by_org.len(), 1);
    assert_eq!(
        overview.by_org.get("org-9f2a-example").map(|g| g.sessions),
        Some(1)
    );
}

/// rm-1553: a hostile journal must not pin unbounded org identity
/// strings onto `Session` — the capture caps at capture
/// (SESSION_IDENTITY_BYTE_CAP) and discloses per field via
/// org_truncated:* counters on the same carrier. Zero-noise: sane
/// identities cap nothing and mint no counters (asserted on the
/// healthy fixture below).
#[test]
fn hostile_org_identity_is_capped_and_disclosed() {
    // Same envelope shape as the fixtures, with a multi-megabyte
    // orgIdentifier and loginEmail — built inline, never on disk.
    let hostile_org = "x".repeat(5 * 1024 * 1024);
    let hostile_email = "y".repeat(5 * 1024 * 1024);
    let raw = format!(
        "{{\"session\":{{\"identity\":{{\"orgIdentifier\":\"{hostile_org}\",\"loginEmail\":\"{hostile_email}\"}},\"cwd\":\"/work/projects/shared\"}},\"type\":\"user\",\"timestamp\":\"2026-10-17T10:00:00.000Z\",\"message\":{{\"role\":\"user\",\"content\":\"hi\"}}}}\n"
    );
    let hostile_session = session("hostile-org.jsonl", &raw);
    assert!(
        !hostile_session.org.is_empty() && hostile_session.org.len() <= 4096,
        "org identity must be captured but bounded, got {} bytes",
        hostile_session.org.len()
    );
    assert!(
        !hostile_session.org_email.is_empty() && hostile_session.org_email.len() <= 4096,
        "loginEmail must be captured but bounded, got {} bytes",
        hostile_session.org_email.len()
    );
    let counters = &hostile_session.metrics.disclosure_counters;
    assert_eq!(
        counters.get("org_truncated:org:claude"),
        Some(&1),
        "org truncation must be disclosed on the carrier"
    );
    assert_eq!(
        counters.get("org_truncated:email:claude"),
        Some(&1),
        "email truncation must be disclosed on the carrier"
    );

    // Zero-noise control: the healthy fixture caps nothing.
    let healthy = session("org-9f2a-example.jsonl", ORG_ALPHA);
    let healthy_counters = &healthy.metrics.disclosure_counters;
    assert_eq!(
        healthy_counters.get("org_truncated:org:claude"),
        None,
        "sane identity must mint no truncation counter"
    );
    assert_eq!(healthy_counters.get("org_truncated:email:claude"), None);
    assert_eq!(healthy.org, "org-9f2a-example");
}

#[test]
fn report_renders_by_org_when_present() {
    let alpha = session("org-9f2a-example", ORG_ALPHA);
    let beta = session("org-6d1c-example", ORG_BETA);
    let sessions = vec![alpha, beta];
    let overview = compute_overview(&sessions);
    let json = report_overview_json(&overview, &sessions);
    assert!(json.contains("by_org"), "facet key present with org data");
    assert!(json.contains("org-9f2a-example"));
    assert!(json.contains("org-6d1c-example"));
}

#[test]
fn cache_round_trip_preserves_org_dimension() {
    let dir = std::env::temp_dir().join(format!("at-org-rt-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let file = dir.join("org-9f2a-example.jsonl");
    std::fs::write(&file, ORG_ALPHA).expect("write fixture");
    let raw = std::fs::read_to_string(&file).expect("read fixture");
    let session =
        parse_raw_session("org-9f2a-example", &file.to_string_lossy(), &raw).expect("parses");
    assert_eq!(session.org, "org-9f2a-example");

    let mut cache = SessionCache::default();
    let _ = store_session(&file, &session, &mut cache);
    let back = cached_session(&file, &mut cache).expect("warm cache hit");
    assert_eq!(back.org, "org-9f2a-example");
    assert_eq!(back.org_email, "alpha@example.com");
    let _ = std::fs::remove_file(&file);
    let _ = std::fs::remove_dir(&dir);
}
