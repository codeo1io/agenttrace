use crate::round4;
use anyhow::{anyhow, Context};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const PRICING_URL: &str =
    "https://raw.githubusercontent.com/BerriAI/litellm/main/model_prices_and_context_window.json";
/// Trimmed LiteLLM chat-model pricing snapshot, vendored so agenttrace is
/// fully offline by default. Regenerate with `scripts/pricing/update-snapshot.sh`
/// and keep `PRICING_SNAPSHOT_DATE` in sync with the date it prints.
const PRICING_SNAPSHOT_JSON: &str = include_str!("pricing_snapshot.json");
const PRICING_SNAPSHOT_DATE: &str = "2026-10-04";
const CACHE_MAX_AGE: Duration = Duration::from_secs(24 * 60 * 60);
static PRICING_CATALOG: OnceLock<PricingCatalog> = OnceLock::new();
static PRICING_OVERRIDE_MODELS: OnceLock<BTreeSet<String>> = OnceLock::new();
static PRICING_OVERRIDE_STATUS: OnceLock<PricingOverrideStatus> = OnceLock::new();

/// Cycle-4 B1: what actually happened to a requested
/// `AGENTTRACE_PRICING_FILE`. Before this, an unreadable path, invalid
/// JSON, wrong-schema keys (the natural trap: pasting a LiteLLM snapshot
/// straight in), or a negative rate all collapsed into the same silent
/// "zero overrides, rc0, empty stderr" outcome while `--test-match`
/// kept advertising the bundled catalog (ccusage #1810 reported the same
/// failure shape in the wild).
#[derive(Debug, Clone)]
pub enum PricingOverrideStatus {
    /// No `AGENTTRACE_PRICING_FILE` was set — bundled catalog only.
    NotRequested,
    /// The file parsed, validated, and its entries were applied.
    Applied {
        path: PathBuf,
        models: usize,
        aliases: usize,
    },
    /// The file was requested but rejected; overrides are NOT active and
    /// `reason` names the first concrete cause.
    Failed { path: PathBuf, reason: String },
}

pub fn pricing_override_status() -> &'static PricingOverrideStatus {
    PRICING_OVERRIDE_STATUS.get_or_init(|| PricingOverrideStatus::NotRequested)
}

#[derive(Debug, Clone, Default, serde::Serialize, Deserialize)]
pub struct Price {
    pub input: f64,
    pub output: f64,
    pub cw: f64,
    pub cr: f64,
    /// Vendor context window in input tokens (LiteLLM
    /// `max_input_tokens`), when the catalog entry carries one (rm-231).
    /// Absent on override files and pre-2026-10 snapshots; `skip`
    /// keeps serialized pricing output byte-identical for models
    /// without it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_input_tokens: Option<u64>,
    /// Vendor deprecation date (LiteLLM `deprecation_date`, ISO
    /// YYYY-MM-DD), when the entry carries one (rm-419).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub deprecation_date: Option<String>,
}

#[derive(Debug, Clone)]
pub struct PricingCatalog {
    pub entries: BTreeMap<String, Price>,
    pub aliases: BTreeMap<String, String>,
    pub source: String,
    /// Catalog key -> provider label ("anthropic", "openrouter", ...)
    /// preserved from the same LiteLLM rows the rates came from
    /// (rm-245). Read-only attribution data: provenance and freshness
    /// semantics stay with pricing_source_for and are untouched here.
    pub providers: BTreeMap<String, String>,
    /// The catalog's own vintage as YYYYMMDD (rm-419): the bundled
    /// snapshot's pinned date, a cached catalog's fetch date, or today
    /// for a just-refreshed remote fetch. Deprecation disclosure
    /// compares model deprecation dates against THIS — never the wall
    /// clock — so report bytes stay stable for a given catalog.
    pub reference_date: Option<u32>,
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
    #[serde(default, rename = "max_input_tokens")]
    max_input_tokens: Option<u64>,
    #[serde(default, rename = "deprecation_date")]
    deprecation_date: Option<String>,
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
        entries.insert(name.clone(), price.clone());
    }
    entries
}

pub fn default_price() -> Price {
    builtin_pricing().get("default").cloned().unwrap_or(Price {
        input: 3.0,
        output: 15.0,
        cw: 0.0,
        cr: 0.0,
        ..Default::default()
    })
}

pub fn pricing_source() -> String {
    let catalog = pricing_catalog();
    let source = catalog_source(catalog);
    match pricing_override_status() {
        PricingOverrideStatus::NotRequested => source,
        PricingOverrideStatus::Applied {
            path,
            models,
            aliases,
        } => format!(
            "{source} + user overrides applied ({} model(s), {aliases} alias(es) from {})",
            models,
            path.display()
        ),
        PricingOverrideStatus::Failed { path, reason } => format!(
            "{source} + user overrides FAILED for {}: {reason}",
            path.display()
        ),
    }
}

/// Model count in the bundled offline snapshot, parsed once, for
/// provenance disclosure (cycle 7 R1: users should be able to see how
/// old the offline catalog they are paying from actually is).
pub fn bundled_snapshot_date() -> &'static str {
    PRICING_SNAPSHOT_DATE
}

pub fn bundled_snapshot_model_count() -> usize {
    static COUNT: std::sync::OnceLock<usize> = std::sync::OnceLock::new();
    *COUNT.get_or_init(|| {
        serde_json::from_str::<serde_json::Value>(PRICING_SNAPSHOT_JSON)
            .ok()
            .and_then(|value| {
                value["_snapshot"]["models"]
                    .as_u64()
                    .map(|models| models as usize)
            })
            .unwrap_or(0)
    })
}

/// Whole days between today (UTC) and the bundled snapshot date, or
/// `None` if the pinned date somehow fails to parse.
pub fn bundled_snapshot_age_days() -> Option<i64> {
    let date = chrono::NaiveDate::parse_from_str(PRICING_SNAPSHOT_DATE, "%Y-%m-%d").ok()?;
    Some((chrono::Utc::now().date_naive() - date).num_days().max(0))
}

/// Parse an ISO `YYYY-MM-DD` string into a comparable YYYYMMDD integer
/// (digit-ordering makes the integer order the date order).
fn iso_to_ymd(iso: &str) -> Option<u32> {
    let digits: String = iso.chars().filter(|c| c.is_ascii_digit()).collect();
    if digits.len() != 8 {
        return None;
    }
    digits.parse().ok()
}

fn ymd_to_iso(ymd: u32) -> String {
    format!(
        "{:04}-{:02}-{:02}",
        ymd / 10000,
        (ymd / 100) % 100,
        ymd % 100
    )
}

fn today_ymd() -> Option<u32> {
    chrono::Utc::now().format("%Y%m%d").to_string().parse().ok()
}

/// True when the entry's vendor deprecation date is on or before the
/// catalog's own vintage — i.e. the vendor has already retired the
/// model by the catalog's reference point (rm-419). Clock-free by
/// construction: both dates come from the catalog itself.
fn past_deprecated(price: &Price, reference_ymd: u32) -> bool {
    price
        .deprecation_date
        .as_deref()
        .and_then(iso_to_ymd)
        .is_some_and(|ymd| ymd <= reference_ymd)
}

/// Count bundled-snapshot models past their vendor deprecation date
/// relative to the snapshot's own date, plus the oldest such date
/// (rm-419). The anchor is the snapshot date, so the census is a
/// property of the vendored file, never of when the binary runs.
pub fn bundled_snapshot_deprecated() -> (usize, Option<String>) {
    static CENSUS: OnceLock<(usize, Option<String>)> = OnceLock::new();
    CENSUS
        .get_or_init(|| {
            let entries = convert_litellm(PRICING_SNAPSHOT_JSON.as_bytes()).entries;
            let reference = iso_to_ymd(PRICING_SNAPSHOT_DATE).unwrap_or(0);
            let mut count = 0;
            let mut oldest: Option<u32> = None;
            for price in entries.values() {
                let Some(ymd) = price.deprecation_date.as_deref().and_then(iso_to_ymd) else {
                    continue;
                };
                if ymd <= reference {
                    count += 1;
                    if !oldest.is_some_and(|current| ymd >= current) {
                        oldest = Some(ymd);
                    }
                }
            }
            (count, oldest.map(ymd_to_iso))
        })
        .clone()
}

