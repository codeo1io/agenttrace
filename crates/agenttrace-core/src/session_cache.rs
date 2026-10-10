use crate::{Anomaly, Diagnostics, Metrics, Session, ToolWarning};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

pub const SESSION_CACHE_SCHEMA_VERSION: i64 = 52;
// Bumped 51 -> 52 (integration of run 2d92ee95, "session-cache
// store integrity" batch — rm-292 LEAD schema-change restore
// lifecycle + rm-041 lossless cache keys + the rm-298 concurrent-
// save lock rider; conflict case 79d8fd06 (first dispatch 51ee80dc), candidate 0aba09b): the
// batch's own ladder moved 24 -> 26 at its base 1511547 (25/26 were
// consumed by sibling/upstream landings between the base and the
// ceiling merge); this integration re-bases the bump onto the
// already-advanced ceiling 51 per the documented convention, one
// invalidation either way. The rm-041 key semantics ride the rung:
// entries and listing members may now hold losslessly-encoded
// non-UTF-8 keys (see `LOSSLESS_KEY_PREFIX`), so pre-52 entries
// keyed under `to_string_lossy` output are invalidated once. What
// the bump does NOT do anymore is discard silently: the
// schema-invalidations path in `load_session_cache` records the
// invalidation (observed/schema version, dropped entries/dirs,
// kept listings) in a bounded journal instead of torching the
// store invisibly — parsed entries drop and regenerate once on
// next scan, walk-current directory listings survive on their own
// DIR_LISTING_WALK_VERSION lane.
// Bumped 50 -> 51 (run 0a55a397 cycle-1 "accounting truth"
// residual batch, attempt 5712b6f0: codex record-arm reasoning
// breakdown rm-617 + pi alias first-present rm-618 + qwen per-reason
// skip tables + the rm-251 R1 per-line rate-limit observation rider).
// The parser now reports: codex `token_usage_record` output as
// `output_tokens` with `reasoning_output_tokens` disclosed beside it
// (upstream #312 oracle — the old fold-in over-counted output +64%
// across upstream's 1166-file corpus), pi hybrid usage snapshots at
// first-present-per-class instead of the alias sum, qwen skip lines
// through `line_skips`/`disclosure_counters` tables that did not
// exist, and `codex_rate_limits_observed` once per snapshot line
// instead of the triangular running total — plus a terminal wire
// carrier so a usage-less quota snapshot's identity facts still land.
// Same keys, same persisted shape: the values behind them move, so a
// warm v50 cache would keep serving the inflated/hidden pre-batch
// totals on a cache hit (rm-230 convention: parser-semantics changes
// that alter the served report bump the schema so cached sessions
// regenerate). Minted at the fleet census ceiling: origin/master was
// 43, sibling worktrees held 43 (33b7b7b5/e4917d16/e7d85133) and 49/50
// (2a5cdb9e/5eb82325), spool claims maxed at 50 — minted 51, and
// this integration lands it above the landed 46 ceiling (rungs 47-50
// never landed) so every warm entry written before rm-618/rm-619/
// rm-251 corrected the served totals regenerates.
// Entries regenerate once on next scan.
// Bumped 45 -> 46 (integration landing of run 33b7b7b5, conflict
// case 58fbbd6c — the run's rung re-based a THIRD time, and the
// first moved by a sibling claim rather than a landed ceiling: the
// sibling integration of run ec762a618372 (conflict case c7c07e48,
// integration item claimed 2026-10-08T11:55Z, resolving 2026-10-09 —
// hours before this case 58fbbd6c opened at 22:37Z) minted its own
// 45 directly above the SAME landed ceiling 44 for its rm-760
// usage-truth disclosure parser corrections, so two independent
// parser-semantics bumps claimed one rung while both trees were
// uncommitted. The lowest-rung-unique-above-the-ceiling rule plus
// one-invalidation-either-way yields to the OLDER claimant: the
// sibling keeps 45 and this run lands 46. The invalidation
// semantics are unchanged from the 44 -> 45 rung below — the
// sibling's 45 does not carry this run's corrections, so a warm
// v45 entry written by a post-sibling build still serves (a) the
// overstated rollout tokens_output (and zero tokens_reasoning)
// that rm-617 removes, and (b) the empty aider session_start that
// rm-899 resolves to the earlier DST arm. No migration can
// reconstruct either correction from stale entries (rm-230
// convention: parser-semantics changes that alter the served
// report bump the schema so cached sessions regenerate). Entries
// regenerate once on next scan.
// Bumped 44 -> 45 (integration of run 33b7b7b5, cycle-1 "truthful
// accounting, honest surfaces" batch, rm-617 LEAD 'codex rollout
// reasoning breakdown' + rm-899 'aider DST-ambiguous starts'; the
// batch minted 43 directly above its base 9c3c599's ceiling 41, then
// re-based twice at review-fix — 44 at ed90e5ec after origin landed
// 43 under fabd9fb8/rm-831 (2026-10-09 16:55Z), 45 at b5b9c6fc after
// origin landed 44 under fb1addd56503 (merge b917ff7, 2026-10-09
// 18:41:12Z, landing 03dc231) — so this integration lands the run's
// 45 straight onto the already-advanced ceiling 44 (the very rung
// that review-fix b5b9c6fc's moving-wall census watched origin take)
// per the same convention, one invalidation either way: (a) the codex
// rollout record arm stopped folding reasoning_output_tokens into
// billed output — a warm v44 entry keeps serving the overstated
// tokens_output (and zero tokens_reasoning) for every unchanged
// rollout journal, exactly the double-add rm-617 removes; (b)
// aider_time now resolves DST-ambiguous local starts with earliest()
// instead of degrading them to an empty session_start — a warm v44
// entry pins the pre-fix empty start for unchanged aider journals.
// Rendering-only fixes in the same batch (rm-897 markdown formula
// cells, rm-898 bounded statusline reads) do not touch cached
// Metrics and ride no rung. No migration can reconstruct either
// correction from stale entries (rm-230 convention: parser-semantics
// changes that alter the served report bump the schema so cached
// sessions regenerate). Entries regenerate once on next scan.
// Bumped 43 -> 44 (integration of run fb1addd56503 "truth and bounds",
// conflict case e0a11c511c6344f4af8cf63ae9ed485d): the batch's one
// surviving net-new parser change on the merged tree alters reported
// tool-failure attribution for UNCHANGED codex journals —
// custom_tool_call_output events are buffered by (call_id, index) and
// failure attribution is re-applied after the parse loop, so replays
// and resumed rollouts whose output lines precede their call lines
// stamp the agent-reported failure verdict identically to ordered
// pairs (assess PoC pocE vs pocE3: the --max-tool-fail-rate 0 gate
// flipped rc2 -> rc0 on line order alone) — tool_calls_fail changes
// for unchanged files, and warm v43 entries still carry the
// order-dependent verdicts with matching size/mtime fingerprints, so
// nothing regenerates them until the schema moves. The batch's other
// halves are superseded at this merge and do NOT ride the bump: the
// copilot per-model MAX fold is retired onto the landed rm-551
// replace-preserving fold (identical numbers on every pinned shape,
// so nothing changes for unchanged copilot files), and the
// empty/missing-type disclosure already landed as the fleet rm-584's
// codex_missing_type counter (schema 43 ceiling). The batch's own
// 27 -> 28 bump at its base 8991144 re-bases onto the
// already-advanced ceiling per the same convention (28 was the rm-529
// rung; 43 the run-fabd9fb8cf73 rm-831 batch below) — one
// invalidation either way (rm-230: parser-semantics changes bump the
// schema so cached sessions regenerate under corrected accounting).
// Entries regenerate once on next scan.
// Bumped 42 -> 43 (integration of run fabd9fb8cf73, rm-831 'hostile-
// value truthfulness in parser usage folds' + the rm-721 non-finite
// credit rider; review fix 3238a517 over implement d8b72dad, after
// review bbe295b60d7c F1): the batch corrects derived totals for
// UNCHANGED source files — antigravity folds saturate instead of
// wrapping (a journal folding i64::MAX inputTokens twice reported a
// wrapped-to-zero total), the shared usage extractor refuses NEGATIVE
// counts at insert with alias-rescue semantics (a poisoned count could
// neither ride the fold nor shadow a later alias carrying real data),
// and the copilot per-model credit sum drops poisoned +inf totals
// BEFORE the event insert with a named copilot_credit_nonfinite
// disclosure counter — so a warm pre-batch cache keeps serving the
// wrapped-zero totals, serialized-null credits and the absent
// disclosure with matching size/mtime fingerprints and never re-parses
// (proven live on the assess corpus by the review: ag-2gen
// tokens_input 0 warm vs i64::MAX cold). The batch landed against its
// base dc65644 at schema 35 and bumped it to 36 there; this
// integration re-bases the bump onto the already-advanced ceiling (36
// was the run-6aaf51aa rm-502 batch, 37 the run-bbde21568cd4 rm-754
// batch, 38 the run-5417681937ae rm-718/rm-719/rm-716 disclosure-
// plane batch, 39 the run-f7f81aeaf re-emission accounting batch, 40
// the run-4c3ca863 rm-585/rm-730 honest-attribution batch, 41 the
// run-ac14e52c rm-520/rm-521 batch, 42 the run-14954d7a rm-880/rm-406
// journal-truth batch) per the same convention; the stewardship's
// fence on this file (unlanded spool rm-803's serde read-path claim)
// does not extend to the version ladder — the bump is orthogonal to
// rm-803 and does not collide with it. Pinned red-first by
// stale_schema_35_cache_cannot_mask_the_hostile_value_folds (the
// stale_schema_NN family). Entries regenerate once on next scan.
// Bumped 41 -> 42 (integration of run 14954d7a, "journal truth:
// contain hostile input, surface hidden wire", rm-880 + rm-406's
// dated 2026-10-07 arm): Metrics gained two persisted-only fields —
// `wire_metadata` (codex 0.160.1 session identity/lineage/quota wire
// the fork of record read and dropped) and `model_attribution`
// (per-model token/cost attribution for multi-model sessions, Claude
// Code advisor turns) — round-tripped through the Go mirror above so
// a cache hit keeps disclosing them, beside rm-778's parse-time cwd
// cap and its `cwd_truncation_disclosure` counter. Same keys, same
// fingerprints: a warm v41 cache would keep serving sessions whose
// hidden wire and advisor split silently vanished on the cache hit
// (rm-230 convention: persisted-shape changes bump the schema so
// cached sessions regenerate). The batch landed against its base
// aa5544a at schema 32 and bumped it to 33 there; its commit gate
// re-based the bump onto origin's ceiling 39 as 40 (landing 33 would
// have downgraded the cache format); this integration re-bases it
// again onto the already-advanced ceiling (41 was the run-ac14e52c
// rm-520/rm-521 governance-audit batch) per the same convention.
// Entries regenerate once on next scan.
// Bumped 40 -> 41 (integration of run ac14e52c, cycle-1 "governance
// audit truthfulness" batch, rm-520 LEAD + rm-521 rider; conflict case
// c1c77f5e076549f1aadd7f3e4e12c32c): Metrics now persist which token
// classes carry upstream-recorded cost (`upstream_priced_*`), the
// basis the audit's cost recompute needs. Warm v40 entries lack the
// split — a warm cache would hand the audit a recorded-cost session
// with `upstream_cost_usd > 0` and all-zero priced classes,
// resurrecting the false drift note this batch fixes (catalog × all
// tokens + recorded vs the stored catalog × unpriced + recorded).
// Dropping the cache once re-parses journals and repopulates the
// basis; no migration can reconstruct it (rm-230 convention:
// parser-semantics changes that alter the served report bump the
// schema so cached sessions regenerate). The batch landed against
// its base 1511547 at schema 24 and bumped it to 25 there;
// integration re-bases the bump onto the already-advanced ceiling
// (25 was the run-66a7d797 rm-450 batch, 26 the run-32192d92 rm-485
// batch, 27 the run-b1ff12f8 rm-542 batch, 28 the run-66e75e39
// rm-529 batch, 29 the run-99d1c79c rm-600 batch, 30 the
// run-cb38b958 rm-538 batch, 31 the run-254b2417 rm-551 batch, 32
// the run-7f9c6d24 usage-accounting batch, 33 the run-91833f02
// rm-616 batch, 34 the run-73fe8e1e rm-710 batch, 35 the
// run-3ec6cec08fb9 rm-720/rm-721 batch, 36 the run-6aaf51aa rm-502
// batch, 37 the run-bbde21568cd4 rm-754 batch, 38 the
// run-5417681937ae rm-718/rm-719/rm-716 disclosure-plane batch, 39
// the run-f7f81aeaf re-emission accounting batch, 40 the
// run-4c3ca863 rm-585/rm-730 honest-attribution batch) per the same
// convention. Entries regenerate once on next scan.
// Bumped 39 -> 40 (integration of run 4c3ca863, cycle-3 "honest
// attribution" batch, rm-585 'Spend by branch' LEAD + rm-730 rider
// 'codex structural skips are assumption disclosures'; conflict case
// a6045b8303664de49ab42c3b1a5136da): (a) Session/GoSession gain a
// `branch` field carried from the claude-code lane's gitBranch
// envelope — a warm v39 entry has no branch, so every cached session
// would roll up under the by_branch "unknown" bucket until its file
// changed; (b) the codex structural counters (codex_ignorable_line,
// codex_world_state) moved from metrics.line_skips to
// metrics.disclosure_counters, so a warm v39 entry keeps serving an
// otherwise-exact parse with a Dropped-lines row and degraded
// data_health.confidence — the served report changes for UNCHANGED
// files either way (rm-230 convention: parser-semantics changes that
// alter the served report bump the schema so cached sessions
// regenerate). The batch landed against its base e9e8fd9 at schema 32
// and bumped it to 33 there; integration re-bases the bump onto the
// already-advanced ceiling (33 was the run-91833f02 rm-616 batch, 34
// the run-73fe8e1e rm-710 batch, 35 the run-3ec6cec08fb9 rm-720/
// rm-721 batch, 36 the run-6aaf51aa rm-502 batch, 37 the
// run-bbde21568cd4 rm-754 batch, 38 the run-5417681937ae rm-718/
// rm-719/rm-716 disclosure-plane batch, 39 the run-f7f81aeaf
// re-emission accounting batch) per the same convention.
// Entries regenerate once on next scan.
// Bumped 38 -> 39 (integration of run f7f81aeaf57b, cycle-1 "report
// truthfulness & honest surfacing" batch, rm-834 (minted
// campaign-locally as rm-693, rebound at integration) + rm-694 +
// rm-835 + rm-836; conflict case 934fbbc94fa34b658257d8789829bea6):
// Claude streaming re-emissions now count turns, tool calls, events
// and last-seen timestamps per re-emission (the old fold treated a
// usage row as terminal and discarded the rest, so a 7-turn session
// reported 3 turns, 3 tools and a session_end frozen at the first
// usage row) — turns, tool counts and session_end are cached inside
// Session entries, so a warm v38 cache keeps serving the undercounted
// totals under matching fingerprints and never re-parses (the rm-230
// convention: parser-semantics changes bump the schema so cached
// sessions regenerate under corrected accounting). The batch landed
// against its base 518170a7 at schema 32 and bumped it to 33 there;
// integration re-bases the bump onto the already-advanced ceiling
// (33 was the run-91833f02 rm-616 batch, 34 the run-73fe8e1e rm-710
// batch, 35 the run-3ec6cec08fb9 rm-720/rm-721 batch, 36 the
// run-6aaf51aa rm-502 batch, 37 the run-bbde21568cd4 rm-754 batch, 38
// the run-5417681937ae rm-718/rm-719/rm-716 disclosure-plane batch)
// per the same convention. Entries regenerate once on next scan.
// Bumped 37 -> 38 (integration of run 5417681937ae, cycle-3
// "disclosure-plane honesty" batch, rm-718 + rm-719 + rm-716; conflict
// case 3c573958563e417b94eeab5c307279df): kimi usage-alias counters
// moved from `Metrics.line_skips` to `Metrics.disclosure_counters` —
// the non-loss disclosure channel — so a healthy kimi corpus stops
// reporting LOW confidence and a "Dropped lines" row for matching the
// vendor's real wire keys; rm-718 widened the generic lane's `Event`
// intake (lowercase `model_used` alias — also landed independently by
// the rm-616 generic-lane batch — plus `cachedContentTokenCount` /
// `cacheReadInputTokens` normalizing onto `cache_read_input_tokens`),
// and rm-716 added the `codex_rollout_no_usage_rows` absence verdict.
// All three change what a re-parse reports for an UNCHANGED source
// file (confidence, disclosed-facts census, model attribution,
// cache-read totals), so a warm v37 cache keeps serving the pre-batch
// values with matching fingerprints and never re-parses (the rm-230
// convention: parser-semantics changes bump the schema so cached
// sessions regenerate under corrected accounting). The batch landed
// against its base 1c5edd1 at schema 32 and bumped it to 33 there;
// integration re-bases the bump onto the already-advanced ceiling
// (33 was the run-91833f02 rm-616 batch, 34 the run-73fe8e1e rm-710
// batch, 35 the run-3ec6cec08fb9 rm-720/rm-721 batch, 36 the
// run-6aaf51aa rm-502 batch, 37 the run-bbde21568cd4 rm-754 batch)
// per the same convention. Entries regenerate once on next scan.
// Bumped 36 -> 37 (integration of run bbde21568cd4, "honest numbers",
// rm-754; conflict case 615546e27c2b4f2d86e02b511f23ddd1): loop
// costs are now priced from the session's own model rates x the
// loop's token mass (with a disclosed synthetic fallback when a
// counted call carries no usable usage block) instead of the
// pricing-independent constants, and LoopCost gained the serialized
// cost_basis field -- the dollar figures change for UNCHANGED source
// files, so the cache invalidates once (the rm-230 convention). The
// batch landed against its base 1c5edd1 at schema 32 and bumped it
// to 33 there; integration re-bases the bump onto the
// already-advanced ceiling (33 was the run-91833f02 rm-616 batch,
// 34 was the run-73fe8e1e rm-710 batch, 35 was the run-3ec6cec08fb9
// rm-720/rm-721 batch, 36 was the run-6aaf51aa rm-502 batch) per the
// same convention. Entries regenerate once on next scan.
// Bumped 35 -> 36 (integration of run 6aaf51aa, rm-502 'Naive-ISO
// timestamps parse for --overview but vanish from --sessions and
// --diagnostics'): timestamp parsing is unified onto lib.rs
// `parse_ts` (the lenient single source of truth), so diagnostics
// derived from RAW event stamps (tool_latencies, trace-step
// durations) now populate for naive-ISO corpora and the opencode
// lane accepts fractional naive stamps — Diagnostics are cached
// inside Session entries, so warm v35 entries would keep reporting
// the empty analytics the split-brain produced (the rm-230
// convention: parser-semantics changes bump the schema so cached
// sessions regenerate under corrected values). The batch landed
// against its base ea5c41e at schema 22 and bumped it to 23 there;
// integration re-bases the bump onto the already-advanced ceiling
// (33 was the run-91833f02 rm-616 batch, 34 was the run-73fe8e1e
// rm-710 batch, 35 was the run-3ec6cec08fb9 rm-720/rm-721 batch)
// per the same convention. Entries regenerate once on next scan.

