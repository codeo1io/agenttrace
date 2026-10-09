use crate::{parse_jsonl_session, session_from_events, Event, Session, ToolCall};
use anyhow::{bail, Context};
use serde_json::{Map, Value};
use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

type TokenUsage = BTreeMap<String, i64>;
type JsonObject = Map<String, Value>;
type JsonlProbe = fn(&[JsonObject]) -> Option<Vec<Event>>;
/// Parse-time disclosure counters (kimi-style tuple channel, upstream
/// #316): clamp facts the aggregator cannot re-derive once the clamped
/// values have already been summed.
type ParseCounters = Vec<(String, i64)>;

pub fn parse_file(path: &Path) -> anyhow::Result<Session> {
    if path.is_dir() {
        return parse_cline_task_dir(path);
    }
    if is_cline_task_file(path) {
        if let Some(dir) = path.parent() {
            if let Ok(session) = parse_cline_task_dir(dir) {
                return Ok(session);
            }
        }
    }
    let raw =
        std::fs::read(path).with_context(|| format!("read session file {}", path.display()))?;
    let name = session_name(path);
    let path_text = path.to_string_lossy().to_string();
    parse_session_bytes(raw, &path_text, &name)
}

/// Decode + parse one session stream (rm-503): the shared tail of
/// `parse_file` past the byte read. Encoding guards keep the exact
/// wording the file path produced, with `label` standing in for the
/// path so stdin failures read `session file <stdin> is …`.
fn parse_session_bytes(raw: Vec<u8>, label: &str, name: &str) -> anyhow::Result<Session> {
    // Windows tooling (notably PowerShell 5.1's `>` redirection) writes
    // UTF-16 with a BOM by default; name the encoding instead of failing
    // with a generic read error (pass-7 P7-2).
    if raw.starts_with(&[0xFF, 0xFE]) || raw.starts_with(&[0xFE, 0xFF]) {
        bail!(
            "session file {} is UTF-16 encoded; convert it to UTF-8 and retry",
            label
        );
    }
    // Codex ≥0.152 stores rollouts as zstd frames (magic 28 B5 2F FD,
    // upstream PR #41357); they are no longer plain JSONL. Name the
    // format instead of failing with a generic "not valid UTF-8"
    // (pass-7 research, candidate 44 minimum).
    if raw.starts_with(&[0x28, 0xB5, 0x2F, 0xFD]) {
        bail!(
            "session file {} is zstd-compressed (Codex rollout format); decompress it to JSONL first (e.g. `zstd -d rollout.jsonl.zst -o rollout.jsonl`, then re-run or pipe)",
            label
        );
    }
    let raw = String::from_utf8(raw)
        .with_context(|| format!("read session file {} (not valid UTF-8)", label))?;
    parse_raw_session(name, label, &raw)
}

/// Parse one session stream piped on stdin (rm-503, `agenttrace … -`):
/// the same decode and parse path as a named file with the byte source
/// swapped. The `<stdin>` label is display-only — no file is ever
/// touched for it — and stdin sessions are ephemeral, so callers keep
/// them out of the session cache.
pub fn parse_stdin_bytes(raw: Vec<u8>) -> anyhow::Result<Session> {
    parse_session_bytes(raw, "<stdin>", "stdin")
}

fn parse_cline_task_dir(dir: &Path) -> anyhow::Result<Session> {
    let metadata = read_json_value(&dir.join("task_metadata.json")).unwrap_or(Value::Null);
    let model = metadata
        .as_object()
        .map(cline_model)
        .unwrap_or_else(|| "unknown".to_string());
    let mut events = Vec::new();
    let mut seen = BTreeSet::new();

    if let Some(raw) = read_json_value(&dir.join("api_conversation_history.json")) {
        if let Some(api_events) = parse_cline_value(&raw, &model) {
            for event in api_events {
                append_cline_event(&mut events, &mut seen, event);
            }
        }
    }
    if let Some(raw) = read_json_value(&dir.join("ui_messages.json")) {
        for event in parse_cline_ui_messages(&raw, &model) {
            append_cline_event(&mut events, &mut seen, event);
        }
    }
    if events.is_empty() {
        bail!("cline: no parseable events in {}", dir.display());
    }
    if let Some(metadata) = metadata.as_object() {
        apply_cline_metadata_timestamps(&mut events, metadata);
    }
    let name = dir
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("cline-task")
        .to_string();
    session_from_events(&name, &dir.to_string_lossy(), events)
}

pub fn parse_raw_session(name: &str, path: &str, raw: &str) -> anyhow::Result<Session> {
    // Strip one UTF-8 BOM at offset 0 and nowhere else (pass-7 P7-2):
    // every parse path funnels through this entry, so a single strip
    // covers every format, and a U+FEFF embedded later in the content
    // survives as content.
    let raw = raw.strip_prefix('\u{feff}').unwrap_or(raw);
    let trimmed = raw.trim();
    if is_aider_history(path, trimmed) {
        return session_from_events(name, path, parse_aider_chat_history(raw)?);
    }
    if trimmed.is_empty() {
        bail!("empty session");
    }
    let parsed_value = serde_json::from_str::<Value>(trimmed).ok();
    if let Some(value) = &parsed_value {
        if is_qwen_code_document(value) {
            return session_from_events(name, path, parse_qwen_code_value(value)?);
        }
        if let Some(events) = parse_openclaw_value(value) {
            return session_from_events(name, path, events);
        }
        if let Some(events) = parse_hermes_json_value(value) {
            return session_from_events(name, path, events);
        }
        // Single-object Antigravity trajectory sidecars
        // (<uuid>.trajectory.json) sit next to the SQLite conversation
        // store and parse as one JSON document, not JSONL (CU-19).
        if let Some(events) = parse_antigravity_trajectory(raw) {
            return session_from_events(name, path, events);
        }
    }
    if parsed_value.is_none() {
        if let Some((events, parse_counters)) = parse_codex_rollout_jsonl(raw) {
            let mut session = session_from_events(name, path, events)?;
            // rm-047 made the fast-path skips visible in parse diagnostics;
            // rm-401 widened the channel to every codex parse decision
            // worth disclosing (ignorable lines, counted/deduped/unpaired
            // token_usage_records).
            for (key, count) in parse_counters {
                // rm-730: the two STRUCTURAL counters are deterministic
                // known-non-loss shapes — event_msg chatter the format
                // defines as ignorable, and the world_state sub-registry
                // record that is deliberately neither tool nor message —
                // so they are assumption-class disclosures under the
                // rm-538 channel split and ride disclosure_counters,
                // where they stay fully visible (report Disclosed-facts
                // row, data_health.disclosures, doctor) but stop tanking
                // data_health.confidence for an otherwise-exact parse
                // (the 39acfe43 live PoC: 97/97 parsed, 0 skipped files,
                // yet confidence "low" via 1,125 structural counters).
                // True loss (codex_unparseable_line,
                // codex_non_object_line:*, codex_missing_type,
                // codex_unmatched_*) and the compaction usage-record
                // accounting decisions stay on line_skips and keep the
                // 39acfe43 contract: genuinely dropped lines degrade
                // confidence.
                if key == "codex_ignorable_line" || key == "codex_world_state" {
                    *session.metrics.disclosure_counters.entry(key).or_insert(0) += count;
                } else {
                    *session.metrics.line_skips.entry(key).or_insert(0) += count;
                }
            }
            return Ok(session);
        }
    }
    // rm-526: `jsonl_objects` silently dropped lines that fail both the
    // strict and the lenient parse, so a torn tail (a writer crash
    // mid-object — the read side of the hazard rm-250 hardened the
    // writers against) vanished whole: the session parsed green with
    // zero skips and its usage undercounted. Count the drops here and
    // attach them to every session parsed out of `objs`, through the
    // same `line_skips` channel the codex parser and the generic
    // fallback already disclose through.
    let (objs, jsonl_line_skips) = jsonl_objects_counted(raw);
    let finish = |events: Vec<Event>| -> anyhow::Result<Session> {
        let mut session = session_from_events(name, path, events)?;
        for (reason, count) in &jsonl_line_skips {
            *session
                .metrics
                .line_skips
                .entry(reason.clone())
                .or_insert(0) += *count;
        }
        Ok(session)
    };
    if parsed_value.is_none() {
        // Workbuddy parses FIRST (probe order) but carries its own
        // disclosure channel for the cache-clamp counter (rm-600,
        // upstream #316) — exactly like kimi's alias counters below.
        // rm-538 (integrated 2026-10-06) moved the whole
        // workbuddy_input_basis:* family onto Metrics.disclosure_counters
        // — the non-loss channel — so the clamp fact rides the SAME map
        // as its cache_subtracted / zeroed_suspected_mismatch siblings
        // (landed by the aggregator's meta arm) instead of degrading
        // data_health.confidence and rendering under "Dropped lines".
        // The session itself still goes through the rm-526 `finish`
        // closure (re-threaded at the independent review, conflict case
        // 7a502782): moving workbuddy out of the probe array for its own
        // counter channel must not lose the jsonl line-skips census — a
        // torn-tail workbuddy journal discloses `unparseable_line` in
        // `line_skips` exactly like every other probe-path format, while
        // its basis counters stay on the non-loss channel.
        if let Some((events, parse_counters)) = parse_workbuddy_jsonl(&objs) {
            let mut session = finish(events)?;
            for (key, count) in parse_counters {
                *session.metrics.disclosure_counters.entry(key).or_insert(0) += count as usize;
            }
            return Ok(session);
        }
        let probes: [JsonlProbe; 4] = [
            parse_antigravity_jsonl,
            parse_cursor_transcript_jsonl,
            parse_claude_transcript_jsonl,
            parse_copilot_session_jsonl,
        ];
        if let Some(events) = probes.iter().find_map(|probe| probe(&objs)) {
            return finish(events);
        }
        // kimi_cli parses outside the probe array — last, exactly where it
        // used to sit — because it carries its own disclosure channel for
        // alias-matched usage fields (rm-400). rm-719 moved that channel
        // from `line_skips` to `disclosure_counters`: an alias match is
        // informational (the vendor's real wire keys were matched, the
        // usage counted exactly), so it renders under "Disclosed facts"
        // like the workbuddy basis family above instead of degrading
        // data_health.confidence as parse loss.
        if let Some((events, usage_alias_counts)) = parse_kimi_wire_jsonl(&objs) {
            let mut session = finish(events)?;
            for (key, count) in usage_alias_counts {
                *session
                    .metrics
                    .disclosure_counters
                    .entry(format!("kimi_usage_alias:{key}"))
                    .or_insert(0) += count;
            }
            return Ok(session);
        }
    }
    if is_qwen_code_jsonl(&objs) {
        return finish(parse_qwen_code_jsonl(&objs)?);
    }
    if is_oh_my_pi_jsonl(&objs) {
        return finish(parse_oh_my_pi_jsonl(path, &objs)?);
    }
    if let Some(events) = parse_claude_code_jsonl(&objs) {
        return finish(events);
    }
    if let Some(events) = parse_copilot_jsonl(&objs) {
        return finish(events);
    }
    if let Ok(events) = serde_json::from_str::<Vec<Event>>(trimmed) {
        if !events.is_empty() {
            let mut events = events;
            for event in &mut events {
                if event.source_tool.is_empty() {
                    event.source_tool = "generic".to_string();
                }
            }
            return session_from_events(name, path, events);
        }
    }
    if let Some(value) = parsed_value {
        if let Some(events) = parse_opencode_storage_value(path, &value) {
            return session_from_events(name, path, events);
        }
        if let Some(events) = parse_cursor_export(&value) {
            return session_from_events(name, path, events);
        }
        if let Some(events) = parse_gemini_value(&value) {
            return session_from_events(name, path, events);
        }
        if let Some(events) = parse_kimi_value(&value) {
            return session_from_events(name, path, events);
        }
        if let Some(events) = parse_cline_value(&value, "unknown") {
            return session_from_events(name, path, events);
        }
        if let Some(events) = parse_messages_value(&value, "codex_cli") {
            return session_from_events(name, path, events);
        }
    }
    if let Ok(session) = parse_jsonl_session(name, path, raw) {
        return Ok(session);
    }
    bail!("unsupported session format: {}", path)
}

// K1 rider history: this fn once carried a module-level CheckpointSnapshot
// alias to stay under clippy's type_complexity budget (rm-538's rider);
// rm-551's per-model fold removed the nested shape and the alias with it.
fn parse_copilot_session_jsonl(objs: &[JsonObject]) -> Option<Vec<Event>> {
    if !objs
        .iter()
        .any(|entry| string(entry.get("type")) == Some("session.start"))
    {
        return None;
    }
    let mut events = Vec::new();
    // rm-485: Copilot's billing truth is the session-wide AIU credit
    // counter, not the per-model token counts (ccusage #1823: a resumed
    // session shut down with empty/partial modelMetrics while
    // totalNanoAiu carried the whole session; usage_checkpoint records are
    // the only usage surface on open/killed sessions). Track the largest
    // counter seen (checkpoints and shutdown snapshots overlap) and whether
    // shutdown metrics were emitted at all.
    // rm-551: reconcile copilot usage PER MODEL, not per snapshot. A
    // checkpoint (or shutdown) is the freshest word on exactly the models
    // it names: it REPLACES those entries and PRESERVES every other
    // model's last-known values, so a rotation across checkpoints and a
    // shutdown naming only a subset of the checkpointed models can no
    // longer drop a model's cumulative tokens wholesale (the assess PoCs;
    // ccusage #1824's subtract_usage reconciliation keeps the same
    // per-entry discipline for credits).
    struct CopilotModelSnapshot {
        timestamp: String,
        usage: BTreeMap<String, i64>,
    }
    let mut per_model: BTreeMap<String, CopilotModelSnapshot> = BTreeMap::new();
    // rm-721 / codeburn #1651: VS Code agent-host journals (workspace.yaml
    // client_name: vscode-agent-host; Copilot CLI 1.0.8x writes the same
    // shape) interleave entry types the counting rules do not fold —
    // assistant.turn_start/turn_end carry only ids, IDE-side entries vary.
    // Name every un-counted entry type instead of letting it vanish (the
    // disclosure rule rm-584 landed for the codex lane).
    let mut uncounted: BTreeMap<String, i64> = BTreeMap::new();
    let mut max_credit_nano: f64 = 0.0;
    // Review 3e3a2198 F4: per-model credit meters (modelMetrics[m].
    // totalNanoAiu). Each entry is one model's OWN bill (codeburn #1651
    // reads them as per-leg deltas), so the values SUM at emit — a global
    // max across them under-reports a 2-model rollup; only a model's own
    // re-emitted snapshots need the running-bill max discipline.
    let mut per_model_credit_nano: BTreeMap<String, f64> = BTreeMap::new();
    // rm-555 (folded into rm-551): shutdown modelMetrics are per-model
    // cumulative snapshots and every resume writes another shutdown
    // record, so only the LATER record per model may count — the per_model
    // fold replaces on insert and the per-arm stale check skips an
    // out-of-order earlier line (ccusage #1823/#1824; upstream #312
    // dedupes "to the latest per model"). The rm-551 snapshot map carries
    // the per-model timestamps this needs; a separate shutdown/checkpoint
    // map pair would re-open the whole-snapshot swap the fold closed.
    // rm-556: a metrics-less shutdown record still ends the session — its
    // timestamp is the latest activity and must reach an event, or the
    // duration loses the tail (the credit event used to ride the
    // checkpoint's timestamp or the empty default).
    let mut latest_shutdown_timestamp = String::new();
    let mut latest_checkpoint_timestamp = String::new();
    for entry in objs.iter() {
        let typ = string(entry.get("type")).unwrap_or("");
        let timestamp = string(entry.get("timestamp")).unwrap_or("").to_string();
        let data = entry.get("data").and_then(Value::as_object);
        match typ {
            "session.start" => events.push(Event {
                role: "session_meta".to_string(),
                timestamp,
                cwd: data
                    .and_then(|data| data.get("context"))
                    .and_then(|context| context.get("cwd"))
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_string(),
                source_tool: "copilot_cli".to_string(),
                ..Event::default()
            }),
            "user.message" | "assistant.message" => events.push(Event {
                role: typ.trim_end_matches(".message").to_string(),
                content: data
                    .and_then(|data| data.get("content").or_else(|| data.get("message")))
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_string(),
                timestamp,
                source_tool: "copilot_cli".to_string(),
                ..Event::default()
            }),
            "tool.execution_start" => events.push(Event {
                role: "assistant".to_string(),
                timestamp,
                tool_calls: vec![ToolCall {
                    id: data
                        .and_then(|data| data.get("toolCallId"))
                        .and_then(Value::as_str)
                        .unwrap_or("")
                        .to_string(),
                    name: data
                        .and_then(|data| data.get("toolName"))
                        .and_then(Value::as_str)
                        .unwrap_or("")
                        .to_string(),
                    args: jsonish(data.and_then(|data| data.get("arguments"))),
                }],
                source_tool: "copilot_cli".to_string(),
                ..Event::default()
            }),
            "tool.execution_complete" => events.push(Event {
                role: "tool".to_string(),
                timestamp,
                tool_call_id: data
                    .and_then(|data| data.get("toolCallId"))
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_string(),
                is_error: data
                    .and_then(|data| data.get("success"))
                    .and_then(Value::as_bool)
                    == Some(false),
                source_tool: "copilot_cli".to_string(),
                ..Event::default()
            }),
            "session.shutdown" => {
                // rm-551: shutdown metrics are the freshest word on the
                // models they name — they replace those per-model entries
                // (identical values on a clean shutdown, newer values on a
                // caught-up one) without touching models the shutdown
                // omitted. Emission is deferred to the single post-loop
                // fold so checkpoints and shutdowns share one code path.
                if let Some(metrics) = data
                    .and_then(|data| data.get("modelMetrics"))
                    .and_then(Value::as_object)
                {
                    for (model, metric) in metrics {
                        // rm-721 / codeburn #1651: the agent-host rollup
                        // carries the per-model running Copilot-credit bill
                        // inside modelMetrics (the CLI-shape top-level
                        // totalNanoAiu is absent there), so credits must read
                        // both shapes — max-fold, matching the rm-485
                        // family's repeated-snapshot semantics.
                        if let Some(nano) = metric.get("totalNanoAiu").and_then(Value::as_f64) {
                            if nano.is_finite() {
                                let slot =
                                    per_model_credit_nano.entry(model.clone()).or_insert(0.0);
                                if nano > *slot {
                                    *slot = nano;
                                }
                            }
                        }
                        if let Some(mut usage) = metric.get("usage").and_then(usage_from_value) {
                            // Copilot modelMetrics follow the GenAI
                            // semconv basis (input_tokens INCLUDES
                            // cached tokens — upstream #316 treats the
                            // workbuddy and Copilot lanes alike), so
                            // normalize to agenttrace's delta basis via
                            // the shared clamped subtraction (rm-600)
                            // before the row enters the rm-551 fold.
                            subtract_cached_input(
                                &mut usage,
                                &["cache_creation_input_tokens", "cache_read_input_tokens"],
                            );
                            // rm-555: only the LATER record per model counts
                            // (a resume re-emits the cumulative snapshot),
                            // so an out-of-order earlier line cannot
                            // overwrite a fresher one.
                            let stale = per_model
                                .get(model)
                                .map(|seen| later_rfc3339(&seen.timestamp, &timestamp) != timestamp)
                                .unwrap_or(false);
                            if !stale {
                                per_model.insert(
                                    model.clone(),
                                    CopilotModelSnapshot {
                                        timestamp: timestamp.clone(),
                                        usage,
                                    },
                                );
                            }
                        }
                    }
                }
                latest_shutdown_timestamp = later_rfc3339(&latest_shutdown_timestamp, &timestamp);
                if let Some(nano) = data
                    .and_then(|data| data.get("totalNanoAiu"))
                    .and_then(Value::as_f64)
                {
                    if nano.is_finite() && nano > max_credit_nano {
                        max_credit_nano = nano;
                    }
                }
            }
            // rm-485/rm-551: checkpoints carry the same shapes as shutdown
            // metrics on sessions that never shut down cleanly. Counters
            // are re-emitted across checkpoints, so a checkpoint replaces
            // only the per-model entries it names (max semantics per entry,
            // never a whole-snapshot swap) and credits fold with max
            // semantics — summing would double-count every overlap
            // (ccusage #1824 reconciliation).
            "session.usage_checkpoint" => {
                if let Some(metrics) = data
                    .and_then(|data| data.get("modelMetrics"))
                    .and_then(Value::as_object)
                {
                    for (model, metric) in metrics {
                        // rm-721: same dual-shape credit read as the
                        // shutdown arm — checkpoints carry modelMetrics
                        // with totalNanoAiu on live agent-host sessions
                        // (top-level nanoAiu but no tokens, #1651).
                        if let Some(nano) = metric.get("totalNanoAiu").and_then(Value::as_f64) {
                            if nano.is_finite() {
                                let slot =
                                    per_model_credit_nano.entry(model.clone()).or_insert(0.0);
                                if nano > *slot {
                                    *slot = nano;
                                }
                            }
                        }
                        if let Some(mut usage) = metric.get("usage").and_then(usage_from_value) {
                            // rm-485 checkpoints carry the same
                            // modelMetrics shapes as shutdown — apply
                            // the same delta-basis clamp (rm-600) so an
                            // open/killed session totals identically to
                            // one that shut down cleanly, before the
                            // row enters the rm-551 fold.
                            subtract_cached_input(
                                &mut usage,
                                &["cache_creation_input_tokens", "cache_read_input_tokens"],
                            );
                            // rm-555: same later-record-per-model rule as the
                            // shutdown arm — a stale replayed checkpoint line
                            // cannot overwrite a fresher snapshot.
                            let stale = per_model
                                .get(model)
                                .map(|seen| later_rfc3339(&seen.timestamp, &timestamp) != timestamp)
                                .unwrap_or(false);
                            if !stale {
                                per_model.insert(
                                    model.clone(),
                                    CopilotModelSnapshot {
                                        timestamp: timestamp.clone(),
                                        usage,
                                    },
                                );
                            }
                        }
                    }
                }
                latest_checkpoint_timestamp =
                    later_rfc3339(&latest_checkpoint_timestamp, &timestamp);
                if let Some(nano) = data
                    .and_then(|data| data.get("totalNanoAiu"))
                    .and_then(Value::as_f64)
                {
                    if nano.is_finite() && nano > max_credit_nano {
                        max_credit_nano = nano;
                    }
                }
            }
            _ => {
                // rm-721: name the un-counted class (turn markers, IDE
                // entries) instead of dropping it silently.
                if !typ.is_empty() {
                    *uncounted.entry(typ.to_string()).or_insert(0) += 1;
                }
            }
        }
    }
    // rm-721: one carrier names every un-counted entry type. A bare
    // meta marker carries no usage and an empty timestamp, so nothing
    // else in the metrics can move (the rm-556 fallback pattern).
    if !uncounted.is_empty() {
        let counters = uncounted
            .into_iter()
            // rm-594 composition at integration: the entry type is
            // journal-derived, so the key must route through the ONE
            // capped+sanitized mint helper like every other site — a
            // hostile agent-host journal can carry an over-long or
            // ANSI-laden `type` field (benign short types are
            // byte-identical to the raw form).
            .map(|(typ, count)| (disclosure_key("copilot_uncounted_entry_type", &typ), count))
            .collect();
        events.insert(
            0,
            Event {
                role: "meta".to_string(),
                disclosure_counters: counters,
                source_tool: "copilot_cli".to_string(),
                ..Event::default()
            },
        );
    }
    // rm-551: one meta event per observed model, stamped with the
    // timestamp that last named it — lib.rs sums across models, prices
    // each block at its own model (usage_blocks_multi_model) and labels a
    // multi-model session "multiple", so no model vanishes and none
    // double-counts (a re-emitted model replaces only its own entry).
    // rm-485/rm-555: checkpoints fill exactly the models no shutdown record
    // carried (the partial-shutdown shape) and the later cumulative record
    // per model replaces the earlier one — both hold because the fold
    // already replaced per model above instead of stacking two emissions.
    for (model, snapshot) in &per_model {
        events.insert(
            0,
            Event {
                role: "meta".to_string(),
                timestamp: snapshot.timestamp.clone(),
                usage: snapshot.usage.clone(),
                model_used: model.clone(),
                source_tool: "copilot_cli".to_string(),
                ..Event::default()
            },
        );
    }
    let mut latest_usage_timestamp = String::new();
    for snapshot in per_model.values() {
        latest_usage_timestamp = later_rfc3339(&latest_usage_timestamp, &snapshot.timestamp);
    }
    // rm-485: emit the session-wide credit counter as a cost-only meta
    // event (1 AIU = 1 AI credit = $0.01 per ccusage #1824 semantics). Max
    // semantics upstream keep repeated snapshots from double-counting.
    // rm-556: the timestamp is the latest activity across checkpoint and
    // shutdown records — a metrics-less shutdown at T+20s over messages
    // ending at T+1s used to leave the duration at 1.0s.
    let credit_timestamp = later_rfc3339(&latest_checkpoint_timestamp, &latest_shutdown_timestamp);
    // Review 3e3a2198 F4: per-model meters SUM (each is one model's bill);
    // the top-level totalNanoAiu (CLI 1.0.8x shutdown/checkpoint records)
    // stays max-folded as the session-wide counter. Both describe the
    // same bill, so the greater of the two is exact on either shape and
    // never double-counts; a global max across per-model meters silently
    // under-reports a 2-model rollup with no disclosure.
    let per_model_credit_sum: f64 = per_model_credit_nano.values().sum();
    let credit_nano = max_credit_nano.max(per_model_credit_sum);
    if credit_nano > 0.0 {
        events.insert(
            0,
            Event {
                role: "meta".to_string(),
                timestamp: credit_timestamp,
                model_used: "unknown".to_string(),
                credit_usd: credit_nano * 0.01 / 1_000_000_000.0,
                source_tool: "copilot_cli".to_string(),
                ..Event::default()
            },
        );
    } else if !latest_shutdown_timestamp.is_empty()
        && later_rfc3339(&latest_usage_timestamp, &latest_shutdown_timestamp)
            == latest_shutdown_timestamp
    {
        // rm-556 fallback: a credit-less, metrics-less shutdown record
        // still ends the session; a bare terminal marker keeps its
        // timestamp inside the duration span (no usage, so nothing else
        // in the metrics can move).
        events.insert(
            0,
            Event {
                role: "meta".to_string(),
                timestamp: latest_shutdown_timestamp,
                source_tool: "copilot_cli".to_string(),
                ..Event::default()
            },
        );
    }
    non_empty(events)
}

// kimi_cli's token_usage wire keys; every match is disclosed per session in
// parse diagnostics (rm-400) so a future wire change cannot silently
// zero usage again.
const KIMI_USAGE_ALIAS_KEYS: [&str; 4] = [
    "input_other",
    "output",
    "input_cache_read",
    "input_cache_creation",
];

fn parse_kimi_wire_jsonl(objs: &[JsonObject]) -> Option<(Vec<Event>, BTreeMap<String, usize>)> {
    let mut usage_alias_counts: BTreeMap<String, usize> = BTreeMap::new();
    if !objs.iter().any(|entry| {
        entry
            .get("message")
            .and_then(Value::as_object)
            .is_some_and(|message| message.contains_key("type") && message.contains_key("payload"))
    }) {
        return None;
    }
    let mut events = Vec::new();
    for entry in objs.iter() {
        let timestamp = entry
            .get("timestamp")
            .and_then(Value::as_f64)
            .map(|seconds| timestamp_millis((seconds * 1000.0) as i64))
            .unwrap_or_default();
        let Some(message) = entry.get("message").and_then(Value::as_object) else {
            continue;
        };
        let typ = string(message.get("type")).unwrap_or("");
        let payload = message.get("payload").and_then(Value::as_object);
        match typ {
            "TurnBegin" | "SteerInput" => events.push(Event {
                role: "user".to_string(),
                content: payload
                    .and_then(|payload| payload.get("user_input"))
                    .map(|value| jsonish(Some(value)))
                    .unwrap_or_default(),
                timestamp,
                source_tool: "kimi_cli".to_string(),
                ..Event::default()
            }),
            "TextPart" => events.push(Event {
                role: "assistant".to_string(),
                content: payload
                    .and_then(|payload| payload.get("text"))
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_string(),
                timestamp,
                source_tool: "kimi_cli".to_string(),
                ..Event::default()
            }),
            "ThinkPart" => events.push(Event {
                role: "assistant".to_string(),
                reasoning: payload
                    .and_then(|payload| payload.get("think").or_else(|| payload.get("text")))
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_string(),
                timestamp,
                source_tool: "kimi_cli".to_string(),
                ..Event::default()
            }),
            "ToolCall" | "ToolCallPart" => events.push(Event {
                role: "assistant".to_string(),
                timestamp,
                tool_calls: vec![ToolCall {
                    id: payload
                        .and_then(|payload| payload.get("id"))
                        .and_then(Value::as_str)
                        .unwrap_or("")
                        .to_string(),
                    name: payload
                        .and_then(|payload| payload.get("function"))
                        .and_then(|function| function.get("name"))
                        .or_else(|| payload.and_then(|payload| payload.get("name")))
                        .and_then(Value::as_str)
                        .unwrap_or("")
                        .to_string(),
                    args: jsonish(
                        payload
                            .and_then(|payload| payload.get("function"))
                            .and_then(|function| function.get("arguments"))
                            .or_else(|| payload.and_then(|payload| payload.get("arguments"))),
                    ),
                }],
                source_tool: "kimi_cli".to_string(),
                ..Event::default()
            }),
            "ToolResult" => events.push(Event {
                role: "tool".to_string(),
                content: payload
                    .map(|payload| jsonish(Some(&Value::Object(payload.clone()))))
                    .unwrap_or_default(),
                timestamp,
                source_tool: "kimi_cli".to_string(),
                ..Event::default()
            }),
            "StatusUpdate" => {
                // rm-400: kimi reports per-StatusUpdate token_usage
                // under its own wire keys (input_other / output /
                // input_cache_read / input_cache_creation); usage_from_value
                // now maps them, and every alias match is disclosed so a
                // future key change surfaces in parse diagnostics instead of
                // as silently-zero usage. Meta usage is summed per key in
                // analyze, so the event rides in arrival order — the old
                // insert(0) inverted multi-record journals.
                if let Some(token_usage) = payload.and_then(|payload| payload.get("token_usage")) {
                    let (usage, matched_keys) = usage_from_value_with_keys(token_usage);
                    for key in matched_keys {
                        if KIMI_USAGE_ALIAS_KEYS.contains(&key) {
                            *usage_alias_counts.entry(key.to_string()).or_insert(0) += 1;
                        }
                    }
                    if let Some(usage) = usage {
                        events.push(Event {
                            role: "meta".to_string(),
                            usage,
                            source_tool: "kimi_cli".to_string(),
                            ..Event::default()
                        });
                    }
                }
            }
            _ => {}
        }
    }
    non_empty(events).map(|events| (events, usage_alias_counts))
}

// rm-720 / codeburn #1655: antigravity generations carry usage via the
// language-server RPC's JSON names (`inputTokens`, `outputTokens`,
// `cacheReadInputTokens`, `cacheCreationInputTokens`,
// `reasoningTokens` — exa.codeium_common_pb.ModelUsageStats fields
// 2/3/4/5/9). No export surface pins one nesting key, so every
// candidate below is probed per generation and the first block that
// yields a number wins; the fold sums the per-generation blocks
// (gen_metadata rows are one completion turn each) and names its basis
// in a disclosure counter. Field 2 is UNCACHED input and field 5 the
// cache read, so the folded map keeps agenttrace's delta basis with no
// GenAI clamp (unlike the copilot lane's rm-600).
//
// Review 3e3a2198 F7 removed `modelUsage` from this list: the qwen lane's
// nested per-model map (see `qwen_model_usage`) would be silently
// swallowed by the flat-key reader below — a dead probe masquerading as
// coverage. The only observed antigravity carrier is `usageStats`
// (codeburn #1655); if a specimen journal ever pins a nested
// `modelUsage` shape, fold it with the qwen per-model mechanism instead
// of re-adding the key here.
const ANTIGRAVITY_USAGE_KEYS: [&str; 3] = ["usageStats", "usage", "usageMetadata"];

// The standalone app names models by config id (Field 19,
// `gemini-pro-default`) or the `model_enum` metadata key (Field 20);
// the display name (Field 21) is absent there. The id passes through
// verbatim: a priced id resolves standalone, and an unpriced one
// surfaces through cost_audit's pricing-coverage channel
// (exact/fallback/unknown) instead of silently costing $0.
const ANTIGRAVITY_MODEL_KEYS: [&str; 4] = ["modelConfigId", "model_enum", "modelVersion", "model"];

fn antigravity_usage_block(obj: &Map<String, Value>) -> Option<BTreeMap<String, i64>> {
    ANTIGRAVITY_USAGE_KEYS
        .iter()
        .find_map(|key| obj.get(*key).and_then(usage_from_value))
}

fn antigravity_model_id(obj: &Map<String, Value>) -> Option<String> {
    ANTIGRAVITY_MODEL_KEYS
        .iter()
        .find_map(|key| string(obj.get(*key)).map(str::to_string))
        .filter(|model| !model.is_empty())
}

/// rm-720 fold state: sums per-generation usage blocks and emits ONE
/// meta event carrying the session totals, the last-seen model id and
/// the basis counters (lib.rs's meta arm folds usage maps additively,
/// so a single totals event is exactly right for summed blocks).
struct AntigravityUsageFold {
    usage: BTreeMap<String, i64>,
    model: String,
    models_seen: usize,
    generations: usize,
    latest_timestamp: String,
}

impl AntigravityUsageFold {
    fn new() -> Self {
        AntigravityUsageFold {
            usage: BTreeMap::new(),
            model: String::new(),
            models_seen: 0,
            generations: 0,
            latest_timestamp: String::new(),
        }
    }

    fn fold(&mut self, usage: &BTreeMap<String, i64>, model: Option<String>, timestamp: &str) {
        self.generations += 1;
        for (key, value) in usage {
            *self.usage.entry(key.clone()).or_insert(0) += *value;
        }
        if let Some(model) = model {
            self.models_seen += 1;
            self.model = model;
        }
        self.latest_timestamp = later_rfc3339(&self.latest_timestamp, timestamp);
    }

    fn meta_event(&self) -> Option<Event> {
        if self.generations == 0 {
            return None;
        }
        // Units (review 3e3a2198 F8): `planner_response_summed` counts
        // GENERATIONS — one folded completion turn each, so a multi-turn
        // session reports N, not 1; `multi_model_last_wins` counts model-id
        // SIGHTINGS across those generations (and the model id itself is
        // last-wins). Neither counter is a session count.
        let mut counters = BTreeMap::new();
        counters.insert(
            "antigravity_usage_basis:planner_response_summed".to_string(),
            self.generations as i64,
        );
        if self.models_seen > 1 {
            counters.insert(
                "antigravity_model:multi_model_last_wins".to_string(),
                self.models_seen as i64,
            );
        }
        Some(Event {
            role: "meta".to_string(),
            timestamp: self.latest_timestamp.clone(),
            usage: self.usage.clone(),
            model_used: self.model.clone(),
            disclosure_counters: counters,
            source_tool: "antigravity_cli".to_string(),
            ..Event::default()
        })
    }
}

/// Parses a decrypted Antigravity trajectory sidecar
/// (`<uuid>.trajectory.json` under
/// `~/.gemini/antigravity-cli/conversations/`). The on-disk conversation
/// store itself is an undocumented SQLite database (agy 1.1.23:
/// `user_version=1`, 7 tables) whose blobs are protobuf payloads; the JSON
/// sidecar is the documented reader surface (schema cross-checked against
/// mjacobs/agy-reader `internal/daemon/types.go`, verified against agy
/// 1.1.23 on 2026-09-01). Planner steps carry per-generation usage
/// blocks under the language-server RPC's JSON names (rm-720, codeburn
/// #1655's `exa.codeium_common_pb.ModelUsageStats` field map) — folded
/// by [`AntigravityUsageFold`] below, never silently dropped.
fn parse_antigravity_trajectory(raw: &str) -> Option<Vec<Event>> {
    let value: Value = serde_json::from_str(raw.trim_start()).ok()?;
    let obj = value.as_object()?;
    let steps = obj.get("steps")?.as_array()?;
    if steps.is_empty() {
        return None;
    }
    let is_trajectory_step = |entry: &&Value| {
        entry
            .get("type")
            .and_then(Value::as_str)
            .is_some_and(|typ| typ.starts_with("CORTEX_STEP_TYPE_"))
            && entry
                .pointer("/metadata/createdAt")
                .and_then(Value::as_str)
                .is_some()
    };
    steps.iter().find(is_trajectory_step)?;
    let mut usage_fold = AntigravityUsageFold::new();
    let mut events = Vec::new();
    for step in steps {
        let typ = step.get("type").and_then(Value::as_str).unwrap_or("");
        let timestamp = step
            .pointer("/metadata/createdAt")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        let event = |role: &str, content: String, is_error: bool| Event {
            role: role.to_string(),
            content,
            timestamp: timestamp.clone(),
            is_error,
            source_tool: "antigravity_cli".to_string(),
            ..Event::default()
        };
        match typ {
            "CORTEX_STEP_TYPE_USER_INPUT" => {
                let content = step
                    .pointer("/userInput/userResponse")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_string();
                if !content.is_empty() {
                    events.push(event("user", content, false));
                }
            }
            "CORTEX_STEP_TYPE_PLANNER_RESPONSE" => {
                let response = step
                    .pointer("/plannerResponse/response")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_string();
                let thinking = step
                    .pointer("/plannerResponse/thinking")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_string();
                let tool_calls: Vec<ToolCall> = step
                    .pointer("/plannerResponse/toolCalls")
                    .and_then(Value::as_array)
                    .into_iter()
                    .flatten()
                    .filter_map(|call| {
                        let name = call.get("name")?.as_str()?.to_string();
                        let args = call
                            .get("argumentsJson")
                            .and_then(Value::as_str)
                            .and_then(|text| serde_json::from_str::<Value>(text).ok())
                            .as_ref()
                            .map(|value| jsonish(Some(value)))
                            .unwrap_or_default();
                        Some(ToolCall {
                            name,
                            args,
                            ..ToolCall::default()
                        })
                    })
                    .collect();
                if response.is_empty() && thinking.is_empty() && tool_calls.is_empty() {
                    continue;
                }
                let mut assistant = event("assistant", response, false);
                assistant.reasoning = thinking;
                assistant.tool_calls = tool_calls;
                events.push(assistant);
                if let Some(planner) = step.pointer("/plannerResponse").and_then(Value::as_object) {
                    if let Some(usage) = antigravity_usage_block(planner) {
                        usage_fold.fold(&usage, antigravity_model_id(planner), &timestamp);
                    }
                }
            }
            "CORTEX_STEP_TYPE_RUN_COMMAND" => {
                let command = step
                    .pointer("/runCommand/commandLine")
                    .and_then(Value::as_str)
                    .or_else(|| {
                        step.pointer("/runCommand/proposedCommandLine")
                            .and_then(Value::as_str)
                    })
                    .unwrap_or("")
                    .to_string();
                if command.is_empty() {
                    continue;
                }
                let exit = step.pointer("/runCommand/exitCode").and_then(Value::as_i64);
                events.push(event("tool", command, exit.is_some_and(|code| code != 0)));
            }
            "CORTEX_STEP_TYPE_ERROR_MESSAGE" => {
                let content = jsonish(step.pointer("/errorMessage"));
                if content.is_empty() {
                    continue;
                }
                events.push(event("tool", content, true));
            }
            "CORTEX_STEP_TYPE_VIEW_FILE" => {
                // The wire key is lower-case "Uri" (daemon types.go tags it
                // `json:"absolutePathUri"`); the upper-case spelling is
                // tolerated as an alias for producers that copied this
                // parser's original (wrong-case) reading.
                let path = step
                    .pointer("/viewFile/absolutePathUri")
                    .or_else(|| step.pointer("/viewFile/absolutePathURI"))
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_string();
                if path.is_empty() {
                    continue;
                }
                let start = step.pointer("/viewFile/startLine").and_then(Value::as_i64);
                let end = step.pointer("/viewFile/endLine").and_then(Value::as_i64);
                let content = match (start, end) {
                    (Some(start), Some(end)) => format!("view {path} lines {start}-{end}"),
                    _ => format!("view {path}"),
                };
                events.push(event("tool", content, false));
            }
            // INVOKE_SUBAGENT children live in their own trajectory files;
            // SYSTEM_MESSAGE/CHECKPOINT/GENERIC carry no transcript content.
            _ => {}
        }
    }
    // rm-720: one meta event carries the folded usage totals and the
    // model id, so pricing resolves or discloses — never silently $0.
    if let Some(meta) = usage_fold.meta_event() {
        events.insert(0, meta);
    }
    non_empty(events)
}

