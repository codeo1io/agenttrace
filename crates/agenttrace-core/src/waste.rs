use crate::{pricing, round4, Metrics, Session, VERSION};

#[derive(Debug, Clone)]
pub struct CacheEfficiency {
    cache_read_tokens: i64,
    total_input_tokens: i64,
    hit_rate: f64,
    wasted_cost: f64,
    rating: &'static str,
    suggestion: &'static str,
}

#[derive(Debug, Clone)]
pub struct ToolBloatItem {
    tool_name: String,
    call_count: usize,
    total_cost: f64,
    is_redundant: bool,
}

#[derive(Debug, Clone)]
pub struct ToolBloatAnalysis {
    tools_per_turn: f64,
    bloat_score: i32,
    bloat_level: &'static str,
    top_bloat: Vec<ToolBloatItem>,
}

#[derive(Debug, Clone)]
pub struct StuckPattern {
    description: String,
    severity: &'static str,
}

#[derive(Debug, Clone)]
pub struct WasteReport {
    cache: CacheEfficiency,
    bloat: ToolBloatAnalysis,
    stuck: Vec<StuckPattern>,
    waste_score: i32,
    waste_level: &'static str,
    total_wasted: f64,
    summary: String,
    top_actions: Vec<String>,
}

pub fn compute_waste_report(session: &Session) -> WasteReport {
    let cache = analyze_cache_efficiency(&session.metrics);
    let bloat = analyze_tool_bloat(&session.metrics);
    let stuck = detect_stuck_from_metrics(&session.metrics);
    let loop_cost = 0.0;
    let loop_percent = 0.0;
    let mut total_wasted = cache.wasted_cost + loop_cost;
    if bloat.bloat_score > 50 {
        total_wasted += session.metrics.cost_estimated * 0.05;
    }

    let mut score = match cache.rating {
        "none" => 20.0,
        "poor" => 15.0,
        "good" => 5.0,
        _ => 0.0,
    };
    score += bloat.bloat_score as f64 * 0.25;
    score += loop_percent * 0.6;
    if score > 30.0 {
        score = 30.0;
    }
    let mut stuck_score = stuck.len() as f64 * 7.0;
    for item in &stuck {
        if item.severity == "critical" {
            stuck_score += 5.0;
        }
    }
    if stuck_score > 20.0 {
        stuck_score = 20.0;
    }
    score += stuck_score;
    if session.metrics.tokens_cache_r > 0
        && session.metrics.tokens_input > 0
        && session.metrics.tokens_cache_r as f64 / (session.metrics.tokens_input as f64) < 0.3
    {
        score += 6.0;
    }
    let waste_score = (score as i32).clamp(0, 100);
    let waste_level = match waste_score {
        70.. => "red",
        40..=69 => "orange",
        15..=39 => "yellow",
        _ => "green",
    };
    let summary = match waste_level {
        "green" => "efficient session - no significant waste".to_string(),
        "yellow" => format!(
            "minor waste - cache {:.0}% hit, room for optimization",
            cache.hit_rate
        ),
        "orange" => format!(
            "wasting ${:.2}: loops {:.0}%, tools {:.1}/turn",
            total_wasted, loop_percent, bloat.tools_per_turn
        ),
        "red" => format!(
            "severe waste ${:.2}: loops {:.0}%, {} stuck, no cache",
            total_wasted,
            loop_percent,
            stuck.len()
        ),
        _ => String::new(),
    };

    let mut top_actions = Vec::new();
    if cache.rating == "none" || cache.rating == "poor" {
        top_actions.push(cache.suggestion.to_string());
    }
    if bloat.bloat_level == "severe" || bloat.bloat_level == "high" {
        if let Some(top) = bloat.top_bloat.first() {
            top_actions.push(format!(
                "top tool {:?} called {}x - reduce or batch",
                top.tool_name, top.call_count
            ));
        } else {
            top_actions.push(bloat_suggestion(bloat.bloat_level).to_string());
        }
    }
    if loop_percent > 20.0 {
        top_actions.push(format!(
            "loop waste ${:.2} ({:.0}%) - add max retries limit",
            loop_cost, loop_percent
        ));
    }
    if top_actions.is_empty() {
        top_actions.push("session running optimally".to_string());
    }

    WasteReport {
        cache,
        bloat,
        stuck,
        waste_score,
        waste_level,
        total_wasted,
        summary,
        top_actions,
    }
}

pub fn render_waste_report(session: &Session) -> String {
    waste_report_text(&compute_waste_report(session))
}

fn analyze_cache_efficiency(metrics: &Metrics) -> CacheEfficiency {
    let hit_rate = if metrics.tokens_input > 0 {
        metrics.tokens_cache_r as f64 / metrics.tokens_input as f64 * 100.0
    } else {
        0.0
    };
    let wasted_tokens = (metrics.tokens_input - metrics.tokens_cache_r).max(0);
    let price = pricing::lookup_price(&metrics.model_used);
    let wasted_cost = round4(wasted_tokens as f64 / 1e6 * price.input);
    let (rating, suggestion) = if hit_rate >= 80.0 {
        (
            "excellent",
            "cache utilization excellent - keep current prompt structure",
        )
    } else if hit_rate >= 40.0 {
        (
            "good",
            "moderate cache hit - place static system instructions at prompt prefix",
        )
    } else if metrics.tokens_cache_w > 0 {
        (
            "poor",
            "low cache hit rate - enable prompt caching with static prefix content",
        )
    } else {
        (
            "none",
            "caching not enabled - enable Anthropic prompt caching to save up to 90% on input cost",
        )
    };
    CacheEfficiency {
        cache_read_tokens: metrics.tokens_cache_r,
        total_input_tokens: metrics.tokens_input,
        hit_rate,
        wasted_cost,
        rating,
        suggestion,
    }
}