// Bumped 34 -> 35 (integration of run 3ec6cec08fb9, rm-720 'Agent-
// lane usage truthfulness' + rm-721 copilot agent-host audit; review
// fix 8a3231c6 over implement b127341f): the batch corrects
// cost/usage for UNCHANGED agent-lane files — antigravity journals
// fold per-generation usage blocks they previously dropped (tokens 0
// / $0 by construction), and copilot agent-host rollups read the
// per-model totalNanoAiu meters (now SUMMED across models, review
// 3e3a2198 F4) they previously ignored — so a warm v34 cache keeps
// serving the pre-batch $0/token totals with matching fingerprints
// and never re-parses (rm-230 convention; the stewardship's original
// 'no schema bump' decision is corrected by review finding F1: the
// cached GoMetrics shape covers credit_usd, so only a schema bump
// regenerates the disclosed numbers). The batch landed against its
// base 1c5edd1 at schema 32 and bumped it to 33 there; integration
// re-bases the bump onto the already-advanced ceiling (33 was the
// run-91833f02 rm-616 generic-lane batch, 34 was the run-73fe8e1e
// rm-710 truthful-usage batch — its parallel 32 -> 33 lane landed
// first via case 2ac5bbe1, so this merge stacks onto its ceiling per
// the one-invalidation rule). Entries regenerate once on next scan.
// Bumped 33 -> 34 (integration of run 73fe8e1e, rm-710 'truthful usage
// accounting across parse -> cache -> report', assess SL1/SL2/NN2 of
// run 6d574820): the batch corrects derived token totals for UNCHANGED
// source files — codex occurrence-aware usage (rm-711: post-compaction
// re-based token_count windows count their fresh `last` snapshots
// instead of colliding with values already counted pre-compaction; the
// dedup ledger is compaction-envelope-scoped and hard-capped) and qwen
// per-turn accumulation (rm-711: every turn's `result` usage counts;
// the session-wide first-wins latch kept only the first result) — so a
// warm v33 cache keeps serving the pre-fix undercounts with matching
// fingerprints and never re-parses. The bump ALSO invalidates the warm
// disclosure amnesia assessed as NN2: pre-rm-526 v32-era entries were
// written before `line_skips` existed on the entry shape, so a warm
// hit replays hidden skips at confidence "high" — one bump covers both
// defect classes at once (rm-230 convention: parser-semantics changes
// bump the schema so cached sessions regenerate under corrected
// accounting). The batch landed against its base 1c5edd1 at schema 32
// and bumped it to 33 there; integration re-bases the bump onto the
// already-advanced ceiling (33 was the run-91833f02 rm-616 generic-
// lane model/usage truth batch) per the same convention. Entries
// regenerate once on next scan.
// Bumped 32 -> 33 (integration of run 91833f02, rm-616 'Generic-lane
// model/usage truth', review fix 0be11bb1 after review f6b98bc3 F1):
// the Event.model_used snake_case alias plus the generic-lane counted
// fold change what UNCHANGED generic-shaped files report — model_used
// (was "default" whenever the journal spelled the wire key
// model_used), token totals, provenance (reported_by_agent instead of
// estimated_from_text), and the new model_or_usage_dropped /
// usage_present_not_counted line_skips — so a warm v32 cache keeps
// serving the pre-fix values verbatim under matching size/mtime
// fingerprints: no re-parse, no self-heal (proven live by the review's
// pre-fix-binary cache-warming PoC). The batch landed against its base
// 8991144 at schema 27 and bumped it to 28 there; integration re-bases
// the bump onto the already-advanced ceiling (28 was rm-529, 29 was
// rm-600, 30 was rm-538, 31 was landed rm-551, 32 was the run-7f9c6d24
// usage-accounting truthfulness batch) per the same convention.
// Entries regenerate once on next scan.
// Bumped 31 -> 32 (integration of run 7f9c6d24, "usage-accounting
// truthfulness", rm-551..rm-556 renumbered rm-601..rm-603 with rm-554/
// rm-556 keeping their numerals and rm-555 folding into landed rm-551):
// the batch corrects derived token totals and durations for UNCHANGED
// source files — claude streaming per-message-id folding, qwen
// alias/cache-inclusive basis, codex reasoning double-add and
// post-compaction forward counting, copilot per-model later-record-wins
// tracking and the shutdown timestamp tail — so a warm v31 cache keeps
// serving the pre-fix totals with matching fingerprints and never
// re-parses (rm-230 convention: parser-semantics changes bump the
// schema so cached sessions regenerate under corrected accounting).
// The batch landed against its base 67dfdb5 at schema 26 and bumped it
// to 27 there; integration re-bases the bump onto the already-advanced
// ceiling (27 was rm-542, 28 was rm-529, 29 was rm-600, 30 was rm-538,
// 31 was landed rm-551) per the same convention. Entries regenerate
// once on next scan.
// Bumped 30 -> 31 (integration of run 254b2417, rm-551 'Copilot usage
// reconciliation is per-model, not per-snapshot'): a checkpoint or
// shutdown now REPLACES the per-model entries it names and PRESERVES
// every other model's last-known values, so a rotation across
// checkpoints and a partial shutdown stop dropping whole models'
// cumulative tokens (the assess PoCs; ccusage #1824's subtract_usage
// reconciliation keeps the same per-entry discipline for credits) —
// reported totals rise for UNCHANGED multi-model files, which a warm
// v30 cache keeps serving with matching fingerprints: no re-parse,
// no self-heal (the rm-230 convention: parser-semantics changes that
// alter reported totals for unchanged files bump the schema so cached
// sessions regenerate under corrected accounting). The batch landed
// against its base 6b03087 at schema 26 and bumped it to 27 there;
// integration re-bases the bump onto the already-advanced ceiling
// (27 was rm-542, 28 was rm-529, 29 was rm-600, 30 was rm-538) per
// the same convention. Entries regenerate once on next scan.
// Bumped 29 -> 30 (integration of run cb38b958, rm-538 'Disclosure-
// channel truthfulness: split assumption-disclosure from parse-
// loss'): the workbuddy input-basis disclosures
// (`workbuddy_input_basis:cache_subtracted`,
// `workbuddy_input_basis:zeroed_suspected_mismatch`) moved from
// metrics.line_skips to metrics.disclosure_counters — same counters,
// different channel — so warm v29 snapshots still serve them under
// "Dropped lines" with degraded data_health.confidence for sessions
// whose parse was otherwise exact. The rm-230 convention applies:
// parser-semantics changes that alter the served report for
// UNCHANGED files bump the schema so cached sessions regenerate
// under the corrected channel. The batch landed against its base
// 67dfdb5 at schema 26 and bumped it to 27 there; integration re-
// bases the bump onto the already-advanced ceiling (27 was rm-542,
// 28 was rm-529, 29 was rm-600) per the same convention. Entries
// regenerate once on next scan.
// Bumped 28 -> 29 (integration of run 99d1c79c, rm-600 'Port the
// workbuddy usage arithmetic set (upstream #311 sum, #316 clamp,
// reasoning-row usage)', minted campaign-locally as rm-542): workbuddy
// usage arithmetic now SUMS across records (upstream #311) and clamps
// the cached count to the source-recorded input (upstream #316 —
// rm-529's clamp generalized to the bool-returning, cache-key-
// parameterized form and adopted for the Copilot modelMetrics/span
// lanes), so cached sessions would silently keep reporting the old
// keep-last, unclamped totals — the rm-230 convention: a parser-
// semantics change that alters reported totals for unchanged files
// must invalidate warm caches. Copilot sessions ride the same shared
// clamp and re-price identically. The batch landed against its base
// 40f0ae8 at schema 25 and bumped it to 26 there; integration re-bases
// the bump onto the already-advanced ceiling (25 was rm-450, 26 was
// rm-485, 27 was rm-542, 28 was rm-529) per the same convention.
// Entries regenerate once on next scan.
// Bumped 27 -> 28 (integration of run 66e75e39, rm-529 'Clamp cache
// counts to cache-inclusive input (port upstream open PR #316)'):
// the shared subtract_cached_input clamps each cache count to what is
// left of the cache-inclusive input (ccusage cached.min(input)), so
// ledgers whose cache_read exceeded their input stop reporting totals
// past what the source itself recorded — warm v27 entries carry the
// inflated totals this batch exists to fix and nothing regenerates
// them until each source file changes again (rm-230 convention:
// parser-semantics changes bump the schema so cached sessions
// regenerate under corrected accounting). The batch landed against
// its base 1511547 at schema 24 and bumped it to 26 there, skipping
// 25 because the landed ceiling had already taken it (e389f1a,
// integration of run 66a7d797, rm-450); integration re-bases the
// bump onto the already-advanced ceiling (25 was rm-450, 26 was
// rm-485, 27 was rm-542) exactly as that landing did. Upstream
// diverged long ago (26 pre-#316, 27 with it) — the fork ladder is
// documented here for the rm-513 merge-hazard rungs.
// Bumped 26 -> 27 (integration of run b1ff12f8, rm-542, minted
// campaign-locally as rm-449): the Codex custom-tools response items are
// now parsed — custom_tool_call / custom_tool_call_output pairs count as
// tool calls and results (with an agent-reported failure status surfacing
// as an error), standalone reasoning items count as reasoning blocks, and
// world_state lines are disclosed as an ignorable counter — so the newest
// real rollouts stop reporting tool_calls_total=0 while carrying 7-20 tool
// pairs. Reported totals change for unchanged files, which warm v26
// entries still carry pre-fix. The batch landed against schema 22 and
// bumped it to 23 there; integration re-bases the bump onto the
// already-advanced ceiling (23 was rm-408, 24 was rm-436/437/438, 25 was
// rm-450, 26 was rm-485) per the rm-230 convention: parser-semantics
// changes bump the schema so cached sessions regenerate under corrected
// accounting. Entries regenerate once on next scan.
// Bumped 25 -> 26 (integration of run 32192d92, rm-485): the copilot
// session-wide credit accounting (totalNanoAiu on shutdown plus the
// freshest usage_checkpoint snapshot) raises reported cost for
// UNCHANGED files, so a warm v25 cache keeps serving the pre-fix
// cost 0.0 / credit null with matching fingerprints — no re-parse,
// no self-heal (proven live on a surgically degraded cache by the
// batch's review 08e99143, F1). The batch landed against schema 22
// and bumped it to 23 there (review fix 955c3cea, pinned by
// stale_schema_version_cache_never_serves_entries); integration
// re-bases the bump onto the already-advanced ceiling (23 was
// rm-408, 24 was rm-436/437/438, 25 was rm-450) per the rm-230
// convention: parser-semantics changes bump the schema so cached
// sessions regenerate under corrected accounting. Entries
// regenerate once on next scan.
// Bumped 24 -> 25 (integration of run 66a7d797, rm-450): the
// workbuddy input-basis disclosure adds metrics.line_skips counters
// that warm v24 snapshots don't carry, so cached sessions would
// silently omit the very disclosure this batch exists to surface
// (rm-230 convention: parser-semantics changes bump the schema so
// cached sessions regenerate under corrected accounting). The batch
// landed against schema 22 and bumped it to 23 there; integration
// re-bases the bump onto the already-advanced ceiling (23 was
// rm-408, 24 was rm-436/437/438) per the same convention. Entries
// regenerate once on next scan.
// Bumped 23 -> 24 (integration of run 6403d975, rm-436/437/438): the
// pi journal accounting batch (disclosure counters, upstream
// recorded-cost passthrough, per-block multi-model pricing) changed
// derived Metrics semantics, so v23 entries carry the pre-fix pi
// totals — the wrong numbers rm-436 exists to fix — with no
// disclosure line, and nothing regenerates them until each source
// file changes again. The batch landed against schema 21 and bumped
// it to 22 there (review fix F1); integration re-bases the bump onto
// the already-advanced ceiling (22 was rm-400/rm-401, 23 was rm-408)
// per the rm-230 convention: parser-semantics changes bump the schema
// so cached sessions regenerate under corrected accounting. v24
// invalidates them once: entries regenerate on the next scan.
// Bumped 22 -> 23 (cycle-1 review fix, run 555a174d, rm-408): GoMetrics
// gained `zero_usage_events` with `#[serde(default)]`, so a warm v22
// entry written before rm-408 still parses as fresh (size+mtime
// fingerprint unchanged, schema version equal) while the new counter
// defaults to 0 — the disclosure then reports clean zeros for exactly
// the historical sessions it exists to flag. Per the rm-230 convention
// the bump invalidates those entries once so they regenerate under the
// rm-408 accounting. Proven red-first by the review 342a1349 warm-cache
// PoC (a stripped v22 cache served a zero-usage session as clean) and
// pinned by `stale_schema_22_cache_cannot_mask_the_disclosure`.
// Bumped 21 -> 22 (integration of run 2c2db6f5, rm-400/rm-401): the
// kimi_cli usage-alias fix and the Codex compaction pairing fix change
// reported totals for unchanged files, so warm v21 entries carry the
// pre-fix under-counts this batch exists to correct (the rm-230
// convention: parser-semantics changes bump the schema so cached
// sessions regenerate under corrected accounting). Entries regenerate
// once on next scan.
// Fork-merge bump (cycle-1 rm-009, v0.9.0 port): upstream #286 moved the
// schema 17 -> 19 (loop_fingerprints diagnostics model); the fork had
// independently evolved 17 with its own fields (byte bound, walk v2).
// The merged schema is neither lineage, so 20 invalidates every legacy
// snapshot family once: fork-17 snapshots lack the new diagnostics
// model; upstream-19 snapshots lack the fork-only cache semantics.
// Bumped 20 -> 21 (run cbe30a9c integration, rm-230/rm-233): the
// flat-transcript parser now preserves the tool_use_id -> tool_call_id
// join (explicit ids verbatim, id-less entries paired positionally per
// tool) and loop detection keys on (tool name, argument identity)
// instead of the name alone, so v20 entries carry stale
// tool_latencies.unmatched counts, trace-step result pairing, and
// loop_cost groups under the old name-only key. Entries regenerate
// once on next scan.
// Bumped 5 → 6 (cycle-4 CU-10): cycle 3's naming change (placeholder
// titles replaced by first-user-message names) shipped while this stayed
// at 5, so v5 snapshots can carry stale names under new semantics. The
// version check regenerates them on next use.
// Bumped 6 → 7 (cycle-1 rm-198): hermes tool outcome semantics changed —
// tool_calls_ok/fail are now derived from the messages table instead of
// fabricating ok == sessions.tool_call_count, so v6 snapshots carry stale
// tool outcome splits and must regenerate once.
// Bumped 7 → 8 (rm-548): snapshots now carry the opencode
// fork-exclusion count, so v7 snapshots would disclose a
// silently-missing count and must regenerate. The same 7 → 8 bump also
// carries the rm-734 poison-gate eviction (run a7110bf0): the sqlite
// snapshot cache predates the read-failure gate — a corrupt or
// unopenable database could bank an EMPTY snapshot ("read succeeded,
// zero sessions"), which every later run then served as truth with no
// way to re-probe the database even after the operator fixed it. The
// poison gate refuses to bank on failure (landed rm-753), and the
// shared bump evicts any empty snapshot a v7 cache already banked for
// a since-repaired database — the fingerprint (size+mtime) would
// otherwise match and keep serving the poisoned entry. Healthy v7
// snapshots re-bank unchanged under both rationales.
// Bumped 8 → 9 at integration (rm-790, run 2023f222 cycle 1; one
// invalidation either way per the rm-230 convention — the candidate
// landed its own 7 → 8 at base 611242d1 while the advanced ceiling had
// already taken 8 via rm-548/rm-734, so the merged wall re-bases to
// 8 → 9): sqlite-backed sessions now carry their source row key
// (`SessionKey`) and opencode children their parent row id
// (`ParentSession`), so the derived-history identity is per-row
// instead of per-(db, second); v8 snapshots would re-fold same-second
// sessions onto one id and mis-report the retired sqlite-lane
// fork-exclusion count (rm-791 supersession — children are retained
// and attributed), so they regenerate once.
// rm-893 rider: the snapshot schema advances 9 -> 10 because
// enrichment-lane row drops (opencode message/part rows and the
// part×message user-text join, hermes role-count and tool-outcome
// groups) now count into the same per-file disclosure as session-row
// drops — a database cached by a pre-rm-893 binary, whose silent
// enrichment drops left the session lane looking clean, would keep
// serving its snapshot forever on an unchanged (size, mtime)
// fingerprint and hide the new drop counts. One regeneration retires
// the stale entry (rm-230 convention).
// rm-607 rider (integration, conflict case fc480c28): the cross-file
// duplicate-dedup keys on `SessionKey`, which has survived the
// snapshot since v9, and runs at union time after each database's
// cache resolves — so the snapshot schema STAYS at 10; the
// candidate's own 7 -> 8 bump is moot on this ceiling.
const SQLITE_SNAPSHOT_SCHEMA_VERSION: i64 = 10;

/// Orphaned temp files (crashed writers) are swept when the cache loads.
/// Live writers finish quickly; one hour is generous enough that a sweep
/// never races an in-flight write (pass-7 P7-5).
const ORPHAN_TEMP_MAX_AGE: std::time::Duration = std::time::Duration::from_secs(60 * 60);

/// Hard entry bound for the session cache (pass-8 F8-3). Beyond it the
/// entries with the oldest source-file fingerprint (mtime) are dropped
/// at save time, so an ever-growing corpus can no longer grow the
/// snapshot without limit.
pub const MAX_SESSION_CACHE_ENTRIES: usize = 20_000;

/// Hard bound on the serialized `sessions.json` size, enforced over
/// EVERY byte the writer emits — entry values, per-path keys, JSON
/// punctuation, the top-level fields, and the `dirs` map (rm-298: the
/// values-only estimate let a real 5,293-entry corpus write 108.9% of
/// this cap while the estimator called it in-bounds). The entry-count
/// bound alone cannot stop unbounded growth: entries carry full tool-arg
/// maps and directory listings, so a corpus of large sessions can grow
/// the cache to hundreds of MB at 20,000 entries. Eviction follows the
/// same policy as the count bound: oldest source-file fingerprint first
/// (pass-9 CU-22).
pub const MAX_SESSION_CACHE_BYTES: usize = 64 * 1024 * 1024;

/// Hard count bound for cached directory listings (rm-298). Before it,
/// only entries had bounds: the `dirs` map grew with directory-tree
/// breadth (2,783 listings / 3.6 MB on the operator corpus) with
/// nothing to stop it. Eviction is oldest directory mtime first, the
/// same policy as the entry bounds.
pub const MAX_SESSION_CACHE_DIRS: usize = 20_000;

/// Hard byte bound for the serialized `dirs` map — one eighth of the
/// overall cache cap (rm-298), so listings can never crowd session
/// entries out of the byte bound: evicting entries alone can always
/// reach the cap because the dirs block already sits inside this
/// budget.
pub const MAX_SESSION_CACHE_DIR_BYTES: usize = MAX_SESSION_CACHE_BYTES / 8;

/// rm-298 (capacity arm): the inclusive floor for a CONFIGURED entry
/// bound. The default [`MAX_SESSION_CACHE_ENTRIES`] is a compile-time
/// constant, but a runtime knob (`session_cache_entries` config key /
/// `AGENTTRACE_SESSION_CACHE_ENTRIES` env) must never zero the cache
/// out from under a running process — a bound of 0 would evict every
/// entry at save time and silently turn the cache into a no-op, so
/// smaller requests clamp up to 1.
pub const MIN_CONFIGURED_SESSION_CACHE_ENTRIES: usize = 1;

/// rm-298 (capacity arm): the inclusive ceiling for a CONFIGURED entry
/// bound. Entries carry full tool-arg maps, so a runaway
/// `AGENTTRACE_SESSION_CACHE_ENTRIES=999999999` would trade the count
/// bound for a memory bound the process never agreed to; larger
/// requests clamp down to this and `--doctor` discloses the effective
/// value that actually governs eviction.
pub const MAX_CONFIGURABLE_SESSION_CACHE_ENTRIES: usize = 1_000_000;

/// Resolve the effective entry bound (rm-298 capacity arm). The
/// config-table value (installed by the CLI from the layered config)
/// outranks the `AGENTTRACE_SESSION_CACHE_ENTRIES` env knob, which
/// outranks the built-in [`MAX_SESSION_CACHE_ENTRIES`] default — the
/// same precedence chain `history::history_path` applies to its knob.
/// Both layers are clamped into
/// [`MIN_CONFIGURED_SESSION_CACHE_ENTRIES`,
/// `MAX_CONFIGURABLE_SESSION_CACHE_ENTRIES`]; a knob that does not
/// parse as a positive integer is ignored, leaving the default in
/// charge (env knobs are best-effort, like every other `AGENTTRACE_*`).
/// A lowered bound evicts on the NEXT save, not at load — the loaded
/// snapshot is read-only until the writer runs.
fn resolve_session_cache_entries(table: Option<usize>, env: Option<&str>) -> usize {
    let requested = table.or_else(|| env.and_then(|raw| raw.trim().parse::<usize>().ok()));
    match requested {
        Some(value) => value.clamp(
            MIN_CONFIGURED_SESSION_CACHE_ENTRIES,
            MAX_CONFIGURABLE_SESSION_CACHE_ENTRIES,
        ),
        None => MAX_SESSION_CACHE_ENTRIES,
    }
}

/// The entry bound `save_session_cache` enforces for THIS process:
/// the default const, or the configured knob (config file > env),
/// clamped into the legal range.
pub fn effective_session_cache_entries() -> usize {
    resolve_session_cache_entries(
        crate::runtime_config::get().session_cache_entries,
        std::env::var("AGENTTRACE_SESSION_CACHE_ENTRIES")
            .ok()
            .as_deref(),
    )
}

/// Which layer supplied the effective entry bound — for `--doctor`'s
/// disclosure (`"config file"` / `"env"` / `"default"`), so
/// `entries<=N` can be checked against where N came from.
pub fn session_cache_entries_source() -> &'static str {
    if crate::runtime_config::get().session_cache_entries.is_some() {
        "config file"
    } else if std::env::var("AGENTTRACE_SESSION_CACHE_ENTRIES")
        .ok()
        .and_then(|raw| raw.trim().parse::<usize>().ok())
        .is_some()
    {
        "env"
    } else {
        "default"
    }
}

/// How many `schema_invalidations` records the journal keeps. The
/// ledger discloses every schema upgrade the store survived (dropped
/// entries, kept listings); it is bounded so a journal that outlives
/// many upgrades cannot grow the disclosure without limit — the byte
/// bound counts it either way.
const MAX_SCHEMA_INVALIDATION_RECORDS: usize = 8;

/// Walk-semantics version for cached directory listings. Bumped when the
/// discovery walk's directory set changes so stale listings are dropped
/// once at load (see `load_session_cache`). v2: symlinked child
/// directories are followed (Codex `#42135`, cycle 7). v3: npm/package
/// manifests are no longer admitted (rm-370, cycle 4) — the cached replay
/// extends `listing.files` verbatim instead of re-running the admission
/// predicates, and a v2 listing still names `package-lock.json`,
/// `models-store.json`, and `*.lock` as files plus `npm` roots as child
/// directories, so a warm journal keeps feeding the walk the manifest
/// noise the blocklist exists to remove. The session-cache schema
/// version cannot cover this one: schema 22 already shipped (rm-400/401)
/// with the pre-blocklist walker, so a schema-22 journal can still carry
/// stale v2 listings. v4 (rm-732, cycle 1): the replay arm re-validates
/// file kind — a v3 journal written before the rm-212 regular-file gate
/// (or between a valid walk and a later fifo creation) still names
/// non-regular entries as files, and admitting one by name wedged every
/// load path on the parser's blocking open; pre-v4 listings are dropped
/// once and re-walked through the gate. v5 (run 2d92ee95, rm-041): listing
/// members and entry keys are losslessly-encoded paths (`at-bytes:`
/// percent-escaping for non-UTF-8 names) — a v4 listing holds
/// `to_string_lossy` member names, and the replay extends `listing.files`
/// verbatim, so a warm v4 journal keeps hiding the very non-UTF-8
/// journals the lossless keys exist to surface (`bad\u{FFFD}name.jsonl`
/// replays as a path that does not exist). Lossy members cannot be told
/// apart from genuine U+FFFD filenames, so the listings drop once at
/// load instead of being filtered. The batch's own 3 -> 4 bump re-bases
/// onto the already-landed v4 lane (rm-732) per the same convention one
/// lane up.
const DIR_LISTING_WALK_VERSION: i64 = 5;

fn dirs_were_empty(doc: &Map<String, Value>) -> bool {
    doc.get("dirs")
        .and_then(Value::as_object)
        .map(|dirs| dirs.is_empty())
        .unwrap_or(true)
}

#[derive(Debug, Clone, Default)]
pub struct SessionCache {
    path: PathBuf,
    entries: BTreeMap<String, CacheEntry>,
    raw_entries: BTreeMap<String, Value>,
    dirs: BTreeMap<String, DirCacheEntry>,
    /// Entry keys this process invalidated (freshness misses, header
    /// decode failures, prunes). The save-side merge under the cache
    /// lock uses the set to keep a concurrent writer's state from
    /// resurrecting keys that were deleted on purpose (rm-298
    /// residual).
    removed: BTreeSet<String>,
    /// Dir-listing keys this process invalidated; same role as
    /// `removed` for the `dirs` map.
    removed_dirs: BTreeSet<String>,
    /// Durable disclosure of schema invalidations observed by this
    /// process or carried forward from the journal — see
    /// `schema_invalidations()`. Persisted as the top-level
    /// `schema_invalidations` member at save time (bounded by
    /// `MAX_SCHEMA_INVALIDATION_RECORDS`).
    schema_invalidations: Vec<Value>,
    dirty: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CacheEntry {
    mod_time: i64,
    size: i64,
    session: GoSession,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CacheEntryHeader {
    mod_time: i64,
    size: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct FileFingerprint {
    mod_time: i64,
    size: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SqliteSnapshot {
    schema_version: i64,
    database: FileFingerprint,
    wal: Option<FileFingerprint>,
    shm: Option<FileFingerprint>,
    sessions: Vec<GoSession>,
    /// Pricing-catalog identity the snapshot was priced under (rm-196).
    /// `None` on snapshots written before the field existed: accepted
    /// once and stamped at the next store; a mismatched `Some` drops
    /// the snapshot so the sessions re-price.
    #[serde(default)]
    pricing_catalog_id: Option<String>,
    /// rm-548: opencode fork copies excluded from `sessions` when the
    /// snapshot was written. Carried so warm snapshots keep disclosing
    /// the exclusion instead of going silent. The schema bump to 8
    /// retires snapshots written before the exclusion existed. After
    /// the rm-791 supersession (schema 9) the SQLITE lane banks 0
    /// here — parent_id rows are retained and attributed children,
    /// not exclusions — while the field stays the shared carrier the
    /// report sums across lanes.
    #[serde(default)]
    fork_excluded: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DirCacheEntry {
    mod_time: i64,
    /// Byte size of the directory itself at store time — the
    /// same-tick freshness guard (rm-041): mtime granularity can hide
    /// a mutation that landed inside the same tick as the stored
    /// listing, while adding or removing a child moves the directory's
    /// own size on the common local filesystems. Absent (listings
    /// stored before the field existed) compares mtime only.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    dir_size: Option<u64>,
    files: Vec<String>,
    dirs: Vec<String>,
    /// rm-548 (independent-review fix): memoized opencode fork-marker
    /// probes for this listing's files — file key -> (fingerprint at
    /// probe time, parentID). Absent (listings written before the field
    /// existed) simply re-probes once and stores; a fingerprint
    /// mismatch (the session doc was rewritten, e.g. it gained a
    /// `parentID`) re-probes too, so the memo can never mask a
    /// newly-forked session. Kept per-directory so it retires with the
    /// listing, and skipped entirely when empty so untouched journals
    /// serialize byte-identically.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    fork_parents: BTreeMap<String, (FileFingerprint, Option<String>)>,
}

#[derive(Debug, Clone)]
pub(crate) struct CachedDirListing {
    pub files: Vec<PathBuf>,
    pub dirs: Vec<PathBuf>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct GoSession {
    #[serde(default, rename = "Name")]
    name: String,
    #[serde(default, rename = "Path")]
    path: String,
    #[serde(default, rename = "CWD")]
    cwd: String,
    /// rm-585: branch the session ran on (claude-code gitBranch,
    /// first cut). Serde-default keeps older caches decodable (empty
    /// branch -> "unknown" bucket); the schema bump above regenerates
    /// them anyway.
    #[serde(default, rename = "Branch")]
    branch: String,
    #[serde(default, rename = "Metrics")]
    metrics: GoMetrics,
    #[serde(default, rename = "Anomalies")]
    anomalies: Vec<GoAnomaly>,
    #[serde(default, rename = "Health")]
    health: i32,
    #[serde(default, rename = "ToolWarnings")]
    tool_warnings: Vec<GoToolWarning>,
    #[serde(default, rename = "Diagnostics")]
    diagnostics: Diagnostics,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct GoMetrics {
    #[serde(default, rename = "EventsTotal")]
    events_total: usize,
    #[serde(default, rename = "UserMessages")]
    user_messages: usize,
    #[serde(default, rename = "AssistantTurns")]
    assistant_turns: usize,
    #[serde(default, rename = "ToolResults")]
    tool_results: usize,
    #[serde(default, rename = "ToolCallsTotal")]
    tool_calls_total: usize,
    #[serde(default, rename = "ToolCallsOK")]
    tool_calls_ok: usize,
    #[serde(default, rename = "ToolCallsFail")]
    tool_calls_fail: usize,
    #[serde(default, rename = "ToolUsage")]
    tool_usage: BTreeMap<String, usize>,
    #[serde(default, rename = "FileUsage")]
    file_usage: BTreeMap<String, usize>,
    #[serde(default, rename = "ToolArgUsage")]
    tool_arg_usage: BTreeMap<String, usize>,
    #[serde(default, rename = "ToolAuthority")]
    tool_authority: BTreeMap<String, usize>,
    #[serde(default, rename = "HighestAuthority")]
    highest_authority: String,
    #[serde(default, rename = "ReasoningBlocks")]
    reasoning_blocks: usize,
    #[serde(default, rename = "ReasoningChars")]
    reasoning_chars: usize,
    #[serde(default, rename = "ReasoningLens")]
    reasoning_lens: Vec<usize>,
    #[serde(default, rename = "ReasoningRedact")]
    reasoning_redact: usize,
    #[serde(default, rename = "TokensInput")]
    tokens_input: i64,
    #[serde(default, rename = "TokensOutput")]
    tokens_output: i64,
    #[serde(default, rename = "TokensReasoning")]
    tokens_reasoning: i64,
    #[serde(default, rename = "TokensCacheW")]
    tokens_cache_w: i64,
    #[serde(default, rename = "TokensCacheR")]
    tokens_cache_r: i64,
    #[serde(default, rename = "GapsSec")]
    gaps_sec: Vec<f64>,
    #[serde(default, rename = "ModelUsed")]
    model_used: String,
    #[serde(default, rename = "SourceTool")]
    source_tool: String,
    #[serde(default, rename = "SessionStart")]
    session_start: String,
    #[serde(default, rename = "SessionEnd")]
    session_end: String,
    #[serde(default, rename = "NaiveUtcStamps")]
    naive_utc_stamps: usize,
    #[serde(default, rename = "DurationSec")]
    duration_sec: f64,
    #[serde(default, rename = "CostEstimated")]
    cost_estimated: f64,
    /// rm-485: session-wide cost-only credit total (USD) for adapters whose
    /// billing truth is a credit counter (Copilot totalNanoAiu). Defaulted
    // so pre-existing cache rows deserialize unchanged.
    #[serde(default, rename = "CreditUsd")]
    credit_usd: f64,
    #[serde(default, rename = "StoredTotalsDelta")]
    stored_totals_delta: i64,
    /// Parse lines lost inside the session source, by reason (pass-7
    /// P7-1). Preserved across cache round-trips; absent on clean parses.
    #[serde(
        default,
        rename = "LineSkips",
        skip_serializing_if = "BTreeMap::is_empty"
    )]
    line_skips: BTreeMap<String, usize>,
    /// rm-408: present-but-zero usage events, preserved across cache
    /// round-trips so the disclosure survives a warm cache.
    #[serde(
        default,
        rename = "ZeroUsageEvents",
        skip_serializing_if = "crate::usize_is_zero"
    )]
    zero_usage_events: usize,
    /// rm-436: USD cost the source journal recorded for usage blocks;
    /// round-trips so a cached session keeps its recorded-cost pricing
    /// instead of reverting to the catalog estimate on cache hit.
    #[serde(default, rename = "UpstreamCostUSD")]
    upstream_cost_usd: f64,
    /// rm-520: token classes already carrying that upstream-recorded
    /// cost (excluded from the catalog formula in Metrics). Round-
    /// tripped so the audit's rm-436-basis recompute survives a cache
    /// hit; v24 caches lacking them are dropped by the schema bump
    /// above rather than silently priced wrong.
    #[serde(default, rename = "UpstreamPricedInput")]
    upstream_priced_input: i64,
    #[serde(default, rename = "UpstreamPricedOutput")]
    upstream_priced_output: i64,
    #[serde(default, rename = "UpstreamPricedCacheW")]
    upstream_priced_cache_w: i64,
    #[serde(default, rename = "UpstreamPricedCacheR")]
    upstream_priced_cache_r: i64,
    /// rm-436/rm-437: parse-time disclosure counters (pi journals),
    /// round-tripped so cache hits keep disclosing.
    #[serde(
        default,
        rename = "DisclosureCounters",
        skip_serializing_if = "BTreeMap::is_empty"
    )]
    disclosure_counters: BTreeMap<String, usize>,
    /// rm-880: journal wire facts the accounting reads but does not
    /// consume (codex 0.160.1 identity/lineage/quota wire),
    /// round-tripped so a cache hit keeps disclosing the hidden wire
    /// instead of silently dropping it — the rm-230 case class that
    /// forced schema v40.
    #[serde(
        default,
        rename = "WireMetadata",
        skip_serializing_if = "BTreeMap::is_empty"
    )]
    wire_metadata: BTreeMap<String, String>,
    /// rm-406 (dated 2026-10-07 arm): per-model attribution for
    /// multi-model sessions (Claude Code advisor turns), round-tripped
    /// so cache hits keep the by_model split.
    #[serde(
        default,
        rename = "ModelAttribution",
        skip_serializing_if = "BTreeMap::is_empty"
    )]
    model_attribution: BTreeMap<String, crate::ModelAttribution>,
    #[serde(default, rename = "Provenance")]
    provenance: crate::MetricProvenance,
    /// rm-790: source row identity for sqlite-backed sessions; empty
    /// for transcript lanes, so the transcript cache stays byte-identical.
    #[serde(
        default,
        rename = "SessionKey",
        skip_serializing_if = "String::is_empty"
    )]
    session_key: String,
    /// rm-791: raw parent row id for sqlite-backed children — SOURCE DATA
    /// that must survive the snapshot round-trip so the post-load
    /// attribution (subagents.rs) can re-derive linkage and rollups.
    #[serde(
        default,
        rename = "ParentSession",
        skip_serializing_if = "String::is_empty"
    )]
    parent_session: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct GoAnomaly {
    #[serde(default, rename = "type")]
    kind: String,
    #[serde(default)]
    severity: String,
    #[serde(default)]
    emoji: String,
    #[serde(default)]
    detail: String,
}

pub fn session_cache_path() -> PathBuf {
    if let Some(dir) = std::env::var_os("AGENTTRACE_SESSION_CACHE_DIR").map(PathBuf::from) {
        if !dir.as_os_str().is_empty() {
            return dir.join("sessions.json");
        }
    }
    user_cache_dir().join("agenttrace").join("sessions.json")
}