fn parse_antigravity_jsonl(objs: &[JsonObject]) -> Option<Vec<Event>> {
    if !objs.iter().any(|entry| {
        matches!(
            string(entry.get("type")),
            Some("PLANNER_RESPONSE" | "USER_INPUT" | "CONVERSATION_HISTORY")
        ) && entry.contains_key("step_index")
            && entry.contains_key("created_at")
    }) {
        return None;
    }
    let mut usage_fold = AntigravityUsageFold::new();
    let mut events = Vec::new();
    for entry in objs.iter() {
        let typ = string(entry.get("type")).unwrap_or("");
        let timestamp = string(entry.get("created_at")).unwrap_or("").to_string();
        match typ {
            "USER_INPUT" => events.push(Event {
                role: "user".to_string(),
                content: string(entry.get("content")).unwrap_or("").to_string(),
                timestamp,
                source_tool: "antigravity_cli".to_string(),
                ..Event::default()
            }),
            "PLANNER_RESPONSE" => {
                let tool_calls = entry
                    .get("tool_calls")
                    .and_then(Value::as_array)
                    .into_iter()
                    .flatten()
                    .filter_map(|call| {
                        let call = call.as_object()?;
                        Some(ToolCall {
                            name: string(call.get("name")).unwrap_or("").to_string(),
                            args: jsonish(call.get("args")),
                            ..ToolCall::default()
                        })
                    })
                    .collect();
                if let Some(usage) = antigravity_usage_block(entry) {
                    usage_fold.fold(&usage, antigravity_model_id(entry), &timestamp);
                }
                events.push(Event {
                    role: "assistant".to_string(),
                    content: string(entry.get("content")).unwrap_or("").to_string(),
                    reasoning: string(entry.get("thinking")).unwrap_or("").to_string(),
                    timestamp,
                    tool_calls,
                    source_tool: "antigravity_cli".to_string(),
                    ..Event::default()
                });
            }
            "CONVERSATION_HISTORY" | "SYSTEM_MESSAGE" => {}
            _ => events.push(Event {
                role: "tool".to_string(),
                content: string(entry.get("content")).unwrap_or("").to_string(),
                timestamp,
                is_error: string(entry.get("status")).is_some_and(|status| status != "DONE"),
                source_tool: "antigravity_cli".to_string(),
                ..Event::default()
            }),
        }
    }
    // rm-720: same fold as the trajectory sidecar — one meta event
    // carries the summed generation totals and the model id.
    if let Some(meta) = usage_fold.meta_event() {
        events.insert(0, meta);
    }
    non_empty(events)
}

fn parse_cursor_transcript_jsonl(objs: &[JsonObject]) -> Option<Vec<Event>> {
    let mut entries = objs.iter().peekable();
    if entries.peek().is_none()
        || !entries.all(|entry| {
            entry.contains_key("role") && entry.get("message").and_then(Value::as_object).is_some()
        })
    {
        return None;
    }
    let mut events = Vec::new();
    for entry in objs.iter() {
        let role = string(entry.get("role")).unwrap_or("");
        let Some(message) = entry.get("message").and_then(Value::as_object) else {
            continue;
        };
        let mut content = Vec::new();
        let mut tool_calls = Vec::new();
        for block in message
            .get("content")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
        {
            let Some(block) = block.as_object() else {
                continue;
            };
            match string(block.get("type")).unwrap_or("") {
                "text" => content.push(string(block.get("text")).unwrap_or("").to_string()),
                "tool_use" => tool_calls.push(ToolCall {
                    id: string(block.get("id")).unwrap_or("").to_string(),
                    name: string(block.get("name")).unwrap_or("").to_string(),
                    args: jsonish(block.get("input")),
                }),
                _ => {}
            }
        }
        events.push(Event {
            role: role.to_string(),
            content: content.join("\n"),
            tool_calls,
            source_tool: "cursor".to_string(),
            ..Event::default()
        });
    }
    non_empty(events)
}

fn parse_claude_transcript_jsonl(objs: &[JsonObject]) -> Option<Vec<Event>> {
    if !objs.iter().any(|entry| {
        matches!(string(entry.get("type")), Some("tool_use" | "tool_result"))
            && entry.contains_key("timestamp")
            && entry.contains_key("tool_name")
    }) {
        return None;
    }
    let mut events = Vec::new();
    let mut flat_call_ordinals: BTreeMap<String, usize> = BTreeMap::new();
    let mut flat_result_ordinals: BTreeMap<String, usize> = BTreeMap::new();
    for entry in objs.iter() {
        let timestamp = string(entry.get("timestamp")).unwrap_or("").to_string();
        match string(entry.get("type")).unwrap_or("") {
            "user" | "assistant" => events.push(Event {
                role: string(entry.get("type")).unwrap_or("").to_string(),
                content: string(entry.get("content")).unwrap_or("").to_string(),
                timestamp,
                source_tool: "claude_code".to_string(),
                ..Event::default()
            }),
            // rm-230 (campaign-local rm-025): preserve the call/result
            // join key so diagnostics can pair each call with its result.
            // The flat format may carry the id explicitly (`tool_use_id` /
            // `id`); entries without one pair positionally PER TOOL (k-th
            // id-less Bash call <-> k-th id-less Bash result) so a present
            // result stops counting as `unmatched` and inflating the
            // review filter. Positional pairing is only used when no id
            // exists, and the ordinals are per-tool and count ID-LESS
            // entries only: explicit ids consume nothing from the
            // positional namespace (review F5: a shared count let an
            // out-of-order mixed stream mint different ordinals per side
            // and stay spuriously unmatched), and per-tool ordinals keep
            // an out-of-order result stream from pairing one tool's
            // result onto another tool's call (review R2-3: a global
            // ordinal attributed Bash's end-time to Read's latency and
            // let one tool's surplus result mask another tool's genuinely
            // missing result).
            "tool_use" => {
                let explicit = ["tool_use_id", "id", "callId", "toolCallId"]
                    .iter()
                    .find_map(|key| string(entry.get(*key)))
                    .unwrap_or("");
                let name = string(entry.get("tool_name")).unwrap_or("").to_string();
                let id = if explicit.is_empty() {
                    let ordinal = flat_call_ordinals.entry(name.clone()).or_insert(0);
                    let id = format!("flat-pair-{name}-{ordinal}");
                    *ordinal += 1;
                    id
                } else {
                    explicit.to_string()
                };
                events.push(Event {
                    role: "assistant".to_string(),
                    timestamp,
                    tool_calls: vec![ToolCall {
                        id,
                        name,
                        args: jsonish(entry.get("tool_input")),
                    }],
                    source_tool: "claude_code".to_string(),
                    ..Event::default()
                });
            }
            "tool_result" => {
                let explicit = ["tool_use_id", "id", "callId", "toolCallId"]
                    .iter()
                    .find_map(|key| string(entry.get(*key)))
                    .unwrap_or("");
                let name = string(entry.get("tool_name")).unwrap_or("").to_string();
                let tool_call_id = if explicit.is_empty() {
                    let ordinal = flat_result_ordinals.entry(name.clone()).or_insert(0);
                    let id = format!("flat-pair-{name}-{ordinal}");
                    *ordinal += 1;
                    id
                } else {
                    explicit.to_string()
                };
                events.push(Event {
                    role: "tool".to_string(),
                    content: jsonish(entry.get("tool_output")),
                    timestamp,
                    tool_call_id,
                    source_tool: "claude_code".to_string(),
                    ..Event::default()
                });
            }
            _ => {}
        }
    }
    non_empty(events)
}

fn parse_workbuddy_jsonl(objs: &[JsonObject]) -> Option<(Vec<Event>, ParseCounters)> {
    if !objs.iter().any(|entry| {
        matches!(
            string(entry.get("type")),
            Some("function_call" | "function_call_result" | "reasoning")
        ) && entry.contains_key("sessionId")
            && entry.contains_key("cwd")
    }) {
        return None;
    }
    let mut events = Vec::new();
    let mut model = "unknown".to_string();
    // Parse-time disclosure counters (kimi-style tuple channel,
    // upstream #316): clamp facts the aggregator cannot re-derive
    // once the clamped values have already been summed.
    let mut parse_counters: ParseCounters = Vec::new();
    // Usage accumulates across records (upstream #311): the previous
    // keep-last arm reported only the FINAL record's tokens, so a
    // 100+200+300-input session reported 300 instead of 600.
    let mut usage_sum: TokenUsage = BTreeMap::new();
    let mut has_usage = false;
    let mut clamped_records: i64 = 0;
    for entry in objs.iter() {
        if let Some(next) = entry
            .get("providerData")
            .and_then(Value::as_object)
            .and_then(|data| string(data.get("model")))
            .filter(|value| !value.is_empty())
        {
            model = next.to_string();
        }
        let timestamp = entry
            .get("timestamp")
            .and_then(number_as_i64)
            .map(timestamp_millis)
            .unwrap_or_default();
        let cwd = string(entry.get("cwd")).unwrap_or("").to_string();
        match string(entry.get("type")).unwrap_or("") {
            "message" => {
                let role = string(entry.get("role")).unwrap_or("");
                let content = workbuddy_content(entry.get("content"));
                if !content.is_empty() {
                    events.push(Event {
                        role: role.to_string(),
                        content,
                        timestamp,
                        cwd,
                        model_used: model.clone(),
                        source_tool: "workbuddy".to_string(),
                        ..Event::default()
                    });
                }
                if let Some((next, clamped)) = workbuddy_usage(entry) {
                    has_usage = true;
                    if clamped {
                        clamped_records += 1;
                    }
                    add_usage_into(&mut usage_sum, next);
                }
            }
            "reasoning" => {
                // Upstream #311 rider: reasoning records can carry a
                // usage block too — read it into the sum instead of
                // silently dropping the record's tokens.
                if let Some((next, clamped)) = workbuddy_usage(entry) {
                    has_usage = true;
                    if clamped {
                        clamped_records += 1;
                    }
                    add_usage_into(&mut usage_sum, next);
                }
                let reasoning = workbuddy_content(
                    entry
                        .get("content")
                        .filter(|value| value.as_array().is_some_and(|items| !items.is_empty()))
                        .or_else(|| entry.get("rawContent")),
                );
                if !reasoning.is_empty() {
                    events.push(Event {
                        role: "assistant".to_string(),
                        reasoning,
                        timestamp,
                        cwd,
                        model_used: model.clone(),
                        source_tool: "workbuddy".to_string(),
                        ..Event::default()
                    });
                }
            }
            "function_call" => {
                if let Some((next, clamped)) = workbuddy_usage(entry) {
                    has_usage = true;
                    if clamped {
                        clamped_records += 1;
                    }
                    add_usage_into(&mut usage_sum, next);
                }
                events.push(Event {
                    role: "assistant".to_string(),
                    timestamp,
                    cwd,
                    tool_calls: vec![ToolCall {
                        id: string(entry.get("callId")).unwrap_or("").to_string(),
                        name: string(entry.get("name")).unwrap_or("").to_string(),
                        args: jsonish(entry.get("arguments")),
                    }],
                    model_used: model.clone(),
                    source_tool: "workbuddy".to_string(),
                    ..Event::default()
                });
            }
            "function_call_result" => events.push(Event {
                role: "tool".to_string(),
                content: jsonish(entry.get("output")),
                timestamp,
                cwd,
                tool_call_id: string(entry.get("callId")).unwrap_or("").to_string(),
                is_error: string(entry.get("status")).is_some_and(|status| status != "completed"),
                model_used: model.clone(),
                source_tool: "workbuddy".to_string(),
                ..Event::default()
            }),
            _ => {}
        }
    }
    if has_usage {
        if clamped_records > 0 {
            // Rides the rm-450 disclosure family — the whole
            // workbuddy_input_basis:* set lands on
            // Metrics.disclosure_counters (the non-loss channel;
            // cache_subtracted / zeroed_suspected_mismatch mint there
            // via the aggregator's meta arm, and rm-538 routed this
            // parse-counter lane there too): a record reported more
            // cached tokens than input, so the cached count was
            // clamped to the source-recorded input — the session
            // total never exceeds what the source recorded. Surfaced
            // via the kimi-style parse-counter channel (see
            // parse_raw_session), not the event channel, so the
            // family lands on ONE map and never degrades
            // data_health.confidence.
            parse_counters.push((
                "workbuddy_input_basis:cache_clamped".to_string(),
                clamped_records,
            ));
        }
        events.insert(
            0,
            Event {
                role: "meta".to_string(),
                usage: usage_sum,
                model_used: model,
                source_tool: "workbuddy".to_string(),
                ..Event::default()
            },
        );
    }
    non_empty(events).map(|events| (events, parse_counters))
}

