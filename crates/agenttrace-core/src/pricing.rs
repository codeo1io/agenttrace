use crate::cache_dir::user_cache_dir;
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
/// (the rm-588 never-drop union: every key the committed bundle priced stays
/// priced unless `scripts/pricing/snapshot-drops.txt` carries a dated manual
/// drop note) and keep `PRICING_SNAPSHOT_DATE` in sync with the date it
/// prints.
const PRICING_SNAPSHOT_JSON: &str = include_str!("pricing_snapshot.json");
const PRICING_SNAPSHOT_DATE: &str = "2026-10-17";
const CACHE_MAX_AGE: Duration = Duration::from_secs(24 * 60 * 60);
static PRICING_CATALOG: OnceLock<PricingCatalog> = OnceLock::new();
static MODELS_DEV_CATALOG: OnceLock<ModelsDevCatalog> = OnceLock::new();
static MODELS_DEV_PARSE_FAILURE: OnceLock<String> = OnceLock::new();
static PRICING_OVERRIDE_MODELS: OnceLock<BTreeSet<String>> = OnceLock::new();
// rm-1426: exact keys of the ACTIVE catalog's raw origin (downloaded
// cache when one is served, else the bundled snapshot), captured when
// the catalog singleton loads — `--update-pricing` refreshes it in
// lockstep with the catalog it publishes.
static PRICING_RAW_KEYS: OnceLock<BTreeSet<String>> = OnceLock::new();
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
    /// Cache-write rate per million tokens (LiteLLM
    /// `cache_creation_input_token_cost`). `None` = the vendor row
    /// publishes no cache rate (rm-1508) — distinct from `Some(0.0)`,
    /// which is a published $0. Absent rates bill at the disclosed
    /// fallback (input × [`CACHE_WRITE_FALLBACK_MULTIPLE`]), never a
    /// silent $0.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cw: Option<f64>,
    /// Cache-read rate per million tokens (LiteLLM
    /// `cache_read_input_token_cost`). `None` = not published (billed
    /// at the disclosed input-rate fallback); `Some(0.0)` = published
    /// $0 and bills as $0 (rm-1508).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cr: Option<f64>,
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
    /// Flex-tier rates (LiteLLM `*_cost_per_token_flex`, rm-610): the
    /// rate a turn that ran on `service_tier: "flex"` bills at.
    /// Absent on most models and on all override files that do not set
    /// them; `skip` keeps serialized pricing output byte-identical for
    /// models without them. A flex turn of a model without flex rates
    /// bills at the standard rates above (published behavior,
    /// codeburn #1642) — never zero.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub input_flex: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output_flex: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cw_flex: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cr_flex: Option<f64>,
    /// Ultrafast-tier rates (LiteLLM `*_ultrafast`, rm-1028-lineage
    /// cycle-2 rider): the rate a turn that ran on
    /// `service_tier: "ultrafast"` bills at. Live as of 2026-10-12 on
    /// 8 LiteLLM keys (gpt-6-astra + gpt-6.1-sol families), selectable
    /// by users in opencode >= 1.19.0. Same semantics as flex: absent
    /// means "no published ultrafast rate" and an ultrafast turn of
    /// such a model bills at the standard rates — disclosed, never
    /// zero and never silently repriced. `_above_272k_tokens_ultrafast`
    /// variants are deliberately not ingested (no long-context tier
    /// extraction to pair them with — the rm-610 flex decision, held).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub input_ultrafast: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output_ultrafast: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cw_ultrafast: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cr_ultrafast: Option<f64>,
    /// Priority-tier rates (rm-1556, cycle-4): the rate a turn that
    /// ran on `service_tier: "priority"` bills at. Live as of
    /// 2026-10-17 on 275 bundled keys (gpt-5.5/5.6 sol/terra/luna,
    /// gpt-6 families, o-series, grok-5, glm/qwen/mini-max classes) at
    /// a consistent 2x of standard on the OpenAI families. Same
    /// semantics as flex/ultrafast: absent means "no published
    /// priority rate" and a priority turn of such a model bills at
    /// the standard rates — disclosed, never zero and never silently
    /// repriced. `_above_272k_tokens_priority` variants are
    /// deliberately not ingested (no long-context tier extraction to
    /// pair them with — the rm-610 flex decision, held for every tier
    /// family).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub input_priority: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output_priority: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cw_priority: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cr_priority: Option<f64>,
}

/// rm-1508: the cache-write fallback multiple — the vendor convention
/// cache-reporting vendors either publish explicitly or imply
/// (Anthropic's 1.25× input premium on 5-minute cache writes; OpenAI
/// writes cache at the bare input rate, which this bounds above).
/// Applied only when the catalog row publishes no cache-write rate at
/// all, and always disclosed via the `cache_rate_fallback`
/// disclosure counter and the report census arm.
pub const CACHE_WRITE_FALLBACK_MULTIPLE: f64 = 1.25;

impl Price {
    /// The cache-read rate a session bills at, and whether it is the
    /// rm-1508 absent-rate fallback (the input rate) rather than a
    /// rate the vendor published.
    pub fn cache_read_billed(&self) -> (f64, bool) {
        match self.cr {
            Some(rate) => (rate, false),
            None => (self.input, true),
        }
    }

    /// The cache-write rate a session bills at, and whether it is the
    /// rm-1508 absent-rate fallback (input ×
    /// [`CACHE_WRITE_FALLBACK_MULTIPLE`]).
    pub fn cache_write_billed(&self) -> (f64, bool) {
        match self.cw {
            Some(rate) => (rate, false),
            None => (self.input * CACHE_WRITE_FALLBACK_MULTIPLE, true),
        }
    }
}

/// rm-1508: whether pricing cache tokens at `price` bills any
/// absent-rate fallback — true only when the tokens to price are
/// nonzero AND the corresponding catalog rate is unpublished. This is
/// the disclosure trigger; the rates themselves come from
/// [`Price::cache_write_billed`]/[`Price::cache_read_billed`].
pub(crate) fn cache_fallback_fired(price: &Price, cache_write: i64, cache_read: i64) -> bool {
    (cache_write > 0 && price.cw.is_none()) || (cache_read > 0 && price.cr.is_none())
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
    // rm-1508: Option-typed so a row that publishes no cache rate
    // stays `None` through deserialize — distinct from an explicit
    // `0`, which is a published $0 and bills as $0. The old zero-fill
    // here was the ingest half of the conflation the generator's
    // `or 0` started.
    #[serde(default, rename = "cache_creation_input_token_cost")]
    cache_write_cost: Option<f64>,
    #[serde(default, rename = "cache_read_input_token_cost")]
    cache_read_cost: Option<f64>,
    #[serde(default)]
    mode: String,
    #[serde(default, rename = "litellm_provider")]
    provider: String,
    #[serde(default, rename = "max_input_tokens")]
    max_input_tokens: Option<u64>,
    #[serde(default, rename = "deprecation_date")]
    deprecation_date: Option<String>,
    // Flex-tier rates (rm-610). LiteLLM also publishes
    // `*_above_272k_tokens_flex` variants (long-context flex tiers);
    // they are deliberately NOT ingested — the fork has no
    // long-context tier extraction to pair them with.
    #[serde(default, rename = "input_cost_per_token_flex")]
    input_cost_flex: Option<f64>,
    #[serde(default, rename = "output_cost_per_token_flex")]
    output_cost_flex: Option<f64>,
    #[serde(default, rename = "cache_creation_input_token_cost_flex")]
    cache_write_cost_flex: Option<f64>,
    #[serde(default, rename = "cache_read_input_token_cost_flex")]
    cache_read_cost_flex: Option<f64>,
    // Ultrafast-tier rates (cycle-2 rider): same ingestion shape as
    // flex; `_above_272k_tokens_ultrafast` variants deliberately not
    // ingested (no long-context tier extraction — the rm-610 decision,
    // held for the third tier family too).
    #[serde(default, rename = "input_cost_per_token_ultrafast")]
    input_cost_ultrafast: Option<f64>,
    #[serde(default, rename = "output_cost_per_token_ultrafast")]
    output_cost_ultrafast: Option<f64>,
    #[serde(default, rename = "cache_creation_input_token_cost_ultrafast")]
    cache_write_cost_ultrafast: Option<f64>,
    #[serde(default, rename = "cache_read_input_token_cost_ultrafast")]
    cache_read_cost_ultrafast: Option<f64>,
    // Priority-tier rates (rm-1556, cycle-4): LiteLLM publishes
    // `input_cost_per_token_priority`/`output_cost_per_token_priority`
    // (and cache-variant) fields; `_above_272k_tokens_priority`
    // variants deliberately not ingested — same decision as flex and
    // ultrafast: no long-context tier extraction to pair them with.
    #[serde(default, rename = "input_cost_per_token_priority")]
    input_cost_priority: Option<f64>,
    #[serde(default, rename = "output_cost_per_token_priority")]
    output_cost_priority: Option<f64>,
    #[serde(default, rename = "cache_creation_input_token_cost_priority")]
    cache_write_cost_priority: Option<f64>,
    #[serde(default, rename = "cache_read_input_token_cost_priority")]
    cache_read_cost_priority: Option<f64>,
}

pub fn lookup_price(model: &str) -> Price {
    lookup_price_tiered(model, None)
}

/// rm-1532: the embedded models.dev pricing extract — the SECOND
/// catalog. Live LiteLLM (checked 2026-10-16, research run b1e8b617)
/// carries no row in ANY form for the dated first-party Anthropic ids
/// Claude Code actually records (`claude-sonnet-4-20250514`,
/// `claude-3-7-sonnet-20250219`, `claude-opus-4-1-20250805`); the only
/// rows ending in those ids are reseller-qualified, so the raw-first
/// ladder's base-model collapse let the lowest-priority reseller
/// winner serve the id — `By Provider: openrouter` for a session that
/// ran first-party Anthropic. A snapshot refresh cannot fix that
/// (upstream absence, not staleness), so first-party rates +
/// attribution come from this extract instead: consulted AFTER the
/// LiteLLM ladder finds no first-party row and BEFORE a reseller row
/// serves. `pricing_source` discloses which catalog served;
/// `catalog_identity` folds the extract bytes in so a changed extract
/// (new binary) reprices cached sessions. Rows self-retire the day
/// LiteLLM ships a first-party row (see `_snapshot.source`).
const MODELS_DEV_SNAPSHOT_JSON: &str = include_str!("models_dev_snapshot.json");
const MODELS_DEV_SNAPSHOT_DATE: &str = "2026-10-16";

/// Providers that are the model's own vendor (rm-1532): a catalog row
/// attributed to one of these ends the search — that IS the
/// first-party answer. Everything else (openrouter,
/// vercel_ai_gateway, github_copilot, bedrock_converse, …) is a
/// reseller/mediation lane: correct for a session that genuinely ran
/// through it, wrong as the default winner for a first-party id.
const FIRST_PARTY_PROVIDERS: &[&str] = &[
    "anthropic",
    "openai",
    "google",
    "meta",
    "mistral",
    "xai",
    "deepseek",
    "qwen",
    "cohere",
    "perplexity",
    "ai21",
    "moonshot",
];

fn is_first_party_provider(provider: &str) -> bool {
    FIRST_PARTY_PROVIDERS.contains(&provider)
}

/// One row of the embedded extract (`models_dev_snapshot.json`).
/// Rates are USD per million tokens, exactly the units [`Price`]
/// stores after LiteLLM's per-token conversion — no scaling here.
#[derive(Debug, Deserialize)]
struct ModelsDevRow {
    provider: String,
    #[serde(default)]
    input: f64,
    #[serde(default)]
    output: f64,
    #[serde(default)]
    cache_read: Option<f64>,
    #[serde(default)]
    cache_write: Option<f64>,
}

/// The parsed second catalog: entries keyed by the model id EXACTLY
/// as a session records it (dated suffix intact — never normalized to
/// the base id), plus each row's first-party provider label.
#[derive(Debug, Default)]
struct ModelsDevCatalog {
    entries: BTreeMap<String, Price>,
    providers: BTreeMap<String, String>,
}

fn models_dev_catalog() -> &'static ModelsDevCatalog {
    MODELS_DEV_CATALOG.get_or_init(|| {
        // rm-1978: build-time content corruption degrades LOUDLY, never
        // panics mid-report and never silently falsifies a lookup. The
        // extract is a fallback enrichment lane: when it cannot serve,
        // extract-only models miss and disclose through the ordinary
        // unpriced-model path (absence, never a wrong number), and the
        // recorded parse failure is named verbatim in `--doctor`'s
        // pricing provenance line via [`models_dev_parse_failure`].
        parse_models_dev_snapshot(MODELS_DEV_SNAPSHOT_JSON).unwrap_or_else(|error| {
            let _ = MODELS_DEV_PARSE_FAILURE.set(error);
            ModelsDevCatalog::default()
        })
    })
}

/// rm-1978: the recorded parse failure for the embedded models.dev
/// extract, present exactly when the snapshot failed to parse
/// (build-time content corruption, never operator input). Surfaced by
/// the doctor's pricing provenance so the degradation is named, not
/// silent.
pub(crate) fn models_dev_parse_failure() -> Option<&'static str> {
    MODELS_DEV_PARSE_FAILURE.get().map(String::as_str)
}

/// rm-1978: pure snapshot parser so the corruption failure mode is
/// unit-pinnable (the old `rm-1532` expect could only fail at runtime).
/// The file's first key is `_snapshot` — provenance prose, not a row —
/// so the map parses as JSON values first and each row converts
/// individually; a malformed row names itself in the returned error.
fn parse_models_dev_snapshot(raw: &str) -> Result<ModelsDevCatalog, String> {
    let mut catalog = ModelsDevCatalog::default();
    let map: BTreeMap<String, serde_json::Value> = serde_json::from_str(raw)
        .map_err(|error| format!("embedded models_dev_snapshot.json failed to parse: {error}"))?;
    for (id, value) in map {
        if id == "_snapshot" {
            continue;
        }
        let row: ModelsDevRow = serde_json::from_value(value)
            .map_err(|error| format!("models_dev_snapshot.json row {id} must parse: {error}"))?;
        // Key: the model id after its "provider/" prefix — the raw
        // dated id a session records. Lowercased to match the
        // primary catalog's canonicalization.
        let key = id
            .rsplit_once('/')
            .map(|(_, model)| model)
            .unwrap_or(&id)
            .to_ascii_lowercase();
        catalog.entries.insert(
            key.clone(),
            Price {
                input: row.input,
                output: row.output,
                // rm-1508 discipline carried into the second catalog:
                // an unpublished extract cache rate stays `None`
                // (bills at the disclosed fallback), a published
                // number — explicit $0 included — stays `Some`.
                // Every row this extract ships today publishes both
                // rates, so served prices are byte-identical to the
                // rm-1532 landing.
                cr: row.cache_read,
                cw: row.cache_write,
                ..Price::default()
            },
        );
        catalog.providers.insert(key, row.provider);
    }
    Ok(catalog)
}

/// rm-1532: which catalog serves a session id — the ONE seam the rate
/// ([`lookup_price_tiered`]), provider ([`provider_for_in`]), and
/// pricing-source ([`pricing_source_for_catalog`]) lookups all
/// resolve through, so they can never disagree about which row served
/// a given session id.
enum ServingCatalog {
    /// The LiteLLM catalog — bundled snapshot, cached, or a
    /// `--update-pricing` refresh (with user overrides applied on
    /// top, which stay supreme: their rate AND identity are the
    /// user's own).
    Primary { key: String },
    /// The embedded models.dev extract served a first-party row the
    /// primary catalog lacks. `key` is the extract's raw dated id.
    ModelsDev { key: String },
}

fn serving_catalog(model: &str, catalog: &PricingCatalog) -> Option<ServingCatalog> {
    if let Some(key) = matching_catalog_key_for(model, &catalog.aliases, &catalog.entries) {
        if pricing_override_models().contains(&key)
            || catalog
                .providers
                .get(&key)
                .is_some_and(|provider| is_first_party_provider(provider))
        {
            return Some(ServingCatalog::Primary { key });
        }
        // The LiteLLM ladder found only a reseller row (or one with no
        // provider label) — rm-1532: before that fall-through serves,
        // the embedded extract may serve the id through its own
        // first-party row.
        if let Some(key) = models_dev_key(model) {
            return Some(ServingCatalog::ModelsDev { key });
        }
        return Some(ServingCatalog::Primary { key });
    }
    models_dev_key(model).map(|key| ServingCatalog::ModelsDev { key })
}

/// rm-1532: the extract's EXACT-id ladder — the raw id and its
/// lowercase form only. Deliberately NO normalization or base-model
/// collapse: the extract speaks for the dated ids it carries rows
/// for, never for base ids whose bundled-corpus winners must stay
/// byte-stable (no collateral re-attribution).
fn models_dev_key(model: &str) -> Option<String> {
    let catalog = models_dev_catalog();
    let raw = model.trim();
    if raw.is_empty() {
        return None;
    }
    let lower = raw.to_ascii_lowercase();
    for candidate in [raw, lower.as_str()] {
        if catalog.entries.contains_key(candidate) {
            return Some(candidate.to_string());
        }
    }
    None
}

