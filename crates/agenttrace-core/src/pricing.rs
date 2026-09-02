use crate::round4;
use anyhow::{anyhow, Context};
use serde::Deserialize;
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;
use std::sync::OnceLock;
use std::time::{Duration, SystemTime};

const PRICING_URL: &str =
    "https://raw.githubusercontent.com/BerriAI/litellm/main/model_prices_and_context_window.json";
/// Trimmed LiteLLM chat-model pricing snapshot, vendored so agenttrace is
/// fully offline by default. Regenerate with `scripts/pricing/update-snapshot.sh`
/// and keep `PRICING_SNAPSHOT_DATE` in sync with the date it prints.
const PRICING_SNAPSHOT_JSON: &str = include_str!("pricing_snapshot.json");
const PRICING_SNAPSHOT_DATE: &str = "2026-09-02";
const CACHE_MAX_AGE: Duration = Duration::from_secs(24 * 60 * 60);
static PRICING_CATALOG: OnceLock<PricingCatalog> = OnceLock::new();
static PRICING_OVERRIDE_MODELS: OnceLock<BTreeSet<String>> = OnceLock::new();

#[derive(Debug, Clone, Copy, Default, serde::Serialize, Deserialize)]
pub struct Price {
    pub input: f64,
    pub output: f64,
    pub cw: f64,
    pub cr: f64,
}

#[derive(Debug, Clone)]
pub struct PricingCatalog {
    pub entries: BTreeMap<String, Price>,
    pub aliases: BTreeMap<String, String>,
    pub source: String,
}

#[derive(Debug, Deserialize)]
struct LiteLlmModel {
    #[serde(default, rename = "input_cost_per_token")]
    input_cost: f64,
    #[serde(default, rename = "output_cost_per_token")]
    output_cost: f64,
    #[serde(default, rename = "cache_creation_input_token_cost")]
    cache_write_cost: f64,
    #[serde(default, rename = "cache_read_input_token_cost")]
    cache_read_cost: f64,
    #[serde(default)]
    mode: String,
    #[serde(default, rename = "litellm_provider")]
    provider: String,
}

pub fn lookup_price(model: &str) -> Price {
    let catalog = pricing_catalog();
    let model = resolve_alias(model, &catalog.aliases);
    lookup_price_in(&model, &catalog.entries)
}

pub fn has_specific_price(model: &str) -> bool {
    if matches!(model.trim(), "" | "default" | "unknown") {
        return false;
    }
    let catalog = pricing_catalog();
    let model = resolve_alias(model, &catalog.aliases);
    match_variants(&model)
        .into_iter()
        .any(|variant| catalog.entries.contains_key(&variant))
}

pub fn list_pricing() -> BTreeMap<String, Price> {
    let mut entries = builtin_pricing();
    entries.remove("default");
    let catalog = pricing_catalog();
    for (name, price) in &catalog.entries {
        entries.insert(name.clone(), *price);
    }
    entries
}

pub fn default_price() -> Price {
    builtin_pricing().get("default").copied().unwrap_or(Price {
        input: 3.0,
        output: 15.0,
        cw: 0.0,
        cr: 0.0,
    })
}

pub fn pricing_source() -> String {
    let catalog = pricing_catalog();
    let source = catalog_source(catalog);
    if pricing_override_models().is_empty() {
        source
    } else {
        format!("{source} + user overrides available")
    }
}

fn catalog_source(catalog: &PricingCatalog) -> String {
    // Labels are deliberately clock-free: identical inputs must produce
    // byte-identical reports (see scripts/ci/check-deterministic-output.sh).
    // The previous labels embedded cache/fetch timestamps.
    match catalog.source.as_str() {
        "cache" => "LiteLLM (cached catalog)".to_string(),
        "cache(stale)" => {
            "LiteLLM (cached catalog, stale; run --update-pricing to refresh)".to_string()
        }
        "remote" => "LiteLLM (just refreshed)".to_string(),
        "snapshot" => format!("LiteLLM snapshot {PRICING_SNAPSHOT_DATE} (bundled)"),
        _ => "built-in fallback (run --update-pricing for the latest catalog)".to_string(),
    }
}

pub fn pricing_source_for(model: &str) -> String {
    let catalog = pricing_catalog();
    pricing_source_for_catalog(model, catalog, pricing_override_models())
}

fn pricing_source_for_catalog(
    model: &str,
    catalog: &PricingCatalog,
    override_models: &BTreeSet<String>,
) -> String {
    let normalized = normalize_model(model);
    let resolved = resolve_alias(&normalized, &catalog.aliases);
    let Some(key) = matching_catalog_key(&resolved, &catalog.entries) else {
        return "built-in fallback".to_string();
    };
    if override_models.contains(&key) {
        "user override".to_string()
    } else if normalized != resolved {
        format!("{} via user override alias", catalog_source(catalog))
    } else {
        catalog_source(catalog)
    }
}

