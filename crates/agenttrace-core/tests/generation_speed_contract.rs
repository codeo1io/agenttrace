// rm-995 (batch run 12e2c18c implement): generation-speed analytics
// contract. Before the change the Session/Metrics model already
// recorded everything needed to derive generation speed — timestamps
// (first/last event) and counted tokens — but no surface rendered it:
// `grep -rniE 'tokens.per.second|tok/s'` found ZERO hits across the
// crates (research 4c1e1b48 N1). The contract pinned here:
//   1. per-session rate = counted tokens / duration over the event
//      timestamp span — RENDER-TIME arithmetic only, so a cold-parsed
//      and a warm-cache session of the same transcript carry the
//      identical number (no new cache field, no schema bump);
//   2. corpus/per-model rollups are WEIGHTED (sum tokens over sum
//      duration), never an average of averages;
//   3. time-to-first-token = first assistant stamp minus earliest
//      event stamp, and only where parse-time timestamps allow — the
//      session cache does not round-trip event stamps, so warm-cache
//      sessions honestly stay out of the TTFT basis (null / absent,
//      with the basis size printed beside every number);
//   4. sessions without a usable span (single stamp, none) are
//      excluded from every rate, and a fully rate-less corpus renders
//      NO speed section at all (0-count-clean).

use agenttrace_core::{
    compute_overview, generation_speed_payload, parse_file, report_overview_json,
    report_overview_text, Session,
};
use serde_json::Value;
use std::fs;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

fn write(tag: &str, raw: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!("at-rm995-{}-{tag}.json", std::process::id()));
    fs::write(&path, raw).expect("write fixture");
    path
}

fn session(tag: &str, raw: &str) -> Session {
    let path = write(tag, raw);
    let session = parse_file(&path).expect("session parses");
    let _ = fs::remove_file(&path);
    session
}

/// claude-style role journal: usage on a meta line, timestamps on the
/// events. tokens = in+out = `tokens`; span = `span_sec`;
/// first assistant stamp sits `ttft_sec` after the earliest event.
fn journal(model: &str, tokens_in: i64, tokens_out: i64, span_sec: i64, ttft_sec: i64) -> String {
    let model_field = if model.is_empty() {
        String::new()
    } else {
        format!("\"ModelUsed\":\"{model}\",")
    };
    format!(
        concat!(
            "{{\"role\":\"session_meta\",\"timestamp\":\"2026-05-02T10:00:00Z\",{model_field}}}\n",
            "{{\"role\":\"meta\",{model_field}\"Usage\":{{\"input_tokens\":{tokens_in},\"output_tokens\":{tokens_out}}}}}\n",
            "{{\"role\":\"user\",\"content\":\"inspect\",\"timestamp\":\"2026-05-02T10:00:00Z\"}}\n",
            "{{\"role\":\"assistant\",\"content\":\"on it\",\"timestamp\":\"2026-05-02T10:00:{ttft_sec:02}Z\"}}\n",
            "{{\"role\":\"tool\",\"content\":\"{{}}\",\"timestamp\":\"2026-05-02T10:00:{span_sec:02}Z\"}}\n"
        ),
        model_field = model_field,
        tokens_in = tokens_in,
        tokens_out = tokens_out,
        ttft_sec = ttft_sec,
        span_sec = span_sec,
    )
}

#[test]
fn per_session_rate_is_counted_tokens_over_span() {
    // 1500 counted tokens over a 10s span = 150 tok/s; TTFT 2s.
    let session = session("rate-150", &journal("claude-sonnet-4", 1000, 500, 10, 2));
    let metrics = &session.metrics;
    assert!(metrics.duration_sec > 0.0, "span parsed: {metrics:?}");
    let rate = metrics.tokens_per_sec().expect("rate computable");
    assert!(
        (rate - 150.0).abs() < 1e-6,
        "1500 tokens / 10s = 150 tok/s, got {rate}"
    );
    let ttft = metrics.ttft_sec().expect("ttft computable");
    assert!(
        (ttft - 2.0).abs() < 1e-6,
        "first assistant stamp sits 2s after the earliest event, got {ttft}"
    );
}

