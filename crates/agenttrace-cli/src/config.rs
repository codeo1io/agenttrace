//! Configuration file support (rm-384, cycle 1).
//!
//! `agenttrace` resolves a small set of knobs from a layered
//! configuration before any report code runs:
//!
//! 1. CLI flags (`--history-dir`, `--pricing-file`, `--weekly-budget`)
//! 2. an explicit `--config PATH` file
//! 3. a project file at `./.agenttrace/config.toml`
//! 4. a user file at `~/.config/agenttrace/config.toml`
//!    (`$XDG_CONFIG_HOME` respected)
//! 5. the `AGENTTRACE_*` environment knobs and built-in defaults
//!
//! Each layer only fills keys left unset by higher layers; the first
//! layer that sets a key wins. `--doctor` discloses every layer that
//! was found and the winning source of each knob.
//!
//! The file is parsed with a deliberate scalar subset of TOML syntax —
//! `key = value` lines with quoted or bare scalars and `#` comments.
//! Tables, arrays, and multi-line values are rejected loudly rather
//! than silently mis-parsed, and unknown keys are rejected too (a
//! typo'd `history_directry` must not silently do nothing). A full
//! TOML implementation would pull a new dependency for three flat
//! keys; the subset keeps the binary dependency-surface unchanged and
//! every deviation loud.

use anyhow::{bail, Context};

/// The knobs a configuration file may set. All optional: absent keys
/// leave lower layers in charge.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ConfigFile {
    /// Directory holding `history.json` (overrides
    /// `AGENTTRACE_HISTORY_DIR`).
    pub history_dir: Option<PathBuf>,
    /// Pricing override file (overrides `AGENTTRACE_PRICING_FILE`).
    pub pricing_file: Option<PathBuf>,
    /// Weekly spend budget in USD (consumed by `--statusline-report`
    /// and `--budget`).
    pub weekly_budget_usd: Option<f64>,
}

impl ConfigFile {
    /// Fold `upper` over `self`: keys set in `upper` replace keys set
    /// here, keys left `None` keep this layer's value.
    fn layered_with(mut self, upper: &ConfigFile) -> ConfigFile {
        if upper.history_dir.is_some() {
            self.history_dir = upper.history_dir.clone();
        }
        if upper.pricing_file.is_some() {
            self.pricing_file = upper.pricing_file.clone();
        }
        if upper.weekly_budget_usd.is_some() {
            self.weekly_budget_usd = upper.weekly_budget_usd;
        }
        self
    }
}

const SUPPORTED_KEYS: &str = "history_dir, pricing_file, weekly_budget_usd";

/// The user-level configuration path:
/// `$XDG_CONFIG_HOME/agenttrace/config.toml` or
/// `~/.config/agenttrace/config.toml`.
pub fn user_config_path() -> Option<PathBuf> {
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")))?;
    Some(base.join("agenttrace").join("config.toml"))
}

/// The project-level configuration path: `./.agenttrace/config.toml`
/// relative to the current working directory.
pub fn project_config_path() -> PathBuf {
    PathBuf::from(".agenttrace").join("config.toml")
}