/// rm-610 (flex) + cycle-2 rider (ultrafast) + cycle-4 rider
/// (priority/fast, rm-1556): resolve a model's rates for the service
/// tier the turn actually ran on. `Some("flex")`/`Some("ultrafast")`/
/// `Some("priority")` bills at the entry's tier slots when the
/// catalog row carries them — input AND output must both be present,
/// and a missing cache slot falls back to that standard rate —
/// while `Some("fast")` bills a published model-specific multiplier
/// over the resolved standard rates (LiteLLM publishes no `_fast`
/// cost fields; see [`fast_tier_price`]) and every other tier (None,
/// "default", ...) bills at the standard rates. A model without tier
/// rates bills tiered turns at the standard rates: that is the
/// published behavior (codeburn #1642, held for every tier family),
/// not a guess.
pub fn lookup_price_tiered(model: &str, service_tier: Option<&str>) -> Price {
    let catalog = pricing_catalog();
    // rm-1053 (run f2fccaa9 cycle 3, assess 53af6c9e F1): resolve
    // through the raw-first ladder (see `resolution_ladder`) — a
    // region/vendor-qualified id must bill at its own catalog row
    // before the base-model collapse is consulted. The tier rewrite
    // (flex + ultrafast) applies on top of whichever row the ladder
    // picked, so a qualified row's tier slots serve its own region
    // too.
    // rm-1532: the ONE serving seam — when the LiteLLM ladder finds no
    // first-party row but the embedded models.dev extract carries one
    // for the exact dated id, the extract's rates serve (its rows
    // carry no tier slots, so the tier rewrite below no-ops on that
    // arm).
    let mut price = match serving_catalog(model, catalog) {
        Some(ServingCatalog::ModelsDev { key }) => models_dev_catalog()
            .entries
            .get(&key)
            .cloned()
            .unwrap_or_default(),
        _ => lookup_price_in(model, &catalog.aliases, &catalog.entries),
    };
    let (input_tier, output_tier, cw_tier, cr_tier) = match service_tier {
        Some("flex") => (
            price.input_flex,
            price.output_flex,
            price.cw_flex,
            price.cr_flex,
        ),
        Some("ultrafast") => (
            price.input_ultrafast,
            price.output_ultrafast,
            price.cw_ultrafast,
            price.cr_ultrafast,
        ),
        // Priority tier (rm-1556, cycle-4): bill the entry's published
        // `_priority` slots when both input and output are present —
        // identical semantics to flex/ultrafast.
        Some("priority") => (
            price.input_priority,
            price.output_priority,
            price.cw_priority,
            price.cr_priority,
        ),
        // Codex "fast" tier (rm-1556, cycle-4): LiteLLM publishes no
        // `_fast` cost fields (only a `supports_fast_mode` boolean),
        // so fast is billed as a published model-specific multiplier
        // over the resolved standard rates — the same convention
        // ccusage adopted for OpenAI's "Fast = 2× API Priority rate"
        // products (ccusage#1855). Unknown models bill standard,
        // disclosed, never guessed.
        Some("fast") => return fast_tier_price(price, model),
        _ => return price,
    };
    if let (Some(input), Some(output)) = (input_tier, output_tier) {
        price.input = input;
        price.output = output;
        if let Some(cw) = cw_tier {
            price.cw = Some(cw);
        }
        if let Some(cr) = cr_tier {
            price.cr = Some(cr);
        }
    }
    price
}

/// Published fast-tier multipliers (rm-1556, cycle-4), mirrored from
/// ccusage's `fast-multiplier-overrides.json` (fetched 2026-10-17, main;
/// OpenAI documents Fast as "2× API Priority rate" for the GPT-5.6
/// Sol/Terra/Luna family, and ccusage tracks the older GPT-5.x/6
/// multipliers from the same source). Keys are exact model ids;
/// `FAST_TIER_PREFIX_MULTIPLIERS` matches normalized prefixes
/// (longest first). An id with no published multiplier returns None —
/// the turn bills at the standard rates, disclosed, in ccusage parity
/// ("reports standard pricing rather than assuming a multiplier for
/// unknown models").
const FAST_TIER_MULTIPLIERS: &[(&str, f64)] = &[
    ("gpt-5.6-sol", 2.0),
    ("gpt-5.6-terra", 2.0),
    ("gpt-5.6-luna", 2.0),
    ("gpt-5.5", 2.5),
    ("gpt-5.4", 2.0),
    ("gpt-5.3-codex", 2.0),
    ("gpt-6-astra", 2.0),
    ("gpt-6-sol", 2.0),
    ("gpt-6.1-sol", 2.0),
    ("gpt-6-luna", 2.0),
];

const FAST_TIER_PREFIX_MULTIPLIERS: &[(&str, f64)] = &[
    ("claude-opus-4-6", 6.0),
    ("claude-opus-4-7", 6.0),
    ("claude-opus-4-8", 2.0),
];

fn fast_multiplier_for(model: &str) -> Option<f64> {
    let normalized = model.trim().to_ascii_lowercase();
    if normalized.is_empty() {
        return None;
    }
    if let Some((_, multiplier)) = FAST_TIER_MULTIPLIERS
        .iter()
        .find(|(id, _)| *id == normalized)
    {
        return Some(*multiplier);
    }
    FAST_TIER_PREFIX_MULTIPLIERS
        .iter()
        // Longest prefix first so claude-opus-4-7x cannot be captured
        // by a shorter prefix later in the table.
        .filter(|(prefix, _)| normalized.len() > prefix.len() && normalized.starts_with(prefix))
        .max_by_key(|(prefix, _)| prefix.len())
        .map(|(_, multiplier)| *multiplier)
}

fn fast_tier_price(mut price: Price, model: &str) -> Price {
    // Resolve through the same raw-first ladder the tier slots use:
    // a vendor-qualified id WITHOUT its own catalog row falls through
    // to its base id and inherits the base row's multiplier, while a
    // qualified id that IS its own row (azure/gpt-5.6-luna) stays
    // itself — outside the override table it bills standard,
    // disclosed, never guessed (test-pinned).
    let catalog = pricing_catalog();
    let resolved = matching_catalog_key_for(model, &catalog.aliases, &catalog.entries)
        .unwrap_or_else(|| model.to_string());
    let Some(multiplier) = fast_multiplier_for(&resolved) else {
        return price;
    };
    price.input *= multiplier;
    price.output *= multiplier;
    // rm-1508 composition (wall, post-base): cache slots are Option —
    // a PUBLISHED rate scales with the multiplier, while an absent
    // rate stays absent: its disclosed fallback
    // ([`Price::cache_write_billed`]/[`Price::cache_read_billed`])
    // derives from the input rate, which is already scaled above, so
    // scaling a fallback placeholder here would double-apply the
    // multiplier and invent a published-looking rate the vendor never
    // published.
    price.cw = price.cw.map(|rate| rate * multiplier);
    price.cr = price.cr.map(|rate| rate * multiplier);
    price
}

pub fn has_specific_price(model: &str) -> bool {
    if matches!(model.trim(), "" | "default" | "unknown") {
        return false;
    }
    let catalog = pricing_catalog();
    // rm-1053: raw-first ladder, so a qualified id counts as priced
    // through its own row too. rm-1532: the serving seam — an id only
    // the embedded extract prices (no LiteLLM row in any form) is
    // specific NOW, not a fallback price.
    serving_catalog(model, catalog).is_some()
}

/// rm-1426: true when no pricing row NAMES this deployment's model, so any
/// quoted price is fabricated for it. Two arms, both silent before this:
/// (a) nothing matches and `lookup_price_in` answers the builtin default
/// ($3.00/$15.00 per Mtok — the `builtin.get("default")` fall-through);
/// (b) `convert_litellm` collapses carrier prefixes onto bare slugs
/// (`zai/glm-5.2` -> `glm-5.2`), so a locally-defined slug that shares a
/// name with ANY public carrier row is priced from whichever carrier wins
/// `provider_priority` (research 2026-10-14: bare `glm-5.2`, the codex
/// `model_catalog_json` / CLIProxyAPI deployment slug, prices at a foreign
/// carrier's $0.152/$12.00 while the ~90 real carrier rows disagree and
/// the operator is privately billed at different rates). Neither arm is
/// the model's own price. Confirmed — false — when an exact published row
/// (bare key or the carrier-prefixed slug as recorded), an operator alias,
/// an `AGENTTRACE_PRICING_FILE` row (see `PRICING_OVERRIDE_MODELS`), or an
/// exact builtin row names the model; `""`/`"default"`/`"unknown"` stay
/// out (missing identity is a different disclosure). Callers disclose
/// (`locally_defined_model_pricing:{slug}`, plus the dollars at the
/// fabricated rate), never silently bill; rm-1052 is the sibling rule for
/// recorded-zero providers.
pub fn is_locally_defined(model: &str) -> bool {
    if matches!(model.trim(), "" | "default" | "unknown") {
        return false;
    }
    let catalog = pricing_catalog();
    let normalized = normalize_model(model);
    // Operator or built-in SKU alias mapping this slug onto a named row.
    // rm-1053 composition (wall, post-base): `resolve_alias` grew a
    // `resolve_alias_detail` core that seeds its chain with the RAW slug
    // as well as the normalized one and returns the UN-redirected input
    // when no alias applies — so the authored baseline comparison
    // (`resolve_alias(model) != normalized`) early-returned here for
    // EVERY carrier-prefixed slug ("myco/gpt-9-private"), silently
    // exempting the whole private-proxy recording class from disclosure
    // (re-proven red by the cache-catalog arm: a one-row cache origin
    // that does not name `zai/glm-5.2` stopped calling it
    // locally-defined). The resolver's own `redirected` flag is the
    // alias-baseline truth under either resolver shape: an alias names
    // the model exactly when a chain step actually redirected it,
    // whatever string the resolver hands back.
    if resolve_alias_detail(model, &catalog.aliases).1 {
        return false;
    }
    // Exact published row — the bare key, or the slug carrying its own
    // carrier prefix as the session recorded it.
    if pricing_raw_keys().contains(model) || pricing_raw_keys().contains(&normalized) {
        return false;
    }
    // Operator override file binds this slug (the pricing-catalog loader
    // records every model an accepted override file names).
    if pricing_override_models().contains(&normalized) {
        return false;
    }
    // Curated builtin row naming this exact model.
    if builtin_pricing().contains_key(&normalized) {
        return false;
    }
    // Composition with the naming lanes the wall landed AFTER this
    // predicate's base (independent review, conflict case bcdd1a88):
    // a slug the ACTIVE catalog prices through a row that names a real
    // published id of the same model is NOT locally-defined — those
    // dollars are the model's own published rate, never a fabricated
    // one. Three lanes, none of which the base knew:
    // (1) code-side vend rows — rm-610's `gpt-5.3-codex` and rm-1570's
    //     `gpt-daybreak-*-latest` are inserted under their own exact
    //     ids while LiteLLM's chat-mode filter keeps the same ids out
    //     of the raw envelope, so the raw-key arm above can never see
    //     them (without this arm every codex 0.163 daybreak session —
    //     the DEFAULT model set — disclosed fabricated pricing at the
    //     ids' own hand-pinned rates);
    // (2) the embedded models.dev extract (rm-1532) — first-party rows
    //     for exact dated ids the LiteLLM envelope does not carry;
    // (3) the resolution ladder's own-model variants (rm-1511 effort /
    //     minor tails, the deepseek family pushers) — the ladder bills
    //     `claude-sonnet-5-5-medium` at `claude-sonnet-5-5`'s own raw
    //     row, which is that model's published rate, not a borrowed
    //     carrier collapse.
    // Arm (b) is untouched: `glm-5.2`'s ladder variants (`glm-5.2`,
    // `glm`) name no raw key of the active origin, so a foreign
    // carrier's collapsed row still does not name the deployment's
    // slug and the disclosure keeps firing.
    let vend = code_vended_pricing_ids();
    if vend.contains(&normalized.as_str()) {
        return false;
    }
    if models_dev_key(model).is_some() {
        return false;
    }
    let raw_keys = pricing_raw_keys();
    let extract = &models_dev_catalog().entries;
    let builtin = builtin_pricing();
    let named_through_the_ladder =
        resolution_ladder(model, &catalog.aliases)
            .iter()
            .any(|variant| {
                raw_keys.contains(variant)
                    || vend.contains(&variant.as_str())
                    || extract.contains_key(variant)
                    || builtin.contains_key(variant)
            });
    !named_through_the_ladder
}

/// Exact keys of the ACTIVE catalog's raw origin — the downloaded
/// cache when one is served, else the bundled snapshot — BEFORE
/// `convert_litellm` normalizes/collapses carrier prefixes. rm-1426 uses
/// this to tell a row that names the deployment's own slug from a foreign
/// carrier's row that merely shares the collapsed name; the set follows
/// the same origin `pricing_catalog()` serves entries from, so a slug the
/// LIVE catalog names is never locally-defined just because the bundled
/// snapshot predates it (and a slug only the stale bundle names is
/// locally-defined, exactly like any other borrowed rate).
fn pricing_raw_keys() -> &'static BTreeSet<String> {
    PRICING_RAW_KEYS.get_or_init(|| litellm_raw_keys(PRICING_SNAPSHOT_JSON.as_bytes()))
}