fn workbuddy_content(value: Option<&Value>) -> String {
    value
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|item| item.get("text").and_then(Value::as_str))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Saturating per-key usage accumulation (upstream #311): workbuddy
/// journals carry one usage block per assistant record and the session
/// total is their SUM — the previous keep-last arm reported only the
/// final record's tokens (a 100+200+300-input session reported 300).
fn add_usage_into(total: &mut TokenUsage, next: TokenUsage) {
    for (key, value) in next {
        let slot = total.entry(key).or_insert(0);
        *slot = slot.saturating_add(value);
    }
}

/// Shared cache-subtraction for usage maps whose source reports
/// `input_tokens` INCLUSIVE of cached tokens — workbuddy records and
/// GitHub Copilot modelMetrics/spans both follow the GenAI semconv
/// shape where `gen_ai.usage.input_tokens` includes cached tokens.
/// Converts the map to agenttrace's delta basis (`tokens_input` and
/// `tokens_cache_r` are priced as disjoint buckets). Upstream #316
/// shape: a record may report MORE cached tokens than input
/// (adversarial or resumed ledgers), so each cache count is clamped to
/// what is left of the input — the session total can never exceed the
/// source-recorded input, and a ledger whose cached part exceeds its
/// input neither goes negative nor double-charges. Returns `true`
/// when a clamp fired, for the parser's disclosure counters.
/// `remaining` starts at `input.max(0)`, so adversarial magnitudes
/// that clamp `input_tokens` to i64::MIN leave every cache count at 0
/// instead of underflowing; a negative cached count is hostile data,
/// is zeroed, and contributes nothing (no clamp flag).
fn subtract_cached_input(usage: &mut TokenUsage, cache_keys: &[&str]) -> bool {
    let Some(input) = usage.get("input_tokens").copied() else {
        return false;
    };
    let mut remaining = input.max(0);
    let mut clamped = false;
    for key in cache_keys {
        if let Some(value) = usage.get_mut(*key) {
            if *value > remaining {
                clamped = true;
                *value = remaining;
            } else if *value < 0 {
                *value = 0;
            }
            remaining -= *value;
        }
    }
    usage.insert("input_tokens".to_string(), remaining);
    clamped
}

fn workbuddy_usage(entry: &Map<String, Value>) -> Option<(TokenUsage, bool)> {
    let mut usage = entry
        .get("message")
        .and_then(|message| message.get("usage"))
        .or_else(|| entry.get("providerData").and_then(|data| data.get("usage")))
        .and_then(usage_from_value)?;
    // WorkBuddy only reports cache reads, so the single-key form of the
    // shared clamp (rm-529's parameterized port); the Copilot arms pass
    // both cache keys.
    let clamped = subtract_cached_input(&mut usage, &["cache_read_input_tokens"]);
    Some((usage, clamped))
}

fn parse_openclaw_value(value: &Value) -> Option<Vec<Event>> {
    let doc = value.as_object()?;
    if string(doc.get("provider")) != Some("openclaw") {
        return None;
    }
    parse_anthropic_message_wrapper(doc, "openclaw")
}

fn parse_hermes_json_value(value: &Value) -> Option<Vec<Event>> {
    let doc = value.as_object()?;
    if !doc.contains_key("messages")
        || !doc.contains_key("model")
        || !doc.contains_key("session_id")
        || doc.contains_key("provider")
    {
        return None;
    }
    if doc.contains_key("usage") && !doc.contains_key("platform") {
        return None;
    }
    let messages = doc.get("messages").and_then(Value::as_array)?;
    let model = string(doc.get("model")).unwrap_or("").to_string();
    let session_start = string(doc.get("session_start")).unwrap_or("").to_string();
    let session_end = string(doc.get("last_updated")).unwrap_or("").to_string();
    let mut events = Vec::new();

    if let Some(usage) = doc.get("usage").and_then(usage_from_value) {
        events.push(Event {
            role: "meta".to_string(),
            usage,
            model_used: model.clone(),
            source_tool: "hermes_json".to_string(),
            ..Event::default()
        });
    }

    for message in messages {
        let Some(message) = message.as_object() else {
            continue;
        };
        let role = string(message.get("role")).unwrap_or("");
        if role == "tool" {
            continue;
        }
        let mut tool_calls = Vec::new();
        if let Some(calls) = message.get("tool_calls").and_then(Value::as_array) {
            for call in calls {
                let Some(call) = call.as_object() else {
                    continue;
                };
                let function = call.get("function").and_then(Value::as_object);
                tool_calls.push(ToolCall {
                    id: string(call.get("id")).unwrap_or("").to_string(),
                    name: function
                        .and_then(|function| string(function.get("name")))
                        .unwrap_or("")
                        .to_string(),
                    args: jsonish(function.and_then(|function| function.get("arguments"))),
                });
            }
        }
        events.push(Event {
            role: role.to_string(),
            content: string(message.get("content")).unwrap_or("").to_string(),
            timestamp: string(message.get("timestamp")).unwrap_or("").to_string(),
            reasoning: string(message.get("reasoning"))
                .or_else(|| string(message.get("reasoning_content")))
                .unwrap_or("")
                .to_string(),
            redacted: boolish(message.get("redacted")),
            tool_calls,
            model_used: model.clone(),
            source_tool: "hermes_json".to_string(),
            ..Event::default()
        });
    }

    for message in messages {
        let Some(message) = message.as_object() else {
            continue;
        };
        if string(message.get("role")) != Some("tool") {
            continue;
        }
        events.push(Event {
            role: "tool".to_string(),
            content: string(message.get("content")).unwrap_or("").to_string(),
            timestamp: string(message.get("timestamp")).unwrap_or("").to_string(),
            tool_call_id: string(message.get("tool_call_id"))
                .unwrap_or("")
                .to_string(),
            is_error: boolish(message.get("is_error")),
            source_tool: "hermes_json".to_string(),
            ..Event::default()
        });
    }

    apply_hermes_session_timestamps(&mut events, &session_start, &session_end);
    non_empty(events)
}

fn apply_hermes_session_timestamps(events: &mut [Event], session_start: &str, session_end: &str) {
    if session_start.is_empty() && session_end.is_empty() {
        return;
    }
    if events.iter().any(|event| !event.timestamp.is_empty()) {
        return;
    }
    if !session_start.is_empty() {
        if let Some(first) = events
            .iter_mut()
            .find(|event| event.role != "meta" && event.role != "session_meta")
        {
            first.timestamp = session_start.to_string();
        }
    }
    if !session_end.is_empty() {
        if let Some(last) = events
            .iter_mut()
            .rev()
            .find(|event| event.role != "meta" && event.role != "session_meta")
        {
            last.timestamp = session_end.to_string();
        }
    }
}

fn parse_anthropic_message_wrapper(
    doc: &Map<String, Value>,
    source_tool: &str,
) -> Option<Vec<Event>> {
    let target = doc.get("session").and_then(Value::as_object).unwrap_or(doc);
    let messages = target.get("messages").and_then(Value::as_array)?;
    let model = string(target.get("model"))
        .or_else(|| string(doc.get("model")))
        .unwrap_or("unknown")
        .to_string();
    let mut events = Vec::new();
    if let Some(usage) = target.get("usage").and_then(usage_from_value) {
        events.push(Event {
            role: "meta".to_string(),
            usage,
            model_used: model.clone(),
            source_tool: source_tool.to_string(),
            ..Event::default()
        });
    } else if model != "unknown" {
        events.push(Event {
            role: "meta".to_string(),
            model_used: model.clone(),
            source_tool: source_tool.to_string(),
            ..Event::default()
        });
    }

    for message in messages {
        let Some(message) = message.as_object() else {
            continue;
        };
        anthropic_wrapper_message_events(message, &model, source_tool, &mut events);
    }
    non_empty(events)
}

fn anthropic_wrapper_message_events(
    message: &Map<String, Value>,
    model: &str,
    source_tool: &str,
    events: &mut Vec<Event>,
) {
    let role = string(message.get("role")).unwrap_or("");
    let ts = string(message.get("timestamp")).unwrap_or("").to_string();
    if role == "tool" {
        events.push(Event {
            role: "tool".to_string(),
            content: tool_result_content(message),
            timestamp: ts,
            tool_call_id: string(message.get("tool_call_id"))
                .unwrap_or("")
                .to_string(),
            is_error: boolish(message.get("is_error")),
            source_tool: source_tool.to_string(),
            ..Event::default()
        });
        return;
    }

    match message.get("content") {
        Some(Value::String(text)) => events.push(Event {
            role: role.to_string(),
            content: text.to_string(),
            timestamp: ts.clone(),
            model_used: model.to_string(),
            source_tool: source_tool.to_string(),
            ..Event::default()
        }),
        Some(Value::Array(blocks)) => {
            for block in blocks {
                let Some(block) = block.as_object() else {
                    continue;
                };
                match string(block.get("type")).unwrap_or("") {
                    "text" => events.push(Event {
                        role: role.to_string(),
                        content: string(block.get("text")).unwrap_or("").to_string(),
                        timestamp: ts.clone(),
                        model_used: model.to_string(),
                        source_tool: source_tool.to_string(),
                        ..Event::default()
                    }),
                    "thinking" => events.push(Event {
                        role: "assistant".to_string(),
                        reasoning: string(block.get("thinking")).unwrap_or("").to_string(),
                        redacted: boolish(block.get("redacted")),
                        timestamp: ts.clone(),
                        model_used: model.to_string(),
                        source_tool: source_tool.to_string(),
                        ..Event::default()
                    }),
                    "tool_use" => events.push(Event {
                        role: "assistant".to_string(),
                        timestamp: ts.clone(),
                        tool_calls: vec![ToolCall {
                            id: string(block.get("id")).unwrap_or("").to_string(),
                            name: string(block.get("name")).unwrap_or("").to_string(),
                            args: jsonish(block.get("input").or_else(|| block.get("arguments"))),
                        }],
                        model_used: model.to_string(),
                        source_tool: source_tool.to_string(),
                        ..Event::default()
                    }),
                    "tool_result" => events.push(Event {
                        role: "tool".to_string(),
                        timestamp: ts.clone(),
                        tool_call_id: string(block.get("tool_use_id")).unwrap_or("").to_string(),
                        content: tool_result_content(block),
                        is_error: boolish(block.get("is_error")),
                        source_tool: source_tool.to_string(),
                        ..Event::default()
                    }),
                    _ => {}
                }
            }
        }
        _ => {}
    }

    if let Some(calls) = message.get("tool_calls").and_then(Value::as_array) {
        let tool_calls = calls
            .iter()
            .filter_map(|call| {
                let call = call.as_object()?;
                let function = call.get("function").and_then(Value::as_object);
                Some(ToolCall {
                    id: string(call.get("id")).unwrap_or("").to_string(),
                    name: function
                        .and_then(|function| string(function.get("name")))
                        .unwrap_or("")
                        .to_string(),
                    args: jsonish(function.and_then(|function| function.get("arguments"))),
                })
            })
            .collect::<Vec<_>>();
        events.push(Event {
            role: role.to_string(),
            timestamp: ts,
            tool_calls,
            model_used: model.to_string(),
            source_tool: source_tool.to_string(),
            ..Event::default()
        });
    }
}

fn is_aider_history(path: &str, trimmed: &str) -> bool {
    if Path::new(path).file_name().and_then(|name| name.to_str()) == Some(".aider.chat.history.md")
    {
        return true;
    }
    if trimmed.starts_with('{') || trimmed.starts_with('[') {
        return false;
    }
    trimmed.contains("# aider chat started at") && trimmed.contains("#### ")
}

fn parse_aider_chat_history(raw: &str) -> anyhow::Result<Vec<Event>> {
    let mut events = Vec::new();
    let mut role = String::new();
    let mut lines: Vec<String> = Vec::new();
    let mut start_ts = String::new();
    let mut model = "unknown".to_string();
    let mut usage = BTreeMap::new();

    for raw_line in raw.lines() {
        let line = raw_line.trim_end_matches('\r');
        if let Some(value) = line.strip_prefix("# aider chat started at ") {
            flush_aider_event(&mut events, &mut role, &mut lines, &start_ts, &model);
            start_ts = aider_time(value);
            continue;
        }
        if let Some(text) = line.strip_prefix("#### ") {
            if role != "user" {
                flush_aider_event(&mut events, &mut role, &mut lines, &start_ts, &model);
                role = "user".to_string();
            }
            lines.push(text.trim().to_string());
            continue;
        }
        if let Some(text) = line.strip_prefix('>') {
            flush_aider_event(&mut events, &mut role, &mut lines, &start_ts, &model);
            let text = text.trim();
            if text.is_empty() {
                continue;
            }
            if let Some(inferred) = infer_aider_model(text) {
                model = inferred;
            }
            merge_aider_usage(&mut usage, text);
            continue;
        }
        if line.trim().is_empty() {
            if !role.is_empty() && !lines.is_empty() {
                lines.push(String::new());
            }
            continue;
        }
        if role != "assistant" {
            flush_aider_event(&mut events, &mut role, &mut lines, &start_ts, &model);
            role = "assistant".to_string();
        }
        lines.push(line.to_string());
    }
    flush_aider_event(&mut events, &mut role, &mut lines, &start_ts, &model);

    if !usage.is_empty() || model != "unknown" || !start_ts.is_empty() {
        let meta = Event {
            role: "meta".to_string(),
            timestamp: start_ts,
            model_used: model,
            source_tool: "aider".to_string(),
            usage,
            ..Event::default()
        };
        events.insert(0, meta);
    }
    if events.is_empty() {
        bail!("aider chat history: no parseable events");
    }
    Ok(events)
}

fn flush_aider_event(
    events: &mut Vec<Event>,
    role: &mut String,
    lines: &mut Vec<String>,
    start_ts: &str,
    model: &str,
) {
    if role.is_empty() {
        lines.clear();
        return;
    }
    let content = lines.join("\n").trim().to_string();
    if !content.is_empty() {
        events.push(Event {
            role: role.clone(),
            content,
            timestamp: start_ts.to_string(),
            model_used: model.to_string(),
            source_tool: "aider".to_string(),
            ..Event::default()
        });
    }
    role.clear();
    lines.clear();
}

fn aider_time(value: &str) -> String {
    chrono::NaiveDateTime::parse_from_str(value, "%Y-%m-%d %H:%M:%S")
        .ok()
        .and_then(|ts| ts.and_local_timezone(chrono::Local).single())
        .map(|ts| ts.to_rfc3339_opts(chrono::SecondsFormat::Secs, false))
        .unwrap_or_default()
}

fn infer_aider_model(text: &str) -> Option<String> {
    let parts = text.split_whitespace().collect::<Vec<_>>();
    for pair in parts.windows(2) {
        if pair[0] == "--model" || pair[0] == "-m" {
            return Some(trim_aider_model(pair[1]));
        }
    }
    let lower = text.to_ascii_lowercase();
    for marker in ["model:", "model="] {
        let Some(start) = lower.find(marker) else {
            continue;
        };
        let value = text[start + marker.len()..]
            .split_whitespace()
            .next()
            .unwrap_or("");
        if !value.is_empty() {
            return Some(trim_aider_model(value));
        }
    }
    None
}

fn trim_aider_model(value: &str) -> String {
    value
        .trim_matches(|ch| matches!(ch, '`' | '\'' | '"'))
        .to_string()
}

fn merge_aider_usage(usage: &mut BTreeMap<String, i64>, text: &str) {
    let lower = text.to_ascii_lowercase();
    let Some(tokens_pos) = lower.find("tokens:") else {
        return;
    };
    let rest = &text[tokens_pos + "tokens:".len()..];
    let Some(sent_pos) = rest.to_ascii_lowercase().find(" sent") else {
        return;
    };
    let input = parse_aider_token_count(&rest[..sent_pos]);
    let after_sent = rest[sent_pos + " sent".len()..].trim_start();
    let lower_after = after_sent.to_ascii_lowercase();
    let Some(received_pos) = lower_after.rfind("received") else {
        return;
    };
    let before_received = after_sent[..received_pos].trim();
    let output_part = before_received
        .rsplit(',')
        .next()
        .unwrap_or(before_received);
    let output = parse_aider_token_count(output_part);
    let cache_write = aider_optional_token(before_received, "cache write");
    let cache_hit = aider_optional_token(before_received, "cache hit");

    usage.insert("input_tokens".to_string(), input);
    usage.insert("cache_creation_input_tokens".to_string(), cache_write);
    usage.insert("cache_read_input_tokens".to_string(), cache_hit);
    usage.insert("output_tokens".to_string(), output);
}

fn aider_optional_token(text: &str, label: &str) -> i64 {
    let lower = text.to_ascii_lowercase();
    let Some(label_pos) = lower.find(label) else {
        return 0;
    };
    let prefix = &text[..label_pos];
    parse_aider_token_count(prefix.rsplit(',').next().unwrap_or(prefix))
}

fn parse_aider_token_count(value: &str) -> i64 {
    let mut value = value
        .trim()
        .trim_matches(',')
        .replace(',', "")
        .to_ascii_lowercase();
    if value.is_empty() {
        return 0;
    }
    let multiplier = if value.ends_with('k') {
        value.pop();
        1000.0
    } else {
        1.0
    };
    value
        .trim()
        .parse::<f64>()
        .map(|number| (number * multiplier) as i64)
        .unwrap_or(0)
}

fn is_oh_my_pi_jsonl(objs: &[JsonObject]) -> bool {
    objs.iter().any(is_oh_my_pi_session_header)
}

fn is_oh_my_pi_session_header(obj: &Map<String, Value>) -> bool {
    string(obj.get("type")) == Some("session")
        && (obj.contains_key("version")
            || obj.contains_key("cwd")
            || obj.contains_key("titleSource")
            || obj.contains_key("parentSession"))
}

fn parse_oh_my_pi_jsonl(path: &str, objs: &[JsonObject]) -> anyhow::Result<Vec<Event>> {
    let source_tool = pi_source_for_path(path);
    let mut meta_events = Vec::new();
    let mut events = Vec::new();
    let mut model = "unknown".to_string();
    let mut seen_header = false;
    // rm-438 (integration of run 6403d975): whether a model_change
    // entry actually moved the tracked model, and where the header
    // meta event lives. session_from_events derives the session model
    // from the FIRST model-bearing event, and a pre-switch
    // usage-bearing message's session_meta event is scanned before the
    // model_change meta — without the stamp below, a switched session
    // whose post-switch turns carry no usage keeps reporting (and
    // pricing at) the pre-switch model, exactly the silent
    // misattribution rm-438 exists to fix.
    let mut model_change_applied = false;
    let mut header_meta_index: Option<usize> = None;
    // rm-436/rm-437: parse-time disclosure counters. Attached to the
    // header meta event (or a dedicated meta event when no header
    // exists) so documented-but-uncounted journal facts surface in
    // data_health and --doctor instead of vanishing into the parser.
    let mut counters: BTreeMap<String, i64> = BTreeMap::new();
    // rm-437 (first cut): v2/v3 journals identify every entry (`id`)
    // and its parent (`parentId`), so the tree shape is knowable.
    // Count branch ends — leaf ids that no other entry references as
    // its parentId. A linear session has exactly one; a journal with
    // sibling branches has more. Counted over BODY entries only
    // (everything after the first `type:"session"` header, mirroring
    // the loop below): the header's own id is a root, never a branch
    // end, and pre-header junk lines must not count either. This
    // first cut still counts ALL branches in the spend and only
    // discloses the shape; active-branch replay is deliberately out
    // of scope.
    let body_start = objs
        .iter()
        .position(|obj| string(obj.get("type")) == Some("session"))
        .map_or(0, |position| position + 1);
    let referenced_parents: BTreeSet<&str> = objs[body_start..]
        .iter()
        .filter_map(|obj| string(obj.get("parentId")).filter(|value| !value.is_empty()))
        .collect();
    let branch_ends = objs[body_start..]
        .iter()
        .filter(|obj| {
            string(obj.get("id"))
                .is_some_and(|id| !id.is_empty() && !referenced_parents.contains(id))
        })
        .count();
    if branch_ends > 1 {
        counters.insert("pi_branches".to_string(), branch_ends as i64);
    }

    for obj in objs.iter() {
        let typ = string(obj.get("type")).unwrap_or("");
        if !seen_header {
            // Older/newer Oh My Pi versions prepend non-session lines (e.g.
            // {"type":"title",...}) before the session header; the sniffing
            // function `is_oh_my_pi_jsonl` already accepts them via `.any()`,
            // so skip anything until the first `type == "session"` object
            // instead of bailing. A file with no session header at all still
            // fails with the original error.
            if typ != "session" {
                continue;
            }
            if string(obj.get("id")).unwrap_or("").is_empty() {
                bail!("oh_my_pi: invalid session header");
            }
            seen_header = true;
            if let Some(cwd) = string(obj.get("cwd")).filter(|value| !value.is_empty()) {
                header_meta_index = Some(meta_events.len());
                meta_events.push(Event {
                    role: "meta".to_string(),
                    cwd: cwd.to_string(),
                    source_tool: source_tool.clone(),
                    ..Event::default()
                });
            }
            continue;
        }

        let ts = string(obj.get("timestamp")).unwrap_or("");
        match typ {
            "message" => {
                let Some(message) = obj.get("message").and_then(Value::as_object) else {
                    continue;
                };
                for event in
                    oh_my_pi_message_events(message, ts, &mut model, &source_tool, &mut counters)
                {
                    if event.role == "meta" {
                        meta_events.push(event);
                    } else {
                        events.push(event);
                    }
                }
            }
            "custom_message" => {
                let (content, _, _, _) = oh_my_pi_content(obj.get("content"));
                if !content.trim().is_empty() {
                    events.push(Event {
                        role: "user".to_string(),
                        content,
                        timestamp: ts.to_string(),
                        model_used: model.clone(),
                        source_tool: source_tool.clone(),
                        ..Event::default()
                    });
                }
            }
            "model_change" => {
                // rm-438: the wire key is `modelId` — verified against
                // the @earendil-works/pi-coding-agent 1.0.2 dist
                // (session-manager.js appendModelChange(provider,
                // modelId)) and the versioned session-format spec.
                // Reading `model` alone left this handler dead on real
                // journals, so every model-less assistant message and
                // usage block after a switch kept the previous model
                // for attribution and pricing. `model` stays as a
                // legacy fallback for pre-rename writers; the provider
                // rides along in the content for context.
                let next_model = string(obj.get("modelId"))
                    .or_else(|| string(obj.get("model")))
                    .filter(|value| !value.is_empty());
                if let Some(next_model) = next_model {
                    let provider = string(obj.get("provider")).unwrap_or("");
                    model_change_applied = true;
                    model = next_model.to_string();
                    meta_events.push(Event {
                        role: "meta".to_string(),
                        content: if provider.is_empty() {
                            format!("model change: {model}")
                        } else {
                            format!("model change: {provider} → {model}")
                        },
                        timestamp: ts.to_string(),
                        model_used: model.clone(),
                        source_tool: source_tool.clone(),
                        ..Event::default()
                    });
                }
            }
            "branch_summary" | "compaction" => {
                let content = string(obj.get("summary")).unwrap_or("").trim().to_string();
                if !content.is_empty() {
                    events.push(Event {
                        role: "assistant".to_string(),
                        content,
                        timestamp: ts.to_string(),
                        model_used: model.clone(),
                        source_tool: source_tool.clone(),
                        ..Event::default()
                    });
                }
            }
            // rm-436: standalone usage entries. The wire shape (spec +
            // 1.0.2 dist appendUsage(kind, provider, model, usage))
            // carries its own provider/model and a usage block with
            // cacheRead/cacheWrite plus an upstream-recorded cost. The
            // old catch-all dropped them whole: cache_warm spend was
            // invisible (PoC: 150 tokens reported where the spec-true
            // total was 50,150 + $0.015). Counted here, attributed to
            // the entry's own model (session-tracked model as
            // fallback), per-kind disclosed; an unknown kind counts as
            // normal usage under its own kind string.
            "usage" => {
                let kind = string(obj.get("kind"))
                    .filter(|value| !value.is_empty())
                    .unwrap_or("unknown");
                *counters
                    .entry(disclosure_key("pi_usage_entry", kind))
                    .or_insert(0) += 1;
                let entry_model = string(obj.get("model"))
                    .filter(|value| !value.is_empty())
                    .map(str::to_string)
                    .unwrap_or_else(|| model.clone());
                let usage = oh_my_pi_usage(obj.get("usage"));
                let recorded_cost_usd = oh_my_pi_recorded_cost_usd(obj.get("usage"));
                // F-C (review 06e542d5): an entry whose usage block
                // carries no token class at all can still record a
                // real upstream cost — emit the meta event whenever
                // EITHER signal exists, so a PRESENT cost is never
                // silently zeroed for lacking tokens.
                if usage.is_some() || recorded_cost_usd.is_some() {
                    meta_events.push(Event {
                        role: "meta".to_string(),
                        timestamp: ts.to_string(),
                        usage: usage.unwrap_or_default(),
                        model_used: entry_model,
                        recorded_cost_usd,
                        source_tool: source_tool.clone(),
                        ..Event::default()
                    });
                }
            }
            other => {
                // rm-436 counter family (rm-437 evidence): every other
                // documented-but-uncounted entry type stays visible as
                // `pi_entry_skipped:<type>` instead of silently
                // dropping on the floor (v3 `custom`, `label`,
                // `session_info`, `thinking_level_change`,
                // `context_edit`, ...). A line with NO type at all has
                // nothing to name and stays uncounted (mirrors the
                // role arm's empty guard).
                if !other.is_empty() {
                    *counters
                        .entry(disclosure_key("pi_entry_skipped", other))
                        .or_insert(0) += 1;
                }
            }
        }
    }

    if !seen_header {
        bail!("oh_my_pi: missing session header");
    }
    if events.is_empty() {
        bail!("oh_my_pi: no parseable events");
    }
    meta_events.extend(events);
    if model_change_applied {
        // The header meta event is the session-level carrier: carrying
        // the FINAL tracked model keeps `session_from_events`'s
        // first-model-wins scan truthful for switched journals without
        // touching the scan (other formats keep their behavior). The
        // model_change meta events stay in place for disclosure.
        match header_meta_index {
            Some(index) => {
                meta_events[index].model_used = model.clone();
            }
            None => meta_events.insert(
                0,
                Event {
                    role: "meta".to_string(),
                    model_used: model.clone(),
                    source_tool: source_tool.clone(),
                    ..Event::default()
                },
            ),
        }
    }
    if !counters.is_empty() {
        // One carrier event, never one counter per entry: attach to
        // the header meta event when it exists (the only meta event
        // with a cwd) so events_total does not grow with disclosures.
        // A journal whose header carries no cwd has no such event —
        // synthesize one carrier instead of panicking on a lookup that
        // can never hit.
        let carrier = match meta_events
            .iter_mut()
            .find(|event| event.role == "meta" && !event.cwd.is_empty())
        {
            Some(carrier) => carrier,
            None => {
                meta_events.push(Event {
                    role: "meta".to_string(),
                    source_tool: source_tool.clone(),
                    ..Event::default()
                });
                meta_events.last_mut().expect("carrier event just pushed")
            }
        };
        carrier.disclosure_counters = counters;
    }
    Ok(meta_events)
}

/// Source label for a pi-family transcript, by the home root that
/// actually owns it (rm-084): `~/.pi` (any agent dir, including
/// relocated agent state) and the XDG pi root are `pi`; the fork roots
/// keep their own identities (`pi_senpi`, `pi_omo`, `oh_my_pi`) so a
/// fork corpus is never attributed to a different agent. A pi-family
/// transcript outside every known root (the `-d` escape for an agent
/// dir relocated elsewhere) keeps the historical family fallback.
fn pi_source_for_path(path: &str) -> String {
    let normalized = path.replace('\\', "/").to_ascii_lowercase();
    if normalized.contains("/.senpi/") {
        "pi_senpi".to_string()
    } else if normalized.contains("/.omo/") {
        "pi_omo".to_string()
    } else if normalized.contains("/.omp/") {
        "oh_my_pi".to_string()
    } else if normalized.contains("/.pi/") || normalized.contains("/.config/pi/") {
        "pi".to_string()
    } else {
        "oh_my_pi".to_string()
    }
}

/// pi `type:"usage"` entries carry a recorded cost block
/// (`usage.cost.total`, USD). Returned only when finite and
/// non-negative; anything else is treated as absent (rm-436): a
/// missing or corrupt cost must not silently zero real spend.
fn oh_my_pi_recorded_cost_usd(usage: Option<&Value>) -> Option<f64> {
    let total = usage?
        .as_object()?
        .get("cost")?
        .as_object()?
        .get("total")?
        .as_f64()?;
    (total.is_finite() && total >= 0.0).then_some(total)
}

/// rm-436 counter-family keys embed journal-authored strings (usage
/// `kind`, entry `type`, message `role`). The counters are facts, not
/// conversation content — but the keys still reach every report
/// surface (doctor text, overview text/markdown/HTML, JSON, the
/// session cache), so the shared control-character sanitizer applies
/// at MINT time (cycle-7 review F1 / review 06e542d5 F-A): control
/// bytes (C0/C1/DEL — exactly `char::is_control`) become U+FFFD
/// here, once, and every consumer is covered — a hostile `kind`
/// carries neither an ESC/OSC terminal-injection sequence nor a
/// newline that breaks the one-line text contract. Printable tails
/// legitimately survive; the contract targets control bytes only.
///
/// rm-594: keys are ALSO length-bounded at mint. A hostile or evolved
/// journal can smuggle megabyte-scale strings in as `type` names and
/// unknown usage-key names (PoC p1-huge: a 1,048,617-char response_item
/// type minted a 1,060,176-byte cache entry and 1,054,082-byte
/// reports). Values longer than [`DISCLOSURE_VALUE_CAP`] chars keep a
/// visible prefix plus a 12-hex FNV-1a digest of the full sanitized
/// value: distinct long keys stay distinct, the shape is deterministic
/// across runs and Rust versions (no SipHash-version drift), and benign
/// short values are byte-identical to the uncapped form — no
/// persisted-shape change, no session-cache schema bump. Every mint
/// site (pi families, codex unmatched types, the hermes unknown-usage
/// disclosure) routes through this ONE helper so future arms inherit
/// both guarantees for free.
pub(crate) fn disclosure_key(prefix: &str, value: &str) -> String {
    format!("{prefix}:{}", capped_disclosure_value(value))
}

/// rm-594: maximum CHARS (not bytes — U+FFFD is 3 bytes) a journal-derived
/// counter value may occupy before the prefix+digest form takes over.
const DISCLOSURE_VALUE_CAP: usize = 96;
/// rm-594: visible prefix retained before the digest, so a human reading
/// the report still sees what the journal actually said.
const DISCLOSURE_VALUE_PREFIX: usize = 48;

/// FNV-1a 64-bit, low 48 bits rendered as EXACTLY 12 hex chars —
/// deterministic across runs and toolchain versions, unlike
/// `DefaultHasher` (rm-594 cap helper; review bd6e4a50 F2: the unmasked
/// u64 rendered 12–16 digits, contradicting every comment and doc that
/// said "12 hex" — mask, don't reword). 48 bits keeps distinct long
/// keys distinct for any realistic hostile corpus.
fn fnv1a_hex(value: &str) -> String {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in value.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    format!("{:012x}", hash & 0xffff_ffff_ffff)
}

/// rm-594: sanitize first (control bytes die), then cap by chars (no
/// partial multi-byte truncation), keeping a prefix + digest when long.
/// Also used by the RENDER choke points (reports.rs counts_cell,
/// doctor.rs disclosures) so legacy cache entries minted before the cap
/// (or any future bypass of the shared mint helper) still render bounded
/// — sanitize is idempotent (U+FFFD is not control) and keys under the
/// cap pass through byte-identical.
pub(crate) fn capped_disclosure_value(value: &str) -> String {
    let sanitized = crate::statusline::sanitize_line_segment(value);
    if sanitized.chars().count() <= DISCLOSURE_VALUE_CAP {
        return sanitized;
    }
    let prefix: String = sanitized.chars().take(DISCLOSURE_VALUE_PREFIX).collect();
    format!("{prefix}…#{}", fnv1a_hex(&sanitized))
}

fn oh_my_pi_message_events(
    message: &Map<String, Value>,
    entry_ts: &str,
    model: &mut String,
    source_tool: &str,
    counters: &mut BTreeMap<String, i64>,
) -> Vec<Event> {
    let role = string(message.get("role")).unwrap_or("");
    let ts = oh_my_pi_timestamp(message.get("timestamp"), entry_ts);
    if let Some(next_model) = string(message.get("model")).filter(|value| !value.is_empty()) {
        *model = next_model.to_string();
    }

    let mut events = Vec::new();
    if let Some(usage) = oh_my_pi_usage(message.get("usage")) {
        events.push(Event {
            role: "meta".to_string(),
            timestamp: ts.clone(),
            usage,
            model_used: model.clone(),
            source_tool: source_tool.to_string(),
            ..Event::default()
        });
    }

    let (content, reasoning, redacted, tool_calls) = oh_my_pi_content(message.get("content"));
    match role {
        "user" => {
            if !content.is_empty() {
                events.push(Event {
                    role: "user".to_string(),
                    content,
                    timestamp: ts,
                    model_used: model.clone(),
                    source_tool: source_tool.to_string(),
                    ..Event::default()
                });
            }
        }
        "developer" => {
            if !content.is_empty() {
                events.push(Event {
                    role: "system".to_string(),
                    content,
                    timestamp: ts,
                    model_used: model.clone(),
                    source_tool: source_tool.to_string(),
                    ..Event::default()
                });
            }
        }
        "assistant" => {
            if !content.is_empty() || !reasoning.is_empty() || !tool_calls.is_empty() {
                events.push(Event {
                    role: "assistant".to_string(),
                    content,
                    reasoning,
                    redacted,
                    tool_calls,
                    timestamp: ts,
                    model_used: model.clone(),
                    source_tool: source_tool.to_string(),
                    ..Event::default()
                });
            }
        }
        "toolResult" => events.push(Event {
            role: "tool".to_string(),
            content,
            timestamp: ts,
            tool_call_id: string(message.get("toolCallId")).unwrap_or("").to_string(),
            is_error: boolish(message.get("isError")),
            model_used: model.clone(),
            source_tool: source_tool.to_string(),
            ..Event::default()
        }),
        other => {
            // rm-436 counter family: message roles without an accounting
            // arm (v3 additions, fork extensions) stay visible as
            // `pi_message_role:<role>` instead of silently vanishing.
            if !other.is_empty() {
                *counters
                    .entry(disclosure_key("pi_message_role", other))
                    .or_insert(0) += 1;
            }
        }
    }
    events
}

fn oh_my_pi_content(raw: Option<&Value>) -> (String, String, bool, Vec<ToolCall>) {
    match raw {
        Some(Value::String(text)) => (text.to_string(), String::new(), false, Vec::new()),
        Some(Value::Array(blocks)) => {
            let mut text_parts = Vec::new();
            let mut reasoning_parts = Vec::new();
            let mut tool_calls = Vec::new();
            let mut redacted = false;
            for block in blocks {
                let Some(block) = block.as_object() else {
                    continue;
                };
                match string(block.get("type")).unwrap_or("") {
                    "text" => {
                        if let Some(text) =
                            string(block.get("text")).filter(|value| !value.is_empty())
                        {
                            text_parts.push(text.to_string());
                        }
                    }
                    "thinking" => {
                        if let Some(thinking) =
                            string(block.get("thinking")).filter(|value| !value.is_empty())
                        {
                            reasoning_parts.push(thinking.to_string());
                        }
                    }
                    "redactedThinking" => {
                        redacted = true;
                        if let Some(data) =
                            string(block.get("data")).filter(|value| !value.is_empty())
                        {
                            reasoning_parts.push(data.to_string());
                        }
                    }
                    "toolCall" => {
                        let id = string(block.get("id")).unwrap_or("").to_string();
                        let name = string(block.get("name")).unwrap_or("").to_string();
                        if !id.is_empty() || !name.is_empty() {
                            tool_calls.push(ToolCall {
                                id,
                                name,
                                args: jsonish(block.get("arguments")),
                            });
                        }
                    }
                    "image" => text_parts.push("[image]".to_string()),
                    _ => {}
                }
            }
            (
                text_parts.join("\n"),
                reasoning_parts.join("\n"),
                redacted,
                tool_calls,
            )
        }
        Some(value) => (jsonish(Some(value)), String::new(), false, Vec::new()),
        None => (String::new(), String::new(), false, Vec::new()),
    }
}

fn oh_my_pi_usage(raw: Option<&Value>) -> Option<BTreeMap<String, i64>> {
    let obj = raw.and_then(Value::as_object)?;
    let mut usage = BTreeMap::new();
    // rm-618: count ONE source per token class, never the sum of the alias
    // pairs — the wire spellings describe the SAME class twice (`input`
    // beside `input_tokens`, `output` beside `output_tokens`, `cacheRead`
    // beside `cache_read_input_tokens`, `cacheWrite` beside
    // `cache_creation_input_tokens`), so the old alias sum double-counted
    // every journal that carried both spellings. First present alias wins,
    // the single-source rule upstream #312 ships and the qwen lane already
    // pinned (rm-602).
    let input = first_number(obj, &["input", "input_tokens"]);
    let output = first_number(obj, &["output", "output_tokens"]);
    let cache_read = first_number(obj, &["cacheRead", "cache_read_input_tokens"]);
    let cache_write = first_number(obj, &["cacheWrite", "cache_creation_input_tokens"]);
    if input > 0 {
        usage.insert("input_tokens".to_string(), input);
    }
    if output > 0 {
        usage.insert("output_tokens".to_string(), output);
    }
    if cache_read > 0 {
        usage.insert("cache_read_input_tokens".to_string(), cache_read);
    }
    if cache_write > 0 {
        usage.insert("cache_creation_input_tokens".to_string(), cache_write);
    }
    non_empty_usage(usage)
}

fn oh_my_pi_timestamp(raw: Option<&Value>, fallback: &str) -> String {
    if let Some(ms) = raw.and_then(number_as_i64).filter(|value| *value > 0) {
        return timestamp_millis_nanos(ms);
    }
    string(raw).unwrap_or(fallback).to_string()
}

fn is_qwen_code_jsonl(objs: &[JsonObject]) -> bool {
    // rm-345: a sessionId-scrubbed claude export can still contain lines that
    // individually look qwen-ish (uuid + message, no claude-only keys), so
    // file-level detection must also require that NO line carries claude
    // evidence — one claude model string or claude-only key anywhere means
    // the whole file is a claude export, not qwen_code.
    objs.iter().any(is_qwen_code_event) && !objs.iter().any(carries_claude_evidence)
}

/// File-level claude signals: the camelCase keys qwen never writes
/// (`sessionId` on non-scrubbed exports, the claude-only keys that survive
/// scrubbing) plus claude model strings (rm-345).
fn carries_claude_evidence(obj: &Map<String, Value>) -> bool {
    obj.contains_key("sessionId")
        || CLAUDE_ONLY_JSONL_KEYS
            .iter()
            .any(|key| obj.contains_key(*key))
        || carries_claude_model(obj)
}

fn is_qwen_code_value(value: &Value) -> bool {
    match value {
        Value::Object(obj) => is_qwen_code_event(obj) || is_qwen_code_json_output(obj),
        Value::Array(items) => items
            .iter()
            .filter_map(Value::as_object)
            .any(is_qwen_code_event),
        _ => false,
    }
}

/// Keys that only claude-code transcripts carry. Privacy scrubbers drop
/// `sessionId` from claude exports but keep these; carrying any of them
/// disqualifies a sessionId-less object from the qwen_code uuid branch, so
/// scrubbed claude exports classify claude_code instead of silently
/// misattributing to qwen_code (rm-345).
const CLAUDE_ONLY_JSONL_KEYS: [&str; 5] = [
    "parentUuid",
    "isSidechain",
    "toolUseResult",
    "requestId",
    "promptId",
];

/// A claude model string inside `message.model` survives sessionId
/// scrubbing, and qwen_code never stamps claude models — so one disqualifies
/// the qwen uuid branch just like the claude-only keys do (rm-345).
/// rm-488: the whole-file-JSON lane historically probed qwen shape via
/// [`is_qwen_code_value`] WITHOUT the file-level claude-evidence guard
/// the JSONL lane applies in [`is_qwen_code_jsonl`], so a
/// single-document transcript carrying claude-only evidence
/// (`parentUuid`, camelCase id, a claude model string) misclassified as
/// qwen_code. Guard the document the same way the line lane guards the
/// file: one claude-evidence hit anywhere disqualifies the qwen reading.
fn is_qwen_code_document(value: &Value) -> bool {
    if !is_qwen_code_value(value) {
        return false;
    }
    match value {
        Value::Object(obj) => !carries_claude_evidence(obj),
        Value::Array(items) => !items
            .iter()
            .filter_map(Value::as_object)
            .any(carries_claude_evidence),
        _ => true,
    }
}

fn carries_claude_model(obj: &Map<String, Value>) -> bool {
    obj.get("message")
        .and_then(Value::as_object)
        .and_then(|message| message.get("model"))
        .and_then(Value::as_str)
        .is_some_and(|model| model.to_ascii_lowercase().starts_with("claude"))
}

fn is_qwen_code_event(obj: &Map<String, Value>) -> bool {
    let typ = string(obj.get("type")).unwrap_or("");
    if !matches!(
        typ,
        "system" | "user" | "assistant" | "result" | "stream_event"
    ) {
        return false;
    }
    if obj.contains_key("session_id") {
        // rm-449 (upstream luoyuctl/agenttrace#304 / 008ca975): a
        // Claude Code transcript that grew a qwen-shaped snake_case id
        // alongside its native camelCase `sessionId` is claude's, not
        // qwen's — the same dual-key disqualifier the uuid arm has
        // carried since rm-345. Before this, the snake_case arm
        // returned true unconditionally and whole dual-id journals
        // misclassified (tokens, model and source all wrong).
        return !obj.contains_key("sessionId");
    }
    if obj.contains_key("uuid") {
        return !obj.contains_key("sessionId")
            && !CLAUDE_ONLY_JSONL_KEYS
                .iter()
                .any(|key| obj.contains_key(*key))
            && !carries_claude_model(obj)
            && (obj.contains_key("message")
                || obj.contains_key("result")
                || obj.contains_key("subtype"));
    }
    false
}

fn is_qwen_code_json_output(obj: &Map<String, Value>) -> bool {
    (obj.contains_key("response") || obj.contains_key("error"))
        && (obj.contains_key("stats") || obj.contains_key("usage"))
}

fn parse_qwen_code_jsonl(objs: &[JsonObject]) -> anyhow::Result<Vec<Event>> {
    parse_qwen_code_objects(objs)
}

fn parse_qwen_code_value(value: &Value) -> anyhow::Result<Vec<Event>> {
    match value {
        Value::Object(obj) if is_qwen_code_json_output(obj) && !is_qwen_code_event(obj) => {
            parse_qwen_code_json_output(obj)
        }
        Value::Object(obj) => parse_qwen_code_objects(std::slice::from_ref(obj)),
        Value::Array(items) => {
            let objs = items
                .iter()
                .filter_map(Value::as_object)
                .cloned()
                .collect::<Vec<_>>();
            parse_qwen_code_objects(&objs)
        }
        _ => bail!("qwen_code: no parseable events"),
    }
}

fn parse_qwen_code_objects<I>(objs: I) -> anyhow::Result<Vec<Event>>
where
    I: IntoIterator,
    I::Item: std::borrow::Borrow<Map<String, Value>>,
{
    let mut meta_events = Vec::new();
    let mut events = Vec::new();
    let mut model = "unknown".to_string();
    let mut has_assistant = false;
    let mut has_usage = false;

    for obj in objs {
        let obj = std::borrow::Borrow::borrow(&obj);
        if !is_qwen_code_event(obj) {
            continue;
        }
        let typ = string(obj.get("type")).unwrap_or("");
        let ts = string(obj.get("timestamp")).unwrap_or("").to_string();
        match typ {
            "system" => {
                if let Some(next_model) = string(obj.get("model")).filter(|value| !value.is_empty())
                {
                    model = next_model.to_string();
                }
                meta_events.push(Event {
                    role: "meta".to_string(),
                    timestamp: ts,
                    model_used: model.clone(),
                    source_tool: "qwen_code".to_string(),
                    ..Event::default()
                });
            }
            "user" => {
                let Some(message) = obj.get("message").and_then(Value::as_object) else {
                    continue;
                };
                events.extend(qwen_message_events(message, &ts, &mut model));
            }
            "assistant" => {
                let Some(message) = obj.get("message").and_then(Value::as_object) else {
                    continue;
                };
                for event in qwen_message_events(message, &ts, &mut model) {
                    if event.role == "meta" {
                        if !event.usage.is_empty() {
                            has_usage = true;
                        }
                        meta_events.push(event);
                    } else {
                        if event.role == "assistant" {
                            has_assistant = true;
                        }
                        events.push(event);
                    }
                }
            }
            "result" => {
                if !has_usage {
                    let usage = qwen_usage(obj.get("usage"))
                        .or_else(|| qwen_stats_usage(obj.get("stats")))
                        .or_else(|| qwen_model_usage(obj.get("modelUsage")));
                    if let Some(usage) = usage {
                        meta_events.push(Event {
                            role: "meta".to_string(),
                            timestamp: ts.clone(),
                            usage,
                            model_used: model.clone(),
                            source_tool: "qwen_code".to_string(),
                            ..Event::default()
                        });
                    }
                }
                if !has_assistant {
                    let content = string(obj.get("result")).unwrap_or("").trim().to_string();
                    if !content.is_empty() {
                        events.push(Event {
                            role: "assistant".to_string(),
                            content,
                            timestamp: ts,
                            model_used: model.clone(),
                            source_tool: "qwen_code".to_string(),
                            ..Event::default()
                        });
                        has_assistant = true;
                    }
                }
                // rm-711 (assess SL2): a `result` record CLOSES the turn
                // it reports — release the usage latch so the next
                // turn's `result` usage counts. The old session-wide
                // first-wins latch kept only the first result across the
                // whole session (a 3×(10/5) journal reported 15 of a
                // truthful 45). Assistant-message usage still wins WITHIN
                // a turn: the assistant arm re-sets the latch after this
                // boundary, so an assistant-usage-plus-result pair keeps
                // counting once (pinned by
                // rust_parses_qwen_code_stream_jsonl). User records
                // deliberately do NOT touch the latch — tool-result user
                // records arrive mid-turn.
                has_usage = false;
            }
            _ => {}
        }
    }

    if events.is_empty() {
        bail!("qwen_code: no parseable events");
    }
    meta_events.extend(events);
    Ok(meta_events)
}

fn parse_qwen_code_json_output(obj: &Map<String, Value>) -> anyhow::Result<Vec<Event>> {
    let mut events = Vec::new();
    let model = "unknown".to_string();
    if let Some(usage) = qwen_usage(obj.get("usage")).or_else(|| qwen_stats_usage(obj.get("stats")))
    {
        events.push(Event {
            role: "meta".to_string(),
            usage,
            model_used: model.clone(),
            source_tool: "qwen_code".to_string(),
            ..Event::default()
        });
    }
    let content = string(obj.get("response")).unwrap_or("").trim().to_string();
    if !content.is_empty() {
        events.push(Event {
            role: "assistant".to_string(),
            content,
            model_used: model,
            source_tool: "qwen_code".to_string(),
            ..Event::default()
        });
    }
    if events.is_empty() {
        bail!("qwen_code: no parseable events");
    }
    Ok(events)
}

fn qwen_message_events(
    message: &Map<String, Value>,
    fallback_ts: &str,
    model: &mut String,
) -> Vec<Event> {
    if let Some(next_model) = string(message.get("model")).filter(|value| !value.is_empty()) {
        *model = next_model.to_string();
    }
    let ts = string(message.get("timestamp")).unwrap_or(fallback_ts);
    let mut events = Vec::new();
    if let Some(usage) = qwen_usage(message.get("usage")) {
        events.push(Event {
            role: "meta".to_string(),
            timestamp: ts.to_string(),
            usage,
            model_used: model.clone(),
            source_tool: "qwen_code".to_string(),
            ..Event::default()
        });
    }

    let (content, reasoning, redacted, tool_calls) = qwen_content(message.get("content"));
    let tool_results = qwen_tool_result_events(message.get("content"), ts, model);
    let role = string(message.get("role")).unwrap_or("assistant");
    match role {
        "assistant" => {
            if !content.is_empty() || !reasoning.is_empty() || !tool_calls.is_empty() {
                events.push(Event {
                    role: "assistant".to_string(),
                    content,
                    reasoning,
                    redacted,
                    tool_calls,
                    timestamp: ts.to_string(),
                    model_used: model.clone(),
                    source_tool: "qwen_code".to_string(),
                    ..Event::default()
                });
            }
        }
        "user" => {
            if !content.is_empty() {
                events.push(Event {
                    role: "user".to_string(),
                    content,
                    timestamp: ts.to_string(),
                    model_used: model.clone(),
                    source_tool: "qwen_code".to_string(),
                    ..Event::default()
                });
            }
            events.extend(tool_results);
        }
        "tool" | "tool_result" | "toolResult" => events.push(Event {
            role: "tool".to_string(),
            content,
            timestamp: ts.to_string(),
            tool_call_id: string(message.get("tool_call_id"))
                .or_else(|| string(message.get("toolCallId")))
                .unwrap_or("")
                .to_string(),
            is_error: boolish(message.get("is_error")) || boolish(message.get("isError")),
            model_used: model.clone(),
            source_tool: "qwen_code".to_string(),
            ..Event::default()
        }),
        _ => {}
    }
    events
}

fn qwen_content(raw: Option<&Value>) -> (String, String, bool, Vec<ToolCall>) {
    match raw {
        Some(Value::String(text)) => (text.to_string(), String::new(), false, Vec::new()),
        Some(Value::Array(blocks)) => {
            let mut text_parts = Vec::new();
            let mut reasoning_parts = Vec::new();
            let mut tool_calls = Vec::new();
            let mut redacted = false;
            for block in blocks {
                let Some(block) = block.as_object() else {
                    continue;
                };
                match string(block.get("type")).unwrap_or("") {
                    "text" => {
                        if let Some(text) =
                            string(block.get("text")).filter(|value| !value.is_empty())
                        {
                            text_parts.push(text.to_string());
                        }
                    }
                    "thinking" | "reasoning" => {
                        if let Some(text) = string(block.get("thinking"))
                            .or_else(|| string(block.get("text")))
                            .filter(|value| !value.is_empty())
                        {
                            reasoning_parts.push(text.to_string());
                        }
                    }
                    "redacted_thinking" | "redactedThinking" => {
                        redacted = true;
                        if let Some(text) = string(block.get("data"))
                            .or_else(|| string(block.get("text")))
                            .filter(|value| !value.is_empty())
                        {
                            reasoning_parts.push(text.to_string());
                        }
                    }
                    "tool_use" | "toolCall" | "function_call" => {
                        let id = string(block.get("id"))
                            .or_else(|| string(block.get("tool_call_id")))
                            .or_else(|| string(block.get("call_id")))
                            .unwrap_or("")
                            .to_string();
                        let name = string(block.get("name"))
                            .or_else(|| {
                                block
                                    .get("function")
                                    .and_then(Value::as_object)
                                    .and_then(|function| string(function.get("name")))
                            })
                            .unwrap_or("")
                            .to_string();
                        let args = jsonish(block.get("input").or_else(|| block.get("arguments")));
                        if !id.is_empty() || !name.is_empty() {
                            tool_calls.push(ToolCall { id, name, args });
                        }
                    }
                    "tool_result" | "toolResult" => {}
                    "image" => text_parts.push("[image]".to_string()),
                    _ => {}
                }
            }
            (
                text_parts.join("\n"),
                reasoning_parts.join("\n"),
                redacted,
                tool_calls,
            )
        }
        Some(value) => (jsonish(Some(value)), String::new(), false, Vec::new()),
        None => (String::new(), String::new(), false, Vec::new()),
    }
}

fn qwen_tool_result_events(raw: Option<&Value>, ts: &str, model: &str) -> Vec<Event> {
    let Some(blocks) = raw.and_then(Value::as_array) else {
        return Vec::new();
    };
    let mut events = Vec::new();
    for block in blocks {
        let Some(block) = block.as_object() else {
            continue;
        };
        let typ = string(block.get("type")).unwrap_or("");
        if typ != "tool_result" && typ != "toolResult" {
            continue;
        }
        events.push(Event {
            role: "tool".to_string(),
            content: tool_result_content(block),
            timestamp: ts.to_string(),
            tool_call_id: string(block.get("tool_use_id"))
                .or_else(|| string(block.get("toolCallId")))
                .unwrap_or("")
                .to_string(),
            is_error: boolish(block.get("is_error")) || boolish(block.get("isError")),
            model_used: model.to_string(),
            source_tool: "qwen_code".to_string(),
            ..Event::default()
        });
    }
    events
}

/// Qwen Code builds usage from Gemini-style metadata: the input aliases
/// (upstream #312: `input_tokens` IS `promptTokenCount`) are cache-
/// INCLUSIVE and multiple aliases of the same class can BOTH be present,
/// so each class counts its FIRST present alias — never the sum — and the
/// cached span rides its own cache-read line beside net input (rm-602:
/// the old alias SUM double-counted a both-alias stream, and the
/// inclusive input was stacked on top of the separately-reported cache
/// span for a 1.7x total). The CU-20 reasoning fold below is unchanged:
/// thinking counters are billed at the output rate and additionally
/// broken out as reasoning_tokens.
fn qwen_usage(raw: Option<&Value>) -> Option<BTreeMap<String, i64>> {
    let obj = raw.and_then(Value::as_object)?;
    let mut usage = BTreeMap::new();
    let cache_read = first_number(
        obj,
        &["cache_read_input_tokens", "cacheRead", "cached_tokens"],
    );
    // rm-602: net input beside the cache line — the inclusive-basis span
    // (promptTokenCount/prompt_tokens/input_tokens) already contains the
    // cached tokens reported separately above.
    let input = (first_number(
        obj,
        &["input_tokens", "prompt_tokens", "input", "promptTokenCount"],
    ) - cache_read)
        .max(0);
    let output = first_number(
        obj,
        &[
            "output_tokens",
            "completion_tokens",
            "output",
            "candidatesTokenCount",
        ],
    );
    // Thinking tokens are billed at the output rate but reported
    // separately from candidates/completion tokens (Gemini
    // usageMetadata.thoughtsTokenCount and the OpenAI-compatible
    // reasoning_tokens aliases); fold them into output and keep the
    // breakdown for the audit (pass-9 CU-20).
    let reasoning = first_number(
        obj,
        &[
            "thoughtsTokenCount",
            "thinkingTokenCount",
            "thinking_tokens",
            "reasoning_tokens",
        ],
    );
    let output = output.saturating_add(reasoning);
    let cache_write = first_number(obj, &["cache_creation_input_tokens", "cacheWrite"]);
    if input > 0 {
        usage.insert("input_tokens".to_string(), input);
    }
    if output > 0 {
        usage.insert("output_tokens".to_string(), output);
    }
    if reasoning > 0 {
        usage.insert("reasoning_tokens".to_string(), reasoning);
    }
    if cache_read > 0 {
        usage.insert("cache_read_input_tokens".to_string(), cache_read);
    }
    if cache_write > 0 {
        usage.insert("cache_creation_input_tokens".to_string(), cache_write);
    }
    non_empty_usage(usage)
}

fn qwen_stats_usage(raw: Option<&Value>) -> Option<BTreeMap<String, i64>> {
    let stats = raw.and_then(Value::as_object)?;
    let models = stats.get("models").and_then(Value::as_object)?;
    let mut usage = BTreeMap::new();
    for model_stats in models.values().filter_map(Value::as_object) {
        let Some(tokens) = model_stats.get("tokens").and_then(Value::as_object) else {
            continue;
        };
        add_usage_value(&mut usage, "input_tokens", tokens.get("input"));
        add_usage_value(&mut usage, "input_tokens", tokens.get("input_tokens"));
        add_usage_value(&mut usage, "output_tokens", tokens.get("output"));
        add_usage_value(&mut usage, "output_tokens", tokens.get("output_tokens"));
        add_usage_value(
            &mut usage,
            "cache_read_input_tokens",
            tokens.get("cacheRead"),
        );
        add_usage_value(
            &mut usage,
            "cache_read_input_tokens",
            tokens.get("cache_read_input_tokens"),
        );
        add_usage_value(
            &mut usage,
            "cache_creation_input_tokens",
            tokens.get("cacheWrite"),
        );
        add_usage_value(
            &mut usage,
            "cache_creation_input_tokens",
            tokens.get("cache_creation_input_tokens"),
        );
    }
    non_empty_usage(usage)
}

fn qwen_model_usage(raw: Option<&Value>) -> Option<BTreeMap<String, i64>> {
    let model_usage = raw.and_then(Value::as_object)?;
    let mut usage = BTreeMap::new();
    for model_stats in model_usage.values().filter_map(Value::as_object) {
        add_usage_value(&mut usage, "input_tokens", model_stats.get("inputTokens"));
        add_usage_value(&mut usage, "input_tokens", model_stats.get("input_tokens"));
        add_usage_value(&mut usage, "output_tokens", model_stats.get("outputTokens"));
        add_usage_value(
            &mut usage,
            "output_tokens",
            model_stats.get("output_tokens"),
        );
        add_usage_value(
            &mut usage,
            "cache_read_input_tokens",
            model_stats.get("cacheReadInputTokens"),
        );
        add_usage_value(
            &mut usage,
            "cache_read_input_tokens",
            model_stats.get("cache_read_input_tokens"),
        );
        add_usage_value(
            &mut usage,
            "cache_creation_input_tokens",
            model_stats.get("cacheCreationInputTokens"),
        );
        add_usage_value(
            &mut usage,
            "cache_creation_input_tokens",
            model_stats.get("cache_creation_input_tokens"),
        );
    }
    non_empty_usage(usage)
}

/// One buffered token_usage_record — top-level, or embedded as a compacted
/// payload's latest_token_usage_record — awaiting pairing against a
/// compaction marker once the whole journal is walked (rm-401).
struct CodexUsageRecord {
    response_id: String,
    usage: BTreeMap<String, i64>,
    timestamp: String,
    model: String,
}

fn collect_codex_usage_record(record: &Value, into: &mut Vec<CodexUsageRecord>, timestamp: &str) {
    // Mirrors the token_count snapshot gate: records without meaningful
    // usage are not buffered.
    let counts = token_usage_map(record.get("usage"));
    if counts.is_empty() || !usage_has_values(&counts) {
        return;
    }
    into.push(CodexUsageRecord {
        response_id: string(record.get("response_id")).unwrap_or("").to_string(),
        usage: counts,
        timestamp: timestamp.to_string(),
        model: string(record.get("model")).unwrap_or("unknown").to_string(),
    });
}

fn parse_codex_rollout_jsonl(raw: &str) -> Option<(Vec<Event>, BTreeMap<String, usize>)> {
    let mut events = Vec::new();
    let mut model = "unknown".to_string();
    let mut saw_codex = false;
    // rm-554 (upstream #312): token_count accounting state — the distinct
    // running totals already counted plus the most recent total. Replaces
    // the rm-162/#286 high-water mark, which refused ALL post-compaction
    // growth inside the old envelope even though those context re-sends
    // are billed usage.
    let mut codex_totals = CodexTotals::default();
    // rm-047: the head-probe fast path used to discard lines invisibly;
    // count every skip so parse diagnostics can surface it. rm-401
    // widened the map to the compaction-usage decisions too.
    let mut counters: BTreeMap<String, usize> = BTreeMap::new();
    // rm-401: token_usage_record events describe one response's
    // usage; only records paired to a compaction marker count (normal turns
    // are already inside the cumulative snapshots). Records can arrive
    // before or after their marker, so both are buffered and paired after
    // the loop.
    let mut usage_records: Vec<CodexUsageRecord> = Vec::new();
    let mut compaction_response_ids: BTreeSet<String> = BTreeSet::new();
    // rm-542: custom_tool_call payloads carry an explicit status; a
    // call that did not complete marks its (later) output as an error,
    // the way other parsers surface tool failures.
    let mut failed_custom_calls: BTreeSet<String> = BTreeSet::new();
    // rm-716 (tokscale #1405): rollouts the pre-Sept-2026 codex CLI
    // wrote carry no `token_count` events and no `token_usage_record`
    // rows — every usage source this lane reads — and used to parse
    // green with zero usage and no distinct verdict. Track whether ANY
    // usage row was seen so the verdict below fires only on true
    // absence.
    let mut saw_token_count_row = false;
    // A plain for-loop, not a filter-closure iterator chain: rm-542's
    // in-loop disclosure counters share `counters` with the ignorable-line
    // count, and a closure holding the mutable borrow across the whole
    // iteration would forbid both (E0499). Streaming one line at a time is
    // kept — collecting the parsed objects would pin a whole journal in
    // memory, which the adversarial 8 MiB single-line corpus exercises.
    for line in raw.lines() {
        if codex_line_is_ignorable(line) {
            *counters
                .entry("codex_ignorable_line".to_string())
                .or_insert(0) += 1;
            continue;
        }
        // rm-584: a line that parses as JSON but is not an object (bare
        // strings, numbers, arrays) used to vanish with no line_skips
        // entry; a line that is not JSON at all vanished the same way.
        // The disclosure contract requires every skipped line to name
        // its shape.
        let obj = match parse_jsonl_value_lenient(line.trim()) {
            Some(Value::Object(map)) => map,
            Some(value) => {
                *counters
                    .entry(format!("codex_non_object_line:{}", json_value_kind(&value)))
                    .or_insert(0) += 1;
                continue;
            }
            None => {
                *counters
                    .entry("codex_unparseable_line".to_string())
                    .or_insert(0) += 1;
                continue;
            }
        };
        let typ = string(obj.get("type")).unwrap_or("");
        let ts = string(obj.get("timestamp")).unwrap_or("").to_string();
        match typ {
            "session_meta" => {
                saw_codex = true;
                let mut cwd = String::new();
                if let Some(payload) = obj.get("payload").and_then(Value::as_object) {
                    if let Some(next_model) = string(payload.get("model")).filter(|m| !m.is_empty())
                    {
                        model = next_model.to_string();
                    }
                    cwd = string(payload.get("cwd")).unwrap_or("").to_string();
                }
                events.push(Event {
                    role: "meta".to_string(),
                    timestamp: ts,
                    cwd,
                    model_used: model.clone(),
                    source_tool: "codex_cli".to_string(),
                    ..Event::default()
                });
            }
            "turn_context" => {
                saw_codex = true;
                if let Some(payload) = obj.get("payload").and_then(Value::as_object) {
                    if let Some(next_model) = string(payload.get("model")).filter(|m| !m.is_empty())
                    {
                        model = next_model.to_string();
                        events.push(Event {
                            role: "meta".to_string(),
                            timestamp: ts,
                            model_used: model.clone(),
                            source_tool: "codex_cli".to_string(),
                            ..Event::default()
                        });
                    }
                }
            }
            "event_msg" => {
                saw_codex = true;
                let Some(payload) = obj.get("payload").and_then(Value::as_object) else {
                    continue;
                };
                if string(payload.get("type")) == Some("token_count") {
                    saw_token_count_row = true;
                    if let Some(usage) =
                        codex_token_count_usage(payload.get("info"), &mut codex_totals)
                    {
                        events.push(Event {
                            role: "meta".to_string(),
                            timestamp: ts,
                            usage,
                            model_used: model.clone(),
                            source_tool: "codex_cli".to_string(),
                            ..Event::default()
                        });
                    }
                }
            }
            "response_item" => {
                saw_codex = true;
                let Some(payload) = obj.get("payload").and_then(Value::as_object) else {
                    continue;
                };
                match string(payload.get("type")).unwrap_or("") {
                    "message" => {
                        let mut role = string(payload.get("role")).unwrap_or("").to_string();
                        if role == "developer" {
                            role = "system".to_string();
                        }
                        let mut content = Vec::new();
                        let mut reasoning = Vec::new();
                        match payload.get("content") {
                            Some(Value::String(text)) => content.push(text.clone()),
                            Some(Value::Array(blocks)) => {
                                for block in blocks {
                                    let Some(block) = block.as_object() else {
                                        continue;
                                    };
                                    match string(block.get("type")).unwrap_or("") {
                                        "input_text" | "output_text" | "text" => {
                                            if let Some(text) =
                                                string(block.get("text")).filter(|t| !t.is_empty())
                                            {
                                                content.push(text.to_string());
                                            }
                                        }
                                        "reasoning" | "thinking" => {
                                            if let Some(text) =
                                                string(block.get("text")).filter(|t| !t.is_empty())
                                            {
                                                reasoning.push(text.to_string());
                                            }
                                        }
                                        _ => {}
                                    }
                                }
                            }
                            _ => {}
                        }
                        for item in reasoning {
                            events.push(Event {
                                role: role.clone(),
                                reasoning: item,
                                timestamp: ts.clone(),
                                model_used: model.clone(),
                                source_tool: "codex_cli".to_string(),
                                ..Event::default()
                            });
                        }
                        events.push(Event {
                            role,
                            content: content.join("\n"),
                            timestamp: ts,
                            model_used: model.clone(),
                            source_tool: "codex_cli".to_string(),
                            ..Event::default()
                        });
                    }
                    "function_call" => {
                        events.push(Event {
                            role: "assistant".to_string(),
                            timestamp: ts,
                            tool_calls: vec![ToolCall {
                                id: string(payload.get("call_id")).unwrap_or("").to_string(),
                                name: string(payload.get("name")).unwrap_or("").to_string(),
                                args: jsonish(
                                    payload.get("arguments").or_else(|| payload.get("input")),
                                ),
                            }],
                            model_used: model.clone(),
                            source_tool: "codex_cli".to_string(),
                            ..Event::default()
                        });
                    }
                    "function_call_output" | "function_call_result" => {
                        events.push(Event {
                            role: "tool".to_string(),
                            timestamp: ts,
                            tool_call_id: string(payload.get("call_id")).unwrap_or("").to_string(),
                            content: jsonish(payload.get("output")),
                            source_tool: "codex_cli".to_string(),
                            ..Event::default()
                        });
                    }
                    "custom_tool_call" => {
                        // rm-542 (run b1ff12f8, cycle 2): the custom-tools
                        // wire. The newest real codex rollouts (2026-09-26,
                        // glm-5.x proxy catalog) carry 7-20 of these per
                        // session and ZERO classic function_calls, so those
                        // sessions parsed as tool_calls_total=0 while real
                        // tool work happened — the entire tool-waste,
                        // tool-failure, and authority lens was blind. Mirror
                        // the function_call arm (same ToolCall shape; the
                        // JS-source `input` string feeds args/tool_usage the
                        // way `arguments` did) and remember a
                        // non-"completed" status so the paired output counts
                        // as a failure instead of a silent success.
                        let status = string(payload.get("status")).unwrap_or("completed");
                        let call_id = string(payload.get("call_id")).unwrap_or("").to_string();
                        // rm-542 F3 (dated append, 2026-10-06): an
                        // id-less call cannot be paired to its output —
                        // `""` is a legal journal value and every later
                        // id-less success output inherited the failure
                        // (PoC p3-emptyid). And a call that REACHES
                        // "completed" after an earlier non-completed
                        // attempt under the same id transitions the pair
                        // to success — the latest status wins, so a
                        // replayed/duplicate id cannot pin a permanent
                        // failure on outputs that succeeded.
                        if !call_id.is_empty() {
                            if status != "completed" {
                                failed_custom_calls.insert(call_id.clone());
                            } else {
                                failed_custom_calls.remove(&call_id);
                            }
                        }
                        events.push(Event {
                            role: "assistant".to_string(),
                            timestamp: ts,
                            tool_calls: vec![ToolCall {
                                id: call_id,
                                name: string(payload.get("name")).unwrap_or("").to_string(),
                                args: jsonish(payload.get("input")),
                            }],
                            model_used: model.clone(),
                            source_tool: "codex_cli".to_string(),
                            ..Event::default()
                        });
                    }
                    "custom_tool_call_output" => {
                        let call_id = string(payload.get("call_id")).unwrap_or("").to_string();
                        events.push(Event {
                            role: "tool".to_string(),
                            timestamp: ts,
                            tool_call_id: call_id.clone(),
                            content: jsonish(payload.get("output")),
                            is_error: failed_custom_calls.contains(&call_id),
                            source_tool: "codex_cli".to_string(),
                            ..Event::default()
                        });
                    }
                    "reasoning" => {
                        // rm-542: standalone reasoning items — the rollout
                        // records them WITHOUT a sibling message item, with
                        // the readable text in summary[] summary_text blocks
                        // (`content` is null; the raw thought ships
                        // encrypted). Emit the same assistant-reasoning
                        // event shape the message arm uses, so
                        // reasoning_blocks/reasoning_chars and the
                        // assistant-turn census see custom-tools sessions
                        // (they used to read 0 while the file carried 3-9
                        // reasoning items per session).
                        let mut texts = Vec::new();
                        if let Some(Value::Array(blocks)) = payload.get("summary") {
                            for block in blocks {
                                let Some(block) = block.as_object() else {
                                    continue;
                                };
                                match string(block.get("type")).unwrap_or("") {
                                    "summary_text" | "text" => {
                                        if let Some(text) =
                                            string(block.get("text")).filter(|t| !t.is_empty())
                                        {
                                            texts.push(text.to_string());
                                        }
                                    }
                                    _ => {}
                                }
                            }
                        }
                        for text in texts {
                            events.push(Event {
                                role: "assistant".to_string(),
                                timestamp: ts.clone(),
                                reasoning: text,
                                model_used: model.clone(),
                                source_tool: "codex_cli".to_string(),
                                ..Event::default()
                            });
                        }
                    }
                    unknown_payload => {
                        // rm-542 acceptance (4): future wire growth must be
                        // visible, not silent — the custom-tools gap hid for
                        // a month because unknown payload types incremented
                        // nothing (rm-401 widened-channel precedent).
                        // rm-594: route unmatched-shape keys through the
                        // shared capped+sanitized mint helper (same shape
                        // as before for benign values; hostile/oversized
                        // values become bounded and control-byte-free).
                        *counters
                            .entry(disclosure_key(
                                "codex_unmatched_response_item",
                                unknown_payload,
                            ))
                            .or_insert(0) += 1;
                    }
                }
            }
            "compacted" => {
                // rm-711 (assess SL1): the compaction marker re-bases the
                // cumulative token_count totals — open a fresh accounting
                // envelope before any pairing so the post-compaction windows
                // count their `last` snapshots instead of colliding with
                // values already counted pre-compaction.
                codex_totals.begin_envelope();
                // rm-401: a compaction boundary. The turn that produced
                // the summary is invisible to every later cumulative snapshot,
                // so its usage arrives as a token_usage_record paired to
                // payload.compaction_response_id — sometimes replayed inside
                // this very payload as latest_token_usage_record. Register the
                // marker; buffered records pair with it after the loop.
                if let Some(payload) = obj.get("payload").and_then(Value::as_object) {
                    if let Some(response_id) = string(payload.get("compaction_response_id")) {
                        if !response_id.is_empty() {
                            compaction_response_ids.insert(response_id.to_string());
                        }
                    }
                    if let Some(record) = payload.get("latest_token_usage_record") {
                        collect_codex_usage_record(record, &mut usage_records, &ts);
                    }
                }
            }
            "token_usage_record" => {
                // rm-401: per-response usage records. Compaction turns
                // carry theirs here; normal turns' usage already lives in the
                // cumulative token_count snapshots.
                if let Some(record) = obj.get("payload") {
                    collect_codex_usage_record(record, &mut usage_records, &ts);
                }
            }
            "world_state" => {
                // rm-542 acceptance (3): sub-registry metadata observed in
                // the 2026-09-26 rollouts — neither tool nor message, but a
                // real top-level type on disk. Explicitly ignored AND
                // counted, so the line is disclosed in parse diagnostics
                // rather than silently dropped.
                saw_codex = true;
                *counters.entry("codex_world_state".to_string()).or_insert(0) += 1;
            }
            "" => {
                // rm-584: a missing or empty top-level type used to fall
                // through silently — count it so a wire-format change that
                // drops the field can never quietly empty a journal.
                *counters
                    .entry("codex_missing_type".to_string())
                    .or_insert(0) += 1;
            }
            unknown_top_level => {
                // rm-542: same disclosure duty as the response_item arm —
                // an unrecognized top-level type is drift, not noise.
                // rm-401: unknown top-level types surface in parse
                // diagnostics under a named counter; rm-594 routes the
                // key through the shared capped+sanitized mint helper.
                *counters
                    .entry(disclosure_key("codex_unmatched_type", unknown_top_level))
                    .or_insert(0) += 1;
            }
        }
    }
    // Pair buffered usage records against compaction markers (rm-401).
    // Counted once per response_id: a marker's latest_token_usage_record and
    // the replayed top-level record describe the same turn (upstream ccusage
    // #1821 dedups the copy). Unpaired records describe turns whose usage is
    // already inside the cumulative snapshots, so they count for nothing but
    // stay visible in diagnostics — the class this defect evaded. Counted
    // usage never touches codex_totals: the compaction turn is outside
    // every cumulative snapshot, so feeding it to the totals state would
    // eat the next distinct total's window.
    let mut counted: BTreeSet<&str> = BTreeSet::new();
    for record in &usage_records {
        if record.response_id.is_empty() || !compaction_response_ids.contains(&record.response_id) {
            *counters
                .entry("codex_token_usage_record_unpaired".to_string())
                .or_insert(0) += 1;
            continue;
        }
        if !counted.insert(record.response_id.as_str()) {
            *counters
                .entry("codex_compaction_usage_duplicate".to_string())
                .or_insert(0) += 1;
            continue;
        }
        let mut cache_read = record
            .usage
            .get("cached_input_tokens")
            .copied()
            .unwrap_or(0);
        if cache_read == 0 {
            cache_read = record
                .usage
                .get("cache_read_input_tokens")
                .copied()
                .unwrap_or(0);
        }
        let cache_write = record
            .usage
            .get("cache_creation_input_tokens")
            .copied()
            .unwrap_or(0);
        let input = (record.usage.get("input_tokens").copied().unwrap_or(0) - cache_read).max(0);
        let output = record
            .usage
            .get("output_tokens")
            .copied()
            .unwrap_or(0)
            .saturating_add(
                record
                    .usage
                    .get("reasoning_output_tokens")
                    .copied()
                    .unwrap_or(0),
            );
        let usage = BTreeMap::from([
            ("input_tokens".to_string(), input),
            ("output_tokens".to_string(), output),
            ("cache_creation_input_tokens".to_string(), cache_write),
            ("cache_read_input_tokens".to_string(), cache_read),
        ]);
        events.push(Event {
            role: "meta".to_string(),
            timestamp: record.timestamp.clone(),
            model_used: record.model.clone(),
            source_tool: "codex_cli".to_string(),
            usage,
            ..Event::default()
        });
        *counters
            .entry("codex_compaction_usage_record".to_string())
            .or_insert(0) += 1;
    }
    // rm-716 (tokscale #1405): a rollout with zero usage rows on every
    // source this lane reads is the pre-Sept-2026 codex CLI's journal
    // shape, not a free session. Ship the absence as a distinct verdict
    // on the non-loss disclosure channel (rm-538/rm-719 convention) —
    // it renders under "Disclosed facts" and never degrades
    // data_health.confidence, because it is an informational fact about
    // the journal's era, not a parse failure. Nothing is fabricated:
    // with no usage rows the token totals stay exactly what the text
    // estimator derived (provenance stays estimated_from_text).
    if saw_codex && !saw_token_count_row && usage_records.is_empty() {
        events.push(Event {
            role: "meta".to_string(),
            timestamp: events
                .last()
                .map(|event| event.timestamp.clone())
                .unwrap_or_default(),
            model_used: model.clone(),
            source_tool: "codex_cli".to_string(),
            disclosure_counters: BTreeMap::from([("codex_rollout_no_usage_rows".to_string(), 1)]),
            ..Event::default()
        });
    }
    if saw_codex {
        non_empty(events).map(|events| (events, counters))
    } else {
        None
    }
}

// Stable serde_json shape name for the rm-584 codex skip counters
// (`codex_non_object_line:<kind>`), so a bare string/number/array line names
// its shape instead of vanishing.
fn json_value_kind(value: &Value) -> &'static str {
    match value {
        Value::Null => "null",
        Value::Bool(_) => "bool",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    }
}

