use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

// rm-196: warm-cache cost truthfulness. A cached session replays the cost
// AND the provenance label baked at store time, so the freshness decision
// must cover pricing identity, not just artifact bytes. This is the
// end-to-end regression for the live PoC: swapping AGENTTRACE_PRICING_FILE
// without touching the session artifact used to leave the old bundled cost
// (5.1x overstatement) and the stale "bundled" label in place.
//
// The fixture is synthesized here (tempdir) rather than copied from the
// campaign PoC corpus: one Claude-lane journal whose single assistant
// message reports exactly 1M input / 1M output tokens for
// claude-sonnet-4-20250514, and an override file pricing that model at
// $1.0/M in and $2.0/M out — so the override-priced cost is exactly 3.0,
// independent of the bundled catalog (and refresh-proof).

const MODEL: &str = "claude-sonnet-4-20250514";

fn fixture_journal() -> String {
    format!(
        concat!(
            "{{\"id\":\"h1\",\"timestamp\":\"2026-10-05T10:00:00.000Z\",\"type\":\"session\",\"version\":3,\"cwd\":\"/tmp/warm\"}}\n",
            "{{\"id\":\"e1\",\"parentId\":\"h1\",\"timestamp\":\"2026-10-05T10:00:01.000Z\",\"type\":\"message\",\"message\":{{\"role\":\"user\",\"content\":[{{\"type\":\"text\",\"text\":\"go\"}}]}}}}\n",
            "{{\"id\":\"e2\",\"parentId\":\"e1\",\"timestamp\":\"2026-10-05T10:00:02.000Z\",\"type\":\"message\",\"message\":{{\"role\":\"assistant\",\"content\":[{{\"type\":\"text\",\"text\":\"ok\"}}],\"model\":\"{model}\",\"usage\":{{\"input\":1000000,\"output\":1000000,\"cacheRead\":0,\"cacheWrite\":0,\"totalTokens\":2000000}}}}}}\n"
        ),
        model = MODEL
    )
}

fn override_file() -> String {
    format!(
        "{{\"prices\":{{\"{model}\":{{\"input\":1.0,\"output\":2.0,\"cw\":0.5,\"cr\":0.25}}}}}}",
        model = MODEL
    )
}

struct Run {
    cost: f64,
    pricing_source: String,
}

fn run_agenttrace(root: &Path, override_path: Option<&Path>) -> Run {
    let mut command = Command::new(env!("CARGO_BIN_EXE_agenttrace"));
    command
        .args(["-d", "corpus", "--diagnostics", "-f", "json"])
        .current_dir(root)
        .env("XDG_CACHE_HOME", root.join("cache"))
        .env("AGENTTRACE_SESSION_CACHE_DIR", root.join("cache/sess"))
        .env("HOME", root);
    match override_path {
        Some(path) => command.env("AGENTTRACE_PRICING_FILE", path),
        None => command.env_remove("AGENTTRACE_PRICING_FILE"),
    };
    let output = command.output().expect("run agenttrace CLI");
    assert!(
        output.status.success(),
        "CLI failed: {:?}\n{}",
        output.status,
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    let doc: serde_json::Value = serde_json::from_str(&stdout).expect("diagnostics JSON parses");
    let metrics = &doc["session"]["metrics"];
    Run {
        cost: metrics["cost_estimated"]
            .as_f64()
            .expect("cost_estimated is a number"),
        pricing_source: metrics["provenance"]["PricingSource"]
            .as_str()
            .expect("PricingSource is a string")
            .to_string(),
    }
}

fn unique_root(label: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!(
        "agenttrace-rm196-e2e-{}-{}-{}",
        label,
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock")
            .as_nanos()
    ));
    fs::create_dir_all(root.join("corpus")).expect("create corpus dir");
    fs::create_dir_all(root.join("cache/sess")).expect("create cache dir");
    root
}