pub fn pricing_cache_path() -> PathBuf {
    user_cache_dir().join("agenttrace").join("pricing.json")
}

pub fn update_pricing() -> anyhow::Result<usize> {
    let (raw, entries) = download_pricing(Duration::from_secs(30))?;
    write_pricing_cache(&raw)?;
    let count = entries.len();
    // Publish the fresh catalog to later pricing_catalog() consumers in
    // this process (a no-op when the singleton was already initialized).
    let mut catalog = PricingCatalog {
        entries,
        aliases: BTreeMap::new(),
        source: "remote".to_string(),
    };
    let override_models = apply_pricing_overrides(&mut catalog);
    let _ = PRICING_OVERRIDE_MODELS.set(override_models);
    let _ = PRICING_CATALOG.set(catalog);
    Ok(count)
}

pub fn render_model_pricing_list() -> String {
    let prices = list_pricing();
    let default = default_price();
    let names = prices.keys().cloned().collect::<Vec<_>>();
    let name_width = pricing_name_width(&names);
    let mut out = String::new();
    out.push_str(&format!(
        "agenttrace v{} - Supported Models\n",
        crate::VERSION
    ));
    out.push_str(&format!("{}\n", "=".repeat(58.max(name_width + 28))));
    out.push_str(&format!("Source: {}\n", pricing_source()));
    out.push_str(&format!(
        "{} model prices loaded. Common/default models are shown first; the complete catalog follows.\n\n",
        prices.len()
    ));
    out.push_str("Common/default pricing\n");
    write_pricing_header(&mut out, name_width);
    out.push_str(&format!(
        "  {:<width$} ${:>8.2}  ${:>8.2}\n",
        "default",
        default.input,
        default.output,
        width = name_width
    ));
    for &name in common_pricing_models() {
        if let Some(price) = prices.get(name) {
            out.push_str(&format!(
                "  {:<width$} ${:>8.2}  ${:>8.2}\n",
                name,
                price.input,
                price.output,
                width = name_width
            ));
        }
    }
    out.push('\n');
    out.push_str(&format!("Full pricing catalog ({} models)\n", prices.len()));
    write_pricing_header(&mut out, name_width);
    for (name, price) in prices {
        out.push_str(&format!(
            "  {:<width$} ${:>8.2}  ${:>8.2}\n",
            name,
            price.input,
            price.output,
            width = name_width
        ));
    }
    out.push('\n');
    out
}

pub fn render_test_match() -> String {
    let mut out = format!("Pricing: {}\n\n", pricing_source());
    for model in [
        "claude-sonnet-4-5-20250929",
        "anthropic/claude-sonnet-4-6",
        "vertex_ai/claude-opus-4-5@20251101",
        "us.anthropic.claude-sonnet-4-5-20250929-v1:0",
        "openai/gpt-4.1",
        "gpt-4.1-mini-2025-04-14",
        "deepseek-chat",
        "deepseek/deepseek-v3.2",
        "gemini-2.5-pro",
        "unknown-model-xyz",
    ] {
        let p = lookup_price(model);
        out.push_str(&format!(
            "  {:<50} → in=${:>7.2}/M  out=${:>7.2}/M  cw=${:>6.2}/M  cr=${:>6.2}/M\n",
            model, p.input, p.output, p.cw, p.cr
        ));
    }
    out
}

pub(crate) fn token_cost(
    input: i64,
    output: i64,
    cache_write: i64,
    cache_read: i64,
    model: &str,
) -> f64 {
    let price = lookup_price(model);
    round4(
        input as f64 / 1e6 * price.input
            + output as f64 / 1e6 * price.output
            + cache_write as f64 / 1e6 * price.cw
            + cache_read as f64 / 1e6 * price.cr,
    )
}

fn pricing_catalog() -> &'static PricingCatalog {
    PRICING_CATALOG.get_or_init(load_catalog_for_current_env)
}

/// Resolve the pricing catalog without touching the network: a cached
/// catalog is served as-is regardless of age, and when no cache exists the
/// bundled snapshot is used. The only network path is the explicit
/// `--update-pricing` action.
fn load_catalog_for_current_env() -> PricingCatalog {
    let mut catalog = load_pricing_cache().unwrap_or_else(fallback_catalog);
    let override_models = apply_pricing_overrides(&mut catalog);
    let _ = PRICING_OVERRIDE_MODELS.set(override_models);
    catalog
}

fn fallback_catalog() -> PricingCatalog {
    let entries = convert_litellm(PRICING_SNAPSHOT_JSON.as_bytes());
    if entries.is_empty() {
        return PricingCatalog {
            entries: builtin_pricing(),
            aliases: BTreeMap::new(),
            source: "builtin".to_string(),
        };
    }
    PricingCatalog {
        entries,
        aliases: BTreeMap::new(),
        source: "snapshot".to_string(),
    }
}