/// Resolve a model's vendor context window (input-token cap) from the
/// active catalog (rm-231). Returns `None` when the model has no
/// catalog entry or the entry carries no window; callers fall back to
/// the documented name-substring ladder and must label the resulting
/// utilization as an estimate.
pub fn lookup_context_window(model: &str) -> Option<u64> {
    lookup_context_window_in(model, pricing_catalog())
}

fn lookup_context_window_in(model: &str, catalog: &PricingCatalog) -> Option<u64> {
    let resolved = resolve_alias(model, &catalog.aliases);
    let key = matching_catalog_key(&resolved, &catalog.entries)?;
    catalog
        .entries
        .get(&key)
        .and_then(|price| price.max_input_tokens.filter(|window| *window > 0))
}

/// Stable identity of the active pricing catalog (rm-196): a 64-bit
/// FNV-1a digest over the catalog's entries and aliases. The session
/// cache keys its freshness on this so a catalog change (snapshot bump
/// on upgrade, `--update-pricing` refresh, override edit) invalidates
/// cached per-session costs, while identical catalogs keep every cache
/// hit. Deliberately excludes `source` (cache vs cache(stale) flips
/// with file age, not content) and `reference_date` (a same-content
/// refresh must not trigger a pointless re-price).
pub fn catalog_identity() -> &'static str {
    static IDENTITY: OnceLock<String> = OnceLock::new();
    IDENTITY.get_or_init(|| catalog_identity_of(pricing_catalog()))
}

pub(crate) fn catalog_identity_of(catalog: &PricingCatalog) -> String {
    fn fnv1a(bytes: &[u8], state: u64) -> u64 {
        bytes.iter().fold(state, |hash, byte| {
            (hash ^ *byte as u64).wrapping_mul(0x100_0000_01b3)
        })
    }
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for (name, price) in &catalog.entries {
        hash = fnv1a(name.as_bytes(), hash);
        for value in [price.input, price.output, price.cw, price.cr] {
            hash = fnv1a(&value.to_bits().to_le_bytes(), hash);
        }
        hash = fnv1a(&price.max_input_tokens.unwrap_or(0).to_le_bytes(), hash);
        hash = fnv1a(
            price.deprecation_date.as_deref().unwrap_or("").as_bytes(),
            hash,
        );
    }
    for (alias, target) in &catalog.aliases {
        hash = fnv1a(alias.as_bytes(), hash);
        hash = fnv1a(target.as_bytes(), hash);
    }
    format!("{hash:016x}")
}

fn catalog_source(catalog: &PricingCatalog) -> String {
    // Labels are deliberately clock-free: identical inputs must produce
    // byte-identical reports (see scripts/ci/check-deterministic-output.sh).
    // The previous labels embedded cache/fetch timestamps.
    let label = match catalog.source.as_str() {
        "cache" => "LiteLLM (cached catalog)".to_string(),
        "cache(stale)" => {
            "LiteLLM (cached catalog, stale; run --update-pricing to refresh)".to_string()
        }
        "remote" => "LiteLLM (just refreshed)".to_string(),
        "snapshot" => format!("LiteLLM snapshot {PRICING_SNAPSHOT_DATE} (bundled)"),
        _ => "built-in fallback (run --update-pricing for the latest catalog)".to_string(),
    };
    // Deprecation census (rm-419): disclose how much of the priced set
    // the vendor has already retired, anchored to the catalog's own
    // vintage so the suffix is a property of the catalog, not of when
    // the binary runs. Empty catalogs keep the legacy label untouched.
    if let Some(reference) = catalog.reference_date {
        let deprecated = catalog
            .entries
            .values()
            .filter(|price| past_deprecated(price, reference))
            .count();
        if deprecated > 0 {
            return format!(
                "{label}; {deprecated} of {} priced models past vendor deprecation",
                catalog.entries.len()
            );
        }
    }
    label
}

pub fn pricing_source_for(model: &str) -> String {
    let catalog = pricing_catalog();
    pricing_source_for_catalog(model, catalog, pricing_override_models())
}

/// Model-vendor attribution (rm-245): the provider label of the catalog
/// row that would price this model. Resolution mirrors the rate lookup
/// exactly - alias first, then variant match - so a provider is claimed
/// only where a price is claimed. Returns None when the catalog does
/// not know the model; callers must bucket those explicitly (as
/// "unknown") instead of dropping them.
pub fn provider_for(model: &str) -> Option<String> {
    let catalog = pricing_catalog();
    provider_for_in(model, catalog)
}

fn provider_for_in(model: &str, catalog: &PricingCatalog) -> Option<String> {
    let normalized = normalize_model(model);
    let resolved = resolve_alias(&normalized, &catalog.aliases);
    let key = matching_catalog_key(&resolved, &catalog.entries)?;
    catalog
        .providers
        .get(&key)
        .cloned()
        .filter(|provider| !provider.trim().is_empty())
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
        // rm-419: a user-set rate keeps the vendor retirement
        // disclosure — the status is catalog knowledge, not a rate
        // claim, so it carries no "unverified" qualifier.
        catalog
            .entries
            .get(&key)
            .zip(catalog.reference_date)
            .filter(|(price, reference)| past_deprecated(price, *reference))
            .map(|(price, _)| {
                format!(
                    "user override; model deprecated {}",
                    price.deprecation_date.as_deref().unwrap_or("unknown date")
                )
            })
            .unwrap_or_else(|| "user override".to_string())
    } else if normalized != resolved {
        // rm-511: an alias override supplies identity, not a rate — the
        // session still prices at catalog rates, so a vendor-retired
        // target carries the plain-catalog arm's retirement disclosure,
        // "(rate unverified)" included (unlike the direct arm above,
        // whose rate the user set themselves).
        let deprecated = catalog
            .reference_date
            .zip(catalog.entries.get(&key))
            .filter(|(reference, price)| past_deprecated(price, *reference))
            .map(|(_, price)| {
                format!(
                    "; model deprecated {} (rate unverified)",
                    price.deprecation_date.as_deref().unwrap_or("unknown date")
                )
            })
            .unwrap_or_default();
        format!(
            "{} via user override alias{}",
            catalog_source(catalog),
            deprecated
        )
    } else if let Some(reference) = catalog.reference_date {
        // Deprecation disclosure (rm-419): a session priced on a model
        // the vendor has already retired (per this catalog's vintage)
        // says so at the rate's provenance line instead of silently
        // billing at catalog rates.
        match catalog
            .entries
            .get(&key)
            .filter(|price| past_deprecated(price, reference))
        {
            Some(price) => format!(
                "{}; model deprecated {} (rate unverified)",
                catalog_source(catalog),
                price.deprecation_date.as_deref().unwrap_or("unknown date")
            ),
            None => catalog_source(catalog),
        }
    } else {
        catalog_source(catalog)
    }
}

pub fn pricing_cache_path() -> PathBuf {
    user_cache_dir().join("agenttrace").join("pricing.json")
}

