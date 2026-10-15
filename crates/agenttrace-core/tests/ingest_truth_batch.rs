//! "Ingest truth" batch contract (run 942ed30a cycle 3).
//!
//! rm-1417 — one conversation, one session: a gemini temp conversation is
//! mirrored across `~/.gemini/tmp/<root>/chats/` and
//! `~/.gemini/tmp/<root>/checkpoints/` (the live chat plus its checkpoint
//! snapshots). Both files carry CUMULATIVE usage snapshots of the SAME
//! conversation, so discovering both used to render two sessions and
//! double-count every cumulative field. The loader must merge a mirror
//! family into ONE session keyed by the tmp root:
//! - per-field MAX across members (never a sum — cumulative snapshots);
//! - the chats-lane member is the representative (live conversation);
//! - usage is REPRICED from the merged token totals;
//! - the merge is DISCLOSED (`cross_file_sessions_merged` counter), never
//!   a silent drop;
//! - the identity is scoped: two distinct chats under one root do NOT
//!   merge (ambiguous root → file-scoped keys), and distinct roots never
//!   merge.
//!
//! rm-1418 — bounded ingest reads: opencode storage record reads are
//! byte-capped (`opencode_record_capped` disclosure when a record exceeds
//! the cap and is skipped instead of read whole).
//!
//! independent_review 09b3e188 review-fix arms (attempt 97ef8844):
//! - F1 — a MULTI-MODEL mirror family (mid-conversation model switch)
//!   reprices per attribution entry, never at the builtin default the
//!   `"multiple"` lookup would bill;
//! - F4 — a session whose EVERY message record is over the cap still
//!   EXISTS: it materializes with the disclosure and zero messages,
//!   instead of vanishing behind a swallowed bail.

use agenttrace_core::{load_sessions_from_dir, lookup_price, parse_file, Session};
use std::fs;
use std::path::{Path, PathBuf};

fn repo_testdata() -> PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("repo root")
}

fn temp_corpus(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "agenttrace-ingest-truth-{}-{}-{tag}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis()
    ));
    fs::create_dir_all(&dir).expect("temp corpus");
    dir
}

fn copy_gemini_fixture(corpus: &Path, family: &str, fixture: &str) -> PathBuf {
    let dir = corpus
        .join(".gemini")
        .join("tmp")
        .join("conv-abc")
        .join(family);
    fs::create_dir_all(&dir).expect("family dir");
    let dst = dir.join(fixture);
    fs::copy(repo_testdata().join("testdata").join(fixture), &dst).expect("fixture copy");
    dst
}

fn gemini_sessions(corpus: &Path) -> Vec<Session> {
    load_sessions_from_dir(Some(corpus))
        .into_iter()
        .filter(|session| session.metrics.source_tool == "gemini_cli")
        .collect()
}

fn round4(value: f64) -> f64 {
    (value * 10_000.0).round() / 10_000.0
}

fn expected_price(model: &str, session: &Session) -> f64 {
    let price = lookup_price(model);
    round4(
        session.metrics.tokens_input as f64 / 1e6 * price.input
            + session.metrics.tokens_output as f64 / 1e6 * price.output
            + session.metrics.tokens_cache_w as f64 / 1e6 * price.cw
            + session.metrics.tokens_cache_r as f64 / 1e6 * price.cr,
    )
}

/// rm-1417 red-first arm: the assess PoC corpus (one conversation mirrored
/// under chats/ + checkpoints/) must render as ONE session with the
/// cumulative MAX of both members and a disclosed merge counter.
#[test]
fn gemini_mirror_conversation_merges_into_one_session() {
    let corpus = temp_corpus("mirror");
    copy_gemini_fixture(&corpus, "chats", "gemini-current-chat.json");
    copy_gemini_fixture(&corpus, "checkpoints", "gemini-checkpoint.json");

    let chat = parse_file(&repo_testdata().join("testdata/gemini-current-chat.json"))
        .expect("parse chat fixture");
    let checkpoint = parse_file(&repo_testdata().join("testdata/gemini-checkpoint.json"))
        .expect("parse checkpoint fixture");

    let sessions = gemini_sessions(&corpus);
    assert_eq!(
        sessions.len(),
        1,
        "one mirrored conversation must render as one session, got {}",
        sessions.len()
    );
    let merged = &sessions[0];
    // The live chat under chats/ is the representative.
    assert_eq!(
        merged.path,
        corpus
            .join(".gemini/tmp/conv-abc/chats/gemini-current-chat.json")
            .to_string_lossy()
            .to_string()
    );
    // MAX of cumulative fields, never a sum.
    assert_eq!(
        merged.metrics.tokens_input,
        chat.metrics
            .tokens_input
            .max(checkpoint.metrics.tokens_input)
    );
    assert_eq!(
        merged.metrics.tokens_output,
        chat.metrics
            .tokens_output
            .max(checkpoint.metrics.tokens_output)
    );
    // Disclosed, never silent.
    assert_eq!(
        merged
            .metrics
            .disclosure_counters
            .get("cross_file_sessions_merged"),
        Some(&1),
        "merge must be disclosed: {:?}",
        merged.metrics.disclosure_counters
    );
    // Repriced from the merged token totals at the representative's model.
    assert_eq!(
        merged.metrics.cost_estimated,
        expected_price("gemini-2.5-pro", merged)
    );

    let _ = fs::remove_dir_all(corpus);
}