// Codex rollouts are dominated by event_msg payloads (item_completed carries
// full tool output); skip them before JSON-decoding the line. The usage-bearing
// token_count event is the one event_msg worth keeping: rescue it whenever its
// marker appears anywhere in the line at a key boundary. compacted lines
// deliberately do NOT take this fast path (rm-401): they are rare (one
// per compaction boundary) and they now carry the pairing data
// (compaction_response_id / latest_token_usage_record) for the compaction
// turn's token_usage_record, which the blanket skip dropped along with the
// snapshots.
fn codex_line_is_ignorable(line: &str) -> bool {
    let Some(head) = line.get(..line.len().min(160)) else {
        return false;
    };
    json_key_present_top_level(head, r#""type":"event_msg""#)
        && !json_key_present(line, r#""type":"token_count""#)
}

/// True when `needle` occurs in `line` at a JSON key position (the byte
/// before it is `{` or `,`). Valid JSON cannot place the raw needle
/// inside a string value — its quotes would have to be escaped — so this
/// anchors the skip decision against lenient-parsed or corrupt lines.
fn json_key_present(line: &str, needle: &str) -> bool {
    let bytes = line.as_bytes();
    let mut from = 0;
    while let Some(at) = line[from..].find(needle) {
        let start = from + at;
        let anchored = start == 0 || matches!(bytes[start - 1], b'{' | b',');
        if anchored {
            return true;
        }
        from = start + 1;
    }
    false
}

/// Like [`json_key_present`], but the needle must sit at the top level
/// of `line`'s root object (container depth 1) rather than inside a
/// nested object or array. rm-047 residual (assess F1, 2026-10-05): the
/// plain `{`/`,` anchor also accepts a NESTED key such as
/// `"payload":{"z0":{"type":"event_msg"}}` — the nested object's
/// own opening brace precedes the needle — so a usage-bearing line that
/// merely mentions the marker inside a payload key was skipped whole
/// (live PoC flipped cost 0.0168 -> 0.0062 and dropped the model row).
fn json_key_present_top_level(line: &str, needle: &str) -> bool {
    let bytes = line.as_bytes();
    let mut from = 0;
    while let Some(at) = line[from..].find(needle) {
        let start = from + at;
        let anchored = start == 0 || matches!(bytes[start - 1], b'{' | b',');
        if anchored && json_container_depth(bytes, start) == 1 {
            return true;
        }
        from = start + 1;
    }
    false
}

/// Container nesting depth at byte offset `pos`: 1 while directly inside
/// `line`'s root object, 2 inside a child object or array, and so on.
/// String-aware — braces and brackets inside (possibly corrupt) string
/// values do not count as structure — and saturating, so truncated
/// heads cannot underflow the depth.
fn json_container_depth(bytes: &[u8], pos: usize) -> usize {
    let mut depth: usize = 0;
    let mut in_string = false;
    let mut escaped = false;
    for &byte in &bytes[..pos] {
        if in_string {
            if escaped {
                escaped = false;
            } else if byte == b'\\' {
                escaped = true;
            } else if byte == b'"' {
                in_string = false;
            }
        } else if byte == b'"' {
            in_string = true;
        } else if byte == b'{' || byte == b'[' {
            depth += 1;
        } else if byte == b'}' || byte == b']' {
            depth = depth.saturating_sub(1);
        }
    }
    depth
}

fn codex_token_count_usage(
    raw_info: Option<&Value>,
    totals: &mut CodexTotals,
) -> Option<TokenUsage> {
    let info = raw_info?.as_object()?;
    let total = token_usage_map(info.get("total_token_usage"));
    let last = token_usage_map(info.get("last_token_usage"));
    let counts = if total.is_empty() {
        last
    } else {
        let prev = totals.prev.replace(total.clone());
        if prev.as_ref() == Some(&total) {
            // rm-554: the same running total re-emitted consecutively
            // (rate-limit-only update) carries no fresh usage — count
            // nothing. Stated as an explicit comparison against `prev`
            // so the guard survives the ledger rollovers rm-711 added
            // below (envelope boundary + size cap).
            return None;
        }
        if totals.seen.len() >= MAX_CODEX_SEEN_TOTALS {
            // rm-711 (assess SL3): bounded-recent distincts — roll the
            // ledger over instead of growing one set entry per call.
            totals.seen.clear();
        }
        if !totals.seen.insert(total.clone()) {
            // rm-554/rm-711: this exact total was already counted in the
            // current compaction envelope — a rewind-and-climb-back, not
            // a fresh window. `compacted` markers clear the ledger
            // (CodexTotals::begin_envelope) because the re-based counters
            // re-emit old values while carrying fresh usage.
            return None;
        }
        if usage_has_values(&last) {
            // The snapshot of the call that just finished IS the fresh
            // usage this window added (post-compaction windows included).
            last
        } else {
            // No last snapshot in older journals: approximate the window
            // as the forward delta from the previous distinct total.
            token_usage_delta(&total, prev.as_ref())
        }
    };
    if counts.is_empty() || !usage_has_values(&counts) {
        return None;
    }

    let mut cache_read = counts.get("cached_input_tokens").copied().unwrap_or(0);
    if cache_read == 0 {
        cache_read = counts.get("cache_read_input_tokens").copied().unwrap_or(0);
    }
    let cache_write = counts
        .get("cache_creation_input_tokens")
        .copied()
        .unwrap_or(0);
    let input = (counts.get("input_tokens").copied().unwrap_or(0) - cache_read).max(0);
    // rm-603 (upstream #312): on the OpenAI wire Codex reports,
    // reasoning_output_tokens is a BREAKDOWN of output_tokens, not an
    // addition — the old fold billed every reasoning turn twice. Keep it
    // on its own reasoning_tokens line (billed at the output rate by CU-20
    // convention, visible in the metrics breakdown); the rm-401
    // token_usage_record lane below keeps its deliberate fold.
    let output = counts.get("output_tokens").copied().unwrap_or(0);
    let reasoning = counts.get("reasoning_output_tokens").copied().unwrap_or(0);

    let mut usage = BTreeMap::new();
    usage.insert("input_tokens".to_string(), input);
    usage.insert("output_tokens".to_string(), output);
    if reasoning > 0 {
        usage.insert("reasoning_tokens".to_string(), reasoning);
    }
    usage.insert("cache_creation_input_tokens".to_string(), cache_write);
    usage.insert("cache_read_input_tokens".to_string(), cache_read);
    Some(usage)
}

/// rm-554 (upstream #312): token_count accounting state — the distinct
/// running totals already counted and the most recent total. Replaces the
/// rm-162/#286 high-water mark, which refused all post-compaction growth
/// inside the old envelope even though those context re-sends are billed
/// usage (the counters are cumulative and NOT monotonic: compaction resets
/// them, and the same total re-emits on rate-limit-only updates).
///
/// rm-711 (assess SL1/SL3): the `seen` ledger is scoped to one compaction
/// envelope (`begin_envelope` clears it at every `compacted` marker) and
/// hard-capped, so the value-dedup never grows without bound while the
/// post-compaction re-based counters still count their fresh `last`
/// snapshots.
#[derive(Default)]
struct CodexTotals {
    prev: Option<TokenUsage>,
    seen: BTreeSet<TokenUsage>,
}

/// rm-711 (assess SL3): hard cap on the distinct-totals ledger. A journal
/// with more distinct totals than this rolls the ledger over (treated as
/// an implicit envelope boundary) instead of pinning one set entry per
/// call in memory. The rate-limit guard is unaffected: its signature is
/// the consecutive duplicate checked against `prev`, not set membership.
/// 1024 distinct totals without a compaction marker is ~1024 calls;
/// rewinds deeper than that lose value-dedup only.
const MAX_CODEX_SEEN_TOTALS: usize = 1024;

impl CodexTotals {
    /// rm-711 (assess SL1): a `compacted` marker re-bases Codex's
    /// cumulative token counters — the compacted context is re-sent and
    /// billed, so values already counted in the previous envelope
    /// legitimately reappear carrying a fresh `last` snapshot. Open a new
    /// envelope: drop the value-dedup ledger and the delta baseline so
    /// the re-based windows count again.
    fn begin_envelope(&mut self) {
        self.seen.clear();
        self.prev = None;
    }
}

fn token_usage_map(raw: Option<&Value>) -> TokenUsage {
    let Some(obj) = raw.and_then(Value::as_object) else {
        return BTreeMap::new();
    };
    [
        "input_tokens",
        "output_tokens",
        "cached_input_tokens",
        "cache_read_input_tokens",
        "cache_creation_input_tokens",
        "reasoning_output_tokens",
    ]
    .iter()
    .filter_map(|key| {
        obj.get(*key)
            .and_then(number_as_i64)
            .filter(|value| *value > 0)
            .map(|value| ((*key).to_string(), value))
    })
    .collect()
}

// Codex can briefly rewind total_token_usage (e.g. after compaction) and then
// climb back; rm-554 (upstream #312) counts each distinct total's last
// snapshot once so the post-compaction climb is real usage — this replaced
// the rm-035/rm-162 high-water helper (upstream #286), which double-refused
// that window.

fn token_usage_delta(cur: &TokenUsage, prev: Option<&TokenUsage>) -> TokenUsage {
    let Some(prev) = prev else {
        return cur.clone();
    };
    [
        "input_tokens",
        "output_tokens",
        "cached_input_tokens",
        "cache_read_input_tokens",
        "cache_creation_input_tokens",
        "reasoning_output_tokens",
    ]
    .iter()
    .filter_map(|key| {
        let delta = cur.get(*key).copied().unwrap_or(0) - prev.get(*key).copied().unwrap_or(0);
        (delta > 0).then(|| ((*key).to_string(), delta))
    })
    .collect()
}

/// rm-905: decide whether a REPEATED claude message id is still a genuine
/// streaming re-emission (fold per rm-601/rm-834) or a distinct response a
/// relay/gateway answered under a reused id (count per emission). The fold
/// keys on the id alone — so a relay that answers every response with ONE
/// foreign id (ccusage #1635: `ocgo`, no request id) had its whole session
/// max-folded down to a single response's numbers. Genuine streaming keeps
/// two invariants the relay shape breaks: the id carries Anthropic's real
/// `msg_*` shape, and every reported token class is non-decreasing against
/// the folded snapshot (re-emissions are running totals of one message). A
/// repeat that fails either invariant is a new response.
fn claude_reemission(
    id: &str,
    folded: &BTreeMap<String, i64>,
    incoming: &BTreeMap<String, i64>,
) -> bool {
    let real_shape = match id.strip_prefix("msg_") {
        Some(rest) => {
            !rest.is_empty()
                && rest
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
        }
        None => false,
    };
    real_shape
        && incoming
            .iter()
            .all(|(key, value)| folded.get(key).is_none_or(|prev| value >= prev))
}

fn parse_claude_code_jsonl(objs: &[JsonObject]) -> Option<Vec<Event>> {
    let mut events = Vec::new();
    let mut model = "unknown".to_string();
    let mut saw_claude = false;
    // rm-601 (upstream #312): streaming writes one assistant row per
    // content block, each repeating the message's usage with growing
    // totals on the SAME message id. Fold re-emissions into ONE meta
    // event per id, keeping the max per token class (never the sum: a
    // 5-block stream tripled the reported output). Exact duplicates
    // collapse to the same values; id-less messages keep the legacy
    // behavior of counting every emission (pinned by fixture).
    // rm-834: the fold is not usage-only — a re-emission is a full
    // SNAPSHOT of the message, so the per-id assistant event is replaced
    // in place by the latest (richest) snapshot and the meta slot's
    // timestamp advances to the last emission. Turns, tool calls and
    // session_end/duration therefore describe the message once, at its
    // final state — instead of once per streamed block.
    let mut usage_by_message: BTreeMap<String, usize> = BTreeMap::new();
    let mut assistant_by_message: BTreeMap<String, usize> = BTreeMap::new();
    let mut tool_results_by_message: BTreeSet<(String, String)> = BTreeSet::new();
    // rm-905: message ids the usage guard has already deemed relay reuse
    // (foreign id shape, or non-monotone totals under a repeated id) —
    // every later fold for that id counts per emission instead of replacing
    // in place, so a relay answering N responses under ONE id still parses
    // as N responses.
    let mut reused_message_ids: BTreeSet<String> = BTreeSet::new();
    let mut cwd = String::new();
    for obj in objs.iter() {
        let typ = string(obj.get("type")).unwrap_or("");
        if cwd.is_empty() {
            if let Some(body_cwd) = string(obj.get("cwd")).filter(|value| !value.is_empty()) {
                cwd = body_cwd.to_string();
                // rm-585 (spend-by-branch, first cut): gitBranch rides
                // the same envelope as cwd on every claude-code line;
                // capture it onto the session_meta event so the session
                // carries the branch it ran on. Detached HEAD writes
                // the literal "HEAD" — kept verbatim here, normalized
                // to the "unknown" bucket only in the overview rollup
                // (never a panic: the value is a plain map key).
                let branch = string(obj.get("gitBranch"))
                    .filter(|value| !value.is_empty())
                    .unwrap_or("")
                    .to_string();
                events.push(Event {
                    role: "session_meta".to_string(),
                    cwd: cwd.clone(),
                    branch,
                    model_used: model.clone(),
                    source_tool: "claude_code".to_string(),
                    ..Event::default()
                });
            }
        }
        match typ {
            "user" => {
                saw_claude = true;
                let ts = string(obj.get("timestamp")).unwrap_or("").to_string();
                let Some(message) = obj.get("message").and_then(Value::as_object) else {
                    continue;
                };
                claude_user_content_events(message.get("content"), &ts, &model, &mut events);
            }
            "assistant" => {
                saw_claude = true;
                let ts = string(obj.get("timestamp")).unwrap_or("").to_string();
                let Some(message) = obj.get("message").and_then(Value::as_object) else {
                    continue;
                };
                if model == "unknown" {
                    if let Some(next_model) = string(message.get("model")).filter(|m| !m.is_empty())
                    {
                        model = next_model.to_string();
                    }
                }
                if let Some(usage) = message.get("usage").and_then(usage_from_value) {
                    let message_id = string(message.get("id")).unwrap_or("");
                    match usage_by_message.get(message_id).copied() {
                        // rm-601/rm-834 re-emission arm: the guard in
                        // `claude_reemission` (rm-905) has already confirmed
                        // this repeat still looks like genuine streaming.
                        Some(index)
                            if claude_reemission(message_id, &events[index].usage, &usage) =>
                        {
                            for (key, value) in usage {
                                let slot = events[index].usage.entry(key).or_insert(0);
                                *slot = (*slot).max(value);
                            }
                            // rm-834: the running total's last emission is
                            // the message's last activity — keep the meta
                            // slot at the final timestamp so session_end and
                            // duration cover the whole stream, not the first
                            // block.
                            if !ts.is_empty() {
                                events[index].timestamp = ts.clone();
                            }
                        }
                        Some(_) => {
                            // rm-905: a repeated id that is NOT a genuine
                            // re-emission — a foreign id shape (a relay
                            // answering every response with ONE id, ccusage
                            // #1635: `ocgo`, no request id) or non-monotone
                            // totals under a real shape — is a NEW response:
                            // count it per emission instead of max-folding it
                            // into invisibility, and disclose the reuse so
                            // the session stays inspectable. Id-less rows
                            // never reach this arm (they never enter the
                            // map — pinned count-every-emission legacy).
                            if !message_id.is_empty() {
                                reused_message_ids.insert(message_id.to_string());
                                usage_by_message.insert(message_id.to_string(), events.len());
                            }
                            events.push(Event {
                                role: "meta".to_string(),
                                timestamp: ts.clone(),
                                usage,
                                model_used: model.clone(),
                                source_tool: "claude_code".to_string(),
                                disclosure_counters: BTreeMap::from([(
                                    "relay_reused_message_id".to_string(),
                                    1,
                                )]),
                                ..Event::default()
                            });
                        }
                        None => {
                            if !message_id.is_empty() {
                                usage_by_message.insert(message_id.to_string(), events.len());
                            }
                            events.push(Event {
                                role: "meta".to_string(),
                                timestamp: ts.clone(),
                                usage,
                                model_used: model.clone(),
                                source_tool: "claude_code".to_string(),
                                ..Event::default()
                            });
                        }
                    }
                }
                let mut assistant_parts = Vec::new();
                let mut reasoning_parts = Vec::new();
                let mut tool_calls = Vec::new();
                for block in message
                    .get("content")
                    .and_then(Value::as_array)
                    .into_iter()
                    .flatten()
                {
                    let Some(block) = block.as_object() else {
                        continue;
                    };
                    match string(block.get("type")).unwrap_or("") {
                        "text" => {
                            if let Some(text) = string(block.get("text")).filter(|t| !t.is_empty())
                            {
                                assistant_parts.push(text.to_string());
                            }
                        }
                        "thinking" => {
                            if let Some(text) =
                                string(block.get("thinking")).filter(|t| !t.is_empty())
                            {
                                reasoning_parts.push(text.to_string());
                            }
                        }
                        "tool_use" => tool_calls.push(ToolCall {
                            id: string(block.get("id")).unwrap_or("").to_string(),
                            name: string(block.get("name")).unwrap_or("").to_string(),
                            args: jsonish(block.get("input").or_else(|| block.get("arguments"))),
                        }),
                        "tool_result" => {
                            // rm-834: within one message id a re-emission
                            // repeats the tool_result blocks it already
                            // carried — only the first copy is a distinct
                            // result. Id-less rows keep legacy behavior.
                            let tool_use_id =
                                string(block.get("tool_use_id")).unwrap_or("").to_string();
                            let message_id = string(message.get("id")).unwrap_or("");
                            if !message_id.is_empty()
                                && !reused_message_ids.contains(message_id)
                                && !tool_results_by_message
                                    .insert((message_id.to_string(), tool_use_id.clone()))
                            {
                                continue;
                            }
                            events.push(Event {
                                role: "tool".to_string(),
                                timestamp: ts.clone(),
                                tool_call_id: tool_use_id,
                                content: tool_result_content(block),
                                is_error: block
                                    .get("is_error")
                                    .and_then(Value::as_bool)
                                    .unwrap_or(false),
                                source_tool: "claude_code".to_string(),
                                ..Event::default()
                            })
                        }
                        _ => {}
                    }
                }
                if !assistant_parts.is_empty()
                    || !reasoning_parts.is_empty()
                    || !tool_calls.is_empty()
                {
                    let assistant_event = Event {
                        role: "assistant".to_string(),
                        content: assistant_parts.join("\n"),
                        reasoning: reasoning_parts.join("\n"),
                        timestamp: ts,
                        tool_calls,
                        model_used: model.clone(),
                        source_tool: "claude_code".to_string(),
                        ..Event::default()
                    };
                    // rm-834: fold the snapshot per message id — the last
                    // re-emission (the complete one) replaces the earlier
                    // partial snapshot in place, so turns, tool calls,
                    // anomalies and spans see the message once. Id-less
                    // rows keep one event per row (pinned legacy).
                    let message_id = string(message.get("id")).unwrap_or("");
                    if message_id.is_empty() {
                        events.push(assistant_event);
                    } else {
                        match assistant_by_message.get_mut(message_id) {
                            // rm-905: an id the usage guard already flagged as
                            // relay reuse counts EVERY emission — no in-place
                            // snapshot replace, so N relay responses under one
                            // id stay N assistant turns.
                            Some(index) if !reused_message_ids.contains(message_id) => {
                                events[*index] = assistant_event
                            }
                            _ => {
                                assistant_by_message.insert(message_id.to_string(), events.len());
                                events.push(assistant_event);
                            }
                        }
                    }
                }
            }
            _ => {}
        }
    }
    if saw_claude {
        non_empty(events)
    } else {
        None
    }
}

fn claude_user_content_events(
    content: Option<&Value>,
    ts: &str,
    model: &str,
    events: &mut Vec<Event>,
) {
    match content {
        Some(Value::String(text)) => events.push(Event {
            role: "user".to_string(),
            content: text.to_string(),
            timestamp: ts.to_string(),
            model_used: model.to_string(),
            source_tool: "claude_code".to_string(),
            ..Event::default()
        }),
        Some(Value::Array(blocks)) => {
            for block in blocks {
                let Some(block) = block.as_object() else {
                    continue;
                };
                match string(block.get("type")).unwrap_or("") {
                    "text" => events.push(Event {
                        role: "user".to_string(),
                        content: string(block.get("text")).unwrap_or("").to_string(),
                        timestamp: ts.to_string(),
                        model_used: model.to_string(),
                        source_tool: "claude_code".to_string(),
                        ..Event::default()
                    }),
                    "tool_result" => events.push(Event {
                        role: "tool".to_string(),
                        timestamp: ts.to_string(),
                        tool_call_id: string(block.get("tool_use_id")).unwrap_or("").to_string(),
                        content: tool_result_content(block),
                        is_error: block
                            .get("is_error")
                            .and_then(Value::as_bool)
                            .unwrap_or(false),
                        source_tool: "claude_code".to_string(),
                        ..Event::default()
                    }),
                    _ => {}
                }
            }
        }
        _ => {}
    }
}

fn parse_copilot_jsonl(objs: &[JsonObject]) -> Option<Vec<Event>> {
    let mut events = Vec::new();
    let mut model = "unknown".to_string();
    let mut saw_copilot = false;
    for span in objs.iter() {
        let name = string(span.get("name")).unwrap_or("");
        if name.is_empty() || !span.contains_key("traceId") {
            continue;
        }
        saw_copilot = true;
        if let Some(next_model) = copilot_string_attr(span, "gen_ai.request.model") {
            model = next_model;
        }
        let ts = copilot_timestamp(span.get("startTimeUnixNano"));
        let usage = copilot_usage(span);
        match name {
            "chat.completion" => {
                let content = copilot_span_content(span);
                if !content.is_empty() {
                    events.push(Event {
                        role: "assistant".to_string(),
                        content,
                        timestamp: ts.clone(),
                        model_used: model.clone(),
                        source_tool: "copilot_cli".to_string(),
                        ..Event::default()
                    });
                }
                if usage_has_values(&usage) {
                    events.push(Event {
                        role: "meta".to_string(),
                        usage,
                        model_used: model.clone(),
                        source_tool: "copilot_cli".to_string(),
                        ..Event::default()
                    });
                }
            }
            "tool.call" => events.push(Event {
                role: "assistant".to_string(),
                timestamp: ts,
                tool_calls: vec![ToolCall {
                    id: copilot_string_attr(span, "tool.call.id")
                        .or_else(|| string(span.get("spanId")).map(str::to_string))
                        .unwrap_or_default(),
                    name: copilot_string_attr(span, "tool.name").unwrap_or_default(),
                    ..ToolCall::default()
                }],
                model_used: model.clone(),
                source_tool: "copilot_cli".to_string(),
                ..Event::default()
            }),
            "tool.result" => events.push(Event {
                role: "tool".to_string(),
                content: copilot_span_content(span),
                timestamp: ts,
                tool_call_id: copilot_string_attr(span, "tool.call.id")
                    .or_else(|| string(span.get("parentSpanId")).map(str::to_string))
                    .unwrap_or_default(),
                is_error: copilot_bool_attr(span, "tool.result.is_error"),
                model_used: model.clone(),
                source_tool: "copilot_cli".to_string(),
                ..Event::default()
            }),
            _ => {
                let content = copilot_span_content(span);
                if !content.is_empty() {
                    events.push(Event {
                        role: "assistant".to_string(),
                        content,
                        timestamp: ts,
                        model_used: model.clone(),
                        source_tool: "copilot_cli".to_string(),
                        ..Event::default()
                    });
                }
            }
        }
    }
    if saw_copilot {
        non_empty(events)
    } else {
        None
    }
}

fn parse_kimi_value(value: &Value) -> Option<Vec<Event>> {
    let doc = value.as_object()?;
    if !doc.contains_key("messages") || !doc.contains_key("model") {
        return None;
    }
    let model = string(doc.get("model")).unwrap_or("unknown").to_string();
    let mut events = Vec::new();
    if let Some(usage) = doc.get("usage").and_then(usage_from_value) {
        events.push(Event {
            role: "meta".to_string(),
            usage,
            model_used: model.clone(),
            source_tool: "kimi_cli".to_string(),
            ..Event::default()
        });
    }
    if let Some(usage) = doc
        .get("metadata")
        .and_then(Value::as_object)
        .and_then(|metadata| metadata.get("usage"))
        .and_then(usage_from_value)
    {
        events.push(Event {
            role: "meta".to_string(),
            usage,
            model_used: model.clone(),
            source_tool: "kimi_cli".to_string(),
            ..Event::default()
        });
    }
    for message in doc.get("messages")?.as_array()? {
        let Some(message) = message.as_object() else {
            continue;
        };
        kimi_message_events(message, &model, &mut events);
    }
    non_empty(events)
}

fn parse_messages_value(value: &Value, source_tool: &str) -> Option<Vec<Event>> {
    let doc = value.as_object()?;
    let messages = doc.get("messages")?.as_array()?;
    let model = string(doc.get("model")).unwrap_or("unknown").to_string();
    let mut events = Vec::new();
    for message in messages {
        if let Some(mut event) = event_from_message(message, &model) {
            event.source_tool = source_tool.to_string();
            events.push(event);
        }
    }
    non_empty(events)
}

fn event_from_message(message: &Value, model: &str) -> Option<Event> {
    let obj = message.as_object()?;
    let role = obj
        .get("role")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    let timestamp = obj
        .get("timestamp")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    let mut content = String::new();
    let mut reasoning = String::new();
    if let Some(value) = obj.get("content") {
        match value {
            Value::String(text) => content = text.clone(),
            Value::Array(blocks) => {
                for block in blocks {
                    let Some(block) = block.as_object() else {
                        continue;
                    };
                    match block.get("type").and_then(Value::as_str).unwrap_or("") {
                        "text" => {
                            if let Some(text) = block.get("text").and_then(Value::as_str) {
                                if !content.is_empty() {
                                    content.push('\n');
                                }
                                content.push_str(text);
                            }
                        }
                        "thinking" => {
                            if let Some(text) = block
                                .get("thinking")
                                .or_else(|| block.get("text"))
                                .and_then(Value::as_str)
                            {
                                reasoning.push_str(text);
                            }
                        }
                        _ => {}
                    }
                }
            }
            _ => {}
        }
    }
    if reasoning.is_empty() {
        reasoning = obj
            .get("reasoning")
            .or_else(|| obj.get("reasoning_content"))
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
    }
    let mut tool_calls = Vec::new();
    if let Some(calls) = obj.get("tool_calls").and_then(Value::as_array) {
        for call in calls {
            if let Some(call) = call.as_object() {
                let mut tool_call = ToolCall {
                    id: string(call.get("id")).unwrap_or("").to_string(),
                    ..ToolCall::default()
                };
                if let Some(function) = call.get("function").and_then(Value::as_object) {
                    tool_call.name = string(function.get("name")).unwrap_or("").to_string();
                    tool_call.args = jsonish(function.get("arguments"));
                }
                if !tool_call.name.is_empty() || !tool_call.args.is_empty() {
                    tool_calls.push(tool_call);
                }
            }
        }
    }
    Some(Event {
        role,
        content,
        timestamp,
        reasoning,
        tool_calls,
        tool_call_id: string(obj.get("tool_call_id")).unwrap_or("").to_string(),
        is_error: obj
            .get("is_error")
            .and_then(Value::as_bool)
            .unwrap_or(false),
        model_used: model.to_string(),
        ..Event::default()
    })
}

fn parse_cursor_export(value: &Value) -> Option<Vec<Event>> {
    let mut events = Vec::new();
    if let Some(doc) = value.as_object() {
        add_cursor_prompts(
            first_value(doc, &["aiService.prompts", "prompts"]),
            &mut events,
        );
        add_cursor_generations(
            first_value(doc, &["aiService.generations", "generations"]),
            &mut events,
        );
        add_cursor_composers(
            first_value(doc, &["composer.composerData", "composerData"]).or(Some(value)),
            &mut events,
        );
    } else if value.is_array() {
        add_cursor_prompts(Some(value), &mut events);
        add_cursor_generations(Some(value), &mut events);
    }
    non_empty(events)
}

fn add_cursor_prompts(value: Option<&Value>, events: &mut Vec<Event>) {
    for item in cursor_array(value) {
        let Some(item) = item.as_object() else {
            continue;
        };
        let Some(text) = string(item.get("text")) else {
            continue;
        };
        if text.is_empty() {
            continue;
        }
        events.push(Event {
            role: "user".to_string(),
            content: text.to_string(),
            source_tool: "cursor".to_string(),
            ..Event::default()
        });
    }
}

fn add_cursor_generations(value: Option<&Value>, events: &mut Vec<Event>) {
    for item in cursor_array(value) {
        let Some(item) = item.as_object() else {
            continue;
        };
        let content = string(item.get("textDescription"))
            .or_else(|| string(item.get("description")))
            .or_else(|| string(item.get("text")))
            .or_else(|| string(item.get("type")))
            .unwrap_or("");
        if content.is_empty() {
            continue;
        }
        events.push(Event {
            role: "assistant".to_string(),
            content: content.to_string(),
            timestamp: cursor_timestamp(item.get("unixMs")),
            source_tool: "cursor".to_string(),
            ..Event::default()
        });
    }
}

fn add_cursor_composers(value: Option<&Value>, events: &mut Vec<Event>) {
    let Some(value) = cursor_object(value) else {
        return;
    };
    for item in cursor_array(value.get("allComposers")) {
        let Some(item) = item.as_object() else {
            continue;
        };
        let fallback_ts = cursor_timestamp(first_value(item, &["lastUpdatedAt", "createdAt"]));
        let msg_events = cursor_composer_message_events(item, &fallback_ts);
        if !msg_events.is_empty() {
            events.extend(msg_events);
            continue;
        }
        let mut content = string(item.get("name")).unwrap_or("").to_string();
        let subtitle = string(item.get("subtitle")).unwrap_or("");
        if !subtitle.is_empty() && subtitle != content {
            if !content.is_empty() {
                content.push('\n');
            }
            content.push_str(subtitle);
        }
        if content.is_empty() {
            content = string(item.get("type")).unwrap_or("").to_string();
        }
        if !content.is_empty() {
            events.push(Event {
                role: "assistant".to_string(),
                content,
                timestamp: fallback_ts,
                source_tool: "cursor".to_string(),
                ..Event::default()
            });
        }
    }
}

fn cursor_composer_message_events(composer: &Map<String, Value>, fallback_ts: &str) -> Vec<Event> {
    let mut messages = Vec::new();
    if let Some(conversation) = composer.get("conversation").and_then(Value::as_object) {
        messages.extend(cursor_array(conversation.get("messages")));
    }
    messages.extend(cursor_array(composer.get("messages")));

    let mut events = Vec::new();
    for message in messages {
        let Some(message) = message.as_object() else {
            continue;
        };
        let role = cursor_role(
            string(message.get("role"))
                .or_else(|| string(message.get("speaker")))
                .or_else(|| string(message.get("type")))
                .unwrap_or(""),
        );
        let content = cursor_text(
            first_value(
                message,
                &["text", "content", "message", "markdown", "rawText"],
            )
            .unwrap_or(&Value::Null),
        );
        if role.is_empty() || content.is_empty() {
            continue;
        }
        let mut timestamp = cursor_timestamp(first_value(
            message,
            &["unixMs", "timestamp", "createdAt", "lastUpdatedAt"],
        ));
        if timestamp.is_empty() {
            timestamp = fallback_ts.to_string();
        }
        events.push(Event {
            role,
            content,
            timestamp,
            source_tool: "cursor".to_string(),
            ..Event::default()
        });
    }
    events
}

fn parse_gemini_value(value: &Value) -> Option<Vec<Event>> {
    let mut events = Vec::new();
    let mut model = "unknown".to_string();
    parse_gemini_object(value, &mut model, &mut events);
    if events.is_empty() {
        if let Some(arr) = value.as_array() {
            parse_gemini_array(arr, "", &model, &mut events);
        }
    }
    non_empty(events)
}

fn parse_gemini_object(value: &Value, model: &mut String, events: &mut Vec<Event>) {
    let Some(obj) = value.as_object() else {
        return;
    };
    for key in ["modelVersion", "model", "modelId"] {
        if let Some(value) = string(obj.get(key)) {
            if !value.is_empty() {
                *model = value.to_string();
            }
        }
    }
    for key in ["usageMetadata", "usage", "tokenUsage"] {
        if let Some(usage) = obj.get(key).and_then(gemini_usage) {
            events.push(Event {
                role: "meta".to_string(),
                usage,
                model_used: model.clone(),
                source_tool: "gemini_cli".to_string(),
                ..Event::default()
            });
        }
    }
    let fallback_ts = string(obj.get("timestamp")).unwrap_or("");
    if let Some(contents) = obj.get("contents").and_then(Value::as_array) {
        parse_gemini_array(contents, fallback_ts, model, events);
    }
    for key in [
        "history",
        "messages",
        "conversation",
        "clientHistory",
        "chatHistory",
    ] {
        if let Some(contents) = obj.get(key).and_then(Value::as_array) {
            parse_gemini_array(contents, fallback_ts, model, events);
        }
    }
    if let Some(candidates) = obj.get("candidates").and_then(Value::as_array) {
        for candidate in candidates {
            if let Some(content) = candidate.get("content").and_then(Value::as_object) {
                parse_gemini_content_object(content, fallback_ts, model, events);
            }
        }
    }
    if obj.contains_key("parts") {
        parse_gemini_content_object(obj, fallback_ts, model, events);
    }
    for key in ["checkpoint", "session", "chat"] {
        if let Some(nested) = obj.get(key) {
            parse_gemini_object(nested, model, events);
        }
    }
}

fn parse_gemini_array(items: &[Value], fallback_ts: &str, model: &str, events: &mut Vec<Event>) {
    for item in items {
        if let Some(item) = item.as_object() {
            parse_gemini_content_object(item, fallback_ts, model, events);
        }
    }
}

fn parse_gemini_content_object(
    obj: &Map<String, Value>,
    fallback_ts: &str,
    model: &str,
    events: &mut Vec<Event>,
) {
    let role = gemini_role(string(obj.get("role")).unwrap_or(""));
    let ts = string(obj.get("timestamp")).unwrap_or(fallback_ts);
    let Some(parts) = obj.get("parts").and_then(Value::as_array) else {
        return;
    };
    for part in parts {
        let Some(part) = part.as_object() else {
            continue;
        };
        if let Some(text) = string(part.get("text")) {
            if part
                .get("thought")
                .and_then(Value::as_bool)
                .unwrap_or(false)
            {
                events.push(Event {
                    role: "assistant".to_string(),
                    reasoning: text.to_string(),
                    timestamp: ts.to_string(),
                    model_used: model.to_string(),
                    source_tool: "gemini_cli".to_string(),
                    ..Event::default()
                });
            } else {
                events.push(Event {
                    role: role.clone(),
                    content: text.to_string(),
                    timestamp: ts.to_string(),
                    model_used: model.to_string(),
                    source_tool: "gemini_cli".to_string(),
                    ..Event::default()
                });
            }
        }
        if let Some(function_call) = part.get("functionCall").and_then(Value::as_object) {
            let name = string(function_call.get("name")).unwrap_or("").to_string();
            let args = jsonish(function_call.get("args"));
            if !name.is_empty() || !args.is_empty() {
                events.push(Event {
                    role: "assistant".to_string(),
                    timestamp: ts.to_string(),
                    tool_calls: vec![ToolCall {
                        name,
                        args,
                        ..ToolCall::default()
                    }],
                    model_used: model.to_string(),
                    source_tool: "gemini_cli".to_string(),
                    ..Event::default()
                });
            }
        }
        if let Some(function_response) = part.get("functionResponse").and_then(Value::as_object) {
            let name = string(function_response.get("name"))
                .unwrap_or("")
                .to_string();
            let content = jsonish(function_response.get("response"));
            if !name.is_empty() || !content.is_empty() {
                events.push(Event {
                    role: "tool".to_string(),
                    content,
                    timestamp: ts.to_string(),
                    tool_call_id: name,
                    source_tool: "gemini_cli".to_string(),
                    ..Event::default()
                });
            }
        }
    }
}

#[derive(Debug)]
struct OpenCodeRecord {
    path: PathBuf,
    doc: Map<String, Value>,
}

fn parse_opencode_storage_value(path: &str, value: &Value) -> Option<Vec<Event>> {
    let doc = value.as_object()?;
    if !is_opencode_storage_session_doc(path, doc) {
        return None;
    }
    parse_opencode_storage_session(path, doc).ok()
}

fn is_opencode_storage_session_doc(path: &str, doc: &Map<String, Value>) -> bool {
    is_opencode_storage_session_file(path)
        && !string(doc.get("id")).unwrap_or("").is_empty()
        && !string(doc.get("projectID")).unwrap_or("").is_empty()
}

fn is_opencode_storage_session_file(path: &str) -> bool {
    let Some(rel) = opencode_storage_rel(path) else {
        return false;
    };
    let parts = rel.split('/').collect::<Vec<_>>();
    parts.len() == 3 && parts[0] == "session" && parts[2].ends_with(".json")
}

fn opencode_storage_rel(path: &str) -> Option<String> {
    if let Ok(root) = std::env::var("OPENCODE_DATA_DIR") {
        if !root.is_empty() {
            if let Some(rel) = rel_from_root(Path::new(&root), Path::new(path)) {
                return Some(rel);
            }
        }
    }

    let clean = path_slash(Path::new(path));
    let marker = "/opencode/storage";
    let idx = clean.rfind(marker)?;
    let start = idx + marker.len();
    if clean.len() == start {
        return Some(String::new());
    }
    if clean.as_bytes().get(start) != Some(&b'/') {
        return None;
    }
    Some(clean[start + 1..].trim_matches('/').to_string())
}

fn rel_from_root(root: &Path, path: &Path) -> Option<String> {
    let abs_root = std::fs::canonicalize(root).ok()?;
    let abs_path = std::fs::canonicalize(path).ok()?;
    let rel = abs_path.strip_prefix(abs_root).ok()?;
    if rel.as_os_str().is_empty() {
        Some(String::new())
    } else {
        Some(path_slash(rel))
    }
}

fn parse_opencode_storage_session(
    path: &str,
    session: &Map<String, Value>,
) -> anyhow::Result<Vec<Event>> {
    let session_id = string(session.get("id")).unwrap_or("");
    if session_id.is_empty() {
        bail!("opencode: missing session id");
    }
    let storage_root = opencode_storage_root_from_session_file(path)
        .with_context(|| format!("opencode: unsupported storage path {path}"))?;
    let mut messages = read_opencode_records(&storage_root.join("message").join(session_id));
    if messages.is_empty() {
        bail!("opencode: no messages found for session {}", session_id);
    }
    sort_opencode_records(&mut messages);

    let mut model = opencode_session_model(session);
    let mut usage = BTreeMap::new();
    // rm-619: parse-time disclosure counters (reasoning dropped beside
    // present totals) — attached to the session meta event below.
    let mut counters = BTreeMap::new();
    let mut body = Vec::new();
    for msg in messages {
        if model == "unknown" {
            let msg_model = opencode_message_model(&msg.doc);
            if !msg_model.is_empty() {
                model = msg_model;
            }
        }
        let message_had_usage =
            add_opencode_tokens(&mut usage, msg.doc.get("tokens"), &mut counters);
        let (events, part_usage) = parse_opencode_message(
            &storage_root,
            &msg.doc,
            &model,
            message_had_usage,
            &mut counters,
        );
        add_usage(&mut usage, &part_usage);
        body.extend(events);
    }
    if body.is_empty() {
        bail!("opencode: no parseable events for session {}", session_id);
    }

    let mut events = Vec::new();
    if model != "unknown" || usage_has_values(&usage) {
        events.push(Event {
            role: "meta".to_string(),
            model_used: model,
            source_tool: "opencode".to_string(),
            usage: if usage_has_values(&usage) {
                usage
            } else {
                BTreeMap::new()
            },
            disclosure_counters: counters,
            ..Event::default()
        });
    }
    events.extend(body);
    Ok(events)
}

fn opencode_storage_root_from_session_file(path: &str) -> Option<PathBuf> {
    if !is_opencode_storage_session_file(path) {
        return None;
    }
    Path::new(path)
        .parent()
        .and_then(Path::parent)
        .and_then(Path::parent)
        .map(Path::to_path_buf)
}

fn read_opencode_records(dir: &Path) -> Vec<OpenCodeRecord> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut records = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() || path.extension().and_then(|ext| ext.to_str()) != Some("json") {
            continue;
        }
        let Ok(raw) = std::fs::read_to_string(&path) else {
            continue;
        };
        let Ok(Value::Object(doc)) = serde_json::from_str::<Value>(&raw) else {
            continue;
        };
        records.push(OpenCodeRecord { path, doc });
    }
    records
}

