//! Property-based parser invariants (rm-452).
//!
//! The host this batch runs on has no registry access, so instead of a
//! proptest dependency this is an in-crate seeded property harness: a
//! deterministic xorshift64* generator drives randomized inputs through
//! the public parse surface, and any failing case reports its seed and
//! index so it can be replayed exactly (set `AGENTTRACE_PROPERTY_SEED`
//! and `AGENTTRACE_PROPERTY_CASES`; CI pins a smaller case count).
//!
//! Scoped exclusion (rm-449, in flight in a sibling worktree): the
//! detector-dispatch property deliberately does NOT generate objects
//! carrying BOTH `session_id` and `sessionId` — that dual-key class is a
//! live misclassification bug with a fix under way; tightening the
//! property to cover it lands with that fix's integration.
//!
//! Contract: a transcript parser may return `Err` for garbage, but it may
//! never panic, hang, or produce different accounting for the same
//! logical content presented with different key order.

use agenttrace_core::parse_raw_session;

struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Rng(seed.max(1))
    }
    fn next(&mut self) -> u64 {
        // xorshift64*
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    fn below(&mut self, n: u64) -> u64 {
        self.next() % n.max(1)
    }
}

fn case_count() -> usize {
    std::env::var("AGENTTRACE_PROPERTY_CASES")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(128)
}

fn seed() -> u64 {
    std::env::var("AGENTTRACE_PROPERTY_SEED")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(0x0045_0DA1_10C5_2026)
}

/// Renders `{ "k": v, ... }` with keys in RNG-chosen order, recursively
/// for nested maps, so the raw text genuinely varies while the logical
/// content stays fixed.
fn render_shuffled(value: &serde_json::Value, rng: &mut Rng, out: &mut String) {
    match value {
        serde_json::Value::Object(map) => {
            let mut keys: Vec<&String> = map.keys().collect();
            // Fisher-Yates under the seeded RNG.
            for i in (1..keys.len()).rev() {
                let j = rng.below((i + 1) as u64) as usize;
                keys.swap(i, j);
            }
            out.push('{');
            for (i, k) in keys.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                out.push_str(&serde_json::to_string(k).expect("key serializes"));
                out.push(':');
                render_shuffled(&map[*k], rng, out);
            }
            out.push('}');
        }
        other => out.push_str(&serde_json::to_string(other).expect("value serializes")),
    }
}

const WORKBUDDY_LINES: &[&str] = &[
    r#"{"type":"function_call","name":"bash","arguments":"{}","sessionId":"wb-prop","cwd":"/tmp/proj","callId":"c1"}"#,
    r#"{"type":"message","role":"user","content":[{"type":"text","text":"run the thing"}],"sessionId":"wb-prop","cwd":"/tmp/proj","message":{"role":"user","usage":{"input_tokens":2000,"output_tokens":50,"cache_read_input_tokens":1500}}}"#,
    r#"{"type":"function_call_result","output":"done","callId":"c1","sessionId":"wb-prop","cwd":"/tmp/proj"}"#,
];

#[test]
fn parser_never_panics_on_arbitrary_or_mutated_input() {
    let cases = case_count();
    let mut rng = Rng::new(seed());
    for case in 0..cases {
        let mode = rng.below(6);
        let raw = match mode {
            0 => (0..rng.below(512) + 1)
                .map(|_| rng.below(256) as u8)
                .collect::<Vec<u8>>(),
            1 => (0..rng.below(512) + 1)
                .map(|_| b' ' + rng.below(95) as u8)
                .collect::<Vec<u8>>(),
            2 | 3 => {
                // Mutate a valid transcript: random flips/inserts/deletes.
                let base = WORKBUDDY_LINES.join("\n").into_bytes();
                let mut buf = base;
                for _ in 0..rng.below(16) + 1 {
                    if buf.is_empty() {
                        break;
                    }
                    match rng.below(3) {
                        0 => {
                            let at = rng.below(buf.len() as u64) as usize;
                            buf[at] ^= rng.below(256) as u8;
                        }
                        1 => buf.insert(rng.below(buf.len() as u64) as usize, rng.below(256) as u8),
                        _ => {
                            buf.remove(rng.below(buf.len() as u64) as usize);
                        }
                    }
                }
                buf
            }
            4 => {
                // Deep nesting / pathological literals.
                let depth = rng.below(48) + 8;
                let mut s = String::new();
                for _ in 0..depth {
                    s.push_str("{\"a\":");
                }
                s.push_str("{\"input_tokens\":99999999999999999999,");
                s.push_str("\"x\":1e999,\"y\":NaN}");
                for _ in 0..depth {
                    s.push('}');
                }
                s.into_bytes()
            }
            _ => WORKBUDDY_LINES[..rng.below(WORKBUDDY_LINES.len() as u64 + 1) as usize]
                .join("\n")
                .into_bytes(),
        };
        let raw = String::from_utf8_lossy(&raw).into_owned();
        let result = std::panic::catch_unwind(|| parse_raw_session("prop", "prop.jsonl", &raw));
        match result {
            Ok(parsed) => {
                if let Ok(session) = parsed {
                    // Whatever survived parsing must keep sane saturating
                    // invariants: totals never negative.
                    assert!(
                        session.metrics.tokens_input >= 0
                            && session.metrics.tokens_output >= 0
                            && session.metrics.tokens_cache_r >= 0,
                        "case {case} (seed {:#x}, mode {mode}): negative totals from \
                         input head {:?}",
                        seed(),
                        &raw[..raw.len().min(80)]
                    );
                }
            }
            Err(_) => panic!(
                "case {case} (seed {:#x}, mode {mode}) panicked on input head {:?}",
                seed(),
                &raw[..raw.len().min(120)]
            ),
        }
    }
}

