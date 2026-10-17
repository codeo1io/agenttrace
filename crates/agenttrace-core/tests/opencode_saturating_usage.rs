//! rm-046: saturating token merges on the opencode storage lane.
//!
//! The acceptance fixture named on the wall row: an adversarial corpus with
//! TWO i64::MAX inputs (message-level folds and the rm-619 step-finish part
//! rescue) parses WITHOUT panicking in debug builds and reports the clamped
//! ceiling — plain `+=` panicked in debug and wrapped negative in release,
//! where the `> 0` consumers silently dropped the whole class (MORE hostile
//! input reported LESS usage). `add_opencode_tokens` inherits the saturating
//! contract from `add_usage_value`/`add_usage`.

use std::fs;

use agenttrace_core::parse_file;

#[test]
fn token_accumulation_saturates_not_wraps() {
    let root = std::env::temp_dir().join(format!("at_rm046_sat_{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    let session_dir = root.join("opencode/storage/session/sess_1");
    let message_dir = root.join("opencode/storage/message/sess_1");
    let part_dir = root.join("opencode/storage/part/m3");
    fs::create_dir_all(&session_dir).unwrap();
    fs::create_dir_all(&message_dir).unwrap();
    fs::create_dir_all(&part_dir).unwrap();
    let session_file = session_dir.join("sess_1.json");
    fs::write(
        &session_file,
        r#"{"id": "sess_1", "slug": "saturation", "projectID": "prj_1", "title": "Saturation probe", "model": {"id": "claude-sonnet-4-5"}, "time": {"created": 1764750000000, "updated": 1764750004000}}"#,
    )
    .unwrap();
    // Two message-level i64::MAX inputs: the second fold must saturate, not
    // panic (debug) or wrap to a negative total (release).
    for (stem, id) in [("m1", "m1"), ("m2", "m2")] {
        fs::write(
            message_dir.join(format!("{stem}.json")),
            format!(
                r#"{{"id": "{id}", "sessionID": "sess_1", "role": "user", "content": "go", "tokens": {{"input": 9223372036854775807, "output": 5}}, "time": {{"created": 1764750001000}}}}"#
            ),
        )
        .unwrap();
    }
    // The part lane adds a third i64::MAX input through the rm-619 rescue
    // (m3 reports no message-level billing, so its step-finish part is the
    // only usage source): i64::MAX + i64::MAX via `add_usage` must saturate.
    fs::write(
        message_dir.join("m3.json"),
        r#"{"id": "m3", "sessionID": "sess_1", "role": "assistant", "time": {"created": 1764750002000}}"#,
    )
    .unwrap();
    fs::write(
        part_dir.join("p1.json"),
        r#"{"id": "p1", "type": "step-finish", "tokens": {"input": 9223372036854775807, "output": 7}, "time": {"created": 1764750002500}}"#,
    )
    .unwrap();

    let session = parse_file(&session_file).unwrap();

    assert_eq!(
        session.metrics.tokens_input,
        i64::MAX,
        "two i64::MAX message folds plus an i64::MAX part rescue saturate at \
         the ceiling — pre-fix this panicked in debug and wrapped negative in \
         release (the >0 consumers then zeroed the class)"
    );
    assert_eq!(
        session.metrics.tokens_output, 17,
        "the honest classes still sum: 5 + 5 message outputs + 7 part output"
    );
    let _ = fs::remove_dir_all(&root);
}