/// Top-level keys of a raw LiteLLM-format catalog (the snapshot constant
/// and the on-disk cache store the same envelope).
fn litellm_raw_keys(raw: &[u8]) -> BTreeSet<String> {
    serde_json::from_slice::<BTreeMap<String, serde_json::Value>>(raw)
        .map(|rows| rows.into_keys().collect())
        .unwrap_or_default()
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
        // The internal default row publishes explicit zeros: the
        // fallback estimate must not invent cache premiums on top of
        // an already-rough estimate (rm-1508).
        cw: Some(0.0),
        cr: Some(0.0),
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

/// rm-1508 census: how the active catalog's rows carry cache rates —
/// published-nonzero / published-$0 / absent — per cache class. This
/// is the distinction ingest now preserves end to end: an absent rate
/// bills at the disclosed fallback while an explicit `0` bills as
/// $0. Exposed on the `--test-match` JSON probe so the population is
/// answerable without a code change; the vendored-bundle counts are
/// pinned by tests/pricing_snapshot_hygiene.rs so a snapshot refresh
/// cannot silently regress the distinction.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize)]
pub struct CacheRateCensus {
    pub models: usize,
    pub cache_write_published: usize,
    pub cache_write_zero: usize,
    pub cache_write_absent: usize,
    pub cache_read_published: usize,
    pub cache_read_zero: usize,
    pub cache_read_absent: usize,
}

/// The cache-rate census of the active catalog (bundled snapshot +
/// codex SKUs + any user overrides), cached after first computation
/// (rm-1508).
pub fn cache_rate_census() -> CacheRateCensus {
    static CENSUS: OnceLock<CacheRateCensus> = OnceLock::new();
    *CENSUS.get_or_init(|| {
        let mut census = CacheRateCensus::default();
        for price in pricing_catalog().entries.values() {
            census.models += 1;
            match price.cw {
                Some(rate) if rate > 0.0 => census.cache_write_published += 1,
                Some(_) => census.cache_write_zero += 1,
                None => census.cache_write_absent += 1,
            }
            match price.cr {
                Some(rate) if rate > 0.0 => census.cache_read_published += 1,
                Some(_) => census.cache_read_zero += 1,
                None => census.cache_read_absent += 1,
            }
        }
        census
    })
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
    // rm-1881: resolve through the rm-1053 raw-first ladder — the same
    // resolver the rate, provider, and pricing-source lanes use — so the
    // window can never disagree with the priced row about which catalog
    // entry served the session id. Pre-fix this was the one lane still
    // on the plain matcher (alias-resolve, then variants), which skipped
    // the raw qualified id: "azure/eu/GPT-5.5" and alias-mapped ids like
    // "aihubmix/glm-5.3-flash" got the BASE row's window (or None)
    // while their rates came from their own rows.
    let key = matching_catalog_key_for(model, &catalog.aliases, &catalog.entries)?;
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
    IDENTITY.get_or_init(|| {
        // rm-1532: the embedded models.dev extract can reprice and
        // re-attribute ids the primary catalog serves through reseller
        // rows, so its bytes ride the identity — a changed extract (a
        // new binary) must reprice cached sessions even when the
        // primary catalog is byte-identical. `catalog_identity_of`
        // (and every synthetic-catalog pin on it) stays untouched.
        let primary = catalog_identity_of(pricing_catalog());
        let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
        for byte in MODELS_DEV_SNAPSHOT_JSON.as_bytes() {
            hash = (hash ^ *byte as u64).wrapping_mul(0x100_0000_01b3);
        }
        format!("{primary}-d{hash:016x}")
    })
}

/// Digest of every catalog fact that can change a cost or an
/// attribution: rate values (standard AND every tier family — flex,
/// rm-610; ultrafast, cycle-2; priority, cycle-4), context
/// windows, deprecation dates, alias rows, and provider labels
/// (rm-613). Only `source` and `reference_date` are excluded — they
/// describe where the data came from, not what it prices. The session
/// cache freshness key consumes this: a rate-identical provider flip
/// or a gained tier slot (any family) must reprice cached sessions.
pub(crate) fn catalog_identity_of(catalog: &PricingCatalog) -> String {
    fn fnv1a(bytes: &[u8], state: u64) -> u64 {
        bytes.iter().fold(state, |hash, byte| {
            (hash ^ *byte as u64).wrapping_mul(0x100_0000_01b3)
        })
    }
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for (name, price) in &catalog.entries {
        hash = fnv1a(name.as_bytes(), hash);
        for value in [price.input, price.output] {
            hash = fnv1a(&value.to_bits().to_le_bytes(), hash);
        }
        // rm-1508: standard cache slots are Option now — hash the
        // flag byte like the tier slots so an absent (fallback-priced)
        // rate never collides with a published one, including $0.
        // This changes the identity for every catalog (one-time
        // warm-cache invalidation), which is the point: the fallback
        // reprices.
        for slot in [price.cw, price.cr] {
            hash = fnv1a(&[slot.is_some() as u8], hash);
            if let Some(value) = slot {
                hash = fnv1a(&value.to_bits().to_le_bytes(), hash);
            }
        }
        hash = fnv1a(&price.max_input_tokens.unwrap_or(0).to_le_bytes(), hash);
        hash = fnv1a(
            price.deprecation_date.as_deref().unwrap_or("").as_bytes(),
            hash,
        );
        // rm-610: a None slot must hash differently from Some(value) —
        // the flag byte keeps gained/lost tier pricing from colliding
        // with a value change (flex and, since the cycle-2 rider,
        // ultrafast — a gained ultrafast slot must reprice cached
        // sessions the same way). The rm-1556 cycle-4 rider extends
        // the loop to priority: a refresh that changes ONLY priority
        // rates must reprice warm caches, not serve stale
        // standard-priced priority turns (the same class rm-610
        // closed for flex).
        for tier in [
            price.input_flex,
            price.output_flex,
            price.cw_flex,
            price.cr_flex,
            price.input_ultrafast,
            price.output_ultrafast,
            price.cw_ultrafast,
            price.cr_ultrafast,
            price.input_priority,
            price.output_priority,
            price.cw_priority,
            price.cr_priority,
        ] {
            hash = fnv1a(&[tier.is_some() as u8], hash);
            if let Some(value) = tier {
                hash = fnv1a(&value.to_bits().to_le_bytes(), hash);
            }
        }
    }
    for (alias, target) in &catalog.aliases {
        hash = fnv1a(alias.as_bytes(), hash);
        hash = fnv1a(target.as_bytes(), hash);
    }
    // rm-613: provider labels are attribution the digest used to
    // miss — a rate-identical provider flip changed nothing here.
    for (model, provider) in &catalog.providers {
        hash = fnv1a(model.as_bytes(), hash);
        hash = fnv1a(provider.as_bytes(), hash);
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
    // rm-1053: the same raw-first ladder the rate lookup uses, so the
    // provider rides the row that served the rate. (This used to
    // normalize the id a second time — normalize of a normalize — so
    // a dated, versioned, regional id resolved to a different key
    // than its own rate did, and by-provider attributed the wrong
    // vendor for exactly those rows.) rm-1532: the ONE serving seam —
    // when the extract serves the rate, the extract's first-party
    // label serves the attribution, so rate and vendor can never
    // disagree about which row spoke.
    match serving_catalog(model, catalog) {
        Some(ServingCatalog::Primary { key }) => catalog
            .providers
            .get(&key)
            .cloned()
            .filter(|provider| !provider.trim().is_empty()),
        Some(ServingCatalog::ModelsDev { key }) => {
            models_dev_catalog().providers.get(&key).cloned()
        }
        None => None,
    }
}

fn pricing_source_for_catalog(
    model: &str,
    catalog: &PricingCatalog,
    override_models: &BTreeSet<String>,
) -> String {
    let normalized = normalize_model(model);
    let resolved = resolve_alias(&normalized, &catalog.aliases);
    // rm-1532: disclose which catalog served. The extract arm comes
    // first so a reseller fall-through can never masquerade as the
    // LiteLLM catalog; its rows are their own provenance, so the
    // vendor-retirement arms below (primary-catalog knowledge) do not
    // apply to it.
    let key = match serving_catalog(model, catalog) {
        Some(ServingCatalog::ModelsDev { .. }) => {
            return format!(
                "models.dev snapshot {MODELS_DEV_SNAPSHOT_DATE} (embedded first-party extract; LiteLLM lacks a first-party row)"
            );
        }
        Some(ServingCatalog::Primary { key }) => key,
        None => return "built-in fallback".to_string(),
    };
    // rm-1053: raw-first ladder — a qualified id's own row is found
    // before the base collapse resolves. `resolved` stays
    // alias-of-normalized: the override arms below detect whether an
    // alias actually moved the id, which raw-form resolution would
    // misreport for every qualified id.
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
    // rm-575: the pricing catalog joins the session cache's relocation
    // contract — a non-empty AGENTTRACE_SESSION_CACHE_DIR moves
    // pricing.json beside sessions.json, so one env relocates every
    // at-rest artifact and an operator scrubbing a shared host leaves
    // no pricing trail behind. The provenance stamp derives from this
    // same path (pricing_meta_cache_path), so both move together. An
    // empty value stays inert, matching the session cache's ladder.
    if let Some(dir) = std::env::var_os("AGENTTRACE_SESSION_CACHE_DIR").map(PathBuf::from) {
        if !dir.as_os_str().is_empty() {
            return dir.join("pricing.json");
        }
    }
    user_cache_dir().join("agenttrace").join("pricing.json")
}

/// Provenance stamp written beside the catalog (`pricing.meta.json`,
/// rm-167). Derived from the catalog path so both always live
/// together — wherever the catalog relocates, the stamp follows.
pub fn pricing_meta_cache_path() -> PathBuf {
    pricing_cache_path().with_extension("meta.json")
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
    // Same redirect parity as the initial catalog: a `--update-pricing`
    // refresh replaces the singleton, and the codex SKU coverage must
    // survive it (rm-610) instead of silently reverting to fallback —
    // the daybreak responses pins ride the same refresh parity (rm-1570).
    apply_codex_sku_entries(&mut catalog);
    apply_codex_sku_aliases(&mut catalog);
    apply_daybreak_responses_entries(&mut catalog);
    // rm-1426 refresh parity: `--update-pricing` publishes a new ACTIVE
    // origin, so its raw keys replace the singleton's set in lockstep with
    // the catalog published just above. The vend rows (codex SKU, daybreak)
    // are code-side inserts, never raw-origin keys, so they deliberately
    // stay out of the set — same shape as `load_catalog_for_current_env`,
    // which captures the origin's keys before any insert runs.
    let _ = PRICING_RAW_KEYS.set(litellm_raw_keys(raw.as_bytes()));
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
        // rm-1053: the regional-twin probe family — every one of
        // these must show its OWN row's rates, not the base-model
        // collapse winner (us $3.30, global $3.00, us-gov $3.60,
        // bedrock GovCloud $3.60 input per M).
        "eu.anthropic.claude-sonnet-4-5-20250929-v1:0",
        "global.anthropic.claude-sonnet-4-5-20250929-v1:0",
        "au.anthropic.claude-sonnet-4-5-20250929-v1:0",
        "us-gov.anthropic.claude-sonnet-4-5-20250929-v1:0",
        "bedrock/us-gov-east-1/anthropic.claude-sonnet-4-5-20250929-v1:0",
        "azure/us-gov/gpt-5.1",
        "openai/gpt-4.1",
        "gpt-5.6",
        "gpt-4.1-mini-2025-04-14",
        "deepseek-chat",
        "deepseek/deepseek-v3.2",
        "gemini-2.5-pro",
        "gpt-6-astra",
        "unknown-model-xyz",
    ] {
        let p = lookup_price(model);
        // rm-1508: show the rates a cache-token turn actually bills
        // at — an absent cache rate renders as the disclosed fallback
        // with its marker line, so the probe never presents a silent
        // $0 for an unpublished rate.
        let (cw, cw_absent) = p.cache_write_billed();
        let (cr, cr_absent) = p.cache_read_billed();
        out.push_str(&format!(
            "  {:<50} → in=${:>7.2}/M  out=${:>7.2}/M  cw=${:>6.2}/M  cr=${:>6.2}/M\n",
            model, p.input, p.output, cw, cr
        ));
        if cw_absent || cr_absent {
            out.push_str(
                "      cache: rates not published — billed at input-rate fallback (cr) / 1.25× input (cw), disclosed per session\n",
            );
        }
        // rm-610 (+ cycle-2 ultrafast rider): disclose the tier rates
        // when the row carries them, so the rate a
        // `service_tier: "flex"`/`"ultrafast"` turn bills at is
        // answerable from the same probe (rows without the tier bill
        // tiered turns at the standard rates above).
        if let (Some(in_flex), Some(out_flex)) = (p.input_flex, p.output_flex) {
            out.push_str(&format!(
                "      flex: in=${:>7.2}/M  out=${:>7.2}/M  cw=${:>6.2}/M  cr=${:>6.2}/M\n",
                in_flex,
                out_flex,
                p.cw_flex.unwrap_or(cw),
                p.cr_flex.unwrap_or(cr)
            ));
        }
        if let (Some(in_uf), Some(out_uf)) = (p.input_ultrafast, p.output_ultrafast) {
            out.push_str(&format!(
                "      ultrafast: in=${:>7.2}/M  out=${:>7.2}/M  cw=${:>6.2}/M  cr=${:>6.2}/M\n",
                in_uf,
                out_uf,
                p.cw_ultrafast.unwrap_or(cw),
                p.cr_ultrafast.unwrap_or(cr)
            ));
        }
    }
    out
}

pub(crate) fn token_cost(
    input: i64,
    output: i64,
    cache_write: i64,
    cache_read: i64,
    model: &str,
    service_tier: Option<&str>,
) -> f64 {
    let price = lookup_price_tiered(model, service_tier);
    // rm-1508: absent cache rates bill at the disclosed fallback
    // (cr = input rate, cw = input × the vendor-convention multiple),
    // never a silent $0. Term order is unchanged so published-rate
    // models price identically.
    let (cw, _) = price.cache_write_billed();
    let (cr, _) = price.cache_read_billed();
    round4(
        input as f64 / 1e6 * price.input
            + output as f64 / 1e6 * price.output
            + cache_write as f64 / 1e6 * cw
            + cache_read as f64 / 1e6 * cr,
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
    let mut catalog = match load_pricing_cache() {
        Some((catalog, raw_keys)) => {
            // rm-1426: the active origin is the cache — its raw keys (not
            // the bundled snapshot's) decide which slugs are named.
            let _ = PRICING_RAW_KEYS.set(raw_keys);
            catalog
        }
        None => {
            let _ = PRICING_RAW_KEYS.set(litellm_raw_keys(PRICING_SNAPSHOT_JSON.as_bytes()));
            fallback_catalog()
        }
    };
    let override_models = apply_pricing_overrides(&mut catalog);
    let _ = PRICING_OVERRIDE_MODELS.set(override_models);
    apply_codex_sku_entries(&mut catalog);
    apply_codex_sku_aliases(&mut catalog);
    apply_daybreak_responses_entries(&mut catalog);
    catalog
}

/// Codex SKU redirect rows (rm-610): model ids the codex CLI reports
/// that LiteLLM's chat-mode catalog carries no row for, mapped to
/// their published equivalents (codeburn #1642: gpt-reserve is Luna
/// Reserve pricing as GPT-5.6 Luna; gpt-5.3-spark is Codex Spark
/// pricing as GPT-5.3 Codex; gpt-5.6-codex is the codex-tuned
/// GPT-5.6). A redirect is only added when the catalog has NO direct
/// entry for the id, so it retires itself the day upstream ships a
/// real row — no manual drop owed at snapshot-refresh time — and a
/// user override alias for the same id still wins (`or_insert`).
fn apply_codex_sku_aliases(catalog: &mut PricingCatalog) {
    for (sku, target) in [
        ("gpt-5.6-codex", "gpt-5.6"),
        ("gpt-reserve", "gpt-5.6-luna"),
        ("gpt-5.3-spark", "gpt-5.3-codex"),
    ] {
        if !catalog.entries.contains_key(sku) {
            catalog
                .aliases
                .entry(sku.to_string())
                .or_insert_with(|| target.to_string());
        }
    }
}

/// The `gpt-5.3-spark` redirect's target row (rm-610): LiteLLM
/// publishes `gpt-5.3-codex` only under `mode: "responses"`, which
/// the chat-mode catalog filter (update-snapshot.sh and
/// `convert_litellm` alike) excludes, so without this row the redirect
/// had no target and both ids priced at fallback. Rates are LiteLLM's
/// own row for the model, byte-identical to codeburn #1642's
/// published numbers ($1.75/M in, $14/M out, $0.175/M cache read, no
/// cache-write rate, no flex tier published). `or_insert` retires the
/// row the day upstream ships a chat-mode entry — the same
/// drop-on-upstream-ship discipline as the aliases, and a user
/// override price for the id still wins (overrides merge first).
/// rm-610 / rm-1570 (composition, conflict case bcdd1a88): the ids the
/// build vendors pricing rows for code-side. `apply_codex_sku_entries`
/// and `apply_daybreak_responses_entries` insert under these exact ids
/// while LiteLLM's chat-mode filter keeps the same ids OUT of the raw
/// snapshot envelope, so no raw-origin key ever names them — the rates
/// are the models' own published rows (drop-on-upstream-ship `or_insert`
/// discipline), and `is_locally_defined` therefore treats each as NAMED.
/// Every new vend row MUST land here together with its inserter;
/// `vend_rows_stay_named_locally_defined` pins the sync.
fn code_vended_pricing_ids() -> &'static [&'static str] {
    &[
        "gpt-5.3-codex",
        "gpt-daybreak-red-latest",
        "gpt-daybreak-blue-latest",
    ]
}

fn apply_codex_sku_entries(catalog: &mut PricingCatalog) {
    catalog
        .entries
        .entry("gpt-5.3-codex".to_string())
        .or_insert_with(|| Price {
            input: 1.75,
            output: 14.0,
            // Upstream publishes an explicit $0 cache-write rate on
            // its gpt-5.3-codex rows (openrouter/aihubmix), so this
            // defense-in-depth row matches: Some(0.0) is a published
            // $0, distinct from an absent rate (rm-1508). The row only
            // ever fires when every provider row is retired off the
            // bundle.
            cw: Some(0.0),
            cr: Some(0.175),
            ..Price::default()
        });
    catalog
        .providers
        .entry("gpt-5.3-codex".to_string())
        .or_insert_with(|| "openai".to_string());
}

/// rm-1570 (run fd0e91a2 cycle 3, money truth): the daybreak family
/// is codex 0.163.0's DEFAULT model set, and LiteLLM publishes the
/// `gpt-daybreak-*-latest` ids only under `mode: "responses"`, which
/// the chat-mode catalog filter (update-snapshot.sh and
/// `convert_litellm` alike) excludes — so the ids priced at the
/// disclosed $3/$15 default instead of their own rates. Assess
/// 2bdc99b3's live PoC measured the damage end to end: a
/// `gpt-daybreak-red-latest` session billed $18.00 on fallback where
/// the published rates put it at $87.50 — a ~4.86x UNDERSTATEMENT
/// (the default rates are cheaper than red's published rates, so a
/// fallback row is money the report silently under-bills, never a
/// safe over-estimate). Rates are LiteLLM's
/// own rows (live capture 2026-10-17), identical to the
/// `daybreak-*-latest` chat twins already bundled: red 1.25e-5 in /
/// 7.5e-5 out / 1.5625e-5 cache write / 1.25e-6 cache read per token,
/// blue 4e-6 / 2e-5 / 5e-6 / 4e-7. `or_insert` retires each row the
/// day upstream ships a chat-mode entry — the same
/// drop-on-upstream-ship discipline as the aliases and the
/// gpt-5.3-codex vend above — and a user override price for the id
/// still wins (overrides merge first).
fn apply_daybreak_responses_entries(catalog: &mut PricingCatalog) {
    for (id, input, output, cw, cr) in [
        ("gpt-daybreak-red-latest", 12.5, 75.0, 15.625, 1.25),
        ("gpt-daybreak-blue-latest", 4.0, 20.0, 5.0, 0.4),
    ] {
        catalog
            .entries
            .entry(id.to_string())
            .or_insert_with(|| Price {
                input,
                output,
                // Post-base, `Price::cw/cr` became Option<f64> (the
                // published-$0 discipline, rm-1508); the daybreak rows
                // are published LiteLLM rates, so they carry Some —
                // never an absent rate (composition fix, conflict case
                // b0d213cb; authored f64 against the run's base).
                cw: Some(cw),
                cr: Some(cr),
                ..Price::default()
            });
        catalog
            .providers
            .entry(id.to_string())
            .or_insert_with(|| "openai".to_string());
    }
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
            // rm-1053 (run bbf61fbd cycle 3, composed at integration
            // onto the landed retention mechanics): a raw name that IS
            // a catalog row corrects THAT row alone — the run's PoC
            // had a rate naming the exact twin id
            // `bedrock/us-gov-west-1/anthropic.…:0` silently land on
            // the shared slot's winner. Any other name lands on the
            // normalized slot it has always landed on, so base-id
            // override files keep their landed (rm-196) behavior.
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
            let normalized = normalize_model(&name);
            let exact_row = name != normalized && catalog.entries.contains_key(&name);
            let target = if exact_row {
                name.clone()
            } else {
                normalized.clone()
            };
            if override_price.max_input_tokens.is_none()
                || override_price.deprecation_date.is_none()
            {
                if let Some(catalog_price) = catalog.entries.get(&target) {
                    if override_price.max_input_tokens.is_none() {
                        override_price.max_input_tokens = catalog_price.max_input_tokens;
                    }
                    if override_price.deprecation_date.is_none() {
                        override_price.deprecation_date = catalog_price.deprecation_date.clone();
                    }
                }
            }
            // Register both spellings so the ladder's override arm
            // (rm-196) and the source line find the entry whichever
            // variant a session reports; the entry itself lives at
            // `target` (the row the user named).
            override_models.insert(name.clone());
            if !exact_row {
                override_models.insert(normalized);
            }
            catalog.entries.insert(target, override_price);
        }
        catalog.aliases.extend(aliases);
    }
    override_models
}

fn load_pricing_cache() -> Option<(PricingCatalog, BTreeSet<String>)> {
    let path = pricing_cache_path();
    let metadata = path.metadata().ok()?;
    let stale = metadata
        .modified()
        .ok()
        .and_then(|time| SystemTime::now().duration_since(time).ok())
        .map(|age| age > CACHE_MAX_AGE)
        .unwrap_or(false);
    let raw = std::fs::read(&path).ok()?;
    // rm-1426: the raw keys travel with the catalog they came from —
    // the active origin decides which slugs count as "named".
    let raw_keys = litellm_raw_keys(&raw);
    let converted = convert_litellm(&raw);
    if converted.entries.is_empty() {
        return None;
    }
    Some((
        PricingCatalog {
            entries: converted.entries,
            aliases: BTreeMap::new(),
            source: if stale { "cache(stale)" } else { "cache" }.to_string(),
            providers: converted.providers,
            // A cached catalog's vintage is its fetch date (rm-419); when
            // the provenance stamp is missing (pre-rm-167 cache) fall back
            // to the bundled snapshot date as a conservative anchor.
            reference_date: pricing_cache_vintage_ymd(&path)
                .or_else(|| iso_to_ymd(PRICING_SNAPSHOT_DATE)),
        },
        raw_keys,
    ))
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

/// Ureq-side transport limit for the pricing download. In ureq 3 the
/// convenience readers (read_to_string/read_to_vec/read_json) cap
/// bodies at 10 MiB by default — under the LiteLLM table the fork
/// already ships snapshots of. The streaming reader used here is
/// unlimited by default; this explicit limit sits ABOVE
/// PRICING_DOWNLOAD_MAX_BYTES (64 MiB, the upstream #318 precedent)
/// so a future refactor onto a convenience reader cannot re-trap the
/// download below the fork's own cap — `read_body_capped` stays the
/// binding constraint either way.
const PRICING_TRANSPORT_MAX_BYTES: u64 = 64 * 1024 * 1024;

// Compile-time ordering pin: the transport limit must sit above the
// fork's own cap or the ureq-side limit becomes the binding one (the
// exact trap the rm-044 migration guards against).
const _: () = assert!(PRICING_TRANSPORT_MAX_BYTES >= PRICING_DOWNLOAD_MAX_BYTES);

fn download_pricing(timeout: Duration) -> anyhow::Result<(String, ConvertedCatalog)> {
    let response = ureq::get(PRICING_URL)
        .config()
        .timeout_global(Some(timeout))
        .build()
        .call()
        .map_err(|err| anyhow!("download failed: {err}"))?;
    let raw_bytes = read_body_capped(
        response
            .into_body()
            .into_with_config()
            .limit(PRICING_TRANSPORT_MAX_BYTES)
            .reader(),
        PRICING_DOWNLOAD_MAX_BYTES,
    )?;
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
    // rm-693: stage through a fresh O_EXCL temp sibling (owner-only)
    // and rename into place so a crash mid-write can no longer leave a
    // torn catalog behind (pass-7 P7-5) and a symlink planted at the
    // predictable `{name}.tmp.{pid}.{seq}` name is bumped past, never
    // truncated through (the shared staging helper also lands the
    // catalog 0600, closing the plain-fs::write 0644 gap);
    // sweep_orphaned_temps still reclaims the temps.
    crate::session_cache::write_private_exclusive(path, raw.as_bytes())?;
    let meta = PricingCacheMeta {
        url: PRICING_URL.to_string(),
        fetched_at_unix: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|elapsed| elapsed.as_secs())
            .unwrap_or(0),
        bytes: raw.len(),
    };
    let meta_path = path.with_extension("meta.json");
    crate::session_cache::write_private_exclusive(&meta_path, &serde_json::to_vec_pretty(&meta)?)?;
    Ok(())
}

