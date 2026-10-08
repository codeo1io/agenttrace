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
//! typo'd `history_directry` must not silently do nothing). Review
//! 5b9a9470 F6 tightened three more deviations to loud rejections:
//! trailing input after a quoted value, values opening with a single
//! quote (a valid TOML literal string, but outside this subset —
//! rejecting beats quote-littering a path), and duplicate keys in one
//! file (TOML rejects duplicates; silent last-wins would discard the
//! earlier value without a trace). A full TOML implementation would
//! pull a new dependency for three flat keys; the subset keeps the
//! binary dependency-surface unchanged and every deviation loud.

use std::collections::HashSet;

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
    // rm-775 (run aa41d9b5 cycle 2): Windows editors write a leading
    // U+FEFF byte-order mark. It is not whitespace, so `trim()` leaves
    // it and every key in the file parsed as `\u{feff}key` — the
    // unknown-key error then contradicted itself by listing the very
    // key it rejected among the supported ones. Exactly ONE BOM is
    // stripped at the door: a second BOM is data, not encoding, and
    // still errors.
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let shown = path.display();
    let mut file = ConfigFile::default();
    let mut seen: HashSet<String> = HashSet::new();
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
        // Review 5b9a9470 F6: duplicate keys used to last-win silently;
        // TOML rejects duplicates and so does the subset. Only known
        // keys reach this check — unknown keys already bail in the
        // match below.
        if matches!(key, "history_dir" | "pricing_file" | "weekly_budget_usd")
            && !seen.insert(key.to_string())
        {
            bail!(
                "{shown}:{number}: duplicate key `{key}` — set each key once per file \
                 (TOML rejects duplicate keys; later values would silently discard earlier ones)"
            );
        }
        let value = match parse_scalar(value.trim()) {
            Ok(scalar) => scalar,
            // `{error:#}` carries the scalar's own message (e.g. the
            // trailing-garbage or single-quote cause) into the line-
            // stamped context so the CLI's Display output names the
            // deviation, not just "malformed value".
            Err(error) => bail!("{shown}:{number}: malformed value for `{key}`: {error:#}"),
        };
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
            ("history_dir", Scalar::Number(amount)) | ("pricing_file", Scalar::Number(amount)) => {
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
        let end = rest
            .find('"')
            .ok_or_else(|| anyhow::anyhow!("unterminated quoted string"))?;
        let trailing = rest[end + 1..].trim();
        if !trailing.is_empty() {
            anyhow::bail!("trailing input after the closing quote: `{trailing}`");
        }
        return Ok(Scalar::String(PathBuf::from(&rest[..end])));
    }
    if text.starts_with('\'') {
        anyhow::bail!(
            "single-quoted strings are not supported — use double quotes, e.g. `key = \"value\"`"
        );
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
    /// Env-knob values for display only: when no layer set the knob,
    /// these remain the effective level-5 values core falls back to.
    pub env_history_dir: Option<PathBuf>,
    /// `AGENTTRACE_PRICING_FILE` value, same display-only role.
    pub env_pricing_file: Option<PathBuf>,
    /// Where each winning knob came from (`CLI flag`, `config file`,
    /// `env`, or `default`).
    pub sources: [(&'static str, &'static str); 3],
}

/// Resolve the configuration for this invocation: discover the user
/// and project files, parse them plus an explicit `--config` file,
/// layer them (user < project < explicit), then apply CLI flags on
/// top. The env knobs sit below every layer and stay core's fallback.
pub fn resolve(args: &crate::Args) -> anyhow::Result<ResolvedConfig> {
    resolve_with_paths(args, user_config_path().as_deref(), &project_config_path())
}