#[test]
fn warm_cache_reprices_when_the_pricing_catalog_changes() {
    let root = unique_root("swap");
    let journal = root.join("corpus/journal.jsonl");
    fs::write(&journal, fixture_journal()).expect("write fixture journal");
    let override_path = root.join("override.json");
    fs::write(&override_path, override_file()).expect("write override file");
    let sessions_json = root.join("cache/sess/sessions.json");

    // Cold scan: priced from the bundled catalog.
    let cold = run_agenttrace(&root, None);
    assert!(
        cold.pricing_source.contains("bundled"),
        "cold run must be bundled-priced, got {}",
        cold.pricing_source
    );

    // Warm scan, same catalog: stable, and the cache journal is not
    // rewritten (pure hit, nothing dirty).
    let warm = run_agenttrace(&root, None);
    assert_eq!(warm.cost, cold.cost);
    assert_eq!(warm.pricing_source, cold.pricing_source);
    let journal_after_warm = fs::read(&sessions_json).expect("sessions.json exists after warm run");
    let warm_again = run_agenttrace(&root, None);
    assert_eq!(warm_again.cost, cold.cost);
    assert_eq!(
        fs::read(&sessions_json).expect("sessions.json readable"),
        journal_after_warm,
        "a pure cache hit must leave the journal bytes untouched"
    );

    // THE regression: override priced in, artifact untouched — the warm
    // entry must be re-priced, both the cost and the provenance label.
    let overridden = run_agenttrace(&root, Some(&override_path));
    assert_eq!(
        overridden.cost, 3.0,
        "1M input @ $1.0/M + 1M output @ $2.0/M must re-price the warm session"
    );
    assert_eq!(overridden.pricing_source, "user override");

    // Catalog restored (override removed), artifact still untouched: the
    // bundled price and label come back — truthfulness is bidirectional.
    let restored = run_agenttrace(&root, None);
    assert_eq!(restored.cost, cold.cost);
    assert!(
        restored.pricing_source.contains("bundled"),
        "removing the override must re-price back to bundled, got {}",
        restored.pricing_source
    );

    let _ = fs::remove_dir_all(root);
}

#[test]
fn warm_cache_reprices_when_the_override_file_content_changes() {
    // Same file path, new bytes: the pricing-catalog identity is a
    // content digest (`pricing::catalog_identity()`), so editing the
    // override re-prices warm sessions without any touch to the session
    // artifacts, the env shape, or the file's mtime/size.
    let root = unique_root("edit");
    let journal = root.join("corpus/journal.jsonl");
    fs::write(&journal, fixture_journal()).expect("write fixture journal");
    let override_path = root.join("override.json");
    // 40x rates first: 40.0 + 80.0 = 120.0.
    fs::write(
        &override_path,
        format!(
            "{{\"prices\":{{\"{MODEL}\":{{\"input\":40.0,\"output\":80.0,\"cw\":0.5,\"cr\":0.25}}}}}}"
        ),
    )
    .expect("write override v1");

    let first = run_agenttrace(&root, Some(&override_path));
    assert_eq!(first.cost, 120.0);
    assert_eq!(first.pricing_source, "user override");

    // Warm under v1, then edit rates down to 1x without touching anything
    // else. v1 keeps distinct two-digit rates (40.0/80.0, 120.0 total) so
    // the re-price is unambiguous against v2's 3.0; the identity hashes
    // the parsed catalog entries, so any content edit — even a
    // same-length one at preserved mtime — is detected.
    let warm_first = run_agenttrace(&root, Some(&override_path));
    assert_eq!(warm_first.cost, 120.0);
    fs::write(
        &override_path,
        format!(
            "{{\"prices\":{{\"{MODEL}\":{{\"input\":1.0,\"output\":2.0,\"cw\":0.5,\"cr\":0.25}}}}}}"
        ),
    )
    .expect("write override v2");
    let second = run_agenttrace(&root, Some(&override_path));
    assert_eq!(
        second.cost, 3.0,
        "an edited override must re-price warm sessions"
    );

    let _ = fs::remove_dir_all(root);
}