/// Parse the scalar `key = value` subset. Errors carry the file path
/// and 1-based line number.
pub fn parse_config(text: &str, path: &Path) -> anyhow::Result<ConfigFile> {
    let shown = path.display();
    let mut file = ConfigFile::default();
    for (index, raw) in text.lines().enumerate() {
        let number = index + 1;
        let line = strip_comment(raw).trim();
        if line.is_empty() {
            continue;
        }
        if line.starts_with('[') {
            bail!(
                "{shown}:{number}: tables and arrays are not supported \
                 (agenttrace config is flat `key = value`; got `{line}`)"
            );
        }
        let Some((key, value)) = line.split_once('=') else {
            bail!(
                "{shown}:{number}: expected `key = value`, got `{line}` \
                 (supported keys: {SUPPORTED_KEYS})"
            );
        };
        let key = key.trim();
        let value = parse_scalar(value.trim())
            .with_context(|| format!("{shown}:{number}: malformed value for `{key}`"))?;
        match (key, value) {
            ("history_dir", Scalar::String(path)) => file.history_dir = Some(path),
            ("pricing_file", Scalar::String(path)) => file.pricing_file = Some(path),
            ("weekly_budget_usd", Scalar::Number(amount)) => {
                if !amount.is_finite() || amount <= 0.0 {
                    bail!(
                        "{shown}:{number}: weekly_budget_usd must be a positive \
                         finite USD amount, got `{amount}`"
                    );
                }
                file.weekly_budget_usd = Some(amount);
            }
            ("history_dir", Scalar::Number(amount))
            | ("pricing_file", Scalar::Number(amount)) => {
                bail!(
                    "{shown}:{number}: `{key}` expects a path string, got the \
                     number `{amount}`"
                );
            }
            ("weekly_budget_usd", Scalar::String(text)) => {
                bail!(
                    "{shown}:{number}: weekly_budget_usd expects a number, \
                     got the string `{}`",
                    text.display()
                );
            }
            _ => bail!(
                "{shown}:{number}: unknown config key `{key}` \
                 (supported keys: {SUPPORTED_KEYS})"
            ),
        }
    }
    Ok(file)
}

enum Scalar {
    String(PathBuf),
    Number(f64),
}

fn parse_scalar(text: &str) -> anyhow::Result<Scalar> {
    if let Some(rest) = text.strip_prefix('"') {
        let end = rest.find('"').ok_or_else(|| {
            anyhow::anyhow!("unterminated quoted string")
        })?;
        return Ok(Scalar::String(PathBuf::from(&rest[..end])));
    }
    if let Ok(amount) = text.parse::<f64>() {
        return Ok(Scalar::Number(amount));
    }
    if text.is_empty() {
        anyhow::bail!("empty value");
    }
    Ok(Scalar::String(PathBuf::from(text)))
}

/// Strip a trailing `# comment` that sits outside quotes.
fn strip_comment(line: &str) -> &str {
    let mut in_quotes = false;
    for (index, character) in line.char_indices() {
        match character {
            '"' => in_quotes = !in_quotes,
            '#' if !in_quotes => return &line[..index],
            _ => {}
        }
    }
    line
}

/// One discovered configuration layer, in ascending precedence.
#[derive(Debug, Clone)]
pub struct ConfigLayer {
    pub name: &'static str,
    pub path: PathBuf,
    pub file: ConfigFile,
}