/// The layer-resolution core over explicit paths, so the precedence
/// matrix is testable without mutating the process environment or
/// working directory (rm-384's test matrix; the fifth level — the
/// `AGENTTRACE_*` env knobs — sits below every file layer and stays
/// core's runtime fallback).
pub fn resolve_with_paths(
    args: &crate::Args,
    user: Option<&Path>,
    project: &Path,
) -> anyhow::Result<ResolvedConfig> {
    let mut layers = Vec::new();
    let mut absent = Vec::new();

    let load = |name: &'static str,
                path: Option<&Path>,
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
                path: path.to_path_buf(),
                file: parse_config(&text, path)?,
            }),
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                if required {
                    anyhow::bail!("--config file not found: {}", path.display());
                }
                absent.push((name, path.to_path_buf()));
            }
            Err(error) => {
                return Err(anyhow::Error::new(error))
                    .context(format!("reading config file {}", path.display()));
            }
        }
        Ok(())
    };

    load("user", user, false, &mut layers, &mut absent)?;
    load("project", Some(project), false, &mut layers, &mut absent)?;
    load(
        "--config",
        args.config.as_deref(),
        true,
        &mut layers,
        &mut absent,
    )?;

    let mut merged = ConfigFile::default();
    for layer in &layers {
        merged = merged.layered_with(&layer.file);
    }

    // CLI flags sit above every file layer.
    let history_dir = args
        .history_dir
        .clone()
        .or_else(|| merged.history_dir.clone());
    let pricing_file = args
        .pricing_file
        .clone()
        .or_else(|| merged.pricing_file.clone());
    let weekly_budget = args.weekly_budget.or(merged.weekly_budget_usd);

    let source_for = |flag: bool, file: bool, env: bool| -> &'static str {
        if flag {
            "CLI flag"
        } else if file {
            "config file"
        } else if env {
            "env"
        } else {
            "default"
        }
    };
    // The env knobs sit below every file layer: they are not part of
    // the merge (core falls back to them only when no layer set the
    // knob), but they are captured here so --doctor discloses the
    // effective level-5 value instead of a misleading "(unset)".
    let env_history_dir = std::env::var_os("AGENTTRACE_HISTORY_DIR").map(PathBuf::from);
    let env_pricing_file = std::env::var_os("AGENTTRACE_PRICING_FILE").map(PathBuf::from);
    let sources = [
        (
            "history_dir",
            source_for(
                args.history_dir.is_some(),
                layers.iter().any(|layer| layer.file.history_dir.is_some()),
                env_history_dir.is_some(),
            ),
        ),
        (
            "pricing_file",
            source_for(
                args.pricing_file.is_some(),
                layers.iter().any(|layer| layer.file.pricing_file.is_some()),
                env_pricing_file.is_some(),
            ),
        ),
        (
            "weekly_budget_usd",
            source_for(
                args.weekly_budget.is_some(),
                layers
                    .iter()
                    .any(|layer| layer.file.weekly_budget_usd.is_some()),
                false,
            ),
        ),
    ];

    Ok(ResolvedConfig {
        layers,
        absent,
        history_dir,
        pricing_file,
        weekly_budget,
        env_history_dir,
        env_pricing_file,
        sources,
    })
}

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// The `--doctor` disclosure as a standalone JSON document (emitted
/// on stderr so stdout stays one pure JSON object in JSON mode).
pub fn disclosure_json(config: &ResolvedConfig) -> String {
    let knob = |key: &str, value: serde_json::Value, source: &str| serde_json::json!({ "key": key, "value": value, "source": source });
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
                config
                    .history_dir
                    .as_ref()
                    .or(config.env_history_dir.as_ref())
                    .map(|path| serde_json::Value::String(path.display().to_string()))
                    .unwrap_or(serde_json::Value::Null),
                config.sources[0].1,
            ),
            knob(
                "pricing_file",
                config
                    .pricing_file
                    .as_ref()
                    .or(config.env_pricing_file.as_ref())
                    .map(|path| serde_json::Value::String(path.display().to_string()))
                    .unwrap_or(serde_json::Value::Null),
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
        out.push_str(&format!(
            "  {} config: {} (not found)\n",
            name,
            path.display()
        ));
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
                .or(config.env_history_dir.as_ref())
                .map(|path| path.display().to_string())
                .unwrap_or_else(|| "(unset)".to_string()),
            config.sources[0].1,
        ),
        (
            config.sources[1].0,
            config
                .pricing_file
                .as_ref()
                .or(config.env_pricing_file.as_ref())
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
    fn leading_bom_strips_exactly_once() {
        // rm-775 (run aa41d9b5 cycle 2): Windows editors write a leading
        // U+FEFF. char::trim does not strip it (it is not whitespace), so
        // every key in a BOM'd file parsed as `\u{feff}weekly_budget_usd`
        // and the unknown-key error contradicted itself — rejecting the
        // key while listing it among the supported ones. Exactly ONE BOM
        // is stripped at the door: a second BOM is data, not encoding,
        // and still errors.
        let dir = std::env::temp_dir().join(format!("at-config-bom-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let path = write_config(&dir, "\u{feff}weekly_budget_usd = 50.0\n");
        let file = parse_config(&fs::read_to_string(&path).unwrap(), &path).unwrap();
        assert_eq!(file.weekly_budget_usd, Some(50.0));

        let path = write_config(&dir, "\u{feff}\u{feff}weekly_budget_usd = 50.0\n");
        let error = parse_config(&fs::read_to_string(&path).unwrap(), &path).unwrap_err();
        assert!(error.to_string().contains("unknown config key"));

        let path = write_config(&dir, "not_a_real_key = 1\n");
        let error = parse_config(&fs::read_to_string(&path).unwrap(), &path).unwrap_err();
        let text = error.to_string();
        assert!(text.contains("not_a_real_key"));
        assert!(!text.contains('\u{feff}'), "clean file, clean key name");
        fs::remove_dir_all(&dir).ok();
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
        assert_eq!(
            file.pricing_file,
            Some(PathBuf::from("/tmp/at-pricing.json"))
        );
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
        assert!(
            error.contains("unknown config key `history_directry`"),
            "{error}"
        );
        let error = parse_config("[tool]\n", path).unwrap_err().to_string();
        assert!(error.contains("tables and arrays"), "{error}");
        let error = parse_config("weekly_budget_usd = -3\n", path)
            .unwrap_err()
            .to_string();
        assert!(error.contains("positive finite USD"), "{error}");
        let error = parse_config("history_dir = 5\n", path)
            .unwrap_err()
            .to_string();
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
        assert_eq!(
            merged.pricing_file,
            Some(PathBuf::from("/user-pricing.json"))
        );
        assert_eq!(merged.weekly_budget_usd, Some(10.0));
    }

    #[test]
    fn resolve_precedence_matrix_covers_all_file_layers_and_flags() {
        // rm-384 acceptance: a test matrix across the precedence chain
        // CLI flag > --config file > project > user. The fifth level
        // (AGENTTRACE_* env knobs) sits below every file layer and
        // stays core's runtime fallback — it is disclosed by --doctor
        // and demonstrated live, not asserted here (reading the real
        // environment in a unit test would race the test harness).
        let root = std::env::temp_dir().join(format!("at-config-matrix-{}", std::process::id()));
        fs::remove_dir_all(&root).ok();
        let user_dir = root.join("user/agenttrace");
        let project_dir = root.join("project/.agenttrace");
        fs::create_dir_all(&user_dir).unwrap();
        fs::create_dir_all(&project_dir).unwrap();
        fs::write(
            user_dir.join("config.toml"),
            "history_dir = \"/user-h\"\npricing_file = \"/user-p\"\nweekly_budget_usd = 10\n",
        )
        .unwrap();
        fs::write(
            project_dir.join("config.toml"),
            "history_dir = \"/project-h\"\nweekly_budget_usd = 20\n",
        )
        .unwrap();
        let user = user_dir.join("config.toml");
        let project = project_dir.join("config.toml");
        let absent_project = root.join("absent-project");

        // All four file/flag levels stacked: the CLI flag wins
        // history_dir, --config wins the budget, and the pricing_file
        // no higher layer sets falls through to the user file.
        let mut args = crate::test_args(None);
        args.config = Some(root.join("explicit.toml"));
        fs::write(root.join("explicit.toml"), "weekly_budget_usd = 30\n").unwrap();
        args.history_dir = Some(PathBuf::from("/cli-h"));
        let resolved = resolve_with_paths(&args, Some(&user), &project).unwrap();
        assert_eq!(resolved.history_dir, Some(PathBuf::from("/cli-h")));
        assert_eq!(resolved.weekly_budget, Some(30.0));
        assert_eq!(resolved.pricing_file, Some(PathBuf::from("/user-p")));
        assert_eq!(resolved.sources[0].1, "CLI flag");
        assert_eq!(resolved.sources[2].1, "config file");

        // Drop the CLI flag: the project layer wins history_dir over
        // the user file.
        args.history_dir = None;
        let resolved = resolve_with_paths(&args, Some(&user), &project).unwrap();
        assert_eq!(resolved.history_dir, Some(PathBuf::from("/project-h")));

        // Drop --config: the project budget wins over the user's.
        args.config = None;
        let resolved = resolve_with_paths(&args, Some(&user), &project).unwrap();
        assert_eq!(resolved.weekly_budget, Some(20.0));

        // Drop the project layer: the user file is the only one left.
        let resolved = resolve_with_paths(&args, Some(&user), &absent_project).unwrap();
        assert_eq!(resolved.history_dir, Some(PathBuf::from("/user-h")));
        assert_eq!(resolved.weekly_budget, Some(10.0));
        assert_eq!(resolved.pricing_file, Some(PathBuf::from("/user-p")));

        // No file sets anything: nothing is installed into the runtime
        // override table, leaving core's env/default fallback in
        // charge, and the probed paths are disclosed as not found.
        let resolved = resolve_with_paths(&args, None, &absent_project).unwrap();
        assert_eq!(resolved.history_dir, None);
        assert_eq!(resolved.weekly_budget, None);
        assert!(resolved.layers.is_empty());
        assert_eq!(resolved.absent.len(), 1);
        assert_eq!(resolved.absent[0].0, "project");

        // An explicit --config file must exist and parse; both failure
        // modes are loud.
        args.config = Some(root.join("missing.toml"));
        let error = resolve_with_paths(&args, None, &absent_project)
            .unwrap_err()
            .to_string();
        assert!(error.contains("--config file not found"), "{error}");
        fs::write(root.join("bad.toml"), "history_directry = /typo\n").unwrap();
        args.config = Some(root.join("bad.toml"));
        let error = resolve_with_paths(&args, None, &absent_project)
            .unwrap_err()
            .to_string();
        assert!(error.contains("bad.toml:1"), "{error}");
        assert!(
            error.contains("unknown config key `history_directry`"),
            "{error}"
        );

        fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn parser_rejects_invalid_forms_loudly() {
        // Review 5b9a9470 F6: three inputs that are invalid TOML (or a
        // valid-TOML form outside this subset) used to be accepted
        // silently — trailing garbage after a quoted value was
        // dropped, single-quoted strings fell into the bare-scalar arm
        // and quote-littered the path, and duplicate keys last-won.
        // All three must fail with the file, the line, and a named
        // cause.
        let path = Path::new("config.toml");

        // (a) trailing input after the closing quote
        let error = format!(
            "{:#}",
            parse_config("history_dir = \"/x\" junk\n", path).unwrap_err()
        );
        assert!(error.contains("config.toml:1"), "{error}");
        assert!(
            error.contains("trailing input after the closing quote: `junk`"),
            "{error}"
        );

        // (b) single-quoted literal string (valid TOML, outside the subset)
        let error = format!(
            "{:#}",
            parse_config("history_dir = '/tmp/h'\n", path).unwrap_err()
        );
        assert!(error.contains("config.toml:1"), "{error}");
        assert!(
            error.contains("single-quoted strings are not supported"),
            "{error}"
        );

        // (c) duplicate keys in one file
        let error = parse_config("history_dir = \"/a\"\nhistory_dir = \"/b\"\n", path)
            .unwrap_err()
            .to_string();
        assert!(error.contains("config.toml:2"), "{error}");
        assert!(error.contains("duplicate key `history_dir`"), "{error}");

        // The accepted forms keep parsing: quoted, bare, numbers, and
        // comments after values.
        let file = parse_config(
            "history_dir = \"/h\" # explicit comment\npricing_file = /p.json\nweekly_budget_usd = 12.5\n",
            path,
        )
        .unwrap();
        assert_eq!(file.history_dir, Some(PathBuf::from("/h")));
        assert_eq!(file.pricing_file, Some(PathBuf::from("/p.json")));
        assert_eq!(file.weekly_budget_usd, Some(12.5));
    }
}
