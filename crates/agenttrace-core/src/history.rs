use crate::{project_name, Anomaly, Diagnostics, Metrics, Session};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::hash::{Hash, Hasher};
use std::path::Path;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DerivedSession {
    id: String,
    project: String,
    source: String,
    model: String,
    start: String,
    end: String,
    duration: f64,
    input: i64,
    output: i64,
    cache_write: i64,
    cache_read: i64,
    cost: f64,
    health: i32,
    anomalies: Vec<(String, String)>,
}

pub fn history_path() -> PathBuf {
    if let Some(dir) = std::env::var_os("AGENTTRACE_HISTORY_DIR").map(PathBuf::from) {
        return dir.join("history.json");
    }
    let base = std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".local/share")))
        .unwrap_or_else(std::env::temp_dir);
    base.join("agenttrace").join("history.json")
}

pub fn preserve_derived_history(sessions: &[Session]) -> anyhow::Result<()> {
    let mut records = load_records();
    for session in sessions {
        let record = DerivedSession::from_session(session);
        records.insert(record.id.clone(), record);
    }
    let path = history_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    // Stage through a unique temp sibling, then rename into place so a
    // crash mid-write can no longer tear the only durable record of
    // derived sessions (pass-7 P7-5).
    let tmp = crate::session_cache::unique_temp_path(&path);
    std::fs::write(&tmp, serde_json::to_vec_pretty(&records)?)?;
    std::fs::rename(&tmp, &path)?;
    Ok(())
}

pub fn merge_preserved_history(live: &mut Vec<Session>) {
    let mut seen = live
        .iter()
        .map(session_id)
        .collect::<std::collections::BTreeSet<_>>();
    // Records preserved before rm-055 are keyed by the raw path
    // spelling; suppress those ids too so switching to canonical ids
    // does not resurrect old records next to their still-live sessions.
    for session in live.iter() {
        seen.insert(legacy_session_id(session));
    }
    for record in load_records().into_values() {
        if seen.insert(record.id.clone()) {
            live.push(record.into_session());
        }
    }
}

fn load_records() -> BTreeMap<String, DerivedSession> {
    let path = history_path();
    let Ok(raw) = std::fs::read(&path) else {
        return BTreeMap::new();
    };
    records_from_bytes(&path, &raw)
}

/// Decode history bytes; a torn or corrupt file is quarantined (renamed
/// to `<name>.json.corrupt`, kept for inspection) instead of silently
/// zeroing the only durable record of derived sessions (pass-7 P7-5).
fn records_from_bytes(path: &Path, raw: &[u8]) -> BTreeMap<String, DerivedSession> {
    match serde_json::from_slice::<BTreeMap<String, DerivedSession>>(raw) {
        Ok(_) => decode_records(raw),
        Err(_) => {
            let quarantine = path.with_extension("json.corrupt");
            let _ = std::fs::rename(path, &quarantine);
            eprintln!(
                "agenttrace: history file {} was unreadable; quarantined as {} (new history starts empty)",
                path.display(),
                quarantine.display()
            );
            BTreeMap::new()
        }
    }
}

fn decode_records(raw: &[u8]) -> BTreeMap<String, DerivedSession> {
    serde_json::from_slice::<BTreeMap<String, DerivedSession>>(raw)
        .unwrap_or_default()
        .into_iter()
        .filter(|(_, record)| record.id.chars().count() >= 8)
        .collect()
}

