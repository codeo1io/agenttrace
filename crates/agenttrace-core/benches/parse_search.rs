//! Benchmark harness — rm-2276 (cycle 3, run 6a4166cb).
//!
//! First pinned baselines for the hot read paths of `agenttrace-core`:
//! cold JSONL parse (format detection + per-lane fold) and session search
//! (index build + ranking). The corpus is fully deterministic (arithmetic
//! payloads, no RNG), so runs are comparable across machines and commits.
//!
//! Opt-in only: `cargo bench -p agenttrace-core --bench parse_search`
//! (NOT wired into `cargo test` or CI — the maintenance battery stays
//! unchanged). Add `-- --quick` for a fast smoke run.
//!
//! Recorded baselines (2026-10-22, 14-core shared runner under sibling
//! load, criterion 0.8.2, `--measurement-time 2`): cold_parse/1000_records
//! ≈ 7.0 ms; search/24_sessions ≈ 356 µs. Treat regressions beyond ±25%
//! as signal.

use agenttrace_core::{parse_jsonl_session, search_sessions, Session};
use criterion::{criterion_group, criterion_main, Criterion};
use std::hint::black_box;
use std::sync::OnceLock;

/// Deterministic claude-code-session-shape corpus: `turns` user/assistant
/// pairs with arithmetic usage patterns (no randomness).
fn corpus(turns: usize, salt: usize) -> String {
    let mut out = String::with_capacity(turns * 512);
    for i in 0..turns {
        let n = i + salt;
        out.push_str(&format!(
            "{{\"parentUuid\":null,\"isSidechain\":false,\"type\":\"user\",\
\"uuid\":\"u-{n:06}\",\"timestamp\":\"2026-10-20T09:00:{:02}.000Z\",\
\"message\":{{\"role\":\"user\",\"content\":\"audit request {n} for parser token fold\"}}}}\n",
            n % 60
        ));
        out.push_str(&format!(
            "{{\"parentUuid\":\"u-{n:06}\",\"isSidechain\":false,\"type\":\"assistant\",\
\"uuid\":\"a-{n:06}\",\"timestamp\":\"2026-10-20T09:01:{:02}.000Z\",\
\"message\":{{\"id\":\"msg-{n:06}\",\"role\":\"assistant\",\"model\":\"claude-opus-4-6\",\
\"content\":[{{\"type\":\"text\",\"text\":\"assistant turn {n} covering token attribution {n}\"}}],\
\"usage\":{{\"input_tokens\":{},\"output_tokens\":{},\
\"cache_creation_input_tokens\":{},\"cache_read_input_tokens\":{}}}}}}}\n",
            n % 60,
            1000 + n * 7,
            200 + n * 3,
            400 + n * 5,
            40_000 + n * 97
        ));
    }
    out
}

fn sessions() -> &'static Vec<Session> {
    static S: OnceLock<Vec<Session>> = OnceLock::new();
    S.get_or_init(|| {
        (0..24)
            .map(|k| {
                parse_jsonl_session(
                    &format!("bench-{k}"),
                    &format!("/tmp/bench-{k}.jsonl"),
                    &corpus(500, k),
                )
                .expect("bench corpus must parse")
            })
            .collect()
    })
}

fn bench_parse(c: &mut Criterion) {
    let raw = corpus(500, 0);
    assert!(raw.lines().count() == 1000);
    c.bench_function("cold_parse/1000_records", |b| {
        b.iter(|| {
            parse_jsonl_session(
                black_box("bench-parse"),
                black_box("/tmp/bench-parse.jsonl"),
                black_box(&raw),
            )
            .expect("bench corpus must parse")
        })
    });
}

fn bench_search(c: &mut Criterion) {
    let sessions = sessions();
    assert!(!sessions.is_empty());
    c.bench_function("search/24_sessions", |b| {
        b.iter(|| {
            search_sessions(
                black_box(sessions),
                black_box("parser token attribution audit"),
                black_box(20),
            )
        })
    });
}

criterion_group!(benches, bench_parse, bench_search);
criterion_main!(benches);