/// The fully resolved configuration plus the evidence
/// [`disclosure_text`] renders.
#[derive(Debug, Clone)]
pub struct ResolvedConfig {
    /// Layers that existed and parsed, ascending precedence
    /// (user, project, explicit).
    pub layers: Vec<ConfigLayer>,
    /// Paths probed but absent.
    pub absent: Vec<(&'static str, PathBuf)>,
    /// Winning `--history-dir` (CLI flag or a config layer).
    pub history_dir: Option<PathBuf>,
    /// Winning `--pricing-file`.
    pub pricing_file: Option<PathBuf>,
    /// Winning weekly budget in USD.
    pub weekly_budget: Option<f64>,
    /// Where each winning knob came from (`--flag`, a layer name, or
    /// `env/default`).
    pub sources: [(&'static str, &'static str); 3],
}

/// Resolve the configuration for this invocation: discover the user
/// and project files, parse them plus an explicit `--config` file,
/// layer them (user < project < explicit), then apply CLI flags on
/// top. The env knobs sit below every layer and stay core's fallback.
pub fn resolve(args: &crate::Args) -> anyhow::Result<ResolvedConfig> {
    let user = user_config_path();
    let project = project_config_path();
    let mut layers = Vec::new();
    let mut absent = Vec::new();

    let load = |name: &'static str,
                    path: Option<&PathBuf>,
                    required: bool,
                    layers: &mut Vec<ConfigLayer>,
                    absent: &mut Vec<(&'static str, PathBuf)>|
     -> anyhow::Result<()> {
        let Some(path) = path else {
            return Ok(());
        };
        match fs::read_to_string(path) {
            Ok(text) => layers.push(ConfigLayer {
                name,
                path: path.clone(),
                file: parse_config(&text, path)?,
            }),
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                if required {
                    anyhow::bail!(
                        "--config file not found: {}",
                        path.display()
                    );
                }
                absent.push((name, path.clone()));
            }
            Err(error) => {
                return Err(anyhow::Error::new(error))
                    .context(format!("reading config file {}", path.display()));
            }
        }
        Ok(())
    };

    load("user", user.as_ref(), false, &mut layers, &mut absent)?;
    load("project", Some(&project), false, &mut layers, &mut absent)?;
    load(
        "--config",
        args.config.as_ref(),
        true,
        &mut layers,
        &mut absent,
    )?;

    let mut merged = ConfigFile::default();
    for layer in &layers {
        merged = merged.layered_with(&layer.file);
    }

    // CLI flags sit above every file layer.
    let history_dir = args.history_dir.clone().or_else(|| merged.history_dir.clone());
    let pricing_file = args.pricing_file.clone().or_else(|| merged.pricing_file.clone());
    let weekly_budget = args.weekly_budget.or(merged.weekly_budget_usd);

    let source_for =
        |flag: bool, key: fn(&ConfigFile) -> bool| -> &'static str {
            if flag {
                "CLI flag"
            } else if layers
                .iter()
                .rev()
                .any(|layer| key(&layer.file))
            {
                "config file"
            } else {
                "env/default"
            }
        };
    let sources = [
        (
            "history_dir",
            source_for(args.history_dir.is_some(), |f| f.history_dir.is_some()),
        ),
        (
            "pricing_file",
            source_for(args.pricing_file.is_some(), |f| f.pricing_file.is_some()),
        ),
        (
            "weekly_budget_usd",
            source_for(
                args.weekly_budget.is_some(),
                |f| f.weekly_budget_usd.is_some(),
            ),
        ),
    ];

    Ok(ResolvedConfig {
        layers,
        absent,
        history_dir,
        pricing_file,
        weekly_budget,
        sources,
    })
}

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// The `--doctor` disclosure as a standalone JSON document (emitted
/// on stderr so stdout stays one pure JSON object in JSON mode).
pub fn disclosure_json(config: &ResolvedConfig) -> String {
    let knob = |key: &str, value: serde_json::Value, source: &str| {
        serde_json::json!({ "key": key, "value": value, "source": source })
    };
    let doc = serde_json::json!({
        "layers": config.layers.iter().map(|layer| serde_json::json!({
            "name": layer.name,
            "path": layer.path.display().to_string(),
        })).collect::<Vec<_>>(),
        "absent": config.absent.iter().map(|(name, path)| serde_json::json!({
            "name": name,
            "path": path.display().to_string(),
        })).collect::<Vec<_>>(),
        "knobs": [
            knob(
                "history_dir",
                config.history_dir.as_ref().map(|path| serde_json::Value::String(path.display().to_string())).unwrap_or(serde_json::Value::Null),
                config.sources[0].1,
            ),
            knob(
                "pricing_file",
                config.pricing_file.as_ref().map(|path| serde_json::Value::String(path.display().to_string())).unwrap_or(serde_json::Value::Null),
                config.sources[1].1,
            ),
            knob(
                "weekly_budget_usd",
                config.weekly_budget.map(serde_json::Value::from).unwrap_or(serde_json::Value::Null),
                config.sources[2].1,
            ),
        ],
    });
    serde_json::to_string_pretty(&doc).unwrap_or_default()
}