fn sort_opencode_records(records: &mut [OpenCodeRecord]) {
    records.sort_by(
        |a, b| match (opencode_record_time(&a.doc), opencode_record_time(&b.doc)) {
            (Some(left), Some(right)) if left != right => left.cmp(&right),
            _ => a.path.cmp(&b.path),
        },
    );
}

fn parse_opencode_message(
    storage_root: &Path,
    msg: &Map<String, Value>,
    model: &str,
    message_had_usage: bool,
    counters: &mut BTreeMap<String, i64>,
) -> (Vec<Event>, BTreeMap<String, i64>) {
    let role = string(msg.get("role")).unwrap_or("");
    let ts = opencode_time_from_map(msg.get("time"), &["created", "start"]);
    let msg_id = string(msg.get("id")).unwrap_or("");
    let mut parts = read_opencode_records(&storage_root.join("part").join(msg_id));
    sort_opencode_records(&mut parts);

    let mut events = Vec::new();
    let mut part_usage = BTreeMap::new();
    for record in parts.iter() {
        let part = &record.doc;
        let part_ts = opencode_part_timestamp(part, &ts);
        match string(part.get("type")).unwrap_or("") {
            "text" => {
                let text = string(part.get("text")).unwrap_or("");
                if !text.is_empty() && (role == "user" || role == "assistant") {
                    events.push(Event {
                        role: role.to_string(),
                        content: text.to_string(),
                        timestamp: part_ts,
                        model_used: model.to_string(),
                        source_tool: "opencode".to_string(),
                        ..Event::default()
                    });
                }
            }
            "reasoning" => {
                let text = string(part.get("text")).unwrap_or("");
                if !text.is_empty() {
                    events.push(Event {
                        role: "assistant".to_string(),
                        reasoning: text.to_string(),
                        timestamp: part_ts,
                        model_used: model.to_string(),
                        source_tool: "opencode".to_string(),
                        ..Event::default()
                    });
                }
            }
            "tool" => events.extend(opencode_tool_events(part, &part_ts, model)),
            "step-finish" if !message_had_usage => {
                add_opencode_tokens(&mut part_usage, part.get("tokens"), counters);
            }
            _ => {}
        }
    }
    if parts.is_empty() {
        let text = string(msg.get("content")).unwrap_or("");
        if !text.is_empty() && (role == "user" || role == "assistant") {
            events.push(Event {
                role: role.to_string(),
                content: text.to_string(),
                timestamp: ts,
                model_used: model.to_string(),
                source_tool: "opencode".to_string(),
                ..Event::default()
            });
        }
    }
    (events, part_usage)
}

fn opencode_tool_events(part: &Map<String, Value>, ts: &str, model: &str) -> Vec<Event> {
    let state = part.get("state").and_then(Value::as_object);
    let status = state
        .and_then(|state| string(state.get("status")))
        .unwrap_or("");
    let call_id = string(part.get("callID"))
        .or_else(|| string(part.get("id")))
        .unwrap_or("")
        .to_string();
    let name = string(part.get("tool"))
        .or_else(|| string(part.get("name")))
        .unwrap_or("")
        .to_string();
    let input = jsonish(state.and_then(|state| state.get("input")));
    let mut call_ts = opencode_time_from_map(state.and_then(|state| state.get("time")), &["start"]);
    if call_ts.is_empty() {
        call_ts = ts.to_string();
    }

    let mut events = vec![Event {
        role: "assistant".to_string(),
        timestamp: call_ts.clone(),
        tool_calls: vec![ToolCall {
            id: call_id.clone(),
            name,
            args: input,
        }],
        model_used: model.to_string(),
        source_tool: "opencode".to_string(),
        ..Event::default()
    }];

    let mut output = jsonish(state.and_then(|state| state.get("output")));
    let mut is_error = status == "error";
    if output.is_empty() {
        output = jsonish(state.and_then(|state| state.get("error")));
        is_error = is_error || !output.is_empty();
    }
    if !output.is_empty() {
        let mut result_ts =
            opencode_time_from_map(state.and_then(|state| state.get("time")), &["end"]);
        if result_ts.is_empty() {
            result_ts = call_ts;
        }
        events.push(Event {
            role: "tool".to_string(),
            content: output,
            timestamp: result_ts,
            tool_call_id: call_id,
            is_error,
            source_tool: "opencode".to_string(),
            ..Event::default()
        });
    }
    events
}

fn opencode_part_timestamp(part: &Map<String, Value>, fallback: &str) -> String {
    let ts = opencode_time_from_map(part.get("time"), &["start", "created"]);
    if !ts.is_empty() {
        return ts;
    }
    if let Some(state) = part.get("state").and_then(Value::as_object) {
        let ts = opencode_time_from_map(state.get("time"), &["start", "created"]);
        if !ts.is_empty() {
            return ts;
        }
    }
    fallback.to_string()
}

fn opencode_record_time(doc: &Map<String, Value>) -> Option<chrono::DateTime<chrono::Utc>> {
    opencode_time_value(doc.get("time"), &["start", "created"]).or_else(|| {
        doc.get("state")
            .and_then(Value::as_object)
            .and_then(|state| opencode_time_value(state.get("time"), &["start", "created"]))
    })
}

fn opencode_time_from_map(raw: Option<&Value>, keys: &[&str]) -> String {
    opencode_time_value(raw, keys)
        .map(|ts| ts.to_rfc3339_opts(chrono::SecondsFormat::AutoSi, true))
        .unwrap_or_default()
}

fn opencode_time_value(
    raw: Option<&Value>,
    keys: &[&str],
) -> Option<chrono::DateTime<chrono::Utc>> {
    match raw? {
        Value::Object(obj) => keys
            .iter()
            .find_map(|key| opencode_parse_time(obj.get(*key))),
        other => opencode_parse_time(Some(other)),
    }
}

fn opencode_parse_time(raw: Option<&Value>) -> Option<chrono::DateTime<chrono::Utc>> {
    match raw? {
        Value::Number(number) => number.as_f64().and_then(opencode_unix_time),
        Value::String(text) => {
            // rm-502: the string arms fold onto lib.rs `parse_ts`, the
            // single source of timestamp truth — this lane used to carry
            // an independent naive copy that dropped fractional seconds.
            // The fold additively widens the accepted space to naive
            // stamps WITH fractions; the unix-float fallback below stays
            // lane-specific (parse_ts is string-ISO only).
            crate::parse_ts(text).or_else(|| text.parse::<f64>().ok().and_then(opencode_unix_time))
        }
        _ => None,
    }
}

fn opencode_unix_time(value: f64) -> Option<chrono::DateTime<chrono::Utc>> {
    if value <= 0.0 {
        return None;
    }
    if value > 1e12 {
        let ms = value as i64;
        let secs = ms / 1000;
        let nsec = ((ms % 1000) * 1_000_000) as u32;
        chrono::DateTime::<chrono::Utc>::from_timestamp(secs, nsec)
    } else {
        chrono::DateTime::<chrono::Utc>::from_timestamp(value as i64, 0)
    }
}

fn opencode_session_model(session: &Map<String, Value>) -> String {
    session
        .get("model")
        .and_then(Value::as_object)
        .and_then(|model| {
            string(model.get("id"))
                .or_else(|| string(model.get("modelID")))
                .map(str::to_string)
        })
        .filter(|model| !model.is_empty())
        .unwrap_or_else(|| "unknown".to_string())
}

fn opencode_message_model(msg: &Map<String, Value>) -> String {
    string(msg.get("modelID"))
        .or_else(|| string(msg.get("model")))
        .unwrap_or("")
        .to_string()
}

fn add_opencode_tokens(
    usage: &mut BTreeMap<String, i64>,
    raw: Option<&Value>,
    counters: &mut BTreeMap<String, i64>,
) -> bool {
    let Some(tokens) = raw.and_then(Value::as_object) else {
        return false;
    };
    add_usage_value(usage, "input_tokens", tokens.get("input"));
    add_usage_value(usage, "output_tokens", tokens.get("output"));
    if let Some(cache) = tokens.get("cache").and_then(Value::as_object) {
        add_usage_value(usage, "cache_read_input_tokens", cache.get("read"));
        add_usage_value(usage, "cache_creation_input_tokens", cache.get("write"));
    }
    // rm-619: opencode emits `tokens.reasoning` as a SIBLING of the
    // total-style counts (upstream #312 fix-table row for opencode), and
    // the total block can be 0/absent while reasoning carries the whole
    // spend — that reasoning previously landed nowhere. When the totals
    // are 0/absent the reasoning block IS the message's entire output-class
    // spend: fold it into output_tokens (billed at the output rate — the
    // CU-20 gemini thinking-token precedent) and break it out on
    // reasoning_tokens. When the totals are present the wire does not say
    // whether output already includes the reasoning, so folding would risk
    // a double count — the value stays out and the drop is disclosed
    // instead of guessed at (rm-436 family: never silently vanish).
    if let Some(reasoning) = tokens
        .get("reasoning")
        .and_then(number_as_i64)
        .filter(|value| *value > 0)
    {
        let totals_present = ["input", "output"].iter().any(|key| {
            tokens
                .get(*key)
                .and_then(number_as_i64)
                .is_some_and(|value| value > 0)
        });
        if totals_present {
            *counters
                .entry(disclosure_key(
                    "opencode_reasoning",
                    "present_beside_totals",
                ))
                .or_insert(0) += 1;
        } else {
            let output = usage.entry("output_tokens".to_string()).or_insert(0);
            *output = output.saturating_add(reasoning);
            usage.insert("reasoning_tokens".to_string(), reasoning);
        }
    }
    true
}

fn add_usage(dst: &mut BTreeMap<String, i64>, src: &BTreeMap<String, i64>) {
    for (key, value) in src {
        let slot = dst.entry(key.clone()).or_insert(0);
        // rm-046: hostile journals can repeat i64::MAX-sized counts;
        // plain `+=` panics in debug and wraps negative in release,
        // where the >0 consumers silently drop the total.
        *slot = (*slot).saturating_add(*value);
    }
}

fn add_usage_value(usage: &mut BTreeMap<String, i64>, key: &str, raw: Option<&Value>) {
    if let Some(value) = raw.and_then(number_as_i64).filter(|value| *value > 0) {
        let slot = usage.entry(key.to_string()).or_insert(0);
        // rm-046: see add_usage — clamp instead of panicking/wrapping.
        *slot = (*slot).saturating_add(value);
    }
}

fn usage_has_values(usage: &BTreeMap<String, i64>) -> bool {
    usage.values().any(|value| *value > 0)
}

fn path_slash(path: &Path) -> String {
    path.components()
        .map(|part| part.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}

fn parse_cline_value(value: &Value, model: &str) -> Option<Vec<Event>> {
    let messages = match value {
        Value::Array(items) => items.as_slice(),
        Value::Object(obj) => obj
            .get("messages")
            .or_else(|| obj.get("conversation"))
            .or_else(|| obj.get("history"))
            .and_then(Value::as_array)
            .map(Vec::as_slice)?,
        _ => return None,
    };
    let mut events = Vec::new();
    for message in messages {
        let Some(message) = message.as_object() else {
            continue;
        };
        let role = cline_role(
            string(message.get("role"))
                .or_else(|| string(message.get("speaker")))
                .or_else(|| string(message.get("author")))
                .unwrap_or(""),
        );
        let ts = cline_timestamp(
            first_value(message, &["timestamp", "ts", "createdAt", "created_at"])
                .unwrap_or(&Value::Null),
        );
        let before = events.len();
        if let Some(content) = message.get("content") {
            cline_content_events(&role, &ts, model, content, &mut events);
        }
        if events.len() == before {
            let text = string(message.get("text"))
                .or_else(|| string(message.get("message")))
                .unwrap_or("");
            if !role.is_empty() && !text.is_empty() {
                events.push(Event {
                    role,
                    content: text.to_string(),
                    timestamp: ts,
                    model_used: model.to_string(),
                    source_tool: "cline".to_string(),
                    ..Event::default()
                });
            }
        }
    }
    non_empty(events)
}

fn cline_content_events(
    role: &str,
    ts: &str,
    model: &str,
    content: &Value,
    events: &mut Vec<Event>,
) {
    match content {
        Value::String(text) if !role.is_empty() && !text.is_empty() => {
            events.push(Event {
                role: role.to_string(),
                content: text.to_string(),
                timestamp: ts.to_string(),
                model_used: model.to_string(),
                source_tool: "cline".to_string(),
                ..Event::default()
            });
        }
        Value::Array(blocks) => {
            for block in blocks {
                let Some(block) = block.as_object() else {
                    continue;
                };
                match string(block.get("type")).unwrap_or("") {
                    "text" => {
                        if let Some(text) = string(block.get("text")) {
                            if !role.is_empty() && !text.is_empty() {
                                events.push(Event {
                                    role: role.to_string(),
                                    content: text.to_string(),
                                    timestamp: ts.to_string(),
                                    model_used: model.to_string(),
                                    source_tool: "cline".to_string(),
                                    ..Event::default()
                                });
                            }
                        }
                    }
                    "thinking" => {
                        events.push(Event {
                            role: "assistant".to_string(),
                            reasoning: string(block.get("thinking"))
                                .or_else(|| string(block.get("text")))
                                .unwrap_or("")
                                .to_string(),
                            timestamp: ts.to_string(),
                            model_used: model.to_string(),
                            source_tool: "cline".to_string(),
                            ..Event::default()
                        });
                    }
                    "tool_use" => {
                        events.push(Event {
                            role: "assistant".to_string(),
                            tool_calls: vec![ToolCall {
                                id: string(block.get("id"))
                                    .or_else(|| string(block.get("tool_use_id")))
                                    .unwrap_or("")
                                    .to_string(),
                                name: string(block.get("name"))
                                    .or_else(|| string(block.get("tool_name")))
                                    .unwrap_or("")
                                    .to_string(),
                                args: jsonish(
                                    block.get("input").or_else(|| block.get("arguments")),
                                ),
                            }],
                            timestamp: ts.to_string(),
                            model_used: model.to_string(),
                            source_tool: "cline".to_string(),
                            ..Event::default()
                        });
                    }
                    "tool_result" => {
                        events.push(Event {
                            role: "tool".to_string(),
                            content: cline_tool_result_content(block),
                            timestamp: ts.to_string(),
                            tool_call_id: string(block.get("tool_use_id"))
                                .or_else(|| string(block.get("tool_call_id")))
                                .or_else(|| string(block.get("id")))
                                .unwrap_or("")
                                .to_string(),
                            is_error: block
                                .get("is_error")
                                .and_then(Value::as_bool)
                                .unwrap_or(false),
                            model_used: model.to_string(),
                            source_tool: "cline".to_string(),
                            ..Event::default()
                        });
                    }
                    _ => {}
                }
            }
        }
        _ => {}
    }
}

fn cursor_array(value: Option<&Value>) -> Vec<Value> {
    match value {
        Some(Value::Array(items)) => items.clone(),
        Some(Value::String(raw)) if !raw.is_empty() => {
            serde_json::from_str::<Vec<Value>>(raw).unwrap_or_default()
        }
        _ => Vec::new(),
    }
}

fn cursor_object(value: Option<&Value>) -> Option<Map<String, Value>> {
    match value {
        Some(Value::Object(obj)) => Some(obj.clone()),
        Some(Value::String(raw)) if !raw.is_empty() => {
            serde_json::from_str::<Map<String, Value>>(raw).ok()
        }
        _ => None,
    }
}

fn cursor_text(value: &Value) -> String {
    match value {
        Value::String(text) => text.clone(),
        Value::Array(items) => items
            .iter()
            .filter_map(|item| {
                item.as_object()
                    .and_then(|obj| string(obj.get("text")))
                    .map(str::to_string)
            })
            .collect::<Vec<_>>()
            .join("\n"),
        Value::Object(obj) => string(obj.get("text"))
            .or_else(|| string(obj.get("content")))
            .unwrap_or("")
            .to_string(),
        _ => String::new(),
    }
}

fn cursor_role(role: &str) -> String {
    match role.to_ascii_lowercase().as_str() {
        "human" | "user" => "user".to_string(),
        "ai" | "assistant" | "bot" | "model" => "assistant".to_string(),
        "tool" => "tool".to_string(),
        other => other.to_string(),
    }
}

fn gemini_role(role: &str) -> String {
    match role {
        "model" => "assistant".to_string(),
        other => other.to_string(),
    }
}

fn cline_role(role: &str) -> String {
    match role.to_ascii_lowercase().as_str() {
        "human" => "user".to_string(),
        "ai" | "bot" => "assistant".to_string(),
        other => other.to_string(),
    }
}

fn cursor_timestamp(value: Option<&Value>) -> String {
    match value {
        Some(Value::String(text)) => text.to_string(),
        Some(Value::Number(number)) => {
            let Some(ms) = number
                .as_i64()
                .or_else(|| number.as_u64().map(|n| n as i64))
            else {
                return String::new();
            };
            timestamp_millis(ms)
        }
        _ => String::new(),
    }
}

fn cline_timestamp(value: &Value) -> String {
    match value {
        Value::String(text) if text.is_empty() => String::new(),
        Value::String(text) if text.parse::<i64>().is_ok() => {
            cline_unix_timestamp(text.parse::<i64>().unwrap_or(0))
        }
        Value::String(text) => text.clone(),
        Value::Number(number) => cline_unix_timestamp(
            number
                .as_i64()
                .or_else(|| number.as_u64().map(|n| n as i64))
                .unwrap_or(0),
        ),
        _ => String::new(),
    }
}

fn cline_unix_timestamp(value: i64) -> String {
    if value > 1_000_000_000_000 {
        timestamp_millis(value)
    } else if value > 1_000_000_000 {
        chrono::DateTime::<chrono::Utc>::from_timestamp(value, 0)
            .map(|ts| ts.to_rfc3339_opts(chrono::SecondsFormat::Secs, true))
            .unwrap_or_default()
    } else {
        String::new()
    }
}

fn timestamp_millis(ms: i64) -> String {
    let secs = ms / 1000;
    let nsec = ((ms % 1000) * 1_000_000) as u32;
    chrono::DateTime::<chrono::Utc>::from_timestamp(secs, nsec)
        .map(|ts| ts.to_rfc3339_opts(chrono::SecondsFormat::Millis, true))
        .unwrap_or_default()
}

fn timestamp_millis_nanos(ms: i64) -> String {
    let secs = ms / 1000;
    let nsec = ((ms % 1000) * 1_000_000) as u32;
    chrono::DateTime::<chrono::Utc>::from_timestamp(secs, nsec)
        .map(|ts| ts.to_rfc3339_opts(chrono::SecondsFormat::Nanos, true))
        .unwrap_or_default()
}

fn gemini_usage(value: &Value) -> Option<BTreeMap<String, i64>> {
    let obj = value.as_object()?;
    let mut usage = BTreeMap::new();
    usage.insert(
        "input_tokens".to_string(),
        first_number(
            obj,
            &[
                "promptTokenCount",
                "inputTokenCount",
                "inputTokens",
                "input_tokens",
                "prompt_tokens",
            ],
        ),
    );
    usage.insert(
        "output_tokens".to_string(),
        first_number(
            obj,
            &[
                "candidatesTokenCount",
                "outputTokenCount",
                "outputTokens",
                "output_tokens",
                "completion_tokens",
            ],
        )
        .saturating_add(first_number(
            obj,
            &[
                "thoughtsTokenCount",
                "thinkingTokenCount",
                "thinking_tokens",
                "reasoning_tokens",
            ],
        )),
    );
    // Thinking tokens are billed at the output rate but reported
    // separately by the API (Gemini usageMetadata.thoughtsTokenCount);
    // folded above, broken out here for the audit (pass-9 CU-20).
    let reasoning = first_number(
        obj,
        &[
            "thoughtsTokenCount",
            "thinkingTokenCount",
            "thinking_tokens",
            "reasoning_tokens",
        ],
    );
    if reasoning > 0 {
        usage.insert("reasoning_tokens".to_string(), reasoning);
    }
    usage.insert(
        "cache_read_input_tokens".to_string(),
        first_number(
            obj,
            &[
                "cachedContentTokenCount",
                "cacheReadInputTokens",
                "cache_read_input_tokens",
            ],
        ),
    );
    Some(usage)
}

/// Compare two timestamp strings and return the later one; ties
/// keep the candidate. Falls back to lexicographic order only when
/// neither stamp parses (rm-556: shutdown/checkpoint records both end
/// sessions, and the latest activity — not the freshest parse — must
/// win). rm-502 (integration of run 6aaf51aa): "parses" is decided by
/// the shared lenient arm (lib.rs `parse_ts`), so naive-ISO record
/// stamps compare chronologically instead of falling to the string
/// arm — the accepted-stamp space stays identical to ingest's.
fn later_rfc3339(current: &str, candidate: &str) -> String {
    let candidate_wins = match (crate::parse_ts(current), crate::parse_ts(candidate)) {
        (Some(a), Some(b)) => b >= a,
        (None, Some(_)) => true,
        (Some(_), None) => false,
        (None, None) => candidate >= current,
    };
    if candidate_wins {
        candidate.to_string()
    } else {
        current.to_string()
    }
}

fn first_number(obj: &Map<String, Value>, keys: &[&str]) -> i64 {
    keys.iter()
        .find_map(|key| obj.get(*key))
        .and_then(number_as_i64)
        .unwrap_or(0)
}

pub(crate) fn number_as_i64(value: &Value) -> Option<i64> {
    match value {
        Value::Number(number) => number.as_i64().or_else(|| {
            number
                .as_u64()
                // Clamp instead of wrapping: u64 values above i64::MAX
                // previously cast to a negative i64 through `as i64`,
                // letting corrupt logs flip token totals negative.
                .map(|n| n.min(i64::MAX as u64) as i64)
                .or_else(|| {
                    number
                        .as_f64()
                        // `as` casts from f64 to i64 saturate, so absurd
                        // magnitudes (e.g. 1e300) land on the i64 bounds
                        // instead of wrapping; downstream aggregation is
                        // saturating as well.
                        .map(|n| n as i64)
                })
        }),
        Value::String(text) => numeric_string_as_i64(text),
        _ => None,
    }
}

/// rm-342: usage counters serialized as strings ("100") must contribute
/// exactly like the identical JSON Number. Integer strings parse
/// directly; float-form ("100.5", "1e3"), padded, and beyond-i64::MAX
/// strings coerce and clamp the same way the Number arm does instead of
/// silently dropping. Non-numeric strings ("true", "lots") stay skipped
/// — they carry no count to honor.
fn numeric_string_as_i64(text: &str) -> Option<i64> {
    let trimmed = text.trim();
    if let Ok(value) = trimmed.parse::<i64>() {
        return Some(value);
    }
    // `as` casts from f64 to i64 saturate, mirroring the Number arm for
    // magnitudes beyond i64::MAX.
    trimmed.parse::<f64>().ok().map(|value| value as i64)
}

fn boolish(value: Option<&Value>) -> bool {
    match value {
        Some(Value::Bool(value)) => *value,
        Some(Value::String(value)) => value == "true",
        _ => false,
    }
}

fn non_empty_usage(usage: BTreeMap<String, i64>) -> Option<BTreeMap<String, i64>> {
    if usage.values().any(|value| *value > 0) {
        Some(usage)
    } else {
        None
    }
}

fn cline_tool_result_content(block: &Map<String, Value>) -> String {
    if let Some(text) = string(block.get("content")) {
        return text.to_string();
    }
    jsonish(block.get("content"))
}

fn kimi_message_events(message: &Map<String, Value>, model: &str, events: &mut Vec<Event>) {
    let role = string(message.get("role")).unwrap_or("");
    let ts = string(message.get("timestamp")).unwrap_or("").to_string();
    match message.get("content") {
        Some(Value::String(text)) => {
            if role == "tool" {
                events.push(Event {
                    role: "tool".to_string(),
                    content: text.to_string(),
                    timestamp: ts,
                    tool_call_id: string(message.get("tool_call_id"))
                        .unwrap_or("")
                        .to_string(),
                    is_error: message
                        .get("is_error")
                        .and_then(Value::as_bool)
                        .unwrap_or(false),
                    source_tool: "kimi_cli".to_string(),
                    ..Event::default()
                });
            } else {
                events.push(Event {
                    role: role.to_string(),
                    content: text.to_string(),
                    timestamp: ts,
                    source_tool: "kimi_cli".to_string(),
                    ..Event::default()
                });
            }
        }
        Some(Value::Array(blocks)) => {
            for block in blocks {
                let Some(block) = block.as_object() else {
                    continue;
                };
                match string(block.get("type")).unwrap_or("") {
                    "text" => events.push(Event {
                        role: role.to_string(),
                        content: string(block.get("text")).unwrap_or("").to_string(),
                        timestamp: ts.clone(),
                        source_tool: "kimi_cli".to_string(),
                        ..Event::default()
                    }),
                    "thinking" => events.push(Event {
                        role: "assistant".to_string(),
                        reasoning: string(block.get("thinking"))
                            .or_else(|| string(block.get("text")))
                            .unwrap_or("")
                            .to_string(),
                        redacted: block
                            .get("redacted")
                            .and_then(Value::as_bool)
                            .unwrap_or(false),
                        timestamp: ts.clone(),
                        source_tool: "kimi_cli".to_string(),
                        ..Event::default()
                    }),
                    "tool_use" => events.push(Event {
                        role: "assistant".to_string(),
                        tool_calls: vec![ToolCall {
                            id: string(block.get("id")).unwrap_or("").to_string(),
                            name: string(block.get("name"))
                                .or_else(|| {
                                    block
                                        .get("function")
                                        .and_then(Value::as_object)
                                        .and_then(|function| string(function.get("name")))
                                })
                                .unwrap_or("")
                                .to_string(),
                            args: jsonish(block.get("input").or_else(|| {
                                block.get("arguments").or_else(|| {
                                    block
                                        .get("function")
                                        .and_then(Value::as_object)
                                        .and_then(|function| function.get("arguments"))
                                })
                            })),
                        }],
                        timestamp: ts.clone(),
                        source_tool: "kimi_cli".to_string(),
                        ..Event::default()
                    }),
                    "tool_result" => events.push(Event {
                        role: "tool".to_string(),
                        timestamp: ts.clone(),
                        tool_call_id: string(block.get("tool_use_id"))
                            .or_else(|| string(block.get("tool_call_id")))
                            .unwrap_or("")
                            .to_string(),
                        content: tool_result_content(block),
                        is_error: block
                            .get("is_error")
                            .and_then(Value::as_bool)
                            .unwrap_or(false),
                        source_tool: "kimi_cli".to_string(),
                        ..Event::default()
                    }),
                    _ => {}
                }
            }
        }
        _ => {
            if let Some(tool_calls) = message.get("tool_calls").and_then(Value::as_array) {
                let calls = tool_calls
                    .iter()
                    .filter_map(|call| {
                        let call = call.as_object()?;
                        let function = call.get("function").and_then(Value::as_object);
                        Some(ToolCall {
                            id: string(call.get("id")).unwrap_or("").to_string(),
                            name: function
                                .and_then(|function| string(function.get("name")))
                                .unwrap_or("")
                                .to_string(),
                            args: jsonish(function.and_then(|function| function.get("arguments"))),
                        })
                    })
                    .collect::<Vec<_>>();
                if !calls.is_empty() {
                    events.push(Event {
                        role: role.to_string(),
                        tool_calls: calls,
                        timestamp: ts,
                        source_tool: "kimi_cli".to_string(),
                        ..Event::default()
                    });
                }
            }
        }
    }
    let _ = model;
}

#[cfg(test)] // test-only since rm-526 moved every production caller to
             // `jsonl_objects_counted`, which discloses the drops instead of
             // silently filter-mapping them away
fn jsonl_objects(raw: &str) -> impl Iterator<Item = Map<String, Value>> + '_ {
    raw.lines()
        .filter_map(|line| parse_jsonl_value_lenient(line.trim()))
        .filter_map(|value| match value {
            Value::Object(obj) => Some(obj),
            _ => None,
        })
}