pub fn update_pricing() -> anyhow::Result<usize> {
    let (raw, converted) = download_pricing(Duration::from_secs(30))?;
    write_pricing_cache(&raw)?;
    let count = converted.entries.len();
    // Publish the fresh catalog to later pricing_catalog() consumers in
    // this process (a no-op when the singleton was already initialized).
    let mut catalog = PricingCatalog {
        entries: converted.entries,
        aliases: BTreeMap::new(),
        source: "remote".to_string(),
        providers: converted.providers,
        reference_date: today_ymd(),
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
    let converted = convert_litellm(PRICING_SNAPSHOT_JSON.as_bytes());
    if converted.entries.is_empty() {
        return PricingCatalog {
            entries: builtin_pricing(),
            aliases: BTreeMap::new(),
            source: "builtin".to_string(),
            providers: BTreeMap::new(),
            reference_date: None,
        };
    }
    PricingCatalog {
        entries: converted.entries,
        aliases: BTreeMap::new(),
        source: "snapshot".to_string(),
        providers: converted.providers,
        reference_date: iso_to_ymd(PRICING_SNAPSHOT_DATE),
    }
}

fn apply_pricing_overrides(catalog: &mut PricingCatalog) -> BTreeSet<String> {
    let mut override_models = BTreeSet::new();
    if let Some((prices, aliases)) = load_pricing_overrides() {
        for (name, mut override_price) in prices {
            // rm-231/rm-419: override files usually carry only the four
            // rate fields (`max_input_tokens`/`deprecation_date` serde
            // default to None), and the previous wholesale
            // `entries.extend(prices)` replaced the whole entry — so a
            // user correcting a stale rate on a 1M-window model
            // silently regressed it to the 200k substring ladder and
            // dropped the deprecation disclosure. Backfill the
            // descriptive fields the override leaves unset; an override
            // that sets `max_input_tokens: 0` still clears the window
            // on purpose (0 is filtered out at lookup time).
            if override_price.max_input_tokens.is_none()
                || override_price.deprecation_date.is_none()
            {
                if let Some(catalog_price) = catalog.entries.get(&name) {
                    if override_price.max_input_tokens.is_none() {
                        override_price.max_input_tokens = catalog_price.max_input_tokens;
                    }
                    if override_price.deprecation_date.is_none() {
                        override_price.deprecation_date = catalog_price.deprecation_date.clone();
                    }
                }
            }
            override_models.insert(name.clone());
            catalog.entries.insert(name, override_price);
        }
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
    let converted = convert_litellm(&raw);
    if converted.entries.is_empty() {
        return None;
    }
    Some(PricingCatalog {
        entries: converted.entries,
        aliases: BTreeMap::new(),
        source: if stale { "cache(stale)" } else { "cache" }.to_string(),
        providers: converted.providers,
        // A cached catalog's vintage is its fetch date (rm-419); when
        // the provenance stamp is missing (pre-rm-167 cache) fall back
        // to the bundled snapshot date as a conservative anchor.
        reference_date: pricing_cache_vintage_ymd(&path)
            .or_else(|| iso_to_ymd(PRICING_SNAPSHOT_DATE)),
    })
}

/// YYYYMMDD of the fetch recorded in a pricing cache's provenance
/// stamp (`pricing.meta.json`, rm-167).
fn pricing_cache_vintage_ymd(cache_path: &Path) -> Option<u32> {
    let raw = std::fs::read(cache_path.with_extension("meta.json")).ok()?;
    let meta: PricingCacheMeta = serde_json::from_slice(&raw).ok()?;
    chrono::DateTime::from_timestamp(meta.fetched_at_unix as i64, 0)?
        .format("%Y%m%d")
        .to_string()
        .parse()
        .ok()
}

fn pricing_override_models() -> &'static BTreeSet<String> {
    PRICING_OVERRIDE_MODELS.get_or_init(BTreeSet::new)
}

/// Cap on a downloaded pricing catalog (rm-167). The LiteLLM catalog is
/// ~4 MB today; 32 MiB leaves growth headroom while bounding what a
/// redirected or hostile endpoint can stream into memory.
const PRICING_DOWNLOAD_MAX_BYTES: u64 = 32 * 1024 * 1024;

fn download_pricing(timeout: Duration) -> anyhow::Result<(String, ConvertedCatalog)> {
    let response = ureq::get(PRICING_URL)
        .timeout(timeout)
        .call()
        .map_err(|err| anyhow!("download failed: {err}"))?;
    let raw_bytes = read_body_capped(response.into_reader(), PRICING_DOWNLOAD_MAX_BYTES)?;
    let raw = String::from_utf8(raw_bytes).context("pricing catalog is not valid UTF-8")?;
    let converted = convert_litellm(raw.as_bytes());
    if converted.entries.is_empty() {
        return Err(anyhow!("no chat models found in downloaded data"));
    }
    Ok((raw, converted))
}

fn read_body_capped(reader: impl Read, cap: u64) -> anyhow::Result<Vec<u8>> {
    let mut bytes = Vec::new();
    reader
        .take(cap + 1)
        .read_to_end(&mut bytes)
        .context("read pricing response")?;
    if bytes.len() as u64 > cap {
        return Err(anyhow!(
            "pricing catalog exceeds the {} MiB download cap",
            cap / (1024 * 1024)
        ));
    }
    Ok(bytes)
}

/// Provenance stamp (rm-167): recorded next to every downloaded cache so
/// the catalog's origin URL, fetch time, and size are answerable from disk
/// without touching the network.
#[derive(Serialize, Deserialize)]
struct PricingCacheMeta {
    url: String,
    fetched_at_unix: u64,
    bytes: usize,
}

fn write_pricing_cache(raw: &str) -> anyhow::Result<()> {
    write_pricing_cache_at(&pricing_cache_path(), raw)
}

fn write_pricing_cache_at(path: &Path, raw: &str) -> anyhow::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    // Stage through a unique temp sibling, then rename into place so a
    // crash mid-write can no longer leave a torn catalog behind
    // (pass-7 P7-5); sweep_orphaned_temps reclaims the temp.
    let tmp = crate::session_cache::unique_temp_path(path);
    std::fs::write(&tmp, raw)?;
    std::fs::rename(&tmp, path)?;
    let meta = PricingCacheMeta {
        url: PRICING_URL.to_string(),
        fetched_at_unix: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|elapsed| elapsed.as_secs())
            .unwrap_or(0),
        bytes: raw.len(),
    };
    let meta_path = path.with_extension("meta.json");
    let meta_tmp = crate::session_cache::unique_temp_path(&meta_path);
    std::fs::write(&meta_tmp, serde_json::to_vec_pretty(&meta)?)?;
    std::fs::rename(&meta_tmp, &meta_path)?;
    Ok(())
}

fn lookup_price_in(model: &str, entries: &BTreeMap<String, Price>) -> Price {
    if let Some(key) = matching_catalog_key(model, entries) {
        return entries.get(&key).cloned().unwrap_or_default();
    }
    let builtin = builtin_pricing();
    for variant in match_variants(model) {
        if let Some(price) = builtin.get(&variant) {
            return price.clone();
        }
    }
    builtin.get("default").cloned().unwrap_or_default()
}