#[test]
fn corpus_rollup_is_weighted_not_an_average_of_averages() {
    // Session A: 1500 tokens / 10s = 150 tok/s (model-a)
    // Session B: 3000 tokens / 30s = 100 tok/s (model-b)
    // Weighted corpus = 4500/40 = 112.5 — the average-of-averages
    // (125.0) is exactly the number this test forbids.
    let a = session("weight-a", &journal("model-a", 1000, 500, 10, 2));
    let b = session("weight-b", &journal("model-b", 2000, 1000, 30, 4));
    let payload = generation_speed_payload([&a, &b]);
    assert_eq!(
        payload.get("sessions_with_rate").and_then(Value::as_u64),
        Some(2)
    );
    let corpus = payload
        .get("tokens_per_sec")
        .and_then(Value::as_f64)
        .expect("corpus rate");
    assert!(
        (corpus - 112.5).abs() < 1e-6,
        "weighted 4500/40 = 112.5 tok/s, not the 125.0 average of averages, got {corpus}"
    );
    let by_model = payload
        .get("by_model")
        .and_then(Value::as_array)
        .expect("by_model rollup");
    assert_eq!(by_model.len(), 2, "one row per model: {by_model:?}");
    assert_eq!(
        by_model[0].get("model").and_then(Value::as_str),
        Some("model-a"),
        "rows sort by rate descending: {by_model:?}"
    );
    assert!(
        (by_model[0]
            .get("tokens_per_sec")
            .and_then(Value::as_f64)
            .unwrap_or(0.0)
            - 150.0)
            .abs()
            < 1e-6
    );
    let ttft = payload
        .get("time_to_first_token")
        .expect("ttft member always present with its basis");
    assert_eq!(ttft.get("sessions").and_then(Value::as_u64), Some(2));
    let avg = ttft.get("avg_sec").and_then(Value::as_f64).expect("avg");
    assert!((avg - 3.0).abs() < 1e-6, "(2s + 4s)/2 = 3s avg, got {avg}");
}

#[test]
fn spanless_sessions_are_excluded_and_clean_corpora_render_no_section() {
    // No timestamps at all: no rate, no TTFT — and a corpus made only
    // of such sessions carries NO speed section in text (0-count-clean)
    // while the JSON member still reports its (zero) basis honestly.
    let spanless_raw = concat!(
        "{\"role\":\"session_meta\",\"ModelUsed\":\"claude-sonnet-4\"}\n",
        "{\"role\":\"meta\",\"ModelUsed\":\"claude-sonnet-4\",\"Usage\":{\"input_tokens\":100,\"output_tokens\":50}}\n",
        "{\"role\":\"user\",\"content\":\"no stamps\"}\n",
        "{\"role\":\"assistant\",\"content\":\"none here\"}\n"
    );
    let spanless = session("spanless", spanless_raw);
    assert!(
        spanless.metrics.duration_sec <= 0.0,
        "no timestamps means no span"
    );
    assert!(
        spanless.metrics.tokens_per_sec().is_none(),
        "no span, no rate"
    );
    assert!(spanless.metrics.ttft_sec().is_none());

    let payload = generation_speed_payload([&spanless]);
    assert_eq!(
        payload.get("sessions_with_rate").and_then(Value::as_u64),
        Some(0)
    );
    assert_eq!(
        payload.get("tokens_per_sec").and_then(Value::as_f64),
        Some(0.0)
    );

    let overview = compute_overview(std::slice::from_ref(&spanless));
    let text = report_overview_text(&overview, std::slice::from_ref(&spanless));
    assert!(
        !text.contains("Generation speed"),
        "0-count-clean: no speed section without a ratable session"
    );
    let json = report_overview_json(&overview, &[spanless]);
    let doc: Value = serde_json::from_str(&json).expect("overview json");
    // The JSON member is unconditional (machines get the basis), the
    // text section is not.
    assert!(
        doc.pointer("/summary/generation_speed/sessions_with_rate")
            .and_then(Value::as_u64)
            == Some(0)
    );
}

