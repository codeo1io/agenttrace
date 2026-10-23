/// rm-576: the SVG usage card must be a deterministic, offline, markup-safe
/// artifact. Contract (selection cc8807d7 / stewardship 202d7e65):
/// - byte-deterministic for a fixed corpus + range + theme (goldens pinned;
///   regenerate deliberately with UPDATE_CARD_GOLDEN=1);
/// - every session-derived string is control-byte sanitized (XML 1.0 would
///   reject raw control bytes) and entity-escaped (& < > " '), so a model or
///   project name that looks like markup renders inert — mirrors the
///   rm-403 markdown contract;
/// - `auto` theme embeds a `prefers-color-scheme` override and still carries
///   a complete light palette as presentation attributes (graceful
///   degradation when a sanitizer strips <style>);
/// - the daily-spend sparkline is anchored to the corpus's own clock, never
///   `Utc::now()`, so the card cannot drift between renders;
/// - an empty corpus renders a valid card instead of panicking.
///
/// rm-1954 completion (run 5cf06bab cycle 9, batch "host-state truth
/// hardening"): every card render embeds a pricing disclosure whose
/// census and cost rows resolve through the public catalog lane — the
/// lane that PREFERS a machine pricing cache over the bundled
/// snapshot and initializes once per process — so every rendering
/// test runs inside `support::with_isolated_pricing_env`
/// (tests/support/mod.rs), and the standing poison-proof fixture pins
/// both directions without relying on ambient host state.
mod support;

use agenttrace_core::{
    compute_overview, demo_sessions, lookup_price, report_overview_svg,
    report_overview_svg_with_context, SvgCardTheme,
};

const GOLDEN_DIR: &str = "tests/fixtures/svg-card";

fn card_of(theme: SvgCardTheme) -> String {
    let sessions = demo_sessions().expect("demo sessions parse");
    let overview = compute_overview(&sessions);
    report_overview_svg(&overview, &sessions, theme)
}

#[test]
fn golden_bytes_are_pinned_for_the_demo_corpus() {
    support::with_isolated_pricing_env(golden_bytes_are_pinned_for_the_demo_corpus_body);
}

fn golden_bytes_are_pinned_for_the_demo_corpus_body() {
    for (name, theme) in [
        ("demo-auto.svg", SvgCardTheme::Auto),
        ("demo-dark.svg", SvgCardTheme::Dark),
        ("demo-light.svg", SvgCardTheme::Light),
    ] {
        let card = card_of(theme);
        let golden_path = format!("{GOLDEN_DIR}/{name}");
        if std::env::var("UPDATE_CARD_GOLDEN").is_ok() {
            std::fs::create_dir_all(GOLDEN_DIR).expect("create golden dir");
            std::fs::write(&golden_path, &card).expect("write golden");
        }
        let golden = std::fs::read_to_string(&golden_path).unwrap_or_else(|err| {
            panic!("golden {golden_path} missing (regen with UPDATE_CARD_GOLDEN=1): {err}")
        });
        assert_eq!(golden, card, "card bytes drifted from golden {golden_path}");
    }
}

#[test]
fn rendering_twice_from_a_fresh_aggregation_is_byte_identical() {
    support::with_isolated_pricing_env(
        rendering_twice_from_a_fresh_aggregation_is_byte_identical_body,
    );
}

fn rendering_twice_from_a_fresh_aggregation_is_byte_identical_body() {
    let first = card_of(SvgCardTheme::Auto);
    let second = card_of(SvgCardTheme::Auto);
    assert_eq!(first, second, "card output is not byte-deterministic");
    assert!(first.starts_with("<?xml version=\"1.0\" encoding=\"UTF-8\"?>"));
    assert!(first.ends_with("</svg>"));
}

#[test]
fn hostile_model_and_project_names_render_inert() {
    support::with_isolated_pricing_env(hostile_model_and_project_names_render_inert_body);
}

fn hostile_model_and_project_names_render_inert_body() {
    let mut sessions = demo_sessions().expect("demo sessions parse");
    sessions[0].metrics.model_used = "\"><script>alert(1)</script>".to_string();
    sessions[1].metrics.model_used = "<img src=x onerror=alert(2)>".to_string();
    sessions[0].name = "sess-<script>alert(3)</script>".to_string();
    sessions[0].cwd = "/tmp/<iframe src=evil>".to_string();
    let overview = compute_overview(&sessions);
    let card = report_overview_svg(&overview, &sessions, SvgCardTheme::Dark);

    assert!(
        !card.contains("<script>alert"),
        "raw <script> survived into SVG"
    );
    assert!(!card.contains("<img src=x"), "raw <img> survived into SVG");
    assert!(!card.contains("<iframe"), "raw <iframe> survived into SVG");
    assert!(
        // 27-char payload → 21-char label truncation happens first, so
        // the escaped form is the truncated prefix (still fully inert).
        card.contains("&lt;script&gt;alert(1)"),
        "escaped payload not present — escaping pipeline changed"
    );
}