/// Every at-rest artifact `--clear-cache` removes (rm-086), built
/// from the same constructors that write the files: the parsed-metrics
/// journal, the Hermes and OpenCode SQLite snapshots, the statusline
/// capture journal, and the LiteLLM pricing catalog. The statusline
/// and pricing files resolve through their own env-aware path
/// constructors, so each artifact is cleared where it actually lives
/// instead of assuming it sits beside the session cache.
/// `privacy_disclosure_lists_every_artifact` (tests) pins PRIVACY.md's
/// disclosure table against these file names, so a new store cannot
/// ship undisclosed.
pub(crate) fn cache_artifact_paths() -> Vec<PathBuf> {
    vec![
        session_cache_path(),
        // Advisory lock sibling of the store (see `CacheFileLock`):
        // registered so `--clear-cache` sweeps it with the rest. It is
        // empty — it carries no session data.
        session_cache_lock_path(),
        sqlite_snapshot_path("hermes"),
        sqlite_snapshot_path("opencode"),
        crate::statusline::statusline_capture_path(),
        crate::pricing::pricing_cache_path(),
    ]
}

/// Advisory-lock sibling for the session cache: `sessions.json.lock`
/// next to the store. It exists only to serialize the save/clear
/// read-modify-write between concurrent agenttrace processes
/// (rm-298 residual) and holds no data.
pub fn session_cache_lock_path() -> PathBuf {
    lock_sibling(&session_cache_path())
}

fn lock_sibling(path: &Path) -> PathBuf {
    match path.file_name() {
        Some(name) => {
            let mut file_name = name.to_os_string();
            file_name.push(".lock");
            path.with_file_name(file_name)
        }
        None => {
            let mut fallback = path.as_os_str().to_os_string();
            fallback.push(".lock");
            PathBuf::from(fallback)
        }
    }
}

/// Bounded advisory lock over a cache rewrite (rm-298 residual).
///
/// Protocol (MSRV-1.80 std-only — `File::try_lock` is 1.89):
/// - acquire = `create_new` (O_EXCL) on the lock sibling, fenced by a
///   `<pid>-<unique>` token written into it;
/// - release = remove the file only if it still carries our token
///   (a peer that stole a stale lock owns the name now — the token
///   check keeps our Drop from deleting theirs);
/// - staleness is content-first, age-second: a lock whose token
///   names a process that no longer exists — or whose content is not
///   a token at all (a stray file parked on the lock name, or a
///   writer that crashed between creating the file and writing its
///   token, past `CACHE_LOCK_EMPTY_GRACE`) — is stolen immediately by
///   atomic rename; a live holder is stolen only after
///   `CACHE_LOCK_STALE_AFTER`, which covers a wedged-but-alive
///   writer. Content-first matters for liveness too: an inert file on
///   the lock name must not wedge every future save behind the full
///   wait budget;
/// - a stolen victim fences its own write (`still_ours` re-reads the
///   token before the rename in `save_session_cache`), so a writer
///   whose lock was stolen mid-flight aborts and re-serializes
///   instead of clobbering the thief's state;
/// - stealing (`STALE_AFTER`) fires long before waiting gives up
///   (`WAIT`): giving up means proceeding unlocked, which reopens the
///   lost-update window the lock exists to close, so it is the last
///   resort, reached only when a live peer holds for `CACHE_LOCK_WAIT`
///   straight.
struct CacheFileLock(PathBuf, String);

const CACHE_LOCK_WAIT: std::time::Duration = std::time::Duration::from_secs(30);
const CACHE_LOCK_RETRY: std::time::Duration = std::time::Duration::from_millis(10);
const CACHE_LOCK_STALE_AFTER: std::time::Duration = std::time::Duration::from_secs(5);
const CACHE_LOCK_EMPTY_GRACE: std::time::Duration = std::time::Duration::from_secs(1);
/// How many fenced re-serialization attempts `save_session_cache`
/// makes when its lock keeps being stolen mid-flight before landing
/// the write anyway (see the fence in `save_session_cache`).
const CACHE_LOCK_ATTEMPTS: usize = 3;

impl CacheFileLock {
    fn acquire(path: &Path) -> Option<Self> {
        let lock_path = lock_sibling(path);
        if let Some(parent) = lock_path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let token = format!(
            "{}-{}",
            std::process::id(),
            unique_temp_path(&lock_path).display()
        );
        let deadline = std::time::Instant::now() + CACHE_LOCK_WAIT;
        loop {
            match fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&lock_path)
            {
                Ok(file) => {
                    drop(file);
                    // Owner-only, like every other cache artifact: the
                    // lock sits in the same private directory.
                    #[cfg(unix)]
                    {
                        use std::os::unix::fs::PermissionsExt;
                        let _ = fs::set_permissions(&lock_path, fs::Permissions::from_mode(0o600));
                    }
                    let _ = fs::write(&lock_path, token.as_bytes());
                    return Some(CacheFileLock(lock_path, token));
                }
                Err(_) => {
                    if lock_stealable(&lock_path) {
                        // Atomic claim: only one racer's rename
                        // succeeds; the loser re-loops. The victim, if
                        // alive, fences at its rename and retries.
                        let claimed = unique_temp_path(&lock_path);
                        if fs::rename(&lock_path, &claimed).is_ok() {
                            let _ = fs::remove_file(&claimed);
                        }
                        continue;
                    }
                    if std::time::Instant::now() >= deadline {
                        // Wait budget spent on a live holder: proceed
                        // unlocked (see the struct comment for why that
                        // is the last-resort degrade).
                        return None;
                    }
                    std::thread::sleep(CACHE_LOCK_RETRY);
                }
            }
        }
    }

    /// Fencing check: true while the lock file on disk still carries
    /// exactly our token. A writer whose lock was stolen while it was
    /// mid-rewrite must not land its rename over the thief's state.
    fn still_ours(&self) -> bool {
        matches!(fs::read_to_string(&self.0), Ok(ref text) if text == &self.1)
    }
}

/// Whether the lock file at `path` may be stolen right now: content
/// first (empty past the grace window, unreadable, or a token naming
/// a dead process), age second (a live-token lock is stolen only past
/// `CACHE_LOCK_STALE_AFTER` — a wedged-but-alive writer; the common
/// crashed-writer case is caught instantly by the dead pid). An empty
/// file inside the grace window is a creator between create and
/// token-write, so it is respected.
fn lock_stealable(lock_path: &Path) -> bool {
    let Ok(metadata) = fs::metadata(lock_path) else {
        return false;
    };
    let age = metadata
        .modified()
        .ok()
        .and_then(|moment| moment.elapsed().ok())
        // An unreadable clock reads as fully aged: the age lane stays
        // usable instead of the lock becoming unstealable.
        .unwrap_or(CACHE_LOCK_STALE_AFTER);
    match fs::read_to_string(lock_path) {
        Ok(text) => lock_decision(&text, age),
        Err(_) => true,
    }
}

/// The staleness decision for a lock whose content read back as
/// `text` at `age` since its last write — the pure core of
/// `lock_stealable`, split out so the age lanes (the empty-file
/// grace window, the wedged-but-alive threshold) can be pinned
/// deterministically instead of through filesystem timestamps a
/// loaded test runner may sample late. Content first: a stray
/// non-token and a token naming a dead process are stealable at any
/// age; age second: an empty file is a creator mid-create only
/// inside the grace window, and a live-token lock is stolen only
/// past `CACHE_LOCK_STALE_AFTER`.
fn lock_decision(text: &str, age: std::time::Duration) -> bool {
    if text.is_empty() {
        return age > CACHE_LOCK_EMPTY_GRACE;
    }
    match lock_token_pid(text) {
        Some(pid) if lock_pid_is_alive(pid) => age > CACHE_LOCK_STALE_AFTER,
        // Not a recognizable token: a stray file parked on the
        // lock name (never ours to respect) or a hand-mangled
        // leftover — steal now rather than wedge every future
        // save behind it.
        _ => true,
    }
}

/// Leading `<pid>-` of a lock token, when the content looks like one.
fn lock_token_pid(text: &str) -> Option<u32> {
    let pid = text.split('-').next()?;
    if !pid.is_empty() && pid.bytes().all(|byte| byte.is_ascii_digit()) {
        pid.parse().ok()
    } else {
        None
    }
}

#[cfg(unix)]
fn lock_pid_is_alive(pid: u32) -> bool {
    // /proc is the std-only liveness probe on the unix family this
    // tool ships on; where it is absent the age lane still catches
    // the crash.
    Path::new("/proc").join(pid.to_string()).exists()
}

#[cfg(not(unix))]
fn lock_pid_is_alive(_pid: u32) -> bool {
    true
}

impl Drop for CacheFileLock {
    fn drop(&mut self) {
        if matches!(fs::read_to_string(&self.0), Ok(ref text) if text == &self.1) {
            let _ = fs::remove_file(&self.0);
        }
        // A missing or foreign-tokened lock is not ours to remove —
        // leave it; the stale-steal path reclaims crashed writers'
        // leftovers.
    }
}

pub fn clear_session_cache() -> anyhow::Result<()> {
    // Serialize with any in-flight cache rewrite before removing the
    // artifacts (rm-298 residual). A clear that races a save can
    // still end with a resurrected store written after the purge —
    // that is inherent to the operation — but the two no longer
    // interleave mid-write.
    let _lock = CacheFileLock::acquire(&session_cache_path());
    let mut paths = cache_artifact_paths();
    paths.extend(legacy_cache_artifact_paths());
    remove_cache_artifacts(&paths)
}

/// Superseded-version leftovers (rm-086 review follow-up): older builds
/// wrote versioned SQLite snapshots (`hermes-sqlite-v7-*.json`,
/// `opencode-sqlite-v7-*.json`); no live code names those files today,
/// so on an upgraded host they would otherwise survive `--clear-cache`
/// forever — the most privacy-sensitive class (full parsed metrics)
/// among the least visible files. Swept by pattern from the same
/// env-aware cache root the registry uses; prefixes are pinned to the
/// two stores that ever wrote them so nothing else in the directory is
/// touched.
fn legacy_cache_artifact_paths() -> Vec<PathBuf> {
    // Both snapshot constructors share one env-aware cache root.
    let snapshot_path = sqlite_snapshot_path("hermes");
    let Some(cache_dir) = snapshot_path.parent() else {
        return Vec::new();
    };
    let Ok(entries) = fs::read_dir(cache_dir) else {
        return Vec::new();
    };
    entries
        .flatten()
        .filter(|entry| {
            let file_name = entry.file_name();
            let Some(file_name) = file_name.to_str() else {
                return false;
            };
            (file_name.starts_with("hermes-sqlite-v") || file_name.starts_with("opencode-sqlite-v"))
                && file_name.ends_with(".json")
        })
        .map(|entry| entry.path())
        .collect()
}

fn remove_cache_artifacts(paths: &[PathBuf]) -> anyhow::Result<()> {
    // rm-086 review follow-up: clear as much as we can, then report —
    // a first failure must not strand the artifacts after it (a partial
    // clear is the worst outcome for a privacy-motivated purge).
    let mut failures: Vec<String> = Vec::new();
    for path in paths {
        match fs::remove_file(path) {
            Ok(()) => {}
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
            Err(err) => failures.push(format!("{}: {}", path.display(), err)),
        }
    }
    if failures.is_empty() {
        Ok(())
    } else {
        anyhow::bail!(
            "failed to remove some cache artifacts: {}",
            failures.join("; ")
        )
    }
}

pub(crate) fn load_sqlite_snapshot(database: &Path, name: &str) -> Option<Vec<Session>> {
    load_sqlite_snapshot_from(database, &sqlite_snapshot_path(name)).map(|(sessions, _)| sessions)
}

/// rm-548: opencode snapshots also carry their fork-exclusion count so
/// the disclosure survives a warm cache.
pub(crate) fn load_sqlite_snapshot_with_meta(
    database: &Path,
    name: &str,
) -> Option<(Vec<Session>, usize)> {
    load_sqlite_snapshot_from(database, &sqlite_snapshot_path(name))
}

fn load_sqlite_snapshot_from(
    database: &Path,
    snapshot_path: &Path,
) -> Option<(Vec<Session>, usize)> {
    let raw = fs::read(snapshot_path).ok()?;
    let snapshot = serde_json::from_slice::<SqliteSnapshot>(&raw).ok()?;
    if snapshot.schema_version != SQLITE_SNAPSHOT_SCHEMA_VERSION
        || snapshot.database != file_fingerprint(database)?
        || snapshot.wal != file_fingerprint(&sqlite_wal_path(database))
        || snapshot.shm != file_fingerprint(&sqlite_shm_path(database))
        || matches!(
            &snapshot.pricing_catalog_id,
            Some(stamped) if stamped != crate::pricing::catalog_identity()
        )
    {
        return None;
    }
    let fork_excluded = snapshot.fork_excluded;
    Some((
        snapshot
            .sessions
            .into_iter()
            .map(|session| session.into_session(&database.to_string_lossy()))
            .collect(),
        fork_excluded,
    ))
}

pub(crate) fn store_sqlite_snapshot(
    database: &Path,
    name: &str,
    sessions: &[Session],
) -> anyhow::Result<()> {
    store_sqlite_snapshot_with_meta(database, name, sessions, 0)
}

/// rm-548: opencode snapshots record their fork-exclusion count so the
/// disclosure survives warm caches.
pub(crate) fn store_sqlite_snapshot_with_meta(
    database: &Path,
    name: &str,
    sessions: &[Session],
    fork_excluded: usize,
) -> anyhow::Result<()> {
    store_sqlite_snapshot_at(
        database,
        &sqlite_snapshot_path(name),
        sessions,
        fork_excluded,
    )
}

fn store_sqlite_snapshot_at(
    database: &Path,
    path: &Path,
    sessions: &[Session],
    fork_excluded: usize,
) -> anyhow::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let snapshot = SqliteSnapshot {
        schema_version: SQLITE_SNAPSHOT_SCHEMA_VERSION,
        database: file_fingerprint(database).ok_or_else(|| anyhow::anyhow!("database missing"))?,
        wal: file_fingerprint(&sqlite_wal_path(database)),
        shm: file_fingerprint(&sqlite_shm_path(database)),
        sessions: sessions.iter().map(GoSession::from_session).collect(),
        pricing_catalog_id: Some(crate::pricing::catalog_identity().to_string()),
        fork_excluded,
    };
    write_private_exclusive(path, &serde_json::to_vec(&snapshot)?)?;
    Ok(())
}

/// Per-writer temp path for atomic cache writes (pass-6 P6-3): the fixed
/// `<name>.json.tmp` sibling made two concurrent agenttrace processes race
/// on the same temp file, failing or tearing the save. The suffix is unique
/// per process and per write within the process; the atomic rename is
/// unchanged. Shared by the pricing-catalog and derived-history writes
/// (pass-7 P7-5).
pub(crate) fn unique_temp_path(path: &Path) -> PathBuf {
    use std::sync::atomic::{AtomicU64, Ordering};
    static SEQ: AtomicU64 = AtomicU64::new(0);
    let seq = SEQ.fetch_add(1, Ordering::Relaxed);
    let mut name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("sessions.json")
        .to_string();
    name.push_str(&format!(".tmp.{}.{}", std::process::id(), seq));
    path.with_file_name(name)
}

/// Owner-only creation for conversation-derived artifacts (cycle-1
/// rm-208): the session cache, the SQLite snapshots, the statusline
/// journal, and derived history all carry session names, tool args,
/// project paths, and transcript-derived metrics, so a group/world-
/// readable copy hands them to every local account. `fs::write`/
/// `File::create` default to 0644 (0664 under a common umask); this
/// helper creates 0o600 on Unix — umask can only tighten it — and
/// keeps the platform default elsewhere. Callers keep their own
/// temp-then-rename atomicity.
///
/// rm-693: creation is EXCLUSIVE (O_EXCL) — every caller stages
/// through a fresh [`unique_temp_path`] sibling, so a pre-existing
/// path at the staging name (a planted symlink) is refused with
/// `AlreadyExists` instead of opened and truncated through; pair with
/// [`write_private_exclusive`] for the sequence-bump retry and rename.
pub(crate) fn write_private(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    #[cfg(unix)]
    {
        use std::io::Write;
        use std::os::unix::fs::OpenOptionsExt;
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(path)?;
        file.write_all(bytes)
    }
    #[cfg(not(unix))]
    {
        fs::write(path, bytes)
    }
}

/// How many consecutive temp names [`write_private_exclusive`] tries
/// before giving up (rm-693). A genuine collision (a stale same-name
/// temp) clears on the first bump; a symlink-poisoned directory fails
/// honestly after this many tries instead of ever truncating through
/// an occupied path.
const EXCLUSIVE_STAGING_ATTEMPTS: u32 = 16;

/// Stage `bytes` into a fresh owner-only temp sibling of `path` and
/// rename it into place — the atomic-write shape the session cache,
/// SQLite snapshots, derived history, statusline journal, and pricing
/// catalog all share, now with rm-693's exclusivity: the temp is
/// created O_EXCL, and an `AlreadyExists` at the predictable
/// `{name}.tmp.{pid}.{seq}` name (a planted symlink, or a stale temp)
/// bumps the sequence instead of truncating through the existing
/// path. The rename still hands the destination a fresh inode
/// atomically, so a crash mid-write can never tear it.
pub(crate) fn write_private_exclusive(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    for _ in 0..EXCLUSIVE_STAGING_ATTEMPTS {
        let temp = unique_temp_path(path);
        match write_private(&temp, bytes) {
            Ok(()) => match fs::rename(&temp, path) {
                Ok(()) => return Ok(()),
                Err(err) => {
                    // rm-250's no-residue invariant: a failed rename
                    // (e.g. the destination is a directory) must not
                    // leave the staged temp behind.
                    let _ = fs::remove_file(&temp);
                    return Err(err);
                }
            },
            Err(err) if err.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(err) => {
                let _ = fs::remove_file(&temp);
                return Err(err);
            }
        }
    }
    Err(std::io::Error::new(
        std::io::ErrorKind::AlreadyExists,
        format!(
            "refusing to stage {}: {} consecutive temp names were already occupied \
             (planted symlinks or stale temps); nothing was written or truncated",
            path.display(),
            EXCLUSIVE_STAGING_ATTEMPTS
        ),
    ))
}

/// Append-mode counterpart of [`write_private`] for the statusline
/// journal (cycle-1 rm-208): creates the journal owner-only the first
/// time it is written; an existing file's permissions are left alone.
///
/// rm-693: the journal is appended to at its final path (append
/// semantics need one inode, so temp-then-rename is wrong here), which
/// means first creation must be O_EXCL and an existing path is opened
/// for append only after inspection — a planted symlink (or any
/// non-regular file) at the journal path is refused loudly, never
/// followed.
pub(crate) fn open_private_append(path: &Path) -> std::io::Result<fs::File> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        match fs::OpenOptions::new()
            .append(true)
            .create_new(true)
            .mode(0o600)
            .open(path)
        {
            Ok(file) => Ok(file),
            Err(err) if err.kind() == std::io::ErrorKind::AlreadyExists => {
                let meta = fs::symlink_metadata(path)?;
                if !meta.file_type().is_file() {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidInput,
                        format!(
                            "refusing to append through {}: the journal path is a symlink \
                             or special file, not a regular journal",
                            path.display()
                        ),
                    ));
                }
                fs::OpenOptions::new().append(true).open(path)
            }
            Err(err) => Err(err),
        }
    }
    #[cfg(not(unix))]
    {
        fs::OpenOptions::new().create(true).append(true).open(path)
    }
}

/// Remove temp files left behind by crashed writers (pass-7 P7-5):
/// `<name>.json.tmp.<pid>.<seq>` siblings in the cache directory older
/// than `max_age`. Returns how many were removed. Live writers finish
/// quickly, so a generous `max_age` keeps the sweep from racing one.
pub(crate) fn sweep_orphaned_temps(path: &Path, max_age: std::time::Duration) -> usize {
    let Some(dir) = path.parent() else {
        return 0;
    };
    let Ok(entries) = fs::read_dir(dir) else {
        return 0;
    };
    let now = std::time::SystemTime::now();
    let mut removed = 0;
    for entry in entries.flatten() {
        let file_name = entry.file_name();
        let Some(name) = file_name.to_str() else {
            continue;
        };
        if !name.contains(".tmp.") {
            continue;
        }
        let Ok(metadata) = entry.metadata() else {
            continue;
        };
        let modified = metadata.modified().unwrap_or(now);
        if now
            .duration_since(modified)
            .unwrap_or(std::time::Duration::ZERO)
            >= max_age
            && fs::remove_file(entry.path()).is_ok()
        {
            removed += 1;
        }
    }
    removed
}

fn sqlite_snapshot_path(name: &str) -> PathBuf {
    session_cache_path().with_file_name(format!("{name}-sqlite.json"))
}

fn sqlite_wal_path(database: &Path) -> PathBuf {
    PathBuf::from(format!("{}-wal", database.to_string_lossy()))
}

fn sqlite_shm_path(database: &Path) -> PathBuf {
    PathBuf::from(format!("{}-shm", database.to_string_lossy()))
}

fn file_fingerprint(path: &Path) -> Option<FileFingerprint> {
    let metadata = fs::metadata(path).ok()?;
    Some(FileFingerprint {
        mod_time: file_mod_time_nanos(&metadata),
        size: metadata.len() as i64,
    })
}

pub fn load_session_cache() -> SessionCache {
    let path = session_cache_path();
    sweep_orphaned_temps(&path, ORPHAN_TEMP_MAX_AGE);
    let Ok(raw) = fs::read_to_string(&path) else {
        return SessionCache {
            path,
            ..SessionCache::default()
        };
    };
    let Ok(Value::Object(doc)) = serde_json::from_str::<Value>(&raw) else {
        return SessionCache {
            path,
            ..SessionCache::default()
        };
    };
    let observed_schema_version = doc.get("schema_version").and_then(Value::as_i64);
    if observed_schema_version != Some(SESSION_CACHE_SCHEMA_VERSION) {
        // Store-integrity lead: a schema bump invalidates parsed
        // entries by design — every bump in the history above changed
        // parse or accounting semantics, so stale numbers must never
        // be served. The bump must not silently torch the whole
        // store, though. Directory listings are walk semantics on
        // their own version lane (`DIR_LISTING_WALK_VERSION`), so
        // listings whose lane is still current survive the bump — an
        // upgrade costs a re-parse, not a full tree re-walk — and the
        // invalidation itself is recorded durably in the journal
        // (`schema_invalidations`) so the discard is disclosed instead
        // of invisible. The next save re-writes the store at the
        // current schema with the survivor set: self-heal forward.
        let listings_current = doc.get("dir_listing_version").and_then(Value::as_i64)
            == Some(DIR_LISTING_WALK_VERSION);
        let mut dirs: BTreeMap<String, DirCacheEntry> = BTreeMap::new();
        if listings_current {
            if let Some(Value::Object(listing_map)) = doc.get("dirs") {
                for (key, raw) in listing_map {
                    if let Ok(entry) = serde_json::from_value::<DirCacheEntry>(raw.clone()) {
                        dirs.insert(key.clone(), entry);
                    }
                }
            }
        }
        let dropped_entries = doc
            .get("entries")
            .and_then(Value::as_object)
            .map(|entries| entries.len())
            .unwrap_or(0);
        let dropped_dirs = doc
            .get("dirs")
            .and_then(Value::as_object)
            .map(|dirs| dirs.len())
            .unwrap_or(0)
            .saturating_sub(dirs.len());
        let mut schema_invalidations = persisted_schema_invalidations(&doc);
        schema_invalidations.push(serde_json::json!({
            "reason": "schema",
            "observed_schema_version": observed_schema_version,
            "schema_version": SESSION_CACHE_SCHEMA_VERSION,
            "dropped_entries": dropped_entries,
            "dropped_dirs": dropped_dirs,
            "kept_dir_listings": dirs.len(),
        }));
        cap_schema_invalidations(&mut schema_invalidations);
        return SessionCache {
            path,
            dirs,
            schema_invalidations,
            dirty: true,
            ..SessionCache::default()
        };
    }
    // Pricing-catalog identity (rm-196): cached per-session costs are
    // only valid for the catalog that priced them — the file
    // fingerprints alone cannot tell a catalog refresh from an
    // untouched session. A stamped id that disagrees with the active
    // catalog (snapshot bump on upgrade, `--update-pricing` refresh,
    // override edit) drops every entry once so they re-price; an
    // unstamped legacy cache is accepted as-is and stamped at the next
    // save, so upgrading alone never forces a full rescan.
    let pricing_catalog_id = crate::pricing::catalog_identity().to_string();
    let pricing_matches = doc
        .get("pricing_catalog_id")
        .and_then(Value::as_str)
        .map(|stamped| stamped == pricing_catalog_id)
        .unwrap_or(true);
    let raw_entries = if pricing_matches {
        doc.get("entries")
            .and_then(Value::as_object)
            .map(|entries| {
                entries
                    .iter()
                    .filter(|(_, value)| decode_cache_entry_header(value).is_some())
                    .map(|(path, value)| (path.clone(), value.clone()))
                    .collect()
            })
            .unwrap_or_default()
    } else {
        BTreeMap::new()
    };
    // Directory listings are versioned by walk semantics: v2 follows
    // symlinked child directories (Codex `#42135`, cycle 7), so listings
    // written by the older walker — which silently omitted them — are
    // dropped once at load instead of hiding files until each parent
    // directory's mtime happens to change.
    let listings_current =
        doc.get("dir_listing_version").and_then(Value::as_i64) == Some(DIR_LISTING_WALK_VERSION);
    let dirs = if listings_current {
        doc.get("dirs")
            .and_then(Value::as_object)
            .map(|dirs| {
                dirs.iter()
                    .filter_map(|(path, value)| {
                        serde_json::from_value::<DirCacheEntry>(value.clone())
                            .ok()
                            .map(|entry| (path.clone(), entry))
                    })
                    .collect()
            })
            .unwrap_or_default()
    } else {
        BTreeMap::new()
    };
    let listing_stale = !listings_current && !dirs_were_empty(&doc);
    let mut schema_invalidations = persisted_schema_invalidations(&doc);
    if !pricing_matches {
        // Review fix R3 (2026-10-06, run 2d92ee95 independent_review
        // 90bc04b9): the pricing arm drops every entry so they re-parse
        // at the new catalog, but unlike the schema arm it appended
        // NOTHING to the disclosure ledger — the invalidation was
        // invisible (acceptance contract on the batch lead, rm-292:
        // "a schema OR pricing-catalog mismatch invalidates entries
        // WITH DISCLOSURE"). Same ledger, same shape, plus the
        // discriminator and the observed id. Records written before the
        // `reason` key existed are schema-arm records by construction.
        let observed_pricing_catalog_id = doc
            .get("pricing_catalog_id")
            .and_then(Value::as_str)
            .unwrap_or("<absent>");
        let dropped_entries = doc
            .get("entries")
            .and_then(Value::as_object)
            .map(|entries| entries.len())
            .unwrap_or(0);
        schema_invalidations.push(serde_json::json!({
            "reason": "pricing_catalog",
            "observed_pricing_catalog_id": observed_pricing_catalog_id,
            "observed_schema_version": doc
                .get("schema_version")
                .and_then(Value::as_i64)
                .unwrap_or(0),
            "schema_version": SESSION_CACHE_SCHEMA_VERSION,
            "dropped_entries": dropped_entries,
            "dropped_dirs": 0,
            "kept_dir_listings": dirs.len(),
        }));
        cap_schema_invalidations(&mut schema_invalidations);
    }
    let mut cache = SessionCache {
        path,
        raw_entries,
        dirs,
        schema_invalidations,
        // A catalog mismatch dirties the cache so the emptied entry map
        // is persisted with the new id at the next save (rm-196).
        dirty: listing_stale || !pricing_matches,
        ..SessionCache::default()
    };
    // Dead-path eviction (pass-8 F8-3): entries whose source file no
    // longer exists are pruned at load time instead of accumulating
    // forever. A pruned entry costs only a re-parse if the file ever
    // reappears, so eviction never loses data.
    prune_dead_entries(&mut cache);
    cache
}