#[test]
fn usage_accounting_is_invariant_under_key_and_line_order() {
    // Same logical transcript, shuffled key order at every object level
    // and shuffled line order: token accounting must not move.
    let cases = case_count();
    let mut rng = Rng::new(seed().wrapping_add(1));
    let template: Vec<serde_json::Value> = WORKBUDDY_LINES
        .iter()
        .map(|l| serde_json::from_str(l).expect("template line parses"))
        .collect();

    for case in 0..cases {
        let mut lines = Vec::new();
        let mut order: Vec<usize> = (0..template.len()).collect();
        for i in (1..order.len()).rev() {
            let j = rng.below((i + 1) as u64) as usize;
            order.swap(i, j);
        }
        for &idx in &order {
            let mut rendered = String::new();
            render_shuffled(&template[idx], &mut rng, &mut rendered);
            lines.push(rendered);
        }
        let raw = lines.join("\n");
        let session = parse_raw_session("perm", "perm.jsonl", &raw)
            .unwrap_or_else(|e| panic!("case {case} (seed {:#x}) failed to parse: {e}", seed()));

        assert_eq!(
            session.metrics.source_tool,
            "workbuddy",
            "case {case} (seed {:#x}) dispatched away from workbuddy",
            seed()
        );
        assert_eq!(
            session.metrics.tokens_input,
            500,
            "case {case} (seed {:#x})",
            seed()
        );
        assert_eq!(
            session.metrics.tokens_output,
            50,
            "case {case} (seed {:#x})",
            seed()
        );
        assert_eq!(
            session.metrics.tokens_cache_r,
            1500,
            "case {case} (seed {:#x})",
            seed()
        );
        assert_eq!(
            session
                .metrics
                .line_skips
                .get("workbuddy_input_basis:cache_subtracted"),
            Some(&1),
            "case {case} (seed {:#x}): basis disclosure moved",
            seed()
        );
    }
}

#[test]
fn parsing_is_deterministic_and_adversarial_key_mixes_stay_safe() {
    // rm-449 exclusion: no generated object carries both `session_id` and
    // `sessionId` — that class is a live bug with a fix in flight in a
    // sibling worktree; cover it when that lands.
    let cases = case_count();
    let mut rng = Rng::new(seed().wrapping_add(2));
    for case in 0..cases {
        let mut obj = serde_json::Map::new();
        match rng.below(4) {
            0 => {
                obj.insert("type".into(), "message".into());
                obj.insert("sessionId".into(), "s".into());
                // deliberately no cwd
            }
            1 => {
                obj.insert("type".into(), "function_call".into());
                obj.insert("cwd".into(), "/tmp".into());
                // deliberately no sessionId
            }
            2 => {
                obj.insert("sessionId".into(), "s".into());
                obj.insert("cwd".into(), "/tmp".into());
                // deliberately no type
            }
            _ => {
                obj.insert(
                    "message".into(),
                    serde_json::json!({"usage": {
                        "input_tokens": 7,
                        "output_tokens": 3,
                    }}),
                );
            }
        }
        if rng.below(2) == 0 {
            obj.insert(
                "content".into(),
                serde_json::json!([{"type":"text","text":"x"}]),
            );
        }
        let raw = serde_json::Value::Object(obj).to_string();
        let first = std::panic::catch_unwind(|| parse_raw_session("mix", "mix.jsonl", &raw));
        let second = std::panic::catch_unwind(|| parse_raw_session("mix", "mix.jsonl", &raw));
        let (first, second) = match (first, second) {
            (Ok(a), Ok(b)) => (a, b),
            _ => panic!("case {case} (seed {:#x}) panicked on {raw}", seed()),
        };
        match (first, second) {
            (Ok(a), Ok(b)) => {
                assert_eq!(
                    a.metrics.tokens_input,
                    b.metrics.tokens_input,
                    "case {case} (seed {:#x}): nondeterministic accounting",
                    seed()
                );
                assert_eq!(a.metrics.source_tool, b.metrics.source_tool);
            }
            (Err(_), Err(_)) => {}
            _ => panic!(
                "case {case} (seed {:#x}): nondeterministic accept/reject",
                seed()
            ),
        }
    }
}

