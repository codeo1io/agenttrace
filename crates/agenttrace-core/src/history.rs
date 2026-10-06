use crate::{project_name, Anomaly, Diagnostics, Metrics, Session};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
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
        // Review fix (2026-10-06): fold away DefaultHasher-era rows describing
        // this same session, so the file converges on one key per session
        // instead of accumulating the legacy key beside the pinned one.
        // Review fix (2026-10-07, round 2 LF5 — disclosed heuristic): era is
        // unrecoverable (no path stored), so the fold applies to ANY same-
        // identity row under a different id, including a genuinely distinct
        // session whose tuple collides (model/end/cost/health excluded) —
        // see `same_derived_session` for the collision shape.
        records.retain(|_, other| other.id == record.id || !same_derived_session(other, &record));
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
    // rm-208: derived history names projects and carries session
    // metrics; write it owner-only instead of the 0644 default.
    crate::session_cache::write_private(&tmp, &serde_json::to_vec_pretty(&records)?)?;
    std::fs::rename(&tmp, &path)?;
    Ok(())
}

pub fn merge_preserved_history(live: &mut Vec<Session>) {
    let mut seen = live
        .iter()
        .map(session_id)
        .collect::<std::collections::BTreeSet<_>>();
    // Review fix (2026-10-06): a DefaultHasher-era row describes a session we
    // already have live under its pinned id — the old id can never match (the
    // original path is not stored in the record, so it cannot be recomputed).
    // Fold such rows away by identity instead of appending a duplicate
    // "history-" row (and remember appended records so legacy duplicates of
    // them are folded too).
    let mut identities: Vec<DerivedSession> =
        live.iter().map(DerivedSession::from_session).collect();
    for record in load_records().into_values() {
        if !seen.insert(record.id.clone()) {
            continue;
        }
        if identities
            .iter()
            .any(|live_record| same_derived_session(live_record, &record))
        {
            continue;
        }
        identities.push(record.clone());
        live.push(record.into_session());
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
    // rm-212: FNV-1a 64-bit over a canonical identity string. std's
    // DefaultHasher (SipHash-1-3) is deterministic today but is NOT
    // guaranteed stable across Rust releases; ids written into a preserved
    // history snapshot must dedupe against ids computed by a later build, so
    // the hash is owned here and pinned by a fixed-vector test. Review fix
    // (2026-10-06): DefaultHasher-era rows cannot be re-keyed (the original
    // path is not stored in the record), so `merge_preserved_history` skips
    // them via `same_derived_session` and `preserve_derived_history` folds
    // them away — an upgrade neither re-admits them as duplicate "history-"
    // rows nor leaves both keys in the file.
    let canonical = format!("{}|{}", session.path, session.metrics.session_start);
    format!("{:016x}", stable_identity_hash(&canonical))
}

const FNV_OFFSET_BASIS: u64 = 0xcbf2_9ce4_8422_2325;
const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;

fn stable_identity_hash(canonical: &str) -> u64 {
    canonical.bytes().fold(FNV_OFFSET_BASIS, |hash, byte| {
        (hash ^ u64::from(byte)).wrapping_mul(FNV_PRIME)
    })
}

/// True when two records describe the same underlying session.
///
/// Review fix (2026-10-06): records written before the id pinning keyed their
/// rows with std's DefaultHasher, so a live session's pinned id can never match
/// them, and the original session path is not stored in the record, so the old
/// id cannot be recomputed. The derived fields that survive the round-trip
/// (start, project, source, token counts, duration, cache split) identify the
/// session well enough to fold such rows away wherever live data is available.
/// `id` is deliberately excluded: that is exactly what differs.
///
/// Review fix (2026-10-07, round 2 LF5 — disclosed heuristic): because era is
/// unrecoverable, this predicate cannot distinguish a legacy DefaultHasher row
/// from a genuinely DISTINCT session whose identity tuple collides. The tuple
/// deliberately excludes model/end/cost/health, so two distinct session files
/// sharing start+project+source+token counts+cache split+duration (realistically
/// empty or duplicated shells) fold into one key, dropping the first row.
/// `merge_preserved_history` shares the same breadth on the dedup side
/// (skip-only, no deletion). Disclosed on the rm-212 row.
fn same_derived_session(a: &DerivedSession, b: &DerivedSession) -> bool {
    a.start == b.start
        && a.project == b.project
        && a.source == b.source
        && a.input == b.input
        && a.output == b.output
        && a.cache_write == b.cache_write
        && a.cache_read == b.cache_read
        && a.duration == b.duration
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

    #[test]
    #[cfg(unix)]
    fn history_file_is_owner_only() {
        // rm-208: derived history names projects and carries session
        // metrics; the 0644 default makes it readable by every local
        // account. It must land owner-only.
        use std::os::unix::fs::PermissionsExt;
        let root = std::env::temp_dir().join(format!(
            "agenttrace-history-perms-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        std::fs::create_dir_all(&root).expect("create temp dir");
        // Point the environment at the temp root under the shared env
        // lock (the default path is the user's real history file).
        let _env = crate::test_env::lock_env();
        let prior = std::env::var_os("AGENTTRACE_HISTORY_DIR");
        std::env::set_var("AGENTTRACE_HISTORY_DIR", &root);
        let session = Session {
            name: "secret task".to_string(),
            path: "/tmp/private/session.jsonl".to_string(),
            cwd: "/work/project".to_string(),
            metrics: Metrics::default(),
            anomalies: Vec::new(),
            health: 95,
            tool_warnings: Vec::new(),
            diagnostics: Diagnostics::default(),
        };
        preserve_derived_history(&[session]).expect("preserve history");
        let path = root.join("history.json");
        let mode = std::fs::metadata(&path)
            .expect("history.json exists")
            .permissions()
            .mode();
        assert_eq!(
            mode & 0o077,
            0,
            "history.json must be owner-only, got {:o}",
            mode & 0o777
        );
        match prior {
            Some(value) => std::env::set_var("AGENTTRACE_HISTORY_DIR", value),
            None => std::env::remove_var("AGENTTRACE_HISTORY_DIR"),
        }
        drop(_env);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn stable_identity_hash_pins_fnv1a_vectors() {
        // rm-212: reference vectors for FNV-1a 64-bit — if std ever changes
        // hashing again these ids must NOT move.
        assert_eq!(stable_identity_hash(""), 0xcbf2_9ce4_8422_2325);
        assert_eq!(stable_identity_hash("a"), 0xaf63_dc4c_8601_ec8c);
        assert_eq!(stable_identity_hash("foobar"), 0x8594_4171_f739_67e8);
    }

    #[test]
    fn session_id_is_deterministic_hex_and_path_sensitive() {
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
        let id = session_id(&session);
        assert_eq!(id.len(), 16);
        assert!(id.chars().all(|c| c.is_ascii_hexdigit()));
        assert_eq!(id, session_id(&session));

        let mut other = session.clone();
        other.path = "/tmp/private/other.jsonl".to_string();
        assert_ne!(id, session_id(&other));
    }

    #[test]
    fn legacy_default_hasher_rows_are_folded_by_identity() {
        // Review fix (2026-10-06): pre-pinning rows (std DefaultHasher ids)
        // duplicated against live sessions on `--include-history` and every
        // `--sessions` run left both keys in the file. Rows whose id disagrees
        // with `stable_identity_hash(label|timestamp)` are re-keyed at load.
        let root = std::env::temp_dir().join(format!(
            "agenttrace-history-rekey-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        std::fs::create_dir_all(&root).expect("create temp dir");
        let _env = crate::test_env::lock_env();
        let prior = std::env::var_os("AGENTTRACE_HISTORY_DIR");
        std::env::set_var("AGENTTRACE_HISTORY_DIR", &root);

        let session = Session {
            name: "legacy row".to_string(),
            path: "/tmp/private/legacy.jsonl".to_string(),
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
        let current = session_id(&session);
        preserve_derived_history(std::slice::from_ref(&session)).expect("seed history file");

        // Simulate the pre-pinning world: same record, but carrying an
        // old-scheme DefaultHasher id in BOTH the map key and the record's
        // own id field (that is what a real legacy row looks like).
        let path = root.join("history.json");
        let mut file: serde_json::Map<String, serde_json::Value> =
            serde_json::from_str(&std::fs::read_to_string(&path).expect("read history file"))
                .expect("history file is a json map");
        let (key, mut value) = match file.iter().next() {
            Some((key, value)) => (key.clone(), value.clone()),
            None => panic!("seeded one record"),
        };
        assert_eq!(key, current, "seeded row must use the current id");
        value.as_object_mut().expect("record is an object").insert(
            "id".to_string(),
            serde_json::Value::String("0123456789abcdef".into()),
        );
        file.clear();
        file.insert("0123456789abcdef".to_string(), value);
        std::fs::write(
            &path,
            serde_json::to_string(&file).expect("serialize legacy map"),
        )
        .expect("write legacy-keyed history file");

        // Merge must NOT append a duplicate row for the same session...
        let mut live = vec![session.clone()];
        merge_preserved_history(&mut live);
        assert_eq!(
            live.len(),
            1,
            "re-keyed row must dedupe against the live session, got {live:?}"
        );

        // ...and a preserve cycle must converge the file back to one key.
        preserve_derived_history(&[session]).expect("re-preserve history");
        let file: serde_json::Map<String, serde_json::Value> =
            serde_json::from_str(&std::fs::read_to_string(&path).expect("re-read history file"))
                .expect("history file is a json map");
        assert_eq!(
            file.keys().collect::<Vec<_>>(),
            vec![&current],
            "no duplicate keys after a preserve cycle: {:?}",
            file.keys().collect::<Vec<_>>()
        );

        match prior {
            Some(value) => std::env::set_var("AGENTTRACE_HISTORY_DIR", value),
            None => std::env::remove_var("AGENTTRACE_HISTORY_DIR"),
        }
        drop(_env);
        let _ = std::fs::remove_dir_all(root);
    }
}