fn session_id(session: &Session) -> String {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    // rm-055: hash the CANONICAL path. Aliased roots (`~/.pi ->
    // ~/.config/pi`) reach one file through several spellings, and the
    // raw spelling forked derived-history identity per alias.
    canonical_identity_path(&session.path).hash(&mut hasher);
    session.metrics.session_start.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

/// Canonical form of a session path for identity purposes (rm-055):
/// alias spellings of one file collapse to one id. `history:`
/// pseudo-paths and files that cannot be resolved hash as-is.
fn canonical_identity_path(path: &str) -> String {
    if path.starts_with("history:") {
        return path.to_string();
    }
    std::fs::canonicalize(path)
        .map(|canonical| canonical.to_string_lossy().into_owned())
        .unwrap_or_else(|_| path.to_string())
}

/// Identity as computed before rm-055 (raw path spelling). Used only to
/// suppress records keyed the old way, so canonicalization cannot fork
/// a still-live session into a live session plus a phantom history
/// entry for every existing user of derived history.
fn legacy_session_id(session: &Session) -> String {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    session.path.hash(&mut hasher);
    session.metrics.session_start.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

impl DerivedSession {
    fn from_session(session: &Session) -> Self {
        Self {
            id: session_id(session),
            project: project_name(session),
            source: session.metrics.source_tool.clone(),
            model: session.metrics.model_used.clone(),
            start: session.metrics.session_start.clone(),
            end: session.metrics.session_end.clone(),
            duration: session.metrics.duration_sec,
            input: session.metrics.tokens_input,
            output: session.metrics.tokens_output,
            cache_write: session.metrics.tokens_cache_w,
            cache_read: session.metrics.tokens_cache_r,
            cost: session.metrics.cost_estimated,
            health: session.health,
            anomalies: session
                .anomalies
                .iter()
                .map(|item| (item.kind.clone(), item.severity.clone()))
                .collect(),
        }
    }

    fn into_session(self) -> Session {
        let short_id = self.id.chars().take(8).collect::<String>();
        Session {
            name: format!("history-{short_id}"),
            path: format!("history:{}", self.id),
            cwd: self.project,
            metrics: Metrics {
                source_tool: self.source,
                model_used: self.model,
                session_start: self.start,
                session_end: self.end,
                duration_sec: self.duration,
                tokens_input: self.input,
                tokens_output: self.output,
                tokens_cache_w: self.cache_write,
                tokens_cache_r: self.cache_read,
                cost_estimated: self.cost,
                ..Metrics::default()
            },
            anomalies: self
                .anomalies
                .into_iter()
                .map(|(kind, severity)| Anomaly {
                    kind,
                    severity,
                    detail: "preserved derived history".to_string(),
                })
                .collect(),
            health: self.health,
            tool_warnings: Vec::new(),
            diagnostics: Diagnostics::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preserved_history_contains_only_derived_fields() {
        let session = Session {
            name: "secret task".to_string(),
            path: "/tmp/private/session.jsonl".to_string(),
            cwd: "/work/project".to_string(),
            metrics: Metrics {
                source_tool: "codex_cli".to_string(),
                model_used: "gpt-5".to_string(),
                session_start: "2026-07-19T00:00:00Z".to_string(),
                tokens_input: 10,
                cost_estimated: 0.01,
                ..Metrics::default()
            },
            anomalies: Vec::new(),
            health: 95,
            tool_warnings: Vec::new(),
            diagnostics: Diagnostics::default(),
        };
        let json = serde_json::to_string(&DerivedSession::from_session(&session)).unwrap();
        assert!(!json.contains("secret task"));
        assert!(!json.contains("/tmp/private/session.jsonl"));
        assert!(json.contains("project"));
        assert!(json.contains("gpt-5"));

        let mut valid = DerivedSession::from_session(&session);
        let mut short = valid.clone();
        short.id = "x".to_string();
        valid.id = "12345678".to_string();
        let raw = serde_json::to_vec(&BTreeMap::from([
            ("valid".to_string(), valid),
            ("short".to_string(), short.clone()),
        ]))
        .unwrap();
        assert_eq!(decode_records(&raw).len(), 1);
        assert_eq!(short.into_session().name, "history-x");
    }

    #[test]
    fn torn_history_file_is_quarantined_not_silently_discarded() {
        // Pass-7 P7-5: a half-written history file (interrupted raw write)
        // used to decode to an empty map and silently wipe the durable
        // record. It must move aside, visibly, with the bytes preserved.
        let root = std::env::temp_dir().join(format!(
            "agenttrace-history-corrupt-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        std::fs::create_dir_all(&root).expect("create temp dir");
        let path = root.join("history.json");
        std::fs::write(&path, b"{\"torn\": ").expect("write torn history");
        let records = records_from_bytes(&path, b"{\"torn\": ");
        assert!(records.is_empty(), "torn history yields no records");
        let quarantine = root.join("history.json.corrupt");
        assert!(
            quarantine.exists(),
            "torn history must be quarantined beside the cache"
        );
        assert!(!path.exists(), "torn history must not be re-read in place");
        let preserved = std::fs::read(&quarantine).expect("quarantine preserves bytes");
        assert_eq!(preserved, b"{\"torn\": ");
        let _ = std::fs::remove_dir_all(root);
    }

    // -------------------------------------------------------------------
    // rm-055 (run 83642957d130 cycle 1): history identity across alias
    // spellings of one session file.
    // -------------------------------------------------------------------

    fn alias_fixture() -> (std::path::PathBuf, Session) {
        let root = std::env::temp_dir().join(format!(
            "agenttrace-history-alias-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let real = root.join(".config/pi/agent/sessions");
        std::fs::create_dir_all(&real).expect("create sessions dir");
        let file = real.join("aaa11111-2222-3333-4444-555566667777.jsonl");
        std::fs::write(&file, "{}\n").expect("write session file");
        std::os::unix::fs::symlink(root.join(".config/pi"), root.join(".pi"))
            .expect("symlink alias root");
        let session = Session {
            name: "alias root session".to_string(),
            path: file.to_string_lossy().into_owned(),
            cwd: "/work/project".to_string(),
            metrics: Metrics {
                source_tool: "pi".to_string(),
                model_used: "qwen3-coder".to_string(),
                session_start: "2026-10-01T00:00:00Z".to_string(),
                cost_estimated: 0.01,
                ..Metrics::default()
            },
            anomalies: Vec::new(),
            health: 95,
            tool_warnings: Vec::new(),
            diagnostics: Diagnostics::default(),
        };
        (root, session)
    }

    #[cfg(unix)]
    #[test]
    fn history_identity_is_stable_across_alias_spellings() {
        // `~/.pi -> ~/.config/pi` presents one file through two
        // spellings; the id fed the derived-history key used to fork per
        // spelling (rm-055, assess F1).
        let (root, via_real) = alias_fixture();
        let mut via_alias = via_real.clone();
        via_alias.path = root
            .join(".pi/agent/sessions/aaa11111-2222-3333-4444-555566667777.jsonl")
            .to_string_lossy()
            .into_owned();
        assert_eq!(
            session_id(&via_real),
            session_id(&via_alias),
            "canonical identity: one file, one history id"
        );
        // Both spellings preserve to ONE record.
        let _env = crate::test_env::lock_env();
        let prior = std::env::var_os("AGENTTRACE_HISTORY_DIR");
        std::env::set_var("AGENTTRACE_HISTORY_DIR", &root);
        preserve_derived_history(&[via_real.clone()]).expect("preserve real spelling");
        preserve_derived_history(&[via_alias.clone()]).expect("preserve alias spelling");
        let records = load_records();
        assert_eq!(
            records.len(),
            1,
            "two spellings, one record (ids keyed canonically)"
        );
        match prior {
            Some(value) => std::env::set_var("AGENTTRACE_HISTORY_DIR", value),
            None => std::env::remove_var("AGENTTRACE_HISTORY_DIR"),
        }
        drop(_env);
        let _ = std::fs::remove_dir_all(root);
    }

    #[cfg(unix)]
    #[test]
    fn merge_suppresses_records_keyed_by_the_legacy_spelling() {
        // Rollout guard for the rm-055 id change: records preserved by
        // older binaries are keyed by the RAW path spelling. Without
        // suppression, every still-live previously-preserved session
        // would come back as a live session PLUS a phantom history twin.
        let (root, mut live) = alias_fixture();
        live.path = root
            .join(".pi/agent/sessions/aaa11111-2222-3333-4444-555566667777.jsonl")
            .to_string_lossy()
            .into_owned();
        let legacy = DerivedSession {
            id: legacy_session_id(&live),
            project: "old".to_string(),
            source: "pi".to_string(),
            model: "qwen3-coder".to_string(),
            start: live.metrics.session_start.clone(),
            end: String::new(),
            duration: 0.0,
            input: 0,
            output: 0,
            cache_write: 0,
            cache_read: 0,
            cost: 0.0,
            health: 100,
            anomalies: Vec::new(),
        };
        let _env = crate::test_env::lock_env();
        let prior = std::env::var_os("AGENTTRACE_HISTORY_DIR");
        std::env::set_var("AGENTTRACE_HISTORY_DIR", &root);
        std::fs::write(
            &root.join("history.json"),
            serde_json::to_vec(&BTreeMap::from([(legacy.id.clone(), legacy)]))
                .expect("serialize legacy records"),
        )
        .expect("seed a legacy record");

        let mut sessions = vec![live.clone()];
        merge_preserved_history(&mut sessions);
        assert_eq!(
            sessions.len(),
            1,
            "the legacy record for a still-live session must not re-enter as history"
        );
        // A record for a GONE session (no live twin) still merges in.
        let gone = DerivedSession {
            id: "deadbeefcafe0123".to_string(),
            project: "gone".to_string(),
            source: "pi".to_string(),
            model: String::new(),
            start: "2026-01-01T00:00:00Z".to_string(),
            end: String::new(),
            duration: 0.0,
            input: 0,
            output: 0,
            cache_write: 0,
            cache_read: 0,
            cost: 0.0,
            health: 100,
            anomalies: Vec::new(),
        };
        let mut map = load_records();
        map.insert(gone.id.clone(), gone.clone());
        std::fs::write(
            &root.join("history.json"),
            serde_json::to_vec(&map).expect("serialize records"),
        )
            .expect("seed legacy + gone records");
        let mut sessions = vec![live.clone()];
        merge_preserved_history(&mut sessions);
        assert_eq!(
            sessions.len(),
            2,
            "gone-session history survives the canonical id change"
        );
        match prior {
            Some(value) => std::env::set_var("AGENTTRACE_HISTORY_DIR", value),
            None => std::env::remove_var("AGENTTRACE_HISTORY_DIR"),
        }
        drop(_env);
        let _ = std::fs::remove_dir_all(root);
    }
}