/// rm-1417 scoped-fallback arm: a root whose chats/ holds TWO distinct
/// conversations is ambiguous — nothing merges (file-scoped keys).
#[test]
fn gemini_ambiguous_root_with_two_chats_does_not_merge() {
    let corpus = temp_corpus("ambiguous");
    copy_gemini_fixture(&corpus, "chats", "gemini-current-chat.json");
    let second = corpus.join(".gemini/tmp/conv-abc/chats/second-chat.json");
    fs::copy(
        repo_testdata().join("testdata/gemini-thoughts-checkpoint.json"),
        &second,
    )
    .expect("second chat");

    let sessions = gemini_sessions(&corpus);
    assert_eq!(sessions.len(), 2, "ambiguous root must stay two sessions");
    for session in &sessions {
        assert!(
            !session
                .metrics
                .disclosure_counters
                .contains_key("cross_file_sessions_merged"),
            "no merge may be claimed for an ambiguous root"
        );
    }

    let _ = fs::remove_dir_all(corpus);
}

/// rm-1417 control arm: distinct tmp roots never merge — two conversations
/// in two roots stay two sessions even when one carries a checkpoint twin.
#[test]
fn gemini_distinct_roots_stay_distinct_sessions() {
    let corpus = temp_corpus("distinct");
    copy_gemini_fixture(&corpus, "chats", "gemini-current-chat.json");
    copy_gemini_fixture(&corpus, "checkpoints", "gemini-checkpoint.json");
    let other_dir = corpus.join(".gemini/tmp/conv-xyz/chats");
    fs::create_dir_all(&other_dir).expect("other root");
    fs::copy(
        repo_testdata().join("testdata/gemini-thoughts-checkpoint.json"),
        other_dir.join("gemini-thoughts-checkpoint.json"),
    )
    .expect("other conversation");

    let sessions = gemini_sessions(&corpus);
    assert_eq!(
        sessions.len(),
        2,
        "mirror family (1) + distinct root (1) = 2 sessions, got {}",
        sessions.len()
    );

    let _ = fs::remove_dir_all(corpus);
}

/// rm-1417 checkpoints-only arm: a conversation that only left checkpoint
/// snapshots still double-counts — merge them too (representative =
/// discovery-order-first checkpoint).
#[test]
fn gemini_checkpoints_only_mirror_merges() {
    let corpus = temp_corpus("checkpointsonly");
    copy_gemini_fixture(&corpus, "checkpoints", "gemini-checkpoint.json");
    copy_gemini_fixture(&corpus, "checkpoints", "gemini-thoughts-checkpoint.json");

    let sessions = gemini_sessions(&corpus);
    assert_eq!(sessions.len(), 1, "checkpoints-only mirror is one session");
    assert_eq!(
        sessions[0]
            .metrics
            .disclosure_counters
            .get("cross_file_sessions_merged"),
        Some(&1)
    );

    let _ = fs::remove_dir_all(corpus);
}