fn matching_catalog_key(model: &str, entries: &BTreeMap<String, Price>) -> Option<String> {
    match_variants(model)
        .into_iter()
        .find(|variant| entries.contains_key(variant))
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct PricingOverrides {
    #[serde(default)]
    prices: BTreeMap<String, Price>,
    #[serde(default)]
    aliases: BTreeMap<String, String>,
}

/// Parsed, validated override payload: (model → price, alias → model).
type PricingOverrideMaps = (BTreeMap<String, Price>, BTreeMap<String, String>);

fn load_pricing_overrides() -> Option<PricingOverrideMaps> {
    let path = std::env::var_os("AGENTTRACE_PRICING_FILE").map(PathBuf::from)?;
    match std::fs::read(&path) {
        Err(err) => {
            record_pricing_override_failure(&path, format!("cannot read: {err}"));
            None
        }
        Ok(raw) => match parse_pricing_overrides(&raw) {
            Ok((prices, aliases)) => {
                let models = prices.len();
                let alias_count = aliases.len();
                let _ = PRICING_OVERRIDE_STATUS.set(PricingOverrideStatus::Applied {
                    path,
                    models,
                    aliases: alias_count,
                });
                Some((prices, aliases))
            }
            Err(reason) => {
                record_pricing_override_failure(&path, reason);
                None
            }
        },
    }
}

/// A requested override file was rejected. Overrides are inactive (the
/// bundled catalog keeps working), but the rejection is loud exactly
/// once per process and `pricing_source()`/`--test-match` report it, so
/// no artifact can quietly compute costs under the wrong assumption.
fn record_pricing_override_failure(path: &Path, reason: String) {
    eprintln!(
        "agenttrace: AGENTTRACE_PRICING_FILE ignored: {}: {reason}",
        path.display()
    );
    let _ = PRICING_OVERRIDE_STATUS.set(PricingOverrideStatus::Failed {
        path: path.to_path_buf(),
        reason,
    });
}

fn parse_pricing_overrides(raw: &[u8]) -> Result<PricingOverrideMaps, String> {
    let overrides: PricingOverrides =
        serde_json::from_slice(raw).map_err(|err| format!("invalid JSON: {err}"))?;
    let mut prices = BTreeMap::new();
    for (model, price) in overrides.prices {
        validate_override_price(&model, &price)?;
        prices.insert(normalize_model(&model), price);
    }
    let aliases = overrides
        .aliases
        .into_iter()
        .map(|(alias, model)| (normalize_model(&alias), normalize_model(&model)))
        .collect();
    Ok((prices, aliases))
}

/// Rates must be finite and non-negative before they can touch costing.
/// Mirrors the `convert_litellm` non-finite guard, but for user-supplied
/// files the rejection is named (model + field) instead of a silent skip:
/// a typo like `-3` used to flow straight into a negative session cost.
fn validate_override_price(model: &str, price: &Price) -> Result<(), String> {
    for (field, value) in [
        ("input", price.input),
        ("output", price.output),
        ("cw", price.cw),
        ("cr", price.cr),
    ] {
        if !value.is_finite() {
            return Err(format!(
                "model {model:?}: {field} rate is not finite ({value})"
            ));
        }
        if value < 0.0 {
            return Err(format!(
                "model {model:?}: {field} rate is negative ({value})"
            ));
        }
    }
    Ok(())
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

/// Catalog key -> price entries plus the model-key -> provider-label
/// mapping harvested from the same source rows (rm-245). The provider
/// label travels with the winning rate entry - same priority
/// selection, same key - so `provider_for` and `lookup_price` can never
/// disagree about which catalog row served a model.
struct ConvertedCatalog {
    entries: BTreeMap<String, Price>,
    providers: BTreeMap<String, String>,
}

fn convert_litellm(raw: &[u8]) -> ConvertedCatalog {
    let Ok(Value::Object(source)) = serde_json::from_slice::<Value>(raw) else {
        return ConvertedCatalog {
            entries: BTreeMap::new(),
            providers: BTreeMap::new(),
        };
    };
    let mut selected: BTreeMap<String, (i32, Price, String)> = BTreeMap::new();
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
            // Vendor context window and deprecation date ride along
            // when the source carries them (rm-231 / rm-419); zero or
            // empty values normalize to absent.
            max_input_tokens: model.max_input_tokens.filter(|window| *window > 0),
            deprecation_date: model
                .deprecation_date
                .as_deref()
                .map(str::trim)
                .filter(|date| !date.is_empty())
                .map(str::to_string),
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
            Some((existing, _, _)) if *existing >= priority => {}
            _ => {
                selected.insert(normalized, (priority, price, model.provider));
            }
        }
    }
    let mut entries = BTreeMap::new();
    let mut providers = BTreeMap::new();
    for (name, (_, price, provider)) in selected {
        providers.insert(name.clone(), provider);
        entries.insert(name, price);
    }
    ConvertedCatalog { entries, providers }
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
                ..Default::default()
            },
        ),
        (
            "claude-opus-4.6",
            Price {
                input: 5.0,
                output: 25.0,
                cw: 6.25,
                cr: 0.50,
                ..Default::default()
            },
        ),
        (
            "claude-opus-4-7",
            Price {
                input: 5.0,
                output: 25.0,
                cw: 6.25,
                cr: 0.50,
                ..Default::default()
            },
        ),
        (
            "claude-opus-4-6",
            Price {
                input: 5.0,
                output: 25.0,
                cw: 6.25,
                cr: 0.50,
                ..Default::default()
            },
        ),
        (
            "claude-opus-4.5",
            Price {
                input: 5.0,
                output: 25.0,
                cw: 6.25,
                cr: 0.50,
                ..Default::default()
            },
        ),
        (
            "claude-opus-4",
            Price {
                input: 15.0,
                output: 75.0,
                cw: 18.75,
                cr: 1.50,
                ..Default::default()
            },
        ),
        (
            "claude-sonnet-4.6",
            Price {
                input: 3.0,
                output: 15.0,
                cw: 3.75,
                cr: 0.30,
                ..Default::default()
            },
        ),
        (
            "claude-sonnet-4-6",
            Price {
                input: 3.0,
                output: 15.0,
                cw: 3.75,
                cr: 0.30,
                ..Default::default()
            },
        ),
        (
            "claude-sonnet-4.5",
            Price {
                input: 3.0,
                output: 15.0,
                cw: 3.75,
                cr: 0.30,
                ..Default::default()
            },
        ),
        (
            "claude-sonnet-4-5",
            Price {
                input: 3.0,
                output: 15.0,
                cw: 3.75,
                cr: 0.30,
                ..Default::default()
            },
        ),
        (
            "claude-sonnet-4",
            Price {
                input: 3.0,
                output: 15.0,
                cw: 3.75,
                cr: 0.30,
                ..Default::default()
            },
        ),
        (
            "claude-haiku-4-5",
            Price {
                input: 1.0,
                output: 5.0,
                cw: 1.25,
                cr: 0.10,
                ..Default::default()
            },
        ),
        (
            "claude-haiku-4.5",
            Price {
                input: 1.0,
                output: 5.0,
                cw: 1.25,
                cr: 0.10,
                ..Default::default()
            },
        ),
        (
            "claude-haiku-3.5",
            Price {
                input: 0.80,
                output: 4.0,
                cw: 1.0,
                cr: 0.08,
                ..Default::default()
            },
        ),
        (
            "gemini-3.1-pro-preview",
            Price {
                input: 2.0,
                output: 12.0,
                cw: 0.0,
                cr: 0.20,
                ..Default::default()
            },
        ),
        (
            "gemini-3-flash-preview",
            Price {
                input: 0.5,
                output: 3.0,
                cw: 0.0,
                cr: 0.05,
                ..Default::default()
            },
        ),
        (
            "gemini-2.5-pro",
            Price {
                input: 1.25,
                output: 10.0,
                cw: 0.0,
                cr: 0.0,
                ..Default::default()
            },
        ),
        (
            "gemini-2.5-flash",
            Price {
                input: 0.15,
                output: 0.60,
                cw: 0.0,
                cr: 0.0,
                ..Default::default()
            },
        ),
        (
            "gpt-5.5",
            Price {
                input: 5.0,
                output: 30.0,
                cw: 0.0,
                cr: 0.50,
                ..Default::default()
            },
        ),
        (
            "gpt-5.4",
            Price {
                input: 2.5,
                output: 15.0,
                cw: 0.0,
                cr: 0.25,
                ..Default::default()
            },
        ),
        (
            "pa/gpt-5.4",
            Price {
                input: 0.0,
                output: 0.0,
                cw: 0.0,
                cr: 0.0,
                ..Default::default()
            },
        ),
        (
            "gpt-5.4-mini",
            Price {
                input: 0.75,
                output: 4.5,
                cw: 0.0,
                cr: 0.075,
                ..Default::default()
            },
        ),
        (
            "gpt-5.3-codex",
            Price {
                input: 1.75,
                output: 14.0,
                cw: 0.0,
                cr: 0.175,
                ..Default::default()
            },
        ),
        (
            "gpt-5.2-codex",
            Price {
                input: 1.75,
                output: 14.0,
                cw: 0.0,
                cr: 0.175,
                ..Default::default()
            },
        ),
        (
            "gpt-5.1",
            Price {
                input: 1.25,
                output: 10.0,
                cw: 0.0,
                cr: 0.0,
                ..Default::default()
            },
        ),
        (
            "gpt-5.1-mini",
            Price {
                input: 0.25,
                output: 2.0,
                cw: 0.0,
                cr: 0.0,
                ..Default::default()
            },
        ),
        (
            "gpt-5.1-codex-mini",
            Price {
                input: 0.25,
                output: 2.0,
                cw: 0.0,
                cr: 0.025,
                ..Default::default()
            },
        ),
        (
            "gpt-4.1",
            Price {
                input: 2.0,
                output: 8.0,
                cw: 0.0,
                cr: 0.0,
                ..Default::default()
            },
        ),
        (
            "gpt-4.1-mini",
            Price {
                input: 0.40,
                output: 1.60,
                cw: 0.0,
                cr: 0.0,
                ..Default::default()
            },
        ),
        (
            "gpt-4.1-nano",
            Price {
                input: 0.10,
                output: 0.40,
                cw: 0.0,
                cr: 0.0,
                ..Default::default()
            },
        ),
        (
            "deepseek-v4-pro",
            Price {
                input: 0.435,
                output: 0.87,
                cw: 0.0,
                cr: 0.003625,
                ..Default::default()
            },
        ),
        (
            "deepseek-v4-flash",
            Price {
                input: 0.14,
                output: 0.28,
                cw: 0.0,
                cr: 0.0028,
                ..Default::default()
            },
        ),
        (
            "deepseek-chat",
            Price {
                input: 0.27,
                output: 1.10,
                cw: 0.07,
                cr: 0.014,
                ..Default::default()
            },
        ),
        (
            "deepseek-reasoner",
            Price {
                input: 0.55,
                output: 2.19,
                cw: 0.14,
                cr: 0.028,
                ..Default::default()
            },
        ),
        (
            "glm-5",
            Price {
                input: 1.0,
                output: 3.20,
                cw: 0.0,
                cr: 0.20,
                ..Default::default()
            },
        ),
        (
            "glm-5-turbo",
            Price {
                input: 1.20,
                output: 4.0,
                cw: 0.0,
                cr: 0.24,
                ..Default::default()
            },
        ),
        (
            "glm-5.1",
            Price {
                input: 1.40,
                output: 4.40,
                cw: 0.0,
                cr: 0.26,
                ..Default::default()
            },
        ),
        (
            "kimi-k2.5",
            Price {
                input: 0.60,
                output: 3.0,
                cw: 0.0,
                cr: 0.10,
                ..Default::default()
            },
        ),
        (
            "kimi-k2.6",
            Price {
                input: 0.95,
                output: 4.0,
                cw: 0.0,
                cr: 0.16,
                ..Default::default()
            },
        ),
        (
            "mimo-v2-pro",
            Price {
                input: 0.10,
                output: 0.30,
                cw: 0.0,
                cr: 0.02,
                ..Default::default()
            },
        ),
        (
            "mimo-v2.5-pro",
            Price {
                input: 0.0,
                output: 0.0,
                cw: 0.0,
                cr: 0.0,
                ..Default::default()
            },
        ),
        (
            "minimax-2.5",
            Price {
                input: 0.30,
                output: 2.40,
                cw: 0.375,
                cr: 0.03,
                ..Default::default()
            },
        ),
        (
            "minimax-2.7",
            Price {
                input: 0.30,
                output: 2.40,
                cw: 0.375,
                cr: 0.03,
                ..Default::default()
            },
        ),
        (
            "minimax-2.7-highspeed",
            Price {
                input: 0.30,
                output: 2.40,
                cw: 0.375,
                cr: 0.03,
                ..Default::default()
            },
        ),
        (
            "minimax-m2.5",
            Price {
                input: 0.30,
                output: 1.20,
                cw: 0.375,
                cr: 0.03,
                ..Default::default()
            },
        ),
        (
            "minimax-m2.5-free",
            Price {
                input: 0.30,
                output: 2.40,
                cw: 0.375,
                cr: 0.03,
                ..Default::default()
            },
        ),
        (
            "minimax-m2.7",
            Price {
                input: 0.30,
                output: 2.40,
                cw: 0.375,
                cr: 0.03,
                ..Default::default()
            },
        ),
        (
            "qwen/qwen3.6-plus-04-02:free",
            Price {
                input: 0.325,
                output: 1.95,
                cw: 0.40625,
                cr: 0.0,
                ..Default::default()
            },
        ),
        (
            "qwen3.6-plus",
            Price {
                input: 0.325,
                output: 1.95,
                cw: 0.40625,
                cr: 0.0,
                ..Default::default()
            },
        ),
        (
            "qwen3.5-plus",
            Price {
                input: 0.40,
                output: 2.40,
                cw: 0.0,
                cr: 0.0,
                ..Default::default()
            },
        ),
        (
            "qwen3.6:35b-a3b-coding-nvfp4",
            Price {
                input: 0.0,
                output: 0.0,
                cw: 0.0,
                cr: 0.0,
                ..Default::default()
            },
        ),
        (
            "doubao-seed-2-0-pro",
            Price {
                input: 0.0,
                output: 0.0,
                cw: 0.0,
                cr: 0.0,
                ..Default::default()
            },
        ),
        (
            "stepfun/step-3.5-flash:free",
            Price {
                input: 0.0,
                output: 0.0,
                cw: 0.0,
                cr: 0.0,
                ..Default::default()
            },
        ),
        (
            "grok-code-fast-1",
            Price {
                input: 0.20,
                output: 1.50,
                cw: 0.0,
                cr: 0.02,
                ..Default::default()
            },
        ),
        (
            "x-ai/grok-code-fast-1",
            Price {
                input: 0.20,
                output: 1.50,
                cw: 0.0,
                cr: 0.02,
                ..Default::default()
            },
        ),
        (
            "<synthetic>",
            Price {
                input: 0.0,
                output: 0.0,
                cw: 0.0,
                cr: 0.0,
                ..Default::default()
            },
        ),
        (
            "grok-3",
            Price {
                input: 3.0,
                output: 15.0,
                cw: 0.0,
                cr: 0.0,
                ..Default::default()
            },
        ),
        (
            "default",
            Price {
                input: 3.0,
                output: 15.0,
                cw: 0.0,
                cr: 0.0,
                ..Default::default()
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
    use std::io::Cursor;

    #[test]
    fn pricing_download_cap_rejects_oversized_bodies() {
        // rm-167: the catalog fetch previously read the entire response
        // into memory unbounded; a redirected or hostile endpoint could
        // stream gigabytes. The cap reads at most cap+1 bytes and errors
        // on exceedance instead of buffering forever.
        let at_cap = read_body_capped(Cursor::new(vec![0u8; 64]), 64).expect("at-cap body ok");
        assert_eq!(at_cap.len(), 64);
        let over_cap = read_body_capped(Cursor::new(vec![0u8; 65]), 64);
        let message = over_cap.expect_err("over-cap body must fail").to_string();
        assert!(
            message.contains("download cap"),
            "error must name the cap, got: {message}"
        );
    }

    #[test]
    fn pricing_cache_write_stamps_provenance_sidecar() {
        // rm-167: every downloaded cache carries a meta sidecar recording
        // origin URL, fetch time, and size so the cache's provenance is
        // answerable from disk without network access.
        let root = std::env::temp_dir().join(format!(
            "agenttrace-pricing-meta-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        std::fs::create_dir_all(&root).expect("create temp dir");
        let path = root.join("pricing.json");
        write_pricing_cache_at(&path, "{\"stub\":true}").expect("write cache");
        let meta_raw =
            std::fs::read(path.with_extension("meta.json")).expect("provenance sidecar exists");
        let meta: PricingCacheMeta =
            serde_json::from_slice(&meta_raw).expect("sidecar is valid JSON");
        assert_eq!(meta.url, PRICING_URL);
        assert_eq!(meta.bytes, "{\"stub\":true}".len());
        assert!(meta.fetched_at_unix > 0, "fetch time is stamped");
        let _ = std::fs::remove_dir_all(root);
    }

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
    fn user_pricing_overrides_reject_invalid_json_with_reason() {
        // Cycle-4 B1 red-first: an unreadable or malformed override file
        // used to collapse into `None` via the `.ok()?` chain — zero
        // overrides, rc0, empty stderr, `--test-match` still advertising
        // "(bundled)". The rejection must now carry a reason.
        let err = parse_pricing_overrides(b"not json at all")
            .expect_err("malformed JSON must be rejected");
        assert!(
            err.contains("invalid JSON"),
            "reason should name the JSON failure: {err}"
        );
    }

    #[test]
    fn user_pricing_overrides_reject_unknown_schema_keys() {
        // The natural trap from the cycle-4 assessment: pasting a LiteLLM
        // snapshot (whose top level is model names, not {prices, aliases})
        // used to deserialize into an empty-but-valid PricingOverrides —
        // the file "loaded" and did nothing. deny_unknown_fields turns
        // that into a named rejection.
        let err = parse_pricing_overrides(
            br#"{"gpt-4o": {"input_cost_per_token": 2.5e-6, "mode": "chat"}}"#,
        )
        .expect_err("wrong-schema keys must be rejected");
        assert!(
            err.contains("unknown field") && err.contains("gpt-4o"),
            "reason should name the offending key: {err}"
        );
    }

    #[test]
    fn user_pricing_overrides_reject_negative_rates_naming_model_and_field() {
        // A sign typo used to flow straight into costing (corroborated
        // live: a -520 USD session cost). Rejection must name model +
        // field so the user can find the bad line.
        let err = parse_pricing_overrides(
            br#"{"prices":{"my-model":{"input":-5,"output":1,"cw":0,"cr":0}}}"#,
        )
        .expect_err("negative rates must be rejected");
        assert!(
            err.contains("my-model") && err.contains("input") && err.contains("negative"),
            "reason should name model, field, and defect: {err}"
        );
        // All-clean sibling in the same file must still be fine.
        parse_pricing_overrides(br#"{"prices":{"my-model":{"input":5,"output":1,"cw":0,"cr":0}}}"#)
            .expect("non-negative rates are accepted");
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
        // Shared crate-wide env lock (see lib.rs `test_env`): the
        // session-cache and statusline tests mutate the same process
        // environment from sibling modules.
        static SEQ: AtomicUsize = AtomicUsize::new(0);
        let _guard = crate::test_env::lock_env();
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
            providers: BTreeMap::new(),
            reference_date: None,
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
            providers: BTreeMap::new(),
            reference_date: None,
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
    fn pricing_source_alias_arm_discloses_deprecation_like_the_plain_arm() {
        // rm-511 golden: all three arms of pricing_source_for_catalog
        // carry the retirement disclosure when the catalog model is past
        // its vendor deprecation date (per the catalog's own vintage).
        // The alias arm prices at catalog rates — the override file
        // supplies identity only — so its clause is the plain arm's,
        // "(rate unverified)" included; the direct arm (rm-419) keeps
        // its qualifier-free clause because that rate is the user's own.
        let retired = || Price {
            deprecation_date: Some("2025-01-01".to_string()),
            ..Price::default()
        };
        let catalog = PricingCatalog {
            entries: BTreeMap::from([("retired-model".to_string(), retired())]),
            aliases: BTreeMap::from([("my-alias".to_string(), "retired-model".to_string())]),
            source: "cache".to_string(),
            providers: BTreeMap::new(),
            reference_date: Some(2026_1004),
        };
        // Plain catalog arm.
        assert_eq!(
            pricing_source_for_catalog("retired-model", &catalog, &BTreeSet::new()),
            "LiteLLM (cached catalog); 1 of 1 priced models past vendor deprecation; \
             model deprecated 2025-01-01 (rate unverified)"
        );
        // Direct user-rate arm keeps the rm-419 clause.
        let overrides = BTreeSet::from(["retired-model".to_string()]);
        assert_eq!(
            pricing_source_for_catalog("retired-model", &catalog, &overrides),
            "user override; model deprecated 2025-01-01"
        );
        // Alias arm (the rm-511 fix): same clause as the plain arm.
        assert_eq!(
            pricing_source_for_catalog("my-alias", &catalog, &BTreeSet::new()),
            "LiteLLM (cached catalog); 1 of 1 priced models past vendor deprecation \
             via user override alias; model deprecated 2025-01-01 (rate unverified)"
        );
        // A non-deprecated alias target stays qualifier-free, and a
        // catalog without a vintage can declare nothing deprecated.
        let fresh = PricingCatalog {
            entries: BTreeMap::from([("fresh-model".to_string(), Price::default())]),
            aliases: BTreeMap::from([("my-alias".to_string(), "fresh-model".to_string())]),
            source: "cache".to_string(),
            providers: BTreeMap::new(),
            reference_date: Some(2026_1004),
        };
        assert_eq!(
            pricing_source_for_catalog("my-alias", &fresh, &BTreeSet::new()),
            "LiteLLM (cached catalog) via user override alias"
        );
        let undated = PricingCatalog {
            entries: BTreeMap::from([("fresh-model".to_string(), Price::default())]),
            aliases: BTreeMap::from([("my-alias".to_string(), "fresh-model".to_string())]),
            source: "cache".to_string(),
            providers: BTreeMap::new(),
            reference_date: None,
        };
        assert_eq!(
            pricing_source_for_catalog("my-alias", &undated, &BTreeSet::new()),
            "LiteLLM (cached catalog) via user override alias"
        );
    }

    #[test]
    fn override_alias_file_discloses_retired_target_end_to_end() {
        // rm-511 fixture: an AGENTTRACE_PRICING_FILE that maps an alias
        // onto a vendor-retired catalog model must surface the
        // retirement disclosure end-to-end — load_pricing_overrides ->
        // aliases extended into the catalog -> pricing_source_for_catalog.
        let root = std::env::temp_dir().join(format!(
            "agenttrace-pricing-alias-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        std::fs::create_dir_all(&root).expect("create temp dir");
        let override_file = root.join("pricing.json");
        std::fs::write(
            &override_file,
            serde_json::json!({
                "aliases": {"my-alias": "wide-retired"}
            })
            .to_string(),
        )
        .expect("write override file");

        let mut catalog = PricingCatalog {
            entries: BTreeMap::from([
                (
                    "wide-retired".to_string(),
                    Price {
                        input: 3.0,
                        output: 15.0,
                        max_input_tokens: Some(1_000_000),
                        deprecation_date: Some("2025-01-01".to_string()),
                        ..Price::default()
                    },
                ),
                ("current-model".to_string(), Price::default()),
            ]),
            aliases: BTreeMap::new(),
            source: "cache".to_string(),
            providers: BTreeMap::new(),
            reference_date: Some(2026_1004),
        };

        let _env = crate::test_env::lock_env();
        let prior_file = std::env::var_os("AGENTTRACE_PRICING_FILE");
        std::env::set_var("AGENTTRACE_PRICING_FILE", &override_file);
        let override_models = apply_pricing_overrides(&mut catalog);
        match prior_file {
            Some(value) => std::env::set_var("AGENTTRACE_PRICING_FILE", value),
            None => std::env::remove_var("AGENTTRACE_PRICING_FILE"),
        }
        drop(_env);
        let _ = std::fs::remove_dir_all(root);

        assert!(
            override_models.is_empty(),
            "an aliases-only file overrides no rates directly"
        );
        assert_eq!(
            pricing_source_for_catalog("my-alias", &catalog, &override_models),
            "LiteLLM (cached catalog); 1 of 2 priced models past vendor deprecation \
             via user override alias; model deprecated 2025-01-01 (rate unverified)",
            "an alias onto a retired model keeps the retirement disclosure \
             (pricing_source_reported the catalog source with no clause before rm-511)"
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
            catalog.entries.contains_key("finite-model"),
            "finite entries survive the conversion"
        );
        assert!(
            !catalog.entries.contains_key("poisoned-model"),
            "entries whose scaled price is non-finite are dropped"
        );
    }

    #[test]
    fn convert_litellm_preserves_the_provider_per_winning_entry() {
        // rm-245: provider attribution must ride with the winning rate
        // entry, not be re-derived from name prefixes. Whatever key
        // normalization produces, the provider map must cover exactly
        // the priced entries and carry each winning row's label.
        let fixture = serde_json::json!({
            "finite-model": {
                "input_cost_per_token": 0.000003,
                "output_cost_per_token": 0.000015,
                "mode": "chat",
                "litellm_provider": "finite"
            },
            "vendor-x/model-y": {
                "input_cost_per_token": 0.000003,
                "output_cost_per_token": 0.000015,
                "mode": "chat",
                "litellm_provider": "vendor-x"
            }
        });
        let converted = convert_litellm(
            serde_json::to_vec(&fixture)
                .expect("serialize catalog")
                .as_slice(),
        );
        let entry_keys: Vec<&String> = converted.entries.keys().collect();
        let provider_keys: Vec<&String> = converted.providers.keys().collect();
        assert_eq!(
            entry_keys, provider_keys,
            "price keys and provider keys must cover the same catalog rows"
        );
        assert_eq!(
            converted.providers.get("finite-model").map(String::as_str),
            Some("finite"),
            "provider label must round-trip with its entry"
        );
        let vendored_key = converted
            .providers
            .keys()
            .find(|key| key.contains("model-y"))
            .expect("vendored entry survives conversion");
        assert_eq!(
            converted.providers.get(vendored_key).map(String::as_str),
            Some("vendor-x"),
            "provider label must round-trip with its entry"
        );
    }

    /// rm-404: PRIVACY.md must enumerate the pricing download URL, derived
    /// from this module's own `PRICING_URL` const so the pin cannot drift
    /// from the code that performs the download (mirrors rm-086's
    /// `privacy_disclosure_lists_every_artifact` pin in session_cache).
    #[test]
    fn privacy_disclosure_names_the_pricing_download_url() {
        let privacy =
            std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../../PRIVACY.md"))
                .expect("PRIVACY.md readable");
        assert!(
            privacy.contains(PRICING_URL),
            "PRIVACY.md must enumerate the --update-pricing download URL {PRICING_URL}"
        );
        assert!(
            !privacy.contains("only exception"),
            "PRIVACY.md must not claim --update-pricing is the only network touch (--fetch upstream also touches the network)"
        );
    }

    #[test]
    fn bundled_snapshot_carries_context_windows_and_deprecation_dates() {
        // rm-231 / rm-419 ingestion: the trimmed snapshot must keep the
        // vendor context window (the source file is literally named
        // model_prices_and_context_window.json) and the deprecation
        // date. Zero-value windows and blank dates normalize to absent.
        let catalog = fallback_catalog();
        let with_window = catalog
            .entries
            .values()
            .filter(|price| price.max_input_tokens.is_some())
            .count();
        let with_deprecation = catalog
            .entries
            .values()
            .filter(|price| price.deprecation_date.is_some())
            .count();
        // The conversion dedups provider-prefixed duplicates down to
        // one entry per normalized model name, so the thresholds sit
        // well under the raw file counts (2,990 / 463) while still
        // failing loudly if ingestion ever drops the fields again.
        assert!(
            with_window > 1_200,
            "expected the bulk of the priced catalog to carry windows, got {with_window}"
        );
        assert!(
            with_deprecation > 100,
            "expected a substantial deprecated-model cohort, got {with_deprecation}"
        );
        // The 1M-window default class (rm-231's motivating case).
        assert_eq!(
            catalog.entries["claude-sonnet-4-5"].max_input_tokens,
            Some(1_000_000)
        );
    }

    #[test]
    fn context_window_lookup_resolves_catalog_entries_and_misses_cleanly() {
        // rm-231: the denominator resolver. A catalog hit returns the
        // vendor window; an unknown model misses with None so the
        // caller can take the labeled fallback ladder. A zero window
        // (malformed source) must behave like an absent one.
        let catalog = PricingCatalog {
            entries: BTreeMap::from([
                (
                    "wide-model".to_string(),
                    Price {
                        max_input_tokens: Some(2_000_000),
                        ..Price::default()
                    },
                ),
                (
                    "zero-window-model".to_string(),
                    Price {
                        max_input_tokens: Some(0),
                        ..Price::default()
                    },
                ),
            ]),
            aliases: BTreeMap::from([("wide-alias".to_string(), "wide-model".to_string())]),
            source: "cache".to_string(),
            providers: BTreeMap::new(),
            reference_date: None,
        };
        assert_eq!(
            lookup_context_window_in("wide-model", &catalog),
            Some(2_000_000)
        );
        assert_eq!(
            lookup_context_window_in("wide-alias", &catalog),
            Some(2_000_000)
        );
        assert_eq!(
            lookup_context_window_in("wide-model-2", &catalog),
            Some(2_000_000),
            "minor-version variant matching must still apply to window lookups"
        );
        assert_eq!(
            lookup_context_window_in("zero-window-model", &catalog),
            None
        );
        assert_eq!(lookup_context_window_in("unknown-model", &catalog), None);
    }

    #[test]
    fn deprecated_models_disclose_their_rate_status() {
        // rm-419: a session priced on a model the vendor has already
        // retired (per the catalog's own vintage) says so at the rate's
        // provenance line; a future-dated deprecation does not.
        let catalog = PricingCatalog {
            entries: BTreeMap::from([
                (
                    "retired-model".to_string(),
                    Price {
                        deprecation_date: Some("2025-01-01".to_string()),
                        ..Price::default()
                    },
                ),
                (
                    "sunset-later-model".to_string(),
                    Price {
                        deprecation_date: Some("2026-11-30".to_string()),
                        ..Price::default()
                    },
                ),
            ]),
            aliases: BTreeMap::new(),
            source: "snapshot".to_string(),
            providers: BTreeMap::new(),
            reference_date: Some(2026_1004),
        };
        let overrides = BTreeSet::new();
        let retired = pricing_source_for_catalog("retired-model", &catalog, &overrides);
        // The per-model disclosure composes with the aggregate census
        // the base label already carries (both are catalog properties).
        assert_eq!(
            retired,
            format!(
                "LiteLLM snapshot {PRICING_SNAPSHOT_DATE} (bundled); 1 of 2 priced models past vendor deprecation; model deprecated 2025-01-01 (rate unverified)"
            )
        );
        let active = pricing_source_for_catalog("sunset-later-model", &catalog, &overrides);
        assert_eq!(
            active,
            format!(
                "LiteLLM snapshot {PRICING_SNAPSHOT_DATE} (bundled); 1 of 2 priced models past vendor deprecation"
            ),
            "future-dated deprecations must not mark the session's own model"
        );
    }

    #[test]
    fn catalog_source_discloses_the_deprecation_census() {
        // rm-419: the aggregate provenance line discloses how much of
        // the priced set the vendor has already retired, anchored to
        // the catalog's own vintage. Catalogs without a vintage (the
        // built-in fallback) keep the legacy label byte-identical.
        let catalog = PricingCatalog {
            entries: BTreeMap::from([
                (
                    "retired-model".to_string(),
                    Price {
                        deprecation_date: Some("2025-01-01".to_string()),
                        ..Price::default()
                    },
                ),
                ("current-model".to_string(), Price::default()),
            ]),
            aliases: BTreeMap::new(),
            source: "cache".to_string(),
            providers: BTreeMap::new(),
            reference_date: Some(2026_1004),
        };
        assert_eq!(
            catalog_source(&catalog),
            "LiteLLM (cached catalog); 1 of 2 priced models past vendor deprecation"
        );
        let unvintaged = PricingCatalog {
            entries: catalog.entries.clone(),
            aliases: BTreeMap::new(),
            source: "builtin".to_string(),
            providers: BTreeMap::new(),
            reference_date: None,
        };
        assert_eq!(
            catalog_source(&unvintaged),
            "built-in fallback (run --update-pricing for the latest catalog)"
        );
    }

    #[test]
    fn catalog_identity_is_stable_and_content_sensitive() {
        // rm-196: the session cache keys its freshness on this digest.
        // It must be stable for identical content, change on any price
        // or alias change, and deliberately ignore the source label —
        // cache vs cache(stale) flips with file age, not content, so
        // counting it would trigger pointless full re-prices.
        let base = PricingCatalog {
            entries: BTreeMap::from([(
                "model-a".to_string(),
                Price {
                    input: 3.0,
                    output: 15.0,
                    ..Price::default()
                },
            )]),
            aliases: BTreeMap::new(),
            source: "cache".to_string(),
            providers: BTreeMap::new(),
            reference_date: None,
        };
        let same_content = PricingCatalog {
            source: "cache(stale)".to_string(),
            reference_date: Some(2026_1004),
            ..base.clone()
        };
        assert_eq!(
            catalog_identity_of(&base),
            catalog_identity_of(&same_content),
            "source and reference_date must not affect the identity"
        );
        let repriced = PricingCatalog {
            entries: BTreeMap::from([(
                "model-a".to_string(),
                Price {
                    input: 3.5,
                    output: 15.0,
                    ..Price::default()
                },
            )]),
            ..base.clone()
        };
        assert_ne!(catalog_identity_of(&base), catalog_identity_of(&repriced));
        let aliased = PricingCatalog {
            aliases: BTreeMap::from([("alias".to_string(), "model-a".to_string())]),
            ..base.clone()
        };
        assert_ne!(catalog_identity_of(&base), catalog_identity_of(&aliased));
        let windowed = PricingCatalog {
            entries: BTreeMap::from([(
                "model-a".to_string(),
                Price {
                    input: 3.0,
                    output: 15.0,
                    max_input_tokens: Some(200_000),
                    ..Price::default()
                },
            )]),
            ..base.clone()
        };
        assert_ne!(catalog_identity_of(&base), catalog_identity_of(&windowed));
    }

    #[test]
    fn bundled_snapshot_deprecation_census_is_deterministic() {
        // rm-419: the census anchors to the snapshot's own date, so it
        // is a property of the vendored file and stable across calls
        // and environments.
        let (count, oldest) = bundled_snapshot_deprecated();
        assert!(count > 0, "the 2026-10 snapshot carries retired models");
        let oldest = oldest.expect("a non-empty census has an oldest date");
        assert_eq!(oldest.len(), 10, "ISO YYYY-MM-DD shape: {oldest}");
        assert!(oldest.as_str() <= PRICING_SNAPSHOT_DATE);
        assert_eq!(
            (count, Some(oldest)),
            bundled_snapshot_deprecated(),
            "the census must be idempotent"
        );
    }

    #[test]
    fn price_overrides_backfill_context_window_and_deprecation() {
        // rm-231/rm-419 regression (review F3): a price override used
        // to replace the whole Price, so correcting a rate on a
        // 1M-window deprecated model regressed it to the 200k
        // substring ladder and dropped the deprecation disclosure —
        // exactly the defect the batch fixes, reintroduced for
        // override users. Partial overrides now backfill the
        // descriptive fields; an explicit zero window still clears it.
        let root = std::env::temp_dir().join(format!(
            "agenttrace-override-backfill-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        std::fs::create_dir_all(&root).expect("create temp dir");
        let override_file = root.join("pricing.json");
        std::fs::write(
            &override_file,
            serde_json::json!({
                "prices": {
                    "wide-retired": {"input": 4.0, "output": 20.0, "cw": 5.0, "cr": 0.5},
                    "explicitly-clear-window": {"input": 1.0, "output": 2.0, "cw": 0.0, "cr": 0.0, "max_input_tokens": 0},
                    "brand-new-model": {"input": 2.0, "output": 4.0, "cw": 0.0, "cr": 0.0}
                }
            })
            .to_string(),
        )
        .expect("write override file");

        let mut catalog = PricingCatalog {
            entries: BTreeMap::from([
                (
                    "wide-retired".to_string(),
                    Price {
                        input: 3.0,
                        output: 15.0,
                        max_input_tokens: Some(1_000_000),
                        deprecation_date: Some("2025-01-01".to_string()),
                        ..Price::default()
                    },
                ),
                (
                    "explicitly-clear-window".to_string(),
                    Price {
                        input: 3.0,
                        output: 15.0,
                        max_input_tokens: Some(500_000),
                        ..Price::default()
                    },
                ),
            ]),
            aliases: BTreeMap::new(),
            source: "cache".to_string(),
            providers: BTreeMap::new(),
            reference_date: Some(2026_1004),
        };

        let _env = crate::test_env::lock_env();
        let prior_file = std::env::var_os("AGENTTRACE_PRICING_FILE");
        std::env::set_var("AGENTTRACE_PRICING_FILE", &override_file);
        let override_models = apply_pricing_overrides(&mut catalog);
        match prior_file {
            Some(value) => std::env::set_var("AGENTTRACE_PRICING_FILE", value),
            None => std::env::remove_var("AGENTTRACE_PRICING_FILE"),
        }
        drop(_env);
        let _ = std::fs::remove_dir_all(root);

        let backfilled = &catalog.entries["wide-retired"];
        assert_eq!(backfilled.input, 4.0, "the override rate must win");
        assert_eq!(backfilled.output, 20.0);
        assert_eq!(
            backfilled.max_input_tokens,
            Some(1_000_000),
            "a rate-only override must not regress the vendor window"
        );
        assert_eq!(
            backfilled.deprecation_date,
            Some("2025-01-01".to_string()),
            "a rate-only override must not drop the deprecation date"
        );
        assert_eq!(
            lookup_context_window_in("wide-retired", &catalog),
            Some(1_000_000),
            "the end-to-end rm-231 denominator must survive an override"
        );
        assert_eq!(
            pricing_source_for_catalog("wide-retired", &catalog, &override_models),
            "user override; model deprecated 2025-01-01",
            "an overridden retired model keeps its retirement disclosure"
        );
        assert_eq!(
            catalog.entries["explicitly-clear-window"].max_input_tokens,
            Some(0),
            "an explicit zero window is an intentional clear, never backfilled"
        );
        assert_eq!(
            lookup_context_window_in("explicitly-clear-window", &catalog),
            None,
            "a cleared window falls back to the labeled ladder"
        );
        assert_eq!(
            catalog.entries["brand-new-model"].max_input_tokens, None,
            "an override for an unknown model has nothing to backfill from"
        );
    }
}