/// Removes cache entries whose source path no longer exists (and dir
/// listings whose directory is gone), marking the cache dirty so the
/// next save persists the smaller snapshot. Returns how many entries
/// were pruned (pass-8 F8-3).
fn prune_dead_entries(cache: &mut SessionCache) -> usize {
    let mut pruned = 0;
    let dead_paths: Vec<String> = cache
        .raw_entries
        .keys()
        .chain(cache.entries.keys())
        // Keys may be losslessly-encoded non-UTF-8 paths (rm-041) —
        // decode before asking the filesystem.
        .filter(|path| !path_from_key(path).exists())
        .cloned()
        .collect();
    for path in dead_paths {
        if cache.entries.remove(&path).is_some() | cache.raw_entries.remove(&path).is_some() {
            // Remember the prune so the save-side merge under the
            // cache lock cannot resurrect a dead key (rm-298
            // residual).
            cache.removed.insert(path);
            pruned += 1;
        }
    }
    let dead_dirs: Vec<String> = cache
        .dirs
        .keys()
        .filter(|dir| !path_from_key(dir).exists())
        .cloned()
        .collect();
    for dir in dead_dirs {
        cache.dirs.remove(&dir);
        cache.removed_dirs.insert(dir);
        pruned += 1;
    }
    if pruned > 0 {
        cache.dirty = true;
    }
    pruned
}

pub fn load_cached_sessions(dir: Option<&Path>) -> Vec<Session> {
    let mut cache = load_session_cache();
    load_cached_sessions_from_cache(dir, &mut cache)
}

pub fn load_cached_sessions_from_cache(
    dir: Option<&Path>,
    cache: &mut SessionCache,
) -> Vec<Session> {
    let paths = cache
        .raw_entries
        .keys()
        .chain(cache.entries.keys())
        // Losslessly-encoded keys (rm-041) decode back to the real
        // path before the directory filter runs.
        .map(|key| path_from_key(key))
        .collect::<BTreeSet<_>>();
    let sessions = paths
        .into_iter()
        .filter(|path| dir.is_none_or(|dir| path.starts_with(dir)))
        .filter_map(|path| cached_session(&path, cache))
        .collect();
    if cache.is_dirty() {
        let _ = save_session_cache(cache);
    }
    sessions
}

impl SessionCache {
    pub fn entry_count(&self) -> usize {
        let mut count = self.raw_entries.len();
        for path in self.entries.keys() {
            if !self.raw_entries.contains_key(path) {
                count += 1;
            }
        }
        count
    }

    pub fn dir_count(&self) -> usize {
        self.dirs.len()
    }

    /// Durable record of the cache invalidations this process observed
    /// (or carried forward from the journal): each record names the
    /// schema that was found, the schema that replaced it, how many
    /// entries were dropped, and how many directory listings survived.
    /// Surfaces like the doctor lane can disclose stale-schema
    /// stranding from here without the loader staying silent.
    pub fn schema_invalidations(&self) -> &[Value] {
        self.schema_invalidations.as_slice()
    }

    pub(crate) fn is_dirty(&self) -> bool {
        self.dirty
    }
}

pub(crate) fn cached_dir_listing(dir: &Path, cache: &mut SessionCache) -> Option<CachedDirListing> {
    let key = cache_key(dir);
    let metadata = fs::metadata(dir).ok()?;
    if !metadata.is_dir() {
        return None;
    }
    let entry = cache.dirs.get(&key)?;
    if entry.mod_time != file_mod_time_nanos(&metadata)
        // Same-tick freshness guard (rm-041): a listing whose directory
        // changed size since it was stored is stale even when the
        // mtime has not moved yet.
        || dir_size_changed(entry, &metadata)
    {
        cache.dirs.remove(&key);
        cache.removed_dirs.insert(key);
        cache.dirty = true;
        return None;
    }
    // Listing members may be losslessly-encoded non-UTF-8 paths
    // (rm-041): decode back to the real paths the walk consumed.
    Some(CachedDirListing {
        files: entry.files.iter().map(|file| path_from_key(file)).collect(),
        dirs: entry
            .dirs
            .iter()
            .map(|child| path_from_key(child))
            .collect(),
    })
}

/// Same-tick freshness (rm-041): mtime granularity can hide a mutation
/// that landed inside the same tick as the stored listing — `create
/// file; read` within one coarse timestamp leaves the cached replay
/// serving the pre-create file set. A stored listing also carries the
/// directory's own byte size; adding or removing a child moves that
/// size on the common local filesystems, catching same-tick mutations
/// the mtime compare cannot. Listings stored before the size was
/// recorded (field absent) compare mtime only, so they behave exactly
/// as before. Residual, documented: a create+delete pair that nets the
/// size back to the stored value within one tick stays invisible until
/// the next mtime change — closing that would cost a `read_dir` per
/// hit, which is the walk the listing exists to skip.
fn dir_size_changed(entry: &DirCacheEntry, metadata: &fs::Metadata) -> bool {
    entry.dir_size.is_some_and(|size| size != metadata.len())
}

pub(crate) fn store_dir_listing(
    dir: &Path,
    files: &[PathBuf],
    dirs: &[PathBuf],
    cache: &mut SessionCache,
) -> anyhow::Result<()> {
    let metadata = fs::metadata(dir)?;
    let key = cache_key(dir);
    // A fresh store wins over any pending deletion of this key, so the
    // save-side merge keeps it (rm-298 residual).
    cache.removed_dirs.remove(&key);
    cache.dirs.insert(
        key,
        DirCacheEntry {
            mod_time: file_mod_time_nanos(&metadata),
            dir_size: Some(metadata.len()),
            files: files.iter().map(|path| cache_key(path)).collect(),
            dirs: dirs.iter().map(|path| cache_key(path)).collect(),
            fork_parents: BTreeMap::new(),
        },
    );
    cache.dirty = true;
    Ok(())
}

/// rm-548 (independent-review fix): memoized opencode fork-marker
/// probe for one storage session info file, served from the parent
/// directory's cached listing. `Some(parent)` = the memo says the doc
/// is (not) a fork; `None` = no usable memo (probe never run, no
/// listing for the parent directory, or the file's fingerprint moved
/// — a rewritten doc re-probes, so the memo can never mask a
/// newly-forked session). Callers probe the file and persist with
/// [`store_fork_parent_probe`].
pub(crate) fn cached_fork_parent(path: &Path, cache: &SessionCache) -> Option<Option<String>> {
    let entry = cache.dirs.get(&cache_key(path.parent()?))?;
    let (fingerprint, parent) = entry.fork_parents.get(&cache_key(path))?;
    let metadata = fs::metadata(path).ok()?;
    if fingerprint.mod_time == file_mod_time_nanos(&metadata)
        && fingerprint.size == metadata.len() as i64
    {
        Some(parent.clone())
    } else {
        None
    }
}

/// rm-548 (independent-review fix): persist one fork-marker probe
/// result onto the parent directory's cached listing (see
/// [`cached_fork_parent`]). When the parent directory has no cached
/// listing the result is simply not persisted — the probe answer is
/// still correct for this run.
pub(crate) fn store_fork_parent_probe(
    path: &Path,
    parent: Option<String>,
    cache: &mut SessionCache,
) {
    let Some(dir) = path.parent() else {
        return;
    };
    let Ok(metadata) = fs::metadata(path) else {
        return;
    };
    let Some(entry) = cache.dirs.get_mut(&cache_key(dir)) else {
        return;
    };
    entry.fork_parents.insert(
        cache_key(path),
        (
            FileFingerprint {
                mod_time: file_mod_time_nanos(&metadata),
                size: metadata.len() as i64,
            },
            parent,
        ),
    );
    cache.dirty = true;
}

pub(crate) fn cached_file_mod_time_if_fresh(
    path: &Path,
    metadata: &fs::Metadata,
    cache: &mut SessionCache,
) -> Option<i64> {
    let key = cache_key(path);
    let header = cached_entry_header(&key, cache)?;
    if header.size == metadata.len() as i64 && header.mod_time == file_mod_time_nanos(metadata) {
        return Some(header.mod_time);
    }
    delete_cached_session_key(&key, cache);
    None
}

fn decode_cache_entry_header(value: &Value) -> Option<CacheEntryHeader> {
    Some(CacheEntryHeader {
        mod_time: value.get("mod_time")?.as_i64()?,
        size: value.get("size")?.as_i64()?,
    })
}

fn cached_entry_header(path: &str, cache: &mut SessionCache) -> Option<CacheEntryHeader> {
    if let Some(entry) = cache.entries.get(path) {
        return Some(CacheEntryHeader {
            mod_time: entry.mod_time,
            size: entry.size,
        });
    }
    cache
        .raw_entries
        .get(path)
        .and_then(decode_cache_entry_header)
}

fn cached_entry(path: &str, cache: &mut SessionCache) -> Option<CacheEntry> {
    if let Some(entry) = cache.entries.get(path) {
        return Some(entry.clone());
    }
    let raw = cache.raw_entries.get(path)?.clone();
    let Ok(entry) = serde_json::from_value::<CacheEntry>(raw) else {
        delete_cached_session_key(path, cache);
        return None;
    };
    cache.entries.insert(path.to_string(), entry.clone());
    Some(entry)
}

fn cached_entry_missing_tool_warnings(path: &str, cache: &SessionCache) -> bool {
    cache
        .raw_entries
        .get(path)
        .and_then(|entry| entry.get("session").or_else(|| entry.get("Session")))
        .and_then(Value::as_object)
        .is_some_and(|session| {
            !session.contains_key("ToolWarnings") && !session.contains_key("tool_warnings")
        })
}

fn cached_entry_missing_tool_arg_usage(path: &str, cache: &SessionCache) -> bool {
    cache
        .raw_entries
        .get(path)
        .and_then(|entry| entry.get("session").or_else(|| entry.get("Session")))
        .and_then(|session| session.get("Metrics").or_else(|| session.get("metrics")))
        .and_then(Value::as_object)
        .is_some_and(|metrics| {
            !metrics.contains_key("ToolArgUsage") && !metrics.contains_key("tool_arg_usage")
        })
}

fn cached_entry_empty_source_tool(path: &str, cache: &SessionCache) -> bool {
    cache
        .raw_entries
        .get(path)
        .and_then(|entry| entry.get("session").or_else(|| entry.get("Session")))
        .and_then(|session| session.get("Metrics").or_else(|| session.get("metrics")))
        .and_then(Value::as_object)
        .is_some_and(|metrics| {
            metrics
                .get("SourceTool")
                .or_else(|| metrics.get("source_tool"))
                .and_then(Value::as_str)
                .is_some_and(str::is_empty)
        })
}

pub(crate) fn delete_cached_session(path: &Path, cache: &mut SessionCache) {
    delete_cached_session_key(&cache_key(path), cache);
}

fn delete_cached_session_key(path: &str, cache: &mut SessionCache) {
    if cache.entries.remove(path).is_some() || cache.raw_entries.remove(path).is_some() {
        // Remember the invalidation so the save-side merge under the
        // cache lock cannot resurrect a key this process dropped
        // (rm-298 residual).
        cache.removed.insert(path.to_string());
        cache.dirty = true;
    }
}

/// Deduplicated cache paths, each sized as the writer will emit it: a
/// path decoded from `raw_entries` also lives in `entries` (see
/// `cached_entry`), so chaining the two maps' keys counts it twice — the
/// byte bound then over-evicts (up to ~2x near the ceiling) and the
/// entry bound wastes drop slots and under-drops (pass-11 A11-2, cycle
/// 7). At save time the decoded copy OVERWRITES the raw copy for the
/// same path, so when both maps hold a path its written-form size is
/// the decoded entry's — exactly what the byte-true bound must count
/// (rm-298).
fn cache_paths_sized_once(cache: &SessionCache) -> Vec<(String, usize)> {
    let mut sized: BTreeMap<String, usize> = BTreeMap::new();
    for (path, value) in &cache.raw_entries {
        let bytes = serde_json::to_string(value)
            .map(|text| text.len())
            .unwrap_or(0);
        sized.insert(path.clone(), bytes);
    }
    for (path, entry) in &cache.entries {
        let bytes = serde_json::to_string(entry)
            .map(|text| text.len())
            .unwrap_or(0);
        sized.insert(path.clone(), bytes);
    }
    sized.into_iter().collect()
}

/// Serialized length of a JSON object key as `save_session_cache`
/// writes it: the surrounding quotes plus any escape sequences
/// (rm-298 — the keys were the invisible ~80 bytes per entry).
fn json_key_len(key: &str) -> usize {
    serde_json::to_string(key)
        .map(|text| text.len())
        .unwrap_or(0)
}

/// Serialized length of a JSON object from its members' total bytes
/// (`key` + `:` + value each) and the member count: two braces plus a
/// comma between adjacent members (rm-298).
fn json_object_len(member_bytes: usize, count: usize) -> usize {
    if count == 0 {
        2
    } else {
        2 + member_bytes + (count - 1)
    }
}

/// Total member bytes of the `dirs` map as written: per listing, the
/// path key, one colon, and the serialized value (rm-298).
fn dirs_member_bytes(cache: &SessionCache) -> usize {
    cache
        .dirs
        .iter()
        .map(|(path, entry)| {
            json_key_len(path)
                + 1
                + serde_json::to_string(entry)
                    .map(|text| text.len())
                    .unwrap_or(0)
        })
        .sum()
}

/// Serialized length of the `schema_invalidations` array as
/// `save_session_cache` writes it (brackets, commas, and records
/// included). Zero records are omitted from the document entirely, so
/// the projection — and a fresh store's bytes — stay byte-identical
/// to the pre-ledger layout.
fn invalidations_member_bytes(records: &[Value]) -> usize {
    if records.is_empty() {
        return 0;
    }
    serde_json::to_string(records)
        .map(|text| text.len())
        .unwrap_or(0)
}

/// Length of the document `save_session_cache` writes for these
/// blocks: the fixed top-level fields, the entries map, the `dirs`
/// member (when non-empty), the `schema_invalidations` ledger (when
/// non-empty), and the closing brace (rm-298).
fn doc_frame_len(
    entries_member_bytes: usize,
    entries_count: usize,
    dirs_bytes: usize,
    dirs_count: usize,
    invalidations_bytes: usize,
) -> usize {
    let mut total = format!(
        "{{\"schema_version\":{},\"dir_listing_version\":{},\"pricing_catalog_id\":",
        SESSION_CACHE_SCHEMA_VERSION, DIR_LISTING_WALK_VERSION
    )
    .len();
    // The catalog-identity stamp rm-196 writes into every cache file
    // is part of the frame, so the byte-true projection must count it:
    // `\"<identity>\"` serialized as JSON (the digest is hex, but stay
    // defensive and round-trip it through serde like every other
    // member).
    total += serde_json::to_string(crate::pricing::catalog_identity())
        .map(|stamped| stamped.len())
        .unwrap_or(0);
    total += 1 + "\"entries\":".len();
    total += json_object_len(entries_member_bytes, entries_count);
    if dirs_count > 0 {
        total += 1 + "\"dirs\":".len() + json_object_len(dirs_bytes, dirs_count);
    }
    if invalidations_bytes > 0 {
        total += 1 + "\"schema_invalidations\":".len() + invalidations_bytes;
    }
    total + 1
}

/// Byte-true projection of the cache file: exactly the number of
/// bytes `save_session_cache` writes for this cache state — entry
/// values, per-path keys, JSON punctuation, top-level fields, and the
/// `dirs` map. The byte bound enforces the cap over THIS number, so
/// the written file cannot exceed the bound through uncounted
/// overhead (rm-298).
fn serialized_doc_size(cache: &SessionCache) -> usize {
    let sized = cache_paths_sized_once(cache);
    let entries_member_bytes: usize = sized
        .iter()
        .map(|(path, bytes)| json_key_len(path) + 1 + bytes)
        .sum();
    doc_frame_len(
        entries_member_bytes,
        sized.len(),
        dirs_member_bytes(cache),
        cache.dirs.len(),
        invalidations_member_bytes(&cache.schema_invalidations),
    )
}

/// Enforces the entry bound by dropping the entries with the oldest
/// source-file fingerprint (mtime) first; keeps at most `max` entries.
/// Returns how many entries were dropped (pass-8 F8-3). The bound walks
/// the deduplicated path union so a path present in both maps costs one
/// drop slot, not two (pass-11 A11-2); an entry without a decodable
/// header is undatable, not unevictable — it counts as the oldest age so
/// the bound is total, not best-effort (F5-5, cycle-5 review).
fn enforce_entry_bound(cache: &mut SessionCache, max: usize) -> usize {
    let total = cache.entry_count();
    if total <= max {
        return 0;
    }
    let mut by_age: Vec<(i64, String)> = cache_paths_sized_once(cache)
        .into_iter()
        .map(|(path, _bytes)| {
            (
                cached_entry_header(&path, cache).map_or(i64::MIN, |header| header.mod_time),
                path,
            )
        })
        .collect();
    by_age.sort_unstable();
    let drop = total - max;
    let mut dropped = 0;
    for (_, path) in by_age.into_iter().take(drop) {
        // `|`, not `||`: a path can live in both maps and both copies
        // must go in the same drop.
        if cache.entries.remove(&path).is_some() | cache.raw_entries.remove(&path).is_some() {
            dropped += 1;
        }
    }
    if dropped > 0 {
        cache.dirty = true;
    }
    dropped
}

/// Enforces the dirs-map bounds (rm-298): listings drop oldest
/// directory mtime first — the same eviction policy as the entry
/// bounds — until the count is at most `max_count` and the serialized
/// `{...}` block is at most `max_bytes`. Before this, `dirs` was
/// bounded by nothing at all, so a broad directory tree could grow
/// the snapshot without limit and then crowd entries out of the byte
/// bound. Returns how many listings were dropped.
fn enforce_dirs_bound(cache: &mut SessionCache, max_count: usize, max_bytes: usize) -> usize {
    let mut by_age: Vec<(i64, String, usize)> = cache
        .dirs
        .iter()
        .map(|(path, entry)| {
            (
                entry.mod_time,
                path.clone(),
                json_key_len(path)
                    + 1
                    + serde_json::to_string(entry)
                        .map(|text| text.len())
                        .unwrap_or(0),
            )
        })
        .collect();
    // (mod_time, path): deterministic even on mtime ties.
    by_age.sort();
    let mut member_bytes: usize = by_age.iter().map(|(_, _, member)| *member).sum();
    let mut remaining = by_age.len();
    // Oldest first; drop listings until both budgets hold.
    let mut dropped = 0;
    for (_, path, member) in by_age {
        if remaining <= max_count && json_object_len(member_bytes, remaining) <= max_bytes {
            break;
        }
        if cache.dirs.remove(&path).is_some() {
            dropped += 1;
            member_bytes -= member;
            remaining -= 1;
        }
    }
    if dropped > 0 {
        cache.dirty = true;
    }
    dropped
}

/// Enforces the serialized-size bound over the byte-true projection of
/// the WRITTEN document (rm-298): entry values, per-path keys, JSON
/// punctuation, top-level fields, and the dirs map — exactly what
/// `save_session_cache` emits. Entries drop oldest source-file
/// fingerprint (mtime) first, the same eviction order as
/// `enforce_entry_bound`; headerless entries count as the oldest age
/// (F5-5); each path of the deduplicated union counts once (pass-11
/// A11-2). The dirs block is bounded separately by `enforce_dirs_bound`,
/// so evicting entries alone always reaches the cap. Returns how many
/// entries were dropped (pass-9 CU-22; byte-true since rm-298).
fn enforce_byte_bound(cache: &mut SessionCache, max: usize) -> usize {
    // Fast path (the common case): one projection over the whole cache,
    // no member bookkeeping. This is also the production consumer of
    // `serialized_doc_size` — the same projection the tests pin to the
    // written file byte for byte.
    if serialized_doc_size(cache) <= max {
        return 0;
    }
    let mut by_age: Vec<(i64, String, usize)> = cache_paths_sized_once(cache)
        .into_iter()
        .map(|(path, bytes)| {
            let member = json_key_len(&path) + 1 + bytes;
            (
                cached_entry_header(&path, cache).map_or(i64::MIN, |header| header.mod_time),
                path,
                // `key`:value as one member; the comma between members
                // is accounted for in the decrement below.
                member,
            )
        })
        .collect();
    let entries_member_bytes: usize = by_age.iter().map(|(_, _, member)| *member).sum();
    let mut entries_count = by_age.len();
    let dirs_bytes = dirs_member_bytes(cache);
    let mut total = doc_frame_len(
        entries_member_bytes,
        entries_count,
        dirs_bytes,
        cache.dirs.len(),
        invalidations_member_bytes(&cache.schema_invalidations),
    );
    if total <= max {
        return 0;
    }
    // Oldest first; drop entries until the written document fits.
    by_age.sort_by_key(|(mod_time, _, _)| *mod_time);
    let mut dropped = 0;
    for (_, path, member) in by_age {
        if total <= max {
            break;
        }
        if cache.entries.remove(&path).is_some() | cache.raw_entries.remove(&path).is_some() {
            dropped += 1;
            // Dropping one member frees its bytes plus one comma —
            // unless it was the only member, when the map collapses
            // to `{}` and only the member bytes go.
            total -= member + usize::from(entries_count >= 2);
            entries_count -= 1;
        }
    }
    if dropped > 0 {
        cache.dirty = true;
    }
    dropped
}

/// Reads the persisted `schema_invalidations` ledger out of a cache
/// document, bounded to the last `MAX_SCHEMA_INVALIDATION_RECORDS`
/// records (oldest dropped first).
fn persisted_schema_invalidations(doc: &Map<String, Value>) -> Vec<Value> {
    let mut records = doc
        .get("schema_invalidations")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    cap_schema_invalidations(&mut records);
    records
}

fn cap_schema_invalidations(records: &mut Vec<Value>) {
    let len = records.len();
    if len > MAX_SCHEMA_INVALIDATION_RECORDS {
        records.drain(0..len - MAX_SCHEMA_INVALIDATION_RECORDS);
    }
}

/// Merges the on-disk store into the in-memory one under the cache
/// lock (rm-298 residual): keys another process saved after this
/// process loaded are unioned in instead of clobbered by this save.
/// Keys deleted locally (`removed` / `removed_dirs`) stay deleted,
/// keys stored locally win over their on-disk copy, and entries
/// priced under a different catalog are left for the catalog gate to
/// re-price rather than served as if they were ours. A store the
/// merge cannot trust (foreign schema, foreign walk version,
/// unparsable bytes) is skipped, which degrades to the pre-merge
/// last-writer-wins behavior instead of guessing.
fn merge_concurrent_disk_state(cache: &mut SessionCache) {
    let Ok(text) = fs::read_to_string(&cache.path) else {
        return;
    };
    let Ok(Value::Object(doc)) = serde_json::from_str::<Value>(&text) else {
        return;
    };
    if doc.get("schema_version").and_then(Value::as_i64) != Some(SESSION_CACHE_SCHEMA_VERSION) {
        return;
    }
    if doc.get("dir_listing_version").and_then(Value::as_i64) != Some(DIR_LISTING_WALK_VERSION) {
        return;
    }
    let pricing_matches = doc
        .get("pricing_catalog_id")
        .and_then(Value::as_str)
        .map(|stamped| stamped == crate::pricing::catalog_identity())
        .unwrap_or(false);
    if pricing_matches {
        if let Some(Value::Object(entries)) = doc.get("entries") {
            for (key, raw) in entries {
                if cache.entries.contains_key(key)
                    || cache.raw_entries.contains_key(key)
                    || cache.removed.contains(key)
                {
                    continue;
                }
                match serde_json::from_value::<CacheEntry>(raw.clone()) {
                    Ok(entry) => {
                        cache.entries.insert(key.clone(), entry);
                    }
                    Err(_) => {
                        cache.raw_entries.insert(key.clone(), raw.clone());
                    }
                }
            }
        }
    }
    if let Some(Value::Object(dirs)) = doc.get("dirs") {
        for (key, raw) in dirs {
            if cache.dirs.contains_key(key) || cache.removed_dirs.contains(key) {
                continue;
            }
            if let Ok(entry) = serde_json::from_value::<DirCacheEntry>(raw.clone()) {
                cache.dirs.insert(key.clone(), entry);
            }
        }
    }
    // Carry the disk's disclosure records alongside ours.
    if let Some(records) = doc.get("schema_invalidations").and_then(Value::as_array) {
        for record in records {
            if !cache.schema_invalidations.contains(record) {
                cache.schema_invalidations.push(record.clone());
            }
        }
        cap_schema_invalidations(&mut cache.schema_invalidations);
    }
}

pub fn save_session_cache(cache: &mut SessionCache) -> anyhow::Result<()> {
    // A default-constructed cache is the in-memory sentinel (library
    // embeds and tests pass `SessionCache::default()` so no host file
    // is touched): with no backing file, a write-through save is a
    // deliberate no-op. Without this guard the tmp sibling lands next
    // to the empty path — i.e. the process cwd — and the rename onto
    // "" can never succeed, littering sessions.json.tmp.<pid>.<n>
    // behind every load (independent review, conflict case
    // 0fc6c845; rm-545's corpus test hit it three times per run).
    // Real caches always carry an absolute path from
    // `load_session_cache`/`session_cache_path()` and are unaffected.
    if cache.path.as_os_str().is_empty() {
        return Ok(());
    }
    if let Some(parent) = cache.path.parent() {
        fs::create_dir_all(parent)?;
    }
    // rm-298 residual: save used to be a lockless read-modify-write —
    // two agenttrace processes that loaded the store before either
    // saved each clobbered the other's entries wholesale. The rewrite
    // now serializes on the advisory lock sibling, merges the freshly
    // re-read on-disk state under it, and FENCES the rename against
    // lock theft (`still_ours`): an attempt whose lock was stolen
    // while it built the document is discarded and re-serialized
    // around the thief's state, so concurrent writers compose instead
    // of racing (see `merge_concurrent_disk_state` and
    // `CacheFileLock`).
    for attempt in 0..CACHE_LOCK_ATTEMPTS {
        let lock = CacheFileLock::acquire(&cache.path);
        merge_concurrent_disk_state(cache);
        // Hard bounds after the merge, before serializing: beyond the
        // EFFECTIVE entry bound (the default const, or the configured
        // `session_cache_entries`/`AGENTTRACE_SESSION_CACHE_ENTRIES`
        // knob, rm-298 capacity arm) the oldest-fingerprint entries are
        // dropped (pass-8 F8-3); the dirs map keeps its own count and byte
        // budgets (rm-298); and the serialized document is capped at
        // MAX_SESSION_CACHE_BYTES over every byte this function writes —
        // keys, punctuation, top-level fields, invalidation records, and
        // dirs included (pass-9 CU-22; byte-true rm-298). The bounds run
        // over the merged state so they hold for the document actually
        // written.
        enforce_entry_bound(cache, effective_session_cache_entries());
        enforce_dirs_bound(cache, MAX_SESSION_CACHE_DIRS, MAX_SESSION_CACHE_DIR_BYTES);
        enforce_byte_bound(cache, MAX_SESSION_CACHE_BYTES);
        let mut doc = Map::new();
        doc.insert(
            "schema_version".to_string(),
            Value::Number(SESSION_CACHE_SCHEMA_VERSION.into()),
        );
        doc.insert(
            "dir_listing_version".to_string(),
            Value::Number(DIR_LISTING_WALK_VERSION.into()),
        );
        doc.insert(
            "pricing_catalog_id".to_string(),
            Value::String(crate::pricing::catalog_identity().to_string()),
        );
        let mut entries = Map::new();
        for (path, value) in &cache.raw_entries {
            entries.insert(path.clone(), value.clone());
        }
        for (path, entry) in &cache.entries {
            entries.insert(
                path.clone(),
                serde_json::to_value(entry).expect("cache entry serialize"),
            );
        }
        doc.insert("entries".to_string(), Value::Object(entries));
        if !cache.dirs.is_empty() {
            let dirs = cache
                .dirs
                .iter()
                .map(|(path, entry)| {
                    (
                        path.clone(),
                        serde_json::to_value(entry).expect("dir cache entry serialize"),
                    )
                })
                .collect();
            doc.insert("dirs".to_string(), Value::Object(dirs));
        }
        // The invalidation ledger (schema upgrades this store survived) —
        // durable disclosure, bounded to the last
        // MAX_SCHEMA_INVALIDATION_RECORDS records and omitted entirely
        // when empty so a fresh store writes the pre-ledger layout.
        if !cache.schema_invalidations.is_empty() {
            doc.insert(
                "schema_invalidations".to_string(),
                Value::Array(cache.schema_invalidations.clone()),
            );
        }
        let tmp = unique_temp_path(&cache.path);
        write_private(&tmp, &serde_json::to_vec(&Value::Object(doc))?)?;
        // Fence the rename: if our lock was stolen while this document
        // was being built, a peer owns the rewrite — discard the
        // attempt and re-serialize around their on-disk state instead
        // of clobbering it. An unlocked attempt (acquire gave up) or a
        // lock still carrying our token proceeds.
        let fenced_out = matches!(&lock, Some(lock) if !lock.still_ours());
        if fenced_out {
            let _ = fs::remove_file(&tmp);
            if attempt + 1 < CACHE_LOCK_ATTEMPTS {
                continue;
            }
            // Last attempt fenced out too: fall through and land the
            // write — the merge above already took the thief's state
            // into account, and refusing to persist at all would drop
            // this process's own entries permanently.
        }
        fs::rename(tmp, &cache.path)?;
        return Ok(());
    }
    Ok(())
}