/// rm-1418: an opencode storage record over the ingest byte cap is skipped
/// and DISCLOSED instead of read whole; the corpus still parses.
#[test]
fn opencode_record_over_cap_is_skipped_and_disclosed() {
    let corpus = temp_corpus("opencode-cap");
    // Admission requires the literal `/opencode/storage` path marker
    // (`open_code_storage_rel`), so the corpus must carry it.
    let storage = corpus.join("opencode").join("storage");
    let session_dir = storage.join("session").join("project_alpha");
    let message_dir = storage.join("message").join("ses_abc");
    let part_dir = storage.join("part").join("msg_user");
    fs::create_dir_all(&session_dir).expect("session dir");
    fs::create_dir_all(&message_dir).expect("message dir");
    fs::create_dir_all(&part_dir).expect("part dir");
    let session_path = session_dir.join("ses_abc.json");
    fs::copy(
        repo_testdata().join("testdata/opencode/storage/session/project_alpha/ses_abc.json"),
        &session_path,
    )
    .expect("session fixture");
    // The session's good user message (so the session parses even with the
    // fat record skipped).
    fs::copy(
        repo_testdata().join("testdata/opencode/storage/message/ses_abc/msg_user.json"),
        message_dir.join("msg_user.json"),
    )
    .expect("good message fixture");
    fs::copy(
        repo_testdata().join("testdata/opencode/storage/part/msg_user/part_text.json"),
        part_dir.join("part_text.json"),
    )
    .expect("good part");

    // A well-formed but >cap message record: valid JSON with a huge padding
    // string. The cap (8 MiB) must reject it at the metadata gate without
    // reading it whole, and the parse must disclose the capped record.
    let mut fat = String::from(
        r#"{"id":"msg_assistant","role":"assistant","time":{"created":1764750000000},"pad":""#,
    );
    while fat.len() <= 8 * 1024 * 1024 {
        fat.push_str(&"A".repeat(64 * 1024));
    }
    fat.push_str(r#""}"#);
    fs::write(message_dir.join("msg_assistant.json"), &fat).expect("fat message");
    fs::write(
        part_dir.join("part_text.json"),
        r#"{"id":"part_text","type":"text","text":"hello"}"#,
    )
    .expect("part record");
    let sessions = load_sessions_from_dir(Some(&storage));
    let opencode: Vec<&Session> = sessions
        .iter()
        .filter(|session| session.metrics.source_tool == "opencode")
        .collect();
    assert!(
        !opencode.is_empty(),
        "corpus with one fat record must still parse"
    );
    let disclosed = opencode
        .iter()
        .filter(|session| {
            session
                .metrics
                .disclosure_counters
                .get("opencode_record_capped")
                .is_some_and(|count| *count > 0)
        })
        .count();
    assert_eq!(
        disclosed, 1,
        "the capped record must be disclosed on the session"
    );

    let _ = fs::remove_dir_all(corpus);
}

/// independent_review 09b3e188 F1 (red-first): a MULTI-MODEL mirror
/// family. `parse_gemini_object` threads one mutable model per object, so
/// a chats file whose nested checkpoint lane switched models mid-chat
/// finalizes as `model_used == "multiple"` with per-model attribution —
/// the canonical session prices that per attribution entry. The merge
/// must do the same: repricing the merged totals at the `"multiple"`
/// lookup bills the whole conversation at the builtin default
/// (claude-opus-4.7), and the member's fuller per-model snapshot must
/// fold per key (cumulative MAX), not be dropped.
#[test]
fn gemini_multi_model_mirror_reprices_per_attribution() {
    let corpus = temp_corpus("multimodel");
    let chats_dir = corpus.join(".gemini/tmp/conv-mm/chats");
    let checkpoints_dir = corpus.join(".gemini/tmp/conv-mm/checkpoints");
    fs::create_dir_all(&chats_dir).expect("chats dir");
    fs::create_dir_all(&checkpoints_dir).expect("checkpoints dir");
    // Headline lane on gemini-2.5-pro, then an embedded checkpoint lane on
    // gemini-2.5-flash — one file, two models, both with usage.
    fs::write(
        chats_dir.join("gemini-current-chat.json"),
        r#"{
            "modelVersion": "gemini-2.5-pro",
            "usageMetadata": {"promptTokenCount": 1000, "candidatesTokenCount": 200},
            "history": [
                {"role": "user", "parts": [{"text": "hello"}]},
                {"role": "model", "parts": [{"text": "hi"}]}
            ],
            "checkpoint": {
                "model": "gemini-2.5-flash",
                "conversation": [
                    {"role": "user", "parts": [{"text": "hello"}]},
                    {"role": "model", "parts": [{"text": "hi again"}]}
                ],
                "tokenUsage": {"inputTokens": 500, "outputTokens": 100}
            }
        }"#,
    )
    .expect("multi-model chats file");
    // The checkpoint member: the SAME conversation's FULLER flash
    // snapshot (a thought part gives the member a reasoning lens the
    // chats lane never had — review fix F2's union pin).
    fs::write(
        checkpoints_dir.join("gemini-checkpoint.json"),
        r#"{
            "checkpoint": {
                "model": "gemini-2.5-flash",
                "conversation": [
                    {"role": "user", "parts": [{"text": "hello"}]},
                    {"role": "model", "parts": [
                        {"text": "hi again"},
                        {"thought": true, "text": "thinking it through carefully"}
                    ]}
                ],
                "tokenUsage": {"inputTokens": 900, "outputTokens": 150}
            }
        }"#,
    )
    .expect("checkpoint member file");

    let sessions = gemini_sessions(&corpus);
    assert_eq!(
        sessions.len(),
        1,
        "multi-model mirror family is still ONE conversation, got {}",
        sessions.len()
    );
    let merged = &sessions[0];
    assert_eq!(merged.metrics.model_used, "multiple");

    let pro = merged
        .metrics
        .model_attribution
        .get("gemini-2.5-pro")
        .expect("pro attribution entry");
    assert_eq!((pro.input_tokens, pro.output_tokens), (1000, 200));
    let flash = merged
        .metrics
        .model_attribution
        .get("gemini-2.5-flash")
        .expect("flash attribution entry");
    assert_eq!(
        (flash.input_tokens, flash.output_tokens),
        (900, 150),
        "the member's fuller flash snapshot wins per key (cumulative MAX)"
    );

    let price_of = |model: &str, input: i64, output: i64| {
        let price = lookup_price(model);
        input as f64 / 1e6 * price.input + output as f64 / 1e6 * price.output
    };
    assert_eq!(
        merged.metrics.cost_estimated,
        round4(price_of("gemini-2.5-pro", 1000, 200) + price_of("gemini-2.5-flash", 900, 150)),
        "multi-model mirrors price each model's tokens at that model's rates — \
         not the builtin default over the merged totals"
    );
    assert_eq!(
        merged.metrics.provenance.cost, "calculated_per_message_tokens",
        "the canonical multi-model provenance"
    );
    assert_eq!(
        merged
            .metrics
            .disclosure_counters
            .get("cross_file_sessions_merged"),
        Some(&1),
        "the multi-model family still discloses its merge"
    );
    assert!(
        !merged.metrics.reasoning_lens.is_empty(),
        "review fix F2: the checkpoint member's reasoning lens survives the fold"
    );

    let _ = fs::remove_dir_all(corpus);
}