#[test]
fn control_bytes_never_reach_the_document_raw() {
    support::with_isolated_pricing_env(control_bytes_never_reach_the_document_raw_body);
}

fn control_bytes_never_reach_the_document_raw_body() {
    let mut sessions = demo_sessions().expect("demo sessions parse");
    sessions[0].metrics.model_used = "ctrl\x01\x02model".to_string();
    let overview = compute_overview(&sessions);
    let card = report_overview_svg(&overview, &sessions, SvgCardTheme::Light);
    assert!(
        !card.contains('\x01'),
        "raw control byte emitted (illegal XML 1.0)"
    );
    assert!(
        card.contains('\u{fffd}'),
        "control bytes should map to U+FFFD"
    );
}

#[test]
fn auto_theme_embeds_media_query_and_light_fallback() {
    support::with_isolated_pricing_env(auto_theme_embeds_media_query_and_light_fallback_body);
}

fn auto_theme_embeds_media_query_and_light_fallback_body() {
    let auto = card_of(SvgCardTheme::Auto);
    assert!(
        auto.contains("@media (prefers-color-scheme: dark)"),
        "auto theme lost its dark override block"
    );
    assert!(
        auto.contains("class=\"card-fg\""),
        "auto theme lost its class hooks"
    );
    // Graceful degradation: the light palette ships as presentation
    // attributes too, so stripping <style> leaves a correct light card.
    assert!(
        auto.contains("fill=\"#0f172a\""),
        "auto theme lost inline light fills"
    );

    let dark = card_of(SvgCardTheme::Dark);
    assert!(
        !dark.contains("@media"),
        "dark theme must be a fixed palette"
    );
    assert!(
        dark.contains("fill=\"#0b1220\""),
        "dark theme lost its background"
    );
}

#[test]
fn themes_render_distinct_documents() {
    support::with_isolated_pricing_env(themes_render_distinct_documents_body);
}

fn themes_render_distinct_documents_body() {
    let light = card_of(SvgCardTheme::Light);
    let dark = card_of(SvgCardTheme::Dark);
    let auto = card_of(SvgCardTheme::Auto);
    assert_ne!(light, dark);
    assert_ne!(light, auto);
    assert_ne!(dark, auto);
}

#[test]
fn empty_corpus_renders_a_valid_card() {
    support::with_isolated_pricing_env(empty_corpus_renders_a_valid_card_body);
}

fn empty_corpus_renders_a_valid_card_body() {
    let overview = compute_overview(&[]);
    let card = report_overview_svg(&overview, &[], SvgCardTheme::Auto);
    assert!(card.starts_with("<?xml version=\"1.0\""));
    assert!(card.ends_with("</svg>"));
    assert!(
        card.contains("TOTAL COST"),
        "stat row missing on empty corpus"
    );
    assert!(
        card.contains("$0.00"),
        "zero-cost headline missing on empty corpus"
    );
}

#[test]
fn sparkline_is_anchored_to_the_corpus_clock_not_wall_clock() {
    support::with_isolated_pricing_env(
        sparkline_is_anchored_to_the_corpus_clock_not_wall_clock_body,
    );
}

fn sparkline_is_anchored_to_the_corpus_clock_not_wall_clock_body() {
    let mut sessions = demo_sessions().expect("demo sessions parse");
    let anchor = chrono::DateTime::parse_from_rfc3339("2026-01-20T12:00:00Z")
        .expect("fixed anchor")
        .with_timezone(&chrono::Utc);
    for (i, session) in sessions.iter_mut().enumerate() {
        let stamp = anchor - chrono::Duration::days(i as i64);
        session.metrics.timestamps = vec![stamp];
        // rm-911: the card's daily series reads the cache-surviving
        // session bounds FIRST (session_end == last event stamp on
        // every lane that has one), so a corpus-clock fixture must set
        // the bounds too — timestamps alone no longer move the axis.
        session.metrics.session_start = stamp.to_rfc3339();
        session.metrics.session_end = stamp.to_rfc3339();
    }
    let overview = compute_overview(&sessions);
    let card = report_overview_svg(&overview, &sessions, SvgCardTheme::Light);
    assert!(
        card.contains("2026-01-20"),
        "sparkline must end at the corpus's latest day, got: {}",
        card.lines()
            .find(|l| l.contains("%Y") || l.contains("2026-"))
            .unwrap_or("<no date labels>")
    );
    // The demo corpus's fixed timestamps are far from today; if the card
    // were anchored to `Utc::now()` today's date would leak in.
    let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
    assert!(
        !card.contains(&today),
        "wall-clock date leaked into a corpus-anchored card"
    );
}

#[test]
fn with_context_carries_the_disclosure_footer() {
    support::with_isolated_pricing_env(with_context_carries_the_disclosure_footer_body);
}