fn lookup_price_in(
    model: &str,
    aliases: &BTreeMap<String, String>,
    entries: &BTreeMap<String, Price>,
) -> Price {
    // rm-1053 (run f2fccaa9 cycle 3): catalog first, on the raw-first
    // ladder — the id exactly as the session recorded it wins before
    // any normalization collapses it onto a base-model row. Only a
    // model the catalog does not know at all falls to the built-in
    // table, which keeps the alias-resolved ladder it always had.
    if let Some(key) = matching_catalog_key_for(model, aliases, entries) {
        return entries.get(&key).cloned().unwrap_or_default();
    }
    // rm-1511 review fix (1a615b64, review ed20477f F1/F4): the catalog-first
    // arm above (matching_catalog_key_for, alias-aware since the landed
    // rm-1053/rm-196 ladder) already offered every variant
    // candidate to the catalog BEFORE this batch — the pre-batch defect was
    // that match_variants had no effort arm, so an effort-suffixed id's
    // candidate list was [raw, normalized] only and never contained the
    // in-catalog base (has_specific_price walks the same list and agreed —
    // no divergence existed). The loop below is the second-order expansion:
    // variants OF variants (claude-sonnet-5-5-pro-high -> the effort arm
    // yields claude-sonnet-5-5-pro, whose own minor arm yields the
    // in-catalog claude-sonnet-5-5), pinned by
    // second_order_variant_expansion_resolves_via_the_catalog_loop.
    for variant in match_variants(model) {
        if variant != model {
            if let Some(key) = matching_catalog_key_for(&variant, aliases, entries) {
                return entries.get(&key).cloned().unwrap_or_default();
            }
        }
    }
    let builtin = builtin_pricing();
    for variant in match_variants(&resolve_alias(model, aliases)) {
        if let Some(price) = builtin.get(&variant) {
            return price.clone();
        }
    }
    builtin.get("default").cloned().unwrap_or_default()
}

/// rm-1053: the ordered id ladder for catalog resolution. The id
/// EXACTLY as the session recorded it leads, so a region/vendor-
/// qualified catalog row (kept under its raw id by
/// `qualified_catalog_id` during conversion) wins before
/// normalization collapses the id onto the base-model row; the
/// alias-resolved variants follow. Rate, provider, and pricing-source
/// lookups all resolve through this one ladder, so they can never
/// disagree about which row served a given session id.
fn resolution_ladder(model: &str, aliases: &BTreeMap<String, String>) -> Vec<String> {
    let resolved = resolve_alias(model, aliases);
    let mut ladder: Vec<String> = Vec::with_capacity(8);
    for variant in match_variants(model)
        .into_iter()
        .chain(match_variants(&resolved))
    {
        if !ladder.contains(&variant) {
            ladder.push(variant);
        }
    }
    // rm-1053: catalog ids are canonicalized to lowercase at
    // conversion (see `qualified_catalog_id`), so a mixed-case
    // deployment id ("anyscale/meta-llama/Meta-Llama-3-70B-Instruct")
    // must still hit its own row — the lowercased raw form slots in
    // right after the exact raw form, before any normalized variant.
    let lower = model.trim().to_ascii_lowercase();
    if !lower.is_empty() && !ladder.contains(&lower) {
        ladder.insert(1, lower);
    }
    ladder
}

fn matching_catalog_key_for(
    model: &str,
    aliases: &BTreeMap<String, String>,
    entries: &BTreeMap<String, Price>,
) -> Option<String> {
    // rm-1053 raw-first: the id exactly as the session recorded it wins
    // before normalization collapses it onto a base-model row. rm-196
    // composed onto that ladder at integration (conflict case
    // a109b812): a user override is the operator's own truth — the
    // entry it published (under the id's normalized form) outranks any
    // row the ladder would otherwise collapse onto. Without this arm
    // a bundled row derived under the raw dated id (e.g.
    // `claude-sonnet-4-5-20250929`, retained from its vendor-marker
    // snapshot twins) shadowed the override forever, silently billing
    // bundled rates with an "overrides applied" banner — the exact
    // silent lie rm-196 exists to prevent. Two guards keep the landed
    // contracts byte-stable: a QUALIFIED first hit (region/vendor
    // deployment id) still wins outright — rm-1053's own-row contract
    // and rm-1532's reseller-row retention are untouched — and with no
    // override active the resolution is identical to the pre-merge
    // ladder (override membership only ever promotes a variant that
    // already carries the user's entry).
    let overrides = pricing_override_models();
    let mut first_hit: Option<String> = None;
    for variant in resolution_ladder(model, aliases) {
        if !entries.contains_key(&variant) {
            continue;
        }
        if overrides.contains(&variant) {
            return Some(variant);
        }
        if first_hit.is_none() {
            if qualified_catalog_id(&variant, &normalize_model(&variant)).is_some() {
                // A qualified catalog row keeps rm-1053 supremacy:
                // the deployment's own row bills the deployment id;
                // a base-id override never steals it.
                return Some(variant);
            }
            first_hit = Some(variant);
        }
        // Otherwise keep scanning: a later override variant outranks
        // the unqualified first hit (rm-196), while later non-override
        // variants never displace it (rm-1053 ladder order).
    }
    first_hit
}

/// rm-1053: the catalog id a region/vendor-qualified LiteLLM key must
/// keep under its own raw name, or `None` when the key is a plain
/// base identity (or a true duplicate of one) and belongs to the
/// provider-priority base collapse unchanged. Matches the qualifier
/// classes `normalize_model` strips: any '/'-path deployment id, and
/// the dotted regional prefixes in front of a vendor marker
/// (`us.`/`eu.`/`global.`/`au.`/`apac.`/`jp.`/`us-gov.` before
/// `.anthropic.`/`.google.`/`.meta.`/`.amazon.`).
fn qualified_catalog_id(key: &str, normalized: &str) -> Option<String> {
    let canonical = key.trim().to_ascii_lowercase();
    if canonical == normalized {
        return None;
    }
    if canonical.contains('/') {
        // Path-qualified deployment ids: openrouter/anthropic/…,
        // vertex_ai/…@default, bedrock/<region>/…, azure/us-gov/….
        return Some(canonical);
    }
    for marker in [".anthropic.", ".google.", ".meta.", ".amazon."] {
        if let Some(idx) = canonical.find(marker) {
            if idx > 0 {
                return Some(canonical);
            }
        }
    }
    None
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
    // rm-384: a resolved config file (CLI > --config > project > user)
    // outranks the env knob; the env knob remains the fallback for
    // library consumers and the config-free statusline host.
    let path = crate::runtime_config::get()
        .pricing_file
        .clone()
        .or_else(|| std::env::var_os("AGENTTRACE_PRICING_FILE").map(PathBuf::from))?;
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
        // rm-1053 (run bbf61fbd cycle 3, composed at integration onto
        // the landed raw-id retention): keep the RAW price name.
        // Normalizing here silently redirected a rate written against
        // a catalog row's own id — a region/vendor-qualified twin such
        // as `bedrock/us-gov-west-1/anthropic.…`, or a bare dated id
        // the snapshot itself ships — onto the shared slot it
        // normalizes onto: the winner row the user did NOT name.
        // `apply_pricing_overrides` normalizes only when the raw name
        // is not itself a catalog row, so every override file that
        // names a non-row id behaves exactly as before (rm-196).
        prices.insert(model, price);
    }
    let aliases: BTreeMap<String, String> = overrides
        .aliases
        .into_iter()
        .map(|(alias, model)| (normalize_model(&alias), normalize_model(&model)))
        .collect();
    // rm-612: `resolve_alias` walks at most ALIAS_HOP_BUDGET hops; a
    // chain that cannot reach a terminal within that budget silently
    // priced the model at fallback while the source line still
    // claimed the file applied. Reject the whole file here — the
    // same loud class as the invalid-JSON and invalid-rate arms —
    // naming the offending alias. File-internal chains only: the
    // walk sees this file's rows alone (bundled alias rows are
    // single-hop SKU redirects, rm-610). Unresolvable cycles fail
    // this arm too: they never resolved anywhere, and the source
    // line's "applied" claim was equally false. rm-968 residual
    // (run 12a73275 cycle 4, assess F3): the degenerate 1-cycle
    // (`x -> x`) failed this arm too — the assess PoC's
    // `selfevil`-aliased-to-itself file was accepted silently and
    // billed at built-in fallback, byte-identical to running with
    // no file, so the residual now fails the same loud arm.
    if let Some((alias, hops)) = alias_chain_over_budget(&aliases) {
        return Err(if hops == 0 {
            format!(
                "alias {alias:?} maps to itself, so it can never resolve to a \
                 priced model; it would silently price at fallback while this \
                 file claims to apply — drop the row or point it at a real model"
            )
        } else {
            format!(
                "alias {alias:?} cannot resolve within the {ALIAS_HOP_BUDGET}-hop budget \
                 ({hops} hops walked without reaching a priced model); it would silently \
                 price at fallback while this file claims to apply"
            )
        });
    }
    Ok((prices, aliases))
}

/// The hop budget `resolve_alias` actually walks (its loop count).
const ALIAS_HOP_BUDGET: usize = 8;

/// Walk each alias's chain one hop beyond the budget: when the
/// current key is STILL an alias after `ALIAS_HOP_BUDGET` hops, the
/// chain is unresolvable within the budget (too deep, or a cycle).
/// Returns the first offending alias (BTreeMap order — the failure
/// message is deterministic) and the hops walked. A walk that ends
/// on `hops == 0` is the rm-968 residual's 1-cycle flag: the alias's
/// own first hop is itself (`x -> x`), which resolves to the key and
/// never to a priced model.
fn alias_chain_over_budget(aliases: &BTreeMap<String, String>) -> Option<(&String, usize)> {
    for alias in aliases.keys() {
        let mut current = alias.clone();
        let mut hops = 0;
        while hops <= ALIAS_HOP_BUDGET {
            let Some(next) = aliases.get(&current) else {
                break;
            };
            if next == &current {
                // rm-968 residual (assess F3): only the FIRST-hop
                // self-encounter is the degenerate row itself — flag
                // it with hops == 0 so the caller renders the
                // maps-to-itself message. A MID-walk self-break (a
                // chain ending on a self-mapped key) keeps the old
                // exit: that terminal is priced or not by other
                // maps, and the 1-cycle row is still flagged when
                // the outer loop reaches ITS key.
                if hops == 0 {
                    return Some((alias, hops));
                }
                break;
            }
            current = next.clone();
            hops += 1;
        }
        if hops > ALIAS_HOP_BUDGET {
            return Some((alias, hops));
        }
    }
    None
}

/// Rates must be finite and non-negative before they can touch costing.
/// Mirrors the `convert_litellm` non-finite guard, but for user-supplied
/// files the rejection is named (model + field) instead of a silent skip:
/// a typo like `-3` used to flow straight into a negative session cost.
///
/// rm-492 (run 12a73275 cycle 4): rates must also sit below the sanity
/// ceiling below — a per-million rate pasted into a per-token field is
/// finite and positive, so it used to pass every arm and silently mint
/// 1e+304 session totals.
fn validate_override_price(model: &str, price: &Price) -> Result<(), String> {
    // rm-1508: cw/cr moved to the Option loop — an override file that
    // omits them now means "no published cache rate" (billed at the
    // disclosed fallback), while an explicit 0 stays a published $0.
    // An override that previously relied on omitting cw to zero cache
    // cost must now say `"cw": 0` (the CHANGELOG carries the
    // migration note).
    for (field, value) in [("input", price.input), ("output", price.output)] {
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
        if value >= ABSURD_RATE_CEILING {
            return Err(absurd_rate_error(model, field, value));
        }
    }
    // rm-610 (+ cycle-2 ultrafast rider): the tier slots are
    // Option-valued — absent means "no published tier rate";
    // present must still be finite and non-negative like every other
    // rate.
    for (field, value) in [
        ("cw", price.cw),
        ("cr", price.cr),
        ("input_flex", price.input_flex),
        ("output_flex", price.output_flex),
        ("cw_flex", price.cw_flex),
        ("cr_flex", price.cr_flex),
        ("input_ultrafast", price.input_ultrafast),
        ("output_ultrafast", price.output_ultrafast),
        ("cw_ultrafast", price.cw_ultrafast),
        ("cr_ultrafast", price.cr_ultrafast),
    ] {
        if let Some(value) = value {
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
            if value >= ABSURD_RATE_CEILING {
                return Err(absurd_rate_error(model, field, value));
            }
        }
    }
    Ok(())
}

/// rm-492: the per-token rate sanity ceiling for user override files.
/// The priciest real catalog rows sit near $0.0001/token ($100 per
/// million tokens); the ceiling sits at $10,000 per token ($10,000,000
/// per million) — four orders of magnitude above anything a SKU could
/// publish — so it can never reject a defensible rate (the landed
/// negative-rate suite, for one, pins single-digit per-token rates as
/// valid). What it does catch is the f64-extreme class: a 1e308 rate
/// is finite and positive, passed every arm, and silently minted
/// 1e+304 session totals (the assess PoC) — the usual cause is a
/// per-million rate pasted into a per-token field.
const ABSURD_RATE_CEILING: f64 = 10_000.0;

/// The named rejection for a rate at or above [`ABSURD_RATE_CEILING`]:
/// model + field + the per-token/per-million unit hint, the same
/// loud class as the non-finite and negative arms.
fn absurd_rate_error(model: &str, field: &str, value: f64) -> String {
    format!(
        "model {model:?}: {field} rate {value} is at or above the sanity ceiling \
         of ${ABSURD_RATE_CEILING:.0} per token (${:.0} per million tokens); override \
         rates are PER-TOKEN — did you paste a per-million rate?",
        ABSURD_RATE_CEILING * 1_000_000.0
    )
}

/// rm-1053: raw-first alias resolution. The caller's exact id is
/// tried against the alias map before any normalization reshapes it,
/// and an id the map has no opinion about comes back UNCHANGED so
/// downstream matchers (`match_variants`, which tries the exact form
/// before the normalized one) can hit region/vendor-qualified catalog
/// rows keyed under their raw ids. The normalized form is the second
/// seed, so aliases keyed by normalized ids (override files, codex
/// SKU aliases) keep resolving exactly as before. Returns the
/// resolved id plus whether the alias map redirected it.
fn resolve_alias_detail(model: &str, aliases: &BTreeMap<String, String>) -> (String, bool) {
    for seed in [model.to_string(), normalize_model(model)] {
        let mut current = seed;
        let mut redirected = false;
        for _ in 0..8 {
            let Some(next) = aliases.get(&current) else {
                break;
            };
            if next == &current {
                break;
            }
            current = next.clone();
            redirected = true;
        }
        if redirected {
            return (current, true);
        }
    }
    (model.to_string(), false)
}

fn resolve_alias(model: &str, aliases: &BTreeMap<String, String>) -> String {
    resolve_alias_detail(model, aliases).0
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

/// Per-token tier rate -> per-million tier slot, dropping absent,
/// zero, negative, non-finite, and overflow-scaling values (rm-610;
/// shared by the flex and ultrafast tier families).
fn tier_slot(rate: Option<f64>) -> Option<f64> {
    rate.filter(|rate| rate.is_finite() && *rate > 0.0)
        .map(|rate| rate * 1e6)
        .filter(|scaled| scaled.is_finite())
}

fn convert_litellm(raw: &[u8]) -> ConvertedCatalog {
    let Ok(Value::Object(source)) = serde_json::from_slice::<Value>(raw) else {
        return ConvertedCatalog {
            entries: BTreeMap::new(),
            providers: BTreeMap::new(),
        };
    };
    let mut selected: BTreeMap<String, (i32, Price, String)> = BTreeMap::new();
    // rm-1053 (run f2fccaa9 cycle 3, assess 53af6c9e F1): region/
    // vendor-qualified LiteLLM ids are their own SKUs, not duplicates
    // of the base model — `us.anthropic.…`, `us-gov.anthropic.…`,
    // `bedrock/us-gov-east-1/…`, `azure/us-gov/…` carry regional list
    // prices the base identity does not (the assess PoC: global
    // billed the us sibling's $3.30 against its own $3.00 row;
    // us-gov under-billed $3.60 as $3.30). Retain each qualified row
    // under its own raw id so a session naming the qualified form
    // bills at THAT row, while the same key still votes in the base
    // collapse below exactly as before — plain session ids keep the
    // base winners they have always had. True duplicates (same
    // vendor, dated snapshots of one model) stay collapsed by
    // provider priority; see `qualified_catalog_id`.
    let mut qualified: BTreeMap<String, (i32, Price, String)> = BTreeMap::new();
    // rm-1053 (run d130035f cycle 5, broad retention): every OTHER
    // key that is not its own normalized form — the dated and
    // regional twin rows the qualified class above does not cover —
    // is also retained under the exact id a session reports
    // (e.g. `claude-sonnet-4-5-20250929-v1:0`), with its own snapshot
    // row, where the raw-first ladder finds it before any normalized
    // variant. Inserted after the collapse, skipping ids that
    // already carry a winner.
    let mut qualified_rows: Vec<(String, Price, String)> = Vec::new();
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
            // rm-1508: preserve absence — `None` (no published cache
            // rate) survives the ×1e6 scaling as `None`, while an
            // explicit `0` stays `Some(0.0)` (published $0 and bills
            // as $0). Absent rates bill at the disclosed fallback,
            // never a silent $0.
            cw: model.cache_write_cost.map(|rate| rate * 1e6),
            cr: model.cache_read_cost.map(|rate| rate * 1e6),
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
            // Flex-tier slots (rm-610): scaled like the standard rates,
            // kept only when finite and positive — a zero or absent
            // flex field means "no published flex rate", never "flex
            // is free". tier_slot filters after scaling so a hostile
            // near-f64-max per-token flex cost cannot survive as inf.
            input_flex: tier_slot(model.input_cost_flex),
            output_flex: tier_slot(model.output_cost_flex),
            cw_flex: tier_slot(model.cache_write_cost_flex),
            cr_flex: tier_slot(model.cache_read_cost_flex),
            // Ultrafast-tier slots (cycle-2 rider): scaled and filtered
            // identically — zero/absent means "no published rate".
            input_ultrafast: tier_slot(model.input_cost_ultrafast),
            output_ultrafast: tier_slot(model.output_cost_ultrafast),
            cw_ultrafast: tier_slot(model.cache_write_cost_ultrafast),
            cr_ultrafast: tier_slot(model.cache_read_cost_ultrafast),
            // Priority-tier slots (rm-1556, cycle-4): scaled and filtered
            // identically — zero/absent means "no published rate".
            input_priority: tier_slot(model.input_cost_priority),
            output_priority: tier_slot(model.output_cost_priority),
            cw_priority: tier_slot(model.cache_write_cost_priority),
            cr_priority: tier_slot(model.cache_read_cost_priority),
        };
        // Hostile or overflowing catalog rates must not reach costing:
        // the 1e6 scaling can turn a near-f64-max per-token cost into
        // inf, which used to survive into reports (pass-8 F8-5). Skip
        // the entry; the model falls back to default pricing and shows
        // up in data_health as fallback_pricing.
        if !(price.input.is_finite()
            && price.output.is_finite()
            // rm-1508: an absent cache rate (None) is legitimate — only
            // a present-but-non-finite rate poisons the entry.
            && price.cw.is_none_or(f64::is_finite)
            && price.cr.is_none_or(f64::is_finite))
        {
            continue;
        }
        let priority = provider_priority(&model.provider);
        if let Some(qualified_id) = qualified_catalog_id(&key, &normalized) {
            match qualified.get(&qualified_id) {
                Some((existing, _, _)) if *existing >= priority => {}
                _ => {
                    qualified.insert(
                        qualified_id,
                        (priority, price.clone(), model.provider.clone()),
                    );
                }
            }
        } else if key != normalized {
            qualified_rows.push((key.clone(), price.clone(), model.provider.clone()));
        }
        match selected.get(&normalized) {
            Some((existing, _, _)) if *existing >= priority => {}
            _ => {
                selected.insert(normalized, (priority, price, model.provider));
            }
        }
    }
    // Qualified ids cannot collide with base identities in the real
    // catalog (base ids carry no path prefix and no regional marker;
    // tests/regional_pricing_contract.rs sweeps the whole bundled
    // snapshot and fails loudly if a refresh ever makes one collide),
    // so this merge is additive in practice. The priority guard stays
    // the same dedup discipline the collapse itself uses, so even a
    // pathological collision resolves deterministically.
    for (name, (priority, price, provider)) in qualified {
        match selected.get(&name) {
            Some((existing, _, _)) if *existing >= priority => {}
            _ => {
                selected.insert(name, (priority, price, provider));
            }
        }
    }
    let mut entries = BTreeMap::new();
    let mut providers = BTreeMap::new();
    for (name, (_, price, provider)) in selected {
        providers.insert(name.clone(), provider);
        entries.insert(name, price);
    }
    // rm-1053 (broad retention, run d130035f cycle 5): every
    // remaining key that is not its own normalized form ALSO stands
    // under its exact raw id — the dated and regional twin rows the
    // collapse above otherwise repriced at one winner's rates. The
    // rate, the provider, and the pricing-source label for that
    // exact id then all cite that one row. Insertion skips raw ids
    // that already carry a normalized winner, so normalized-form
    // lookups keep today's rows.
    for (raw, price, provider) in qualified_rows {
        if entries.contains_key(&raw) {
            continue;
        }
        providers.insert(raw.clone(), provider);
        entries.insert(raw, price);
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
    // rm-1511: clients (Antigravity CLI emits claude-sonnet-5-5-medium / -low)
    // append a reasoning-effort tier to the model id. No catalog (bundled or
    // upstream) carries effort-suffixed keys, but the exact base does — strip
    // the tier as a candidate so the base rate beats the builtin default. The
    // raw id keeps the suffix (see `effort_tier_of`), so tier-aware lanes lose
    // nothing.
    if let Some(base) = strip_effort_suffix(&normalized) {
        variants.push(base);
    }
    variants
}