#[test]
fn overview_surfaces_carry_the_speed_and_its_basis() {
    let a = session("surface-a", &journal("model-a", 1000, 500, 10, 2));
    let b = session("surface-b", &journal("model-b", 2000, 1000, 30, 4));
    let sessions = [a, b];
    let overview = compute_overview(&sessions);
    let text = report_overview_text(&overview, &sessions);
    assert!(text.contains("── Generation speed ──"), "{text}");
    assert!(text.contains("150.0 tok/s"), "{text}");
    assert!(text.contains("model-a"), "{text}");
    assert!(
        text.contains("corpus: 112.5 tok/s over 2 session(s) with a span"),
        "{text}"
    );
    assert!(
        text.contains("time-to-first-token: 3.0s avg over 2 session(s)"),
        "{text}"
    );

    let json = report_overview_json(&overview, &sessions);
    let doc: Value = serde_json::from_str(&json).expect("overview json");
    assert!(doc
        .pointer("/summary/generation_speed/tokens_per_sec")
        .and_then(Value::as_f64)
        .is_some_and(|rate| (rate - 112.5).abs() < 1e-6));
    assert!(doc
        .pointer("/summary/generation_speed/basis")
        .and_then(Value::as_str)
        .is_some_and(|basis| basis.contains("counted tokens / duration_sec")));
}

// ---------------------------------------------------------------------------
// Warm-cache parity: the rate is render-time arithmetic over persisted
// Metrics, so a cache hit must carry the IDENTICAL rate, while TTFT —
// which needs parse-time timestamps the cache deliberately does not
// round-trip — must honestly degrade to None instead of being
// approximated.
// ---------------------------------------------------------------------------

fn lock_env() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

fn with_session_cache(cache: &std::path::Path, f: impl FnOnce()) {
    let result = {
        let _guard = lock_env()
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());
        let previous = std::env::var_os("AGENTTRACE_SESSION_CACHE_DIR");
        std::env::set_var("AGENTTRACE_SESSION_CACHE_DIR", cache);
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f));
        if let Some(previous) = previous {
            std::env::set_var("AGENTTRACE_SESSION_CACHE_DIR", previous);
        } else {
            std::env::remove_var("AGENTTRACE_SESSION_CACHE_DIR");
        }
        result
    };
    if let Err(payload) = result {
        std::panic::resume_unwind(payload);
    }
}

#[test]
fn warm_cache_keeps_the_rate_and_degrades_ttft_honestly() {
    let root = std::env::temp_dir().join(format!("at-rm995-warm-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    let sessions_dir = root.join("sessions");
    let cache_dir = root.join("cache");
    fs::create_dir_all(&sessions_dir).expect("mkdir sessions");
    fs::write(
        sessions_dir.join("session.jsonl"),
        journal("claude-sonnet-4", 1000, 500, 10, 2),
    )
    .expect("write journal");

    with_session_cache(&cache_dir, || {
        let cold_report = agenttrace_core::load_sessions_with_options(
            Some(&sessions_dir),
            &agenttrace_core::LoadOptions::default(),
        );
        assert_eq!(cold_report.sessions.len(), 1);
        let cold = &cold_report.sessions[0];
        let cold_rate = cold.metrics.tokens_per_sec();
        let cold_ttft = cold.metrics.ttft_sec();
        assert!(cold_rate.is_some(), "cold parse carries the rate");
        assert!(cold_ttft.is_some(), "cold parse carries the TTFT");

        let warm_report = agenttrace_core::load_sessions_with_options(
            Some(&sessions_dir),
            &agenttrace_core::LoadOptions::default(),
        );
        assert_eq!(warm_report.sessions.len(), 1);
        let warm_metrics = &warm_report.sessions[0].metrics;
        assert_eq!(
            warm_report.cache_hits, 1,
            "second pass must actually be a cache hit, or this test proves nothing"
        );
        assert!(
            warm_metrics
                .tokens_per_sec()
                .is_some_and(|rate| { (rate - cold_rate.expect("cold rate")).abs() < 1e-9 }),
            "warm-cache rate identical to cold: {:?} vs {cold_rate:?}",
            warm_metrics.tokens_per_sec()
        );
        assert!(
            warm_metrics.ttft_sec().is_none(),
            "the cache does not round-trip event stamps — warm TTFT must be None, not approximated"
        );
    });

    let _ = fs::remove_dir_all(root);
}