/// The `--doctor` disclosure block (text modes). One line per probed
/// layer, one line per knob with its winning source.
pub fn disclosure_text(config: &ResolvedConfig) -> String {
    let mut out = String::from("Configuration:\n");
    for layer in &config.layers {
        out.push_str(&format!(
            "  {} config: {} (loaded)\n",
            layer.name,
            layer.path.display()
        ));
    }
    for (name, path) in &config.absent {
        out.push_str(&format!("  {} config: {} (not found)\n", name, path.display()));
    }
    if config.layers.is_empty() && config.absent.is_empty() {
        out.push_str("  no config files discovered\n");
    }
    let knobs = [
        (
            config.sources[0].0,
            config
                .history_dir
                .as_ref()
                .map(|path| path.display().to_string())
                .unwrap_or_else(|| "(unset)".to_string()),
            config.sources[0].1,
        ),
        (
            config.sources[1].0,
            config
                .pricing_file
                .as_ref()
                .map(|path| path.display().to_string())
                .unwrap_or_else(|| "(unset)".to_string()),
            config.sources[1].1,
        ),
        (
            config.sources[2].0,
            config
                .weekly_budget
                .map(|amount| format!("${amount:.2}"))
                .unwrap_or_else(|| "(unset)".to_string()),
            config.sources[2].1,
        ),
    ];
    for (key, value, source) in knobs {
        out.push_str(&format!("  {key}: {value} ({source})\n"));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_config(dir: &std::path::Path, text: &str) -> PathBuf {
        let path = dir.join("config.toml");
        fs::write(&path, text).unwrap();
        path
    }

    #[test]
    fn parses_supported_keys_comments_and_quotes() {
        let dir = std::env::temp_dir().join(format!("at-config-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let path = write_config(
            &dir,
            "# agenttrace config\nhistory_dir = \"/tmp/at-history\"  # comment\npricing_file = /tmp/at-pricing.json\nweekly_budget_usd = 12.5\n",
        );
        let file = parse_config(&fs::read_to_string(&path).unwrap(), &path).unwrap();
        assert_eq!(file.history_dir, Some(PathBuf::from("/tmp/at-history")));
        assert_eq!(file.pricing_file, Some(PathBuf::from("/tmp/at-pricing.json")));
        assert_eq!(file.weekly_budget_usd, Some(12.5));
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn unknown_keys_and_tables_fail_loudly_with_line_numbers() {
        let path = Path::new("config.toml");
        let error = parse_config("history_directry = /tmp\n", path)
            .unwrap_err()
            .to_string();
        assert!(error.contains("config.toml:1"), "{error}");
        assert!(error.contains("unknown config key `history_directry`"), "{error}");
        let error = parse_config("[tool]\n", path).unwrap_err().to_string();
        assert!(error.contains("tables and arrays"), "{error}");
        let error = parse_config("weekly_budget_usd = -3\n", path).unwrap_err().to_string();
        assert!(error.contains("positive finite USD"), "{error}");
        let error = parse_config("history_dir = 5\n", path).unwrap_err().to_string();
        assert!(error.contains("expects a path string"), "{error}");
    }

    #[test]
    fn layering_lets_higher_layers_override_only_set_keys() {
        let user = ConfigFile {
            history_dir: Some(PathBuf::from("/user-history")),
            pricing_file: Some(PathBuf::from("/user-pricing.json")),
            weekly_budget_usd: Some(10.0),
        };
        let project = ConfigFile {
            history_dir: Some(PathBuf::from("/project-history")),
            ..ConfigFile::default()
        };
        let merged = user.layered_with(&project);
        assert_eq!(merged.history_dir, Some(PathBuf::from("/project-history")));
        assert_eq!(merged.pricing_file, Some(PathBuf::from("/user-pricing.json")));
        assert_eq!(merged.weekly_budget_usd, Some(10.0));
    }
}