/// Effort tiers clients append to model ids (rm-1511): `-low/-medium/-high`
/// (Antigravity style) and `-thinking` / `:thinking` (models.dev style).
const EFFORT_TIERS: [(&str, &str); 5] = [
    ("-medium", "medium"),
    ("-high", "high"),
    ("-low", "low"),
    ("-thinking", "thinking"),
    (":thinking", "thinking"),
];

/// The reasoning-effort tier a client suffixed onto a model id, if any
/// (`claude-sonnet-5-5-medium` -> `medium`). Pricing resolution strips the
/// tier; this re-derives it so tier-aware lanes keep their input.
///
/// Limitation (review 1a615b64 F5): the marker test is purely lexical, so
/// ids whose tier-like tail is part of a catalogued SKU also report a tier
/// (`aihubmix/gpt-5.4-high`, `azure_ai/kimi-k2-thinking` — 48 such keys in
/// the bundled snapshot). Pricing is unaffected — the exact catalog match
/// wins before any stripping — but a `Some` here is not proof that a client
/// appended an effort tier; check the catalog first when that distinction
/// matters.
pub fn effort_tier_of(model: &str) -> Option<&'static str> {
    let normalized = normalize_model(model);
    EFFORT_TIERS
        .iter()
        .find(|(marker, _)| normalized.ends_with(marker) && normalized.len() > marker.len())
        .map(|(_, tier)| *tier)
}

fn strip_effort_suffix(normalized: &str) -> Option<String> {
    let (marker, _) = EFFORT_TIERS
        .iter()
        .find(|(marker, _)| normalized.ends_with(marker) && normalized.len() > marker.len())?;
    let base = normalized[..normalized.len() - marker.len()].trim_end_matches(['-', ':']);
    if base.is_empty() {
        None
    } else {
        Some(base.to_string())
    }
}