pub fn cached_session(path: &Path, cache: &mut SessionCache) -> Option<Session> {
    let key = cache_key(path);
    let header = cached_entry_header(&key, cache)?;
    if !is_fresh(path, &header) {
        delete_cached_session_key(&key, cache);
        return None;
    }
    if cached_entry_missing_tool_warnings(&key, cache) {
        delete_cached_session_key(&key, cache);
        return None;
    }
    if cached_entry_missing_tool_arg_usage(&key, cache) {
        delete_cached_session_key(&key, cache);
        return None;
    }
    if cached_entry_empty_source_tool(&key, cache) {
        delete_cached_session_key(&key, cache);
        return None;
    }
    let entry = cached_entry(&key, cache)?;
    // The fallback path is display-only (Session.path is a String),
    // but it must name the real file: decode a losslessly-encoded key
    // (rm-041) instead of exposing the encoded form.
    let fallback_path = path_from_key(&key).to_string_lossy().to_string();
    Some(entry.session.clone().into_session(&fallback_path))
}

pub fn store_session(
    path: &Path,
    session: &Session,
    cache: &mut SessionCache,
) -> anyhow::Result<()> {
    let metadata = fs::metadata(path)?;
    let key = cache_key(path);
    cache.entries.insert(
        key.clone(),
        CacheEntry {
            mod_time: file_mod_time_nanos(&metadata),
            size: metadata.len() as i64,
            session: GoSession::from_session(session),
        },
    );
    cache.raw_entries.remove(&key);
    // A fresh store wins over any pending deletion of this key, so the
    // save-side merge keeps it (rm-298 residual).
    cache.removed.remove(&key);
    cache.dirty = true;
    Ok(())
}

impl GoSession {
    fn from_session(session: &Session) -> Self {
        Self {
            name: session.name.clone(),
            path: session.path.clone(),
            cwd: session.cwd.clone(),
            branch: session.branch.clone(),
            metrics: GoMetrics::from_metrics(&session.metrics),
            anomalies: session
                .anomalies
                .iter()
                .map(GoAnomaly::from_anomaly)
                .collect(),
            health: session.health,
            tool_warnings: session
                .tool_warnings
                .iter()
                .map(GoToolWarning::from_tool_warning)
                .collect(),
            diagnostics: session.diagnostics.clone(),
        }
    }

