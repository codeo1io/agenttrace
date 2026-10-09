//! rm-845 end-to-end: subscription plan-tier catalog entries price at
//! their plan rates and are DISCLOSED as plan scopes, so a 0-cost
//! coding-plan session never reads as a fallback-priced model.
//!
//! The bundled snapshot carries a `_plan_scope` section (models.dev
//! authority; ccusage#1832 provider-id grammar) with
//! `zai-coding-plan/glm-5.3-highspeed` at 0/0 (subscription included)
//! and `zai/glm-4.7-flashx` at direct API rates.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

struct Run {
    cost: f64,
    pricing_source: String,
    disclosures: String,
}

fn fixture_journal(model: &str) -> String {
    format!(
        concat!(
            "{{\"id\":\"h1\",\"timestamp\":\"2026-10-05T10:00:00.000Z\",\"type\":\"session\",\"version\":3,\"cwd\":\"/tmp/plan\"}}\n",
            "{{\"id\":\"e1\",\"parentId\":\"h1\",\"timestamp\":\"2026-10-05T10:00:01.000Z\",\"type\":\"message\",\"message\":{{\"role\":\"user\",\"content\":[{{\"type\":\"text\",\"text\":\"go\"}}]}}}}\n",
            "{{\"id\":\"e2\",\"parentId\":\"e1\",\"timestamp\":\"2026-10-05T10:00:02.000Z\",\"type\":\"message\",\"message\":{{\"role\":\"assistant\",\"content\":[{{\"type\":\"text\",\"text\":\"ok\"}}],\"model\":\"{model}\",\"usage\":{{\"input\":1000000,\"output\":1000000,\"cacheRead\":0,\"cacheWrite\":0,\"totalTokens\":2000000}}}}}}\n"
        ),
        model = model
    )
}

fn run_agenttrace(root: &Path) -> Run {
    let mut command = Command::new(env!("CARGO_BIN_EXE_agenttrace"));
    command
        .args(["-d", "corpus", "--diagnostics", "-f", "json"])
        .current_dir(root)
        .env("XDG_CACHE_HOME", root.join("cache"))
        .env("AGENTTRACE_SESSION_CACHE_DIR", root.join("cache/sess"))
        .env("HOME", root);
    command.env_remove("AGENTTRACE_PRICING_FILE");
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
        disclosures: serde_json::to_string(&metrics["disclosure_counters"])
            .expect("disclosure counters serialize"),
    }
}

fn unique_root(label: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!(
        "agenttrace-rm845-e2e-{}-{}-{}",
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
fn plan_tier_session_prices_at_plan_rates_and_discloses_the_scope() {
    let root = unique_root("plan");
    fs::write(
        root.join("corpus/journal.jsonl"),
        fixture_journal("zai-coding-plan/glm-5.3-highspeed"),
    )
    .expect("write fixture journal");

    let run = run_agenttrace(&root);
    assert!(
        run.pricing_source.contains("bundled"),
        "plan-tier entry must resolve from the bundled catalog, got {}",
        run.pricing_source
    );
    assert_eq!(
        run.cost, 0.0,
        "subscription-included plan tier prices at exactly 0 (not fallback)"
    );
    assert!(
        run.disclosures.contains("plan_tier_pricing"),
        "the plan scope must be disclosed in data_health, got {}",
        run.disclosures
    );
    fs::remove_dir_all(&root).ok();
}

#[test]
fn plan_tier_direct_rate_entry_prices_at_models_dev_rates() {
    let root = unique_root("direct");
    fs::write(
        root.join("corpus/journal.jsonl"),
        fixture_journal("zai/glm-4.7-flashx"),
    )
    .expect("write fixture journal");

    let run = run_agenttrace(&root);
    assert_eq!(
        run.cost, 0.47,
        "1M input @ $0.07/M + 1M output @ $0.40/M = $0.47 (models.dev direct rate)"
    );
    assert!(
        run.disclosures.contains("plan_tier_pricing"),
        "direct-rate plan entry is still a plan scope"
    );
    fs::remove_dir_all(&root).ok();
}

#[test]
fn ccusage_grammar_arrivals_price_at_plan_rates_not_fallback() {
    // ccusage#1832 provider-id grammar: real journals arrive with the
    // plan tier as a provider prefix on the model id (`builtin:zai-*`,
    // `account:zai-*`). The pricing normalizer keeps the final path
    // segment, so these arrivals resolve onto the SAME bundled plan
    // entry — priced at plan rates from the bundled catalog, never the
    // built-in fallback.
    for arrival in [
        "builtin:zai-coding-plan/glm-5.3-highspeed",
        "account:zai-coding-plan/glm-5.3-highspeed",
        "z.ai/glm-5.3-highspeed",
    ] {
        let root = unique_root("grammar");
        fs::write(root.join("corpus/journal.jsonl"), fixture_journal(arrival))
            .expect("write fixture journal");

        let run = run_agenttrace(&root);
        assert!(
            run.pricing_source.contains("bundled"),
            "{arrival}: plan-tier arrival must resolve from the bundled catalog, got {}",
            run.pricing_source
        );
        assert!(
            !run.pricing_source.contains("fallback"),
            "{arrival}: plan-tier arrival must not read as fallback-priced, got {}",
            run.pricing_source
        );
        assert_eq!(
            run.cost, 0.0,
            "{arrival}: subscription-included tier prices at exactly 0"
        );
        assert!(
            run.disclosures.contains("plan_tier_pricing"),
            "{arrival}: plan scope disclosed, got {}",
            run.disclosures
        );
        fs::remove_dir_all(&root).ok();
    }
}

#[test]
fn unknown_model_discloses_fallback_not_plan_tier() {
    let root = unique_root("ctrl");
    fs::write(
        root.join("corpus/journal.jsonl"),
        fixture_journal("totally-unknown-model-x"),
    )
    .expect("write fixture journal");

    let run = run_agenttrace(&root);
    assert!(
        !run.disclosures.contains("plan_tier_pricing"),
        "unknown model must NOT claim a plan scope: {}",
        run.disclosures
    );
    fs::remove_dir_all(&root).ok();
}