fn apply_pricing_overrides(catalog: &mut PricingCatalog) -> BTreeSet<String> {
    let mut override_models = BTreeSet::new();
    if let Some((prices, aliases)) = load_pricing_overrides() {
        override_models.extend(prices.keys().cloned());
        catalog.entries.extend(prices);
        catalog.aliases.extend(aliases);
    }
    override_models
}

fn load_pricing_cache() -> Option<PricingCatalog> {
    let path = pricing_cache_path();
    let metadata = path.metadata().ok()?;
    let stale = metadata
        .modified()
        .ok()
        .and_then(|time| SystemTime::now().duration_since(time).ok())
        .map(|age| age > CACHE_MAX_AGE)
        .unwrap_or(false);
    let raw = std::fs::read(&path).ok()?;
    let entries = convert_litellm(&raw);
    if entries.is_empty() {
        return None;
    }
    Some(PricingCatalog {
        entries,
        aliases: BTreeMap::new(),
        source: if stale { "cache(stale)" } else { "cache" }.to_string(),
    })
}

fn pricing_override_models() -> &'static BTreeSet<String> {
    PRICING_OVERRIDE_MODELS.get_or_init(BTreeSet::new)
}

fn download_pricing(timeout: Duration) -> anyhow::Result<(String, BTreeMap<String, Price>)> {
    let raw = ureq::get(PRICING_URL)
        .timeout(timeout)
        .call()
        .map_err(|err| anyhow!("download failed: {err}"))?
        .into_string()
        .context("read pricing response")?;
    let entries = convert_litellm(raw.as_bytes());
    if entries.is_empty() {
        return Err(anyhow!("no chat models found in downloaded data"));
    }
    Ok((raw, entries))
}

fn write_pricing_cache(raw: &str) -> anyhow::Result<()> {
    let path = pricing_cache_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    // Stage through a unique temp sibling, then rename into place so a
    // crash mid-write can no longer leave a torn catalog behind
    // (pass-7 P7-5); sweep_orphaned_temps reclaims the temp.
    let tmp = crate::session_cache::unique_temp_path(&path);
    std::fs::write(&tmp, raw)?;
    std::fs::rename(&tmp, &path)?;
    Ok(())
}

fn lookup_price_in(model: &str, entries: &BTreeMap<String, Price>) -> Price {
    if let Some(key) = matching_catalog_key(model, entries) {
        return entries.get(&key).copied().unwrap_or_default();
    }
    let builtin = builtin_pricing();
    for variant in match_variants(model) {
        if let Some(price) = builtin.get(&variant) {
            return *price;
        }
    }
    builtin.get("default").copied().unwrap_or_default()
}

fn matching_catalog_key(model: &str, entries: &BTreeMap<String, Price>) -> Option<String> {
    match_variants(model)
        .into_iter()
        .find(|variant| entries.contains_key(variant))
}

#[derive(Default, Deserialize)]
struct PricingOverrides {
    #[serde(default)]
    prices: BTreeMap<String, Price>,
    #[serde(default)]
    aliases: BTreeMap<String, String>,
}

fn load_pricing_overrides() -> Option<(BTreeMap<String, Price>, BTreeMap<String, String>)> {
    let path = std::env::var_os("AGENTTRACE_PRICING_FILE").map(PathBuf::from)?;
    parse_pricing_overrides(&std::fs::read(path).ok()?)
}

fn parse_pricing_overrides(
    raw: &[u8],
) -> Option<(BTreeMap<String, Price>, BTreeMap<String, String>)> {
    let overrides: PricingOverrides = serde_json::from_slice(raw).ok()?;
    Some((
        overrides
            .prices
            .into_iter()
            .map(|(model, price)| (normalize_model(&model), price))
            .collect(),
        overrides
            .aliases
            .into_iter()
            .map(|(alias, model)| (normalize_model(&alias), normalize_model(&model)))
            .collect(),
    ))
}

fn resolve_alias(model: &str, aliases: &BTreeMap<String, String>) -> String {
    let mut current = normalize_model(model);
    for _ in 0..8 {
        let Some(next) = aliases.get(&current) else {
            break;
        };
        if next == &current {
            break;
        }
        current = next.clone();
    }
    current
}