fn analyze_tool_bloat(metrics: &Metrics) -> ToolBloatAnalysis {
    let tools_per_turn = if metrics.assistant_turns > 0 {
        metrics.tool_calls_total as f64 / metrics.assistant_turns as f64
    } else {
        0.0
    };
    let avg_cost_per_turn = if metrics.assistant_turns > 0 && metrics.cost_estimated > 0.0 {
        metrics.cost_estimated / metrics.assistant_turns as f64
    } else {
        0.0
    };
    let (bloat_score, bloat_level) = if tools_per_turn > 5.0 {
        (90, "severe")
    } else if tools_per_turn > 3.0 {
        (65, "high")
    } else if tools_per_turn > 1.5 {
        (35, "medium")
    } else {
        (10, "low")
    };
    let mut tools = metrics.tool_usage.iter().collect::<Vec<_>>();
    tools.sort_by(|a, b| b.1.cmp(a.1));
    let top_bloat = tools
        .into_iter()
        .take(5)
        .map(|(tool_name, call_count)| ToolBloatItem {
            tool_name: tool_name.clone(),
            call_count: *call_count,
            total_cost: avg_cost_per_turn * *call_count as f64,
            is_redundant: *call_count > metrics.assistant_turns && metrics.assistant_turns > 0,
        })
        .collect();
    ToolBloatAnalysis {
        tools_per_turn,
        bloat_score,
        bloat_level,
        top_bloat,
    }
}

fn detect_stuck_from_metrics(metrics: &Metrics) -> Vec<StuckPattern> {
    let long_gaps = metrics.gaps_sec.iter().filter(|gap| **gap > 120.0).count();
    if long_gaps >= 3 {
        vec![StuckPattern {
            description: format!("{long_gaps} gaps >120s - agent appears stuck"),
            severity: "critical",
        }]
    } else {
        Vec::new()
    }
}

fn waste_report_text(report: &WasteReport) -> String {
    let sep = "━".repeat(60);
    let mut out = String::new();
    out.push_str(&sep);
    out.push('\n');
    out.push_str(&format!("  AGENTTRACE v{} - Waste Analysis\n", VERSION));
    out.push_str(&sep);
    out.push('\n');
    out.push('\n');
    out.push_str(&format!(
        "  Score: {}/100 ({} {})\n",
        report.waste_score,
        level_emoji(report.waste_level),
        waste_level_label(report.waste_level)
    ));
    out.push_str(&format!("  Wasted: ${:.4}\n", report.total_wasted));
    out.push_str(&format!("  {}\n", report.summary));
    out.push('\n');
    out.push_str("  -- Cache --\n");
    out.push_str(&format!(
        "  {} ({:.0}% hit, {} read / {} input)\n",
        cache_rating_label(report.cache.rating),
        report.cache.hit_rate,
        report.cache.cache_read_tokens,
        report.cache.total_input_tokens
    ));
    if report.cache.wasted_cost > 0.0 {
        out.push_str(&format!(
            "  Cache waste: ${:.4}\n",
            report.cache.wasted_cost
        ));
    }
    out.push_str(&format!("  Suggestion: {}\n", report.cache.suggestion));
    out.push('\n');
    out.push_str("  -- Tool Bloat --\n");
    out.push_str(&format!(
        "  {} ({:.1} tools/turn)\n",
        bloat_level_label(report.bloat.bloat_level),
        report.bloat.tools_per_turn
    ));
    for item in &report.bloat.top_bloat {
        let redundant = if item.is_redundant { " *redundant" } else { "" };
        out.push_str(&format!(
            "    {:<25} {:>3}x ${:.3}{}\n",
            item.tool_name, item.call_count, item.total_cost, redundant
        ));
    }
    out.push('\n');
    out.push_str("  -- Stuck --\n");
    if report.stuck.is_empty() {
        out.push_str("  none\n");
    } else {
        for stuck in &report.stuck {
            out.push_str(&format!("  [{}] {}\n", stuck.severity, stuck.description));
        }
    }
    out.push('\n');
    out.push_str("  -- Actions --\n");
    for (index, action) in report.top_actions.iter().enumerate() {
        out.push_str(&format!("  {}. {}\n", index + 1, action));
    }
    out.push('\n');
    out.push_str(&sep);
    out.push('\n');
    out
}

fn cache_rating_label(rating: &str) -> &'static str {
    match rating {
        "excellent" => "excellent",
        "good" => "good",
        "poor" => "poor",
        _ => "none",
    }
}

fn bloat_level_label(level: &str) -> &'static str {
    match level {
        "severe" => "severe",
        "high" => "high",
        "medium" => "medium",
        _ => "low",
    }
}

fn bloat_suggestion(level: &str) -> &'static str {
    match level {
        "severe" => "severe tool bloat: limit max tool calls per turn or split into smaller tasks",
        "high" => "too many tool calls: check if simple tasks use over-complex agent orchestration",
        "medium" => "moderate tool usage: watch for unnecessary tool call patterns",
        _ => "tool usage is lean",
    }
}

fn waste_level_label(level: &str) -> &'static str {
    match level {
        "red" => "SEVERE",
        "orange" => "HIGH",
        "yellow" => "MODERATE",
        _ => "LOW",
    }
}

fn level_emoji(level: &str) -> &'static str {
    match level {
        "red" => "🔴",
        "orange" => "🟠",
        "yellow" => "🟡",
        _ => "🟢",
    }
}