/// Counting sibling of [`jsonl_objects`] (rm-526): returns the objects
/// plus a `line_skips` map naming every line the iterator dropped —
/// `unparseable_line` for lines that survive neither the strict nor the
/// lenient parse (a torn tail), `non_object_line` for bare JSON values
/// that parse but are not objects. Empty lines stay skipped silently,
/// matching the generic fallback's `parse_jsonl_session`.
fn jsonl_objects_counted(raw: &str) -> (Vec<Map<String, Value>>, BTreeMap<String, usize>) {
    let mut objs = Vec::new();
    let mut skips: BTreeMap<String, usize> = BTreeMap::new();
    for line in raw.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        match parse_jsonl_value_lenient(line) {
            Some(Value::Object(obj)) => objs.push(obj),
            Some(_) => *skips.entry("non_object_line".to_string()).or_insert(0) += 1,
            None => *skips.entry("unparseable_line".to_string()).or_insert(0) += 1,
        }
    }
    (objs, skips)
}

/// Lenient single-line JSONL parse: strict first, then lone-surrogate
/// repair. Shared by the format detectors and the generic-JSONL fallback
/// (`parse_jsonl_session`) so a recoverable line is never silently
/// dropped (pass-7 P7-1).
pub(crate) fn parse_jsonl_value_lenient(line: &str) -> Option<Value> {
    serde_json::from_str::<Value>(line)
        .ok()
        .or_else(|| repair_lone_surrogates(line).and_then(|line| serde_json::from_str(&line).ok()))
}

/// Parses the four bytes of a `\uXXXX` escape. Reading hex from **bytes**
/// (not a `&str` slice) is what keeps `repair_lone_surrogates` panic-free:
/// when a rejected `\u` escape is followed by multi-byte UTF-8, the four
/// bytes after it are simply not ASCII hex and the escape passes through
/// untouched (pass-6 P6-1 — the old `&line[i+2..i+6]` sliced
/// mid-character and crashed every report action, release builds
/// included).
fn hex_escape_u16(mut bytes: &[u8]) -> Option<u16> {
    let mut value: u16 = 0;
    for _ in 0..4 {
        let byte = *bytes.first()?;
        let digit = (byte as char).to_digit(16)?;
        value = value.checked_mul(16)?.checked_add(digit as u16)?;
        bytes = &bytes[1..];
    }
    Some(value)
}

fn repair_lone_surrogates(line: &str) -> Option<String> {
    let bytes = line.as_bytes();
    let mut out = String::with_capacity(line.len());
    let mut changed = false;
    let mut i = 0;
    while i < bytes.len() {
        // Escaped-backslash pairs advance together so literal `\uXXXX`
        // text (written `\\uXXXX` in the raw line) is never mistaken for
        // an escape start and rewritten (pass-7 residual on the cycle-3
        // repair). Both bytes are ASCII, so the slice below cannot cut a
        // character.
        if bytes[i] == b'\\' && bytes.get(i + 1) == Some(&b'\\') {
            out.push_str(&line[i..i + 2]);
            i += 2;
            continue;
        }
        if bytes[i] == b'\\' && bytes.get(i + 1) == Some(&b'u') && i + 6 <= bytes.len() {
            if let Some(code) = hex_escape_u16(&bytes[i + 2..i + 6]) {
                if (0xD800..=0xDBFF).contains(&code) {
                    let pair = i + 12 <= bytes.len()
                        && bytes.get(i + 6) == Some(&b'\\')
                        && bytes.get(i + 7) == Some(&b'u')
                        && hex_escape_u16(&bytes[i + 8..i + 12])
                            .is_some_and(|next| (0xDC00..=0xDFFF).contains(&next));
                    if pair {
                        // Both hex groups are ASCII, so `i..i + 12` lies on
                        // char boundaries and the slice below cannot cut a
                        // character.
                        out.push_str(&line[i..i + 12]);
                        i += 12;
                    } else {
                        out.push_str("\\ufffd");
                        changed = true;
                        i += 6;
                    }
                    continue;
                }
                if (0xDC00..=0xDFFF).contains(&code) {
                    out.push_str("\\ufffd");
                    changed = true;
                    i += 6;
                    continue;
                }
            }
        }
        let ch = line[i..].chars().next()?;
        out.push(ch);
        i += ch.len_utf8();
    }
    changed.then_some(out)
}

/// usage_from_value plus the wire keys that produced each value. The
/// kimi_cli StatusUpdate arm uses the matched keys to disclose
/// alias-matched fields in parse diagnostics (rm-400): the alias
/// table is a standing guess about other tools' wire formats, so every
/// alias match is surfaced instead of silently-zeroing usage on a future
/// key change.
/// rm-449 F4: every wire key the usage extractor recognizes, across all
/// alias families (input/output/cache-creation/cache-read) and the
/// reasoning-token synonyms. The kimi_cli StatusUpdate arm consumes and
/// discloses these via `usage_from_value_with_keys`; on the
/// hermes/generic lane the vocabulary separates UNKNOWN keys
/// (`usage_unknown_key:…`, PoC p4-kimi) and known-but-unmapped ALIASES
/// (`usage_alias_unmapped:…`, review bd6e4a50 F1) from the five
/// canonical names that lane actually accounts
/// ([`USAGE_KEYS_CONSUMED`]).
const USAGE_WIRE_KEYS_RECOGNIZED: &[&str] = &[
    "input_tokens",
    "prompt_tokens",
    "inputTokens",
    "promptTokenCount",
    "input_other",
    "output_tokens",
    "completion_tokens",
    "outputTokens",
    "candidatesTokenCount",
    "output",
    "cache_creation_input_tokens",
    "cacheCreationInputTokens",
    "cache_creation",
    "cacheWriteTokens",
    "input_cache_creation",
    "cache_read_input_tokens",
    "cacheReadInputTokens",
    // rm-720 (run 3ec6cec08fb9, integration review fe4827ef): the
    // extractor grew this alias (generic table) and gemini_usage has
    // mapped it since the assess-F4 landing — the vocabulary must list
    // every key the extractor recognizes or the hermes/generic lane
    // misclassifies it as `usage_unknown_key` instead of the truthful
    // `usage_alias_unmapped` tier.
    "cachedContentTokenCount",
    "cache_read",
    "cacheReadTokens",
    "input_cache_read",
    "thoughtsTokenCount",
    "thinkingTokenCount",
    "thinking_tokens",
    "reasoning_tokens",
    // rm-720 (run 3ec6cec08fb9, integration review fe4827ef): antigravity
    // RPC reasoning synonym added to the extractor's reasoning family.
    "reasoningTokens",
];

pub(crate) fn usage_wire_key_is_recognized(key: &str) -> bool {
    USAGE_WIRE_KEYS_RECOGNIZED.contains(&key)
}

/// rm-449 F4 review fix (review bd6e4a50 F1): the CANONICAL post-flatten
/// names the hermes/generic lane's accounting actually consumes —
/// `session_from_events` reads exactly these five names from the
/// TOP-LEVEL `usage` field (`Event` deserializes no other usage
/// location). This is NARROWER than [`USAGE_WIRE_KEYS_RECOGNIZED`]:
/// the vocabulary holds other tools' wire synonyms that the kimi lane
/// maps but this lane does not. A numeric usage key is CONSUMED here
/// only when it flattens to one of these names at the top-level
/// location — a known alias (`prompt_tokens`), an unknown name, or a
/// canonical name at a scanned-but-ignored location (`message.usage`,
/// `providerData.usage`) is disclosed instead of silently zeroing
/// tokens (lib.rs `unrecognized_usage_keys`).
pub(crate) const USAGE_KEYS_CONSUMED: &[&str] = &[
    "input_tokens",
    "output_tokens",
    "reasoning_tokens",
    "cache_creation_input_tokens",
    "cache_read_input_tokens",
];

pub(crate) fn usage_key_is_consumed(key: &str) -> bool {
    USAGE_KEYS_CONSUMED.contains(&key)
}

fn usage_from_value_with_keys(value: &Value) -> (Option<BTreeMap<String, i64>>, Vec<&'static str>) {
    let Some(obj) = value.as_object() else {
        return (None, Vec::new());
    };
    let mut usage = BTreeMap::new();
    let mut matched_keys = Vec::new();
    for (target, keys) in [
        (
            "input_tokens",
            &[
                "input_tokens",
                "prompt_tokens",
                "inputTokens",
                "promptTokenCount",
                "input_other", // kimi_cli: non-cached input
            ][..],
        ),
        (
            "output_tokens",
            &[
                "output_tokens",
                "completion_tokens",
                "outputTokens",
                "candidatesTokenCount",
                // kimi_cli; number_as_i64's coercion keeps this numeric-only
                // so a bare string like "high" cannot match
                "output",
            ][..],
        ),
        (
            "cache_creation_input_tokens",
            &[
                "cache_creation_input_tokens",
                "cacheCreationInputTokens",
                "cache_creation",
                "cacheWriteTokens",
                "input_cache_creation", // kimi_cli
            ][..],
        ),
        (
            "cache_read_input_tokens",
            &[
                "cache_read_input_tokens",
                "cacheReadInputTokens",
                // Gemini API JSON shape; the antigravity standalone app
                // reports its cache reads through it (rm-720, codeburn
                // #1655's ModelUsageStats field map).
                "cachedContentTokenCount",
                "cache_read",
                "cacheReadTokens",
                "input_cache_read", // kimi_cli
            ][..],
        ),
    ] {
        // Prefer the first present key that actually yields a number,
        // matching the legacy parser's per-format precedence (canonical
        // keys first, aliases behind); remember which wire key won so
        // alias matches can be disclosed per session (rm-400).
        if let Some((key, value)) = keys
            .iter()
            .filter_map(|key| {
                obj.get(*key)
                    .and_then(number_as_i64)
                    .map(|value| (*key, value))
            })
            .next()
        {
            usage.insert(target.to_string(), value);
            matched_keys.push(key);
        }
    }
    // Thinking tokens ride the output rate but are reported separately
    // (Gemini usageMetadata.thoughtsTokenCount and the OpenAI-compatible
    // reasoning aliases); fold into output and keep the breakdown
    // (pass-9 CU-20).
    if let Some(reasoning) = [
        "thoughtsTokenCount",
        "thinkingTokenCount",
        "thinking_tokens",
        "reasoning_tokens",
        // antigravity RPC name, ModelUsageStats field 9 (rm-720,
        // codeburn #1655: thinking_output_tokens bills at the output
        // rate like every other reasoning alias)
        "reasoningTokens",
    ]
    .iter()
    .find_map(|key| obj.get(*key))
    .and_then(number_as_i64)
    .filter(|value| *value > 0)
    {
        if let Some(output) = usage.get_mut("output_tokens") {
            *output = output.saturating_add(reasoning);
        } else {
            usage.insert("output_tokens".to_string(), reasoning);
        }
        usage.insert("reasoning_tokens".to_string(), reasoning);
    }
    if usage.is_empty() {
        (None, matched_keys)
    } else {
        (Some(usage), matched_keys)
    }
}

fn usage_from_value(value: &Value) -> Option<BTreeMap<String, i64>> {
    usage_from_value_with_keys(value).0
}

fn tool_result_content(block: &Map<String, Value>) -> String {
    if let Some(text) = string(block.get("content")) {
        return text.to_string();
    }
    jsonish(block.get("content").or_else(|| block.get("response")))
}

fn copilot_usage(span: &Map<String, Value>) -> BTreeMap<String, i64> {
    let mut usage = BTreeMap::new();
    for (target, key) in [
        ("input_tokens", "gen_ai.usage.input_tokens"),
        ("output_tokens", "gen_ai.usage.output_tokens"),
        (
            "cache_creation_input_tokens",
            "gen_ai.usage.cache_creation_input_tokens",
        ),
        (
            "cache_read_input_tokens",
            "gen_ai.usage.cache_read_input_tokens",
        ),
    ] {
        if let Some(value) = copilot_i64_attr(span, key).filter(|value| *value > 0) {
            usage.insert(target.to_string(), value);
        }
    }
    // Copilot spans follow the GenAI semconv basis (`gen_ai.usage.
    // input_tokens` INCLUDES cached tokens), so normalize to the delta
    // basis with the shared clamped subtraction (rm-600).
    subtract_cached_input(
        &mut usage,
        &["cache_creation_input_tokens", "cache_read_input_tokens"],
    );
    usage
}

fn copilot_span_content(span: &Map<String, Value>) -> String {
    for key in ["content", "text", "message", "body"] {
        if let Some(text) = string(span.get(key)).filter(|value| !value.is_empty()) {
            return text.to_string();
        }
    }
    for key in ["content", "message", "body", "gen_ai.response.text"] {
        if let Some(text) = copilot_string_attr(span, key).filter(|value| !value.is_empty()) {
            return text;
        }
    }
    String::new()
}

fn copilot_attr<'a>(span: &'a Map<String, Value>, target: &str) -> Option<&'a Value> {
    match span.get("attributes")? {
        Value::Object(attrs) => attrs.get(target),
        Value::Array(attrs) => attrs.iter().find_map(|attr| {
            let attr = attr.as_object()?;
            if string(attr.get("key")) == Some(target) {
                attr.get("value")
            } else {
                None
            }
        }),
        _ => None,
    }
}

fn copilot_string_attr(span: &Map<String, Value>, target: &str) -> Option<String> {
    match copilot_attr(span, target)? {
        Value::String(text) => Some(text.clone()),
        Value::Object(obj) => string(obj.get("stringValue")).map(str::to_string),
        _ => None,
    }
}

fn copilot_i64_attr(span: &Map<String, Value>, target: &str) -> Option<i64> {
    let value = copilot_attr(span, target)?;
    number_as_i64(value).or_else(|| {
        value
            .as_object()
            .and_then(|obj| {
                obj.get("intValue")
                    .or_else(|| obj.get("stringValue"))
                    .or_else(|| obj.get("doubleValue"))
            })
            .and_then(number_as_i64)
    })
}

fn copilot_bool_attr(span: &Map<String, Value>, target: &str) -> bool {
    let Some(value) = copilot_attr(span, target) else {
        return false;
    };
    value.as_bool().unwrap_or_else(|| {
        value
            .as_object()
            .and_then(|obj| obj.get("boolValue").and_then(Value::as_bool))
            .unwrap_or(false)
    })
}

fn copilot_timestamp(raw: Option<&Value>) -> String {
    let Some(value) = raw.and_then(number_as_i64) else {
        return String::new();
    };
    let secs = value / 1_000_000_000;
    let nsec = (value % 1_000_000_000) as u32;
    chrono::DateTime::<chrono::Utc>::from_timestamp(secs, nsec)
        .map(|ts| ts.to_rfc3339_opts(chrono::SecondsFormat::Nanos, true))
        .unwrap_or_default()
}

fn first_value<'a>(obj: &'a Map<String, Value>, keys: &[&str]) -> Option<&'a Value> {
    keys.iter().find_map(|key| obj.get(*key))
}

fn string(value: Option<&Value>) -> Option<&str> {
    value.and_then(Value::as_str)
}

fn jsonish(value: Option<&Value>) -> String {
    match value {
        Some(Value::String(text)) => text.to_string(),
        Some(Value::Null) | None => String::new(),
        Some(other) => serde_json::to_string(other).unwrap_or_default(),
    }
}

fn non_empty(events: Vec<Event>) -> Option<Vec<Event>> {
    if events.is_empty() {
        None
    } else {
        Some(events)
    }
}

fn is_cline_task_file(path: &Path) -> bool {
    matches!(
        path.file_name().and_then(|name| name.to_str()),
        Some("api_conversation_history.json" | "ui_messages.json" | "task_metadata.json")
    )
}

fn read_json_value(path: &Path) -> Option<Value> {
    let raw = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&raw).ok()
}

fn cline_model(metadata: &Map<String, Value>) -> String {
    for key in ["model", "modelId", "model_id", "apiModelId"] {
        if let Some(model) = string(metadata.get(key)) {
            if !model.is_empty() {
                return model.to_string();
            }
        }
    }
    if let Some(config) = metadata.get("apiConfiguration").and_then(Value::as_object) {
        for key in ["model", "modelId", "model_id", "apiModelId"] {
            if let Some(model) = string(config.get(key)) {
                if !model.is_empty() {
                    return model.to_string();
                }
            }
        }
    }
    "unknown".to_string()
}

fn parse_cline_ui_messages(value: &Value, model: &str) -> Vec<Event> {
    let messages: &[Value] = match value {
        Value::Array(items) => items,
        Value::Object(obj) => obj
            .get("messages")
            .and_then(Value::as_array)
            .map(Vec::as_slice)
            .unwrap_or(&[]),
        _ => &[],
    };
    let mut events = Vec::new();
    for message in messages {
        let Some(message) = message.as_object() else {
            continue;
        };
        let text = string(message.get("text"))
            .or_else(|| string(message.get("content")))
            .or_else(|| string(message.get("message")))
            .unwrap_or("");
        if text.is_empty() {
            continue;
        }
        let ts = cline_timestamp(
            first_value(message, &["ts", "timestamp", "createdAt", "created_at"])
                .unwrap_or(&Value::Null),
        );
        let kind = string(message.get("type")).unwrap_or("");
        let ask = string(message.get("ask")).unwrap_or("");
        let say = string(message.get("say")).unwrap_or("");
        let mut role = "assistant";
        if kind == "ask" || !ask.is_empty() {
            role = "user";
        }
        if say == "tool" || ask == "tool" {
            role = "assistant";
        }
        events.push(Event {
            role: role.to_string(),
            content: text.to_string(),
            timestamp: ts,
            model_used: model.to_string(),
            source_tool: "cline".to_string(),
            ..Event::default()
        });
    }
    events
}

fn append_cline_event(events: &mut Vec<Event>, seen: &mut BTreeSet<String>, mut event: Event) {
    if event.source_tool.is_empty() {
        event.source_tool = "cline".to_string();
    }
    if event.model_used.is_empty() {
        event.model_used = "unknown".to_string();
    }
    if event.role.is_empty() {
        return;
    }
    let key = cline_event_key(&event);
    if seen.insert(key) {
        events.push(event);
    }
}

fn cline_event_key(event: &Event) -> String {
    let tool_parts = event
        .tool_calls
        .iter()
        .map(|tool| format!("{}:{}:{}", tool.id, tool.name, tool.args))
        .collect::<Vec<_>>()
        .join(",");
    [
        event.role.as_str(),
        event.content.as_str(),
        event.timestamp.as_str(),
        event.tool_call_id.as_str(),
        tool_parts.as_str(),
    ]
    .join("\0")
}

fn apply_cline_metadata_timestamps(events: &mut [Event], metadata: &Map<String, Value>) {
    if events.is_empty() || events.iter().any(|event| !event.timestamp.is_empty()) {
        return;
    }
    let start = cline_timestamp(
        first_value(metadata, &["createdAt", "created_at", "ts"]).unwrap_or(&Value::Null),
    );
    let end = cline_timestamp(
        first_value(metadata, &["updatedAt", "updated_at", "lastUpdatedAt"])
            .unwrap_or(&Value::Null),
    );
    if !start.is_empty() {
        events[0].timestamp = start;
    }
    if !end.is_empty() {
        if let Some(last) = events.last_mut() {
            last.timestamp = end;
        }
    }
}