fn convert_litellm(raw: &[u8]) -> BTreeMap<String, Price> {
    let Ok(Value::Object(source)) = serde_json::from_slice::<Value>(raw) else {
        return BTreeMap::new();
    };
    let mut selected: BTreeMap<String, (i32, Price)> = BTreeMap::new();
    for (key, value) in source {
        let Ok(model) = serde_json::from_value::<LiteLlmModel>(value) else {
            continue;
        };
        if model.mode != "chat" || (model.input_cost == 0.0 && model.output_cost == 0.0) {
            continue;
        }
        let normalized = normalize_model(&key);
        if normalized == "default" || normalized == "unknown" {
            continue;
        }
        let price = Price {
            input: model.input_cost * 1e6,
            output: model.output_cost * 1e6,
            cw: model.cache_write_cost * 1e6,
            cr: model.cache_read_cost * 1e6,
        };
        // Hostile or overflowing catalog rates must not reach costing:
        // the 1e6 scaling can turn a near-f64-max per-token cost into
        // inf, which used to survive into reports (pass-8 F8-5). Skip
        // the entry; the model falls back to default pricing and shows
        // up in data_health as fallback_pricing.
        if !(price.input.is_finite()
            && price.output.is_finite()
            && price.cw.is_finite()
            && price.cr.is_finite())
        {
            continue;
        }
        let priority = provider_priority(&model.provider);
        match selected.get(&normalized) {
            Some((existing, _)) if *existing >= priority => {}
            _ => {
                selected.insert(normalized, (priority, price));
            }
        }
    }
    selected
        .into_iter()
        .map(|(name, (_, price))| (name, price))
        .collect()
}

fn provider_priority(provider: &str) -> i32 {
    match provider {
        "anthropic" | "openai" | "deepseek" | "gemini" | "xai" | "mistral" => 10,
        "cohere" => 9,
        "openrouter" => 8,
        "vercel_ai_gateway" => 7,
        "github_copilot" => 6,
        "bedrock_converse"
        | "bedrock"
        | "vertex_ai-anthropic_models"
        | "vertex_ai-language-models"
        | "azure"
        | "azure_ai" => 5,
        _ => 0,
    }
}

fn match_variants(raw: &str) -> Vec<String> {
    let normalized = normalize_model(raw);
    let mut variants = vec![raw.to_string(), normalized.clone()];
    if normalized.matches('-').count() >= 2 {
        let parts = normalized.split('-').collect::<Vec<_>>();
        let last = parts.last().copied().unwrap_or("");
        let minor = last.len() <= 3
            && (last.chars().next().is_some_and(|c| c.is_ascii_digit())
                || matches!(last, "mini" | "nano" | "flash" | "lite" | "pro"));
        if minor {
            variants.push(parts[..parts.len() - 1].join("-"));
            if parts.len() >= 3 {
                variants.push(parts[..parts.len() - 2].join("-"));
            }
        }
    }
    if normalized.contains("deepseek") {
        if normalized.contains("v3") || normalized.contains("chat") {
            variants.push("deepseek-chat".to_string());
            variants.push("deepseek-v3".to_string());
        }
        if normalized.contains("r1") || normalized.contains("reasoner") {
            variants.push("deepseek-reasoner".to_string());
            variants.push("deepseek-r1".to_string());
        }
    }
    variants
}

fn normalize_model(raw: &str) -> String {
    if raw.is_empty() || raw == "unknown" {
        return "default".to_string();
    }
    let mut value = raw.trim().to_ascii_lowercase();
    if let Some((_, candidate)) = value.rsplit_once('/') {
        if !candidate.starts_with('v') && !candidate.starts_with("20") {
            value = candidate.to_string();
        }
    }
    for marker in [".anthropic.", ".google.", ".meta.", ".amazon."] {
        if let Some(idx) = value.find(marker) {
            if idx > 0 {
                value = value[idx + marker.len()..].to_string();
                break;
            }
        }
    }
    value = strip_date_suffix(&value);
    value = strip_version_suffix(&value);
    while value.contains("--") {
        value = value.replace("--", "-");
    }
    let value = value.trim_matches(['-', '.']).to_string();
    if value.is_empty() {
        "default".to_string()
    } else {
        value
    }
}

fn strip_date_suffix(value: &str) -> String {
    for (idx, sep) in value.char_indices().rev() {
        if sep != '-' && sep != '@' {
            continue;
        }
        let suffix = &value[idx + sep.len_utf8()..];
        let digit_count = suffix.chars().take_while(|c| c.is_ascii_digit()).count();
        if digit_count >= 4
            && suffix
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
        {
            return value[..idx].to_string();
        }
    }
    value.to_string()
}

fn strip_version_suffix(value: &str) -> String {
    for sep in [':', '@'] {
        if let Some(idx) = value.rfind(sep) {
            let suffix = &value[idx + 1..];
            let suffix = suffix.strip_prefix('v').unwrap_or(suffix);
            if !suffix.is_empty() && suffix.chars().all(|c| c.is_ascii_digit() || c == '.') {
                return value[..idx].to_string();
            }
        }
    }
    value.to_string()
}

