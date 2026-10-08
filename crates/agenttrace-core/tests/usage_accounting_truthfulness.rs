//! usage-accounting truthfulness regression harness (run 7f9c6d24 cycle 2;
//! ROADMAP rows rm-601/rm-602/rm-603/rm-554/rm-556, with the campaign's
//! rm-555 folded into the landed copilot per-model rm-551 at integration):
//! cycle 2 (run 7f9c6d24 cycle 2). Ports the usage arms of upstream PR
//! #312 (merged 2026-10-05 into luoyuctl/agenttrace 706bf586) onto this
//! tree's per-lane architecture; every fixture in
//! `tests/fixtures/usage-accounting/` measured RED (over- or
//! under-counted, or truncated duration) on the pre-batch build at
//! 67dfdb5 — each test names its old wrong value so flipping back is the
//! regression gate.
//!
//! Batch rows (ROADMAP.md):
//! - rm-601 claude streaming: one usage row per streamed content block on
//!   the SAME message id — fold per id, max per class (never the sum).
//! - rm-602 qwen aliases: input aliases are cache-INCLUSIVE and can appear
//!   in pairs — first alias per class, net input beside the cache line.
//! - rm-603 codex reasoning: `reasoning_output_tokens` is a BREAKDOWN of
//!   `output_tokens` on the OpenAI wire, never an addend.
//! - rm-554 codex compaction: each DISTINCT cumulative total counts once
//!   (its `last_token_usage` when present), so post-compaction growth
//!   inside the old envelope counts — replacing the rm-162/#286
//!   high-water refusal.
//! - rm-555 (campaign numeral, folded into the landed rm-551) copilot
//!   shutdown: per-model tracking — a partial shutdown no longer suppresses
//!   checkpoint-only models, and a resume's later shutdown record REPLACES
//!   the earlier one per model.
//! - rm-556 copilot timestamp: a metrics-less shutdown record ends the
//!   session; its timestamp reaches the credit event (or a terminal
//!   marker) so duration reflects it.
//!
//! Closeout batch (run 2f02ecafb267 cycle 2, same theme, red-first at
//! 518170a):
//! - rm-685 codex totals: re-emission freshness is OCCURRENCE-ADJACENT —
//!   a post-compaction total that merely happens to be value-equal to an
//!   earlier one still counts, an identical `last_token_usage` under a
//!   new total falls to the totals delta, and the ledger is bounded
//!   (previous event's total + snapshot, plus a capped ring for the
//!   legacy delta path's value-dedup) instead of a per-distinct-total
//!   set (adversarial long journals peaked 6.3x the control's RSS
//!   through the retained set).
//! - rm-686 generic usage: Gemini-shaped `promptTokenCount` is
//!   cache-INCLUSIVE — `cachedContentTokenCount` joins the cache-read
//!   aliases and nets out of input with the shared clamped subtraction
//!   (the qwen arm's rule).
//! - rm-602 extension: the qwen stats/modelUsage arms take the first
//!   PRESENT alias per class (never the sum), like `qwen_usage`.

use agenttrace_core::parse_file;
use std::path::PathBuf;

fn fixture(name: &str) -> agenttrace_core::Session {
    let path: PathBuf = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/usage-accounting")
        .join(name);
    parse_file(&path).unwrap_or_else(|error| panic!("{name}: {error}"))
}

#[test]
fn claude_streaming_folds_per_message_id_at_max() {
    // rm-601: a 3-block stream repeats the message's usage on msg_01 with
    // growing output (50 -> 120 -> 200). Pre-batch the three emissions
    // summed (3000 in / 370 out / 600 cache — 3.0x the truth); the fold
    // keeps one meta event at the max per class.
    let session = fixture("claude-stream-growing.jsonl");
    assert_eq!(session.metrics.source_tool, "claude_code");
    assert_eq!(session.metrics.tokens_input, 1000);
    assert_eq!(session.metrics.tokens_output, 200);
    assert_eq!(session.metrics.tokens_cache_r, 200);
}

#[test]
fn claude_identical_duplicate_usage_stays_deduped() {
    // rm-601 guard: an exact duplicate re-emission on the same id collapses
    // to the same values (the pre-batch exact-string snapshot dedupe
    // behavior is preserved).
    let session = fixture("claude-stream-duplicate.jsonl");
    assert_eq!(session.metrics.tokens_input, 300);
    assert_eq!(session.metrics.tokens_output, 30);
}