fn session_name(path: &Path) -> String {
    path.file_stem()
        .and_then(|name| name.to_str())
        .unwrap_or("session")
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn codex_total_usage_rewind_after_compaction_counts_forward() {
        // rm-035 (upstream PR #286, be25c4c) → rm-554 (2026-10-06, upstream
        // #312): Codex rewinds total_token_usage after compaction and
        // climbs back. The rm-162/#286 high-water mark kept the running
        // maximum per class, so the rebound back over the old mark was
        // free — but the compacted context is re-sent and billed after
        // the reset, so growth from the rewound baseline is real usage.
        // Each distinct total now counts its forward delta once (these
        // journals carry no last_token_usage snapshots); the rewind event
        // itself fabricates nothing (all deltas negative).
        // Dated pin change 2026-10-06: 620/480/590 → 870/630 with output
        // decomposed as 500 output + 90 reasoning (reasoning no longer
        // added to output, rm-553 — rebound rm-603 at integration); the
        // two decompositions cost the same at the output rate.
        let total = |input: i64, cached: i64, output: i64, reasoning: i64| {
            serde_json::json!({
                "input_tokens": input,
                "cached_input_tokens": cached,
                "output_tokens": output,
                "reasoning_output_tokens": reasoning
            })
        };
        let meta = serde_json::json!({
            "timestamp": "2026-09-30T01:00:00Z",
            "type": "session_meta",
            "payload": {"cwd": "/tmp/probe", "model": "gpt-5.3-codex"}
        })
        .to_string();
        let token_count = |second: usize, totals: Value| {
            serde_json::json!({
                "timestamp": format!("2026-09-30T01:00:{second:02}Z"),
                "type": "event_msg",
                "payload": {
                    "type": "token_count",
                    "info": {"total_token_usage": totals}
                }
            })
            .to_string()
        };
        let raw = [
            meta,
            // Rising cumulative: the first token_count reports everything.
            token_count(1, total(1000, 400, 200, 50)),
            // Compaction rewinds the context-side counters (input/cached)
            // while output keeps climbing: this event still emits and is
            // exactly where the raw-total previous went wrong.
            token_count(2, total(600, 250, 350, 60)),
            // Rebound over the old high-water mark.
            token_count(3, total(1100, 480, 500, 90)),
        ]
        .join("\n");
        let session =
            parse_raw_session("codex", "rollout.jsonl", &raw).expect("codex rollout parses");
        // Forward deltas from each distinct total: ev1 (1000, 400, 200,
        // 50); ev2 contributes only the climbing classes (output +150,
        // reasoning +10); ev3 contributes (input +500 of which cached
        // +230, output +150, reasoning +30). Input is net of cache reads:
        // 600 + (500 - 230) = 870 input / 630 cache read / 500 output /
        // 90 reasoning. The old high-water pin (620/480/590 output-folded)
        // refused the post-compaction window — the undercount rm-554
        // removes.
        assert_eq!(session.metrics.tokens_input, 870);
        assert_eq!(session.metrics.tokens_cache_r, 630);
        assert_eq!(session.metrics.tokens_output, 500);
        assert_eq!(session.metrics.tokens_reasoning, 90);
        assert_eq!(session.metrics.provenance.tokens, "reported_by_agent");
    }

    #[test]
    fn token_accumulation_saturates_instead_of_wrapping() {
        // rm-046: hostile journals can repeat i64::MAX-sized counts across
        // messages. The map-merge sites used plain `+=`, which panics in
        // debug builds and wraps negative in release, where the >0
        // consumers then silently dropped the totals — a 5 + MAX journal
        // reported input 0.
        let mut usage = BTreeMap::new();
        add_usage_value(&mut usage, "input_tokens", Some(&serde_json::json!(5)));
        add_usage_value(
            &mut usage,
            "input_tokens",
            Some(&serde_json::json!(i64::MAX)),
        );
        add_usage_value(
            &mut usage,
            "input_tokens",
            Some(&serde_json::json!(i64::MAX)),
        );
        assert_eq!(
            usage.get("input_tokens").copied(),
            Some(i64::MAX),
            "accumulation clamps at i64::MAX instead of panicking or wrapping"
        );

        let mut left = BTreeMap::from([("output_tokens".to_string(), i64::MAX)]);
        let right = BTreeMap::from([("output_tokens".to_string(), 1)]);
        add_usage(&mut left, &right);
        assert_eq!(left.get("output_tokens").copied(), Some(i64::MAX));
    }

    #[test]
    fn codex_ignorable_probe_rescues_token_count_beyond_the_head_window() {
        // rm-047: the negative needle required exact
        // `"payload":{"type":"token_count"}` adjacency inside the first
        // 160 bytes, so a real token_count event with reordered payload
        // keys (or a type marker past the window) was dropped as
        // ignorable event_msg noise — silently losing the session's only
        // usage record.
        let noise = r#"{"timestamp":"2026-09-30T01:00:00Z","type":"event_msg","payload":{"type":"agent_message_delta","delta":"hi"}}"#;
        assert!(codex_line_is_ignorable(noise));
        // The compacted compaction snapshot used to be blanket-ignorable;
        // since rm-401 those lines parse (compaction pairing rides
        // them), so they no longer take the fast path.
        let compacted = r#"{"timestamp":"2026-09-30T01:00:00Z","type":"compacted","payload":{}}"#;
        assert!(!codex_line_is_ignorable(compacted));

        // Same event, payload keys reordered and padded past the window.
        let rescued = format!(
            r#"{{"timestamp":"2026-09-30T01:00:00Z","type":"event_msg","payload":{{"padding":"{}","type":"token_count"}}}}"#,
            "p".repeat(200)
        );
        assert!(
            !codex_line_is_ignorable(&rescued),
            "token_count marker beyond the head window must be rescued by the whole-line scan"
        );

        // Escaped marker text inside a JSON value never matches the raw
        // needle; the scan must keep skipping these.
        let quoted = r#"{"timestamp":"2026-09-30T01:00:00Z","type":"event_msg","payload":{"type":"agent_message_delta","delta":"\"type\":\"token_count\""}}"#;
        assert!(codex_line_is_ignorable(quoted));

        // Corrupt (non-JSON) quoting puts the raw needle mid-value: the
        // key-boundary anchor must not treat it as the token_count key.
        let corrupt = r#"{"timestamp":"2026-09-30T01:00:00Z","type":"event_msg","payload":{"delta":"seen "type":"token_count" inline"}}"#;
        assert!(codex_line_is_ignorable(corrupt));
    }

    #[test]
    fn codex_ignorable_nested_type_key_does_not_skip() {
        // rm-047 residual (assess F1, 2026-10-05): the positive probe
        // anchored on any `{` or `,` before the needle, so a NESTED
        // {"z0":{"type":"event_msg"}} key (at any depth) made a
        // usage-bearing line look like event_msg chatter. The live PoC
        // corpus (/tmp/at-assess-fa0283/probes) flipped cost
        // 0.0168 -> 0.0062 and dropped the model row because the
        // token_usage_record line was skipped whole. The probe now
        // requires the marker at the root object's top level.
        let bare_nested = r#"{"z0":{"type":"event_msg"}}"#;
        assert!(
            !codex_line_is_ignorable(bare_nested),
            "a nested type key is not an event_msg envelope"
        );

        let record = r#"{"timestamp":"2026-10-01T09:01:00.000Z","type":"token_usage_record","payload":{"z0":{"type":"event_msg"},"response_id":"resp-9","usage":{"input_tokens":5000,"output_tokens":700,"cached_input_tokens":4000},"model":"gpt-5.3-codex"}}"#;
        assert!(
            !codex_line_is_ignorable(record),
            "the assess PoC's token_usage_record line must not skip"
        );

        let array_root = r#"[{"type":"event_msg"}]"#;
        assert!(!codex_line_is_ignorable(array_root));

        // Depth tracking must not be fooled by braces inside string
        // values: the marker here IS top level and stays skippable.
        let brace_in_string =
            r#"{"payload":"{","timestamp":"2026-09-30T01:00:00Z","type":"event_msg"}"#;
        assert!(codex_line_is_ignorable(brace_in_string));

        // A closed nested container before the marker does not strand
        // the top-level key at depth 2.
        let recovered = r#"{"payload":{"a":1},"type":"event_msg","more":{}}"#;
        assert!(codex_line_is_ignorable(recovered));
    }

    #[test]
    fn codex_nested_marker_keeps_usage_identical_to_clean_journal() {
        // Differential form of the assess F1 PoC: the hostile journal is
        // byte-identical to the control except for the injected nested
        // {"z0":{"type":"event_msg"}} key inside the
        // token_usage_record payload. Parsed metrics must not differ
        // (pre-fix the line was skipped whole: -64% cost, model row
        // lost).
        let meta = r#"{"timestamp":"2026-10-01T09:00:00.000Z","type":"session_meta","payload":{"id":"s-nested","cwd":"/tmp/x","originator":"codex_cli_rs","source":"api"}}"#;
        let token_count = r#"{"timestamp":"2026-10-01T09:00:30.000Z","type":"event_msg","payload":{"type":"token_count","info":{"total_token_usage":{"input_tokens":1000,"output_tokens":200,"cached_input_tokens":0,"reasoning_output_tokens":0}}}}"#;
        let record = r#"{"timestamp":"2026-10-01T09:01:00.000Z","type":"token_usage_record","payload":{"response_id":"resp-9","usage":{"input_tokens":5000,"output_tokens":700,"cached_input_tokens":4000},"model":"gpt-5.3-codex"}}"#;
        let hostile_record = r#"{"timestamp":"2026-10-01T09:01:00.000Z","type":"token_usage_record","payload":{"z0":{"type":"event_msg"},"response_id":"resp-9","usage":{"input_tokens":5000,"output_tokens":700,"cached_input_tokens":4000},"model":"gpt-5.3-codex"}}"#;
        let compacted = r#"{"timestamp":"2026-10-01T09:01:01.000Z","type":"compacted","payload":{"compaction_response_id":"resp-9"}}"#;

        let control = parse_raw_session(
            "codex",
            "control.jsonl",
            &[meta, token_count, record, compacted].join("\n"),
        )
        .expect("control journal parses");
        let hostile = parse_raw_session(
            "codex",
            "hostile.jsonl",
            &[meta, token_count, hostile_record, compacted].join("\n"),
        )
        .expect("hostile journal parses");

        assert_eq!(control.metrics.tokens_input, hostile.metrics.tokens_input);
        assert_eq!(control.metrics.tokens_output, hostile.metrics.tokens_output);
        assert_eq!(
            control.metrics.tokens_cache_r,
            hostile.metrics.tokens_cache_r
        );
        assert_eq!(
            control.metrics.cost_estimated,
            hostile.metrics.cost_estimated
        );
        assert!(
            !hostile
                .metrics
                .line_skips
                .contains_key("codex_ignorable_line"),
            "the injected nested marker must not skip the usage line"
        );
    }

    #[test]
    fn codex_ignorable_skips_are_counted_in_parse_diagnostics() {
        // rm-047: the head-probe fast path used to discard event_msg and
        // compacted lines with no trace; parse diagnostics now count them.
        let meta = serde_json::json!({
            "timestamp": "2026-09-30T01:00:00Z",
            "type": "session_meta",
            "payload": {"cwd": "/tmp/probe", "model": "gpt-5.3-codex"}
        })
        .to_string();
        let noise = |second: usize| {
            serde_json::json!({
                "timestamp": format!("2026-09-30T01:00:{second:02}Z"),
                "type": "event_msg",
                "payload": {"type": "agent_message_delta", "delta": "chatter"}
            })
            .to_string()
        };
        let token_count = serde_json::json!({
            "timestamp": "2026-09-30T01:00:05Z",
            "type": "event_msg",
            "payload": {
                "type": "token_count",
                "info": {"total_token_usage": {"input_tokens": 100, "cached_input_tokens": 0, "output_tokens": 40, "reasoning_output_tokens": 0}}
            }
        })
        .to_string();
        let raw = [meta, noise(1), noise(2), token_count].join("\n");
        let session =
            parse_raw_session("codex", "rollout.jsonl", &raw).expect("codex rollout parses");
        assert_eq!(session.metrics.tokens_input, 100);
        // rm-730 pin change: the ignorable-line counter moved from
        // line_skips to disclosure_counters (assumption-class, not
        // loss) — the counter stays visible, but no longer degrades
        // data_health.confidence.
        assert_eq!(
            session
                .metrics
                .disclosure_counters
                .get("codex_ignorable_line"),
            Some(&2),
            "skipped codex chatter is visible in parse diagnostics"
        );
        assert!(
            session.metrics.line_skips.is_empty(),
            "structural skips must not render as dropped lines: {:?}",
            session.metrics.line_skips
        );
    }

    #[test]
    fn codex_line_skips_disclose_every_dropped_shape() {
        // rm-584: a typeless payload line, a bare JSON string, a bare
        // number, and an unparseable line used to vanish with
        // line_skips: {} (PoC corpus /tmp/at-assess-92e8/codex-shapes.jsonl).
        let meta = serde_json::json!({
            "timestamp": "2026-10-05T20:00:00Z",
            "type": "session_meta",
            "payload": {"cwd": "/tmp/probe"}
        })
        .to_string();
        let typeless = serde_json::json!({
            "timestamp": "2026-10-05T20:00:01Z",
            "payload": {"type": "custom_tool_call", "call_id": "c1", "name": "exec"}
        })
        .to_string();
        let raw = [
            meta,
            typeless,
            serde_json::json!("a bare json string line").to_string(),
            serde_json::json!(12345).to_string(),
            "{ not json".to_string(),
        ]
        .join("\n");
        let session =
            parse_raw_session("codex", "codex-shapes.jsonl", &raw).expect("corpus parses");
        let skips = &session.metrics.line_skips;
        assert_eq!(
            skips.get("codex_missing_type"),
            Some(&1),
            "a payload line with no top-level type must be disclosed: {skips:?}"
        );
        assert_eq!(
            skips.get("codex_non_object_line:string"),
            Some(&1),
            "bare string lines must be disclosed: {skips:?}"
        );
        assert_eq!(
            skips.get("codex_non_object_line:number"),
            Some(&1),
            "bare number lines must be disclosed: {skips:?}"
        );
        assert_eq!(
            skips.get("codex_unparseable_line"),
            Some(&1),
            "non-JSON lines must be disclosed: {skips:?}"
        );
    }

    #[test]
    fn codex_custom_tools_rollout_parses_calls_reasoning_and_disclosure() {
        // rm-542 (run b1ff12f8, cycle 2): the custom-tools wire. Real
        // rollouts (2026-09-26 census: 7-20 custom_tool_call pairs per
        // session, ZERO classic function_calls) carry all their tool work
        // in response_item/custom_tool_call + custom_tool_call_output and
        // their reasoning in standalone response_item/reasoning items —
        // every one of which fell to the `_ => {}` arm, so the newest
        // real sessions reported tool_calls_total=0, tool_results=0,
        // reasoning_blocks=0. The vendored, sanitized fixture (see
        // tests/fixtures/codex-custom-tools/README.md for provenance,
        // census, and the sha256 pin) is the golden corpus: the pinned
        // totals below are the fixture's live-measured post-fix values,
        // and the scrub is metrics-neutral against the unsanitized
        // original.
        let raw = std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/codex-custom-tools/rollout.jsonl"
        ))
        .expect("vendored codex custom-tools fixture ships with the crate");
        let session = parse_raw_session("codex", "rollout.jsonl", &raw).expect("rollout parses");
        assert_eq!(session.metrics.source_tool, "codex_cli");
        assert_eq!(session.metrics.model_used, "glm-5.2");
        // Census pin: 7 custom_tool_call pairs => 7 calls, 7 results,
        // every one completed (no failure status in the real corpus).
        assert_eq!(session.metrics.tool_calls_total, 7);
        assert_eq!(session.metrics.tool_results, 7);
        assert_eq!(session.metrics.tool_calls_ok, 7);
        assert_eq!(session.metrics.tool_calls_fail, 0);
        assert_eq!(session.metrics.tool_usage.get("exec"), Some(&7));
        // Census pin: 5 standalone reasoning items => 5 reasoning
        // blocks (the message arm contributes none here — this file's
        // reasoning lives outside messages).
        assert_eq!(session.metrics.reasoning_blocks, 5);
        // Reported usage pins (token_count snapshots are preserved by
        // the sanitizer): 23,312 in / 192,128 cache read. Dated pin change
        // 2026-10-06 (rm-603, upstream #312): Codex reasoning is now a
        // breakdown, not an addend — the 9,790 out pin was 5,531 real
        // output + 4,259 reasoning folded in; the fixture's sanitizer-
        // neutral snapshots split them.
        assert_eq!(session.metrics.tokens_input, 23_312);
        assert_eq!(session.metrics.tokens_output, 5_531);
        assert_eq!(session.metrics.tokens_reasoning, 4_259);
        assert_eq!(session.metrics.tokens_cache_r, 192_128);
        // Disclosure pins: world_state is explicitly ignored AND counted
        // (1 in the census) — rm-730 routes that structural counter onto
        // the disclosure channel — and nothing in this real-shape
        // fixture is unmatched: every response_item payload type it
        // carries is handled, so the unmatched-disclosure counters
        // (which stay on line_skips) must be absent.
        assert_eq!(
            session.metrics.disclosure_counters.get("codex_world_state"),
            Some(&1)
        );
        let unmatched: Vec<&String> = session
            .metrics
            .line_skips
            .keys()
            .filter(|key| key.starts_with("codex_unmatched"))
            .collect();
        assert_eq!(
            unmatched,
            Vec::<&String>::new(),
            "a handled real-shape corpus must disclose nothing as unmatched"
        );
    }

    #[test]
    fn codex_custom_tool_failure_status_marks_paired_output_as_error() {
        // rm-542: custom_tool_call carries an explicit `status`; a call
        // that did not complete must surface as a tool FAILURE through
        // its paired output, the way other parsers report tool errors —
        // not as a silent success. (The vendored corpus is all
        // `completed`, so the failure arm is pinned synthetically.)
        let meta = serde_json::json!({
            "timestamp": "2026-09-26T20:00:00Z",
            "type": "session_meta",
            "payload": {"cwd": "/tmp/probe", "model": "gpt-5.3-codex"}
        })
        .to_string();
        let call = |status: &str, call_id: &str, ts: &str| {
            serde_json::json!({
                "timestamp": ts,
                "type": "response_item",
                "payload": {
                    "type": "custom_tool_call",
                    "status": status,
                    "call_id": call_id,
                    "name": "exec",
                    "input": "echo probe"
                }
            })
            .to_string()
        };
        let output = |call_id: &str, ts: &str| {
            serde_json::json!({
                "timestamp": ts,
                "type": "response_item",
                "payload": {
                    "type": "custom_tool_call_output",
                    "call_id": call_id,
                    "output": [{"type": "input_text", "text": "timed out"}]
                }
            })
            .to_string()
        };
        let raw = [
            meta,
            call("completed", "call_good", "2026-09-26T20:00:01Z"),
            output("call_good", "2026-09-26T20:00:02Z"),
            call("failed", "call_bad", "2026-09-26T20:00:03Z"),
            output("call_bad", "2026-09-26T20:00:04Z"),
        ]
        .join("\n");
        let session = parse_raw_session("codex", "rollout.jsonl", &raw).expect("rollout parses");
        assert_eq!(session.metrics.tool_calls_total, 2);
        assert_eq!(session.metrics.tool_results, 2);
        assert_eq!(session.metrics.tool_calls_fail, 1);
        assert_eq!(session.metrics.tool_calls_ok, 1);
        assert_eq!(
            session.metrics.provenance.tool_results, "reported_by_agent",
            "an agent-reported failure status is reported, not inferred"
        );
    }

    #[test]
    fn codex_unknown_wire_shapes_are_disclosed_not_silent() {
        // rm-542 acceptance: future wire growth must be visible, not
        // silent — the custom-tools gap hid for a month because unknown
        // payload types incremented nothing (rm-401 precedent). An
        // unknown response_item payload type and an unknown top-level
        // type each land in parse diagnostics under a named counter.
        let meta = serde_json::json!({
            "timestamp": "2026-09-26T20:00:00Z",
            "type": "session_meta",
            "payload": {"cwd": "/tmp/probe", "model": "gpt-5.3-codex"}
        })
        .to_string();
        let future_item = serde_json::json!({
            "timestamp": "2026-09-26T20:00:01Z",
            "type": "response_item",
            "payload": {"type": "future_widget", "payload": {"opaque": true}}
        })
        .to_string();
        let future_top = serde_json::json!({
            "timestamp": "2026-09-26T20:00:02Z",
            "type": "future_envelope",
            "payload": {"opaque": true}
        })
        .to_string();
        let raw = [meta, future_item, future_top].join("\n");
        let session = parse_raw_session("codex", "rollout.jsonl", &raw).expect("rollout parses");
        assert_eq!(
            session
                .metrics
                .line_skips
                .get("codex_unmatched_response_item:future_widget"),
            Some(&1),
            "unknown response_item payload types are disclosed by name"
        );
        assert_eq!(
            session
                .metrics
                .line_skips
                .get("codex_unmatched_type:future_envelope"),
            Some(&1),
            "unknown top-level types are disclosed by name"
        );
    }

    #[test]
    fn codex_usage_sums_saturate_instead_of_overflowing() {
        // rm-162: two legal in-range i64 counters on an adversarial
        // journal must never wrap or panic. rm-603 (upstream #312) removed
        // the output+reasoning fold from this lane — reasoning is now a
        // breakdown on its own line — so each class clamps independently
        // and the saturation contract lives on unchanged.
        let info = serde_json::json!({
            "last_token_usage": {
                "input_tokens": 10,
                "output_tokens": i64::MAX,
                "reasoning_output_tokens": i64::MAX
            }
        });
        let mut totals = CodexTotals::default();
        let usage = codex_token_count_usage(Some(&info), &mut totals).expect("usage event");
        assert_eq!(usage["input_tokens"], 10);
        assert_eq!(usage["output_tokens"], i64::MAX);
        assert_eq!(usage["reasoning_tokens"], i64::MAX);
    }

    #[test]
    fn codex_usage_counts_forward_across_compaction_resets() {
        // rm-554 (2026-10-06, upstream #312): Codex rewinds
        // total_token_usage after compaction and then climbs back. The old
        // rm-162/#286 high-water mark refused the whole post-compaction
        // window — but the compacted context is re-sent and billed on every
        // call after the reset, so growth inside the old envelope is real
        // usage. Each DISTINCT total now counts once (its `last` snapshot
        // when present, else the forward delta from the previous total);
        // a rewind snapshot without `last` still fabricates no usage.
        let step = |total_input: i64| {
            serde_json::json!({
                "total_token_usage": {"input_tokens": total_input}
            })
        };
        let mut totals = CodexTotals::default();
        let first = codex_token_count_usage(Some(&step(2500)), &mut totals).expect("first event");
        assert_eq!(first["input_tokens"], 2500);
        // Rewind to 1000 fabricates no usage (negative delta -> no event).
        let rewound = codex_token_count_usage(Some(&step(1000)), &mut totals);
        assert!(rewound.is_none());
        // Rebound to 3000 counts the full 2000-token growth from the reset
        // baseline — the old pin (500, climb past the old mark only) was
        // the undercount this defect removes.
        let rebound =
            codex_token_count_usage(Some(&step(3000)), &mut totals).expect("rebound event");
        assert_eq!(rebound["input_tokens"], 2000);
        // A re-emitted total (rate-limit-only update) counts nothing.
        let repeated = codex_token_count_usage(Some(&step(3000)), &mut totals);
        assert!(repeated.is_none());
    }

    #[test]
    fn add_usage_accumulators_saturate_instead_of_overflowing() {
        // rm-162 / IR-1: add_usage and add_usage_value are journal-
        // controlled accumulators on the OpenCode path; i64::MAX-scale
        // magnitudes must saturate, not panic (debug) or wrap negative
        // (release) the way the plain += did.
        let mut dst: BTreeMap<String, i64> = BTreeMap::new();
        dst.insert("input_tokens".to_string(), i64::MAX - 5);
        let mut src: BTreeMap<String, i64> = BTreeMap::new();
        src.insert("input_tokens".to_string(), 100);
        add_usage(&mut dst, &src);
        assert_eq!(dst["input_tokens"], i64::MAX);

        let mut usage: BTreeMap<String, i64> = BTreeMap::new();
        add_usage_value(
            &mut usage,
            "output_tokens",
            Some(&serde_json::Value::from(i64::MAX)),
        );
        assert_eq!(usage["output_tokens"], i64::MAX);
        // A second MAX-scale accumulation still holds at MAX.
        add_usage_value(
            &mut usage,
            "output_tokens",
            Some(&serde_json::Value::from(i64::MAX)),
        );
        assert_eq!(usage["output_tokens"], i64::MAX);
    }

    #[test]
    fn bom_is_stripped_once_at_offset_zero_and_nowhere_else() {
        // Pass-7 P7-2: a UTF-8 BOM used to defeat every format detector
        // ("unsupported session format"); stripping once at the shared
        // entry fixes every path, while a U+FEFF embedded later in the
        // content survives as content.
        let bom_prefixed = concat!(
            "\u{feff}",
            "{\"role\":\"user\",\"content\":\"bom-prefixed\"}\n"
        );
        let session =
            parse_raw_session("bom", "bom.jsonl", bom_prefixed).expect("BOM-prefixed file parses");
        assert_eq!(session.metrics.user_messages, 1);

        let mid_content = "{\"role\":\"user\",\"content\":\"keeps \u{feff} inside\"}\n";
        let session =
            parse_raw_session("mid", "mid.jsonl", mid_content).expect("mid-content BOM parses");
        assert_eq!(session.metrics.user_messages, 1);
    }

    #[test]
    fn claude_flat_transcript_preserves_the_call_result_join() {
        // rm-230 (campaign-local rm-025): flat tool_use/tool_result
        // entries must keep the id that pairs them (explicit
        // `tool_use_id` when present); id-less entries pair positionally
        // per tool (k-th call <-> k-th result of the SAME tool). Before
        // the fix every flat call reported unmatched in diagnostics.
        let raw = concat!(
            "{\"type\":\"user\",\"timestamp\":\"2026-01-01T00:00:00Z\",\"content\":\"go\"}\n",
            "{\"type\":\"tool_use\",\"timestamp\":\"2026-01-01T00:00:01Z\",\"tool_name\":\"Bash\",\"tool_input\":{\"path\":\"a\"},\"tool_use_id\":\"tu-1\"}\n",
            "{\"type\":\"tool_result\",\"timestamp\":\"2026-01-01T00:00:02Z\",\"tool_name\":\"Bash\",\"tool_output\":\"ok\",\"tool_use_id\":\"tu-1\"}\n",
            "{\"type\":\"tool_use\",\"timestamp\":\"2026-01-01T00:00:03Z\",\"tool_name\":\"Read\",\"tool_input\":{\"path\":\"b\"}}\n",
            "{\"type\":\"tool_result\",\"timestamp\":\"2026-01-01T00:00:04Z\",\"tool_name\":\"Read\",\"tool_output\":\"ok\"}\n"
        );
        let events = parse_claude_transcript_jsonl(&jsonl_objects(raw).collect::<Vec<_>>())
            .expect("flat claude transcript detected");
        let calls: Vec<&ToolCall> = events.iter().flat_map(|e| &e.tool_calls).collect();
        let results: Vec<&Event> = events.iter().filter(|e| e.role == "tool").collect();
        // Explicit ids are kept verbatim on both sides.
        assert_eq!(calls[0].id, "tu-1");
        assert_eq!(results[0].tool_call_id, "tu-1");
        // Id-less entries pair positionally among THEMSELVES, per tool:
        // the k-th id-less call <-> k-th id-less result of the SAME
        // tool (explicit ids consume no positional slot; review F5).
        assert_eq!(calls[1].id, "flat-pair-Read-0");
        assert_eq!(results[1].tool_call_id, "flat-pair-Read-0");
    }

    #[test]
    fn claude_flat_transcript_positional_pairs_survive_out_of_order_mixing() {
        // Review F5: with one shared per-side counter incremented by
        // every entry, an out-of-order mixed stream (explicit call,
        // id-less call, id-less result, explicit result) minted
        // flat-pair-1 on the call side but flat-pair-0 on the result
        // side -- both entries then stayed spuriously unmatched
        // (verified: 2 unmatched pre-fix). Ordinals that count id-less
        // entries only keep the sides in agreement regardless of how
        // explicit and id-less entries interleave.
        let raw = concat!(
            "{\"type\":\"tool_use\",\"timestamp\":\"2026-01-01T00:00:01Z\",\"tool_name\":\"Bash\",\"tool_input\":{\"cmd\":\"ls a\"},\"tool_use_id\":\"tu-1\"}\n",
            "{\"type\":\"tool_use\",\"timestamp\":\"2026-01-01T00:00:02Z\",\"tool_name\":\"Read\",\"tool_input\":{\"path\":\"b\"}}\n",
            "{\"type\":\"tool_result\",\"timestamp\":\"2026-01-01T00:00:03Z\",\"tool_name\":\"Read\",\"tool_output\":\"ok\"}\n",
            "{\"type\":\"tool_result\",\"timestamp\":\"2026-01-01T00:00:04Z\",\"tool_name\":\"Bash\",\"tool_output\":\"ok\",\"tool_use_id\":\"tu-1\"}\n"
        );
        let session = parse_raw_session("mix", "mix.jsonl", raw).expect("flat session parses");
        for lat in &session.diagnostics.tool_latencies {
            assert_eq!(
                lat.unmatched, 0,
                "{} must not report unmatched on a fully-paired mixed stream",
                lat.tool_name
            );
        }
        let total: usize = session
            .diagnostics
            .tool_latencies
            .iter()
            .map(|lat| lat.count)
            .sum();
        assert_eq!(total, 2, "both calls must be accounted for");
    }

    #[test]
    fn claude_flat_transcript_positional_pairs_stay_within_one_tool() {
        // Review R2-3: with one GLOBAL per-side ordinal, an id-less
        // result stream that interleaves tools paired the wrong tool's
        // result onto a call -- tool_latencies then attributed Bash's
        // end-time to Read (and vice versa), and one tool's surplus
        // result masked another tool's genuinely missing one. Per-tool
        // ordinals keep positional pairing inside one tool: Bash(call
        // @00:01) pairs with Bash(result @00:04) -> 3s, Read(call @00:02)
        // pairs with Read(result @00:09) -> 7s, both unmatched 0.
        let raw = concat!(
            "{\"type\":\"tool_use\",\"timestamp\":\"2026-01-01T00:00:01Z\",\"tool_name\":\"Bash\",\"tool_input\":{\"cmd\":\"ls a\"}}\n",
            "{\"type\":\"tool_use\",\"timestamp\":\"2026-01-01T00:00:02Z\",\"tool_name\":\"Read\",\"tool_input\":{\"path\":\"b\"}}\n",
            "{\"type\":\"tool_result\",\"timestamp\":\"2026-01-01T00:00:09Z\",\"tool_name\":\"Read\",\"tool_output\":\"ok\"}\n",
            "{\"type\":\"tool_result\",\"timestamp\":\"2026-01-01T00:00:04Z\",\"tool_name\":\"Bash\",\"tool_output\":\"ok\"}\n"
        );
        let session = parse_raw_session("cross", "cross.jsonl", raw).expect("flat session parses");
        let lat = |name: &str| {
            session
                .diagnostics
                .tool_latencies
                .iter()
                .find(|lat| lat.tool_name == name)
                .unwrap_or_else(|| panic!("{name} latency missing"))
        };
        assert_eq!(lat("Bash").unmatched, 0);
        assert_eq!(lat("Bash").max_sec, 3.0);
        assert_eq!(lat("Read").unmatched, 0);
        assert_eq!(lat("Read").max_sec, 7.0);
    }

    #[test]
    fn claude_flat_transcript_missing_result_is_not_masked_by_another_tool() {
        // Review R2-3 consequence (2): with global ordinals, one Bash
        // call plus two Read results kept every entry paired -- Bash's
        // genuinely missing result was silently swallowed by Read's
        // surplus. Per-tool ordinals keep the defect visible: Bash is
        // unmatched, Read's second result is simply extra.
        let raw = concat!(
            "{\"type\":\"tool_use\",\"timestamp\":\"2026-01-01T00:00:01Z\",\"tool_name\":\"Bash\",\"tool_input\":{\"cmd\":\"ls a\"}}\n",
            "{\"type\":\"tool_use\",\"timestamp\":\"2026-01-01T00:00:02Z\",\"tool_name\":\"Read\",\"tool_input\":{\"path\":\"b\"}}\n",
            "{\"type\":\"tool_result\",\"timestamp\":\"2026-01-01T00:00:03Z\",\"tool_name\":\"Read\",\"tool_output\":\"ok\"}\n",
            "{\"type\":\"tool_result\",\"timestamp\":\"2026-01-01T00:00:04Z\",\"tool_name\":\"Read\",\"tool_output\":\"ok again\"}\n"
        );
        let session = parse_raw_session("mask", "mask.jsonl", raw).expect("flat session parses");
        let bash = session
            .diagnostics
            .tool_latencies
            .iter()
            .find(|lat| lat.tool_name == "Bash")
            .expect("Bash latency present");
        assert_eq!(bash.unmatched, 1, "a missing Bash result must stay visible");
    }

    #[test]
    fn claude_flat_transcript_diagnoses_as_paired_end_to_end() {
        // rm-230 (campaign-local rm-025) acceptance, through the real
        // entry point: a flat transcript whose calls and results pair
        // (explicit ids, or positional for id-less entries) reports ZERO
        // unmatched calls in session diagnostics. Before the fix every
        // flat call was unmatched.
        let raw = concat!(
            "{\"type\":\"tool_use\",\"timestamp\":\"2026-01-01T00:00:01Z\",\"tool_name\":\"Bash\",\"tool_input\":{\"cmd\":\"ls a\"},\"tool_use_id\":\"tu-1\"}\n",
            "{\"type\":\"tool_result\",\"timestamp\":\"2026-01-01T00:00:04Z\",\"tool_name\":\"Bash\",\"tool_output\":\"ok\",\"tool_use_id\":\"tu-1\"}\n",
            "{\"type\":\"tool_use\",\"timestamp\":\"2026-01-01T00:00:05Z\",\"tool_name\":\"Bash\",\"tool_input\":{\"cmd\":\"ls a\"}}\n",
            "{\"type\":\"tool_result\",\"timestamp\":\"2026-01-01T00:00:08Z\",\"tool_name\":\"Bash\",\"tool_output\":\"ok\"}\n"
        );
        let session = parse_raw_session("flat", "flat.jsonl", raw).expect("flat session parses");
        let latencies = &session.diagnostics.tool_latencies;
        assert_eq!(latencies.len(), 1);
        assert_eq!(latencies[0].tool_name, "Bash");
        assert_eq!(latencies[0].count, 2);
        assert_eq!(latencies[0].unmatched, 0);
        // Review F10 (rm-230 acceptance): pin the escalation surface the
        // acceptance names. session_findings escalates latency only when
        // max_sec >= 5.0 OR unmatched > 0; on this fully-paired corpus
        // (unmatched 0, every latency < 5s) no "latency" finding may
        // appear -- before the join fix this corpus escalated (unmatched
        // fed the high-severity filter in session_findings).
        let findings = crate::session_findings(&session, &[]);
        assert!(
            findings.iter().all(|finding| finding.kind != "latency"),
            "a fully-paired flat transcript must not escalate a latency finding"
        );
    }

    #[test]
    fn utf16_files_fail_with_a_named_encoding_error() {
        // Pass-7 P7-2: PowerShell's default `>` redirection produces
        // UTF-16; the failure must name the encoding and the fix, not a
        // generic invalid-UTF-8 read error.
        let root = std::env::temp_dir().join(format!(
            "agenttrace-utf16-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        fs::create_dir_all(&root).expect("create temp dir");
        let file = root.join("utf16.jsonl");
        // UTF-16LE BOM + `"x"` payload.
        fs::write(&file, [0xFF, 0xFE, 0x22, 0x00, 0x78, 0x00, 0x22, 0x00])
            .expect("write utf16 file");
        let err = parse_file(&file).expect_err("utf16 file must be rejected with a named error");
        let message = err.to_string();
        assert!(
            message.contains("UTF-16"),
            "error must name the encoding, got: {message}"
        );
        assert!(
            message.contains("convert"),
            "error must name the remedy, got: {message}"
        );
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn escaped_backslash_pairs_never_mask_surrogate_repair() {
        // Pass-7 residual on the cycle-3 repair: literal `\\ud800` text
        // (an escaped backslash followed by `ud800`) used to be mistaken
        // for a lone-surrogate escape and rewritten, corrupting the
        // literal while the real surrogate sat a field later.
        let line = r#"{"a":"\\ud800","b":"\ud800"}"#;
        let repaired = repair_lone_surrogates(line).expect("real lone surrogate still repairs");
        assert_eq!(repaired, r#"{"a":"\\ud800","b":"\ufffd"}"#);
    }

    #[test]
    fn antigravity_trajectory_sidecar_parses_user_planner_and_commands() {
        // CU-19: `<uuid>.trajectory.json` sidecars under
        // ~/.gemini/antigravity-cli/conversations/ carry CORTEX_STEP_TYPE_*
        // steps. The on-disk store (.db/.pb) is rejected upstream; this is
        // the documented reader surface (agy-reader types.go, agy 1.1.23).
        let raw = serde_json::json!({
            "trajectoryId": "11111111-2222-3333-4444-555555555555",
            "trajectoryType": "CORTEX_TRAJECTORY_TYPE_MAIN",
            "steps": [
                {
                    "type": "CORTEX_STEP_TYPE_USER_INPUT",
                    "status": "COMPLETED",
                    "metadata": {"createdAt": "2026-09-01T10:00:00Z"},
                    "userInput": {"userResponse": "Ship the fix"}
                },
                {
                    "type": "CORTEX_STEP_TYPE_PLANNER_RESPONSE",
                    "status": "COMPLETED",
                    "metadata": {"createdAt": "2026-09-01T10:00:05Z"},
                    "plannerResponse": {
                        "response": "Running the failing test first.",
                        "thinking": "The regression came from the cache eviction.",
                        "toolCalls": [
                            {
                                "name": "run_commands",
                                "argumentsJson": "{\"commands\":[\"cargo test\"]}"
                            }
                        ]
                    }
                },
                {
                    "type": "CORTEX_STEP_TYPE_RUN_COMMAND",
                    "status": "COMPLETED",
                    "metadata": {"createdAt": "2026-09-01T10:00:06Z"},
                    "runCommand": {"commandLine": "cargo test", "exitCode": 2}
                },
                {
                    "type": "CORTEX_STEP_TYPE_CHECKPOINT",
                    "status": "COMPLETED",
                    "metadata": {"createdAt": "2026-09-01T10:00:07Z"}
                }
            ]
        })
        .to_string();
        let session = parse_raw_session("traj", "traj.trajectory.json", &raw)
            .expect("trajectory sidecar parses");
        assert!(session.metrics.user_messages >= 1);
        assert!(session.metrics.assistant_turns >= 1);
        assert!(session.metrics.tool_calls_total >= 1);
        assert!(session.metrics.tool_results >= 1);
        assert!(
            session.metrics.tool_calls_fail >= 1,
            "non-zero exit code must count as a failed tool call"
        );
        let events = parse_antigravity_trajectory(&raw).expect("events");
        let planner = events
            .iter()
            .find(|event| event.role == "assistant")
            .expect("planner response survives as an assistant event");
        assert!(
            planner.reasoning.contains("cache eviction"),
            "planner thinking must land in the reasoning field"
        );
        assert!(
            planner.tool_calls[0].args.contains("cargo test"),
            "argumentsJson must be parsed from its JSON string: {:?}",
            planner.tool_calls[0].args
        );
        assert!(events
            .iter()
            .all(|event| event.source_tool == "antigravity_cli"));
    }

    #[test]
    fn antigravity_view_file_accepts_wire_case_path_key() {
        // Pass-10 F1: the daemon serializes the field as
        // `json:"absolutePathUri"` (types.go); the upper-case `URI`
        // spelling is kept only as a tolerated alias. Real sidecars must
        // never silently drop VIEW_FILE steps.
        let step = |key: &str| {
            let mut view_file = serde_json::Map::new();
            view_file.insert(key.to_string(), serde_json::json!("file:///work/x.rs"));
            view_file.insert("startLine".to_string(), serde_json::json!(1));
            view_file.insert("endLine".to_string(), serde_json::json!(2));
            serde_json::json!({
                "trajectoryId": "11111111-2222-3333-4444-555555555555",
                "trajectoryType": "CORTEX_TRAJECTORY_TYPE_MAIN",
                "steps": [
                    {
                        "type": "CORTEX_STEP_TYPE_VIEW_FILE",
                        "status": "COMPLETED",
                        "metadata": {"createdAt": "2026-09-01T10:00:07Z"},
                        "viewFile": view_file
                    }
                ]
            })
            .to_string()
        };
        for key in ["absolutePathUri", "absolutePathURI"] {
            let events =
                parse_antigravity_trajectory(&step(key)).expect("view-file sidecar parses");
            let view = events
                .iter()
                .find(|event| event.role == "tool")
                .unwrap_or_else(|| panic!("VIEW_FILE step with `{key}` must yield a tool event"));
            assert_eq!(
                view.content, "view file:///work/x.rs lines 1-2",
                "wire-case key `{key}` must carry path and line range"
            );
            assert!(!view.is_error);
        }
    }

    #[test]
    fn antigravity_trajectory_sniff_rejects_generic_steps_arrays() {
        // `steps` alone is not evidence: countless JSON payloads nest a
        // steps array without CORTEX_STEP_TYPE markers or createdAt
        // metadata. The sniff requires both.
        let generic = serde_json::json!({
            "steps": [
                {"type": "build", "command": "make"},
                {"type": "test", "command": "make check"}
            ]
        })
        .to_string();
        assert!(
            parse_raw_session("generic", "generic.trajectory.json", &generic).is_err(),
            "generic steps arrays must fall through to the unsupported-format error"
        );
        assert!(parse_antigravity_trajectory(&generic).is_none());
    }

    #[test]
    fn antigravity_trajectory_folds_usage_blocks_and_model_id() {
        // rm-720 / codeburn #1655: planner generations carry
        // ModelUsageStats under the RPC JSON names; the fold sums them
        // into one meta event (field 2 uncached input, field 5 cache read —
        // delta basis, no clamp) and passes the standalone app's
        // config-id model through verbatim (pricing resolves or discloses
        // — never silently $0).
        let raw = serde_json::json!({
            "trajectoryId": "11111111-2222-3333-4444-555555555555",
            "steps": [
                {
                    "type": "CORTEX_STEP_TYPE_PLANNER_RESPONSE",
                    "status": "COMPLETED",
                    "metadata": {"createdAt": "2026-09-01T10:00:05Z"},
                    "plannerResponse": {
                        "response": "First generation.",
                        "usageStats": {
                            "inputTokens": 100,
                            "outputTokens": 50,
                            "cacheReadInputTokens": 200
                        },
                        "modelConfigId": "gemini-pro-default"
                    }
                },
                {
                    "type": "CORTEX_STEP_TYPE_PLANNER_RESPONSE",
                    "status": "COMPLETED",
                    "metadata": {"createdAt": "2026-09-01T10:00:09Z"},
                    "plannerResponse": {
                        "response": "Second generation.",
                        "usageMetadata": {
                            "inputTokens": 30,
                            "outputTokens": 20,
                            "cachedContentTokenCount": 7,
                            "reasoningTokens": 10
                        },
                        "modelConfigId": "gemini-pro-default"
                    }
                }
            ]
        })
        .to_string();
        let session = parse_raw_session("traj", "traj.trajectory.json", &raw)
            .expect("usage-bearing trajectory parses");
        assert_eq!(session.metrics.tokens_input, 130, "summed generations");
        assert_eq!(
            session.metrics.tokens_output, 80,
            "reasoning bills at the output rate (50 + 20 + 10)"
        );
        assert_eq!(session.metrics.tokens_reasoning, 10);
        assert_eq!(
            session.metrics.tokens_cache_r, 207,
            "cacheReadInputTokens and cachedContentTokenCount are one basis"
        );
        assert_eq!(
            session.metrics.model_used, "gemini-pro-default",
            "config id passes through verbatim"
        );
        assert_eq!(
            session
                .metrics
                .disclosure_counters
                .get("antigravity_usage_basis:planner_response_summed"),
            Some(&2),
            "the fold names its basis"
        );
    }

    #[test]
    fn antigravity_jsonl_folds_usage_blocks() {
        // rm-720: the jsonl lane folds PLANNER_RESPONSE usage blocks the
        // same way; `model_enum` (Field 20) is the standalone id shape
        // when the config id is absent.
        let raw = [
            r#"{"type":"USER_INPUT","step_index":0,"created_at":"2026-09-01T10:00:00Z","content":"Ship it"}"#,
            r#"{"type":"PLANNER_RESPONSE","step_index":1,"created_at":"2026-09-01T10:00:05Z","content":"Done","usage":{"inputTokens":40,"outputTokens":10,"cacheReadInputTokens":7},"model_enum":"MODEL_PLACEHOLDER_M16"}"#,
        ]
        .join("\n");
        let session = parse_raw_session("ag", "x.jsonl", &raw).expect("jsonl parses");
        assert_eq!(session.metrics.tokens_input, 40);
        assert_eq!(session.metrics.tokens_output, 10);
        assert_eq!(session.metrics.tokens_cache_r, 7);
        assert_eq!(session.metrics.model_used, "MODEL_PLACEHOLDER_M16");
        assert_eq!(
            session.metrics.source_tool, "antigravity_cli",
            "the lane still claims the file"
        );
    }

    #[test]
    fn thinking_tokens_fold_into_output_and_break_out_at_every_usage_site() {
        // CU-20: thoughtsTokenCount (Gemini), thinkingTokenCount and the
        // OpenAI-compatible reasoning aliases are billed at the output
        // rate, so they fold into output_tokens and are additionally
        // reported as reasoning_tokens for the audit.
        let qwen = qwen_usage(Some(&serde_json::json!({
            "output_tokens": 10,
            "reasoning_tokens": 5
        })))
        .expect("qwen usage");
        assert_eq!(qwen["output_tokens"], 15);
        assert_eq!(qwen["reasoning_tokens"], 5);

        let gemini = gemini_usage(&serde_json::json!({
            "promptTokenCount": 100,
            "candidatesTokenCount": 20,
            "thoughtsTokenCount": 40
        }))
        .expect("gemini usage");
        assert_eq!(gemini["input_tokens"], 100);
        assert_eq!(gemini["output_tokens"], 60);
        assert_eq!(gemini["reasoning_tokens"], 40);

        let generic = usage_from_value(&serde_json::json!({
            "prompt_tokens": 10,
            "completion_tokens": 7,
            "thinking_tokens": 3
        }))
        .expect("generic usage");
        assert_eq!(generic["output_tokens"], 10);
        assert_eq!(generic["reasoning_tokens"], 3);

        // Zero thinking tokens leave the map unchanged: corpora without
        // thinking models must not grow a spurious key.
        let bare = usage_from_value(&serde_json::json!({
            "prompt_tokens": 10,
            "completion_tokens": 7
        }))
        .expect("bare usage");
        assert_eq!(bare["output_tokens"], 7);
        assert!(!bare.contains_key("reasoning_tokens"));
    }

    #[test]
    fn thinking_tokens_reach_the_session_metrics_breakdown() {
        // End to end through the Gemini checkpoint parser: the folded
        // number lands in metrics.tokens_output and the breakdown in
        // metrics.tokens_reasoning (pass-9 CU-20).
        let raw = serde_json::json!({
            "checkpoint": {
                "model": "gemini-2.5-flash",
                "conversation": [
                    {
                        "role": "user",
                        "timestamp": "2026-01-02T10:00:00Z",
                        "parts": [{"text": "Resume this checkpoint."}]
                    }
                ],
                "tokenUsage": {
                    "inputTokens": 80,
                    "outputTokens": 20,
                    "thoughtsTokenCount": 40
                }
            }
        })
        .to_string();
        let session = parse_raw_session("cp", "chats/cp.json", &raw).expect("checkpoint parses");
        assert_eq!(session.metrics.tokens_input, 80);
        assert_eq!(
            session.metrics.tokens_output, 60,
            "thoughtsTokenCount is billed at the output rate and must fold in"
        );
        assert_eq!(session.metrics.tokens_reasoning, 40);
    }

    #[test]
    fn oh_my_pi_usage_picks_one_alias_per_class_never_the_sum() {
        // rm-618: the wire writes the same token class under two spellings;
        // the old alias SUM double-counted every journal carrying both
        // (this shape reported 200/100/60/20 pre-fix).
        let value = serde_json::json!({
            "input": 100,
            "input_tokens": 100,
            "output": 50,
            "output_tokens": 50,
            "cacheRead": 30,
            "cache_read_input_tokens": 30,
            "cacheWrite": 10,
            "cache_creation_input_tokens": 10,
        });
        let usage = oh_my_pi_usage(Some(&value)).expect("usage map");
        assert_eq!(usage.get("input_tokens"), Some(&100));
        assert_eq!(usage.get("output_tokens"), Some(&50));
        assert_eq!(usage.get("cache_read_input_tokens"), Some(&30));
        assert_eq!(usage.get("cache_creation_input_tokens"), Some(&10));
    }

    #[test]
    fn claude_reemission_folds_real_shapes_and_counts_relays() {
        // rm-905: genuine streaming keeps the msg_* id shape and
        // non-decreasing totals; anything else under a repeated id is a
        // distinct response.
        let folded = BTreeMap::from([
            ("input_tokens".to_string(), 1000),
            ("output_tokens".to_string(), 200),
        ]);
        let reemit = BTreeMap::from([
            ("input_tokens".to_string(), 1000),
            ("output_tokens".to_string(), 400),
        ]);
        assert!(claude_reemission("msg_01XFDUDYJgAAC", &folded, &reemit));
        // Duplicate emission of the same snapshot is still a re-emission.
        assert!(claude_reemission("msg_01XFDUDYJgAAC", &folded, &folded));
        // Foreign id shape: ccusage #1635's relay answers with one id.
        let relay = BTreeMap::from([
            ("input_tokens".to_string(), 100),
            ("output_tokens".to_string(), 50),
        ]);
        assert!(!claude_reemission("ocgo", &folded, &relay));
        // `msg_` alone is not a real id; `-`/`_` inside the suffix ARE legal
        // in real ids, so they do not flip a genuine shape to reuse.
        assert!(!claude_reemission("msg_", &BTreeMap::new(), &relay));
        assert!(claude_reemission("msg_oc-go_x", &BTreeMap::new(), &relay));
        // Real shape, but a class went DOWN: an independent response, not a
        // running total.
        let rewind = BTreeMap::from([
            ("input_tokens".to_string(), 1000),
            ("output_tokens".to_string(), 120),
        ]);
        assert!(!claude_reemission("msg_01XFDUDYJgAAC", &folded, &rewind));
        // A class only the later emission reports folds in (or_insert(0)).
        let growing = BTreeMap::from([
            ("input_tokens".to_string(), 1000),
            ("output_tokens".to_string(), 400),
            ("cache_read_input_tokens".to_string(), 12),
        ]);
        assert!(claude_reemission("msg_01XFDUDYJgAAC", &folded, &growing));
    }

    #[test]
    fn workbuddy_usage_survives_negative_input_with_cache_read() {
        // input_tokens clamps to i64::MIN for adversarial magnitudes; the
        // cache subtraction previously underflowed there (debug panic,
        // release wrap to a huge positive).
        let value = serde_json::json!({
            "message": {
                "usage": {
                    "input_tokens": i64::MIN,
                    "output_tokens": 10,
                    "cache_read_input_tokens": 5
                }
            }
        });
        let (usage, clamped) = workbuddy_usage(value.as_object().expect("object")).expect("usage");
        assert_eq!(usage.get("input_tokens"), Some(&0));
        assert_eq!(usage.get("cache_read_input_tokens"), Some(&0));
        assert_eq!(usage.get("output_tokens"), Some(&10));
        // The negative total clamps the cached count to zero and flags
        // the clamp (rm-600, upstream #316 shape).
        assert_eq!(usage.get("cache_read_input_tokens"), Some(&0));
        assert!(clamped);
    }

    #[test]
    fn subtract_cached_input_clamps_the_cached_count_to_the_input() {
        // Upstream #316 live PoC (wb7): a record reports 150 cached
        // tokens against 100 input — the session total (0 + 150 = 150)
        // previously EXCEEDED the source-recorded input. Both sides
        // clamp: input keeps 0 uncached, the cached count drops to 100,
        // and the sum equals the recorded input exactly.
        let mut usage: BTreeMap<String, i64> = BTreeMap::from([
            ("input_tokens".to_string(), 100),
            ("output_tokens".to_string(), 10),
            ("cache_read_input_tokens".to_string(), 150),
        ]);
        assert!(subtract_cached_input(
            &mut usage,
            &["cache_read_input_tokens"]
        ));
        assert_eq!(usage.get("input_tokens"), Some(&0));
        assert_eq!(usage.get("cache_read_input_tokens"), Some(&100));

        // In-range records are untouched: cached strictly below input.
        let mut fine: BTreeMap<String, i64> = BTreeMap::from([
            ("input_tokens".to_string(), 2000),
            ("cache_read_input_tokens".to_string(), 1500),
        ]);
        assert!(!subtract_cached_input(
            &mut fine,
            &["cache_read_input_tokens"]
        ));
        assert_eq!(fine.get("input_tokens"), Some(&500));
        assert_eq!(fine.get("cache_read_input_tokens"), Some(&1500));

        // A hostile negative cached count contributes nothing and never
        // inflates input.
        let mut hostile: BTreeMap<String, i64> = BTreeMap::from([
            ("input_tokens".to_string(), 100),
            ("cache_read_input_tokens".to_string(), -50),
        ]);
        assert!(!subtract_cached_input(
            &mut hostile,
            &["cache_read_input_tokens"]
        ));
        assert_eq!(hostile.get("input_tokens"), Some(&100));
        assert_eq!(hostile.get("cache_read_input_tokens"), Some(&0));

        // No cached count at all: untouched.
        let mut plain: BTreeMap<String, i64> = BTreeMap::from([("input_tokens".to_string(), 42)]);
        assert!(!subtract_cached_input(
            &mut plain,
            &["cache_read_input_tokens"]
        ));
        assert_eq!(plain.get("input_tokens"), Some(&42));
    }

    #[test]
    fn add_usage_into_sums_saturating_per_key() {
        let mut total: BTreeMap<String, i64> = BTreeMap::from([("input_tokens".to_string(), 100)]);
        add_usage_into(
            &mut total,
            BTreeMap::from([
                ("input_tokens".to_string(), 200),
                ("output_tokens".to_string(), 20),
            ]),
        );
        add_usage_into(
            &mut total,
            BTreeMap::from([("input_tokens".to_string(), 300)]),
        );
        assert_eq!(total.get("input_tokens"), Some(&600));
        assert_eq!(total.get("output_tokens"), Some(&20));
        add_usage_into(
            &mut total,
            BTreeMap::from([("input_tokens".to_string(), i64::MAX)]),
        );
        assert_eq!(total.get("input_tokens"), Some(&i64::MAX));
    }

    #[test]
    fn workbuddy_usage_clamps_cache_read_above_input() {
        // WorkBuddy `input_tokens` is cache-INCLUSIVE. A cache_read above the
        // reported input is a basis mismatch: the cached part must clamp to
        // the input so the session total never exceeds what the source
        // recorded (100 vs the old 0 + 150 = 150) — port of upstream PR #316.
        let value = serde_json::json!({
            "message": {
                "usage": {
                    "input_tokens": 100,
                    "output_tokens": 20,
                    "cache_read_input_tokens": 150
                }
            }
        });
        let (usage, clamped) = workbuddy_usage(value.as_object().expect("object")).expect("usage");
        assert_eq!(usage.get("input_tokens"), Some(&0));
        assert_eq!(usage.get("cache_read_input_tokens"), Some(&100));
        assert_eq!(usage.get("output_tokens"), Some(&20));
        assert!(clamped);
    }

    #[test]
    fn number_as_i64_saturates_extreme_numbers_instead_of_wrapping() {
        // u64::MAX previously wrapped to -1 through `as i64`, letting a
        // corrupt log flip token totals negative; it now saturates.
        let over_i64 = serde_json::json!(9_223_372_036_854_775_808u64);
        assert_eq!(number_as_i64(&over_i64), Some(i64::MAX));
        let u64_max = serde_json::json!(u64::MAX);
        assert_eq!(number_as_i64(&u64_max), Some(i64::MAX));
        // 1e300 exceeds i64 range; the f64 cast saturates at the bounds
        // rather than producing a garbage in-range value.
        let huge_float = serde_json::json!(1e300);
        assert_eq!(number_as_i64(&huge_float), Some(i64::MAX));
        let huge_negative = serde_json::json!(-1e300);
        assert_eq!(number_as_i64(&huge_negative), Some(i64::MIN));
        // In-range values keep exact parsing.
        assert_eq!(number_as_i64(&serde_json::json!(42)), Some(42));
        assert_eq!(number_as_i64(&serde_json::json!("42")), Some(42));
    }

    #[test]
    fn repair_lone_surrogates_never_slices_mid_character() {
        // Pass-6 P6-1: a rejected `\u` escape followed by multi-byte
        // UTF-8 inside the next four bytes used to slice `line` at a
        // non-char-boundary and panic. The hex must be read from bytes:
        // when the four bytes are not ASCII hex the escape passes through
        // untouched (None = unchanged), never a panic.
        assert_eq!(repair_lone_surrogates(r#"{"prompt":"\u中文测试"}"#), None);
        // High surrogate whose would-be pair hex contains multi-byte
        // UTF-8: the pair check must fail safely and the lone surrogate
        // becomes U+FFFD while the rest of the line is copied verbatim.
        assert_eq!(
            repair_lone_surrogates(r#"{"prompt":"\ud800\u中文测试"}"#).as_deref(),
            Some(r#"{"prompt":"\ufffd\u中文测试"}"#)
        );
        // Non-hex ASCII after `\u` passes through unchanged.
        assert_eq!(
            repair_lone_surrogates(r#"{"prompt":"\uzzzz not hex"}"#),
            None
        );
        // Truncated escape at end of line passes through unchanged.
        assert_eq!(
            repair_lone_surrogates(r#"{"prompt":"truncated \u4e2"}"#),
            None
        );
        // Valid escapes stay untouched (no change -> None).
        assert_eq!(
            repair_lone_surrogates(r#"{"prompt":"\u00e9 valid bmp escape"}"#),
            None
        );
        assert_eq!(
            repair_lone_surrogates(r#"{"prompt":"\ud83d\ude0e emoji pair only"}"#),
            None
        );
        // A lone surrogate half still becomes U+FFFD.
        assert_eq!(
            repair_lone_surrogates(r#"{"prompt":"lone \ud800 half"}"#).as_deref(),
            Some(r#"{"prompt":"lone \ufffd half"}"#)
        );
        assert_eq!(
            repair_lone_surrogates(r#"{"prompt":"lone \udc00 low half"}"#).as_deref(),
            Some(r#"{"prompt":"lone \ufffd low half"}"#)
        );
    }

    #[test]
    fn lenient_parse_repairs_lone_surrogates_and_keeps_valid_pairs() {
        // End to end through the lenient path: the lone surrogate is
        // replaced, the valid pair survives as one character, and the
        // value is a real object again.
        let value =
            parse_jsonl_value_lenient(r#"{"prompt":"lone \ud800 with pair \ud83d\ude00 end"}"#)
                .expect("repaired line must parse");
        assert_eq!(
            value.get("prompt").and_then(|p| p.as_str()),
            Some("lone \u{fffd} with pair \u{1f600} end")
        );
        // The P6-1 reproducers must degrade to "unparseable line", not
        // panic.
        assert!(parse_jsonl_value_lenient(r#"{"prompt":"\u中文测试"}"#).is_none());
        assert!(parse_jsonl_value_lenient(r#"{"prompt":"\uzzzz not hex"}"#).is_none());
        assert!(parse_jsonl_value_lenient(r#"{"prompt":"truncated \u4e2"}"#).is_none());
    }

    #[test]
    fn numeric_string_usage_coerces_like_numbers() {
        // rm-342 (run 71a7d5db, cycle 3): usage counters serialized as
        // strings must contribute exactly like the identical JSON Number.
        // Integer strings already parsed; float-form, padded, and
        // beyond-i64::MAX strings used to drop silently while their
        // Number twins coerced.
        assert_eq!(number_as_i64(&Value::from(100i64)), Some(100));
        assert_eq!(number_as_i64(&Value::from(100.5)), Some(100));
        assert_eq!(
            number_as_i64(&Value::from(9223372036854775808u64)),
            Some(i64::MAX)
        );
        assert_eq!(number_as_i64(&Value::String("100".to_string())), Some(100));
        assert_eq!(
            number_as_i64(&Value::String(" 100 ".to_string())),
            Some(100)
        );
        assert_eq!(
            number_as_i64(&Value::String("100.5".to_string())),
            Some(100)
        );
        assert_eq!(number_as_i64(&Value::String("1e3".to_string())), Some(1000));
        assert_eq!(
            number_as_i64(&Value::String("9223372036854775808".to_string())),
            Some(i64::MAX)
        );
        // Not numeric: booleans and prose carry no count to honor.
        assert_eq!(number_as_i64(&Value::Bool(true)), None);
        assert_eq!(number_as_i64(&Value::String("true".to_string())), None);
        assert_eq!(number_as_i64(&Value::String("lots".to_string())), None);
    }

    #[test]
    fn pi_usage_with_string_counts_contributes_tokens() {
        // rm-342 end-to-end pin on the assess N9 PoC shape: input
        // "100" (string) + output 50 (number) must total 150 — the
        // counts are honored, never silently dropped.
        let usage = serde_json::json!({
            "input_tokens": "100",
            "output_tokens": 50,
        });
        let map = oh_my_pi_usage(Some(&usage)).expect("usage map");
        assert_eq!(map.get("input_tokens"), Some(&100));
        assert_eq!(map.get("output_tokens"), Some(&50));
    }

    #[test]
    fn qwen_predicate_rejects_claude_only_markers() {
        // rm-345: privacy-scrubbed claude-code exports drop sessionId but keep
        // claude-only keys. The uuid branch of is_qwen_code_event used to
        // classify any sessionId-less `message`/`result`/`subtype` object as
        // qwen_code, silently misattributing those exports. Any claude-only
        // marker must disqualify the object.
        let claude_only_markers = [
            "parentUuid",
            "isSidechain",
            "toolUseResult",
            "requestId",
            "promptId",
        ];
        for marker in claude_only_markers {
            let mut obj = Map::new();
            obj.insert("type".into(), Value::String("user".into()));
            obj.insert("uuid".into(), Value::String("u-1".into()));
            obj.insert("message".into(), serde_json::json!({"role": "user"}));
            obj.insert(marker.into(), Value::Null);
            assert!(
                !is_qwen_code_event(&obj),
                "claude-only marker `{marker}` must disqualify a claude export"
            );
        }

        // A claude model string on a uuid-keyed object is just as
        // disqualifying as the keys above — it survives sessionId scrubbing
        // and qwen_code never stamps claude models.
        let mut claude_model = Map::new();
        claude_model.insert("type".into(), Value::String("assistant".into()));
        claude_model.insert("uuid".into(), Value::String("u-3".into()));
        claude_model.insert(
            "message".into(),
            serde_json::json!({"role": "assistant", "model": "claude-opus-4.6"}),
        );
        assert!(!is_qwen_code_event(&claude_model));

        // Genuine qwen_code events (uuid + message, no claude-only keys,
        // no sessionId, no claude model) must still match.
        let mut genuine = Map::new();
        genuine.insert("type".into(), Value::String("user".into()));
        genuine.insert("uuid".into(), Value::String("u-2".into()));
        genuine.insert(
            "message".into(),
            serde_json::json!({"role": "user", "content": "hi"}),
        );
        assert!(is_qwen_code_event(&genuine));

        // A uuid-keyed object naming a qwen model still matches.
        let mut qwen_model = Map::new();
        qwen_model.insert("type".into(), Value::String("assistant".into()));
        qwen_model.insert("uuid".into(), Value::String("u-4".into()));
        qwen_model.insert(
            "message".into(),
            serde_json::json!({"role": "assistant", "model": "qwen3-coder-plus"}),
        );
        assert!(is_qwen_code_event(&qwen_model));

        // And the sessionId fast path is untouched.
        let mut session_id = Map::new();
        session_id.insert("type".into(), Value::String("user".into()));
        session_id.insert("session_id".into(), Value::String("s-1".into()));
        assert!(is_qwen_code_event(&session_id));
    }

    #[test]
    fn claude_export_stripped_of_session_id_is_not_qwen_code() {
        // rm-345 end-to-end: a claude-code transcript scrubbed of sessionId
        // keeps claude-only keys (parentUuid here), so is_qwen_code_jsonl must
        // not claim the file and parse_claude_code_jsonl must — the session
        // classifies claude_code instead of qwen_code.
        let dir = std::env::temp_dir().join("agenttrace-rm345-claude-stripped");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("stripped.jsonl");
        std::fs::write(
            &path,
            "{\"uuid\":\"a1\",\"parentUuid\":\"a0\",\"type\":\"user\",\"message\":{\"role\":\"user\",\"content\":\"do the thing\"}}\n",
        )
        .unwrap();

        let parsed = parse_file(&path).expect("claude transcript must parse");
        assert_eq!(
            parsed.metrics.source_tool, "claude_code",
            "sessionId-stripped claude export must not classify qwen_code"
        );

        // The marker-free scrub shape from the live PoC: no claude-only keys,
        // but the assistant line still stamps a claude model string.
        let model_only = dir.join("stripped-model-only.jsonl");
        std::fs::write(
            &model_only,
            "{\"type\":\"user\",\"uuid\":\"b1\",\"timestamp\":\"2026-05-03T10:00:00Z\",\"message\":{\"role\":\"user\",\"content\":\"degraded claude\"}}\n\
             {\"type\":\"assistant\",\"uuid\":\"b2\",\"timestamp\":\"2026-05-03T10:00:01Z\",\"message\":{\"role\":\"assistant\",\"model\":\"claude-opus-4.6\",\"content\":[{\"type\":\"text\",\"text\":\"reply\"}]}}\n",
        )
        .unwrap();
        let parsed = parse_file(&model_only).expect("marker-free transcript must parse");
        assert_eq!(
            parsed.metrics.source_tool, "claude_code",
            "a claude model string must disqualify the qwen uuid branch"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn claude_code_git_branch_rides_the_session_meta_event() {
        // rm-585 (spend-by-branch, first cut): the claude-code envelope
        // carries gitBranch beside cwd on every line; the session must
        // expose it so the by_branch rollup can attribute spend. The
        // detached-HEAD literal "HEAD" is preserved verbatim here (the
        // overview normalizes it to the "unknown" bucket), and a
        // journal with no gitBranch parses with an empty branch —
        // never a panic, never an invented value.
        let dir = std::env::temp_dir().join("agenttrace-rm585-git-branch");
        std::fs::create_dir_all(&dir).unwrap();

        let branchy = dir.join("branchy.jsonl");
        std::fs::write(
            &branchy,
            "{\"type\":\"user\",\"uuid\":\"u1\",\"parentUuid\":\"u0\",\"timestamp\":\"2026-10-07T09:00:00Z\",\"cwd\":\"/work/proj\",\"gitBranch\":\"feature/rm585\",\"sessionId\":\"s1\",\"message\":{\"role\":\"user\",\"content\":\"on a branch\"}}\n",
        )
        .unwrap();
        let parsed = parse_file(&branchy).expect("claude journal parses");
        assert_eq!(
            parsed.metrics.source_tool, "claude_code",
            "parentUuid keeps the journal on the claude lane"
        );
        assert_eq!(parsed.branch, "feature/rm585");
        assert_eq!(parsed.cwd, "/work/proj");

        let detached = dir.join("detached.jsonl");
        std::fs::write(
            &detached,
            "{\"type\":\"user\",\"uuid\":\"u2\",\"parentUuid\":\"u1\",\"timestamp\":\"2026-10-07T09:01:00Z\",\"cwd\":\"/work/proj\",\"gitBranch\":\"HEAD\",\"sessionId\":\"s2\",\"message\":{\"role\":\"user\",\"content\":\"detached\"}}\n",
        )
        .unwrap();
        let parsed = parse_file(&detached).expect("detached-HEAD journal parses");
        assert_eq!(parsed.branch, "HEAD");

        let bare = dir.join("bare.jsonl");
        std::fs::write(
            &bare,
            "{\"type\":\"user\",\"uuid\":\"u3\",\"parentUuid\":\"u2\",\"timestamp\":\"2026-10-07T09:02:00Z\",\"cwd\":\"/work/proj\",\"sessionId\":\"s3\",\"message\":{\"role\":\"user\",\"content\":\"no branch\"}}\n",
        )
        .unwrap();
        let parsed = parse_file(&bare).expect("branchless journal parses");
        assert_eq!(parsed.branch, "");

        let _ = std::fs::remove_dir_all(&dir);
    }

    // --- kimi_cli token_usage aliases (rm-400; renumbered at
    // integration from the campaign-local id) -------------------------

    #[test]
    fn kimi_statusupdate_usage_uses_official_wire_aliases() {
        // Official MoonshotAI/kimi-code wire fixture (MIT, 130 lines,
        // 15 usage-bearing StatusUpdate records): the token_usage wire
        // keys (input_other / output / input_cache_read /
        // input_cache_creation) matched no alias in usage_from_value, so
        // every record was dropped by the empty->None gate and the
        // session fell back to text-estimate totals (132) instead of the
        // reported 563,628.
        let raw = std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/kimi-cli/wire.jsonl"
        ))
        .expect("official kimi wire fixture ships with the crate");
        let session = parse_raw_session("kimi", "wire-official-fixture.jsonl", &raw)
            .expect("kimi session parses");
        assert_eq!(session.metrics.provenance.tokens, "reported_by_agent");
        assert_eq!(session.metrics.tokens_input, 62_198, "input_other sum");
        assert_eq!(session.metrics.tokens_output, 4_790, "output sum");
        assert_eq!(
            session.metrics.tokens_cache_r, 496_640,
            "input_cache_read sum"
        );
        assert_eq!(
            session.metrics.tokens_cache_w, 0,
            "input_cache_creation sum"
        );
        assert_eq!(
            session.metrics.tokens_input
                + session.metrics.tokens_output
                + session.metrics.tokens_cache_r
                + session.metrics.tokens_cache_w,
            563_628,
            "grand total across every component"
        );
    }

    #[test]
    fn kimi_statusupdate_usage_events_preserve_arrival_order() {
        // The StatusUpdate arm used events.insert(0, ..), so with more
        // than one usage-bearing record the journal's arrival order was
        // inverted. Totals are order-independent (meta usage is summed
        // per key in analyze), but anything walking events must see wire
        // order.
        let su = |input_other: i64, output: i64| {
            serde_json::json!({
                "timestamp": 1,
                "message": {"type": "StatusUpdate", "payload": {"token_usage": {
                    "input_other": input_other, "output": output,
                    "input_cache_read": 0, "input_cache_creation": 0
                }}}
            })
            .to_string()
        };
        let raw = [su(10, 1), su(20, 2)].join("\n");
        let objs = jsonl_objects(&raw).collect::<Vec<_>>();
        let events = parse_kimi_wire_jsonl(&objs).expect("kimi session parses").0;
        let usage: Vec<i64> = events
            .iter()
            .filter(|event| event.role == "meta" && !event.usage.is_empty())
            .map(|event| event.usage.get("input_tokens").copied().unwrap_or(-1))
            .collect();
        assert_eq!(
            usage,
            vec![10, 20],
            "usage events must keep wire arrival order"
        );
    }

    #[test]
    fn kimi_alias_matches_are_numeric_only_and_canonical_first() {
        // Canonical keys keep precedence over the kimi aliases, and the
        // bare "output" key only matches when numeric — a string like
        // "high" in another format's payload must never over-match
        // through number_as_i64.
        // Two lines so parse_raw_session dispatches through the jsonl path
        // (a single-line raw parses as one JSON document instead).
        let raw = [
            serde_json::json!({
                "timestamp": 1,
                "message": {"type": "StatusUpdate", "payload": {"token_usage": {
                    "input_tokens": 7, "input_other": 999,
                    "output": "high", "input_cache_read": 5, "input_cache_creation": 2
                }}}
            }),
            serde_json::json!({
                "timestamp": 2,
                "message": {"type": "SomethingElse", "payload": {}}
            }),
        ]
        .iter()
        .map(|value| value.to_string())
        .collect::<Vec<_>>()
        .join("\n");
        let session = parse_raw_session("kimi", "wire-alias-semantics.jsonl", &raw)
            .expect("kimi session parses");
        assert_eq!(session.metrics.provenance.tokens, "reported_by_agent");
        assert_eq!(
            session.metrics.tokens_input, 7,
            "canonical input_tokens wins over input_other"
        );
        assert_eq!(
            session.metrics.tokens_output, 0,
            "non-numeric output string never matches"
        );
        assert_eq!(session.metrics.tokens_cache_r, 5);
        assert_eq!(session.metrics.tokens_cache_w, 2);
    }

    #[test]
    fn kimi_usage_alias_matches_are_disclosed_in_parse_diagnostics() {
        // The alias table is a standing guess about another tool's wire
        // format; every alias-matched field is counted per session so a
        // future key change surfaces in the reports instead of as
        // silently-zero usage again. rm-719: the counters ride the
        // non-loss disclosure channel ("Disclosed facts", never
        // degrading data_health.confidence) — a wire-key match is an
        // informational fact about the vendor's journal, not parse loss.
        let raw = [
            serde_json::json!({
                "timestamp": 1,
                "message": {"type": "StatusUpdate", "payload": {"token_usage": {
                    "input_other": 11, "output": 3,
                    "input_cache_read": 5, "input_cache_creation": 0
                }}}
            }),
            serde_json::json!({
                "timestamp": 2,
                "message": {"type": "SomethingElse", "payload": {}}
            }),
        ]
        .iter()
        .map(|value| value.to_string())
        .collect::<Vec<_>>()
        .join("\n");
        let session = parse_raw_session("kimi", "wire-alias-disclosure.jsonl", &raw)
            .expect("kimi session parses");
        assert_eq!(
            session
                .metrics
                .disclosure_counters
                .get("kimi_usage_alias:input_other"),
            Some(&1)
        );
        assert_eq!(
            session
                .metrics
                .disclosure_counters
                .get("kimi_usage_alias:output"),
            Some(&1)
        );
        assert_eq!(
            session
                .metrics
                .disclosure_counters
                .get("kimi_usage_alias:input_cache_read"),
            Some(&1)
        );
        assert_eq!(
            session
                .metrics
                .disclosure_counters
                .get("kimi_usage_alias:input_cache_creation"),
            Some(&1),
            "a zero-valued matched field is still a wire-shape signal"
        );
        assert!(
            session.metrics.line_skips.is_empty(),
            "alias matches are not parse loss: line_skips {:#?}",
            session.metrics.line_skips
        );
    }

    // --- codex token_usage_record across compaction (rm-401;
    // renumbered at integration) ----------------------------------------

    fn codex_compaction_corpus(record_response_id: &str) -> String {
        // Cycle-2 research PoC corpus: a pre-compaction cumulative
        // snapshot, a compaction marker, the compaction turn's
        // token_usage_record (500 in / 300 cached / 200 out — never part
        // of any later cumulative), and a post-compaction snapshot. Before
        // the fix the parser counted only the snapshots (1,910) and the
        // record vanished without a trace; the truth is 2,610.
        let lines = [
            serde_json::json!({"timestamp":"2026-10-02T10:00:00Z","type":"session_meta","payload":{"model":"gpt-5.1-codex","cwd":"/home/user/proj"}}),
            serde_json::json!({"timestamp":"2026-10-02T10:00:01Z","type":"turn_context","payload":{"model":"gpt-5.1-codex","cwd":"/home/user/proj"}}),
            serde_json::json!({"timestamp":"2026-10-02T10:01:00Z","type":"event_msg","payload":{"type":"token_count","info":{"total_token_usage":{"input_tokens":1000,"cached_input_tokens":600,"output_tokens":400,"reasoning_output_tokens":150},"last_token_usage":{"input_tokens":1000,"cached_input_tokens":600,"output_tokens":400,"reasoning_output_tokens":150}}}}),
            serde_json::json!({"timestamp":"2026-10-02T10:01:30Z","type":"compacted","payload":{"compaction_response_id":"resp_comp_42","message":"Previous conversation compacted."}}),
            serde_json::json!({"timestamp":"2026-10-02T10:02:00Z","type":"token_usage_record","payload":{"response_id":record_response_id,"usage":{"input_tokens":500,"cached_input_tokens":300,"output_tokens":200},"request_id":"req_9f21","model":"gpt-5.1-codex"}}),
            serde_json::json!({"timestamp":"2026-10-02T10:03:00Z","type":"event_msg","payload":{"type":"token_count","info":{"total_token_usage":{"input_tokens":1200,"cached_input_tokens":800,"output_tokens":520,"reasoning_output_tokens":190},"last_token_usage":{"input_tokens":200,"cached_input_tokens":200,"output_tokens":120,"reasoning_output_tokens":40}}}}),
        ];
        lines
            .iter()
            .map(|value| value.to_string())
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[test]
    fn codex_compaction_token_usage_record_is_counted_once() {
        let session = parse_raw_session(
            "codex",
            "rollout-compaction-once.jsonl",
            &codex_compaction_corpus("resp_comp_42"),
        )
        .expect("codex rollout parses");
        // Snapshots: 400 net input + 600 cache + 400 out (+150 reasoning
        // on its own line), then a 200-climb giving 0 net input + 200
        // cache + 120 out (+40 reasoning). Compaction turn: 200 net input
        // + 300 cache + 200 out. Dated pin change 2026-10-06 (rm-553,
        // rebound rm-603 at integration, upstream #312): output dropped
        // 910 → 720 because reasoning is now a breakdown line (190)
        // instead of folded into output — the two decompositions cost
        // the same at the output rate.
        assert_eq!(session.metrics.tokens_input, 600);
        assert_eq!(session.metrics.tokens_cache_r, 1_100);
        assert_eq!(session.metrics.tokens_output, 720);
        assert_eq!(session.metrics.tokens_reasoning, 190);
        assert_eq!(
            session
                .metrics
                .line_skips
                .get("codex_compaction_usage_record"),
            Some(&1),
            "the counted record is visible in parse diagnostics"
        );
    }

    #[test]
    fn codex_compaction_usage_is_counted_once_when_copied_into_the_marker() {
        // Remote-compaction shape (upstream ccusage #1821): the compacted
        // payload carries latest_token_usage_record AND the same record
        // is replayed as a top-level token_usage_record with the same
        // response_id — the compaction turn's usage must be counted
        // exactly once, with the dedup decision disclosed.
        let lines = [
            serde_json::json!({"timestamp":"2026-10-01T09:00:00Z","type":"session_meta","payload":{"cwd":"/tmp/x","model":"gpt-5.3-codex"}}),
            serde_json::json!({"timestamp":"2026-10-01T09:00:01Z","type":"turn_context","payload":{"model":"gpt-5.3-codex"}}),
            serde_json::json!({"timestamp":"2026-10-01T09:00:05Z","type":"event_msg","payload":{"type":"token_count","info":{"total_token_usage":{"input_tokens":1000,"cached_input_tokens":0,"output_tokens":200,"reasoning_output_tokens":0}}}}),
            serde_json::json!({"timestamp":"2026-10-01T09:01:00Z","type":"compacted","payload":{"message":"Conversation compacted","compaction_response_id":"resp_compact_1","latest_token_usage_record":{"response_id":"resp_compact_1","usage":{"input_tokens":1500,"cached_input_tokens":800,"output_tokens":300,"reasoning_output_tokens":120}}}}),
            serde_json::json!({"timestamp":"2026-10-01T09:01:01Z","type":"token_usage_record","payload":{"response_id":"resp_compact_1","usage":{"input_tokens":1500,"cached_input_tokens":800,"output_tokens":300,"reasoning_output_tokens":120}}}),
        ];
        let raw = lines
            .iter()
            .map(|value| value.to_string())
            .collect::<Vec<_>>()
            .join("\n");
        let session = parse_raw_session("codex", "rollout-compaction-replayed.jsonl", &raw)
            .expect("codex rollout parses");
        // 1000 from the snapshot + 700 net from the record, exactly once.
        assert_eq!(session.metrics.tokens_input, 1_700);
        assert_eq!(session.metrics.tokens_cache_r, 800);
        assert_eq!(session.metrics.tokens_output, 620);
        assert_eq!(
            session
                .metrics
                .line_skips
                .get("codex_compaction_usage_record"),
            Some(&1)
        );
        assert_eq!(
            session
                .metrics
                .line_skips
                .get("codex_compaction_usage_duplicate"),
            Some(&1),
            "the replayed copy is disclosed as deduped, not silently eaten"
        );
    }

    #[test]
    fn codex_unpaired_token_usage_record_is_visible_but_not_counted() {
        // A token_usage_record whose response_id matches no compaction
        // marker describes a normal turn whose usage is already inside
        // the cumulative snapshots; counting it would double-count. The
        // old parser dropped the line with no trace at all — the class
        // this defect evaded — so the record stays visible as a counter.
        let session = parse_raw_session(
            "codex",
            "rollout-compaction-unpaired.jsonl",
            &codex_compaction_corpus("resp_ordinary"),
        )
        .expect("codex rollout parses");
        // Snapshot totals only (the 1,720 shape — dated pin change
        // 2026-10-06, rm-553 rebounded rm-603 at integration: 190
        // reasoning tokens now ride their own metrics line outside this
        // input+cache+output sum, where the
        // folded form counted them as 190 more output); the unpaired
        // record adds nothing.
        assert_eq!(
            session.metrics.tokens_input
                + session.metrics.tokens_cache_r
                + session.metrics.tokens_output,
            1_720
        );
        assert_eq!(
            session
                .metrics
                .line_skips
                .get("codex_token_usage_record_unpaired"),
            Some(&1)
        );
        assert!(!session
            .metrics
            .line_skips
            .contains_key("codex_compaction_usage_record"));
    }

    #[test]
    fn codex_compaction_record_does_not_advance_the_totals_baseline() {
        // The compaction turn's usage lives outside every later
        // cumulative snapshot, so it is added standalone — and it must
        // never touch the rm-554 totals state: a record larger than the
        // running total would otherwise become the baseline the next
        // distinct total diffs against and eat its window.
        // Dated pin change 2026-10-06 (rm-554, upstream #312): the rebound
        // from the rewound 1000 baseline now counts its full growth (2000)
        // instead of the old mark-relative climb (500), so the pinned
        // total moved 8000 → 9500.
        // Dated pin change 2026-10-07 (rm-711, assess SL1): the explicit
        // `compacted` marker announces the counter re-base, so the ledger
        // opens a fresh envelope there (CodexTotals::begin_envelope) and
        // the post-compaction baseline counts once as the re-sent
        // context (1000) instead of being fabrication-clamped to zero
        // against the stale 2500 mark — the pinned total moved 9500 →
        // 10500. The marker is what makes the rewind distinguishable
        // from an unmarked brief rewind, which stays clamped.
        let lines = [
            serde_json::json!({"timestamp":"2026-10-01T09:00:00Z","type":"session_meta","payload":{"cwd":"/tmp/x","model":"gpt-5.3-codex"}}),
            serde_json::json!({"timestamp":"2026-10-01T09:00:05Z","type":"event_msg","payload":{"type":"token_count","info":{"total_token_usage":{"input_tokens":2500}}}}),
            serde_json::json!({"timestamp":"2026-10-01T09:01:00Z","type":"compacted","payload":{"compaction_response_id":"resp_c"}}),
            serde_json::json!({"timestamp":"2026-10-01T09:01:01Z","type":"token_usage_record","payload":{"response_id":"resp_c","usage":{"input_tokens":5000}}}),
            serde_json::json!({"timestamp":"2026-10-01T09:02:00Z","type":"event_msg","payload":{"type":"token_count","info":{"total_token_usage":{"input_tokens":1000}}}}),
            serde_json::json!({"timestamp":"2026-10-01T09:03:00Z","type":"event_msg","payload":{"type":"token_count","info":{"total_token_usage":{"input_tokens":3000}}}}),
        ];
        let raw = lines
            .iter()
            .map(|value| value.to_string())
            .collect::<Vec<_>>()
            .join("\n");
        let session = parse_raw_session("codex", "rollout-compaction-hwm.jsonl", &raw)
            .expect("codex rollout parses");
        // 2500 (first snapshot) + 5000 (the record, standalone) + 1000
        // (post-compaction re-based baseline — the re-sent context,
        // billed once, rm-711) + 2000 (growth from the re-based 1000
        // baseline to 3000 — billed post-compaction re-sends, rm-554).
        // If the record had touched the totals state the final
        // snapshot's window would count 0 (delta from a 5000 baseline).
        assert_eq!(session.metrics.tokens_input, 10_500);
        assert_eq!(
            session
                .metrics
                .line_skips
                .get("codex_compaction_usage_record"),
            Some(&1)
        );
    }

    #[test]
    fn opencode_parse_time_agrees_with_the_single_source_of_timestamp_truth() {
        // rm-502: the lane's private naive copy folded onto lib.rs
        // `parse_ts`. Fractional naive stamps now parse (the old copy
        // dropped the fraction), every ISO string the shared parser
        // accepts yields the same instant on this lane, and the
        // unix-float fallback stays lane-specific.
        let shared = |value: &str| crate::parse_ts(value);
        let lane = |value: Option<&Value>| opencode_parse_time(value);
        for text in [
            "2026-10-05T03:00:00Z",
            "2026-10-05T03:00:00+02:00",
            "2026-10-05T03:00:00",
            "2026-10-05T03:00:00.500",
            "2026-10-05T03:00:00.500000",
        ] {
            assert_eq!(
                lane(Some(&Value::String(text.to_string()))),
                shared(text),
                "opencode lane must agree with parse_ts on {text}"
            );
        }
        // Additive widening pinned: fractional naive stamps used to fall
        // through to the unix-float arm (None for a non-numeric string).
        assert!(shared("2026-10-05T03:00:00.500").is_some());
        // Lane-specific fallback preserved: raw unix floats.
        assert_eq!(
            lane(Some(&Value::Number(serde_json::Number::from(
                1735689600u64
            )))),
            chrono::DateTime::<chrono::Utc>::from_timestamp(1735689600, 0)
        );
        assert_eq!(
            lane(Some(&Value::String("1735689600".to_string()))),
            chrono::DateTime::<chrono::Utc>::from_timestamp(1735689600, 0)
        );
        assert_eq!(lane(None), None);
        assert_eq!(lane(Some(&Value::Bool(true))), None);
    }
}