#[test]
fn warm_cache_preserves_the_flex_service_tier_on_hits() {
    // rm-610 + rm-613 end to end, on the codex flex shape: a flex turn
    // of 1M in / 1M out on gpt-5.6 costs $12 on the flex row (not $24
    // standard), the tier rides the session cache so a warm hit replays
    // the tier-priced cost AND provenance, and the overview audit's
    // "current rates" recompute stays tier-aware instead of claiming
    // the session costs double and flagging drift.
    let root = unique_root("flex");
    let journal = root.join("corpus/flex.jsonl");
    fs::write(&journal, fixture_codex_flex_journal()).expect("write codex flex journal");

    let cold = run_agenttrace(&root, None);
    assert_eq!(cold.cost, 12.0, "1M/1M on the gpt-5.6 flex row is $12");
    assert!(
        cold.pricing_source.contains("service_tier: flex"),
        "cold run must disclose the tier billed on, got {}",
        cold.pricing_source
    );

    let warm = run_agenttrace(&root, None);
    assert_eq!(warm.cost, 12.0);
    assert!(
        warm.pricing_source.contains("service_tier: flex"),
        "a warm cache hit must replay the tier-priced provenance (the tier
        rides the cache snapshot), got {}",
        warm.pricing_source
    );

    // The overview's cost audit agrees with itself: the "current"
    // estimate is the flex row, and no drift note fires.
    let output = Command::new(env!("CARGO_BIN_EXE_agenttrace"))
        .args(["-d", "corpus", "--overview", "-f", "json"])
        .current_dir(&root)
        .env("XDG_CACHE_HOME", root.join("cache"))
        .env("AGENTTRACE_SESSION_CACHE_DIR", root.join("cache/sess"))
        .env("HOME", &root)
        .env_remove("AGENTTRACE_PRICING_FILE")
        .output()
        .expect("run agenttrace --overview");
    assert!(output.status.success(), "overview failed");
    let doc: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("overview JSON parses");
    let audit = &doc["cost_audit"];
    assert_eq!(
        audit["stored_estimated_cost_usd"].as_f64(),
        Some(12.0),
        "stored total is the flex-priced cost"
    );
    assert_eq!(
        audit["current_estimated_cost_usd"].as_f64(),
        Some(12.0),
        "the current-rates recompute must bill the flex row too (was a
        false 24.0 with a drift note before the tier rode the metrics)"
    );
    let _ = fs::remove_dir_all(root);
}

fn fixture_codex_flex_journal() -> String {
    // A codex rollout whose turn_context discloses
    // thread_settings.service_tier = "flex" (the codeburn #1642 field
    // shape) and one token_count snapshot of exactly 1M in / 1M out.
    concat!(
        "{\"timestamp\":\"2026-10-01T09:00:00Z\",\"type\":\"session_meta\",\"payload\":{\"id\":\"s1\",\"cwd\":\"/tmp/x\",\"model\":\"gpt-5.6\"}}\n",
        "{\"timestamp\":\"2026-10-01T09:00:05Z\",\"type\":\"turn_context\",\"payload\":{\"model\":\"gpt-5.6\",\"cwd\":\"/tmp/x\",\"thread_settings\":{\"service_tier\":\"flex\"}}}\n",
        "{\"timestamp\":\"2026-10-01T09:00:10Z\",\"type\":\"event_msg\",\"payload\":{\"type\":\"token_count\",\"info\":{\"total_token_usage\":{\"input_tokens\":1000000,\"output_tokens\":1000000}}}}\n"
    )
    .to_string()
}