fn builtin_pricing() -> BTreeMap<String, Price> {
    [
        (
            "claude-opus-4.7",
            Price {
                input: 5.0,
                output: 25.0,
                cw: 6.25,
                cr: 0.50,
            },
        ),
        (
            "claude-opus-4.6",
            Price {
                input: 5.0,
                output: 25.0,
                cw: 6.25,
                cr: 0.50,
            },
        ),
        (
            "claude-opus-4-7",
            Price {
                input: 5.0,
                output: 25.0,
                cw: 6.25,
                cr: 0.50,
            },
        ),
        (
            "claude-opus-4-6",
            Price {
                input: 5.0,
                output: 25.0,
                cw: 6.25,
                cr: 0.50,
            },
        ),
        (
            "claude-opus-4.5",
            Price {
                input: 5.0,
                output: 25.0,
                cw: 6.25,
                cr: 0.50,
            },
        ),
        (
            "claude-opus-4",
            Price {
                input: 15.0,
                output: 75.0,
                cw: 18.75,
                cr: 1.50,
            },
        ),
        (
            "claude-sonnet-4.6",
            Price {
                input: 3.0,
                output: 15.0,
                cw: 3.75,
                cr: 0.30,
            },
        ),
        (
            "claude-sonnet-4-6",
            Price {
                input: 3.0,
                output: 15.0,
                cw: 3.75,
                cr: 0.30,
            },
        ),
        (
            "claude-sonnet-4.5",
            Price {
                input: 3.0,
                output: 15.0,
                cw: 3.75,
                cr: 0.30,
            },
        ),
        (
            "claude-sonnet-4-5",
            Price {
                input: 3.0,
                output: 15.0,
                cw: 3.75,
                cr: 0.30,
            },
        ),
        (
            "claude-sonnet-4",
            Price {
                input: 3.0,
                output: 15.0,
                cw: 3.75,
                cr: 0.30,
            },
        ),
        (
            "claude-haiku-4-5",
            Price {
                input: 1.0,
                output: 5.0,
                cw: 1.25,
                cr: 0.10,
            },
        ),
        (
            "claude-haiku-4.5",
            Price {
                input: 1.0,
                output: 5.0,
                cw: 1.25,
                cr: 0.10,
            },
        ),
        (
            "claude-haiku-3.5",
            Price {
                input: 0.80,
                output: 4.0,
                cw: 1.0,
                cr: 0.08,
            },
        ),
        (
            "gemini-3.1-pro-preview",
            Price {
                input: 2.0,
                output: 12.0,
                cw: 0.0,
                cr: 0.20,
            },
        ),
        (
            "gemini-3-flash-preview",
            Price {
                input: 0.5,
                output: 3.0,
                cw: 0.0,
                cr: 0.05,
            },
        ),
        (
            "gemini-2.5-pro",
            Price {
                input: 1.25,
                output: 10.0,
                cw: 0.0,
                cr: 0.0,
            },
        ),
        (
            "gemini-2.5-flash",
            Price {
                input: 0.15,
                output: 0.60,
                cw: 0.0,
                cr: 0.0,
            },
        ),
        (
            "gpt-5.5",
            Price {
                input: 5.0,
                output: 30.0,
                cw: 0.0,
                cr: 0.50,
            },
        ),
        (
            "gpt-5.4",
            Price {
                input: 2.5,
                output: 15.0,
                cw: 0.0,
                cr: 0.25,
            },
        ),
        (
            "pa/gpt-5.4",
            Price {
                input: 0.0,
                output: 0.0,
                cw: 0.0,
                cr: 0.0,
            },
        ),
        (
            "gpt-5.4-mini",
            Price {
                input: 0.75,
                output: 4.5,
                cw: 0.0,
                cr: 0.075,
            },
        ),
        (
            "gpt-5.3-codex",
            Price {
                input: 1.75,
                output: 14.0,
                cw: 0.0,
                cr: 0.175,
            },
        ),
        (
            "gpt-5.2-codex",
            Price {
                input: 1.75,
                output: 14.0,
                cw: 0.0,
                cr: 0.175,
            },
        ),
        (
            "gpt-5.1",
            Price {
                input: 1.25,
                output: 10.0,
                cw: 0.0,
                cr: 0.0,
            },
        ),
        (
            "gpt-5.1-mini",
            Price {
                input: 0.25,
                output: 2.0,
                cw: 0.0,
                cr: 0.0,
            },
        ),
        (
            "gpt-5.1-codex-mini",
            Price {
                input: 0.25,
                output: 2.0,
                cw: 0.0,
                cr: 0.025,
            },
        ),
        (
            "gpt-4.1",
            Price {
                input: 2.0,
                output: 8.0,
                cw: 0.0,
                cr: 0.0,
            },
        ),
        (
            "gpt-4.1-mini",
            Price {
                input: 0.40,
                output: 1.60,
                cw: 0.0,
                cr: 0.0,
            },
        ),
        (
            "gpt-4.1-nano",
            Price {
                input: 0.10,
                output: 0.40,
                cw: 0.0,
                cr: 0.0,
            },
        ),
        (
            "deepseek-v4-pro",
            Price {
                input: 0.435,
                output: 0.87,
                cw: 0.0,
                cr: 0.003625,
            },
        ),
        (
            "deepseek-v4-flash",
            Price {
                input: 0.14,
                output: 0.28,
                cw: 0.0,
                cr: 0.0028,
            },
        ),
        (
            "deepseek-chat",
            Price {
                input: 0.27,
                output: 1.10,
                cw: 0.07,
                cr: 0.014,
            },
        ),
        (
            "deepseek-reasoner",
            Price {
                input: 0.55,
                output: 2.19,
                cw: 0.14,
                cr: 0.028,
            },
        ),
        (
            "glm-5",
            Price {
                input: 1.0,
                output: 3.20,
                cw: 0.0,
                cr: 0.20,
            },
        ),
        (
            "glm-5-turbo",
            Price {
                input: 1.20,
                output: 4.0,
                cw: 0.0,
                cr: 0.24,
            },
        ),
        (
            "glm-5.1",
            Price {
                input: 1.40,
                output: 4.40,
                cw: 0.0,
                cr: 0.26,
            },
        ),
        (
            "kimi-k2.5",
            Price {
                input: 0.60,
                output: 3.0,
                cw: 0.0,
                cr: 0.10,
            },
        ),
        (
            "kimi-k2.6",
            Price {
                input: 0.95,
                output: 4.0,
                cw: 0.0,
                cr: 0.16,
            },
        ),
        (
            "mimo-v2-pro",
            Price {
                input: 0.10,
                output: 0.30,
                cw: 0.0,
                cr: 0.02,
            },
        ),
        (
            "mimo-v2.5-pro",
            Price {
                input: 0.0,
                output: 0.0,
                cw: 0.0,
                cr: 0.0,
            },
        ),
        (
            "minimax-2.5",
            Price {
                input: 0.30,
                output: 2.40,
                cw: 0.375,
                cr: 0.03,
            },
        ),
        (
            "minimax-2.7",
            Price {
                input: 0.30,
                output: 2.40,
                cw: 0.375,
                cr: 0.03,
            },
        ),
        (
            "minimax-2.7-highspeed",
            Price {
                input: 0.30,
                output: 2.40,
                cw: 0.375,
                cr: 0.03,
            },
        ),
        (
            "minimax-m2.5",
            Price {
                input: 0.30,
                output: 1.20,
                cw: 0.375,
                cr: 0.03,
            },
        ),
        (
            "minimax-m2.5-free",
            Price {
                input: 0.30,
                output: 2.40,
                cw: 0.375,
                cr: 0.03,
            },
        ),
        (
            "minimax-m2.7",
            Price {
                input: 0.30,
                output: 2.40,
                cw: 0.375,
                cr: 0.03,
            },
        ),
        (
            "qwen/qwen3.6-plus-04-02:free",
            Price {
                input: 0.325,
                output: 1.95,
                cw: 0.40625,
                cr: 0.0,
            },
        ),
        (
            "qwen3.6-plus",
            Price {
                input: 0.325,
                output: 1.95,
                cw: 0.40625,
                cr: 0.0,
            },
        ),
        (
            "qwen3.5-plus",
            Price {
                input: 0.40,
                output: 2.40,
                cw: 0.0,
                cr: 0.0,
            },
        ),
        (
            "qwen3.6:35b-a3b-coding-nvfp4",
            Price {
                input: 0.0,
                output: 0.0,
                cw: 0.0,
                cr: 0.0,
            },
        ),
        (
            "doubao-seed-2-0-pro",
            Price {
                input: 0.0,
                output: 0.0,
                cw: 0.0,
                cr: 0.0,
            },
        ),
        (
            "stepfun/step-3.5-flash:free",
            Price {
                input: 0.0,
                output: 0.0,
                cw: 0.0,
                cr: 0.0,
            },
        ),
        (
            "grok-code-fast-1",
            Price {
                input: 0.20,
                output: 1.50,
                cw: 0.0,
                cr: 0.02,
            },
        ),
        (
            "x-ai/grok-code-fast-1",
            Price {
                input: 0.20,
                output: 1.50,
                cw: 0.0,
                cr: 0.02,
            },
        ),
        (
            "<synthetic>",
            Price {
                input: 0.0,
                output: 0.0,
                cw: 0.0,
                cr: 0.0,
            },
        ),
        (
            "grok-3",
            Price {
                input: 3.0,
                output: 15.0,
                cw: 0.0,
                cr: 0.0,
            },
        ),
        (
            "default",
            Price {
                input: 3.0,
                output: 15.0,
                cw: 0.0,
                cr: 0.0,
            },
        ),
    ]
    .into_iter()
    .map(|(name, price)| (name.to_string(), price))
    .collect()
}