fn with_context_carries_the_disclosure_footer_body() {
    let sessions = demo_sessions().expect("demo sessions parse");
    let overview = compute_overview(&sessions);
    let health = agenttrace_core::data_health(&sessions, sessions.len(), 0);
    let card = report_overview_svg_with_context(
        &overview,
        &sessions,
        &health,
        agenttrace_core::TimeRange::All,
        true,
        SvgCardTheme::Auto,
    );
    assert!(card.contains("window "), "scope footer missing");
    assert!(card.contains("pricing "), "pricing disclosure missing");
    assert!(
        card.contains("data confidence "),
        "confidence disclosure missing"
    );
    assert!(card.ends_with("</svg>"));
}

#[test]
fn theme_parser_rejects_unknown_values() {
    assert_eq!(SvgCardTheme::parse("auto"), Some(SvgCardTheme::Auto));
    assert_eq!(SvgCardTheme::parse("dark"), Some(SvgCardTheme::Dark));
    assert_eq!(SvgCardTheme::parse("light"), Some(SvgCardTheme::Light));
    assert_eq!(SvgCardTheme::parse("solarized"), None);
    assert_eq!(
        SvgCardTheme::parse("AUTO"),
        None,
        "case-sensitive like --lang"
    );
}

#[test]
fn daily_series_survives_the_warm_cache_replay_shape() {
    support::with_isolated_pricing_env(daily_series_survives_the_warm_cache_replay_shape_body);
}

fn daily_series_survives_the_warm_cache_replay_shape_body() {
    // rm-911 (assess F1): `SessionMetrics::timestamps` is
    // #[serde(skip)], so the warm session cache replays sessions with
    // an EMPTY timestamp list while `session_start`/`session_end`
    // survive the round trip (session_cache's GoMetrics carries them
    // as strings). The card's daily-spend series used to read only
    // `timestamps`, so warm replays rendered the flat 1970-01-01
    // window (live PoC: /tmp/at-assess/c6-warm.svg). The series now
    // derives from the surviving bounds, so the replay-shape corpus
    // must render the SAME card as the full-shape corpus — byte for
    // byte, and never the 1970 flat window.
    let mut warm_shape = demo_sessions().expect("demo sessions parse");
    for session in warm_shape.iter_mut() {
        assert!(
            !session.metrics.session_end.is_empty(),
            "fixture sanity: demo sessions carry a session_end bound"
        );
        session.metrics.timestamps.clear();
    }
    let cold = card_of(SvgCardTheme::Light);
    let warm = {
        let overview = compute_overview(&warm_shape);
        report_overview_svg(&overview, &warm_shape, SvgCardTheme::Light)
    };
    assert!(
        !warm.contains("1970-01-01"),
        "the warm-cache replay shape must not collapse the axis to the epoch"
    );
    assert_eq!(
        cold, warm,
        "the card must not depend on cache-dropped fields: same corpus, \
         timestamps present vs replay-shape, byte-identical render"
    );
}

// rm-1954 completion: the standing poison-proof fixture. `card_of`
// renders the pricing disclosure from the serving catalog, so a
// mutated machine pricing cache at the default user-cache location
// re-points demo-corpus costs (the observed cycle-9 host red: svg
// golden `2.304` vs `7.395` total cost under the 2026-10-21 machine
// cache). The fixture seeds a mutated bundled snapshot — the demo
// corpus's primary model `claude-sonnet-4-5` with a tripled input
// rate — under a scratch HOME and proves BOTH legs: guarded the card
// prices from the bundled row, unguarded the poison still wins.
#[test]
fn mutated_machine_cache_does_not_flip_the_card_pricing() {
    support::assert_poison_proof_standing("svg_card_poison_probe", &mutated_machine_cache_bytes());
}

fn mutated_machine_cache_bytes() -> String {
    let mut snapshot: serde_json::Value =
        serde_json::from_str(include_str!("../src/pricing_snapshot.json")).unwrap();
    snapshot["claude-sonnet-4-5"]["input_cost_per_token"] = serde_json::json!(9.5e-6);
    snapshot.to_string()
}

// Spawned by the fixture above; runs only under its sentinel env —
// deliberately UNGUARDED: the fixture isolates via spawned env, not
// the in-process guard. Asserts the representative card-pricing
// truth: the demo corpus's primary model prices from the bundled row.
#[test]
fn svg_card_poison_probe() {
    if std::env::var_os("AGENTTRACE_TEST_PRICING_PROBE").is_none() {
        return;
    }
    let bundled: serde_json::Value =
        serde_json::from_str(include_str!("../src/pricing_snapshot.json")).unwrap();
    let row = &bundled["claude-sonnet-4-5"];
    let p = lookup_price("claude-sonnet-4-5");
    // `Price` is USD per MILLION tokens; the snapshot literal is USD
    // per token — the same *1e6 scaling the regional suite applies.
    assert_eq!(
        p.input,
        row["input_cost_per_token"].as_f64().unwrap() * 1e6,
        "card pricing must serve the bundled row"
    );
    assert_eq!(
        p.output,
        row["output_cost_per_token"].as_f64().unwrap() * 1e6
    );
}