/// For a model id with NO specific catalog price, the in-catalog base id
/// hiding under a single unrecognized trailing `-token`, if one exists
/// (`claude-sonnet-5-5-ultra` -> `claude-sonnet-5-5`). rm-1511 disclosure:
/// such sessions price at the builtin default while the exact base was
/// available, and the data-health lane names them instead of only counting.
pub fn specific_base_for_unpriced(model: &str) -> Option<String> {
    if has_specific_price(model) {
        return None;
    }
    let normalized = normalize_model(model);
    if let Some(base) = strip_effort_suffix(&normalized) {
        if has_specific_price(&base) {
            return Some(base);
        }
        return None;
    }
    let (head, tail) = normalized.rsplit_once('-')?;
    let tail_is_token =
        !tail.is_empty() && tail.len() <= 10 && tail.chars().all(|c| c.is_ascii_alphabetic());
    if tail_is_token && has_specific_price(head) {
        return Some(head.to_string());
    }
    None
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
    // rm-1053: strip suffixes to a fixed point. A single
    // strip-date-then-strip-version pass is NOT idempotent on
    // dated+versioned ids (`claude-sonnet-4-5-20250929-v1:0` ->
    // `…-20250929-v1` on pass one, `claude-sonnet-4-5` on pass two:
    // the `:0` shields the date on the first pass because the version
    // strip has not run yet), and a consumer that normalized an
    // already-normalized id silently resolved a different catalog row
    // than the rate lookup. Looping both strips until the value stops
    // changing makes `normalize_model(normalize_model(x)) ==
    // normalize_model(x)` hold by construction — each strip only
    // shortens the string, so the loop terminates.
    loop {
        let stripped = strip_version_suffix(&strip_date_suffix(&value));
        if stripped == value {
            break;
        }
        value = stripped;
    }
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
                cw: Some(6.25),
                cr: Some(0.50),
                ..Default::default()
            },
        ),
        (
            "claude-opus-4.6",
            Price {
                input: 5.0,
                output: 25.0,
                cw: Some(6.25),
                cr: Some(0.50),
                ..Default::default()
            },
        ),
        (
            "claude-opus-4-7",
            Price {
                input: 5.0,
                output: 25.0,
                cw: Some(6.25),
                cr: Some(0.50),
                ..Default::default()
            },
        ),
        (
            "claude-opus-4-6",
            Price {
                input: 5.0,
                output: 25.0,
                cw: Some(6.25),
                cr: Some(0.50),
                ..Default::default()
            },
        ),
        (
            "claude-opus-4.5",
            Price {
                input: 5.0,
                output: 25.0,
                cw: Some(6.25),
                cr: Some(0.50),
                ..Default::default()
            },
        ),
        (
            "claude-opus-4",
            Price {
                input: 15.0,
                output: 75.0,
                cw: Some(18.75),
                cr: Some(1.50),
                ..Default::default()
            },
        ),
        (
            "claude-sonnet-4.6",
            Price {
                input: 3.0,
                output: 15.0,
                cw: Some(3.75),
                cr: Some(0.30),
                ..Default::default()
            },
        ),
        (
            "claude-sonnet-4-6",
            Price {
                input: 3.0,
                output: 15.0,
                cw: Some(3.75),
                cr: Some(0.30),
                ..Default::default()
            },
        ),
        (
            "claude-sonnet-4.5",
            Price {
                input: 3.0,
                output: 15.0,
                cw: Some(3.75),
                cr: Some(0.30),
                ..Default::default()
            },
        ),
        (
            "claude-sonnet-4-5",
            Price {
                input: 3.0,
                output: 15.0,
                cw: Some(3.75),
                cr: Some(0.30),
                ..Default::default()
            },
        ),
        (
            "claude-sonnet-4",
            Price {
                input: 3.0,
                output: 15.0,
                cw: Some(3.75),
                cr: Some(0.30),
                ..Default::default()
            },
        ),
        (
            "claude-haiku-4-5",
            Price {
                input: 1.0,
                output: 5.0,
                cw: Some(1.25),
                cr: Some(0.10),
                ..Default::default()
            },
        ),
        (
            "claude-haiku-4.5",
            Price {
                input: 1.0,
                output: 5.0,
                cw: Some(1.25),
                cr: Some(0.10),
                ..Default::default()
            },
        ),
        (
            "claude-haiku-3.5",
            Price {
                input: 0.80,
                output: 4.0,
                cw: Some(1.0),
                cr: Some(0.08),
                ..Default::default()
            },
        ),
        (
            "gemini-3.1-pro-preview",
            Price {
                input: 2.0,
                output: 12.0,
                cw: Some(0.0),
                cr: Some(0.20),
                ..Default::default()
            },
        ),
        (
            "gemini-3-flash-preview",
            Price {
                input: 0.5,
                output: 3.0,
                cw: Some(0.0),
                cr: Some(0.05),
                ..Default::default()
            },
        ),
        (
            "gemini-2.5-pro",
            Price {
                input: 1.25,
                output: 10.0,
                cw: Some(0.0),
                cr: Some(0.0),
                ..Default::default()
            },
        ),
        (
            "gemini-2.5-flash",
            Price {
                input: 0.15,
                output: 0.60,
                cw: Some(0.0),
                cr: Some(0.0),
                ..Default::default()
            },
        ),
        (
            "gpt-5.5",
            Price {
                input: 5.0,
                output: 30.0,
                cw: Some(0.0),
                cr: Some(0.50),
                ..Default::default()
            },
        ),
        (
            "gpt-5.4",
            Price {
                input: 2.5,
                output: 15.0,
                cw: Some(0.0),
                cr: Some(0.25),
                ..Default::default()
            },
        ),
        (
            "pa/gpt-5.4",
            Price {
                input: 0.0,
                output: 0.0,
                cw: Some(0.0),
                cr: Some(0.0),
                ..Default::default()
            },
        ),
        (
            "gpt-5.4-mini",
            Price {
                input: 0.75,
                output: 4.5,
                cw: Some(0.0),
                cr: Some(0.075),
                ..Default::default()
            },
        ),
        (
            "gpt-5.3-codex",
            Price {
                input: 1.75,
                output: 14.0,
                cw: Some(0.0),
                cr: Some(0.175),
                ..Default::default()
            },
        ),
        (
            "gpt-5.2-codex",
            Price {
                input: 1.75,
                output: 14.0,
                cw: Some(0.0),
                cr: Some(0.175),
                ..Default::default()
            },
        ),
        (
            "gpt-5.1",
            Price {
                input: 1.25,
                output: 10.0,
                cw: Some(0.0),
                cr: Some(0.0),
                ..Default::default()
            },
        ),
        (
            "gpt-5.1-mini",
            Price {
                input: 0.25,
                output: 2.0,
                cw: Some(0.0),
                cr: Some(0.0),
                ..Default::default()
            },
        ),
        (
            "gpt-5.1-codex-mini",
            Price {
                input: 0.25,
                output: 2.0,
                cw: Some(0.0),
                cr: Some(0.025),
                ..Default::default()
            },
        ),
        (
            "gpt-4.1",
            Price {
                input: 2.0,
                output: 8.0,
                cw: Some(0.0),
                cr: Some(0.0),
                ..Default::default()
            },
        ),
        (
            "gpt-4.1-mini",
            Price {
                input: 0.40,
                output: 1.60,
                cw: Some(0.0),
                cr: Some(0.0),
                ..Default::default()
            },
        ),
        (
            "gpt-4.1-nano",
            Price {
                input: 0.10,
                output: 0.40,
                cw: Some(0.0),
                cr: Some(0.0),
                ..Default::default()
            },
        ),
        (
            "deepseek-v4-pro",
            Price {
                input: 0.435,
                output: 0.87,
                cw: Some(0.0),
                cr: Some(0.003625),
                ..Default::default()
            },
        ),
        (
            "deepseek-v4-flash",
            Price {
                input: 0.14,
                output: 0.28,
                cw: Some(0.0),
                cr: Some(0.0028),
                ..Default::default()
            },
        ),
        (
            "deepseek-chat",
            Price {
                input: 0.27,
                output: 1.10,
                cw: Some(0.07),
                cr: Some(0.014),
                ..Default::default()
            },
        ),
        (
            "deepseek-reasoner",
            Price {
                input: 0.55,
                output: 2.19,
                cw: Some(0.14),
                cr: Some(0.028),
                ..Default::default()
            },
        ),
        (
            "glm-5",
            Price {
                input: 1.0,
                output: 3.20,
                cw: Some(0.0),
                cr: Some(0.20),
                ..Default::default()
            },
        ),
        (
            "glm-5-turbo",
            Price {
                input: 1.20,
                output: 4.0,
                cw: Some(0.0),
                cr: Some(0.24),
                ..Default::default()
            },
        ),
        (
            "glm-5.1",
            Price {
                input: 1.40,
                output: 4.40,
                cw: Some(0.0),
                cr: Some(0.26),
                ..Default::default()
            },
        ),
        (
            "kimi-k2.5",
            Price {
                input: 0.60,
                output: 3.0,
                cw: Some(0.0),
                cr: Some(0.10),
                ..Default::default()
            },
        ),
        (
            "kimi-k2.6",
            Price {
                input: 0.95,
                output: 4.0,
                cw: Some(0.0),
                cr: Some(0.16),
                ..Default::default()
            },
        ),
        (
            "mimo-v2-pro",
            Price {
                input: 0.10,
                output: 0.30,
                cw: Some(0.0),
                cr: Some(0.02),
                ..Default::default()
            },
        ),
        (
            "mimo-v2.5-pro",
            Price {
                input: 0.0,
                output: 0.0,
                cw: Some(0.0),
                cr: Some(0.0),
                ..Default::default()
            },
        ),
        (
            "minimax-2.5",
            Price {
                input: 0.30,
                output: 2.40,
                cw: Some(0.375),
                cr: Some(0.03),
                ..Default::default()
            },
        ),
        (
            "minimax-2.7",
            Price {
                input: 0.30,
                output: 2.40,
                cw: Some(0.375),
                cr: Some(0.03),
                ..Default::default()
            },
        ),
        (
            "minimax-2.7-highspeed",
            Price {
                input: 0.30,
                output: 2.40,
                cw: Some(0.375),
                cr: Some(0.03),
                ..Default::default()
            },
        ),
        (
            "minimax-m2.5",
            Price {
                input: 0.30,
                output: 1.20,
                cw: Some(0.375),
                cr: Some(0.03),
                ..Default::default()
            },
        ),
        (
            "minimax-m2.5-free",
            Price {
                input: 0.30,
                output: 2.40,
                cw: Some(0.375),
                cr: Some(0.03),
                ..Default::default()
            },
        ),
        (
            "minimax-m2.7",
            Price {
                input: 0.30,
                output: 2.40,
                cw: Some(0.375),
                cr: Some(0.03),
                ..Default::default()
            },
        ),
        (
            "qwen/qwen3.6-plus-04-02:free",
            Price {
                input: 0.325,
                output: 1.95,
                cw: Some(0.40625),
                cr: Some(0.0),
                ..Default::default()
            },
        ),
        (
            "qwen3.6-plus",
            Price {
                input: 0.325,
                output: 1.95,
                cw: Some(0.40625),
                cr: Some(0.0),
                ..Default::default()
            },
        ),
        (
            "qwen3.5-plus",
            Price {
                input: 0.40,
                output: 2.40,
                cw: Some(0.0),
                cr: Some(0.0),
                ..Default::default()
            },
        ),
        (
            "qwen3.6:35b-a3b-coding-nvfp4",
            Price {
                input: 0.0,
                output: 0.0,
                cw: Some(0.0),
                cr: Some(0.0),
                ..Default::default()
            },
        ),
        (
            "doubao-seed-2-0-pro",
            Price {
                input: 0.0,
                output: 0.0,
                cw: Some(0.0),
                cr: Some(0.0),
                ..Default::default()
            },
        ),
        (
            "stepfun/step-3.5-flash:free",
            Price {
                input: 0.0,
                output: 0.0,
                cw: Some(0.0),
                cr: Some(0.0),
                ..Default::default()
            },
        ),
        (
            "grok-code-fast-1",
            Price {
                input: 0.20,
                output: 1.50,
                cw: Some(0.0),
                cr: Some(0.02),
                ..Default::default()
            },
        ),
        (
            "x-ai/grok-code-fast-1",
            Price {
                input: 0.20,
                output: 1.50,
                cw: Some(0.0),
                cr: Some(0.02),
                ..Default::default()
            },
        ),
        (
            "<synthetic>",
            Price {
                input: 0.0,
                output: 0.0,
                cw: Some(0.0),
                cr: Some(0.0),
                ..Default::default()
            },
        ),
        (
            "grok-3",
            Price {
                input: 3.0,
                output: 15.0,
                cw: Some(0.0),
                cr: Some(0.0),
                ..Default::default()
            },
        ),
        (
            "default",
            Price {
                input: 3.0,
                output: 15.0,
                cw: Some(0.0),
                cr: Some(0.0),
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

// The cache ladder lives in `crate::cache_dir` — one shared
// resolver for pricing, session_cache, doctor, and statusline
// (consolidated from this file's byte-identical copy; that copy
// resolved the macOS `Library/Caches` arm BEFORE an absolute
// $XDG_CACHE_HOME and joined a set-but-empty OR relative HOME into a
// RELATIVE `.cache` against the CWD).
#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn rm1978_models_dev_snapshot_corruption_degrades_loudly_never_panics() {
        // Red-first pin of the restructured failure mode (rm-1978
        // cohort, pricing.rs site): pre-restructure this exact input
        // PANICKED inside the OnceLock initializer (rm-1532's expect),
        // taking down any report that touched pricing. Now a malformed
        // snapshot is a named error and a degraded empty catalog — the
        // extract lane stops serving (lookups miss through the ordinary
        // unpriced-model disclosure, absence never a wrong number) and
        // [`models_dev_parse_failure`] names the cause for `--doctor`.
        let whole_file_corrupt = parse_models_dev_snapshot("{ not json").unwrap_err();
        assert!(whole_file_corrupt.contains("models_dev_snapshot.json"));
        assert!(whole_file_corrupt.contains("failed to parse"));

        let row_corrupt =
            parse_models_dev_snapshot(r#"{"_snapshot":"ok","anthropic/claude-3-5":42}"#)
                .unwrap_err();
        assert!(row_corrupt.contains("row anthropic/claude-3-5"));
        assert!(row_corrupt.contains("must parse"));

        // A snapshot of only provenance prose parses to an empty but
        // valid catalog (zero rows is legal — absence is honest).
        let prose_only = parse_models_dev_snapshot(r#"{"_snapshot":"ok"}"#).unwrap();
        assert!(prose_only.entries.is_empty());
        assert!(prose_only.providers.is_empty());

        // The bundled snapshot itself parses clean — the invariant the
        // disclosure exists to guard. A regression here fails this pin
        // instead of panicking inside a user's report.
        assert!(parse_models_dev_snapshot(MODELS_DEV_SNAPSHOT_JSON).is_ok());
    }

    #[test]
    fn pricing_cache_path_honors_session_cache_env() {
        // rm-575: the pricing catalog joins the session cache's
        // relocation contract. A non-empty AGENTTRACE_SESSION_CACHE_DIR
        // moves pricing.json beside sessions.json (and the provenance
        // stamp derives from this same path, so both relocate
        // together); an empty value stays inert exactly like the
        // session cache's ladder. Pinning XDG/HOME keeps the
        // default-root arms deterministic.
        let _env = crate::test_env::lock_env();
        let root = std::env::temp_dir().join(format!(
            "agenttrace-pricing-env-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        std::fs::create_dir_all(&root).expect("create temp dir");
        let previous_env = std::env::var_os("AGENTTRACE_SESSION_CACHE_DIR");
        let previous_home = std::env::var_os("HOME");
        let previous_xdg = std::env::var_os("XDG_CACHE_HOME");
        let default_root = root.join("home").join(".cache").join("agenttrace");
        std::env::set_var("HOME", root.join("home"));
        std::env::remove_var("XDG_CACHE_HOME");

        std::env::set_var("AGENTTRACE_SESSION_CACHE_DIR", &root);
        assert_eq!(
            pricing_cache_path(),
            root.join("pricing.json"),
            "set env must relocate the catalog beside the session cache"
        );

        std::env::set_var("AGENTTRACE_SESSION_CACHE_DIR", "");
        assert_eq!(
            pricing_cache_path(),
            default_root.join("pricing.json"),
            "empty env must fall through to the default root, matching session_cache_path"
        );

        std::env::remove_var("AGENTTRACE_SESSION_CACHE_DIR");
        assert_eq!(
            pricing_cache_path(),
            default_root.join("pricing.json"),
            "unset env must keep the default root"
        );

        match previous_env {
            Some(value) => std::env::set_var("AGENTTRACE_SESSION_CACHE_DIR", value),
            None => std::env::remove_var("AGENTTRACE_SESSION_CACHE_DIR"),
        }
        match previous_home {
            Some(value) => std::env::set_var("HOME", value),
            None => std::env::remove_var("HOME"),
        }
        match previous_xdg {
            Some(value) => std::env::set_var("XDG_CACHE_HOME", value),
            None => std::env::remove_var("XDG_CACHE_HOME"),
        }
        let _ = std::fs::remove_dir_all(&root);
    }

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
    #[cfg(unix)]
    fn pricing_cache_write_is_owner_only_and_refuses_preplanted_temps() {
        // rm-693: the catalog cache used to land through a plain
        // `fs::write` on a predictable temp sibling — a symlink planted
        // at that name was followed (the victim behind it clobbered)
        // AND the catalog plus its sidecar landed 0644. The stage must
        // be O_EXCL with a sequence bump, and both files owner-only.
        use std::os::unix::fs::PermissionsExt;
        let root = std::env::temp_dir().join(format!(
            "agenttrace-pricing-excl-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        std::fs::create_dir_all(&root).expect("create temp dir");
        let path = root.join("pricing.json");
        let victim = root.join("victim.txt");
        std::fs::write(&victim, "secret bytes").expect("write victim");
        let planted = crate::session_cache::unique_temp_path(&path);
        std::os::unix::fs::symlink(&victim, &planted).expect("plant symlink at the next temp");
        write_pricing_cache_at(&path, "{\"stub\":true}").expect("write past the planted link");
        assert_eq!(
            std::fs::read(&victim).unwrap(),
            b"secret bytes",
            "the victim behind the planted link must be untouched"
        );
        assert_eq!(
            std::fs::read_to_string(&path).unwrap(),
            "{\"stub\":true}",
            "the catalog itself must land at the destination"
        );
        let mode = std::fs::metadata(&path).unwrap().permissions().mode() & 0o777;
        assert_eq!(
            mode, 0o600,
            "catalog cache must be owner-only (got {mode:o})"
        );
        let meta_path = path.with_extension("meta.json");
        let meta_mode = std::fs::metadata(&meta_path)
            .expect("provenance sidecar exists")
            .permissions()
            .mode()
            & 0o777;
        assert_eq!(
            meta_mode, 0o600,
            "provenance sidecar must be owner-only (got {meta_mode:o})"
        );
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
    fn user_pricing_overrides_keep_raw_names_and_normalize_aliases() {
        // rm-1053 (run bbf61fbd cycle 3, composed at integration):
        // parse keeps the raw price name — normalizing here used to
        // redirect a rate written against a retained raw catalog id
        // (a region/vendor-qualified twin) onto the shared slot it
        // normalizes onto. `apply_pricing_overrides` decides where a
        // raw name lands; aliases stay normalized (they are lookup
        // keys, not catalog rows).
        let (prices, aliases) = parse_pricing_overrides(
            br#"{"prices":{"Provider/My-Model":{"input":1,"output":2,"cw":3,"cr":4}},"aliases":{"MY-ALIAS":"Provider/My-Model"}}"#,
        )
        .expect("pricing overrides");
        assert!(
            prices.contains_key("Provider/My-Model"),
            "rm-1053: parse keeps the raw price name — normalizing here used to redirect a twin-keyed override onto the shared slot"
        );
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
    fn bundled_snapshot_prices_claude_haiku_5_5_exactly() {
        // rm-006 cycle-1 refresh: claude-haiku-5-5 was missing from the
        // 2026-10-04 bundle, so a 1000-in/500-out haiku-5-5 session fell
        // back to default pricing — $0.0105 instead of the true $0.00035,
        // the 30x disclosed overestimate proven live (research 61b8a2ed,
        // /tmp/at-research-61b8/haiku/h5.jsonl). Pin the exact rates so a
        // future refresh that loses or corrupts the entry goes red here
        // instead of silently re-enabling the fallback.
        assert!(has_specific_price("claude-haiku-5-5"));
        let price = lookup_price("claude-haiku-5-5");
        // Price is per million tokens; live rates are 1e-7 in / 5e-7 out
        // / 1.25e-7 cache-write / 1e-8 cache-read per token.
        let eps = 1e-12;
        assert!((price.input - 0.1).abs() < eps, "input {}", price.input);
        assert!((price.output - 0.5).abs() < eps, "output {}", price.output);
        assert!((price.cw.unwrap() - 0.125).abs() < eps, "cw {:?}", price.cw);
        assert!((price.cr.unwrap() - 0.01).abs() < eps, "cr {:?}", price.cr);
        // 1000 input + 500 output tokens: $0.00035 in exact arithmetic,
        // disclosed as $0.0004 by token_cost's 4-decimal rounding.
        // (Tier argument `None`: the tier-less world is byte-identical.)
        assert_eq!(
            token_cost(1000, 500, 0, 0, "claude-haiku-5-5", None),
            0.0004
        );
        // The fallback this replaces charged 30x more on this workload.
        assert_eq!(
            token_cost(1000, 500, 0, 0, "nonexistent-model-xyz", None),
            0.0105
        );
    }

    #[test]
    fn bundled_snapshot_union_header_stays_honest() {
        // rm-588 never-drop contract, artifact side: the refresh writes
        // the keys that survived only through the union (upstream no
        // longer prices them; the committed bundle did) into
        // _snapshot.retained_from_previous. Keep the header honest: the
        // models count must equal the entry count and every retained key
        // must still be priced, so a hand-edit or a regressed refresh
        // cannot quietly drop prices while the header still claims them.
        let snapshot: serde_json::Value =
            serde_json::from_str(PRICING_SNAPSHOT_JSON).expect("bundled snapshot parses");
        let map = snapshot.as_object().expect("snapshot is an object");
        let entries = map.keys().filter(|k| k.as_str() != "_snapshot").count();
        let header = &snapshot["_snapshot"];
        assert_eq!(
            header["models"].as_u64(),
            Some(entries as u64),
            "_snapshot.models drifted from the bundled entry count"
        );
        let retained: Vec<&str> = header["retained_from_previous"]
            .as_array()
            .expect("retained_from_previous is a list")
            .iter()
            .map(|v| v.as_str().expect("retained key is a string"))
            .collect();
        for key in &retained {
            assert!(
                map.contains_key(*key),
                "union-retained key {key} vanished from the bundle — \
                 rm-588 never-drop contract broken"
            );
        }
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

    #[test]
    fn normalize_model_is_idempotent_across_the_dated_and_versioned_family() {
        // rm-1053: normalization must be a fixed point. The pre-fix
        // pipeline ran strip-date-then-strip-version exactly once, so
        // `claude-sonnet-4-5-20250929-v1:0` normalized to
        // `claude-sonnet-4-5-20250929-v1` on pass one (the `:0` shields
        // the date suffix because the version strip has not removed it
        // yet) and to `claude-sonnet-4-5` on pass two. Any consumer that
        // normalized an already-normalized id silently resolved a
        // different catalog row than a consumer that normalized once.
        // Pinned across every key the bundled snapshot ships, so a new
        // suffix shape cannot reintroduce the drift silently.
        let raw = serde_json::from_str::<serde_json::Value>(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/pricing_snapshot.json"
        )))
        .expect("bundled snapshot parses");
        let keys = raw
            .as_object()
            .expect("snapshot is an object")
            .keys()
            .cloned()
            .collect::<Vec<_>>();
        assert!(!keys.is_empty(), "snapshot unexpectedly empty");
        for key in &keys {
            let once = normalize_model(key);
            assert_eq!(
                normalize_model(&once),
                once,
                "normalize_model is not idempotent at snapshot key {key}: {once} -> {}",
                normalize_model(&once)
            );
        }
        // Explicit family probes beyond the snapshot: the dated+
        // versioned collision shapes, the bedrock path form, an
        // at-separated date, and a no-op id must all be fixed points.
        for probe in [
            "us.anthropic.claude-sonnet-4-5-20250929-v1:0",
            "us-gov.anthropic.claude-sonnet-4-5-20250929-v1:0",
            "claude-sonnet-4-5-20250929-v1:0",
            "anthropic.claude-3-5-haiku-20241022-v1:0",
            "bedrock/us-gov-west-1/anthropic.claude-sonnet-4-5-20250929-v1:0",
            "vertex_ai/claude-sonnet-4-5@20250929",
            "model-20250101-v2:0",
            "claude-sonnet-4-5",
        ] {
            let once = normalize_model(probe);
            assert_eq!(
                normalize_model(&once),
                once,
                "normalize_model is not idempotent at probe {probe}: {once} -> {}",
                normalize_model(&once)
            );
        }
    }

    #[test]
    fn convert_litellm_retains_region_and_vendor_qualified_rows_under_their_raw_ids() {
        // rm-1053: normalization collapses every region of a model
        // family onto one key and the winner selection then prices ALL
        // of them at one region's rates (first-sorted `au.` won the
        // sonnet-4-5 dated key at $3.30/M). A `us-gov.` session
        // billed at the `au.` rate is a false number, so a qualified
        // row must ALSO stand under the raw id a session actually
        // reports, where the raw-first matcher finds it before any
        // normalized variant. Winner selection for normalized keys is
        // untouched: normalized-form lookups keep the same rows.
        let fixture = serde_json::json!({
            "au.anthropic.claude-sonnet-4-5-20250929-v1:0": {
                "input_cost_per_token": 0.0000033,
                "output_cost_per_token": 0.0000165,
                "mode": "chat",
                "litellm_provider": "bedrock_converse"
            },
            "us-gov.anthropic.claude-sonnet-4-5-20250929-v1:0": {
                "input_cost_per_token": 0.0000036,
                "output_cost_per_token": 0.000018,
                "mode": "chat",
                "litellm_provider": "bedrock_converse"
            }
        });
        let converted = convert_litellm(
            serde_json::to_vec(&fixture)
                .expect("serialize catalog")
                .as_slice(),
        );
        // The raw ids stand in the catalog with their own rates ...
        let us_gov = converted
            .entries
            .get("us-gov.anthropic.claude-sonnet-4-5-20250929-v1:0")
            .expect("us-gov qualified row survives under its raw id");
        assert!((us_gov.input - 3.6).abs() < 1e-9);
        assert!((us_gov.output - 18.0).abs() < 1e-9);
        let au = converted
            .entries
            .get("au.anthropic.claude-sonnet-4-5-20250929-v1:0")
            .expect("au qualified row survives under its raw id");
        assert!((au.input - 3.3).abs() < 1e-9);
        // ... and the normalized winner row still exists with the
        // first-sorted winner's rates. Idempotent normalization strips
        // the dated+versioned raw id all the way to the base form, so
        // the winner selection happens under `claude-sonnet-4-5`.
        let normalized = converted
            .entries
            .get("claude-sonnet-4-5")
            .expect("normalized winner row is preserved");
        assert!((normalized.input - 3.3).abs() < 1e-9);
        // Provider coverage stays in lockstep with the entries.
        assert_eq!(
            converted.entries.keys().collect::<Vec<_>>(),
            converted.providers.keys().collect::<Vec<_>>(),
            "retained raw rows must carry their provider labels"
        );
        assert_eq!(
            converted
                .providers
                .get("us-gov.anthropic.claude-sonnet-4-5-20250929-v1:0")
                .map(String::as_str),
            Some("bedrock_converse")
        );
    }

    #[test]
    fn exact_key_overrides_correct_the_twin_row() {
        // rm-1053 override lane (run bbf61fbd cycle 3, composed at
        // integration onto the landed raw-id retention): a user rate
        // naming a retained raw catalog id — a regional twin kept
        // under its own row by the conversion — corrects THAT row
        // alone; the shared slot the id normalizes onto keeps its
        // winner. Rate, provider, and provenance still cite one row
        // (the user's).
        let fixture = serde_json::json!({
            "anthropic.claude-sonnet-4-5-20250929-v1:0": {
                "input_cost_per_token": 0.000003,
                "output_cost_per_token": 0.000015,
                "mode": "chat",
                "litellm_provider": "anthropic"
            },
            "bedrock/us-gov-west-1/anthropic.claude-sonnet-4-5-20250929-v1:0": {
                "input_cost_per_token": 0.0000036,
                "output_cost_per_token": 0.000018,
                "mode": "chat",
                "litellm_provider": "bedrock"
            }
        });
        let converted = convert_litellm(
            serde_json::to_vec(&fixture)
                .expect("serialize catalog")
                .as_slice(),
        );
        let mut catalog = PricingCatalog {
            entries: converted.entries,
            aliases: BTreeMap::new(),
            source: "cache".to_string(),
            providers: converted.providers,
            reference_date: iso_to_ymd(PRICING_SNAPSHOT_DATE),
        };
        let twin = "bedrock/us-gov-west-1/anthropic.claude-sonnet-4-5-20250929-v1:0";
        // Pre-override: the twin already bills its own retained row
        // (the landed rm-1053 mechanics this lane composes onto).
        let before = lookup_price_in(twin, &catalog.aliases, &catalog.entries);
        assert!((before.input - 3.6).abs() < 1e-9);

        let root = std::env::temp_dir().join(format!(
            "agenttrace-pricing-twin-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        std::fs::create_dir_all(&root).expect("create temp dir");
        let override_file = root.join("pricing.json");
        std::fs::write(
            &override_file,
            serde_json::json!({
                "prices": {twin: {"input": 4.0, "output": 20.0, "cw": 0, "cr": 0}}
            })
            .to_string(),
        )
        .expect("write override file");

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

        // The twin's own row carries the user's rates, provider, and
        // source — all three surfaces cite one row.
        let price = lookup_price_in(twin, &catalog.aliases, &catalog.entries);
        assert_eq!((price.input, price.output), (4.0, 20.0));
        assert_eq!(provider_for_in(twin, &catalog), Some("bedrock".to_string()));
        assert_eq!(
            pricing_source_for_catalog(twin, &catalog, &override_models),
            "user override"
        );
        assert!(override_models.contains(twin));
        // ... while the shared slot the twin normalizes onto keeps its
        // winner: a twin-scoped override corrects one row, not the
        // family.
        let slot = "anthropic.claude-sonnet-4-5";
        let slot_price = catalog.entries.get(slot).expect("shared slot row");
        assert!((slot_price.input - 3.0).abs() < 1e-9);
        let base = lookup_price_in(slot, &catalog.aliases, &catalog.entries);
        assert!((base.input - 3.0).abs() < 1e-9);
    }

    #[test]
    fn region_qualified_ids_price_and_attribute_from_their_own_catalog_row() {
        // rm-1053 (assess F2 refresh, cycle 5): four regional fixtures
        // of one model family all rendered $19.80 — the load-time
        // collapse priced us-gov at the au rate ($19.80 vs true $21.60)
        // and the provider path, which normalized the id twice,
        // printed "anthropic" while the rate came from a
        // bedrock_converse row. Both surfaces must cite ONE row: the
        // exact id's own snapshot row.
        let catalog = fallback_catalog();
        let cases = [
            // (session model id, $/Mtok in, $/Mtok out, provider)
            (
                "us.anthropic.claude-sonnet-4-5-20250929-v1:0",
                3.3,
                16.5,
                "bedrock_converse",
            ),
            (
                "au.anthropic.claude-sonnet-4-5-20250929-v1:0",
                3.3,
                16.5,
                "bedrock_converse",
            ),
            (
                "eu.anthropic.claude-sonnet-4-5-20250929-v1:0",
                3.3,
                16.5,
                "bedrock_converse",
            ),
            (
                "jp.anthropic.claude-sonnet-4-5-20250929-v1:0",
                3.3,
                16.5,
                "bedrock_converse",
            ),
            (
                "us-gov.anthropic.claude-sonnet-4-5-20250929-v1:0",
                3.6,
                18.0,
                "bedrock_converse",
            ),
            (
                "global.anthropic.claude-sonnet-4-5-20250929-v1:0",
                3.0,
                15.0,
                "bedrock_converse",
            ),
            (
                "bedrock/us-gov-east-1/anthropic.claude-sonnet-4-5-20250929-v1:0",
                3.6,
                18.0,
                "bedrock",
            ),
            // Undecorated family ids keep the base row.
            ("claude-sonnet-4-5-20250929-v1:0", 3.0, 15.0, "bedrock"),
            ("claude-sonnet-4-5", 3.0, 15.0, "anthropic"),
        ];
        for (model, input, output, provider) in cases {
            // Resolve exactly like the production rate path — one
            // raw-first ladder call — so the pin cannot pass through a
            // different resolution chain than a real session does.
            let price = lookup_price_in(model, &catalog.aliases, &catalog.entries);
            assert!(
                (price.input - input).abs() < 1e-9 && (price.output - output).abs() < 1e-9,
                "{model} priced from the wrong row: got ({}, {}), expected ({input}, {output})",
                price.input,
                price.output
            );
            assert_eq!(
                provider_for_in(model, &catalog),
                Some(provider.to_string()),
                "{model} attributed to the wrong provider"
            );
            // The single-row invariant: the rate path and the provider
            // path resolve the SAME catalog key — the model's own raw
            // id for qualified rows, its normalized winner otherwise.
            let key = matching_catalog_key_for(model, &catalog.aliases, &catalog.entries)
                .expect("row key resolves");
            assert_eq!(
                catalog.providers.get(&key).map(String::as_str),
                Some(provider),
                "resolved row {key} must carry the provider printed for {model}"
            );
            assert!(
                (catalog.entries[&key].input - input).abs() < 1e-9
                    && (catalog.entries[&key].output - output).abs() < 1e-9,
                "resolved row {key} must carry the rate billed for {model}"
            );
        }
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
        // The vendor re-published claude-sonnet-4-5's window at 200k
        // in the 2026-10-17 snapshot (was 1M through 2026-10-08); the
        // pin follows the vendor, the ingestion claim stays the same.
        assert_eq!(
            catalog.entries["claude-sonnet-4-5"].max_input_tokens,
            Some(200_000)
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

    // --- rm-610: flex-tier rates, SKU redirects, tiered lookup ---------

    #[test]
    fn flex_slots_ingest_scale_and_filter() {
        let raw = br#"{"m": {
            "mode": "chat",
            "input_cost_per_token": 4e-6,
            "output_cost_per_token": 2e-5,
            "input_cost_per_token_flex": 2e-6,
            "output_cost_per_token_flex": 0,
            "cache_read_input_token_cost_flex": -1,
            "input_cost_per_token_above_272k_tokens_flex": 9e-6
        }}"#;
        let converted = convert_litellm(raw);
        let price = &converted.entries["m"];
        assert_eq!(price.input, 4.0, "standard input scales to per-million");
        assert_eq!(price.input_flex, Some(2.0), "flex input scales too");
        assert_eq!(
            price.output_flex, None,
            "a zero flex rate is 'no flex rate'"
        );
        assert_eq!(price.cr_flex, None, "a negative flex rate is dropped");
        assert_eq!(
            price.cw_flex, None,
            "the _above_272k long-context variant is deliberately not ingested"
        );
    }

    #[test]
    fn tiered_lookup_uses_flex_only_for_flex() {
        // 4e-6 * 1e6 is not exactly 0.4 in binary — compare with a
        // tolerance instead of == on the cache slots.
        fn near(a: f64, b: f64) -> bool {
            (a - b).abs() < 1e-9
        }
        let standard = lookup_price_tiered("gpt-5.6", None);
        let flex = lookup_price_tiered("gpt-5.6", Some("flex"));
        assert_eq!((standard.input, standard.output), (4.0, 20.0));
        assert!(near(standard.cw.unwrap(), 5.0) && near(standard.cr.unwrap(), 0.4));
        // Codeburn #1642 published numbers for GPT-5.6: flex is half.
        assert_eq!((flex.input, flex.output), (2.0, 10.0));
        assert!(near(flex.cw.unwrap(), 2.5) && near(flex.cr.unwrap(), 0.2));
        // rm-1556 (cycle-4): the refreshed catalog publishes a
        // priority row for gpt-5.6 at exactly 2x standard
        // (8e-6/4e-5 per token), so priority now BILLS the published
        // priority rate instead of the pre-refresh standard fallback.
        // "fast" stays standard for bare gpt-5.6 — the ccusage
        // override table carries only the -sol/-terra/-luna variants.
        for tier in ["default", "fast", "scale", "garbage"] {
            let priced = lookup_price_tiered("gpt-5.6", Some(tier));
            assert_eq!(
                (priced.input, priced.output),
                (4.0, 20.0),
                "tier {tier} is not flex"
            );
        }
        assert_eq!(
            {
                let p = lookup_price_tiered("gpt-5.6", Some("priority"));
                (p.input, p.output)
            },
            (8.0, 40.0),
            "priority bills the published 2x priority row"
        );
        // A model without flex slots bills flex at standard rates.
        let no_flex = lookup_price_tiered("deepseek-chat", Some("flex"));
        let plain = lookup_price("deepseek-chat");
        assert_eq!((no_flex.input, no_flex.output), (plain.input, plain.output));
    }

    #[test]
    fn codex_sku_redirects_cover_missing_rows() {
        // All three ids absent from LiteLLM's chat catalog; each must
        // resolve to its published equivalent (codeburn #1642).
        assert_eq!(
            (
                lookup_price("gpt-5.6-codex").input,
                lookup_price("gpt-5.6-codex").output
            ),
            (
                lookup_price("gpt-5.6").input,
                lookup_price("gpt-5.6").output
            ),
            "the codex-tuned GPT-5.6 prices as GPT-5.6"
        );
        assert_eq!(
            (
                lookup_price("gpt-reserve").input,
                lookup_price("gpt-reserve").output
            ),
            (
                lookup_price("gpt-5.6-luna").input,
                lookup_price("gpt-5.6-luna").output
            ),
            "Luna Reserve prices as GPT-5.6 Luna"
        );
        let spark = lookup_price("gpt-5.3-spark");
        assert_eq!(
            (spark.input, spark.output),
            (1.75, 14.0),
            "Codex Spark prices as GPT-5.3 Codex"
        );
        // A redirect retires itself when a real row ships.
        let mut catalog = PricingCatalog {
            entries: BTreeMap::from([("gpt-reserve".to_string(), Price::default())]),
            aliases: BTreeMap::new(),
            source: "test".to_string(),
            providers: BTreeMap::new(),
            reference_date: None,
        };
        apply_codex_sku_aliases(&mut catalog);
        assert!(
            !catalog.aliases.contains_key("gpt-reserve"),
            "a direct entry must not be shadowed by a redirect"
        );
    }

    #[test]
    fn codex_sku_entry_is_verified_and_retires_on_upstream_ship() {
        // The gpt-5.3-codex row LiteLLM publishes only under
        // mode:"responses" (excluded by the chat filter), lifted
        // code-side with drop-on-upstream-ship discipline.
        let mut catalog = PricingCatalog {
            entries: BTreeMap::new(),
            aliases: BTreeMap::new(),
            source: "test".to_string(),
            providers: BTreeMap::new(),
            reference_date: None,
        };
        apply_codex_sku_entries(&mut catalog);
        let row = &catalog.entries["gpt-5.3-codex"];
        // rm-1508: the vend row matches upstream's published truth —
        // an explicit $0 cache-write (a published zero, distinct from
        // an absent rate) and a $0.175 cache-read rate.
        assert_eq!(
            (row.input, row.output, row.cw, row.cr),
            (1.75, 14.0, Some(0.0), Some(0.175)),
            "the vend row carries LiteLLM's own gpt-5.3-codex rates (explicit $0 cache-write)"
        );
        assert_eq!(catalog.providers["gpt-5.3-codex"], "openai");
        // Retire: an upstream chat row (or a user override merged
        // earlier) is never clobbered by the vend.
        let mut shipped = PricingCatalog {
            entries: BTreeMap::from([(
                "gpt-5.3-codex".to_string(),
                Price {
                    input: 9.0,
                    ..Price::default()
                },
            )]),
            aliases: BTreeMap::new(),
            source: "test".to_string(),
            providers: BTreeMap::new(),
            reference_date: None,
        };
        apply_codex_sku_entries(&mut shipped);
        assert_eq!(
            shipped.entries["gpt-5.3-codex"].input, 9.0,
            "a real upstream row must win over the vend"
        );
    }

    #[test]
    fn daybreak_responses_rows_are_pinned_exact() {
        // rm-1570 (run fd0e91a2 cycle 3, money truth): codex 0.163.0's
        // DEFAULT family billed against the disclosed $3/$15 default
        // (~4.86x UNDERSTATEMENT on daybreak-red — assess 2bdc99b3's
        // live PoC billed an $87.50 session $18.00) because
        // LiteLLM publishes the gpt-daybreak family only under mode
        // "responses", which the chat-mode snapshot filter excludes.
        // The pin mirrors the published per-token rates exactly (live
        // capture 2026-10-17: input 1.25e-5, output 7.5e-5, cache
        // write 1.5625e-5, cache read 1.25e-6 red; input 4e-6, output
        // 2e-5, cache write 5e-6, cache read 4e-7 blue — identical to
        // the daybreak-*-latest chat twins already bundled) and
        // retires per the rm-610 drop-on-upstream-ship discipline.
        let _env = crate::test_env::lock_env();
        let mut catalog = PricingCatalog {
            entries: BTreeMap::new(),
            aliases: BTreeMap::new(),
            source: "test".to_string(),
            providers: BTreeMap::new(),
            reference_date: None,
        };
        catalog.entries.insert(
            "already-shipped".to_string(),
            Price {
                input: 1.0,
                output: 2.0,
                ..Price::default()
            },
        );
        apply_daybreak_responses_entries(&mut catalog);
        for (id, input, output, cw, cr) in [
            ("gpt-daybreak-red-latest", 12.5, 75.0, 15.625, 1.25),
            ("gpt-daybreak-blue-latest", 4.0, 20.0, 5.0, 0.4),
        ] {
            let price = catalog
                .entries
                .get(id)
                .unwrap_or_else(|| panic!("{id} missing from pinned catalog"));
            assert_eq!(price.input, input, "{id} input");
            assert_eq!(price.output, output, "{id} output");
            assert_eq!(price.cw, Some(cw), "{id} cache write");
            assert_eq!(price.cr, Some(cr), "{id} cache read");
            assert_eq!(catalog.providers[id], "openai", "{id} provider row");
        }
        assert!(
            catalog
                .entries
                .get("already-shipped")
                .is_some_and(|price| price.input == 1.0),
            "or_insert: a snapshot-shipped row always wins over the pin"
        );
        // The shipped bundle carries the family too: both catalog
        // build sites pin, so the codex default resolves exact through
        // has_specific_price instead of the disclosed default (the
        // assess PoC's fallback arm).
        let shipped = load_catalog_for_current_env();
        assert!(shipped.entries.contains_key("gpt-daybreak-red-latest"));
        assert!(shipped.entries.contains_key("gpt-daybreak-blue-latest"));
        assert!(has_specific_price("gpt-daybreak-red-latest"));
        assert!(has_specific_price("gpt-daybreak-blue-latest"));
        // Review fix 9ce27e5c (F5): the pin resolves a real token bill
        // exactly — 1M in + 1M out is $87.50 red / $24.00 blue, against
        // the $18.00 the disclosed $3/$15 default billed the SAME
        // traffic (the PoC's measured ~4.86x understatement ratio).
        assert_eq!(
            token_cost(1_000_000, 1_000_000, 0, 0, "gpt-daybreak-red-latest", None),
            87.5
        );
        assert_eq!(
            token_cost(1_000_000, 1_000_000, 0, 0, "gpt-daybreak-blue-latest", None),
            24.0
        );
        assert_eq!(
            token_cost(1_000_000, 1_000_000, 0, 0, "zzz-unknown-xyz", None),
            18.0,
            "the fallback default's own 1M+1M bill — the understated side of the 4.86x ratio"
        );
    }

    #[test]
    fn vend_rows_stay_named_locally_defined() {
        // Composition pin (conflict case bcdd1a88): every id the
        // code-side vend inserters publish must sit in
        // `code_vended_pricing_ids`, or `is_locally_defined` flags the
        // model's own vend row as fabricated pricing — the rm-1570
        // daybreak regression this merge review closed (codex 0.163's
        // DEFAULT model set carried exact hand-pinned rates yet
        // disclosed `locally_defined_model_pricing:` on every session).
        let mut catalog = PricingCatalog {
            entries: BTreeMap::new(),
            aliases: BTreeMap::new(),
            source: "test".to_string(),
            providers: BTreeMap::new(),
            reference_date: None,
        };
        apply_codex_sku_entries(&mut catalog);
        apply_daybreak_responses_entries(&mut catalog);
        assert!(!catalog.entries.is_empty(), "vend inserters ran");
        for id in catalog.entries.keys() {
            assert!(
                code_vended_pricing_ids().contains(&id.as_str()),
                "vend row {id} is missing from code_vended_pricing_ids"
            );
        }
    }

    #[test]
    fn token_cost_applies_the_tier() {
        // 1M input, nothing else: $2 at flex, $4 at standard.
        assert_eq!(token_cost(1_000_000, 0, 0, 0, "gpt-5.6", Some("flex")), 2.0);
        assert_eq!(token_cost(1_000_000, 0, 0, 0, "gpt-5.6", None), 4.0);
    }

    #[test]
    fn override_files_validate_flex_rates() {
        let raw = br#"{"prices": {"m": {"input": 1, "output": 2, "cw": 0, "cr": 0, "input_flex": -0.5}}}"#;
        let err = parse_pricing_overrides(raw).unwrap_err();
        assert!(
            err.contains("input_flex") && err.contains("negative"),
            "got: {err}"
        );
        let raw = br#"{"prices": {"m": {"input": 1, "output": 2, "cw": 0, "cr": 0, "input_flex": 0.5, "output_flex": 0.25}}}"#;
        let (prices, _) = parse_pricing_overrides(raw).unwrap();
        assert_eq!(
            (prices["m"].input_flex, prices["m"].output_flex),
            (Some(0.5), Some(0.25)),
            "override flex slots are accepted when finite and positive"
        );
    }

    #[test]
    fn test_match_discloses_flex_rates() {
        let text = render_test_match();
        assert!(text.contains("gpt-5.6"), "the flex probe model is listed");
        assert!(
            text.contains("flex:"),
            "flex rates are disclosed on their own line"
        );
    }

    // --- rm-612: alias-chain hop budget ---------------------------------

    #[test]
    fn alias_chain_within_budget_applies() {
        // 7 hops: deepseek-chat -> ... -> gpt-4.1 (the assess PoC file).
        let raw = include_str!("../tests/fixtures/pricing-overrides/chain7.json");
        let (_, aliases) = parse_pricing_overrides(raw.as_bytes()).unwrap();
        assert_eq!(aliases.len(), 7);
        assert_eq!(
            resolve_alias("deepseek-chat", &aliases),
            "gpt-4.1",
            "a 7-hop chain resolves inside the budget"
        );
    }

    #[test]
    fn alias_chain_over_budget_is_rejected_loudly() {
        // 9 hops: silently priced at fallback before rm-612.
        let raw = include_str!("../tests/fixtures/pricing-overrides/chain9.json");
        let err = parse_pricing_overrides(raw.as_bytes()).unwrap_err();
        assert!(
            err.contains("8-hop budget") && err.contains("deepseek-chat"),
            "the rejection names the alias and the budget: {err}"
        );
    }

    #[test]
    fn alias_cycles_are_rejected_not_silently_fallback() {
        let raw = br#"{"aliases": {"a": "b", "b": "a"}}"#;
        let err = parse_pricing_overrides(raw).unwrap_err();
        assert!(
            err.contains("8-hop budget"),
            "a cycle never resolves: {err}"
        );
    }

    #[test]
    fn alias_self_cycle_is_rejected_loudly() {
        // rm-968 residual (run 12a73275 cycle 4, assess F3): the
        // degenerate 1-cycle used to pass validation — the walk broke
        // on `next == current` at hop 0 — and the model silently
        // priced at built-in fallback, byte-identical to running with
        // no file at all (the assess PoC: cost 0.0105 and
        // PricingSource=built-in fallback both with and without the
        // file). Same loud arm as the budget-overrun chains now.
        let raw = include_str!("../tests/fixtures/pricing-overrides/self.json");
        let err = parse_pricing_overrides(raw.as_bytes()).unwrap_err();
        assert!(
            err.contains("selfevil") && err.contains("maps to itself"),
            "the rejection names the self-alias: {err}"
        );
    }

    #[test]
    fn alias_self_cycle_terminating_a_chain_is_caught_by_its_own_row() {
        // a -> x -> x: walking `a` exits on the mid-walk self-break at
        // hop 1 (the old behavior, correct for a chain ENDING on a
        // self-mapped key), but the degenerate row `x -> x` is itself
        // the failure — the outer loop flags it when it reaches `x`,
        // so the file is still rejected whole instead of letting `a`
        // resolve onto an unpriced `x` at fallback.
        let raw = br#"{"aliases": {"a": "x", "x": "x"}}"#;
        let err = parse_pricing_overrides(raw).unwrap_err();
        assert!(
            err.contains("\"x\"") && err.contains("maps to itself"),
            "the 1-cycle row is the named offender: {err}"
        );
    }

    #[test]
    fn absurd_override_rates_are_rejected_loudly() {
        // rm-492: a 1e308 rate is finite and positive, so it passed
        // every landed arm and silently minted 1e+304 session totals
        // (the assess PoC). The ceiling arm rejects it with the
        // per-token/per-million unit hint.
        let raw = include_str!("../tests/fixtures/pricing-overrides/absurd.json");
        let err = parse_pricing_overrides(raw.as_bytes()).unwrap_err();
        assert!(
            err.contains("absurd") && err.contains("sanity ceiling"),
            "names the model and the unit hint: {err}"
        );
        assert!(
            err.to_lowercase().contains("per-token"),
            "carries the per-token/per-million unit hint: {err}"
        );
        // The flex slots join the same ceiling arm (they are rates
        // too, and a per-million paste lands there just as easily).
        let raw =
            br#"{"prices": {"m": {"input": 1, "output": 2, "cw": 0, "cr": 0, "input_flex": 1e308}}}"#;
        let err = parse_pricing_overrides(raw).unwrap_err();
        assert!(
            err.contains("input_flex") && err.contains("sanity ceiling"),
            "flex slots hit the same named ceiling: {err}"
        );
        // And the ceiling is a ceiling, not a clamp: a single-digit
        // per-token rate (the landed negative-rate suite's own valid
        // example) stays a numeric choice.
        let raw = br#"{"prices": {"m": {"input": 5, "output": 0.5, "cw": 0, "cr": 0}}}"#;
        let (prices, _) = parse_pricing_overrides(raw).unwrap();
        assert_eq!(prices["m"].input, 5.0);
    }

    // --- rm-613: catalog identity covers flex slots and providers -------

    #[test]
    fn ultrafast_slots_ingest_scale_filter_and_price() {
        // Cycle-2 rider (run b5d391ec, roadmap rm-164 refresh): the
        // third tier family. The bundled snapshot carries the live
        // LiteLLM ultrafast fields for gpt-6-astra (source md5
        // 836ab347216484198f80e3cce0cadfd0, fetched 2026-10-12), so
        // the public API must price an ultrafast turn at the
        // published rates, not the standard row it silently billed
        // at before.
        let converted = convert_litellm(PRICING_SNAPSHOT_JSON.as_bytes());
        let astra = &converted.entries["gpt-6-astra"];
        assert_eq!(astra.input, 10.0, "standard input scales to per-million");
        assert_eq!(
            astra.input_ultrafast,
            Some(60.0),
            "6e-5 ultrafast input scales to 60/M"
        );
        assert_eq!(astra.output_ultrafast, Some(300.0));
        assert_eq!(astra.cw_ultrafast, Some(75.0));
        assert_eq!(astra.cr_ultrafast, Some(6.0));
        assert_eq!(
            astra.input_flex,
            Some(5.0),
            "the flex row is untouched by the ultrafast addition"
        );
        let raw = br#"{"m": {
            "mode": "chat",
            "input_cost_per_token": 4e-6,
            "output_cost_per_token": 2e-5,
            "input_cost_per_token_ultrafast": 2e-6,
            "output_cost_per_token_ultrafast": 0,
            "cache_read_input_token_cost_ultrafast": -1,
            "input_cost_per_token_above_272k_tokens_ultrafast": 9e-6
        }}"#;
        let converted = convert_litellm(raw);
        let price = &converted.entries["m"];
        assert_eq!(price.input, 4.0);
        assert_eq!(price.input_ultrafast, Some(2.0), "ultrafast scales too");
        assert_eq!(price.output_ultrafast, None, "a zero rate is 'no rate'");
        assert_eq!(price.cr_ultrafast, None, "a negative rate is dropped");
        assert_eq!(
            price.cw_ultrafast, None,
            "the _above_272k ultrafast variant is deliberately not ingested"
        );
    }

    #[test]
    fn tiered_lookup_uses_ultrafast_only_for_ultrafast() {
        // Published LiteLLM numbers for gpt-6-astra: standard
        // $10/$50, flex $5/$25, ultrafast $60/$300 per million.
        let standard = lookup_price_tiered("gpt-6-astra", None);
        let ultrafast = lookup_price_tiered("gpt-6-astra", Some("ultrafast"));
        let flex = lookup_price_tiered("gpt-6-astra", Some("flex"));
        assert_eq!((standard.input, standard.output), (10.0, 50.0));
        assert_eq!((ultrafast.input, ultrafast.output), (60.0, 300.0));
        assert!((ultrafast.cw.unwrap() - 75.0).abs() < 1e-9);
        assert!((ultrafast.cr.unwrap() - 6.0).abs() < 1e-9);
        assert_eq!(
            (flex.input, flex.output),
            (5.0, 25.0),
            "flex is not ultrafast"
        );
        // rm-1556 (cycle-4): the refreshed catalog publishes a
        // priority row for gpt-6-astra at 2x standard ($20/$100), so
        // priority bills it; "fast" ALSO bills 2x here because the
        // ccusage fast-multiplier override table pins gpt-6-astra at
        // 2.0 exactly. "default"/"scale" keep the standard row.
        for tier in ["default", "scale"] {
            let priced = lookup_price_tiered("gpt-6-astra", Some(tier));
            assert_eq!((priced.input, priced.output), (10.0, 50.0), "tier {tier}");
        }
        assert_eq!(
            {
                let p = lookup_price_tiered("gpt-6-astra", Some("priority"));
                (p.input, p.output)
            },
            (20.0, 100.0),
            "priority bills the published 2x priority row"
        );
        assert_eq!(
            {
                let p = lookup_price_tiered("gpt-6-astra", Some("fast"));
                (p.input, p.output)
            },
            (20.0, 100.0),
            "fast bills the ccusage-published 2.0x multiplier"
        );
        // A model without ultrafast slots bills ultrafast at standard.
        let no_uf = lookup_price_tiered("deepseek-chat", Some("ultrafast"));
        let plain = lookup_price("deepseek-chat");
        assert_eq!((no_uf.input, no_uf.output), (plain.input, plain.output));
    }

    #[test]
    fn rm1556_priority_slots_ingest_scale_filter_and_bill() {
        // rm-1556 (cycle-4, run 1fb751544907): the fourth tier family.
        // The bundled 2026-10-17 snapshot publishes the full priority
        // row for gpt-5.6 at exactly 2x standard ($8/$40, cache $10/
        // $0.80 per million), so the loader must ingest, 1e6-scale,
        // and bill it — a priority turn previously fell through to
        // the standard row silently.
        let converted = convert_litellm(PRICING_SNAPSHOT_JSON.as_bytes());
        let gpt = &converted.entries["gpt-5.6"];
        assert_eq!(
            gpt.input_priority,
            Some(8.0),
            "priority input scales per-million"
        );
        assert_eq!(gpt.output_priority, Some(40.0));
        assert_eq!(gpt.cw_priority, Some(10.0));
        assert!(
            matches!(gpt.cr_priority, Some(v) if (v - 0.8).abs() < 1e-9),
            "priority cache-read scales per-million: {:?}",
            gpt.cr_priority
        );
        // Ingest contract mirrors flex/ultrafast exactly: zero and
        // negative rates mean "no published rate", and the
        // `_above_272k_tokens_priority` long-context variant is
        // deliberately not ingested (the rm-610 decision, held for
        // every tier family).
        let raw = br#"{"m": {
            "mode": "chat",
            "input_cost_per_token": 4e-6,
            "output_cost_per_token": 2e-5,
            "input_cost_per_token_priority": 2e-6,
            "output_cost_per_token_priority": 0,
            "cache_creation_input_token_cost_priority": -1,
            "cache_read_input_token_cost_priority": 8e-7,
            "input_cost_per_token_above_272k_tokens_priority": 9e-6
        }}"#;
        let converted = convert_litellm(raw);
        let price = &converted.entries["m"];
        assert_eq!(
            price.input_priority,
            Some(2.0),
            "priority scales like every tier family"
        );
        assert_eq!(price.output_priority, None, "a zero rate is 'no rate'");
        assert_eq!(price.cw_priority, None, "a negative rate is dropped");
        assert!(
            matches!(price.cr_priority, Some(v) if (v - 0.8).abs() < 1e-9),
            "positive priority cache rates scale like every tier family: {:?}",
            price.cr_priority
        );
        assert_eq!(price.input, 4.0, "the standard row is untouched");
        // Bill: the public tiered API bills all four axes of the
        // published priority row.
        let priority = lookup_price_tiered("gpt-5.6", Some("priority"));
        assert_eq!((priority.input, priority.output), (8.0, 40.0));
        // rm-1508 composition (wall, post-base): the published cache
        // slots are Option here; unwrap() keeps the pin strict.
        assert!((priority.cw.unwrap() - 10.0).abs() < 1e-9);
        assert!((priority.cr.unwrap() - 0.8).abs() < 1e-9);
        // A vendor-qualified row with its own published priority
        // fields bills them (azure/gpt-5.6-luna: 2x of its own
        // standard, not the base row's absolute numbers).
        let qualified = lookup_price_tiered("azure/gpt-5.6-luna", Some("priority"));
        assert!(
            (qualified.input - 0.2 * 2.0).abs() < 1e-9
                && (qualified.output - 1.2 * 2.0).abs() < 1e-9,
            "the qualified row bills its own published 2x priority numbers"
        );
    }

    #[test]
    fn rm1556_priority_zero_vs_absent_input_parity() {
        // The test-first None-parity guard the stewardship request
        // pinned BEFORE the snapshot swap: a zero published priority
        // rate and an absent priority field mean the same thing — no
        // published rate — so both must ingest to None and a priority
        // turn of a model without priority slots bills the standard
        // row, never zero and never silently repriced.
        let raw = br#"{
            "zero": {
                "mode": "chat",
                "input_cost_per_token": 4e-6,
                "output_cost_per_token": 2e-5,
                "input_cost_per_token_priority": 0,
                "output_cost_per_token_priority": 0
            },
            "absent": {
                "mode": "chat",
                "input_cost_per_token": 4e-6,
                "output_cost_per_token": 2e-5
            }
        }"#;
        let converted = convert_litellm(raw);
        let zero = &converted.entries["zero"];
        let absent = &converted.entries["absent"];
        assert_eq!(zero.input_priority, None, "zero publishes no rate");
        assert_eq!(absent.input_priority, None);
        assert_eq!(zero.output_priority, absent.output_priority);
        assert_eq!((zero.input, zero.output), (absent.input, absent.output));
        // End-to-end: deepseek-chat carries no priority slots in the
        // 2026-10-17 catalog, so a priority turn bills standard.
        let priority = lookup_price_tiered("deepseek-chat", Some("priority"));
        let standard = lookup_price("deepseek-chat");
        assert_eq!(
            (priority.input, priority.output),
            (standard.input, standard.output)
        );
    }

    #[test]
    fn rm1556_priority_zero_vs_absent_cache_parity() {
        // Same guard on the cache axes: zero-published and absent
        // priority cache rates both mean "no rate", and when a model
        // publishes priority input/output but no priority cache
        // rates, the priority turn keeps the STANDARD cache rates on
        // those axes (the flex/ultrafast partial-row discipline).
        let raw = br#"{
            "zero": {
                "mode": "chat",
                "input_cost_per_token": 4e-6,
                "output_cost_per_token": 2e-5,
                "cache_creation_input_token_cost": 5e-6,
                "cache_read_input_token_cost": 4e-7,
                "cache_creation_input_token_cost_priority": 0,
                "cache_read_input_token_cost_priority": 0
            },
            "absent": {
                "mode": "chat",
                "input_cost_per_token": 4e-6,
                "output_cost_per_token": 2e-5,
                "cache_creation_input_token_cost": 5e-6,
                "cache_read_input_token_cost": 4e-7
            }
        }"#;
        let converted = convert_litellm(raw);
        let zero = &converted.entries["zero"];
        let absent = &converted.entries["absent"];
        assert_eq!(zero.cw_priority, None, "zero publishes no cache rate");
        assert_eq!(zero.cr_priority, None);
        assert_eq!(zero.cw_priority, absent.cw_priority);
        assert_eq!(zero.cr_priority, absent.cr_priority);
        // gpt-5.5 publishes priority input/output/cache-read but no
        // priority cache-creation rate in the 2026-10-17 catalog: the
        // priority turn keeps the standard cache-creation axis.
        let priority = lookup_price_tiered("gpt-5.5", Some("priority"));
        let standard = lookup_price("gpt-5.5");
        assert!(
            (priority.input - standard.input * 2.5).abs() < 1e-9
                && (priority.output - standard.output * 2.5).abs() < 1e-9,
            "the published 2.5x priority row bills"
        );
        assert_eq!(
            priority.cw, standard.cw,
            "absent priority cw keeps standard"
        );
    }

    #[test]
    fn rm1556_fast_multiplier_table_bills_published_overrides_only() {
        // The ccusage #1855 override set mirrored verbatim: exact ids,
        // strictly-longer family prefixes for dated variants, and
        // everything else — including a vendor-qualified id that IS
        // its own catalog row — bills standard, disclosed, in
        // ccusage parity ("reports standard pricing rather than
        // assuming a multiplier for unknown models").
        let gpt55 = lookup_price("gpt-5.5");
        let fast = lookup_price_tiered("gpt-5.5", Some("fast"));
        assert_eq!(
            (fast.input, fast.output),
            (gpt55.input * 2.5, gpt55.output * 2.5)
        );
        let sol = lookup_price("gpt-5.6-sol");
        let fast = lookup_price_tiered("gpt-5.6-sol", Some("fast"));
        assert_eq!(
            (fast.input, fast.output),
            (sol.input * 2.0, sol.output * 2.0)
        );
        // Dated variants inherit by family prefix, and the cache
        // axes scale with the token axes.
        let dated = lookup_price("claude-opus-4-6-20260205");
        let fast = lookup_price_tiered("claude-opus-4-6-20260205", Some("fast"));
        assert_eq!(
            (fast.input, fast.output),
            (dated.input * 6.0, dated.output * 6.0)
        );
        // rm-1508 composition (wall, post-base): published cache
        // slots are Option; the dated claude row publishes both, so
        // unwrap() keeps the pin strict. A row without a published
        // cache rate would keep None — its fallback derives from the
        // already-scaled input rate (see fast_tier_price).
        assert!((fast.cw.unwrap() - dated.cw.unwrap() * 6.0).abs() < 1e-9);
        assert!((fast.cr.unwrap() - dated.cr.unwrap() * 6.0).abs() < 1e-9);
        // The bare family id is NOT strictly longer than its own
        // prefix and has no exact-table entry: standard, never guessed.
        let bare = lookup_price("claude-opus-4-6");
        let fast = lookup_price_tiered("claude-opus-4-6", Some("fast"));
        assert_eq!((fast.input, fast.output), (bare.input, bare.output));
        // Unknown models and own-row qualified ids bill standard.
        for model in ["deepseek-chat", "azure/gpt-5.6-luna"] {
            let standard = lookup_price(model);
            let fast = lookup_price_tiered(model, Some("fast"));
            assert_eq!(
                (fast.input, fast.output),
                (standard.input, standard.output),
                "{model}"
            );
        }
        // Table unit pins: normalization and the prefix bounds.
        assert_eq!(fast_multiplier_for("  GPT-5.5 "), Some(2.5));
        assert_eq!(fast_multiplier_for("claude-opus-4-7-anything"), Some(6.0));
        assert_eq!(fast_multiplier_for("claude-opus-4-8-20260101"), Some(2.0));
        assert_eq!(fast_multiplier_for(""), None);
        assert_eq!(
            fast_multiplier_for("gpt-5.6"),
            None,
            "bare gpt-5.6 has no fast row"
        );
    }

    #[test]
    fn override_negative_ultrafast_rate_is_rejected() {
        let raw = br#"{"prices": {"m": {"input": 1, "output": 2, "cw": 0, "cr": 0, "input_ultrafast": -0.5}}}"#;
        let err = parse_pricing_overrides(raw)
            .expect_err("a negative ultrafast override rate must be rejected");
        let err_text = err.to_string();
        assert!(
            err_text.contains("input_ultrafast") && err_text.contains("negative"),
            "the rejection must name the field: {err_text}"
        );
    }

    #[test]
    fn catalog_identity_covers_flex_and_providers() {
        fn catalog_with(input_flex: Option<f64>, provider: &str) -> PricingCatalog {
            PricingCatalog {
                entries: BTreeMap::from([(
                    "m".to_string(),
                    Price {
                        input: 1.0,
                        output: 2.0,
                        input_flex,
                        ..Price::default()
                    },
                )]),
                aliases: BTreeMap::new(),
                source: "test".to_string(),
                providers: BTreeMap::from([("m".to_string(), provider.to_string())]),
                reference_date: None,
            }
        }
        let base = catalog_identity_of(&catalog_with(None, "openai"));
        let gained_flex = catalog_identity_of(&catalog_with(Some(0.5), "openai"));
        let other_flex = catalog_identity_of(&catalog_with(Some(0.6), "openai"));
        let provider_flip = catalog_identity_of(&catalog_with(None, "azure"));
        assert_ne!(
            base, gained_flex,
            "gaining a flex slot must reprice the cache"
        );
        assert_ne!(
            gained_flex, other_flex,
            "a flex value change must reprice the cache"
        );
        assert_ne!(
            base, provider_flip,
            "a rate-identical provider flip changes attribution and must change the digest (rm-613)"
        );
    }

    #[test]
    fn catalog_identity_covers_ultrafast_slots() {
        // Cycle-2 rider: a gained or changed ultrafast rate changes
        // what an `service_tier: "ultrafast"` turn bills at, so it
        // must reprice cached sessions exactly like a flex change.
        fn catalog_with(input_ultrafast: Option<f64>) -> PricingCatalog {
            PricingCatalog {
                entries: BTreeMap::from([(
                    "m".to_string(),
                    Price {
                        input: 1.0,
                        output: 2.0,
                        input_ultrafast,
                        ..Price::default()
                    },
                )]),
                aliases: BTreeMap::new(),
                source: "test".to_string(),
                providers: BTreeMap::new(),
                reference_date: None,
            }
        }
        let base = catalog_identity_of(&catalog_with(None));
        let gained = catalog_identity_of(&catalog_with(Some(60.0)));
        let changed = catalog_identity_of(&catalog_with(Some(61.0)));
        assert_ne!(
            base, gained,
            "gaining ultrafast pricing must reprice the cache"
        );
        assert_ne!(
            gained, changed,
            "an ultrafast value change must reprice the cache"
        );
    }

    #[test]
    fn catalog_identity_covers_priority_slots() {
        // rm-1556 cycle-4 rider (independent review case
        // 19787e53f2b8): the run landed the priority tier family but
        // not its digest arm, so a snapshot refresh changing ONLY
        // priority rates would have kept the identity byte-stable and
        // served stale standard-priced priority turns from warm
        // caches — exactly the collision class rm-610 closed for flex
        // (and cycle-2 for ultrafast). A gained or changed priority
        // rate changes what a `service_tier: "priority"` turn bills
        // at, so it must reprice cached sessions like every tier
        // family.
        fn catalog_with(input_priority: Option<f64>) -> PricingCatalog {
            PricingCatalog {
                entries: BTreeMap::from([(
                    "m".to_string(),
                    Price {
                        input: 1.0,
                        output: 2.0,
                        input_priority,
                        ..Price::default()
                    },
                )]),
                aliases: BTreeMap::new(),
                source: "test".to_string(),
                providers: BTreeMap::new(),
                reference_date: None,
            }
        }
        let base = catalog_identity_of(&catalog_with(None));
        let gained = catalog_identity_of(&catalog_with(Some(60.0)));
        let changed = catalog_identity_of(&catalog_with(Some(61.0)));
        assert_ne!(
            base, gained,
            "gaining priority pricing must reprice the cache"
        );
        assert_ne!(
            gained, changed,
            "a priority value change must reprice the cache"
        );
    }
}