#[test]
fn claude_idless_usage_still_counts_every_emission() {
    // rm-601 pinned edge: messages with no id cannot be paired across
    // rows, so every emission keeps counting (upstream #312 keeps the same
    // fallback). Dated pin: legacy behavior retained 2026-10-06.
    let session = fixture("claude-stream-idless.jsonl");
    assert_eq!(session.metrics.tokens_input, 200);
    assert_eq!(session.metrics.tokens_output, 20);
}

#[test]
fn qwen_input_aliases_are_cache_inclusive_and_first_wins() {
    // rm-602: promptTokenCount (1000) already contains the 600-token
    // cached read, so net input is 400 beside the separate cache line.
    // Pre-batch: input 1000 stacked on top of the cache span (1700 total
    // vs the true 1100).
    let session = fixture("qwen-cache-inclusive.jsonl");
    assert_eq!(session.metrics.source_tool, "qwen_code");
    assert_eq!(session.metrics.model_used, "qwen3-coder-plus");
    assert_eq!(session.metrics.tokens_input, 400);
    assert_eq!(session.metrics.tokens_output, 100);
    assert_eq!(session.metrics.tokens_cache_r, 600);
}

#[test]
fn qwen_paired_aliases_count_first_present_never_the_sum() {
    // rm-602: hybrid snapshots carry input_tokens AND prompt_tokens (and
    // both output aliases) describing the same classes. Pre-batch summed
    // every alias (1400 in / 140 out); first-present counts each class
    // once.
    let session = fixture("qwen-alias-pair.jsonl");
    assert_eq!(session.metrics.tokens_input, 700);
    assert_eq!(session.metrics.tokens_output, 70);
}

#[test]
fn codex_reasoning_is_a_breakdown_not_an_addend() {
    // rm-603: on the OpenAI wire output_tokens (400) already includes the
    // 150 reasoning tokens. Pre-batch folded reasoning into output (550).
    // The breakdown rides its own reasoning_tokens line; input is net of
    // the 600-token cached read.
    let session = fixture("codex-reasoning-breakdown.jsonl");
    assert_eq!(session.metrics.source_tool, "codex_cli");
    assert_eq!(session.metrics.tokens_input, 400);
    assert_eq!(session.metrics.tokens_output, 400);
    assert_eq!(session.metrics.tokens_reasoning, 150);
    assert_eq!(session.metrics.tokens_cache_r, 600);
}

#[test]
fn codex_post_compaction_growth_counts_forward() {
    // rm-554: the cumulative total rewinds after compaction (1200 -> 300)
    // and climbs again (500, last 200/80). Every event carries its
    // window's last_token_usage, which is the fresh call's billed usage —
    // the compacted context is re-sent after the reset. Truth is
    // 1200+300+200 in / 520+100+80 out. Pre-batch the rm-162/#286
    // high-water guard refused both post-rewind windows entirely
    // (1200/520 reported).
    let session = fixture("codex-post-compaction-counts.jsonl");
    assert_eq!(session.metrics.tokens_input, 1700);
    assert_eq!(session.metrics.tokens_output, 700);
}

#[test]
fn copilot_partial_shutdown_keeps_checkpoint_only_models() {
    // rm-555 (folded into landed rm-551): the shutdown record carries only gpt-5-codex while the
    // checkpoint saw gpt-5-codex AND gpt-5. Pre-batch a single global
    // "shutdown emitted" bool suppressed the checkpoint wholesale, so
    // gpt-5's 500/100 never surfaced (1000/200 reported instead of
    // 1500/300).
    let session = fixture("copilot-partial-shutdown.jsonl");
    assert_eq!(session.metrics.source_tool, "copilot_cli");
    assert_eq!(session.metrics.tokens_input, 1500);
    assert_eq!(session.metrics.tokens_output, 300);
    // The checkpoint's credit total (5e9 nano AIU) still rides the credit
    // event at $0.01 per 1e9 nano AIU (rm-485, unchanged).
    assert!((session.metrics.credit_usd - 0.05).abs() < 1e-9);
}

#[test]
fn copilot_resumed_shutdown_replaces_instead_of_stacking() {
    // rm-555 (folded into landed rm-551): a resumed session writes a second shutdown record with the
    // cumulative 800/80 snapshot. Pre-batch both shutdowns emitted and
    // stacked (1300/130 reported); the later per-model record now
    // replaces the earlier one.
    let session = fixture("copilot-resumed-shutdown.jsonl");
    assert_eq!(session.metrics.tokens_input, 800);
    assert_eq!(session.metrics.tokens_output, 80);
    assert!((session.metrics.credit_usd - 0.02).abs() < 1e-9);
}

