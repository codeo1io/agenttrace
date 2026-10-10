use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

// rm-911 (arm b): warm-cache card parity. `SessionMetrics::timestamps`
// is `#[serde(skip)]` (crates/agenttrace-core/src/lib.rs), so a session
// replayed from the warm session cache carries an EMPTY timestamp list —
// and the usage card's daily-spend series, which bucketed each session
// at its last event date, saw no dates at all: the warm card rendered
// the flat 1970-01-01 window while the cold card showed the real corpus
// days (assess F1 PoC: /tmp/at-assess/c6-cold.svg vs c6-warm.svg, axis
// 2026-09-26/2026-10-09 collapsing to 1970-01-01/1970-01-01).
//
// The series is now derived from the cache-SURVIVING session bounds
// (`session_end` — exactly the first/last event stamps on every lane
// that has them, rm-502's otel `session_bounds` precedent), so a cold
// render and a warm replay of the SAME corpus produce byte-identical
// output for every overview lane. This is the end-to-end golden:
// same corpus, fresh cache, then warm cache, byte-equal bytes.
//
// The fixture spans two days across two journals with distinct costs,
// so a wrong (flat) derivation cannot hide inside a single bucket.

fn journal(session_id: &str, day: &str, assistant_cost: &str) -> String {
    format!(
        concat!(
            "{{\"type\":\"user\",\"timestamp\":\"{day}T09:00:00Z\",\"message\":{{\"role\":\"user\",\"content\":\"go\"}},\"uuid\":\"u1\",\"sessionId\":\"{sid}\"}}\n",
            "{{\"type\":\"assistant\",\"timestamp\":\"{day}T09:00:05Z\",\"message\":{{\"role\":\"assistant\",\"model\":\"claude-sonnet-4-20250514\",\"content\":[{{\"type\":\"text\",\"text\":\"ok\"}}]}},\"uuid\":\"a1\",\"sessionId\":\"{sid}\",\"costUSD\":{cost}}}\n"
        ),
        day = day,
        sid = session_id,
        cost = assistant_cost,
    )
}

fn unique_root() -> PathBuf {
    let root = std::env::temp_dir().join(format!(
        "agenttrace-rm911-warm-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock")
            .as_nanos()
    ));
    fs::create_dir_all(root.join("corpus")).expect("create corpus dir");
    root
}

fn run_overview(root: &Path, format: &str) -> String {
    let output = Command::new(env!("CARGO_BIN_EXE_agenttrace"))
        .args(["--overview", "-d", "corpus", "-f", format])
        .current_dir(root)
        .env("HOME", root)
        .env("XDG_CACHE_HOME", root.join("cache"))
        .env("AGENTTRACE_SESSION_CACHE_DIR", root.join("cache/sess"))
        .output()
        .expect("run agenttrace CLI");
    assert!(
        output.status.success(),
        "CLI failed: {:?}\n{}",
        output.status,
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn seed_corpus(root: &Path) {
    fs::write(
        root.join("corpus/a.jsonl"),
        journal("s-early", "2026-10-04", "0.02"),
    )
    .expect("write early journal");
    fs::write(
        root.join("corpus/b.jsonl"),
        journal("s-late", "2026-10-06", "0.01"),
    )
    .expect("write late journal");
}

#[test]
fn warm_card_replay_is_byte_identical_and_uses_real_days() {
    let root = unique_root();
    seed_corpus(&root);

    // Cold scan: fresh cache (the sandbox starts empty), real corpus days.
    let cold = run_overview(&root, "svg");
    assert!(
        cold.starts_with("<?xml"),
        "cold card must be SVG, got {cold:?}"
    );
    assert!(
        cold.contains("2026-10-06"),
        "cold card must show the latest corpus day on the axis, got: {}",
        axis_of(&cold)
    );
    assert!(
        !cold.contains("1970-01-01"),
        "cold card must never show the 1970 flat window"
    );

    // The cold run must actually have populated the session cache; the
    // warm run below is then a genuine cache replay (and a pure hit
    // leaves the cache journal bytes untouched).
    let cache_journal = root.join("cache/sess/sessions.json");
    let cold_cache = fs::read(&cache_journal).expect("cold run populated the session cache");
    assert!(!cold_cache.is_empty());

    // Warm replay of the same corpus: byte-identical card, real days.
    let warm = run_overview(&root, "svg");
    assert!(
        !warm.contains("1970-01-01"),
        "warm card must show the corpus days, not the 1970 flat window: {}",
        axis_of(&warm)
    );
    assert_eq!(
        cold, warm,
        "cold and warm renders of the same corpus must be byte-identical"
    );
    assert_eq!(
        fs::read(&cache_journal).expect("cache journal readable after warm"),
        cold_cache,
        "a pure warm hit must leave the cache journal bytes untouched"
    );

    let _ = fs::remove_dir_all(root);
}

#[test]
fn warm_replay_is_byte_identical_for_markdown_html_and_csv() {
    // rm-911's acceptance covers the other shareable lanes too: they
    // must not leak cache-dropped fields into their renders. The one
    // legitimate cold-vs-warm delta is the parse-coverage disclosure
    // ("N cache hits" — the honesty table reporting the warm replay
    // itself), so both sides are normalized on that single counter and
    // must then be byte-identical. Verified against the unfixed tree:
    // these lanes never consumed `timestamps`, so this is the
    // "proven timestamps-free" arm of the acceptance (a property pin,
    // green before and after the fix), while the SVG test above is the
    // bug repro.
    for format in ["markdown", "html", "csv"] {
        let root = unique_root();
        seed_corpus(&root);
        let cold = run_overview(&root, format);
        assert!(
            !cold.is_empty(),
            "-f {format} cold render must be non-empty"
        );
        let warm = run_overview(&root, format);
        // General counter normalizer: ANY digit-run immediately before
        // " cache hits" is the disclosed parse-coverage counter (however
        // many sessions the corpus has), so it is folded to N on both
        // sides — not just the 0/1/2 this fixture happens to produce
        // (review-fix F2: the enumerated form would drift on a corpus
        // with ≥3 sessions).
        let strip_hits = |s: &str| -> String {
            let parts: Vec<&str> = s.split(" cache hits").collect();
            let last = parts.len() - 1;
            parts
                .iter()
                .enumerate()
                .map(|(i, p)| {
                    if i == last {
                        (*p).to_string()
                    } else {
                        format!("{}N", p.trim_end_matches(|c: char| c.is_ascii_digit()))
                    }
                })
                .collect::<Vec<_>>()
                .join(" cache hits")
        };
        assert_eq!(
            strip_hits(&cold),
            strip_hits(&warm),
            "-f {format}: cold and warm renders must differ at most in the \
             disclosed cache-hit counter"
        );
        let _ = fs::remove_dir_all(root);
    }
}

/// Extract the daily-axis date labels out of a rendered card so failure
/// messages show exactly what the sparkline window degenerated to.
fn axis_of(svg: &str) -> String {
    svg.lines()
        .filter(|line| line.contains("card-muted") && line.contains("y=\"38"))
        .map(|line| line.to_string())
        .collect::<Vec<_>>()
        .join(" | ")
}