fn fixture_journal_with(model: &str) -> String {
    format!(
        concat!(
            "{{\"id\":\"h1\",\"timestamp\":\"2026-10-05T10:00:00.000Z\",\"type\":\"session\",\"version\":3,\"cwd\":\"/tmp/warm\"}}\n",
            "{{\"id\":\"e1\",\"parentId\":\"h1\",\"timestamp\":\"2026-10-05T10:00:01.000Z\",\"type\":\"message\",\"message\":{{\"role\":\"user\",\"content\":[{{\"type\":\"text\",\"text\":\"go\"}}]}}}}\n",
            "{{\"id\":\"e2\",\"parentId\":\"e1\",\"timestamp\":\"2026-10-05T10:00:02.000Z\",\"type\":\"message\",\"message\":{{\"role\":\"assistant\",\"content\":[{{\"type\":\"text\",\"text\":\"ok\"}}],\"model\":\"{model}\",\"usage\":{{\"input\":1000000,\"output\":1000000,\"cacheRead\":0,\"cacheWrite\":0,\"totalTokens\":2000000}}}}}}\n"
        ),
        model = model
    )
}

#[test]
fn catalog_keyed_alias_overrides_reprice_the_raw_row() {
    // Review F1 (run 7bb862e4 cycle 3, independent_review fbc04223):
    // `claude-sonnet-4-5` keys a catalog row, so the rm-1053 raw-first
    // lookup ignored a user AGENTTRACE_PRICING_FILE alias on it —
    // billing the untouched base row ($18.00 for 1M/1M) while the CLI
    // header disclosed "+ user overrides applied (0 model(s),
    // 1 alias(es))". rm-511: an alias supplies identity — it must win.
    // Both redirect targets are themselves catalog rows: the plain
    // opus row (5.0/25.0 -> $30.00) and the us regional twin
    // (3.3/16.5 -> $19.80).
    let root = unique_root("alias-f1");
    let journal = root.join("corpus/journal.jsonl");
    fs::write(&journal, fixture_journal_with("claude-sonnet-4-5")).expect("write fixture");
    let opus_redirect = root.join("alias-opus.json");
    fs::write(
        &opus_redirect,
        "{\"aliases\":{\"claude-sonnet-4-5\":\"claude-opus-4-6\"}}",
    )
    .expect("write opus redirect");
    let twin_redirect = root.join("alias-twin.json");
    fs::write(
        &twin_redirect,
        "{\"aliases\":{\"claude-sonnet-4-5\":\"us.anthropic.claude-sonnet-4-5-20250929-v1:0\"}}",
    )
    .expect("write twin redirect");

    let baseline = run_agenttrace(&root, None);
    assert_eq!(baseline.cost, 18.0, "1M/1M on the base sonnet row is $18");
    let opus = run_agenttrace(&root, Some(&opus_redirect));
    assert_eq!(
        opus.cost, 30.0,
        "alias -> opus must reprice onto the opus row (was silently $18.00 — review F1)"
    );
    assert!(
        opus.pricing_source.contains("alias") || opus.pricing_source.contains("override"),
        "the alias override must stay disclosed in the provenance, got {}",
        opus.pricing_source
    );
    let twin = run_agenttrace(&root, Some(&twin_redirect));
    assert_eq!(
        twin.cost, 19.8,
        "alias -> us twin must reprice onto the twin row (was silently $18.00 — review F1)"
    );
    // rm-1053 stays intact: the raw twin id, with no alias, bills its
    // own row.
    let twin_root = unique_root("alias-f1-twin");
    fs::write(
        twin_root.join("corpus/journal.jsonl"),
        fixture_journal_with("us.anthropic.claude-sonnet-4-5-20250929-v1:0"),
    )
    .expect("write twin fixture");
    let twin_raw = run_agenttrace(&twin_root, None);
    assert_eq!(twin_raw.cost, 19.8, "the raw twin id keeps its own row");
    let _ = fs::remove_dir_all(root);
    let _ = fs::remove_dir_all(twin_root);
}