/// independent_review 09b3e188 F4 (red-first): a session whose EVERY
/// message record is over the cap. The records were FOUND and skipped —
/// not absent — so the session must materialize with the
/// `opencode_record_capped` disclosure and zero messages, instead of
/// bailing "no messages found" behind a swallowed Err and surfacing as
/// "unsupported session format".
#[test]
fn opencode_all_records_capped_session_discloses_instead_of_vanishing() {
    let corpus = temp_corpus("opencode-allcapped");
    let storage = corpus.join("opencode").join("storage");
    let session_dir = storage.join("session").join("project_alpha");
    let message_dir = storage.join("message").join("ses_abc");
    fs::create_dir_all(&session_dir).expect("session dir");
    fs::create_dir_all(&message_dir).expect("message dir");
    fs::copy(
        repo_testdata().join("testdata/opencode/storage/session/project_alpha/ses_abc.json"),
        session_dir.join("ses_abc.json"),
    )
    .expect("session fixture");
    // The ONLY message record: well-formed JSON but far over the cap.
    let mut fat = String::from(
        r#"{"id":"msg_assistant","role":"assistant","time":{"created":1764750000000},"pad":""#,
    );
    while fat.len() <= 8 * 1024 * 1024 {
        fat.push_str(&"A".repeat(64 * 1024));
    }
    fat.push_str(r#""}"#);
    fs::write(message_dir.join("msg_assistant.json"), &fat).expect("the only (fat) message");

    let sessions = load_sessions_from_dir(Some(&storage));
    let opencode: Vec<&Session> = sessions
        .iter()
        .filter(|session| session.metrics.source_tool == "opencode")
        .collect();
    assert_eq!(
        opencode.len(),
        1,
        "an all-capped session EXISTS with a disclosure — found-and-skipped is not absent \
         (got {} opencode sessions)",
        opencode.len()
    );
    let session = opencode[0];
    assert_eq!(
        session
            .metrics
            .disclosure_counters
            .get("opencode_record_capped"),
        Some(&1),
        "the capped skip is disclosed on the surviving session"
    );
    assert_eq!(
        session.metrics.user_messages, 0,
        "zero messages survived — the session is the disclosure carrier"
    );

    let _ = fs::remove_dir_all(corpus);
}