    fn into_session(self, fallback_path: &str) -> Session {
        Session {
            name: self.name,
            path: if self.path.is_empty() {
                fallback_path.to_string()
            } else {
                self.path
            },
            cwd: self.cwd,
            branch: self.branch,
            metrics: self.metrics.into_metrics(),
            anomalies: self
                .anomalies
                .into_iter()
                .map(GoAnomaly::into_anomaly)
                .collect(),
            health: self.health,
            tool_warnings: self
                .tool_warnings
                .into_iter()
                .map(GoToolWarning::into_tool_warning)
                .collect(),
            diagnostics: self.diagnostics,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct GoToolWarning {
    #[serde(default, rename = "ToolName")]
    tool_name: String,
    #[serde(default, rename = "Pattern")]
    pattern: String,
    #[serde(default, rename = "Count")]
    count: usize,
    #[serde(default, rename = "Detail")]
    detail: String,
    #[serde(default, rename = "Severity")]
    severity: String,
}

impl GoToolWarning {
    fn from_tool_warning(warning: &ToolWarning) -> Self {
        Self {
            tool_name: warning.tool_name.clone(),
            pattern: warning.pattern.clone(),
            count: warning.count,
            detail: warning.detail.clone(),
            severity: warning.severity.clone(),
        }
    }

    fn into_tool_warning(self) -> ToolWarning {
        ToolWarning {
            tool_name: self.tool_name,
            pattern: self.pattern,
            count: self.count,
            detail: self.detail,
            severity: self.severity,
        }
    }
}

impl GoMetrics {
    fn from_metrics(metrics: &Metrics) -> Self {
        Self {
            events_total: metrics.events_total,
            user_messages: metrics.user_messages,
            assistant_turns: metrics.assistant_turns,
            tool_results: metrics.tool_results,
            tool_calls_total: metrics.tool_calls_total,
            tool_calls_ok: metrics.tool_calls_ok,
            tool_calls_fail: metrics.tool_calls_fail,
            tool_usage: metrics.tool_usage.clone(),
            file_usage: metrics.file_usage.clone(),
            tool_arg_usage: metrics.tool_arg_usage.clone(),
            tool_authority: metrics.tool_authority.clone(),
            highest_authority: metrics.highest_authority.clone(),
            reasoning_blocks: metrics.reasoning_blocks,
            reasoning_chars: metrics.reasoning_chars,
            reasoning_lens: metrics.reasoning_lens.clone(),
            reasoning_redact: metrics.reasoning_redact,
            tokens_input: metrics.tokens_input,
            tokens_output: metrics.tokens_output,
            tokens_reasoning: metrics.tokens_reasoning,
            tokens_cache_w: metrics.tokens_cache_w,
            tokens_cache_r: metrics.tokens_cache_r,
            gaps_sec: metrics.gaps_sec.clone(),
            model_used: metrics.model_used.clone(),
            source_tool: metrics.source_tool.clone(),
            session_start: metrics.session_start.clone(),
            session_end: metrics.session_end.clone(),
            naive_utc_stamps: metrics.naive_utc_stamps,
            duration_sec: metrics.duration_sec,
            cost_estimated: metrics.cost_estimated,
            credit_usd: metrics.credit_usd,
            stored_totals_delta: metrics.stored_totals_delta,
            line_skips: metrics.line_skips.clone(),
            zero_usage_events: metrics.zero_usage_events,
            upstream_cost_usd: metrics.upstream_cost_usd,
            upstream_priced_input: metrics.upstream_priced_input,
            upstream_priced_output: metrics.upstream_priced_output,
            upstream_priced_cache_w: metrics.upstream_priced_cache_w,
            upstream_priced_cache_r: metrics.upstream_priced_cache_r,
            disclosure_counters: metrics.disclosure_counters.clone(),
            wire_metadata: metrics.wire_metadata.clone(),
            model_attribution: metrics.model_attribution.clone(),
            provenance: metrics.provenance.clone(),
            session_key: metrics.session_key.clone(),
            parent_session: metrics.parent_session.clone(),
        }
    }

    fn into_metrics(self) -> Metrics {
        Metrics {
            events_total: self.events_total,
            user_messages: self.user_messages,
            assistant_turns: self.assistant_turns,
            tool_results: self.tool_results,
            tool_calls_total: self.tool_calls_total,
            tool_calls_ok: self.tool_calls_ok,
            tool_calls_fail: self.tool_calls_fail,
            tool_usage: self.tool_usage,
            file_usage: self.file_usage,
            tool_arg_usage: self.tool_arg_usage,
            tool_authority: self.tool_authority,
            highest_authority: self.highest_authority,
            reasoning_blocks: self.reasoning_blocks,
            reasoning_chars: self.reasoning_chars,
            reasoning_lens: self.reasoning_lens,
            reasoning_redact: self.reasoning_redact,
            tokens_input: self.tokens_input,
            tokens_output: self.tokens_output,
            tokens_reasoning: self.tokens_reasoning,
            tokens_cache_w: self.tokens_cache_w,
            tokens_cache_r: self.tokens_cache_r,
            timestamps: Vec::new(),
            gaps_sec: self.gaps_sec,
            model_used: self.model_used,
            source_tool: self.source_tool,
            session_start: self.session_start,
            session_end: self.session_end,
            naive_utc_stamps: self.naive_utc_stamps,
            duration_sec: self.duration_sec,
            cost_estimated: self.cost_estimated,
            credit_usd: self.credit_usd,
            stored_totals_delta: self.stored_totals_delta,
            line_skips: self.line_skips.clone(),
            zero_usage_events: self.zero_usage_events,
            upstream_cost_usd: self.upstream_cost_usd,
            // rm-520: the upstream-priced token split round-trips with
            // the row (schema 41) so a cached recorded-cost session
            // keeps its rm-436 basis. Rows written before that bump
            // deserialize as zeros beside a positive
            // `upstream_cost_usd` — governance treats that shape as a
            // basis it must not report as rate drift (rm-578).
            upstream_priced_input: self.upstream_priced_input,
            upstream_priced_output: self.upstream_priced_output,
            upstream_priced_cache_w: self.upstream_priced_cache_w,
            upstream_priced_cache_r: self.upstream_priced_cache_r,
            disclosure_counters: self.disclosure_counters,
            wire_metadata: self.wire_metadata,
            model_attribution: self.model_attribution,
            provenance: self.provenance,
            // rm-790/rm-791: the source row key and the raw parent row id
            // round-trip — they are source data the attribution consumes
            // after load; the rm-545 subagent rollups stay re-derived on
            // every load (discovery.rs), so the cached shape carries zeros
            // for them.
            session_key: self.session_key,
            parent_session: self.parent_session,
            ..Metrics::default()
        }
    }
}

impl GoAnomaly {
    fn from_anomaly(anomaly: &Anomaly) -> Self {
        Self {
            kind: anomaly.kind.clone(),
            severity: anomaly.severity.clone(),
            emoji: anomaly_emoji(&anomaly.severity).to_string(),
            detail: anomaly.detail.clone(),
        }
    }

    fn into_anomaly(self) -> Anomaly {
        Anomaly {
            kind: self.kind,
            severity: self.severity,
            detail: self.detail,
        }
    }
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

fn cache_key(path: &Path) -> String {
    if let Some(text) = path.to_str() {
        return text.to_string();
    }
    #[cfg(unix)]
    {
        lossless_key_from_bytes(std::os::unix::ffi::OsStrExt::as_bytes(path.as_os_str()))
    }
    // Non-unix platforms have no portable lossless byte round trip
    // through JSON keys; the lossy fallback preserves the prior
    // behavior there instead of failing the walk.
    #[cfg(not(unix))]
    {
        path.to_string_lossy().into_owned()
    }
}

/// Prefix marking a key whose source path was not valid UTF-8
/// (rm-041). Cache keys are JSON object keys, so a losslessly-encoded
/// key preserves every byte of the original path instead of
/// collapsing to `U+FFFD` — a `bad\xff\xfename.jsonl` journal used to
/// be invisible through the cached replay (the listing named a file
/// that does not exist) and two distinct non-UTF-8 names could
/// collide on one lossy key. Keys are absolute paths and therefore
/// begin with the root separator, which is what keeps the prefix
/// unambiguous against verbatim keys.
const LOSSLESS_KEY_PREFIX: &str = "at-bytes:";

#[cfg(unix)]
fn lossless_key_from_bytes(bytes: &[u8]) -> String {
    use std::fmt::Write as _;
    let mut key = String::from(LOSSLESS_KEY_PREFIX);
    for &byte in bytes {
        match byte {
            // `%` itself is escaped, so the escape alphabet is
            // self-describing and decoding is unambiguous.
            b'%' => key.push_str("%25"),
            0x21..=0x7E => key.push(byte as char),
            _ => {
                let _ = write!(key, "%{byte:02X}");
            }
        }
    }
    key
}

/// Inverse of `cache_key` (rm-041): decodes a losslessly-encoded key
/// back to the original path. Keys without the prefix are verbatim
/// paths. A prefixed key that does not decode cleanly (a truncated or
/// malformed escape — only reachable through hand-edited journals)
/// fails safe: the key is returned as-is rather than guessed at.
fn path_from_key(key: &str) -> PathBuf {
    let Some(encoded) = key.strip_prefix(LOSSLESS_KEY_PREFIX) else {
        return PathBuf::from(key);
    };
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStringExt;
        decode_lossless_key(encoded)
            .map(|bytes| PathBuf::from(std::ffi::OsString::from_vec(bytes)))
            .unwrap_or_else(|| PathBuf::from(key))
    }
    // Non-unix: encoded keys are never produced (see `cache_key`), so
    // the verbatim fallback is the only honest answer.
    #[cfg(not(unix))]
    {
        PathBuf::from(key)
    }
}

#[cfg(unix)]
fn decode_lossless_key(encoded: &str) -> Option<Vec<u8>> {
    let bytes = encoded.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        let byte = bytes[index];
        if byte == b'%' {
            let hex = bytes.get(index + 1..index + 3)?;
            let high = (hex[0] as char).to_digit(16)?;
            let low = (hex[1] as char).to_digit(16)?;
            decoded.push((high * 16 + low) as u8);
            index += 3;
        } else if byte.is_ascii_graphic() {
            decoded.push(byte);
            index += 1;
        } else {
            return None;
        }
    }
    Some(decoded)
}

fn is_fresh(path: &Path, entry: &CacheEntryHeader) -> bool {
    let Ok(metadata) = fs::metadata(path) else {
        return false;
    };
    entry.size == metadata.len() as i64 && entry.mod_time == file_mod_time_nanos(&metadata)
}

#[cfg(unix)]
fn file_mod_time_nanos(metadata: &fs::Metadata) -> i64 {
    use std::os::unix::fs::MetadataExt;
    metadata.mtime() * 1_000_000_000 + metadata.mtime_nsec()
}

#[cfg(not(unix))]
fn file_mod_time_nanos(metadata: &fs::Metadata) -> i64 {
    metadata
        .modified()
        .ok()
        .and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|duration| duration.as_nanos().min(i64::MAX as u128) as i64)
        .unwrap_or(0)
}

fn anomaly_emoji(severity: &str) -> &'static str {
    match severity {
        "high" => "🔴",
        "medium" => "🟡",
        "low" => "🟢",
        _ => "",
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn concurrent_persist_never_races_on_a_shared_temp_file() {
        // Pass-6 P6-3: cache persist used a fixed `<name>.json.tmp`
        // sibling, so two concurrent writers raced on the same temp file
        // (failed or torn save). With the per-writer suffix every persist
        // must succeed, the final snapshot must load, and no temp files
        // may survive the atomic renames.
        let root = std::env::temp_dir().join(format!(
            "agenttrace-temp-race-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let database = root.join("state.db");
        let snapshot = root.join("snapshot.json");
        fs::create_dir_all(&root).expect("create temp dir");
        fs::write(&database, b"db").expect("write database");
        let session = Session {
            name: "cached".to_string(),
            path: database.to_string_lossy().to_string(),
            cwd: String::new(),
            branch: String::new(),
            metrics: Metrics::default(),
            anomalies: Vec::new(),
            health: 100,
            tool_warnings: Vec::new(),
            diagnostics: Diagnostics::default(),
        };
        let writers: Vec<_> = (0..8)
            .map(|i| {
                std::thread::spawn({
                    let database = database.clone();
                    let snapshot = snapshot.clone();
                    let session = session.clone();
                    move || store_sqlite_snapshot_at(&database, &snapshot, &[session], 0).map(|_| i)
                })
            })
            .collect();
        for writer in writers {
            writer
                .join()
                .expect("writer thread must not panic")
                .expect("concurrent persist must not fail");
        }
        assert_eq!(
            load_sqlite_snapshot_from(&database, &snapshot)
                .expect("snapshot must survive the race")
                .0
                .len(),
            1
        );
        let leftovers: Vec<_> = std::fs::read_dir(&root)
            .expect("read temp dir")
            .flatten()
            .filter(|entry| entry.file_name().to_string_lossy().contains(".tmp."))
            .collect();
        assert!(
            leftovers.is_empty(),
            "temp files must not survive: {leftovers:?}"
        );
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn sqlite_snapshot_is_invalidated_by_database_wal_or_shm_changes() {
        let root = std::env::temp_dir().join(format!(
            "agenttrace-sqlite-cache-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let database = root.join("state.db");
        let snapshot = root.join("snapshot.json");
        fs::create_dir_all(&root).expect("create temp dir");
        fs::write(&database, b"db").expect("write database");
        let session = Session {
            name: "cached".to_string(),
            path: database.to_string_lossy().to_string(),
            cwd: String::new(),
            branch: String::new(),
            metrics: Metrics::default(),
            anomalies: Vec::new(),
            health: 100,
            tool_warnings: Vec::new(),
            diagnostics: Diagnostics::default(),
        };

        store_sqlite_snapshot_at(&database, &snapshot, &[session], 0).expect("store snapshot");
        assert_eq!(
            load_sqlite_snapshot_from(&database, &snapshot)
                .expect("cache hit")
                .0
                .len(),
            1
        );

        fs::write(sqlite_wal_path(&database), b"wal").expect("write wal");
        assert!(load_sqlite_snapshot_from(&database, &snapshot).is_none());

        store_sqlite_snapshot_at(
            &database,
            &snapshot,
            &[Session {
                name: "cached".to_string(),
                path: database.to_string_lossy().to_string(),
                cwd: String::new(),
                branch: String::new(),
                metrics: Metrics::default(),
                anomalies: Vec::new(),
                health: 100,
                tool_warnings: Vec::new(),
                diagnostics: Diagnostics::default(),
            }],
            0,
        )
        .expect("store snapshot with wal");
        fs::write(sqlite_shm_path(&database), b"shm").expect("write shm");
        assert!(load_sqlite_snapshot_from(&database, &snapshot).is_none());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn sqlite_snapshot_schema_ten_round_trips_provenance_and_rejects_older_schemas() {
        let root = std::env::temp_dir().join(format!(
            "agenttrace-sqlite-schema-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let database = root.join("state.db");
        let snapshot = root.join("snapshot.json");
        fs::create_dir_all(&root).expect("create temp dir");
        fs::write(&database, b"db").expect("write database");
        let session = Session {
            name: "cached".to_string(),
            path: database.to_string_lossy().to_string(),
            cwd: String::new(),
            branch: String::new(),
            metrics: Metrics {
                stored_totals_delta: 720,
                // rm-790/rm-791: the source row key and the raw parent row
                // id are SOURCE DATA — they must survive the snapshot
                // round-trip so identity and re-attribution hold on cache
                // hits.
                session_key: "ses-cached".to_string(),
                parent_session: "ses-parent".to_string(),
                provenance: crate::MetricProvenance {
                    tokens: "stored_session_totals".to_string(),
                    duration: "timestamp_span".to_string(),
                    tool_results: "reported_by_agent".to_string(),
                    pricing_source: "LiteLLM (cached)".to_string(),
                    ..crate::MetricProvenance::default()
                },
                ..Metrics::default()
            },
            anomalies: Vec::new(),
            health: 100,
            tool_warnings: Vec::new(),
            diagnostics: Diagnostics::default(),
        };
        store_sqlite_snapshot_at(&database, &snapshot, &[session], 0).expect("store snapshot");
        let raw = fs::read_to_string(&snapshot).expect("read snapshot");
        let doc: serde_json::Value = serde_json::from_str(&raw).expect("snapshot json");
        // Version ten (rm-893): enrichment-lane row drops began
        // counting into the per-file disclosure, so v9 snapshots
        // banked while those drops were silent must regenerate
        // instead of hiding the new drop counts. Version nine carries
        // three rationales: rm-548 — snapshots now carry their opencode
        // fork-exclusion count, so v7 snapshots would disclose a
        // silently-missing count and must regenerate; the rm-734 poison
        // gate — a failed read banks
        // nothing and any EMPTY v7 snapshot a pre-gate run banked for
        // an unreadable database is evicted (the fingerprint alone
        // would keep serving it); and rm-790 — sqlite-backed sessions
        // now carry their source row key and opencode children their
        // parent row id, so the derived-history identity is per-row
        // instead of per-(db, second) and v8 snapshots would re-fold
        // same-second sessions onto one id (one invalidation either
        // way: the candidate's own 7 → 8 landed at base 611242d1 and
        // re-bases onto the advanced ceiling as 8 → 9). Version seven
        // (cycle-1 rm-198): hermes tool outcome semantics changed
        // (ok/fail now derive from the messages table), so v6 snapshots
        // carry stale splits.
        assert_eq!(doc["schema_version"], 10);
        assert_eq!(
            doc.pointer("/sessions/0/Metrics/Provenance/Tokens")
                .and_then(serde_json::Value::as_str),
            Some("stored_session_totals")
        );
        assert_eq!(
            doc.pointer("/sessions/0/Metrics/StoredTotalsDelta"),
            Some(&serde_json::Value::from(720)),
            "the stored-versus-derived delta must survive the snapshot cache"
        );
        let loaded = load_sqlite_snapshot_from(&database, &snapshot).expect("schema ten cache hit");
        let (loaded, _) = loaded;
        assert_eq!(loaded[0].metrics.provenance.duration, "timestamp_span");
        assert_eq!(loaded[0].metrics.stored_totals_delta, 720);
        assert_eq!(loaded[0].metrics.provenance.tokens, "stored_session_totals");
        assert_eq!(
            loaded[0].metrics.session_key, "ses-cached",
            "rm-790: the source row key round-trips through the snapshot"
        );
        assert_eq!(
            loaded[0].metrics.parent_session, "ses-parent",
            "rm-791: the raw parent row id round-trips for post-load attribution"
        );
        let mut old = doc;
        old["schema_version"] = serde_json::Value::from(9);
        fs::write(
            &snapshot,
            serde_json::to_vec(&old).expect("schema nine json"),
        )
        .expect("write old snapshot");
        assert!(load_sqlite_snapshot_from(&database, &snapshot).is_none());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn orphaned_temp_siblings_are_swept_on_cache_load() {
        // Pass-7 P7-5: a crashed writer leaves `<name>.json.tmp.<pid>.<seq>`
        // siblings behind forever. The sweep (run when the cache loads,
        // production age one hour) removes only temp siblings past the
        // age; real cache files are never touched.
        let root = std::env::temp_dir().join(format!(
            "agenttrace-sweep-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        fs::create_dir_all(&root).expect("create temp dir");
        let cache = root.join("sessions.json");
        fs::write(&cache, b"{}").expect("write cache");
        let orphan = unique_temp_path(&cache);
        fs::write(&orphan, b"torn").expect("write orphan temp");
        let neighbor = root.join("unrelated.txt");
        fs::write(&neighbor, b"keep").expect("write neighbor");
        let removed = sweep_orphaned_temps(&cache, std::time::Duration::ZERO);
        assert_eq!(removed, 1, "the orphaned temp is removed");
        assert!(!orphan.exists(), "orphan temp is gone");
        assert!(cache.exists(), "the cache itself is untouched");
        assert!(neighbor.exists(), "non-temp neighbors are untouched");
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    #[cfg(unix)]
    fn write_private_refuses_a_preplanted_symlink_instead_of_following_it() {
        // rm-693: every atomic writer stages through a predictable
        // `<name>.tmp.<pid>.<seq>` sibling, and the staging open used
        // create+truncate — a symlink planted at that name was FOLLOWED:
        // the victim behind the link was clobbered and the rename left
        // the destination AS the link (a persistent implant). The
        // staging open must be O_EXCL: a pre-existing path is refused,
        // never truncated through.
        let root = std::env::temp_dir().join(format!(
            "agenttrace-stage-excl-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        fs::create_dir_all(&root).expect("create temp dir");
        let staged = root.join("sessions.json.tmp.planted");
        let victim = root.join("victim.txt");
        fs::write(&victim, "secret bytes").expect("write victim");
        std::os::unix::fs::symlink(&victim, &staged).expect("plant symlink");
        let error = write_private(&staged, b"attacker bytes").expect_err("symlink must be refused");
        assert_eq!(
            error.kind(),
            std::io::ErrorKind::AlreadyExists,
            "staging must refuse with AlreadyExists, got {error:?}"
        );
        assert_eq!(
            fs::read(&victim).unwrap(),
            b"secret bytes",
            "the victim behind the planted link must be untouched"
        );
        assert!(
            fs::symlink_metadata(&staged)
                .unwrap()
                .file_type()
                .is_symlink(),
            "the planted link itself is left in place"
        );
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    #[cfg(unix)]
    fn exclusive_staging_bumps_the_sequence_past_a_planted_temp() {
        // rm-693: one planted symlink at the next temp name must not
        // fail the write — the writer bumps the sequence and stages at
        // the following sibling — with the victim behind the link
        // intact and the destination landing as a regular file.
        let root = std::env::temp_dir().join(format!(
            "agenttrace-stage-bump-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        fs::create_dir_all(&root).expect("create temp dir");
        let target = root.join("sessions.json");
        let victim = root.join("victim.txt");
        fs::write(&victim, "secret bytes").expect("write victim");
        let planted = unique_temp_path(&target);
        std::os::unix::fs::symlink(&victim, &planted).expect("plant symlink");
        write_private_exclusive(&target, b"cache bytes")
            .expect("write must succeed via the next sequence number");
        assert_eq!(fs::read(&target).unwrap(), b"cache bytes");
        assert_eq!(
            fs::read(&victim).unwrap(),
            b"secret bytes",
            "the victim behind the planted link must be untouched"
        );
        assert!(
            fs::symlink_metadata(&target).unwrap().file_type().is_file(),
            "the destination must be a regular file, never the link"
        );
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    #[cfg(unix)]
    fn exclusive_staging_fails_honestly_when_every_temp_name_is_occupied() {
        // rm-693: a directory whose every candidate temp name is a
        // planted symlink must yield a loud refusal — never a write
        // through a link, never a truncate, never a silent success.
        // Probe-burn sequence numbers while planting links at the
        // returned names, then hand-format the same-width window past
        // the counter so the writer's retry range is covered wherever
        // it starts.
        let root = std::env::temp_dir().join(format!(
            "agenttrace-stage-poison-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        fs::create_dir_all(&root).expect("create temp dir");
        let target = root.join("sessions.json");
        let victim = root.join("victim.txt");
        fs::write(&victim, "secret bytes").expect("write victim");
        let base = target
            .file_name()
            .and_then(|name| name.to_str())
            .expect("target name is UTF-8")
            .to_string();
        let seq_of = |name: &std::path::Path| {
            name.file_name()
                .and_then(|name| name.to_str())
                .and_then(|name| name.rsplit('.').next())
                .and_then(|name| name.parse::<u64>().ok())
                .expect("temp name carries a numeric sequence")
        };
        let mut last_seq = 0u64;
        for _ in 0..64 {
            let planted = unique_temp_path(&target);
            last_seq = seq_of(&planted);
            std::os::unix::fs::symlink(&victim, &planted).expect("plant probe symlink");
        }
        for seq in (last_seq + 1)..=(last_seq + 64) {
            let planted = root.join(format!("{base}.tmp.{}.{}", std::process::id(), seq));
            std::os::unix::fs::symlink(&victim, &planted).expect("plant forward symlink");
        }
        let error = write_private_exclusive(&target, b"cache bytes")
            .expect_err("a fully poisoned temp window must fail the write");
        assert_eq!(error.kind(), std::io::ErrorKind::AlreadyExists);
        let message = error.to_string();
        assert!(
            message.contains("refusing to stage"),
            "the refusal must say so: {message}"
        );
        assert!(
            message.contains(&target.display().to_string()),
            "the refusal must name the destination: {message}"
        );
        assert!(!target.exists(), "no destination may materialize");
        assert_eq!(
            fs::read(&victim).unwrap(),
            b"secret bytes",
            "the victim behind every planted link must be untouched"
        );
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    #[cfg(unix)]
    fn statusline_journal_append_refuses_a_preplanted_symlink() {
        // rm-693: the journal writer appends at its final path (append
        // semantics need one inode), so first creation is O_EXCL and an
        // existing path is appended through only after inspection: a
        // planted symlink at the journal path is refused with the
        // victim behind it intact, while a regular journal keeps
        // appending in place.
        use std::io::Write;
        let root = std::env::temp_dir().join(format!(
            "agenttrace-journal-excl-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        fs::create_dir_all(&root).expect("create temp dir");
        let journal = root.join("statusline.jsonl");
        let victim = root.join("victim.log");
        fs::write(&victim, "secret line\n").expect("write victim");
        std::os::unix::fs::symlink(&victim, &journal).expect("plant journal symlink");
        let error = open_private_append(&journal).expect_err("symlinked journal must be refused");
        let message = error.to_string();
        assert!(
            message.contains("refusing to append"),
            "refusal must say so: {message}"
        );
        assert_eq!(
            fs::read(&victim).unwrap(),
            b"secret line\n",
            "the victim behind the planted link must be untouched"
        );
        assert!(
            fs::symlink_metadata(&journal)
                .unwrap()
                .file_type()
                .is_symlink(),
            "the planted link itself is left in place"
        );
        // Remove the link; a real journal is created owner-only and
        // keeps appending in place across calls.
        fs::remove_file(&journal).expect("drop planted link");
        let mut first = open_private_append(&journal).expect("fresh journal appends");
        writeln!(first, "first").expect("append first line");
        drop(first);
        use std::os::unix::fs::PermissionsExt;
        let mode = fs::metadata(&journal).unwrap().permissions().mode() & 0o777;
        assert_eq!(
            mode, 0o600,
            "fresh journal must be owner-only (got {mode:o})"
        );
        let mut second = open_private_append(&journal).expect("existing regular journal appends");
        writeln!(second, "second").expect("append second line");
        drop(second);
        assert_eq!(
            fs::read_to_string(&journal).unwrap(),
            "first\nsecond\n",
            "appends must land in place, not replace the journal"
        );
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn dead_paths_are_pruned_on_load_and_do_not_resurrect() {
        // Pass-8 F8-3: cache entries whose source file no longer exists
        // used to accumulate forever (761 of 1,487 entries dead on the
        // operator snapshot). Pruning at load marks the cache dirty so
        // the next save persists the smaller snapshot; live entries and
        // dir listings are untouched.
        let root = std::env::temp_dir().join(format!(
            "agenttrace-prune-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        fs::create_dir_all(&root).expect("create temp dir");
        let live = root.join("live.jsonl");
        let dead = root.join("dead.jsonl");
        let listed = root.join("listed");
        let ghost = root.join("ghost");
        fs::write(&live, b"session").expect("write live file");
        fs::write(&dead, b"session").expect("write dead file");
        fs::create_dir_all(&listed).expect("create listed dir");

        let session = Session {
            name: "cached".to_string(),
            path: live.to_string_lossy().to_string(),
            cwd: String::new(),
            branch: String::new(),
            metrics: Metrics::default(),
            anomalies: Vec::new(),
            health: 100,
            tool_warnings: Vec::new(),
            diagnostics: Diagnostics::default(),
        };
        let mut cache = SessionCache::default();
        store_session(&live, &session, &mut cache).expect("store live entry");
        store_session(&dead, &session, &mut cache).expect("store dead entry");
        let listing = DirCacheEntry {
            mod_time: 0,
            dir_size: None,
            files: Vec::new(),
            dirs: Vec::new(),
            fork_parents: BTreeMap::new(),
        };
        cache
            .dirs
            .insert(listed.to_string_lossy().to_string(), listing.clone());
        cache
            .dirs
            .insert(ghost.to_string_lossy().to_string(), listing);
        assert_eq!(cache.entry_count(), 2);

        fs::remove_file(&dead).expect("delete dead source");
        let pruned = prune_dead_entries(&mut cache);
        assert_eq!(pruned, 2, "one dead entry plus one dead dir");
        assert_eq!(cache.entry_count(), 1, "the live entry survives");
        assert!(cache.dirty, "pruning must persist through the next save");
        assert!(
            cache
                .dirs
                .contains_key(&listed.to_string_lossy().to_string()),
            "live dir listings are untouched"
        );
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn entry_bound_evicts_oldest_mod_time_first() {
        // Pass-8 F8-3: the snapshot used to grow without bound. Past
        // MAX_SESSION_CACHE_ENTRIES the entries with the oldest source
        // mtime drop first (oldest work is least likely to be re-read).
        let root = std::env::temp_dir().join(format!(
            "agenttrace-bound-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        fs::create_dir_all(&root).expect("create temp dir");
        let mut cache = SessionCache::default();
        let mut paths = Vec::new();
        for i in 0..5u64 {
            let file = root.join(format!("session-{i}.jsonl"));
            fs::write(&file, b"session").expect("write session file");
            let stamp = std::time::SystemTime::UNIX_EPOCH
                + std::time::Duration::from_secs(1_000_000 + i * 1_000);
            let handle = fs::File::options()
                .write(true)
                .open(&file)
                .expect("open file");
            handle.set_modified(stamp).expect("set deterministic mtime");
            drop(handle);
            let session = Session {
                name: format!("session-{i}"),
                path: file.to_string_lossy().to_string(),
                cwd: String::new(),
                branch: String::new(),
                metrics: Metrics::default(),
                anomalies: Vec::new(),
                health: 100,
                tool_warnings: Vec::new(),
                diagnostics: Diagnostics::default(),
            };
            store_session(&file, &session, &mut cache).expect("store entry");
            paths.push(file.to_string_lossy().to_string());
        }
        assert_eq!(cache.entry_count(), 5);

        let dropped = enforce_entry_bound(&mut cache, 3);
        assert_eq!(dropped, 2, "the two oldest entries drop");
        assert_eq!(cache.entry_count(), 3);
        assert!(cache.dirty, "eviction must persist through the next save");
        assert!(
            !paths[..2]
                .iter()
                .any(|path| cache.entries.contains_key(&cache_key(Path::new(path)))),
            "oldest mtimes are the ones evicted"
        );
        assert!(
            paths[2..]
                .iter()
                .all(|path| cache.entries.contains_key(&cache_key(Path::new(path)))),
            "newest mtimes all survive"
        );
        // Idempotent: an in-bounds cache is left alone and stays clean.
        let before = cache.dirty;
        let dropped_again = enforce_entry_bound(&mut cache, 3);
        assert_eq!(dropped_again, 0);
        assert_eq!(cache.dirty, before);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn entry_bound_resolution_precedence_and_clamping() {
        // rm-298 capacity arm: the effective bound resolves config
        // table > env > default, and both knob layers clamp into
        // [MIN_CONFIGURED, MAX_CONFIGURABLE]. The resolver is pure so
        // every arm is testable without touching the process-global
        // table or the environment.
        assert_eq!(
            resolve_session_cache_entries(None, None),
            MAX_SESSION_CACHE_ENTRIES,
            "no knob set: the built-in default governs"
        );
        assert_eq!(
            resolve_session_cache_entries(None, Some("1500")),
            1_500,
            "env knob parses and wins over the default"
        );
        assert_eq!(
            resolve_session_cache_entries(None, Some(" 42 ")),
            42,
            "surrounding whitespace is tolerated, like a shell export"
        );
        assert_eq!(
            resolve_session_cache_entries(Some(777), Some("1500")),
            777,
            "the config table outranks the env knob"
        );
        assert_eq!(
            resolve_session_cache_entries(None, Some("garbage-not-a-number")),
            MAX_SESSION_CACHE_ENTRIES,
            "an unparseable env knob is ignored, not fatal"
        );
        assert_eq!(
            resolve_session_cache_entries(None, Some("-5")),
            MAX_SESSION_CACHE_ENTRIES,
            "a negative value fails usize parsing and falls to default"
        );
        assert_eq!(
            resolve_session_cache_entries(None, Some("0")),
            MIN_CONFIGURED_SESSION_CACHE_ENTRIES,
            "zero clamps up to the floor instead of disabling the cache"
        );
        assert_eq!(
            resolve_session_cache_entries(None, Some("999999999")),
            MAX_CONFIGURABLE_SESSION_CACHE_ENTRIES,
            "a runaway value clamps down to the ceiling"
        );
        assert_eq!(
            resolve_session_cache_entries(Some(0), None),
            MIN_CONFIGURED_SESSION_CACHE_ENTRIES,
            "the table layer clamps too — the CLI validates, but the
            bound must be safe against any table value"
        );
    }

    #[test]
    fn effective_entry_bound_reads_the_env_knob() {
        // rm-298 capacity arm: the live wrapper threads the real
        // environment into the resolver. The process-global config
        // table is deliberately NOT exercised here — the
        // runtime_config tests may have installed one by now, and its
        // first-writer-wins shape means this test cannot assert the
        // table arm without racing them; the table value for this
        // knob is None in every table any current test installs, so
        // the env arm below is what this assert pins.
        let _env = crate::test_env::lock_env();
        std::env::remove_var("AGENTTRACE_SESSION_CACHE_ENTRIES");
        assert_eq!(effective_session_cache_entries(), MAX_SESSION_CACHE_ENTRIES);
        assert_eq!(session_cache_entries_source(), "default");
        std::env::set_var("AGENTTRACE_SESSION_CACHE_ENTRIES", "1500");
        assert_eq!(effective_session_cache_entries(), 1_500);
        assert_eq!(session_cache_entries_source(), "env");
        std::env::set_var("AGENTTRACE_SESSION_CACHE_ENTRIES", "999999999");
        assert_eq!(
            effective_session_cache_entries(),
            MAX_CONFIGURABLE_SESSION_CACHE_ENTRIES
        );
        // An unparseable knob leaves the default in charge.
        std::env::set_var("AGENTTRACE_SESSION_CACHE_ENTRIES", "many");
        assert_eq!(effective_session_cache_entries(), MAX_SESSION_CACHE_ENTRIES);
        std::env::remove_var("AGENTTRACE_SESSION_CACHE_ENTRIES");
    }

    #[test]
    fn entry_bound_honors_a_configured_maximum_at_save_time() {
        // rm-298 capacity arm: save_session_cache enforces the
        // EFFECTIVE bound, not the const — with the env knob set to a
        // smaller value, a save evicts down to it. This is the
        // eviction arm the doctor's `entries<=N` line discloses.
        let _env = crate::test_env::lock_env();
        std::env::set_var("AGENTTRACE_SESSION_CACHE_ENTRIES", "2");
        let root = std::env::temp_dir().join(format!(
            "agenttrace-configured-bound-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        fs::remove_dir_all(&root).ok();
        fs::create_dir_all(&root).expect("create temp dir");
        let cache_path = root.join("sessions.json");
        let mut cache = SessionCache {
            path: cache_path.clone(),
            ..SessionCache::default()
        };
        let mut stamps = Vec::new();
        for i in 0..4u64 {
            let file = root
                .join(format!("session-{i}.jsonl"))
                .to_string_lossy()
                .to_string();
            fs::write(&file, b"session").expect("write source file");
            let stamp =
                std::time::SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(2_000_000 + i);
            let handle = fs::File::options()
                .write(true)
                .open(&file)
                .expect("open for mtime");
            handle.set_modified(stamp).expect("deterministic mtime");
            drop(handle);
            stamps.push(file.clone());
            let session = Session {
                name: format!("session-{i}"),
                path: file,
                cwd: String::new(),
                metrics: Metrics::default(),
                anomalies: Vec::new(),
                health: 100,
                tool_warnings: Vec::new(),
                diagnostics: Diagnostics::default(),
                branch: String::new(),
            };
            store_session(Path::new(&stamps[i as usize]), &session, &mut cache)
                .expect("store entry");
        }
        assert_eq!(cache.entry_count(), 4);
        save_session_cache(&mut cache).expect("save under bound 2");
        assert_eq!(
            cache.entry_count(),
            2,
            "the configured bound — not the 20,000 const — governs eviction"
        );
        assert!(
            !cache
                .entries
                .contains_key(&cache_key(Path::new(&stamps[0]))),
            "the oldest sources are the ones the configured bound drops"
        );
        assert!(
            cache
                .entries
                .contains_key(&cache_key(Path::new(&stamps[3]))),
            "the newest sources survive"
        );
        std::env::remove_var("AGENTTRACE_SESSION_CACHE_ENTRIES");
        let _ = fs::remove_dir_all(root);
    }
    #[test]
    fn tokens_reasoning_survives_the_cache_round_trip() {
        // CU-20: the reasoning breakdown is part of the cached metrics
        // schema (TokensReasoning); a cache written by this build must
        // restore it, and one written before the field existed must
        // default to zero instead of failing to deserialize.
        let metrics = Metrics {
            tokens_input: 80,
            tokens_output: 60,
            tokens_reasoning: 40,
            ..Metrics::default()
        };
        let cached = GoMetrics::from_metrics(&metrics);
        assert_eq!(cached.tokens_reasoning, 40);
        let restored: Metrics = cached.into_metrics();
        assert_eq!(restored.tokens_reasoning, 40);
        assert_eq!(restored.tokens_output, 60);

        // Old-schema JSON without TokensReasoning deserializes to zero.
        let legacy = serde_json::json!({
            "TokensInput": 5,
            "TokensOutput": 5
        });
        let legacy: GoMetrics = serde_json::from_value(legacy).expect("legacy entry parses");
        assert_eq!(legacy.tokens_reasoning, 0);
    }

    #[test]
    fn byte_bound_evicts_oldest_entries_until_the_estimate_fits() {
        // CU-22: the entry-count bound alone cannot stop unbounded growth;
        // once the serialized estimate exceeds the ceiling the oldest
        // source files drop first, in the same order as the count bound.
        // Hold the env lock: `save_session_cache` enforces the EFFECTIVE
        // entry bound, and the knob tests set
        // `AGENTTRACE_SESSION_CACHE_ENTRIES` in this process — without
        // the lock this fixture races them and evicts to their knob.
        let _env = crate::test_env::lock_env();
        let root = std::env::temp_dir().join(format!(
            "agenttrace-byte-bound-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        fs::create_dir_all(&root).expect("create temp dir");
        let mut cache = SessionCache::default();
        let mut paths = Vec::new();
        for i in 0..4u64 {
            let file = root.join(format!("session-{i}.jsonl"));
            fs::write(&file, b"session").expect("write session file");
            let stamp = std::time::SystemTime::UNIX_EPOCH
                + std::time::Duration::from_secs(1_000_000 + i * 1_000);
            let handle = fs::File::options()
                .write(true)
                .open(&file)
                .expect("open file");
            handle.set_modified(stamp).expect("set deterministic mtime");
            drop(handle);
            let session = Session {
                name: format!("session-{i}"),
                path: file.to_string_lossy().to_string(),
                cwd: String::new(),
                branch: String::new(),
                metrics: Metrics::default(),
                anomalies: Vec::new(),
                health: 100,
                tool_warnings: Vec::new(),
                diagnostics: Diagnostics::default(),
            };
            store_session(&file, &session, &mut cache).expect("store entry");
            paths.push(file.to_string_lossy().to_string());
        }

        // Four default entries are tiny; use a ceiling that fits only the
        // newest two to exercise the eviction path deterministically.
        // Byte-true ceiling (rm-298): the whole written document minus
        // the two oldest members, each freeing its `key`:value plus one
        // comma (four members remain as each drops). BTreeMap order is
        // session-0..3, matching the ascending mtimes.
        let members: Vec<usize> = cache
            .entries
            .iter()
            .map(|(path, entry)| {
                json_key_len(path) + 1 + serde_json::to_string(entry).expect("serialize").len() + 1
            })
            .collect();
        let max = serialized_doc_size(&cache) - members[0] - members[1];
        let dropped = enforce_byte_bound(&mut cache, max);
        assert_eq!(dropped, 2, "the two oldest entries drop first");
        assert_eq!(cache.entry_count(), 2);
        assert!(cache.dirty, "eviction must persist through the next save");
        assert!(
            !paths[..2]
                .iter()
                .any(|path| cache.entries.contains_key(&cache_key(Path::new(path)))),
            "oldest mtimes are the ones evicted"
        );
        assert!(
            paths[2..]
                .iter()
                .all(|path| cache.entries.contains_key(&cache_key(Path::new(path)))),
            "newest mtimes survive"
        );
        // In-bounds caches stay untouched and clean.
        let before = cache.dirty;
        assert_eq!(enforce_byte_bound(&mut cache, usize::MAX), 0);
        assert_eq!(cache.dirty, before);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn bounds_count_deduplicated_paths_once() {
        // A11-2 (cycle 7): a decoded entry lives in both `entries` and
        // `raw_entries` (see `cached_entry`). Chaining the two maps'
        // keys counted such a path twice, so the entry bound wasted
        // drop slots and under-dropped in one pass (the two oldest
        // slots were the same path), and the byte bound summed both
        // copies and over-evicted near the ceiling. Both bounds now
        // walk the deduplicated union.
        // Hold the env lock: the count bound honors the effective
        // `AGENTTRACE_SESSION_CACHE_ENTRIES` knob, which sibling knob
        // tests mutate in this process mid-test otherwise.
        let _env = crate::test_env::lock_env();
        let root = std::env::temp_dir().join(format!(
            "agenttrace-union-bound-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        fs::create_dir_all(&root).expect("create temp dir");
        let mut cache = SessionCache::default();
        let mut paths = Vec::new();
        for i in 0..4u64 {
            let file = root.join(format!("session-{i}.jsonl"));
            fs::write(&file, b"session").expect("write session file");
            let stamp = std::time::SystemTime::UNIX_EPOCH
                + std::time::Duration::from_secs(2_000_000 + i * 1_000);
            let handle = fs::File::options()
                .write(true)
                .open(&file)
                .expect("open file");
            handle.set_modified(stamp).expect("set deterministic mtime");
            drop(handle);
            let session = Session {
                name: format!("session-{i}"),
                path: file.to_string_lossy().to_string(),
                cwd: String::new(),
                branch: String::new(),
                metrics: Metrics::default(),
                anomalies: Vec::new(),
                health: 100,
                tool_warnings: Vec::new(),
                diagnostics: Diagnostics::default(),
            };
            store_session(&file, &session, &mut cache).expect("store entry");
            paths.push(file.to_string_lossy().to_string());
        }
        // Simulate a loaded cache: every decoded entry is mirrored in
        // raw form exactly the way a load + re-decode leaves it.
        cache.raw_entries = cache
            .entries
            .iter()
            .map(|(path, entry)| {
                (
                    path.clone(),
                    serde_json::to_value(entry).expect("serialize entry"),
                )
            })
            .collect();

        assert_eq!(cache.entry_count(), 4);
        assert_eq!(
            cache_paths_sized_once(&cache).len(),
            4,
            "the union helper must size each path once, not once per map copy"
        );

        let dropped = enforce_entry_bound(&mut cache, 2);
        assert_eq!(dropped, 2, "two distinct paths drop, not two copies of one");
        assert_eq!(cache.entry_count(), 2);
        for path in &paths[..2] {
            let key = cache_key(Path::new(path));
            assert!(!cache.entries.contains_key(&key));
            assert!(!cache.raw_entries.contains_key(&key));
        }
        for path in &paths[2..] {
            let key = cache_key(Path::new(path));
            assert!(cache.entries.contains_key(&key));
        }

        // Byte bound over the same duplicated state: the ceiling fits
        // exactly the two newest distinct paths, so exactly those two
        // survive — the pre-fix double-counted total evicted more.
        let mut cache = SessionCache::default();
        for (i, path) in paths.iter().enumerate() {
            let file = PathBuf::from(path);
            let session = Session {
                name: format!("session-{i}"),
                path: path.clone(),
                cwd: String::new(),
                branch: String::new(),
                metrics: Metrics::default(),
                anomalies: Vec::new(),
                health: 100,
                tool_warnings: Vec::new(),
                diagnostics: Diagnostics::default(),
            };
            store_session(&file, &session, &mut cache).expect("store entry");
        }
        cache.raw_entries = cache
            .entries
            .iter()
            .map(|(path, entry)| {
                (
                    path.clone(),
                    serde_json::to_value(entry).expect("serialize entry"),
                )
            })
            .collect();
        let size_of: std::collections::BTreeMap<String, usize> =
            cache_paths_sized_once(&cache).into_iter().collect();
        assert!(
            size_of.values().all(|bytes| *bytes > 0),
            "entries must be sized, not zeroed"
        );
        // `paths` was stored oldest-mtime first; the byte-true ceiling
        // (rm-298) fits exactly the two newest distinct paths: the whole
        // written document minus the two oldest members, each freeing
        // its `key`:value plus one comma (four members remain as each
        // drops).
        let member = |path: &str| json_key_len(path) + 1 + size_of[path] + 1;
        let max = serialized_doc_size(&cache) - member(&paths[0]) - member(&paths[1]);
        let dropped = enforce_byte_bound(&mut cache, max);
        assert_eq!(dropped, 2, "the two oldest distinct paths drop, exactly");
        assert_eq!(cache.entry_count(), 2);
        for path in &paths[..2] {
            let key = cache_key(Path::new(path));
            assert!(!cache.entries.contains_key(&key));
            assert!(!cache.raw_entries.contains_key(&key));
        }
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn headerless_cache_entries_are_oldest_not_unevictable() {
        // F5-5 (cycle-5 review, carried into the union form): a raw
        // entry without a decodable header used to be skipped by the
        // bounds entirely, so corrupt entries could pin the cache at
        // its ceiling forever. They count as the oldest age instead.
        let mut cache = SessionCache::default();
        cache.raw_entries.insert(
            "/gone/headerless-entry".to_string(),
            serde_json::json!({"nonsense": true}),
        );
        cache.raw_entries.insert(
            "/gone/dated-entry".to_string(),
            serde_json::json!({"mod_time": 5, "size": 1, "session": {"Name": "dated"}}),
        );
        assert_eq!(cache.entry_count(), 2);
        let dropped = enforce_entry_bound(&mut cache, 1);
        assert_eq!(dropped, 1);
        assert!(
            !cache.raw_entries.contains_key("/gone/headerless-entry"),
            "the headerless entry is the oldest and must be evictable"
        );
        assert!(cache.raw_entries.contains_key("/gone/dated-entry"));
    }

    #[test]
    fn byte_bound_covers_the_written_document_not_a_model() {
        // rm-298: `enforce_byte_bound` used to size entry VALUES only,
        // while `save_session_cache` also writes per-path keys, JSON
        // punctuation, the top-level fields, and the whole `dirs` map —
        // on a real 5,293-entry corpus that gap wrote sessions.json at
        // 72,257,626 B = 108.9% of the 64 MiB "hard bound" (assess
        // c22757c9). The contract the doc comment states is about the
        // WRITTEN FILE, so the fixture asserts on the file on disk, not
        // on the estimator's model of it: with a ceiling the old
        // values-only estimate called in-bounds, the written document
        // must still fit under it, and eviction must be oldest-first.
        // Hold the env lock: the save path enforces the effective
        // `AGENTTRACE_SESSION_CACHE_ENTRIES` knob, which the sibling
        // knob tests mutate in this process otherwise.
        let _env = crate::test_env::lock_env();
        let root = std::env::temp_dir().join(format!(
            "agenttrace-byte-true-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        fs::create_dir_all(&root).expect("create temp dir");
        let mut cache = SessionCache {
            path: root.join("sessions.json"),
            ..SessionCache::default()
        };
        for i in 0..8i64 {
            let path = format!("/corpus/projects/probe/session-{i:02}-with-a-long-key-name.jsonl");
            let value = serde_json::json!({
                "mod_time": 1_000_000 + i * 1_000,
                "size": 1,
                "session": {
                    "Name": format!("session-{i}"),
                    "Path": path,
                    "pad": "x".repeat(1_500),
                },
            });
            cache.raw_entries.insert(path, value);
        }
        for i in 0..4i64 {
            cache.dirs.insert(
                format!("/corpus/projects/probe-{i}"),
                DirCacheEntry {
                    mod_time: 500_000 + i,
                    dir_size: None,
                    files: vec!["f".repeat(120); 6],
                    dirs: Vec::new(),
                    fork_parents: BTreeMap::new(),
                },
            );
        }
        // The pre-rm-298 model's in-bounds ceiling: the values-only sum.
        let values_only: usize = cache_paths_sized_once(&cache).iter().map(|(_, b)| *b).sum();
        let max = values_only;
        enforce_byte_bound(&mut cache, max);
        save_session_cache(&mut cache).expect("save cache");
        let written = fs::metadata(&cache.path).expect("cache file written").len() as usize;
        assert!(
            written <= max,
            "the written sessions.json must obey the byte bound: {written} > {max}"
        );
        // Eviction stays oldest-mtime-first and never touches dirs.
        assert!(
            !cache
                .raw_entries
                .contains_key("/corpus/projects/probe/session-00-with-a-long-key-name.jsonl"),
            "the oldest entry is the first to go"
        );
        assert!(
            cache
                .raw_entries
                .contains_key("/corpus/projects/probe/session-07-with-a-long-key-name.jsonl"),
            "the newest entry survives"
        );
        assert_eq!(
            cache.dirs.len(),
            4,
            "entry eviction never drops dir listings"
        );
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn serialized_doc_size_predicts_the_written_file_exactly() {
        // rm-298: the byte bound is only as honest as its projection.
        // Pin that `serialized_doc_size` equals the file
        // `save_session_cache` writes, byte for byte — including a key
        // that needs JSON escaping, a path living in both maps (the
        // decoded copy overwrites the raw copy at save time), and a
        // non-empty dirs map — so any drift between the model and the
        // writer fails here first. Hold the env lock: the save path
        // enforces the effective `AGENTTRACE_SESSION_CACHE_ENTRIES`
        // knob, which the sibling knob tests mutate in this process —
        // without it the written file can evict to their knob and the
        // byte-for-byte pin breaks (observed racing the "2" knob).
        let _env = crate::test_env::lock_env();
        let root = std::env::temp_dir().join(format!(
            "agenttrace-doc-size-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        fs::create_dir_all(&root).expect("create temp dir");
        let mut cache = SessionCache {
            path: root.join("sessions.json"),
            ..SessionCache::default()
        };
        for i in 0..3i64 {
            let path = format!("/corpus/projects/probe \"quoted-{i}\"\\slash/s-{i}.jsonl");
            let session = GoSession {
                name: format!("session-{i}"),
                path: path.clone(),
                ..GoSession::default()
            };
            let value = serde_json::json!({
                "mod_time": 900_000 + i * 1_000,
                "size": 1,
                "session": serde_json::to_value(&session).expect("serialize session"),
                "pad": "y".repeat(600),
            });
            cache.raw_entries.insert(path.clone(), value);
            if i == 1 {
                // Raw and decoded copies differ for this path: the
                // decoded copy overwrites the raw one at save time, and
                // the projection must count the WRITTEN form (the raw
                // copy's stray fields never reach the file).
                cache.entries.insert(
                    path,
                    CacheEntry {
                        mod_time: 900_000 + i * 1_000,
                        size: 1,
                        session,
                    },
                );
            }
        }
        cache.dirs.insert(
            "/corpus/projects/probe \"quoted-0\"\\slash".to_string(),
            DirCacheEntry {
                mod_time: 500_000,
                dir_size: None,
                files: vec!["f".repeat(120); 6],
                dirs: Vec::new(),
                fork_parents: BTreeMap::new(),
            },
        );
        let projected = serialized_doc_size(&cache);
        save_session_cache(&mut cache).expect("save cache");
        let written = fs::metadata(&cache.path).expect("cache file written").len() as usize;
        assert_eq!(
            written, projected,
            "the projection must equal the written file byte for byte"
        );
        let doc: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(&cache.path).expect("read cache file"))
                .expect("written cache parses as JSON");
        assert!(
            doc.get("dirs").is_some(),
            "the dirs map is part of the written document"
        );
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn fork_parent_probe_memoizes_and_reprobes_on_rewrite() {
        // rm-548 (independent-review fix): the fork-marker probe memo
        // lives on the parent directory's cached listing, keyed by the
        // file's fingerprint. A fresh memo misses (probe + store), a
        // matching fingerprint serves it, a rewrite — even a same-mtime
        // one caught by the size change — re-probes, and a vanished
        // file never serves a stale answer, so the memo cannot mask a
        // newly-forked session.
        let root = std::env::temp_dir().join(format!(
            "agenttrace-fork-memo-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).expect("create temp dir");
        let info_dir = root.join("storage/session/info");
        fs::create_dir_all(&info_dir).expect("create info dir");
        let info = info_dir.join("s1.json");
        fs::write(&info, "{\"parentID\":\"parent-1\"}").expect("write info doc");
        let mut cache = SessionCache {
            path: root.join("sessions.json"),
            ..SessionCache::default()
        };
        store_dir_listing(&info_dir, &[], &[], &mut cache).expect("store listing");
        assert_eq!(
            cached_fork_parent(&info, &cache),
            None,
            "a fresh memo must miss so the caller probes the file"
        );
        store_fork_parent_probe(&info, Some("parent-1".to_string()), &mut cache);
        assert!(cache.dirty, "storing a probe marks the cache dirty");
        assert_eq!(
            cached_fork_parent(&info, &cache),
            Some(Some("parent-1".to_string())),
            "a stored memo with a matching fingerprint serves without reading the file"
        );
        // A shorter rewrite changes the fingerprint: the memo must
        // miss again even though a memo entry exists.
        fs::write(&info, "{\"parentID\":\"\"}").expect("rewrite info doc");
        assert_eq!(
            cached_fork_parent(&info, &cache),
            None,
            "a rewritten doc must re-probe, never serve the old answer"
        );
        store_fork_parent_probe(&info, None, &mut cache);
        assert_eq!(
            cached_fork_parent(&info, &cache),
            Some(None),
            "a non-fork memo is a real answer, not a miss"
        );
        fs::remove_file(&info).expect("remove info doc");
        assert_eq!(
            cached_fork_parent(&info, &cache),
            None,
            "a vanished file has no usable fingerprint, so no memo serves"
        );
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn dirs_map_gains_count_and_byte_bounds_of_its_own() {
        // rm-298: `dirs` had no bound at all — only entries did — so a
        // broad directory tree grew the snapshot without limit. The
        // listings now drop oldest mtime first until both the count
        // and the serialized-block budgets hold.
        let mut cache = SessionCache::default();
        for i in 0..6i64 {
            cache.dirs.insert(
                format!("/corpus/projects/probe-{i}"),
                DirCacheEntry {
                    mod_time: 400_000 + i * 1_000,
                    dir_size: None,
                    files: vec![format!("file-{i}-{}.jsonl", "f".repeat(60)); 4],
                    dirs: Vec::new(),
                    fork_parents: BTreeMap::new(),
                },
            );
        }
        // Count bound alone: keep the three newest listings.
        let dropped = enforce_dirs_bound(&mut cache, 3, usize::MAX);
        assert_eq!(dropped, 3);
        assert_eq!(cache.dirs.len(), 3);
        assert!(!cache.dirs.contains_key("/corpus/projects/probe-0"));
        assert!(cache.dirs.contains_key("/corpus/projects/probe-5"));
        assert!(cache.dirty, "evicting listings marks the cache dirty");

        // Byte bound: one byte under the current block — the oldest of
        // the three goes and the block fits the budget.
        cache.dirty = false;
        let budget = json_object_len(dirs_member_bytes(&cache), cache.dirs.len()) - 1;
        let dropped = enforce_dirs_bound(&mut cache, usize::MAX, budget);
        assert_eq!(dropped, 1);
        assert_eq!(cache.dirs.len(), 2);
        assert!(!cache.dirs.contains_key("/corpus/projects/probe-3"));
        assert!(
            json_object_len(dirs_member_bytes(&cache), cache.dirs.len()) <= budget,
            "the serialized dirs block must fit the byte budget"
        );
        assert!(cache.dirty);
    }

    #[test]
    fn stale_dir_listings_from_the_pre_symlink_walker_are_dropped_once() {
        // Cycle 7 (Codex #42135): listings written before symlinked
        // child directories were followed silently hid those
        // directories until each parent's mtime changed, because the
        // cached replay never re-read the directory. A journal without
        // the current `dir_listing_version` loses its listings once at
        // load; a current one keeps them.
        let root = std::env::temp_dir().join(format!(
            "agenttrace-listing-version-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        fs::create_dir_all(&root).expect("create temp dir");
        let journal = root.join("sessions.json");
        // The listing key must be a live directory: load-time pruning
        // evicts listings whose directory no longer exists.
        let listed_dir = root.join("real");
        fs::create_dir_all(&listed_dir).expect("create listed dir");
        let listing = serde_json::json!({
            "mod_time": 1,
            "files": [],
            "dirs": [],
        });
        let write_journal = |versioned: bool| {
            let mut doc = serde_json::Map::new();
            doc.insert(
                "schema_version".to_string(),
                serde_json::json!(SESSION_CACHE_SCHEMA_VERSION),
            );
            if versioned {
                doc.insert(
                    "dir_listing_version".to_string(),
                    serde_json::json!(DIR_LISTING_WALK_VERSION),
                );
            }
            doc.insert("entries".to_string(), serde_json::json!({}));
            doc.insert(
                "dirs".to_string(),
                serde_json::json!({listed_dir.to_string_lossy().to_string(): listing}),
            );
            fs::write(
                &journal,
                serde_json::to_string(&doc).expect("serialize journal"),
            )
            .expect("write journal");
        };

        // Shared env lock (see lib.rs `test_env`): sibling-module tests
        // (pricing, statusline) mutate the same variables.
        let _env = crate::test_env::lock_env();
        let prior_cache = std::env::var_os("AGENTTRACE_SESSION_CACHE_DIR");
        std::env::set_var("AGENTTRACE_SESSION_CACHE_DIR", &root);
        write_journal(false);
        let cache = load_session_cache();
        assert_eq!(
            cache.dirs.len(),
            0,
            "unversioned listings from the pre-symlink walker must be dropped"
        );
        assert!(cache.dirty, "the one-time invalidation must persist");

        write_journal(true);
        let cache = load_session_cache();
        assert_eq!(cache.dirs.len(), 1, "current-version listings survive");
        match prior_cache {
            Some(value) => std::env::set_var("AGENTTRACE_SESSION_CACHE_DIR", value),
            None => std::env::remove_var("AGENTTRACE_SESSION_CACHE_DIR"),
        }
        drop(_env);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn pricing_catalog_refresh_invalidates_cached_costs_once() {
        // rm-196: the cache's file fingerprints (mod_time+size) cannot
        // tell a pricing-catalog refresh from an untouched session, so
        // cached per-session costs used to survive snapshot bumps and
        // --update-pricing refreshes at stale rates. The journal now
        // stamps the active catalog identity; a stamp that disagrees
        // with the live catalog drops every entry once (they re-price),
        // an unstamped legacy journal is accepted as-is, and the same
        // stamp keeps every hit.
        let root = std::env::temp_dir().join(format!(
            "agenttrace-pricing-id-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        fs::create_dir_all(&root).expect("create temp dir");
        let journal = root.join("sessions.json");
        // The entry path must exist on disk or load-time pruning would
        // remove it for an unrelated reason.
        let session_file = root.join("session.jsonl");
        fs::write(&session_file, b"{}\n").expect("write session file");
        let stamp = |pricing_catalog_id: Option<&str>| {
            let mut doc = serde_json::Map::new();
            doc.insert(
                "schema_version".to_string(),
                serde_json::json!(SESSION_CACHE_SCHEMA_VERSION),
            );
            if let Some(id) = pricing_catalog_id {
                doc.insert("pricing_catalog_id".to_string(), serde_json::json!(id));
            }
            doc.insert(
                "entries".to_string(),
                serde_json::json!({
                    session_file.to_string_lossy().to_string(): {
                        "mod_time": 1,
                        "size": 1,
                    }
                }),
            );
            doc.insert("dirs".to_string(), serde_json::json!({}));
            fs::write(
                &journal,
                serde_json::to_string(&doc).expect("serialize journal"),
            )
            .expect("write journal");
        };

        // Shared env lock (see lib.rs `test_env`): the load path reads
        // the live pricing catalog for its identity stamp.
        let _env = crate::test_env::lock_env();
        let prior_cache = std::env::var_os("AGENTTRACE_SESSION_CACHE_DIR");
        std::env::set_var("AGENTTRACE_SESSION_CACHE_DIR", &root);

        // The identity read here is whatever the process's active
        // catalog computes; the test only needs internal consistency
        // between the stamp and the loader, never a specific value.
        let live_id = crate::pricing::catalog_identity().to_string();

        stamp(Some("0123456789abcdef"));
        let cache = load_session_cache();
        assert_eq!(
            cache.raw_entries.len(),
            0,
            "a journal stamped with a foreign catalog id must re-price"
        );
        assert!(
            cache.dirty,
            "the invalidation must persist the emptied journal"
        );

        stamp(Some(&live_id));
        let cache = load_session_cache();
        assert_eq!(
            cache.raw_entries.len(),
            1,
            "a journal stamped with the active catalog id keeps its hits"
        );
        assert!(!cache.dirty, "a matching stamp must not dirty the journal");

        stamp(None);
        let cache = load_session_cache();
        assert_eq!(
            cache.raw_entries.len(),
            1,
            "an unstamped legacy journal is accepted, not mass-invalidated"
        );

        match prior_cache {
            Some(value) => std::env::set_var("AGENTTRACE_SESSION_CACHE_DIR", value),
            None => std::env::remove_var("AGENTTRACE_SESSION_CACHE_DIR"),
        }
        drop(_env);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn stale_schema_version_cache_never_serves_entries() {
        // F1 of the cycle-1 independent review (08e99143, fixed at
        // 955c3cea): rm-485 changes reported cost for UNCHANGED copilot
        // files, so a warm cache written by the pre-fix build kept
        // serving cost 0.0 / credit null forever — the fingerprints
        // still match, so nothing re-parses and the cache never
        // self-heals (proven live on a surgically degraded v22 cache).
        // The rm-230 convention: parser-semantics changes bump
        // SESSION_CACHE_SCHEMA_VERSION, and a stale doc must drop its
        // entries at load, never serve them — however well-formed.
        let root = std::env::temp_dir().join(format!(
            "agenttrace-stale-schema-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).expect("create temp cache root");
        let journal = root.join("journal.jsonl");
        fs::write(
            &journal,
            "{\"type\":\"user\",\"message\":{\"role\":\"user\",\"content\":\"hi\"},\"sessionId\":\"s1\",\"timestamp\":\"2026-05-07T02:00:00Z\"}\n",
        )
        .expect("write journal");
        let session = crate::parse_file(&journal).expect("parse journal");

        // Shared env lock (see lib.rs `test_env`): sibling-module tests
        // (pricing, statusline) mutate the same variables.
        let _env = crate::test_env::lock_env();
        let prior_cache = std::env::var_os("AGENTTRACE_SESSION_CACHE_DIR");
        std::env::set_var("AGENTTRACE_SESSION_CACHE_DIR", &root);
        let cache_doc = root.join("sessions.json");
        let mut cache = load_session_cache();
        store_session(&journal, &session, &mut cache).expect("store session");
        save_session_cache(&mut cache).expect("save cache");
        // Sanity: the current schema version serves the stored entry.
        let fresh = load_session_cache();
        assert_eq!(
            fresh.entry_count(),
            1,
            "a current-version cache must serve the stored entry"
        );
        // Surgical version downgrade = a warm cache from the previous
        // build: entries untouched, fingerprints still valid. (Downgrade
        // through serde_json only — jq corrupts >2^53 fingerprint ints,
        // as the review's first attempt proved.)
        let mut doc: Value =
            serde_json::from_str(&fs::read_to_string(&cache_doc).expect("read cache doc"))
                .expect("cache json");
        doc["schema_version"] = serde_json::json!(SESSION_CACHE_SCHEMA_VERSION - 1);
        fs::write(
            &cache_doc,
            serde_json::to_string(&doc).expect("serialize downgraded cache"),
        )
        .expect("write downgraded cache");
        let stale = load_session_cache();
        assert_eq!(
            stale.entry_count(),
            0,
            "a stale-schema cache must drop every entry, never serve it"
        );
        assert!(
            stale.dirty,
            "the one-time invalidation must mark the cache dirty so it regenerates"
        );

        match prior_cache {
            Some(value) => std::env::set_var("AGENTTRACE_SESSION_CACHE_DIR", value),
            None => std::env::remove_var("AGENTTRACE_SESSION_CACHE_DIR"),
        }
        drop(_env);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn clear_cache_removes_every_artifact_and_only_those() {
        // rm-086: --clear-cache used to remove sessions.json and the two
        // SQLite snapshots while leaving the statusline journal and the
        // pricing catalog on disk — the two artifacts a
        // privacy-motivated user most expects to be gone. The artifact
        // set comes from cache_artifact_paths() itself (file names are
        // env-independent), so this fails if a store is added to the
        // code but not to the clear set.
        let root =
            std::env::temp_dir().join(format!("agenttrace-clear-cache-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).expect("create temp cache root");
        let paths: Vec<PathBuf> = cache_artifact_paths()
            .iter()
            .map(|path| root.join(path.file_name().expect("artifact file name")))
            .collect();
        assert_eq!(
            paths.len(),
            6,
            "registry: session cache, its advisory lock, two sqlite snapshots, statusline journal, pricing catalog"
        );
        for path in &paths {
            fs::write(path, b"x").expect("write artifact");
        }
        let bystander = root.join("unrelated.txt");
        fs::write(&bystander, b"x").expect("write bystander");
        // rm-086 review follow-up: superseded-version leftovers from older
        // builds must leave with the purge too (they are invisible to the
        // registry — no live code names them — but they hold the same
        // parsed metrics), while near-miss names stay untouched.
        let legacy_hermes = root.join("hermes-sqlite-v7-20250801.json");
        let legacy_opencode = root.join("opencode-sqlite-v7-20250801.json");
        let legacy_bystander = root.join("other-sqlite-v7.json");
        for path in [&legacy_hermes, &legacy_opencode, &legacy_bystander] {
            fs::write(path, b"x").expect("write legacy artifact");
        }

        // e2e through the real entry point: pin the env-aware roots
        // (the session-cache dir — and XDG_CACHE_HOME too, because the
        // pricing catalog resolves through user_cache_dir(), not through
        // AGENTTRACE_SESSION_CACHE_DIR) and drive clear_session_cache(),
        // which composes the registry with the legacy sweep —
        // remove_cache_artifacts alone would not exercise
        // legacy_cache_artifact_paths() at all. Artifacts are written
        // where the pinned env actually resolves them.
        let _env = crate::test_env::lock_env();
        let previous_cache_root = std::env::var_os("AGENTTRACE_SESSION_CACHE_DIR");
        let previous_xdg_cache = std::env::var_os("XDG_CACHE_HOME");
        std::env::set_var("AGENTTRACE_SESSION_CACHE_DIR", &root);
        std::env::set_var("XDG_CACHE_HOME", root.join("xdg-cache"));
        let live: Vec<PathBuf> = cache_artifact_paths();
        for path in &live {
            fs::create_dir_all(path.parent().expect("artifact parent"))
                .expect("create artifact parent");
            fs::write(path, b"x").expect("write artifact");
        }
        clear_session_cache().expect("clear removes registry + legacy sweep");
        for path in &live {
            assert!(!path.exists(), "{} must be removed", path.display());
        }
        assert!(
            !legacy_hermes.exists(),
            "legacy versioned snapshot must be swept"
        );
        assert!(
            !legacy_opencode.exists(),
            "legacy versioned opencode snapshot must be swept"
        );
        assert!(
            bystander.exists(),
            "clear touches only registered artifacts, not the whole directory"
        );
        assert!(
            legacy_bystander.exists(),
            "the legacy sweep matches only the two store prefixes, not every *-sqlite-v*.json"
        );

        // A second pass is a no-op, not an error: every artifact is
        // already NotFound.
        clear_session_cache().expect("second clear is a no-op");

        match previous_cache_root {
            Some(value) => std::env::set_var("AGENTTRACE_SESSION_CACHE_DIR", value),
            None => std::env::remove_var("AGENTTRACE_SESSION_CACHE_DIR"),
        }
        match previous_xdg_cache {
            Some(value) => std::env::set_var("XDG_CACHE_HOME", value),
            None => std::env::remove_var("XDG_CACHE_HOME"),
        }
        drop(_env);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn privacy_disclosure_lists_every_artifact() {
        // rm-086: PRIVACY.md must disclose every at-rest artifact the
        // code writes — the cache-root registry plus the preserved
        // history file — with its purge path. Names come from the same
        // constructors that build the paths, so adding a store without
        // disclosing it fails here.
        let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join("PRIVACY.md");
        let privacy = fs::read_to_string(&repo_root)
            .unwrap_or_else(|err| panic!("read {}: {err}", repo_root.display()));
        let mut names: Vec<String> = cache_artifact_paths()
            .iter()
            .chain(std::iter::once(&crate::history::history_path()))
            .map(|path| {
                path.file_name()
                    .expect("artifact file name")
                    .to_string_lossy()
                    .to_string()
            })
            .collect();
        names.sort();
        names.dedup();
        for name in &names {
            assert!(
                privacy.contains(name.as_str()),
                "PRIVACY.md must disclose at-rest artifact {name}"
            );
        }
        // rm-086 review follow-up: the swept superseded-version leftovers
        // are disclosed as a pattern row, not as exact file names.
        for pattern in ["hermes-sqlite-v", "opencode-sqlite-v"] {
            assert!(
                privacy.contains(pattern),
                "PRIVACY.md must disclose the legacy {pattern}* purge"
            );
        }
    }

    #[test]
    #[cfg(unix)]
    fn session_cache_and_snapshot_artifacts_are_owner_only() {
        // rm-208: sessions.json and the SQLite snapshots carry session
        // names, tool arguments, and transcript-derived metrics. The
        // fs::write default (0644) hands them to every local account;
        // both artifacts must land owner-only.
        use std::os::unix::fs::PermissionsExt;
        let root = std::env::temp_dir().join(format!(
            "agenttrace-artifact-perms-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let database = root.join("state.db");
        let snapshot = root.join("hermes.db.json");
        fs::create_dir_all(&root).expect("create temp dir");
        fs::write(&database, b"db").expect("write database");
        let session = Session {
            name: "private session".to_string(),
            path: database.to_string_lossy().to_string(),
            cwd: "/work/secret".to_string(),
            branch: String::new(),
            metrics: Metrics::default(),
            anomalies: Vec::new(),
            health: 100,
            tool_warnings: Vec::new(),
            diagnostics: Diagnostics::default(),
        };
        store_sqlite_snapshot_at(&database, &snapshot, &[session], 0).expect("store snapshot");
        let mode = fs::metadata(&snapshot)
            .expect("snapshot exists")
            .permissions()
            .mode();
        assert_eq!(
            mode & 0o077,
            0,
            "sqlite snapshot must be owner-only, got {:o}",
            mode & 0o777
        );
        let mut cache = SessionCache {
            path: root.join("sessions.json"),
            ..Default::default()
        };
        save_session_cache(&mut cache).expect("save cache");
        let mode = fs::metadata(&cache.path)
            .expect("sessions.json exists")
            .permissions()
            .mode();
        assert_eq!(
            mode & 0o077,
            0,
            "sessions.json must be owner-only, got {:o}",
            mode & 0o777
        );
        // The advisory lock sibling exists only while a rewrite holds
        // it (O_EXCL acquire, token-fenced release) and must be
        // owner-only for the while (rm-298 residual) — gone once
        // released.
        let lock = CacheFileLock::acquire(&cache.path).expect("lock acquired");
        let mode = fs::metadata(lock_sibling(&cache.path))
            .expect("sessions.json.lock exists while held")
            .permissions()
            .mode();
        assert_eq!(
            mode & 0o077,
            0,
            "sessions.json.lock must be owner-only, got {:o}",
            mode & 0o777
        );
        drop(lock);
        assert!(
            !lock_sibling(&cache.path).exists(),
            "a released lock leaves no file behind"
        );
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn lock_staleness_is_content_first_and_never_wedges_on_stray_files() {
        // rm-298 residual protocol: an inert file parked on the lock
        // name (or a token naming a process that no longer exists)
        // must be stealable IMMEDIATELY — a purely age-based rule
        // would wedge every save behind the wait budget — while a
        // fresh lock from a live process (ours) and the empty window
        // between create and token-write are respected.
        let root = std::env::temp_dir().join(format!(
            "agenttrace-lock-stale-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).expect("create temp dir");
        let lock_path = lock_sibling(&root.join("sessions.json"));
        fs::create_dir_all(lock_path.parent().expect("parent")).expect("create cache dir");

        // A stray non-token file: stealable at once, whatever its age.
        fs::write(&lock_path, b"x").expect("write stray lock");
        assert!(
            lock_stealable(&lock_path),
            "a stray file never wedges saves"
        );

        // A token naming a dead process: stealable at once (this
        // process is alive by construction, so borrow a pid that is
        // not — /proc has none above pid_max). The liveness probe is
        // unix-only; elsewhere the age lane carries staleness.
        #[cfg(unix)]
        {
            fs::write(&lock_path, format!("999999999-{}", lock_path.display()))
                .expect("write dead-owner lock");
            assert!(
                lock_stealable(&lock_path),
                "a dead owner's lock is stale now"
            );
        }
        assert_eq!(lock_token_pid("999999999-x"), Some(999999999));
        assert_eq!(lock_token_pid("not-a-token"), None);
        assert_eq!(lock_token_pid(""), None);

        // A fresh empty file is a creator between create and
        // token-write: respected inside the grace window. The grace
        // semantics themselves are pinned deterministically on
        // `lock_decision` below (synthetic ages, no clock in the
        // loop); this filesystem leg only confirms the wiring, and
        // tolerates a loaded runner aging the observation past the
        // window before the stat lands — it rewrites the empty file
        // and resamples rather than failing on scheduler noise.
        let mut respected_empty = false;
        for _ in 0..8 {
            fs::write(&lock_path, b"").expect("write empty lock");
            let stealable = lock_stealable(&lock_path);
            let sampled_inside_grace = fs::metadata(&lock_path)
                .ok()
                .and_then(|meta| meta.modified().ok())
                .and_then(|written| written.elapsed().ok())
                .map(|age| age <= CACHE_LOCK_EMPTY_GRACE)
                .unwrap_or(false);
            assert!(
                !(stealable && sampled_inside_grace),
                "a just-created empty lock belongs to a live creator"
            );
            if !stealable {
                respected_empty = true;
                break;
            }
        }
        assert!(
            respected_empty,
            "the empty-file grace window was never observed respected"
        );

        // A live owner (this process) holding a fresh lock: respected.
        // Same tolerance as above — the deterministic age lane is
        // the `lock_decision` pin on the live token below; re-writing
        // our own token is a keepalive that refreshes the lock's age
        // without changing ownership (`still_ours` stays true).
        let lock = CacheFileLock::acquire(&root.join("sessions.json"))
            .expect("acquire over the respected lock states");
        assert!(lock.still_ours(), "the lock we hold carries our token");
        let mut respected_live = false;
        for _ in 0..8 {
            let _ = fs::write(&lock.0, lock.1.as_bytes());
            let stealable = lock_stealable(&lock.0);
            let sampled_fresh = fs::metadata(&lock.0)
                .ok()
                .and_then(|meta| meta.modified().ok())
                .and_then(|written| written.elapsed().ok())
                .map(|age| age <= CACHE_LOCK_STALE_AFTER)
                .unwrap_or(false);
            assert!(
                !(stealable && sampled_fresh),
                "a fresh lock from a live process is not stealable"
            );
            if !stealable {
                respected_live = true;
                break;
            }
        }
        assert!(
            respected_live,
            "a live owner's fresh lock was never observed respected"
        );
        drop(lock);
        assert!(!lock_path.exists(), "release removes the lock file");

        // Deterministic age-lane pins (no filesystem timestamps in
        // the loop): the empty grace window and the wedged-but-alive
        // threshold are respected below their bounds and stealable
        // above them, while content-first classes (stray, dead
        // owner) steal at any age — this process is alive by
        // construction, so its token exercises the live lane.
        assert!(!lock_decision("", std::time::Duration::from_millis(10)));
        assert!(lock_decision(
            "",
            CACHE_LOCK_EMPTY_GRACE + std::time::Duration::from_millis(10)
        ));
        let live_token = format!("{}-held", std::process::id());
        assert!(!lock_decision(
            &live_token,
            std::time::Duration::from_millis(10)
        ));
        assert!(lock_decision(
            &live_token,
            CACHE_LOCK_STALE_AFTER + std::time::Duration::from_millis(10)
        ));
        assert!(lock_decision(
            "not-a-token",
            std::time::Duration::from_millis(10)
        ));
        #[cfg(unix)]
        assert!(lock_decision(
            "999999999-x",
            std::time::Duration::from_millis(10)
        ));

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn lossless_key_encoding_round_trips_every_path() {
        // rm-041: keys are JSON object keys, so a non-UTF-8 path must
        // survive the round trip without collapsing to `U+FFFD` —
        // while UTF-8 paths keep their verbatim key (nothing changes
        // for the 99.999% case) and hand-written junk fails safe.
        let verbatim = [
            "/home/u/.claude/sessions/2026/ok.jsonl",
            "/tmp/emoji-\u{1F600}-name.jsonl",
            "/tmp/percent-%-and-%25-name.jsonl",
            "/tmp/spaces in name.jsonl",
            // Valid multi-byte UTF-8 stays verbatim too — only non-
            // UTF-8 byte sequences need the encoded form.
            "/tmp/\u{00E4}-umlaut.jsonl",
        ];
        for path in verbatim {
            let path = PathBuf::from(path);
            let key = cache_key(&path);
            assert!(
                !key.starts_with(LOSSLESS_KEY_PREFIX),
                "UTF-8 key stays verbatim: {key}"
            );
            assert_eq!(path_from_key(&key), path, "verbatim key decodes to itself");
        }
        #[cfg(unix)]
        {
            use std::os::unix::ffi::OsStringExt;
            let raw = [
                b"/tmp/bad\xff\xfename.jsonl".to_vec(),
                b"/tmp/pct-%-in-\xff-name.jsonl".to_vec(),
                b"/tmp/lone-\x80-start.jsonl".to_vec(),
            ];
            for bytes in raw {
                let path = PathBuf::from(std::ffi::OsString::from_vec(bytes.clone()));
                let key = cache_key(&path);
                assert!(
                    key.starts_with(LOSSLESS_KEY_PREFIX),
                    "non-UTF-8 key is encoded: {key}"
                );
                assert!(
                    key.is_ascii(),
                    "encoded keys are plain JSON-safe ASCII: {key}"
                );
                assert_eq!(
                    path_from_key(&key),
                    path,
                    "lossless key round-trips every byte"
                );
            }
            // Fail-safe: a prefixed key with a malformed escape is
            // returned verbatim rather than guessed at.
            assert_eq!(
                path_from_key("at-bytes:%zz-broken"),
                PathBuf::from("at-bytes:%zz-broken")
            );
        }
        // Keys without the prefix always decode to themselves.
        assert_eq!(
            path_from_key("/plain/path.jsonl"),
            PathBuf::from("/plain/path.jsonl")
        );
    }

    #[test]
    fn schema_invalidation_ledger_is_bounded_and_exposed() {
        // Store-integrity lead: the disclosure ledger is capped at
        // MAX_SCHEMA_INVALIDATION_RECORDS (a journal that outlives
        // many upgrades cannot grow it without limit) and survives
        // saves so the disclosure stays durable until `--clear-cache`.
        let root = std::env::temp_dir().join(format!(
            "agenttrace-ledger-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).expect("create temp dir");
        let _env = crate::test_env::lock_env();
        let prior_cache = std::env::var_os("AGENTTRACE_SESSION_CACHE_DIR");
        std::env::set_var("AGENTTRACE_SESSION_CACHE_DIR", &root);

        // A journal already carrying a full ledger (eight records),
        // then one more schema flip: the oldest record drops, the new
        // one lands, and the cap holds.
        let mut doc = serde_json::Map::new();
        doc.insert(
            "schema_version".to_string(),
            serde_json::json!(SESSION_CACHE_SCHEMA_VERSION - 1),
        );
        doc.insert(
            "pricing_catalog_id".to_string(),
            serde_json::json!(crate::pricing::catalog_identity()),
        );
        doc.insert(
            "dir_listing_version".to_string(),
            serde_json::json!(DIR_LISTING_WALK_VERSION),
        );
        doc.insert(
            "schema_invalidations".to_string(),
            serde_json::json!((0..MAX_SCHEMA_INVALIDATION_RECORDS)
                .map(|i| {
                    serde_json::json!({"observed_schema_version": i, "schema_version": i + 1})
                })
                .collect::<Vec<_>>()),
        );
        fs::write(
            session_cache_path(),
            serde_json::to_string(&Value::Object(doc)).expect("serialize doc"),
        )
        .expect("write doc");

        let mut cache = load_session_cache();
        let exposed = cache.schema_invalidations();
        assert_eq!(
            exposed.len(),
            MAX_SCHEMA_INVALIDATION_RECORDS,
            "the ledger stays capped after appending the new record"
        );
        let observed: Vec<i64> = exposed
            .iter()
            .filter_map(|record| {
                record
                    .get("observed_schema_version")
                    .and_then(Value::as_i64)
            })
            .collect();
        // The hand-written 0th record was dropped; the flip's own
        // record (observed = SESSION_CACHE_SCHEMA_VERSION - 1) is last.
        assert!(
            !observed.contains(&0),
            "the oldest record drops past the cap"
        );
        assert_eq!(
            *observed.last().expect("ledger non-empty"),
            SESSION_CACHE_SCHEMA_VERSION - 1,
            "the newest flip is the newest record"
        );

        // The ledger persists through a save and reloads intact.
        save_session_cache(&mut cache).expect("save store");
        let raw = fs::read_to_string(session_cache_path()).expect("read store");
        let doc: Value = serde_json::from_str(&raw).expect("store parses");
        let persisted = doc
            .get("schema_invalidations")
            .and_then(Value::as_array)
            .expect("ledger persisted");
        assert_eq!(persisted.len(), MAX_SCHEMA_INVALIDATION_RECORDS);
        let cache = load_session_cache();
        assert_eq!(
            cache.schema_invalidations().len(),
            MAX_SCHEMA_INVALIDATION_RECORDS,
            "a matching-schema load carries the ledger forward untouched"
        );

        match prior_cache {
            Some(value) => std::env::set_var("AGENTTRACE_SESSION_CACHE_DIR", value),
            None => std::env::remove_var("AGENTTRACE_SESSION_CACHE_DIR"),
        }
        drop(_env);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn pricing_catalog_mismatch_is_disclosed_in_the_ledger() {
        // Review fix R3 (2026-10-06, run 2d92ee95 independent_review
        // 90bc04b9): the pricing arm re-parses every entry when the
        // catalog identity changes but used to record NOTHING — the
        // invalidation was invisible, violating the lead's acceptance
        // contract ("a schema or pricing-catalog mismatch invalidates
        // entries WITH DISCLOSURE"). The disclosure must land in the
        // SAME bounded ledger the schema arm writes, naming the reason
        // and the observed catalog id.
        let root = std::env::temp_dir().join(format!(
            "agenttrace-pricing-flip-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).expect("create temp dir");
        let corpus = root.join("corpus");
        fs::create_dir_all(&corpus).expect("create corpus dir");
        let journal = corpus.join("session.jsonl");
        fs::write(&journal, b"{}").expect("write journal");
        let _env = crate::test_env::lock_env();
        let prior_cache = std::env::var_os("AGENTTRACE_SESSION_CACHE_DIR");
        std::env::set_var("AGENTTRACE_SESSION_CACHE_DIR", &root);

        // Populate a store at the CURRENT schema and CURRENT catalog.
        let mut cache = load_session_cache();
        let session = Session {
            name: "price".to_string(),
            path: journal.to_string_lossy().to_string(),
            cwd: String::new(),
            branch: String::new(),
            metrics: Metrics::default(),
            anomalies: Vec::new(),
            health: 100,
            tool_warnings: Vec::new(),
            diagnostics: Diagnostics::default(),
        };
        store_session(&journal, &session, &mut cache).expect("store session");
        store_dir_listing(&corpus, std::slice::from_ref(&journal), &[], &mut cache)
            .expect("store listing");
        save_session_cache(&mut cache).expect("save store");
        assert!(cache.schema_invalidations().is_empty());

        // Rewrite the persisted pricing_catalog_id to a foreign value,
        // leaving schema + walk lane untouched: exactly the reviewer's
        // PoC B.
        let raw = fs::read_to_string(session_cache_path()).expect("read store");
        let mut foreign =
            serde_json::from_str::<serde_json::Value>(&raw).expect("store parses as JSON");
        foreign["pricing_catalog_id"] = serde_json::json!("bogus-catalog-identity-0000");
        fs::write(
            session_cache_path(),
            serde_json::to_string(&foreign).expect("serialize foreign store"),
        )
        .expect("write foreign store");

        // Load under the real catalog: every entry is dropped for
        // re-parse, AND the ledger gains the disclosure record.
        let mut reloaded = load_session_cache();
        let journal_metadata = fs::metadata(&journal).expect("journal metadata");
        assert!(
            cached_file_mod_time_if_fresh(&journal, &journal_metadata, &mut reloaded).is_none(),
            "entries re-parse at the new catalog"
        );
        let records = reloaded.schema_invalidations().to_vec();
        assert_eq!(records.len(), 1, "the pricing arm appends a ledger record");
        let record = &records[0];
        assert_eq!(
            record.get("reason").and_then(serde_json::Value::as_str),
            Some("pricing_catalog"),
            "the record names the pricing arm"
        );
        assert_eq!(
            record
                .get("observed_pricing_catalog_id")
                .and_then(serde_json::Value::as_str),
            Some("bogus-catalog-identity-0000"),
            "the record names the observed (foreign) catalog id"
        );
        assert_eq!(
            record
                .get("dropped_entries")
                .and_then(serde_json::Value::as_i64),
            Some(1),
            "the record counts the re-parsed entries"
        );
        assert_eq!(
            record
                .get("kept_dir_listings")
                .and_then(serde_json::Value::as_i64),
            Some(1),
            "walk-current listings survive the pricing flip"
        );

        // The record persists: saving re-stamps the real catalog id and
        // the next load must NOT double-append.
        save_session_cache(&mut reloaded).expect("save reloaded store");
        let mut again = load_session_cache();
        assert_eq!(
            again.schema_invalidations().len(),
            1,
            "no duplicate record on reload"
        );
        save_session_cache(&mut again).expect("save again store");
        let raw = fs::read_to_string(session_cache_path()).expect("read final store");
        let final_doc: serde_json::Value = serde_json::from_str(&raw).expect("final store parses");
        assert_ne!(
            final_doc
                .get("pricing_catalog_id")
                .and_then(serde_json::Value::as_str),
            Some("bogus-catalog-identity-0000"),
            "the real catalog id is re-stamped"
        );

        let _ = fs::remove_dir_all(&root);
        if let Some(prior) = prior_cache {
            std::env::set_var("AGENTTRACE_SESSION_CACHE_DIR", prior);
        } else {
            std::env::remove_var("AGENTTRACE_SESSION_CACHE_DIR");
        }
    }

    #[test]
    fn schema_flip_restores_dir_listings_and_records_the_invalidation() {
        // Session-cache store integrity (lead): a schema bump must
        // invalidate parsed entries — every bump in the schema history
        // changed parse or accounting semantics, so stale numbers must
        // never be served — but it must not silently torch the whole
        // store. Directory listings are walk semantics on their own
        // version lane, so when that lane still matches they survive
        // the bump (no full tree re-walk on upgrade), and the
        // invalidation itself is recorded in the journal instead of
        // happening invisibly.
        let root = std::env::temp_dir().join(format!(
            "agenttrace-schema-flip-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).expect("create temp dir");
        let corpus = root.join("corpus");
        fs::create_dir_all(&corpus).expect("create corpus dir");
        let journal = corpus.join("session.jsonl");
        fs::write(&journal, b"{}").expect("write journal");
        let _env = crate::test_env::lock_env();
        let prior_cache = std::env::var_os("AGENTTRACE_SESSION_CACHE_DIR");
        std::env::set_var("AGENTTRACE_SESSION_CACHE_DIR", &root);

        // Populate a store at the CURRENT schema: one entry + one
        // live listing for a real directory.
        let mut cache = load_session_cache();
        let session = Session {
            name: "flip".to_string(),
            path: journal.to_string_lossy().to_string(),
            cwd: String::new(),
            branch: String::new(),
            metrics: Metrics::default(),
            anomalies: Vec::new(),
            health: 100,
            tool_warnings: Vec::new(),
            diagnostics: Diagnostics::default(),
        };
        store_session(&journal, &session, &mut cache).expect("store session");
        store_dir_listing(&corpus, std::slice::from_ref(&journal), &[], &mut cache)
            .expect("store listing");
        save_session_cache(&mut cache).expect("save store");

        // Flip the persisted schema to simulate an upgrade between
        // invocations; every other field of the document stays as
        // written.
        let raw = fs::read_to_string(session_cache_path()).expect("read store");
        let mut flipped: serde_json::Value =
            serde_json::from_str(&raw).expect("store parses as JSON");
        flipped["schema_version"] = serde_json::json!(SESSION_CACHE_SCHEMA_VERSION - 1);
        fs::write(
            session_cache_path(),
            serde_json::to_string(&flipped).expect("serialize flipped store"),
        )
        .expect("write flipped store");

        let mut cache = load_session_cache();
        let journal_metadata = fs::metadata(&journal).expect("journal metadata");
        assert!(
            cached_file_mod_time_if_fresh(&journal, &journal_metadata, &mut cache).is_none(),
            "stale-schema entries are never served"
        );
        let listing = cached_dir_listing(&corpus, &mut cache)
            .expect("walk-version listings survive the schema bump");
        assert_eq!(listing.files, vec![journal.clone()]);
        assert!(
            cache.is_dirty(),
            "the invalidation must persist at the next save"
        );

        // The post-flip save lands at the new schema with the listing
        // preserved, the entries dropped, and the disclosure persisted.
        save_session_cache(&mut cache).expect("save migrated store");
        let raw = fs::read_to_string(session_cache_path()).expect("read migrated store");
        let doc: serde_json::Value =
            serde_json::from_str(&raw).expect("migrated store parses as JSON");
        assert_eq!(
            doc["schema_version"],
            serde_json::json!(SESSION_CACHE_SCHEMA_VERSION)
        );
        assert_eq!(
            doc["entries"].as_object().map(|entries| entries.len()),
            Some(0),
            "stale-schema entries stay dropped"
        );
        assert_eq!(
            doc["dirs"].as_object().map(|dirs| dirs.len()),
            Some(1),
            "the restored listing is part of the migrated store"
        );
        let records = doc["schema_invalidations"]
            .as_array()
            .expect("the invalidation is disclosed in the journal");
        assert_eq!(records.len(), 1);
        assert_eq!(
            records[0]["observed_schema_version"],
            serde_json::json!(SESSION_CACHE_SCHEMA_VERSION - 1)
        );
        assert_eq!(
            records[0]["schema_version"],
            serde_json::json!(SESSION_CACHE_SCHEMA_VERSION)
        );
        assert_eq!(records[0]["dropped_entries"], serde_json::json!(1));
        assert_eq!(records[0]["dropped_dirs"], serde_json::json!(0));
        assert_eq!(records[0]["kept_dir_listings"], serde_json::json!(1));

        // The migrated store still serves the listing on the next run.
        let mut cache = load_session_cache();
        assert!(
            cached_dir_listing(&corpus, &mut cache).is_some(),
            "the restored listing still serves after the save"
        );

        match prior_cache {
            Some(value) => std::env::set_var("AGENTTRACE_SESSION_CACHE_DIR", value),
            None => std::env::remove_var("AGENTTRACE_SESSION_CACHE_DIR"),
        }
        drop(_env);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    #[cfg(unix)]
    fn non_utf8_paths_round_trip_through_the_cache_store() {
        // rm-041: cache keys and listing members used to collapse
        // non-UTF-8 paths to `U+FFFD`, so a `bad\xff\xfename.jsonl`
        // journal was invisible through the cached replay — the
        // listing named a file that does not exist, and two distinct
        // non-UTF-8 names could collide on one lossy key. Keys are now
        // lossless: the listing member must equal the real path.
        use std::os::unix::ffi::OsStringExt;
        let root = std::env::temp_dir().join(format!(
            "agenttrace-lossless-keys-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).expect("create temp dir");
        let corpus = root.join("corpus");
        fs::create_dir_all(&corpus).expect("create corpus dir");
        let journal = corpus.join(std::ffi::OsString::from_vec(
            b"bad\xff\xfename.jsonl".to_vec(),
        ));
        fs::write(&journal, b"{}").expect("write journal");
        let _env = crate::test_env::lock_env();
        let prior_cache = std::env::var_os("AGENTTRACE_SESSION_CACHE_DIR");
        std::env::set_var("AGENTTRACE_SESSION_CACHE_DIR", &root);

        let mut cache = load_session_cache();
        store_dir_listing(&corpus, std::slice::from_ref(&journal), &[], &mut cache)
            .expect("store listing");
        save_session_cache(&mut cache).expect("save store");

        let mut cache = load_session_cache();
        let listing =
            cached_dir_listing(&corpus, &mut cache).expect("listing serves for a UTF-8 directory");
        assert_eq!(
            listing.files,
            vec![journal.clone()],
            "the non-UTF-8 file name must round-trip losslessly"
        );

        // The entry round trip must keep working through the encoded
        // key: store, persist, reload, serve.
        let session = Session {
            name: "raw name".to_string(),
            path: journal.to_string_lossy().to_string(),
            cwd: String::new(),
            branch: String::new(),
            // A non-empty source tool keeps the empty-source-tool
            // eviction gate (below) from treating this entry as
            // pre-gate legacy junk once it reloads into raw_entries.
            metrics: Metrics {
                source_tool: "agenttrace-test".to_string(),
                ..Metrics::default()
            },
            anomalies: Vec::new(),
            health: 100,
            tool_warnings: Vec::new(),
            diagnostics: Diagnostics::default(),
        };
        store_session(&journal, &session, &mut cache).expect("store session");
        save_session_cache(&mut cache).expect("save store");
        let mut cache = load_session_cache();
        let served = cached_session(&journal, &mut cache)
            .expect("the cached entry serves through the lossless key");
        assert_eq!(served.name, "raw name");

        match prior_cache {
            Some(value) => std::env::set_var("AGENTTRACE_SESSION_CACHE_DIR", value),
            None => std::env::remove_var("AGENTTRACE_SESSION_CACHE_DIR"),
        }
        drop(_env);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    #[cfg(unix)]
    fn legacy_lossy_listing_members_are_dropped_once_at_load() {
        // rm-041 upgrade seam: a pre-v4 store carries listing members
        // built with `to_string_lossy`, so a non-UTF-8 journal is stored
        // as a verbatim U+FFFD name — and the replay extends
        // `listing.files` verbatim, which would keep hiding the real
        // file through the warm cache forever. A lossy member cannot
        // be told apart from a genuine U+FFFD filename, so the walk
        // lane (not a filter) is what drops them: the listing below
        // has a CURRENT mtime and no size to drift, so only the
        // walk-version gate can invalidate it.
        use std::os::unix::ffi::OsStringExt;
        let root = std::env::temp_dir().join(format!(
            "agenttrace-legacy-lossy-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).expect("create temp dir");
        let corpus = root.join("corpus");
        fs::create_dir_all(&corpus).expect("create corpus dir");
        let journal = corpus.join(std::ffi::OsString::from_vec(
            b"bad\xff\xfename.jsonl".to_vec(),
        ));
        fs::write(&journal, b"{}").expect("write journal");
        let _env = crate::test_env::lock_env();
        let prior_cache = std::env::var_os("AGENTTRACE_SESSION_CACHE_DIR");
        std::env::set_var("AGENTTRACE_SESSION_CACHE_DIR", &root);

        // Seed the store exactly as a pre-upgrade binary would have:
        // older schema, the last lossy-member walk lane, lossy member
        // name. The lane value is the ABSOLUTE legacy 3, not
        // `DIR_LISTING_WALK_VERSION - 1`: the defect this pins is a
        // binary whose current lane still equals the lossy lane — a
        // const-relative seed would self-adjust and never see it.
        let legacy_lossy_lane: i64 = 3;
        let corpus_metadata = fs::metadata(&corpus).expect("corpus metadata");
        let lossy_member = format!("{}/bad\u{FFFD}name.jsonl", corpus.to_string_lossy());
        let legacy = serde_json::json!({
            "schema_version": SESSION_CACHE_SCHEMA_VERSION - 1,
            "dir_listing_version": legacy_lossy_lane,
            "dirs": {
                corpus.to_string_lossy().to_string(): {
                    "mod_time": file_mod_time_nanos(&corpus_metadata),
                    "files": [lossy_member],
                    "dirs": [],
                },
            },
        });
        fs::write(
            session_cache_path(),
            serde_json::to_string(&legacy).expect("serialize legacy store"),
        )
        .expect("write legacy store");

        let mut cache = load_session_cache();
        assert!(
            cached_dir_listing(&corpus, &mut cache).is_none(),
            "a lossy-member listing from the previous walk lane must not serve"
        );
        // The drop is disclosed like every schema-side invalidation.
        let record = cache
            .schema_invalidations()
            .last()
            .expect("invalidation recorded");
        assert_eq!(record["kept_dir_listings"], serde_json::json!(0));
        assert_eq!(record["dropped_dirs"], serde_json::json!(1));

        // Self-heal forward: the re-walk stores the listing with the
        // lossless member, and the store serves the real path after a
        // save/reload.
        store_dir_listing(&corpus, std::slice::from_ref(&journal), &[], &mut cache)
            .expect("store fresh listing");
        save_session_cache(&mut cache).expect("save store");
        let mut cache = load_session_cache();
        let listing = cached_dir_listing(&corpus, &mut cache).expect("fresh listing serves");
        assert_eq!(listing.files, vec![journal.clone()]);

        match prior_cache {
            Some(value) => std::env::set_var("AGENTTRACE_SESSION_CACHE_DIR", value),
            None => std::env::remove_var("AGENTTRACE_SESSION_CACHE_DIR"),
        }
        drop(_env);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn directory_listing_freshness_also_compares_the_directory_size() {
        // rm-041: listing freshness compared the directory mtime
        // alone, so a mutation that landed inside the same mtime tick
        // as the stored listing stayed invisible until the next mtime
        // change. The stored listing also carries the directory's byte
        // size; a drift in either invalidates.
        let root = std::env::temp_dir().join(format!(
            "agenttrace-same-tick-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).expect("create temp dir");
        let corpus = root.join("corpus");
        fs::create_dir_all(&corpus).expect("create corpus dir");
        let journal = corpus.join("session.jsonl");
        fs::write(&journal, b"{}").expect("write journal");
        let _env = crate::test_env::lock_env();
        let prior_cache = std::env::var_os("AGENTTRACE_SESSION_CACHE_DIR");
        std::env::set_var("AGENTTRACE_SESSION_CACHE_DIR", &root);

        let rewrite_listing = |mod_time: i64, dir_size: serde_json::Value| {
            let mut cache = load_session_cache();
            store_dir_listing(&corpus, std::slice::from_ref(&journal), &[], &mut cache)
                .expect("store listing");
            save_session_cache(&mut cache).expect("save store");
            let raw = fs::read_to_string(session_cache_path()).expect("read store");
            let mut doc: serde_json::Value =
                serde_json::from_str(&raw).expect("store parses as JSON");
            let key = corpus.to_string_lossy().to_string();
            doc["dirs"][key.as_str()]["mod_time"] = serde_json::json!(mod_time);
            doc["dirs"][key.as_str()]["dir_size"] = dir_size;
            fs::write(
                session_cache_path(),
                serde_json::to_string(&doc).expect("serialize store"),
            )
            .expect("write store");
        };
        let current = fs::metadata(&corpus).expect("corpus metadata");
        let now_nanos = file_mod_time_nanos(&current);
        let real_size = current.len();

        // Same-tick drift: the mtime still matches, the size does not.
        rewrite_listing(now_nanos, serde_json::json!(real_size + 4096));
        let mut cache = load_session_cache();
        assert!(
            cached_dir_listing(&corpus, &mut cache).is_none(),
            "a same-tick size drift must invalidate the listing"
        );

        // No false invalidation: mtime and size both current.
        rewrite_listing(now_nanos, serde_json::json!(real_size));
        let mut cache = load_session_cache();
        assert!(
            cached_dir_listing(&corpus, &mut cache).is_some(),
            "a listing whose mtime and size both match still serves"
        );

        match prior_cache {
            Some(value) => std::env::set_var("AGENTTRACE_SESSION_CACHE_DIR", value),
            None => std::env::remove_var("AGENTTRACE_SESSION_CACHE_DIR"),
        }
        drop(_env);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn concurrent_saves_merge_instead_of_last_writer_wins() {
        // rm-298 residual: save was a lockless read-modify-write — two
        // agenttrace processes that load the store before either saves
        // each overwrite the other's entries wholesale. Saves now
        // serialize on an advisory lock and merge the freshly re-read
        // on-disk state, so every writer's entry survives.
        use std::sync::{Arc, Barrier};
        let root =
            std::env::temp_dir().join(format!("agenttrace-merge-save-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        let corpus = root.join("corpus");
        fs::create_dir_all(&corpus).expect("create corpus dir");
        let _env = crate::test_env::lock_env();
        let prior_cache = std::env::var_os("AGENTTRACE_SESSION_CACHE_DIR");
        std::env::set_var("AGENTTRACE_SESSION_CACHE_DIR", &root);

        // Seed the on-disk store with one entry.
        let seed_file = corpus.join("seed.jsonl");
        fs::write(&seed_file, b"{}").expect("write seed");
        let mut cache = load_session_cache();
        let session = Session {
            name: "seed".to_string(),
            path: seed_file.to_string_lossy().to_string(),
            cwd: String::new(),
            branch: String::new(),
            metrics: Metrics {
                source_tool: "agenttrace-test".to_string(),
                ..Metrics::default()
            },
            anomalies: Vec::new(),
            health: 100,
            tool_warnings: Vec::new(),
            diagnostics: Diagnostics::default(),
        };
        store_session(&seed_file, &session, &mut cache).expect("store seed");
        save_session_cache(&mut cache).expect("save seed");

        // Every writer loads BEFORE any of them saves (the barrier),
        // exactly like processes that start together.
        let writers: Vec<_> = (0..6)
            .map(|i| {
                let file = corpus.join(format!("writer-{i}.jsonl"));
                fs::write(&file, b"{}").expect("write journal");
                (i, file)
            })
            .collect();
        let barrier = Arc::new(Barrier::new(writers.len()));
        let mut handles = Vec::new();
        for (i, file) in writers.iter() {
            let i = *i;
            let barrier = Arc::clone(&barrier);
            let file = file.clone();
            handles.push(std::thread::spawn(move || {
                let mut cache = load_session_cache();
                barrier.wait();
                let session = Session {
                    name: format!("writer-{i}"),
                    path: file.to_string_lossy().to_string(),
                    cwd: String::new(),
                    branch: String::new(),
                    metrics: Metrics {
                        source_tool: "agenttrace-test".to_string(),
                        ..Metrics::default()
                    },
                    anomalies: Vec::new(),
                    health: 100,
                    tool_warnings: Vec::new(),
                    diagnostics: Diagnostics::default(),
                };
                store_session(&file, &session, &mut cache).expect("store session");
                save_session_cache(&mut cache).expect("save cache");
            }));
        }
        for handle in handles {
            handle.join().expect("writer thread");
        }

        let mut cache = load_session_cache();
        let served = cached_session(&seed_file, &mut cache).expect("seed survives");
        assert_eq!(served.name, "seed");
        for (i, file) in &writers {
            let served = cached_session(file, &mut cache).unwrap_or_else(|| {
                panic!("writer-{i}'s entry was lost to a concurrent save");
            });
            assert_eq!(served.name, format!("writer-{i}"));
        }

        match prior_cache {
            Some(value) => std::env::set_var("AGENTTRACE_SESSION_CACHE_DIR", value),
            None => std::env::remove_var("AGENTTRACE_SESSION_CACHE_DIR"),
        }
        drop(_env);
        let _ = fs::remove_dir_all(root);
    }
}