#[test]
fn workbuddy_usage_sums_across_records_upstream_311() {
    // rm-542 / upstream #311: workbuddy journals carry one usage block
    // per assistant record and the session total is their SUM. The
    // previous keep-last arm reported only the FINAL record's tokens —
    // the research live PoC (wb1): 100/10 + 200/20 + 300/30 reported
    // 330 instead of 660.
    // Detector guard: the workbuddy probe requires at least one
    // function_call/function_call_result/reasoning record, so the
    // corpus carries one non-usage record alongside the usage rows.
    let raw = [
        r#"{"type":"function_call","name":"bash","arguments":"{}","callId":"c1","sessionId":"wb-sum","cwd":"/tmp"}"#,
        r#"{"type":"message","role":"user","content":[{"type":"text","text":"a"}],"sessionId":"wb-sum","cwd":"/tmp","message":{"role":"user","usage":{"input_tokens":100,"output_tokens":10}}}"#,
        r#"{"type":"message","role":"user","content":[{"type":"text","text":"b"}],"sessionId":"wb-sum","cwd":"/tmp","message":{"role":"user","usage":{"input_tokens":200,"output_tokens":20}}}"#,
        r#"{"type":"message","role":"user","content":[{"type":"text","text":"c"}],"sessionId":"wb-sum","cwd":"/tmp","message":{"role":"user","usage":{"input_tokens":300,"output_tokens":30}}}"#,
    ]
    .join("\n");
    let session = parse_raw_session("wb", "wb-sum.jsonl", &raw).expect("parses");
    assert_eq!(session.metrics.source_tool, "workbuddy");
    assert_eq!(
        session.metrics.tokens_input, 600,
        "sum, not keep-last (was 300)"
    );
    assert_eq!(session.metrics.tokens_output, 60);

    // The sum is order-invariant: reversed records give the same total.
    let reversed: Vec<&str> = raw.split('\n').collect();
    let mut reversed = reversed.clone();
    reversed.reverse();
    let session = parse_raw_session("wb", "wb-sum.jsonl", &reversed.join("\n")).expect("parses");
    assert_eq!(session.metrics.tokens_input, 600);
    assert_eq!(session.metrics.tokens_output, 60);
}

#[test]
fn workbuddy_clamps_cached_above_input_upstream_316() {
    // rm-542 / upstream #316 live PoC (wb7): a record reports 150
    // cached tokens against 100 input. The previous subtraction kept
    // the full cache count, so input+cache (0 + 150 = 150) EXCEEDED
    // the source-recorded input. Both sides clamp, and the clamp rides
    // the rm-450 disclosure family so the report never hides it.
    let raw = [
        r#"{"type":"function_call","name":"bash","arguments":"{}","callId":"c1","sessionId":"wb-clamp","cwd":"/tmp"}"#,
        r#"{"type":"message","role":"user","content":[{"type":"text","text":"go"}],"sessionId":"wb-clamp","cwd":"/tmp","message":{"role":"user","usage":{"input_tokens":100,"output_tokens":10,"cache_read_input_tokens":150}}}"#,
    ]
    .join("\n");
    let session = parse_raw_session("wb", "wb-clamp.jsonl", &raw).expect("parses");
    assert_eq!(session.metrics.tokens_input, 0);
    assert_eq!(
        session.metrics.tokens_cache_r, 100,
        "cached clamps to the recorded input"
    );
    assert_eq!(session.metrics.tokens_output, 10);
    assert_eq!(
        session
            .metrics
            .line_skips
            .get("workbuddy_input_basis:cache_clamped"),
        Some(&1),
        "clamp disclosed, not silent"
    );
}

#[test]
fn workbuddy_reasoning_records_contribute_usage_upstream_311_rider() {
    // rm-542 rider: the "reasoning" arm previously never read the
    // usage block, silently dropping those tokens from the session
    // total (research PoC wb3).
    let raw = [
        r#"{"type":"reasoning","content":[{"type":"text","text":"thinking"}],"sessionId":"wb-reason","cwd":"/tmp","message":{"role":"assistant","usage":{"input_tokens":5,"output_tokens":7}}}"#,
        r#"{"type":"message","role":"user","content":[{"type":"text","text":"hi"}],"sessionId":"wb-reason","cwd":"/tmp","message":{"role":"user","usage":{"input_tokens":50,"output_tokens":3}}}"#,
    ]
    .join("\n");
    let session = parse_raw_session("wb", "wb-reason.jsonl", &raw).expect("parses");
    assert_eq!(
        session.metrics.tokens_input, 55,
        "reasoning record usage counted"
    );
    assert_eq!(session.metrics.tokens_output, 10);
}