#[test]
fn codex_value_equal_total_after_a_rewind_still_counts() {
    // rm-685 (assess F1): totals 300 -> 1200 -> 300, every event carrying
    // its window's last_token_usage. The rm-554 `seen` set keyed the
    // re-emission dedup on Value equality across the WHOLE journal, so
    // the post-compaction third window — value-equal to the first total
    // but occurrence-fresh (the counter moved 1200 -> 300 to get there)
    // — was silently dropped: 1200/400 reported against the truth
    // 1500/500 (the compacted context is re-sent and billed).
    let session = fixture("codex-seen-collision.jsonl");
    assert_eq!(session.metrics.source_tool, "codex_cli");
    assert_eq!(session.metrics.tokens_input, 1500);
    assert_eq!(session.metrics.tokens_output, 500);
}

#[test]
fn codex_identical_last_under_a_new_total_counts_the_delta() {
    // rm-685 (assess F5): totals {100,50} -> {200,80} with last_token_usage
    // pinned {100,50} on both events. The repeated snapshot was counted
    // twice (200/100 reported); the second window's growth is the totals
    // delta, so the truth is 200/80.
    let session = fixture("codex-same-last.jsonl");
    assert_eq!(session.metrics.tokens_input, 200);
    assert_eq!(session.metrics.tokens_output, 80);
}

#[test]
fn qwen_stats_dual_aliases_count_first_present_never_the_sum() {
    // rm-602 extension (assess F3): the stats arm ADDed both spellings of
    // each class (`input` + `input_tokens`, `output` + `output_tokens`),
    // so a dual-alias snapshot doubled — 200/100 reported; the
    // qwen_usage first-present rule counts each class once: 100/50.
    let session = fixture("qwen-stats-dual-alias.jsonl");
    assert_eq!(session.metrics.source_tool, "qwen_code");
    assert_eq!(session.metrics.tokens_input, 100);
    assert_eq!(session.metrics.tokens_output, 50);
}

#[test]
fn generic_gemini_shaped_usage_nets_inclusive_input_against_the_cache() {
    // rm-686 (assess F4): a claude-family journal whose usage block is
    // Gemini-shaped — promptTokenCount 900 (cache-INCLUSIVE) beside
    // cachedContentTokenCount 800. The generic extractor took the 900 as
    // net input with cache_r 0 (900/0/70 reported — an 800-token
    // over-count at full input rate); cachedContentTokenCount is now a
    // cache-read alias and the inclusive input nets against it with the
    // shared clamped subtraction (the qwen arm's parity target):
    // 100/800/70.
    let session = fixture("claude-gemini-usage.jsonl");
    assert_eq!(session.metrics.source_tool, "claude_code");
    assert_eq!(session.metrics.tokens_input, 100);
    assert_eq!(session.metrics.tokens_cache_r, 800);
    assert_eq!(session.metrics.tokens_output, 70);
}

#[test]
fn copilot_metrics_less_shutdown_timestamp_reaches_the_duration() {
    // rm-556: the shutdown record has no modelMetrics and only
    // totalNanoAiu, but its timestamp (T+20s) is the session's latest
    // activity. Pre-batch the credit event rode the empty default, so
    // duration_sec stayed 1.0 (the last message's timestamp).
    let session = fixture("copilot-shutdown-timestamp.jsonl");
    // (tokens_input 1 is the user message's text estimate — not this pin.)
    assert!((session.metrics.credit_usd - 0.03).abs() < 1e-9);
    assert_eq!(session.metrics.session_end, "2026-01-04T00:00:20Z");
    assert_eq!(session.metrics.duration_sec, 20.0);
}

#[test]
fn copilot_later_checkpoint_still_ends_the_session() {
    // rm-556 no-regression arm: a checkpoint AFTER the shutdown (open
    // session, killed before the next shutdown) is the latest activity —
    // the credit event keeps the checkpoint's T+30s and the checkpoint's
    // per-model usage (rm-485) still surfaces.
    let session = fixture("copilot-checkpoint-after-shutdown.jsonl");
    assert_eq!(session.metrics.tokens_input, 10);
    assert_eq!(session.metrics.tokens_output, 1);
    assert_eq!(session.metrics.session_end, "2026-01-04T00:00:30Z");
    assert_eq!(session.metrics.duration_sec, 30.0);
}