fn common_pricing_models() -> &'static [&'static str] {
    &[
        "claude-sonnet-4",
        "claude-opus-4.5",
        "gpt-5.1",
        "gpt-5.1-mini",
        "gpt-4.1",
        "gpt-4.1-mini",
        "gemini-2.5-pro",
        "gemini-2.5-flash",
        "deepseek-chat",
        "deepseek-reasoner",
        "grok-code-fast-1",
    ]
}

fn pricing_name_width(names: &[String]) -> usize {
    names
        .iter()
        .map(|name| name.len())
        .max()
        .unwrap_or(0)
        .max("Model".len())
        .max(22)
}

fn write_pricing_header(out: &mut String, name_width: usize) {
    out.push_str(&format!(
        "  {:<width$} {:>10} {:>10}\n",
        "Model",
        "Input $/M",
        "Output $/M",
        width = name_width
    ));
    out.push_str(&format!("  {}\n", "-".repeat(name_width + 24)));
}

fn user_cache_dir() -> PathBuf {
    if cfg!(target_os = "macos") {
        if let Some(home) = std::env::var_os("HOME").map(PathBuf::from) {
            return home.join("Library").join("Caches");
        }
    }
    if let Some(cache) = std::env::var_os("XDG_CACHE_HOME").map(PathBuf::from) {
        if !cache.as_os_str().is_empty() {
            return cache;
        }
    }
    if let Some(home) = std::env::var_os("HOME").map(PathBuf::from) {
        return home.join(".cache");
    }
    std::env::temp_dir()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtin_fallback_includes_go_alias_slice() {
        let prices = builtin_pricing();
        for name in [
            "claude-haiku-3.5",
            "pa/gpt-5.4",
            "gpt-5.1-codex-mini",
            "deepseek-v4-pro",
            "glm-5.1",
            "mimo-v2.5-pro",
            "minimax-2.7-highspeed",
            "qwen/qwen3.6-plus-04-02:free",
            "qwen3.6:35b-a3b-coding-nvfp4",
            "stepfun/step-3.5-flash:free",
            "grok-3",
        ] {
            assert!(
                prices.contains_key(name),
                "missing builtin pricing for {name}"
            );
        }
    }

    #[test]
    fn user_pricing_overrides_normalize_models_and_aliases() {
        let (prices, aliases) = parse_pricing_overrides(
            br#"{"prices":{"Provider/My-Model":{"input":1,"output":2,"cw":3,"cr":4}},"aliases":{"MY-ALIAS":"Provider/My-Model"}}"#,
        )
        .expect("pricing overrides");
        assert!(prices.contains_key("my-model"));
        assert_eq!(
            aliases.get("my-alias").map(String::as_str),
            Some("my-model")
        );
    }

    #[test]
    fn aliases_resolve_normalized_chains_without_looping() {
        let aliases = BTreeMap::from([
            ("provider-model".to_string(), "model-v2".to_string()),
            ("model-v2".to_string(), "model".to_string()),
            ("loop".to_string(), "loop-2".to_string()),
            ("loop-2".to_string(), "loop".to_string()),
        ]);
        assert_eq!(resolve_alias("PROVIDER-MODEL", &aliases), "model");
        assert_eq!(resolve_alias("loop", &aliases), "loop");
    }

    /// Run `body` with the pricing cache environment pointed at an empty
    /// temporary directory unique to this invocation. The lock serializes
    /// invocations: they mutate process-global env vars, so parallel runs
    /// previously read each other's cache state, and the shared pid-keyed
    /// directory let one test's cleanup delete another's seeded cache file
    /// (`cache re-read: NotFound` flakes under the combined CI test run).
    fn with_isolated_cache_env<T>(body: impl FnOnce() -> T) -> T {
        use std::sync::atomic::{AtomicUsize, Ordering};
        static LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
        static SEQ: AtomicUsize = AtomicUsize::new(0);
        let _guard = LOCK.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        let prior_xdg = std::env::var_os("XDG_CACHE_HOME");
        let prior_home = std::env::var_os("HOME");
        let seq = SEQ.fetch_add(1, Ordering::SeqCst);
        let dir = std::env::temp_dir().join(format!(
            "agenttrace-pricing-test-{}-{seq}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("temp cache dir");
        std::env::set_var("XDG_CACHE_HOME", &dir);
        std::env::set_var("HOME", &dir);
        let result = body();
        match prior_xdg {
            Some(value) => std::env::set_var("XDG_CACHE_HOME", value),
            None => std::env::remove_var("XDG_CACHE_HOME"),
        }
        match prior_home {
            Some(value) => std::env::set_var("HOME", value),
            None => std::env::remove_var("HOME"),
        }
        let _ = std::fs::remove_dir_all(&dir);
        result
    }

    #[test]
    fn catalog_uses_bundled_snapshot_without_cache_and_never_writes_one() {
        with_isolated_cache_env(|| {
            let catalog = load_catalog_for_current_env();
            assert_eq!(catalog.source, "snapshot");
            assert!(!catalog.entries.is_empty());
            assert!(catalog.entries.contains_key("claude-sonnet-4-5"));
            // The offline read path must not create or rewrite a cache file:
            // this is the guarantee that reports and tests never download.
            assert!(!pricing_cache_path().exists());
        });
    }

    #[test]
    fn stale_cache_is_served_as_is_without_download_or_rewrite() {
        with_isolated_cache_env(|| {
            let raw = br#"{"anthropic/claude-sonnet-4-5":{"input_cost_per_token":3e-6,"output_cost_per_token":1.5e-5,"cache_creation_input_token_cost":3.75e-7,"cache_read_input_token_cost":3e-7,"mode":"chat","litellm_provider":"anthropic"}}"#;
            let path = pricing_cache_path();
            std::fs::create_dir_all(path.parent().expect("cache parent")).expect("cache dir");
            std::fs::write(&path, raw).expect("cache seed");
            std::fs::File::open(&path)
                .expect("cache open")
                .set_modified(SystemTime::now() - Duration::from_secs(7 * 24 * 60 * 60))
                .expect("backdate cache");
            let before = std::fs::read(&path).expect("cache read");
            let catalog = load_catalog_for_current_env();
            assert_eq!(catalog.source, "cache(stale)");
            assert_eq!(
                catalog.entries.get("claude-sonnet-4-5").map(|p| p.input),
                Some(3.0)
            );
            let after = std::fs::read(&path).expect("cache re-read");
            assert_eq!(
                before, after,
                "the read path must never rewrite the pricing cache"
            );
        });
    }

    #[test]
    fn pricing_source_labels_carry_no_wall_clock() {
        let catalog = |source: &str| PricingCatalog {
            entries: BTreeMap::new(),
            aliases: BTreeMap::new(),
            source: source.to_string(),
        };
        assert_eq!(
            catalog_source(&catalog("cache")),
            "LiteLLM (cached catalog)"
        );
        assert_eq!(
            catalog_source(&catalog("cache(stale)")),
            "LiteLLM (cached catalog, stale; run --update-pricing to refresh)"
        );
        assert_eq!(
            catalog_source(&catalog("remote")),
            "LiteLLM (just refreshed)"
        );
        assert_eq!(
            catalog_source(&catalog("snapshot")),
            format!("LiteLLM snapshot {PRICING_SNAPSHOT_DATE} (bundled)")
        );
        assert_eq!(
            catalog_source(&catalog("builtin")),
            "built-in fallback (run --update-pricing for the latest catalog)"
        );
    }

    #[test]
    fn pricing_snapshot_date_is_pinned_to_the_bundled_payload() {
        // The const and the snapshot payload were previously kept in sync
        // by prose only (a header comment in update-snapshot.sh). This pin
        // goes red the moment one drifts from the other (P5-3).
        let snapshot: serde_json::Value =
            serde_json::from_str(PRICING_SNAPSHOT_JSON).expect("bundled snapshot parses");
        let date = snapshot["_snapshot"]["date"]
            .as_str()
            .expect("_snapshot.date is a string");
        assert_eq!(
            date, PRICING_SNAPSHOT_DATE,
            "PRICING_SNAPSHOT_DATE drifted from the bundled pricing_snapshot.json; \
             update the const together with the snapshot (see \
             scripts/pricing/update-snapshot.sh)"
        );
    }

    #[test]
    fn pricing_source_is_specific_to_the_price_that_matched() {
        let catalog = PricingCatalog {
            entries: BTreeMap::from([
                ("catalog-model".to_string(), Price::default()),
                ("override-model".to_string(), Price::default()),
            ]),
            aliases: BTreeMap::from([("alias-model".to_string(), "catalog-model".to_string())]),
            source: "cache".to_string(),
        };
        let overrides = BTreeSet::from(["override-model".to_string()]);
        assert!(
            pricing_source_for_catalog("catalog-model", &catalog, &overrides)
                .starts_with("LiteLLM")
        );
        assert_eq!(
            pricing_source_for_catalog("override-model", &catalog, &overrides),
            "user override"
        );
        assert!(
            pricing_source_for_catalog("alias-model", &catalog, &overrides)
                .contains("via user override alias")
        );
    }

    #[test]
    fn convert_litellm_rejects_non_finite_scaled_rates() {
        // Pass-8 F8-5: per-token rates near f64::MAX turn into inf after
        // the *1e6 scaling and used to flow into costing, poisoning
        // totals until json_float panicked. Non-finite entries are
        // skipped; the model falls back to default pricing (and shows
        // up as fallback_pricing in data health) instead of lying with
        // a NaN cost.
        let hostile = serde_json::json!({
            "finite-model": {
                "input_cost_per_token": 0.000003,
                "output_cost_per_token": 0.000015,
                "mode": "chat",
                "litellm_provider": "finite"
            },
            "poisoned-model": {
                "input_cost_per_token": 1.7976931348623157e308,
                "output_cost_per_token": 0.000015,
                "mode": "chat",
                "litellm_provider": "poisoned"
            }
        });
        let catalog = convert_litellm(
            serde_json::to_vec(&hostile)
                .expect("serialize catalog")
                .as_slice(),
        );
        assert!(
            catalog.contains_key("finite-model"),
            "finite entries survive the conversion"
        );
        assert!(
            !catalog.contains_key("poisoned-model"),
            "entries whose scaled price is non-finite are dropped"
        );
    }
}
