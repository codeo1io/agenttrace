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

#[test]
fn org_identity_spoofing_characters_are_sanitized() {
    // rm-1571 review fix 9ce27e5c (F7): the acceptance listed
    // spoofing-char safety among the contract proofs but no arm
    // pinned it. The cap sanitizes BEFORE bounding (the rm-594
    // discipline), so a journal smuggling ANSI escapes, bidi
    // overrides, BOMs or joiners into an orgIdentifier cannot reach
    // the report lanes: every such char is replaced by U+FFFD,
    // whether or not the value is long enough to hit the digest
    // form. The ESC sequences ride the journal as JSON `\u001b`
    // escapes (raw control bytes are not legal inside a JSON string —
    // the parser unescapes them into the value, which is exactly the
    // smuggling path); the non-ASCII format chars are legal raw.
    // (U+2028 line-separator and ZWSP deliberately survive the
    // sanitizer — the rm-594 lane rules treat them as safe-in-JSON /
    // visible-in-text — so they are NOT asserted here.)
    let spoofed = "ev\\u001b[31mall-good\\u001b[0m\u{202e}nig\u{200e}ma\u{feff}\u{200d}corp";
    let journal = format!(
        "{{\"sessionId\":\"spoof\",\"type\":\"user\",\"cwd\":\"/w\",\"session\":{{\"identity\":{{\"orgIdentifier\":\"{org}\",\"loginEmail\":\"ok@example.com\"}}}},\"message\":{{\"role\":\"user\",\"content\":\"hi\"}}}}\n",
        org = spoofed,
    );
    let parsed = parse_raw_session("spoof", "/work/projects/x/spoof.jsonl", &journal)
        .expect("spoofing identity journal parses");
    for control in [
        '\u{1b}', '\u{202e}', '\u{200e}', '\u{feff}', '\u{200d}', '\n', '\r',
    ] {
        assert!(
            !parsed.org.contains(control),
            "spoofing control {control:?} must not survive into the org identity"
        );
    }
    assert!(
        parsed.org.contains('\u{FFFD}'),
        "controls are replaced with a visible placeholder, not silently dropped"
    );
    assert!(
        parsed.org.contains("all-good") && parsed.org.contains("corp"),
        "the legitimate payload text survives the sanitize"
    );
}

#[test]
fn hostile_org_identifier_is_capped_and_disclosed() {
    // rm-1571 (run fd0e91a2 cycle 3, hostile-input bound): the
    // identity fields are journal-controlled strings, so a hostile
    // 5 MiB `orgIdentifier` (assess 2bdc99b3 PoC) used to flow raw
    // into `by_org` names, the JSON overview (a 5,250,754-byte report
    // off one session) and the session-cache write. The capture site
    // now applies the repo's disclosure-cap discipline (sanitize,
    // then prefix+digest past the cap — the rm-594/rm-880
    // wire-metadata treatment): the org stays bounded and
    // distinguishable, the email rides the same cap, and the
    // truncation discloses via `org_identity_truncated` instead of
    // silently rewriting the record. Sane orgs round-trip
    // byte-identically (the zero-noise contract above).
    let hostile_org = "A".repeat(5 * 1024 * 1024);
    let hostile_email = "b".repeat(4096);
    let journal = format!(
        "{{\"sessionId\":\"hostile\",\"type\":\"user\",\"cwd\":\"/w\",\"session\":{{\"identity\":{{\"orgIdentifier\":\"{org}\",\"loginEmail\":\"{email}\"}}}},\"message\":{{\"role\":\"user\",\"content\":\"hi\"}}}}\n",
        org = hostile_org,
        email = hostile_email,
    );
    let hostile = parse_raw_session("hostile", "/work/projects/x/hostile.jsonl", &journal)
        .expect("hostile identity journal parses");
    assert!(hostile.org.chars().count() <= 96, "org is bounded");
    assert!(hostile.org_email.chars().count() <= 96, "email is bounded");
    assert!(
        hostile.org.contains('\u{2026}') && hostile.org.contains('#'),
        "digest form: bounded but still distinguishable from other hostile orgs"
    );
    assert_eq!(
        hostile
            .metrics
            .disclosure_counters
            .get("org_identity_truncated"),
        Some(&1),
        "truncation discloses instead of silently rewriting the record"
    );
    let overview = compute_overview(std::slice::from_ref(&hostile));
    let json = report_overview_json(&overview, std::slice::from_ref(&hostile));
    assert!(
        json.len() < 100_000,
        "the overview lane stays bounded (PoC measured 5,250,754 bytes)"
    );
    assert_eq!(overview.by_org.len(), 1);

    // Sane identity stays byte-identical and counter-free.
    let alpha = session("org-9f2a-example", ORG_ALPHA);
    assert_eq!(alpha.org, "org-9f2a-example");
    assert_eq!(alpha.org_email, "alpha@example.com");
    assert!(!alpha
        .metrics
        .disclosure_counters
        .contains_key("org_identity_truncated"));
}
