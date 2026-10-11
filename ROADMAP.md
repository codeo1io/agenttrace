# agenttrace — Roadmap

> Autonomously maintained by the roadmap sync (reliability-first). Items cite reproducible codebase signals; acceptance is proven by cited evidence.

**Vision**: A reliable, customer-friendly repository advanced by evidence-cited roadmap cycles owned by the autonomy loop

**Pillars**: reliability work outranks customer-experience work; every roadmap item cites reproducible codebase signals; acceptance is proven by cited evidence, never claimed

## Fleet context

- dependents (changes here affect): (host), agent, maestro
- graph: evidence-derived (imports/refs/deploy surfaces); advisory

## Open items

### Deterministic project-dir decode (host-state-independent attribution)
- id: `rm-381` | track: reliability | priority: 96.0 | status: in_progress
- acceptance: decode no longer depends on unrelated host directories — attribution for a fixed transcript corpus is byte-identical across hosts (verified with a decoy-directory fixture); ambiguous candidates are resolved by a documented deterministic rule (longest verified path, cwd-first when present) and unresolved/ambiguous dirs are disclosed in --doctor; the fresh PoC triple (decoy rc0 / true rc1 / decoy-removed rc0) is pinned as a regression fixture
- evidence: campaign-recorded in agenttrace ROADMAP.md

### Copilot session-wide credit accounting — totalNanoAiu and usage_checkpoint events are invisible
- id: `rm-485` | track: reliability | priority: 92.0 | status: in_progress
- acceptance: shutdown arm additionally reads data.totalNanoAiu as the session-wide credit total; session.usage_checkpoint parsed as cost-only usage snapshots (session-wide totalNanoAiu plus per-model modelMetrics where present); credits converted at 1 AIU = 1 AI credit = $0.01 as cost-only rows attributed to the session (model `unknown` where per-model data is absent) with gap reconciliation so a later shutdown's attribution never double-counts; sessions from pre-checkpoint Copilot versions excluded loudly; parse diagnostics disclose counted credit totals vs modelMetrics-attributed totals; golden fixtures pin all three shapes (resumed-empty-metrics, open-checkpoint-only, healthy control)
- evidence: campaign-recorded in agenttrace ROADMAP.md

### Hermetic project-decode and grouping tests (no host-ancestor state)
- id: `rm-382` | track: reliability | priority: 91.0 | status: in_progress
- acceptance: the test (and every decode/grouping test) creates fixtures under a fully controlled namespace (unique root, no host ancestor interference) so `cargo test --workspace` is green regardless of TMPDIR/host layout; a regression test plants a shadow dir at a plausible ancestor and asserts attribution is unaffected
- evidence: campaign-recorded in agenttrace ROADMAP.md

### Restore the formatting gate at master
- id: `rm-173` | track: reliability | priority: 90.0 | status: in_progress
- acceptance: cargo fmt --check exits 0 at HEAD after re-formatting exactly the 8 cited sites; a rust-toolchain.toml (or equivalent CI-side rustfmt pin) stabilizes the formatting toolchain so the gate cannot drift with stable bumps; next master CI run is green through the fmt step
- evidence: campaign-recorded in agenttrace ROADMAP.md

### Fork release-channel identity decision
- id: `rm-195` | track: reliability | priority: 90.0 | status: candidate
- acceptance: a decision lands — EITHER all four channels re-scoped to fork-owned identities (npm scope + tarball name derived from `npm pack --json` per #272, own homebrew tap, own winget PackageIdentifier, plugin identity) with check-release-surfaces.sh pinning each, OR the upstream-bound lanes are explicitly disabled with a documented manual path; no lane can silently publish under another owner's identity
- evidence: campaign-recorded in agenttrace ROADMAP.md

### Join flat-transcript tool results to their calls
- id: `rm-230` | track: reliability | priority: 90.0 | status: in_progress
- acceptance: flat-transcript parsing preserves the tool_use_id-to-tool_call_id pairing so a present result never counts as unmatched; a regression fixture with 3 parallel same-name tool_use blocks plus 3 present results reports unmatched 0 with correct per-tool counts; governance no longer escalates on that corpus
- evidence: campaign-recorded in agenttrace ROADMAP.md

### Close the AgentMeasure token-accounting exposures (re-emission, id-less dedup, 1h cache pricing)
- id: `rm-251` | track: reliability | priority: 90.0 | status: candidate
- acceptance: (1) each of the 13 parser formats carries a DOCUMENTED re-emission contract (delta-vs-cumulative vs id-dedup vs latest-wins — one stated rule per format) with a fixture per format family (claude, codex, kimi at minimum); (2) id-less identical usage lines dedup (content-hash or timestamp-window fallback) with the rule stated; (3) ephemeral_1h tokens priced at 2× input or loudly disclosed as 5m-assumed wherever cache pricing renders; (4) the kimi sum-vs-latest decision settled against a REAL kimi corpus before the behavior is called a bug (the audit's own real-data rule), outcome recorded either way; (5) AgentMeasure conformance/pack fixtures run offline as the rm-053 CI suite
- evidence: campaign-recorded in agenttrace ROADMAP.md

### Kimi usage records silently zero — the alias table misses the wire keys
- id: `rm-400` | track: reliability | priority: 90.0 | status: in_progress
- acceptance: usage_from_value accepts the kimi aliases (input <- input_other; output <- output behind a numeric-only guard so a bare string "output" field in another format cannot over-match; cache_read <- input_cache_read; cache_creation <- input_cache_creation) with the alias table documented per format; the official fixture lands as a golden test asserting the summed truth (input_other 62,198 + output 4,790 + cache_read 496,640 = 563,628) against today's 132 estimator tokens; StatusUpdate usage events preserve arrival order (no insert(0)); a diagnostics counter discloses alias-matched fields per session; kimi sessions stop falling back to estimator pricing (fallback_pricing 0 on the fixture)
- evidence: campaign-recorded in agenttrace ROADMAP.md

### First-class compaction episode + usage accounting across parsers
- id: `rm-421` | track: reliability | priority: 90.0 | status: candidate
- acceptance: a CompactionEpisode model per source (omp compaction/branch_summary markers, claude isCompactSummary/leafUuid chain, codex compacted snapshots); compaction tokens attributed to a DISTINCT usage class, never folded into input/output; compacted turns marked so they stop counting as assistant turns; episode count + compaction token share surfaced in diagnostics/overview; hostile fixtures per source pin turn counts, usage classes, and episode counts (the omp PoC fixture already exists at /tmp/at-assess-16bbd/fx/omp/s.jsonl)
- evidence: campaign-recorded in agenttrace ROADMAP.md

### Sanitize terminal-control bytes in CLI text renderers (OSC/CSI injection)
- id: `rm-383` | track: reliability | priority: 89.0 | status: in_progress
- acceptance: every CLI text renderer routes strings through the shared control-character sanitizer (contract per memory: control bytes → U+FFFD, printable CSI tails may legitimately survive); regression fixtures assert no raw ESC/OSC bytes in text outputs while the sanitizer-output assertion targets control bytes; statusline parity kept (rm-034 done)
- evidence: campaign-recorded in agenttrace ROADMAP.md

### Port upstream v0.9.0 onto the fork and stand up a fork-vs-upstream delta report
- id: `rm-012` | track: reliability | priority: 88.0 | status: in_progress
- acceptance: (1) PR #286 (Codex cost double-count fix, loop_fingerprints model, ATTENTION_* triage thresholds) and #284 (Oh My Pi leading-non-session-lines parser fix) content present on the rebased fork branch with session_cache.rs conflicts resolved preserving the v7 fix's semantics; (2) a scripts/upstream-delta report (git log --oneline fork-base..upstream/master -- per-file) regenerates on demand into the spool; (3) full suite green at the new head
- evidence: campaign-recorded in agenttrace ROADMAP.md

### Port upstream #295 WinGet manifest validation fix
- id: `rm-174` | track: reliability | priority: 88.0 | status: in_progress
- acceptance: render-channels.sh output matches upstream #295's validated shape (no PortableCommandAlias lines, yaml-language-server $schema header on all three manifests); diff vs upstream's render script at 52ab2cd is limited to fork-specific channel names; the rm-156 lane's rendered fixtures are re-validated
- evidence: campaign-recorded in agenttrace ROADMAP.md

### Byte-true session-cache bound and a first dirs bound
- id: `rm-298` | track: reliability | priority: 88.0 | status: in_progress
- acceptance: the bound is enforced over exactly the bytes the writer emits (values + path keys + structural JSON + dirs) or the writer truncates to the budgeted bytes, so a fresh full-corpus cache write measures ≤ cap on disk, byte-exact; dirs gains its own count and byte bound; eviction order is deterministic and pinned by a test that builds an over-capacity fixture and asserts the WRITTEN FILE's size ≤ cap (not the estimator's model of it); the real-corpus run on this host lands ≤ 64 MiB
- evidence: campaign-recorded in agenttrace ROADMAP.md

### Route tool p95 through the house percentile — the nearest-rank copy in diagnostics disarms the slow-tool gates
- id: `rm-420` | track: reliability | priority: 88.0 | status: in_progress
- acceptance: diagnostics.rs tool p95 (and the inline trunc+clamp copy at :300-303 p95_gap) route through crate::percentile; the pin test's include set extended to scan diagnostics.rs so any future inlined percentile copy fails CI; golden fixture 19x2s+1x31s corpus asserts p95_sec=31.0 and is_slow=true; overview and diagnostics p95 agree on the same session; fold the f22ff7d1 delta at integration if it is still unlanded
- evidence: campaign-recorded in agenttrace ROADMAP.md

### pi UsageEntry accounting: count type:"usage" journal spend (cache_warm et al.)
- id: `rm-436` | track: reliability | priority: 88.0 | status: in_progress
- acceptance: pi type:"usage" entries parse into meta events attributed to the entry's OWN provider/model fields (wire carries kind/provider/model/usage), with input/output/cacheRead/cacheWrite from entry.usage AND entry.usage.cost honored when present (upstream-recorded cost beats catalog estimate — composes with the pricing_source() disclosure family); a per-kind disclosure counter (e.g. pi_usage_entry:cache_warm) surfaces in --doctor/data_health; golden fixtures pin cache_warm and an unknown kind; PoC re-run shows poc-usage.jsonl at spec-true totals (50,150 tokens incl. $0.015 cacheRead) instead of 150/$0.0
- evidence: campaign-recorded in agenttrace ROADMAP.md

### Workbuddy usage-basis clamp zeroes input silently; the basis is pinned by no vendor fixture
- id: `rm-497` | track: reliability | priority: 88.0 | status: in_progress
- acceptance: a basis mismatch (input < cache_read) surfaces as a per-session diagnostics counter (pattern: kimi_usage_alias from rm-400, codex compaction counters from rm-401 — e.g. workbuddy_usage_basis_clamped) instead of silently-zero input; a red-first golden fixture pins the today-silent PoC (input 50 / cache_read 5000 -> tokens_input 0) with the counter firing; a vendor-basis fixture pair documents the netting contract in-tree (control 175/40 -> net 135); usage on reasoning/function_call_result lines is read or disclosed-dropped
- evidence: campaign-recorded in agenttrace ROADMAP.md

### Copilot usage reconciliation is snapshot-level, not per-model — rotation and partial shutdowns silently drop whole models
- id: `rm-551` | track: reliability | priority: 88.0 | status: in_progress
- acceptance: per-model freshest-by-timestamp fold — a checkpoint REPLACES the per-model values only for the models it names and PRESERVES values for models it omits; a partial shutdown contributes only its delta per model with any unexplained remainder surfaced as cost-only unknown attribution (never dropping a named model's tokens); regression fixtures pin BOTH PoC shapes (rotation preserves gpt-4.1 1,000,000/100,000 alongside claude-sonnet-4 50,000/5,000; partial-shutdown preserves o4-mini 400,000/40,000 alongside gpt-4.1) plus the truncated-session.start misattribution shape (either honest generic-source labeling with a disclosure counter, or a documented copilot-detection fallback); by_model attribution includes every modelMetrics key ever observed; rm-485's credit max-semantics and provenance arms unchanged; credit_usd fixtures from the assess PoCs stay green (0.02 / 0.0095)
- evidence: campaign-recorded in agenttrace ROADMAP.md

### Codex output_tokens double-adds reasoning_output_tokens — upstream #312 removed the add
- id: `rm-617` | track: reliability | priority: 88.0 | status: in_progress
- acceptance: codex output excludes the reasoning double-add per upstream #312 semantics (port the removal, keep the rm-162 saturating discipline on the remaining sums); reasoning tokens stay visible wherever reports surface them (separate counter or disclosed inclusion — never silently dropped); golden codex fixture red-first on a reasoning-carrying rollout; full suite green
- evidence: campaign-recorded in agenttrace ROADMAP.md

### pi journal: compaction/branch_summary usage is silently dropped (handled-but-partial accounting)
- id: `rm-939` | track: reliability | priority: 88.0 | status: in_progress
- acceptance: compaction/branch_summary usage folded into session metrics (tokens in/out, cache read/write, upstream cost when reported) with a named disclosure counter (e.g. pi_compaction_usage_counted) surfaced in metrics disclosures; a sentinel fixture (compaction usage 38000 in / 900 out / $0.0123) pinned green end-to-end; PREVENTION: a pi writer-surface contract test enumerating all 13 dist session-manager event types (session/label/text/usage/thinking_level_change/session_info/model_change/message/custom_message/custom/context_edit/compaction/branch_summary) asserting each is either fully accounted or pi_entry_skipped-disclosed — per-field silent drops fail CI (research C1 folded here)
- evidence: campaign-recorded in agenttrace ROADMAP.md

### Saturate token accumulation (i64 overflow panics debug builds, silently corrupts release)
- id: `rm-046` | track: reliability | priority: 87.0 | status: in_progress
- acceptance: every token-map merge site (add_usage, add_usage_value, add_opencode_tokens) uses saturating or checked arithmetic; an adversarial fixture with two i64::MAX inputs parses without panic in debug builds and reports clamped totals with an explicit saturation marker in cost provenance; release cost math on the same fixture stays bounded and flagged; regression test token_accumulation_saturates_not_wraps pins the fixture
- evidence: campaign-recorded in agenttrace ROADMAP.md

### Generalize terminal-control sanitization to every text report sink
- id: `rm-239` | track: reliability | priority: 87.0 | status: in_progress
- acceptance: every string printed by ANY default text-mode lane passes the same control-byte sanitization as statusline's sanitize_line_segment via a shared helper (not a per-site copy): governance reports (render_plain_value), overview text (timeline + model rows), and any other content-derived text field; a regression fixture carrying OSC-52 and CSI payloads in session title, model name, and MCP server name asserts ZERO raw control bytes in every text lane while the json/md/html lanes remain byte-identical to today (they are already safe — serde_json and the html escaper neutralize them); TUI stays immune (ratatui buffer filters control chars — verified at the vendored source)
- evidence: campaign-recorded in agenttrace ROADMAP.md

### Port the upstream v0.10.0 wave onto the fork (self-update, i18n, calendar aggregation, subagent attribution)
- id: `rm-448` | track: reliability | priority: 87.0 | status: candidate
- acceptance: fork-vs-upstream delta report refreshed past v0.10.0 (the rm-012 mechanism); every wave component dispositioned adopt/port/skip with a recorded reason; #305 subagent-cost attribution reconciled with the fork's sidechain/isSidechain lane by TITLE (renumber at integration); update.rs adoption records a security review against the rm-051 install-receipt discipline (checksum/signature posture before any self-update ships); MSRV decision (stay 1.80 vs adopt 1.85) recorded with toolchain evidence; full suite + gates green on the merged tree
- evidence: campaign-recorded in agenttrace ROADMAP.md

### qwen_usage sums input-synonym keys and never subtracts cache_read — upstream #312 first_number + clamp
- id: `rm-618` | track: reliability | priority: 87.0 | status: in_progress
- acceptance: one synonym source per field per upstream #312's first_number semantics (or an equivalent single-source rule stated in the format's contract); input = prompt count minus cache_read with each cache count clamped to the remaining input (upstream #316 subtract_cached_input discipline — pairs with rm-450's watch); golden qwen fixture red-first at input 100; full suite green
- evidence: campaign-recorded in agenttrace ROADMAP.md

### Discover pi-fork and pi-profile session homes (~/.senpi, ~/.omo, ~/.pi/<profile>)
- id: `rm-084` | track: reliability | priority: 86.0 | status: in_progress
- acceptance: discovery enumerates pi-family homes across forks (senpi/omo class) and profile variants under each home with the same dedup/uniqueness rules as ~/.pi; default --overview on a host carrying such corpora reports them, or the opt-in escape (-d / env) is documented as the supported path; the source label matches the actual home (pi_source_for_path extended or replaced); -d <DIR> gets non-blank --help text; golden tests pin a multi-home, multi-profile fixture corpus
- evidence: campaign-recorded in agenttrace ROADMAP.md

### Cross-session usage dedupe and fork identity
- id: `rm-232` | track: reliability | priority: 86.0 | status: candidate
- acceptance: message identity equals (session_id, message uuid/request id, timestamp) across files; corpus aggregates (overview, insights, context_trends, pricing totals) dedupe before summing; a deduped_messages truthfulness counter surfaces the correction in diagnostics; dedupe scoping keeps daily/weekly/monthly windows in agreement
- evidence: campaign-recorded in agenttrace ROADMAP.md

### First-class configuration file (~/.config/agenttrace)
- id: `rm-384` | track: reliability | priority: 86.0 | status: in_progress
- acceptance: agenttrace reads ~/.config/agenttrace/config.toml plus a project-local override plus an explicit --config flag, with a documented precedence chain (CLI > --config > project > user > env defaults); --doctor discloses the active config path(s) and resolved key values; a config-schema test pins parsing/round-trip and precedence order
- evidence: campaign-recorded in agenttrace ROADMAP.md

### Codex compaction usage records silently dropped
- id: `rm-401` | track: reliability | priority: 86.0 | status: in_progress
- acceptance: token_usage_record events parse into usage with response_id matched to the preceding compacted marker's compaction_response_id; usage on a copied/compacted response is counted exactly once with the dedup decision disclosed in diagnostics; the compaction turn does not advance the high-water baseline (composing with, not regressing, the landed rm-035 within-file rewind fix); a red-first golden fixture reproducing the PoC (1,910 reported vs 2,610 ccusage-semantics, -26.8%) pins the delta; the unparsed-event diagnostics counter moves on the fixture (the class this defect evaded)
- evidence: campaign-recorded in agenttrace ROADMAP.md

### Saturate the by_task_type accumulator (rm-046 fixed the parser side; the reports-side sums were left bare)
- id: `rm-541` | track: reliability | priority: 86.0 | status: in_progress
- acceptance: both accumulations use saturating_add with a comment citing the governance.rs:988-1001 same-class precedent; red-first regression test in crates/agenttrace-core/tests/attribution_dimensions.rs — two sessions each pinned at i64::MAX output inside ONE task-type bucket assert (a) no debug panic, (b) release-mode bucket equals i64::MAX (not 0) while summary totals stay saturated-consistent; the analyze() clamp comment at lib.rs ~:733 gains a by_task_type sentence so the "can never wrap" claim covers every accumulator
- evidence: campaign-recorded in agenttrace ROADMAP.md

### Codex fast path drops torn-tail lines with no disclosure — rm-526's promise has a bypass
- id: `rm-709` | track: reliability | priority: 86.0 | status: in_progress
- acceptance: (a) fast-path skip increments the torn-tail counter for BOTH paths — no silent `continue`; (b) fast≡slow oracle: codex-torn PoC corpus reports identical tokens/cost/anomalies via fast and slow paths (2700-class truth restored or the torn line disclosed, never silently dropped); (c) sweep test re-verifies rm-526's 'every format parser discloses torn-tail lines' claim across all parsers including the fast path
- evidence: campaign-recorded in agenttrace ROADMAP.md

### Re-verify cycle-1 assess findings against post-v0.9.0 upstream before implement spend
- id: `rm-013` | track: reliability | priority: 85.0 | status: in_progress
- acceptance: every open assess finding (AF-1..AF-3 plus any cross-campaign batch item touching ported files) has a written disposition after the rm-012 rebase — re-anchored with new line evidence, dropped as upstream-fixed with the fixing PR cited, or kept verbatim — recorded in a verification table before any implement phase spends budget on those items
- evidence: campaign-recorded in agenttrace ROADMAP.md

### pi tree journals: branch-aware accounting for v2/v3 id/parentId sessions
- id: `rm-437` | track: reliability | priority: 85.0 | status: in_progress
- acceptance: pi v2/v3 sessions get branch-aware semantics — turns/health/timeline replay the ACTIVE branch (leaf reachable from the last user turn per spec navigation) while spend reports ALL branches with a disclosed branch count (pi_branches:N in --doctor/data_health); minimum viable first cut: all-branches spend + branch-count disclosure, active-branch replay as the second cut; golden fixtures pin sibling-branch shape (two assistants sharing a parentId) and forked-session shape; PoC re-run discloses the shape instead of silently reporting 410 tokens / 2 turns for a one-active-branch session
- evidence: campaign-recorded in agenttrace ROADMAP.md

### Parse Codex custom-tools response items and disclose unmatched wire shapes
- id: `rm-542` | track: reliability | priority: 85.0 | status: in_progress
- acceptance: (1) custom_tool_call / custom_tool_call_output parse as tool call/result with latency pairing and failure state from payload.status (name, namespace, and the JS-source input feed tool_usage like function_call does); (2) standalone response_item reasoning feeds reasoning_blocks/reasoning_chars; (3) world_state gets an explicit ignorable-line counter; (4) any still-unknown payload type increments a `codex_unmatched_response_item:<type>` line_skips/doctor disclosure (rm-401 widened-channel precedent) so future wire growth is visible, not silent; (5) contract test on a real-shape fixture captured from a 2026-09-26 rollout asserting tool_calls_total > 0, tool_usage populated, reasoning counts nonzero, assistant turn count matches the message census
- evidence: campaign-recorded in agenttrace ROADMAP.md

### Land the OTel export validity + GenAI semconv currency set with the `-f otel` CLI half
- id: `rm-599` | track: reliability | priority: 85.0 | status: in_progress
- acceptance: 1) span ids derived from the per-session hash (not the ordinal), all-zero rejected, golden asserts non-zero + uniqueness across an N-session export; 2) non-finite f64 attrs omitted (or string-escaped) with a visible disclosure line — never a null value variant; 3) the three granular cache/reasoning Recommended attrs emitted when the Metrics carry them; 4) the inclusive-vs-exclusive input basis recorded in code + docs and pinned by a golden (either add cache back for the attr or document the divergence — decision explicit, not accidental); 5) control-byte class composes with the rm-539 sanitization lane when it lands (shared helper or stated equivalence); 6) rider: write_stdout gains the trailing-newline parity `-o` files have (main.rs:502-504 vs :683-688) so text lanes stop gluing onto the next shell line; 7) lands WITH the queued rm-389 + rm-212 + `-f otel` CLI batch as ONE change-unit so no release ships spec-invalid exports; conductor validation digest validation:v1:<sha> recorded in the shipping PR
- evidence: campaign-recorded in agenttrace ROADMAP.md

### Compose terminal-control and invisible-character sanitization at every output choke point
- id: `rm-933` | track: reliability | priority: 85.0 | status: in_progress
- acceptance: ONE shared predicate — sanitize_line_segment's contract extended from Cc to the Cf spoofing set (tags/bidi/zero-width → U+FFFD on render lanes; JSON lanes escape \u-sequences instead of emitting raw) — composed at EVERY output choke point: the overview text residual sites (cost-driver rows, timeline, model/provider/task_type labels — rm-239's ceded scope), the markdown and html arms (rm-239's md/html-safe premise is falsified live), the overview_csv group cells (making csv_export's own doc contract true), and the three TUI sites; a cross-format invariant test replays a kitchen-sink corpus (OSC-52 base64 payloads + U+E0000-tagged ASCII + U+202E bidi + zero-widths) through every lane asserting ZERO raw ESC and ZERO invisible-Cf characters in text/markdown/html/csv outputs and escaped-only JSON; the already-sanitized sessions and statusline lanes get the Cf case pinned so the extended predicate cannot regress
- evidence: campaign-recorded in agenttrace ROADMAP.md

### Token-accounting conformance harness in CI
- id: `rm-053` | track: reliability | priority: 84.0 | status: in_progress
- acceptance: a conformance job runs a checked-in fixture pack derived from the five audited classes against parser+pricing and fails on any silent double-count, mis-price, or unprovable total; at minimum the overflow/clamp and re-emitted-event classes are represented by fixtures at landing; the job gates PRs touching parser.rs or pricing.rs
- evidence: campaign-recorded in agenttrace ROADMAP.md

### Loop detection must not flag parallel same-name calls as retries
- id: `rm-233` | track: reliability | priority: 84.0 | status: in_progress
- acceptance: loop detection keys on (tool name, argument identity) so at least 3 distinct-argument parallel calls inside one assistant turn are not a retry loop; arguments that differ only in key order count as identical (review F7); the parallel-batch corpus reports no loop block; waste loop_cost excludes those groups
- evidence: campaign-recorded in agenttrace ROADMAP.md

### Claude Code streaming usage re-emissions — fold to the per-message max, not the exact-snapshot dedupe
- id: `rm-601` | track: reliability | priority: 84.0 | status: in_progress
- acceptance: per message.id the parser keeps the MAX snapshot per token class (never the sum across growing re-emissions; exact duplicates stay deduped as today); id-less messages keep a pinned behavior either way (fixture); regression fixtures: growing-output stream (red today at 3.0×), identical-duplicate guard, id-less stream; golden benign transcripts unchanged; composes with rm-251's cross-file key — a forked file re-emitting streamed messages folds once per file AND once per message
- evidence: campaign-recorded in agenttrace ROADMAP.md

### rm-890
- id: `rm-890` | track: reliability | priority: 84.0 | status: in_progress
- acceptance: one shared admission predicate (last-activity basis) used by BOTH the sqlite lanes and the file-lane retain; overnight-sqlite fixture test (created-before-window, updated-inside) pinning `--range 7d` inclusion for the hermes and opencode lanes (zero tests reference session_within_since today — the fixture closes that); README.md:192-196 sentence stands unchanged with behavior now matching it; clippy --workspace --all-targets --locked -D warnings + cargo fmt --all --check rc0.
- evidence: campaign-recorded in agenttrace ROADMAP.md

### Eliminate the TUI dual-renderer test/production binding
- id: `rm-014` | track: reliability | priority: 82.0 | status: candidate
- acceptance: one helper implementation compiled identically under test and production (either unify shared.rs/presentation.rs or migrate tests off the legacy renderer and delete it), with a parity test asserting identical outputs for the duplicated helper set; no allow(dead_code) remains on the duplicated symbols
- evidence: campaign-recorded in agenttrace ROADMAP.md

### Honor --lang in every report renderer
- id: `rm-085` | track: reliability | priority: 82.0 | status: candidate
- acceptance: --lang is effective for every user-facing action (overview in all four formats, sessions, audit, recommend, mcp-governance, context-trends, delivery-evidence) or an unsupported action exits with an explicit flag-not-supported error; golden tests pin non-English output for each renderer family; --help documents the flag's coverage
- evidence: campaign-recorded in agenttrace ROADMAP.md

### Per-cost pricing provenance object (upstream #103 promoted task)
- id: `rm-175` | track: reliability | priority: 82.0 | status: candidate
- acceptance: session/report JSON gains additive cost_provenance with match status (exact|alias|normalized|default), source class (provider_reported|pricing_lookup|unknown), and confidence; provider metadata survives model normalization; existing fields byte-compatible; golden tests pin one fixture per match class and one provider-reported session
- evidence: campaign-recorded in agenttrace ROADMAP.md

### Cross-root canonical-identity dedup: aliasing roots must not double-count sessions
- id: `rm-338` | track: reliability | priority: 82.0 | status: in_progress
- acceptance: discovery dedups by canonical file identity (fs::canonicalize with fallback to the listed path when canonicalize errors) across ALL registered roots; a root aliasing or nested inside another registered root is deduped or rejected with the decision disclosed; --doctor discloses the alias/overlap decisions alongside its existing symlink label; a red-first regression fixture (two roots aliasing one canonical dir, symlink AND nested-overlap shapes) proves each session counted exactly once; live totals on a non-aliasing host are byte-identical before/after the fix
- evidence: campaign-recorded in agenttrace ROADMAP.md

### Doctor must persist its parses and bound its corpus walk (uncached, silent, unbounded today)
- id: `rm-367` | track: reliability | priority: 82.0 | status: in_progress
- acceptance: staged, each independently shippable: (a) doctor's walk routes through the caching finder and every successful parse persists (store_session + save_session_cache), so an immediate rerun on the same -d corpus reports reusable entries > 0 and strictly lower wall time than the cold run; (b) a per-root wall-clock budget with honest partial-scan disclosure ('scanned N of M roots in Ts') keeps doctor's output valid and rc 0 under truncation instead of a silent multi-minute stall; (c) progress output on stderr before the first parse so a hung run is diagnosable; (d) roots over a stated size get a count-sampled or size-capped pass for the parseable/failed columns (doctor needs counts, not attribution). Regression: a fixture pins that a budget-interrupted run still reports honest partial counts
- evidence: campaign-recorded in agenttrace ROADMAP.md

### Port upstream #304: newer Claude Code transcripts carrying both session_id and sessionId misclassify as Qwen Code
- id: `rm-449` | track: reliability | priority: 82.0 | status: in_progress
- acceptance: is_qwen_code_event returns false when sessionId is present regardless of session_id; a fixture transcript carrying both keys classifies as claude_code with tokens/cost attributed there, and the pre-fix binary demonstrably misclassified it (red-first); no regression on snake_case-only qwen corpora; targeted + full suites green
- evidence: campaign-recorded in agenttrace ROADMAP.md

### CSV statement hostile-cell hardening — overview group keys bypass the sanitizer lane (rm-409 residual)
- id: `rm-540` | track: reliability | priority: 82.0 | status: in_progress
- acceptance: every transcript-derived cell in the csv statement routes through sanitize_line_segment before csv_cell, mirroring the sessions-row wiring; guard_formula extended to the first non-whitespace character (or the documented OWASP trigger set including tab/CR/LF prefixes); hostile-corpus regression fixtures pin the OSC-52, tab-prefixed HYPERLINK, and leading-space payloads (corpora pattern preserved from /tmp/at-assess-871cae94/poc/{over,m2}); RFC4180 quoting retained — CRLF row-forgery containment proven live must stay; deterministic-output csv arm and the README column contract unchanged; composer discipline: merge by title with the uncommitted 278b2bda/6e96bed5 TSV/CSV-cell-injection lane per rm-409's own acceptance routing (that lane owns the TSV arm; this row owns the LANDED csv_export.rs defects)
- evidence: campaign-recorded in agenttrace ROADMAP.md

### Port the workbuddy usage arithmetic set (upstream #311 sum, #316 clamp, reasoning-row usage)
- id: `rm-600` | track: reliability | priority: 82.0 | status: in_progress
- acceptance: 1) multi-record usage summed with saturating adds (port #311's shape), pinned by a wb1-shaped golden (660, not 330); 2) cache counts clamped to input (#316 subtract_cached_input semantics; reused for Copilot as upstream does), pinned by a wb7-shaped golden (input+cache ≤ source input; effective basis stated); 3) reasoning-row usage read (or skipped loudly via the rm-450 disclosure channel — silence is the bug); 4) SESSION_CACHE_SCHEMA_VERSION bumped so warm caches regenerate (totals change; stale warm caches would mask the fix — rm-230 convention); 5) rm-450's workbuddy_input_basis disclosure counters re-verified truthful under the new arithmetic; 6) migration note: totals for existing workbuddy sessions rise toward truth — call it out in the next CHANGELOG entry; conductor validation digest validation:v1:<sha> recorded in the shipping PR
- evidence: campaign-recorded in agenttrace ROADMAP.md

### Bound the drain joins on run_bounded's fast-exit arm (and governance's wait-after-exit)
- id: `rm-731` | track: reliability | priority: 82.0 | status: in_progress
- acceptance: (1) red-first PATH-shim fixture — git whose first invocation exits rc0 immediately but leaves `sleep 25 &` holding stdout/stderr — must return inside the rev-parse probe's UPSTREAM_GIT_TIMEOUT=10s bound (upstream.rs:344) or fail with the named deadline error; current live behavior 25,106 ms rc0 (control with real git 81 ms); (2) the same shape pinned for governance.rs's post-exit joins; (3) the mechanism is the one rm-583 parked — dedicated process group via pre_exec setsid with a group kill, or dropping the joins as the timeout arm does — extended to the happy path; (4) upstream.rs:40's doc comment restated to the post-rm-583 contract
- evidence: campaign-recorded in agenttrace ROADMAP.md

### Report-contract truthfulness batch (search truncation disclosure, rates.total, unknown-reason labels)
- id: `rm-011` | track: reliability | priority: 81.0 | status: candidate
- acceptance: search JSON carries total_matches and truncated fields and either applies --sort/--order or exits with an explicit unsupported-flag error; rates_per_million_usd.total is computed from its components or the field is removed across all cost-audit serializations; inspect_reason fallback labels unknown reasons as unknown in every locale
- evidence: campaign-recorded in agenttrace ROADMAP.md

### Parse JSONL once across format probes
- id: `rm-036` | track: reliability | priority: 80.0 | status: candidate
- acceptance: format detection parses the source text once and shares the parsed objects across all probes; unknown-format files show a single parse pass; existing parser golden behavior unchanged
- evidence: campaign-recorded in agenttrace ROADMAP.md

### Disclose local at-rest artifacts in PRIVACY.md and align --clear-cache
- id: `rm-086` | track: reliability | priority: 80.0 | status: in_progress
- acceptance: PRIVACY.md enumerates every artifact the tool writes under ~/.cache/agenttrace (sessions.json, hermes-sqlite.json, opencode-sqlite.json, statusline.jsonl journal, pricing cache) with the data class each carries and the purge command; --clear-cache removes or explicitly reports every listed artifact including the statusline journal; a registry test keeps the PRIVACY list in sync with the code's cache-path construction so new stores cannot ship undisclosed
- evidence: campaign-recorded in agenttrace ROADMAP.md

### Pricing tier and price-class support (priority, above_1hr cache creation, batch, above_200k)
- id: `rm-164` | track: reliability | priority: 80.0 | status: candidate
- acceptance: pricing structures accept per-tier rates (priority/flex, above_200k, cache_creation above_1hr) when the catalog provides them; cost reports and statusline rate math select the tier actually used by the session; golden tests cover a priority-tier fixture and an above-1hr cache fixture
- evidence: campaign-recorded in agenttrace ROADMAP.md

### install.sh must refuse on missing .sha256 sidecar
- id: `rm-234` | track: reliability | priority: 80.0 | status: in_progress
- acceptance: install.sh treats a missing or malformed sidecar as fatal exactly like the other two channels; the three-installer parity test (crates/agenttrace-core/src/lib.rs, all_three_installers_refuse_when_the_sidecar_cannot_be_verified -- cited by name, line anchors rot) asserts refusal semantics for all three channels, not just sidecar naming; a missing-sidecar run of each channel shows all three refusing; annotation (round-2 review R2-5): install.sh refusal is pinned BEHAVIORALLY (scripts/ci/check-install-runtime.sh test G missing sidecar, test J mismatched sidecar), install.ps1/npm refusal is pinned textually in the parity test (their refusal is structural: IWR throws / download() rejects non-200), and behavioral probes for those two channels await a pwsh/node lane
- evidence: campaign-recorded in agenttrace ROADMAP.md

### Codex MultiAgent V2 replayed parent-prefix double-counts across files
- id: `rm-402` | track: reliability | priority: 80.0 | status: candidate
- acceptance: a child rollout whose session id / replay markers tie it to a parent rollout in the same corpus inherits the parent's final snapshot as its baseline (replayed-prefix tokens counted once, attributed to the parent); the inheritance and its attribution are disclosed in diagnostics; a red-first two-file golden fixture pins the class (PoC: parent 1,400 + child-from-zero 1,700 = 3,100 reported vs 1,700 true, +82%); single-file corpora are byte-identical before/after
- evidence: campaign-recorded in agenttrace ROADMAP.md

### Deduplicate symlinked/hardlinked session files at the collection boundary
- id: `rm-597` | track: reliability | priority: 80.0 | status: in_progress
- acceptance: a corpus with real.jsonl + link.jsonl symlink + a directory-level symlink control reports exactly one session with single-counted cost/tokens across --overview/--sessions/--search/--doctor; a hardlink alias gets the same treatment; a symlink that is the ONLY path to a transcript still counts exactly once; directory-level symlink descent semantics unchanged
- evidence: campaign-recorded in agenttrace ROADMAP.md

### Qwen usage basis — cache-inclusive promptTokenCount and alias-summed input
- id: `rm-602` | track: reliability | priority: 80.0 | status: in_progress
- acceptance: qwen_usage reads the FIRST present alias per class (never sums aliases); when input came from an inclusive-basis alias (promptTokenCount/prompt_tokens), the cache-inclusive span is handled one way, pinned by fixtures (net input 400 + cache 600 for the PoC, or the documented inclusive equivalent); a both-aliases-present fixture pins single-count input; the CU-20 reasoning/thoughtsTokenCount output folding stays unchanged; golden qwen corpora re-derived and pinned
- evidence: campaign-recorded in agenttrace ROADMAP.md

### Revisited / multi-emitted sessions undercount usage (codex revisit, qwen multi-emission)
- id: `rm-711` | track: reliability | priority: 80.0 | status: in_progress
- acceptance: (a) usage accumulates across re-emissions/revisits (occurrence-aware totals, not last-wins); (b) both PoC corpora report truth tokens (2700 / 45) with anomalies disclosing multi-emission if ambiguity remains; (c) oracle tests pin revisit + multi-emission fixtures
- evidence: campaign-recorded in agenttrace ROADMAP.md

### Warm-replay must re-validate file kind — a stale dir listing re-admits a fifo and wedges the load
- id: `rm-732` | track: reliability | priority: 80.0 | status: in_progress
- acceptance: (1) red-first planted-listing pin — a stored dir listing whose files array contains a fifo entry under an otherwise-valid warm cache must be filtered with the run exiting rc0; current live behavior rc=124 under an external `timeout 15` (the replayed fifo passes admit, then the parallel parse worker's read blocks forever, wedging every --sessions/--dir/TUI load and doctor via its cached-walk delegation, with no message and no timeout); (2) DIR_LISTING_WALK_VERSION bumped 3→4 (one-time cold re-walk) AND/OR the replay arm re-stats entries with the same is_file() re-check — both is safest; (3) the CHANGELOG rm-212 bullet scoped to the cold path or made true again; (4) composes without overlap with rm-041's freshness half (same mtime-only trust in cached_dir_listing — that row keeps lossy keys + freshness, this row owns the replay file-kind gate + the walk-version contract)
- evidence: campaign-recorded in agenttrace ROADMAP.md

### rm-911
- id: `rm-911` | track: reliability | priority: 80.0 | status: in_progress
- acceptance: the card's daily series is computed from cache-SURVIVING data — either serialize timestamps into the session cache (SESSION_CACHE_SCHEMA_VERSION bump + stale-cache invalidation + warm fixture re-stamped in the same unit, rm-230 convention; re-verify the live origin/master schema ceiling at every gate — it moves) or bucket per-session start/end dates with an explicit disclosure when intra-session day splits are lost; a golden cold-vs-warm test renders the SAME corpus twice (fresh cache, then warm) and asserts byte-identical SVG; the warm path is covered for the markdown/html/csv lanes too or proven timestamps-free; the live PoC corpus is preserved as a fixture
- evidence: campaign-recorded in agenttrace ROADMAP.md

### Refactor 3 high-complexity function(s)
- id: `rm-001` | track: reliability | priority: 79.0 | status: candidate
- signals: reliability.complexity_hot:npm/scripts/install.js::L24, reliability.complexity_hot:npm/scripts/install.js::L28, reliability.complexity_hot:npm/scripts/install.js::L40
- acceptance: Each flagged function is decomposed below the branch threshold with behavior locked by characterization tests
- evidence: ast-based branch-count check passes at HEAD (full suite green; conductor validation digest validation:v1:<sha> recorded in the shipping PR)

### Catch up statusline schema: spend_limit window + structured repo identity
- id: `rm-037` | track: reliability | priority: 78.0 | status: candidate
- acceptance: spend_limit parsed and rendered alongside the two existing windows and recorded in journal insights when present; workspace.repo identity captured into the journal record and used by insights attribution when available; payload model tolerates unknown future fields without error
- evidence: campaign-recorded in agenttrace ROADMAP.md

### Absolute-time report scoping (--since/--until)
- id: `rm-087` | track: reliability | priority: 78.0 | status: candidate
- acceptance: --since/--until (plus a --last N[d|w] shorthand if cheap) are accepted by overview/sessions/waste/compare/governance actions in every output format; boundary semantics documented (inclusive start, exclusive end, or stated otherwise) and pinned by fixtures across the boundary; the timezone rule is stated once and reused; scoping composes with rm-085's i18n threading so scoped reports render in the selected language
- evidence: campaign-recorded in agenttrace ROADMAP.md

### Ship a share-safe report mode
- id: `rm-158` | track: reliability | priority: 78.0 | status: candidate
- acceptance: a --share-safe flag (or equivalent mode) renders every output format (json/markdown/html/console) with deterministic redaction — paths hashed or basename-only, model/provider names optionally masked, no message/tool excerpts; PRIVACY.md's manual-review caveat is replaced by the mode's guarantee; golden tests pin redaction across all formats
- evidence: campaign-recorded in agenttrace ROADMAP.md

### Timezone-truthful rate-limit resets and daily buckets
- id: `rm-165` | track: reliability | priority: 78.0 | status: candidate
- acceptance: statusline renders reset times in the user's local timezone with an explicit offset label; insights daily buckets honor a configurable timezone; tests cross a UTC-midnight boundary in a non-UTC locale
- evidence: campaign-recorded in agenttrace ROADMAP.md

### Pin and verify the source-build fallback
- id: `rm-235` | track: reliability | priority: 78.0 | status: in_progress
- acceptance: the fallback builds a pinned verifiable ref (release tag with its commit SHA recorded and checked), never a floating master tip; the fallback path is exercised once in CI or a recorded manual run; docs state exactly what the fallback installs
- evidence: campaign-recorded in agenttrace ROADMAP.md

### Warm-path status snapshot keyed by stat-only corpus fingerprints
- id: `rm-299` | track: reliability | priority: 78.0 | status: candidate
- acceptance: a persisted snapshot keyed by a stat-only fingerprint (dev/ino/mtime/size per corpus file) makes repeat invocation on an unchanged corpus complete in single-digit seconds for the default report surface; any corpus mutation (or fingerprint mismatch) forces correct re-derivation, pinned by a mutation-correctness gate (touch one file → affected rows re-derived; restore → cache hits again); the snapshot is offline-only (no network) and covered by cache_artifact_paths() and PRIVACY.md's artifact table from day one (no rm-202-class orphan)
- evidence: campaign-recorded in agenttrace ROADMAP.md

### Waste forensics: attribute every cache rebuild and price the rebuild premium
- id: `rm-339` | track: reliability | priority: 78.0 | status: candidate
- acceptance: the waste report attributes each cache rebuild to a named reason and exposes a rebuild-premium cost (write-cost minus read-cost of the re-sent prefix) aggregated by reason/agent/session/model; hit_rate above 100% is clamped or explicitly disclosed rather than graded "excellent"; a synthetic-fixture matrix mirroring cachemiss's classes pins every attribution, red-first where the current code mislabels
- evidence: campaign-recorded in agenttrace ROADMAP.md

### Weekly budget targets and window-burn surface
- id: `rm-385` | track: reliability | priority: 78.0 | status: in_progress
- acceptance: a user-configurable weekly (7-day) budget knob (riding rm-384 config), a window-burn view (per-day/per-window spend vs budget) in --overview or a dedicated report, and a --statusline-report extension rendering remaining budget from already-parsed seven_day telemetry; negative guard recorded: no live-watch mode (ccusage deliberately removed live monitoring — negative signal)
- evidence: campaign-recorded in agenttrace ROADMAP.md

### Claude Code subagent transcript rollup — link children to their parent session
- id: `rm-487` | track: reliability | priority: 78.0 | status: candidate
- acceptance: discovery links <session>/subagents/agent-*.jsonl (and the sibling layout variant) to the parent post-load; child sessions carry parent_session (id + path); parents expose subagent_count/subagent_cost/subagent_tokens as separate rollups that never fold into the parent's own usage; fleet totals count each transcript exactly once (golden test on a parent+child+forked-copy corpus asserting no double count — composes with the rm-232 dedupe fix); overview/TUI surface the rollups
- evidence: campaign-recorded in agenttrace ROADMAP.md

### lru advisory ride and the deny gate's unsound-class blindness
- id: `rm-498` | track: reliability | priority: 78.0 | status: in_progress
- acceptance: Cargo.lock pins lru >= 0.18.2 (target 0.18.5, zero API change); deny.toml [advisories] gains an explicit unsound scope decision (`unsound = "transitive"` or `"all"`) so the Dependency policy job surfaces this class by policy, not by luck; gate proof both ways recorded in the shipping PR (repo config FAILS the class only after the scope line — and goes green at the new lock with the widened scope); cargo test --locked green
- evidence: campaign-recorded in agenttrace ROADMAP.md

### Clamp cache counts to cache-inclusive input (port upstream open PR #316)
- id: `rm-529` | track: reliability | priority: 78.0 | status: in_progress
- acceptance: a shared subtract_cached_input-style helper clamps each cache count to the remaining cache-inclusive input and the WorkBuddy arm adopts it (dropping the inline subtraction); red-first fixture (input 100 / cache_read 150) yields uncached input + clamped cache summing to the source's cache-inclusive input (100, not 150), pinned by a port of upstream's rust_workbuddy_clamps_cache_read_above_input_without_inflating_total; codex contract parity pinned (delta cache read stays unclamped; existing workbuddy_usage_survives_negative_input_with_cache_read stays green); the schema-bump decision folds into the fork's ladder with one-shot journal regeneration per the rm-230 convention and a disclosed re-baseline IF any corpus total changes (none expected per the 3928-record negative); full test battery green
- evidence: campaign-recorded in agenttrace ROADMAP.md

### Route the counts cells and disclosure keys through the control-byte sanitizer (rm-528 residual — the last unsanitized cell family)
- id: `rm-595` | track: reliability | priority: 78.0 | status: in_progress
- acceptance: every render site emitting a counts cell or disclosure key routes journal-derived bytes through the shared sanitizer — text (:633/:639), markdown (:668/:674 incl. markdown_cell), html (:696/:702 incl. html_escape), doctor (:588-599), diagnostics; red-first hostile corpus asserting 0 raw ESC/OSC sequences across ALL surfaces (od-exact, both terminal stdout and -o files) while the counters still identify the shape; the landed rm-528 "every CLI text renderer routes strings through the shared control-character sanitizer" contract becomes literally true at these call sites; composes with rm-594's key cap (a truncated key must not smuggle a partial escape sequence past the sanitizer — sanitize after/beyond the truncate point, or make the sanitize length-safe)
- evidence: campaign-recorded in agenttrace ROADMAP.md

### Codex token_count output already includes reasoning — stop adding it on top
- id: `rm-603` | track: reliability | priority: 78.0 | status: in_progress
- acceptance: codex token_count reasoning reports as its own reasoning_tokens line and is NEVER added to output_tokens; the existing i64-scale saturation pins (rm-162 family) stay green; regression fixture codex-reasoning.jsonl pins output 400 + reasoning 150 reported separately; the token_usage_record lane (rm-401 corpora) asserted unchanged on its own pins; if the two lanes' bases differ, the difference is stated where the report surfaces them
- evidence: campaign-recorded in agenttrace ROADMAP.md

### Session cache drops LineSkips — warm re-runs hide disclosure and flip confidence low→high
- id: `rm-710` | track: reliability | priority: 78.0 | status: in_progress
- acceptance: (a) cache entries carry line_skips (serde-default field, NO schema-version bump — #17005-era precedent) or skip-disclosure is recomputed on hit; (b) warm re-run reproduces the cold run's line_skips and confidence byte-for-byte on the PoC corpus; (c) regression test pins warm==cold disclosure oracle
- evidence: campaign-recorded in agenttrace ROADMAP.md

### Same-second SQLite siblings collapse into one durable history record (session_id fold collision)
- id: `rm-790` | track: reliability | priority: 78.0 | status: in_progress
- acceptance: (1) sqlite-lane history identity gains a per-row discriminator from the source DB (opencode session.id / hermes session uuid) inside the FNV preimage, or an equivalent uniqueness guarantee within (db, second); (2) the planted PoC (/tmp/at-assess-cd02/poc-history-collision — two same-second opencode sessions in one DB) yields TWO history records that both survive a re-ingest; (3) the live-host shape (6 same-second buckets covering 13 sessions in ~/.local/share/opencode/opencode.db, `select time_created/1000 s, count(*) c from session group by 1 having c>1`) ingests 13 sessions, not 6; (4) the LF5 tuple-fold disclosure class (history.rs:158-174) keeps its distinct disclosed status — no regression there
- evidence: campaign-recorded in agenttrace ROADMAP.md

### Drop write-only overview recompute from the refresh_filtered hot path
- id: `rm-015` | track: reliability | priority: 76.0 | status: candidate
- acceptance: refresh_filtered performs no OverviewDerived/self.overview computation per keystroke (removed, or computed once per filter-commit / lazily on first read); a test or benchmark demonstrates the per-keystroke work drops from O(visible sessions) clone to O(matching) filter; visible behavior unchanged
- evidence: campaign-recorded in agenttrace ROADMAP.md

### Add transcript-derived 5-hour billing block analytics
- id: `rm-042` | track: reliability | priority: 76.0 | status: candidate
- acceptance: a blocks command/report groups transcript usage into 5-hour windows aligned to first use, reports per-block cost and token totals by model, and marks the active block with its current burn rate; works with no statusline configured
- evidence: campaign-recorded in agenttrace ROADMAP.md

### Automated pricing snapshot freshness (models.dev + LiteLLM)
- id: `rm-176` | track: reliability | priority: 76.0 | status: in_progress
- acceptance: a scheduled snapshot-bump workflow refreshes the bundled snapshot on cadence (weekly or models.dev-change-triggered) and records snapshot-date + model-count deltas in the PR body; models.dev added as a second source with documented precedence; doctor surfaces snapshot age; catalog growth stays additive (no silent price removals)
- evidence: campaign-recorded in agenttrace ROADMAP.md

### Claude Desktop 3p usage-ledger home + Cowork cloud-coverage disclosure
- id: `rm-537` | track: reliability | priority: 76.0 | status: candidate
- acceptance: (a) discovery+parser: a structural home for the 3p usage-ledger (ndjson; records carry surface cowork|code, model ids, token usage, recorded cost) with surface→source_tool mapping and recorded-cost precedence (rm-436's recorded-cost rule family); (b) dedup guard: a session root containing BOTH the ledger and Desktop-nested .claude/projects/**/*.jsonl transcripts must count each API call once (reconcile by model + token equality + ±30s inclusive timestamp window, ledger authoritative, suppress exactly one matching transcript call — pinned by a red-first double-count fixture); (c) disclosure: docs/data-health note that Cowork Pro/Max sessions created after 2026-10-06 are cloud-side and invisible to local corpora (truthfulness family, rm-450 lineage); route variants (us/global) are separate billing SKUs
- evidence: campaign-recorded in agenttrace ROADMAP.md

### Port the cache-inclusive-input clamp and reconcile the session-cache schema ladder
- id: `rm-547` | track: reliability | priority: 76.0 | status: candidate
- acceptance: red-first fixture input=100/cache_read=150 asserts the session total is exactly 100 (not 150) and diagnostics disclose the clamped amount; WorkBuddy uses the shared subtract_cached_input helper (no inline copy); Codex keeps its intentional unclamped delta contract (existing pin untouched, mirroring upstream's carve-out); the input_tokens BASIS (cache-inclusive vs delta) is stated per source in parse diagnostics per #310; the schema-bump discipline follows the rm-485 convention with next-free slot documented (upstream 27 vs our ladder reconciled at merge-forward, one bump not two)
- evidence: campaign-recorded in agenttrace ROADMAP.md

### Keyword hosts dispatch before flag validation — leading flags silently misroute
- id: `rm-558` | track: reliability | priority: 76.0 | status: candidate
- acceptance: keyword hosts share the positional lane's pre-flight: --statusline-report before a statusline positional is an error naming both routes OR runs the report (one loud contract, pinned either way); --fetch before a statusline keyword errors loudly (mirroring the upstream lane's own fetch guard at :1537); a report-requesting invocation NEVER appends to the statusline journal (regression: journal line count unchanged); rm-505's trailing-flag guard and help routes stay green
- evidence: campaign-recorded in agenttrace ROADMAP.md

### Cap and sanitize journal-derived disclosure-counter keys (rm-542 residual)
- id: `rm-594` | track: reliability | priority: 76.0 | status: in_progress
- acceptance: counter keys derived from journal strings are length-capped at the mint sites (first N chars + short hash of the remainder, N pinned by a unit test) so any single hostile or corrupt line contributes O(1) key bytes; red-first regression drives a ~1MB unknown type through parse and asserts (a) the emitted key/report line is byte-bounded by the stated bound, (b) the persisted cache entry stays bounded, (c) the truncated shape still identifies the original (prefix + hash) — visible, not silent; the cap composes with sanitization so no raw control byte survives into a key (ties to rm-595's render arm); no behavior change for real wire shapes (all observed real types are short); SESSION_CACHE_SCHEMA_VERSION bump per the rm-230 convention if the persisted key shape changes
- evidence: campaign-recorded in agenttrace ROADMAP.md

### Subagent attribution honesty & machine-format completeness (rm-797..rm-801 + rm-855, cycle 2 batch — rm-800 rebounded to rm-855 at the commit gate)
- id: `rm-797` | track: reliability | priority: 76.0 | status: in_progress
- acceptance: linkage and dedup share ONE identity model — the parent lookup key is the same-file identity (dev/inode on Unix, canonicalized path elsewhere) with exact-path retained only as the fallback for paths that do not resolve on disk (in-memory slices, unit fixtures); a child under ANY spelling of a hardlinked parent links and rolls (PoC: /tmp/at-assess-35b3/dedup parent row flips 0/0.0000 -> 1/0.0021); a self-link (child transcript hardlinked onto its own parent file) never rolls a session into itself
- evidence: campaign-recorded in agenttrace ROADMAP.md

### Markdown export lane lacks the formula guard (markdown_cell re-arms =/+/-/@ payloads on paste)
- id: `rm-897` | track: reliability | priority: 76.0 | status: in_progress
- acceptance: markdown_cell (or the group-key rendering path) neutralizes leading =, +, -, @ cells the way csv_cell does, WITHOUT regressing the rm-403 entity-escape or the rm-540 marker-shaped-cell code-span semantics; a hostile-key fixture (=SUM(1+1), =cmd|'/c calc', =HYPERLINK(...), +1024, -1, @ref) renders guarded in markdown, benign keys stay byte-identical, and the CSV lane is unchanged; red-first regression test derived from the assess PoC corpus (/tmp/at-assess-33b7/fakehome, 15-branch hostile set)
- evidence: campaign-recorded in agenttrace ROADMAP.md

### Unify crossterm on 0.29
- id: `rm-043` | track: reliability | priority: 75.0 | status: candidate
- acceptance: a single crossterm version in the dependency graph; TUI behavior unchanged (full tui test suite green); no new advisories introduced
- evidence: campaign-recorded in agenttrace ROADMAP.md

### rm-798
- id: `rm-798` | track: reliability | priority: 75.0 | status: in_progress
- acceptance: (1) json wraps as {matched_sessions, returned_sessions, truncated, limit, sessions: [...]} — capped lists self-describe, uncapped output carries truncated:false; (2) csv appends a `# truncated: showing N of M matching sessions (--limit L)` marker row ONLY when rows were dropped (`#` marker rows are already part of the dialect's table framing); (3) every format's cap is disclosed out-of-band on stderr (Note: --limit caps this list view only: showing N of M matching sessions); (4) the TSV/text arm is byte-unchanged (pinned contract)
- evidence: campaign-recorded in agenttrace ROADMAP.md

### rm-799
- id: `rm-799` | track: reliability | priority: 75.0 | status: in_progress
- acceptance: orphaned subagent transcripts stay standalone rows (no fabricated parent) AND the count is surfaced: LoadReport grows unlinked_subagents, every report path discloses it on stderr (warning line naming the count), and the e2e pins it end-to-end
- evidence: campaign-recorded in agenttrace ROADMAP.md

### Refresh the install surface to the moved upstream distribution
- id: `rm-017` | track: reliability | priority: 74.0 | status: candidate
- acceptance: install.ps1 and install.sh download from a pinned GitHub release artifact with checksum verification (extending the install-verification mechanism already planned cross-campaign), docs name the current package surfaces, and a dry-run/manual install on each target OS succeeds from a clean environment
- evidence: campaign-recorded in agenttrace ROADMAP.md

### Remove dead self-hosted cache steps from hosted-runner CI lanes
- id: `rm-039` | track: reliability | priority: 74.0 | status: candidate
- acceptance: lint lane uses actions/cache keyed on Cargo.lock (or the dead steps are deleted) so PR wall-time no longer includes a dead 369M tar; dependency-review comment names exactly the gates that exist
- evidence: campaign-recorded in agenttrace ROADMAP.md

### Machine-readable live-quota emit surface
- id: `rm-201` | track: reliability | priority: 74.0 | status: candidate
- acceptance: a documented one-shot command (e.g. --statusline-json or `agenttrace quota`) emits current 5h/weekly windows, limits, resets_at, and TTL-correct spend as stable JSON for external hosts; offline-by-default, no daemon; schema documented and pinned by a golden test
- evidence: campaign-recorded in agenttrace ROADMAP.md

### Attribution dimensions: by provider and by task type
- id: `rm-245` | track: reliability | priority: 74.0 | status: in_progress
- acceptance: overview/json carry a by-provider rollup derived from pricing provenance (pricing_source labels already recorded) rather than a new config surface; a task-type dimension (a documented taxonomy, e.g. coding/debugging/planning) inferred from tool mix and content with per-type cost and token totals; a top-cost-drivers section naming which sessions/models/tools drive spend; golden tests on a fixture corpus with hand-labeled expectations; docs updated
- evidence: campaign-recorded in agenttrace ROADMAP.md

### Ignore relative XDG_CACHE_HOME per the basedir spec
- id: `rm-248` | track: reliability | priority: 74.0 | status: candidate
- acceptance: every artifact-root constructor that reads XDG_CACHE_HOME ignores relative values (falling back to HOME/.cache), or accepts them only with a doctor-visible warning; a test pins that `XDG_CACHE_HOME=rel-cache agenttrace …` from an empty cwd creates nothing under ./rel-cache; PRIVACY.md's artifact table stays truthful about where artifacts live
- evidence: campaign-recorded in agenttrace ROADMAP.md

### First-scan peak memory: whole-file reads x 16 parallel workers x serde_json::Value DOM, with no size admission in the walk
- id: `rm-451` | track: reliability | priority: 74.0 | status: candidate
- acceptance: a stated per-file admission bound (flag and/or constant) with a loud skip counter for over-bound files surfaced in doctor (skip, never crash); peak RSS measured on a synthetic oversized-rollout corpus pre/post with numbers recorded; zero behavior change for normal-size files; targeted + full suites green
- evidence: campaign-recorded in agenttrace ROADMAP.md

### Disqualify claude evidence on the qwen single-document path
- id: `rm-488` | track: reliability | priority: 74.0 | status: in_progress
- acceptance: is_qwen_code_event's session_id arm disqualifies on sessionId presence (mirror of 008ca97); the whole-file-JSON path at :103 applies carries_claude_evidence (or routes through the guarded file-level predicate); red-first regression test: the dual-id single-line fixture classifies claude_code; multi-line dual-id and genuine-qwen fixtures stay green (no behavior change)
- evidence: campaign-recorded in agenttrace ROADMAP.md

### OpenCode v2 fork accounting — session_v2/session_message semantics (ccusage #1780/#1782)
- id: `rm-507` | track: reliability | priority: 74.0 | status: in_progress
- acceptance: the v2 schema is ingested (or explicitly disclosed as unsupported in --doctor with a corpus count either way); fork rows excluded from aggregates when the boundary resolves; unresolvable boundaries and seq-missing schemas disclose rather than silently sum; golden fixture db exercising parent+fork+through/before+nested forks; the existing `session`-table aggregate read's behavior on v2 databases characterized (cumulative fork rows corrected or disclosed); doctor reports the v2 corpus count; sequencing rider: lands on the same module as the rm-044 rusqlite jump — sequence or co-land
- evidence: campaign-recorded in agenttrace ROADMAP.md

### Upstream v0.10.x merge hazards nobody has scoped: pricing-architecture divergence, cache-schema renumber, #313 root-slim renames
- id: `rm-513` | track: reliability | priority: 74.0 | status: in_progress
- acceptance: (1) a written merge-hazard plan (spool artifact committed with the merge) makes the pricing.rs decision EXPLICIT — keep fork bundled-snapshot and reject upstream runtime-fetch (record the offline-first rationale) or layer runtime refresh over the bundle — with the choice recorded on this item and the diff showing one architecture, not a hybrid; (2) the schema renumber lands above 26 with a forced-reparse acceptance test on a fixture v22/v7 cache and the CHANGELOG disclosing the #312 accounting deltas (CC/Codex lower, WorkBuddy higher — and WorkBuddy reconciled to upstream #311 sum-every-record, #310 closed fixed); (3) every fork-side path anchor held against upstream-renamed files is enumerated and remapped in the same plan; (4) merged head: full suite green, wall/CI references to renamed paths updated
- evidence: campaign-recorded in agenttrace ROADMAP.md

### Demo-lane hermeticity: `--demo` must never write real host state
- id: `rm-533` | track: reliability | priority: 74.0 | status: in_progress
- acceptance: demo lane never writes real host derived state — either reject --demo --preserve-history loudly at validate_primary_action (error naming both flags, the rm-341 pattern) or route demo history into the cache lane's epoch sandbox; a demo_contract pin asserts zero host-state deltas (history path + cache dir) across every --demo invocation; the assess PoC re-run shows history.json untouched; full battery green
- evidence: campaign-recorded in agenttrace ROADMAP.md

### pi azure-provider sessions price at the wrong rates — provider is dropped from the model string
- id: `rm-557` | track: reliability | priority: 74.0 | status: candidate
- acceptance: the pi lane derives a provider-respecting pricing key (azure → azure_ai/ prefix alias; legacy azure-openai-responses → the same alias) via a documented alias table, falling back LOUDLY (unknown-model disclosure) when the alias resolves nowhere; pi reports show the effective priced model; regression fixture pi-azure.jsonl pins the azure_ai rates; existing pi corpora (default provider, provider-switch pins from rm-438) unchanged
- evidence: campaign-recorded in agenttrace ROADMAP.md

### cost_audit must not re-price recorded-cost sessions into false drift
- id: `rm-570` | track: reliability | priority: 74.0 | status: candidate
- acceptance: cost_audit distinguishes recorded-cost sessions from estimate-only sessions — for recorded-basis sessions the audit reports the recorded cost as the comparison basis (or an explicit 'recorded cost; not re-priced' note) instead of re-pricing all tokens at current catalog rates; current-estimate re-pricing remains for estimate-only sessions; no regression on the catalog-miss/unknown-model note lanes
- evidence: campaign-recorded in agenttrace ROADMAP.md

### rm-578
- id: `rm-578` | track: reliability | priority: 74.0 | status: in_progress
- acceptance: --audit stops emitting the "current rates recalculate a different total than the stored estimate" note (json pricing_note AND text rendering) on sessions whose stored estimate rests on a recorded-cost basis; the re-price mirrors lib.rs's basis (exclude upstream-priced tokens, or gate the note on catalog-identity change instead of a rate-vs-basis diff); regression fixture = pi v3 journal with a type:usage entry carrying usage.cost.total asserting NO drift note, while the same tokens WITHOUT the recorded cost DO re-price; docs/guides/governance-reports.md documents the note's exact semantics (it currently never mentions the note)
- evidence: campaign-recorded in agenttrace ROADMAP.md

### Treat empty XDG_CONFIG_HOME/HOME as unset in user config resolution
- id: `rm-683` | track: reliability | priority: 74.0 | status: in_progress
- acceptance: user_config_path treats empty XDG_CONFIG_HOME AND empty HOME as unset, falling back to the spec default; a path-resolution test table pins unset→~/.config, empty→~/.config (NOT relative), set→override; live 'XDG_CONFIG_HOME= agenttrace --doctor' prints an absolute user-config path (no more 'agenttrace/config.toml (not found)' relative line); no behavior change for the project-config layer
- evidence: campaign-recorded in agenttrace ROADMAP.md

### Preserved-history identity survives transcript moves — re-key, don't double-count
- id: `rm-685` | track: reliability | priority: 74.0 | status: in_progress
- acceptance: renaming/moving a transcript under the same project re-keys the preserved-derived-history entry (identity keyed on session identity rather than path, or a move detector re-keys in place); --overview --include-history counts the moved session exactly once; a regression fixture mvs a session file between two --overview runs and asserts history entries == sessions and cost parity
- evidence: campaign-recorded in agenttrace ROADMAP.md

### Session cache self-evicts at its byte cap on large HOMEs — warm scans re-parse 86% of sessions
- id: `rm-712` | track: reliability | priority: 74.0 | status: candidate
- acceptance: (a) warm hit-rate ≥90% at the 17,887-session HOME (size-aware eviction, entry compression, or documented larger cap — entries survive between runs); (b) warm --overview ≤2min at that corpus; (c) rm-420 p95 lane gains a large-HOME cache fixture so regression is measurable
- evidence: campaign-recorded in agenttrace ROADMAP.md

### SQLite ingestion silently drops failed rows and reports unreadable databases as empty
- id: `rm-753` | track: reliability | priority: 74.0 | status: in_progress
- acceptance: (1) row-decode failures surface as a per-file failed-row counter (doctor + a stderr note on report paths — the rm-542 disclosure family), never filter_map-silent; (2) a discovered-but-unreadable DB is disclosed as unreadable, distinct from absent — --doctor may not print parsed=0 failed=0 for it, and a report run must not exit rc=1 'No session files found' over a file it found but could not open; (3) hostile-row goldens: NULL-id row, wrong-typed column, negative tokens, i64::MAX — each reported or counted-and-disclosed, none silently dropped (SQLite permits NULL in any PRIMARY KEY column that is not INTEGER PRIMARY KEY, so hostile rows are legal at open); (4) perf rider: the load paths stop sorting the full part table per session load (:494/:530) when this module is touched
- evidence: campaign-recorded in agenttrace ROADMAP.md

### Usage-key case folding bypasses the usage-disclosure classifier
- id: `rm-756` | track: reliability | priority: 74.0 | status: in_progress
- acceptance: the disclosure scan matches the same key-case/alias set Event accepts (fold case once at the scan site — no second classifier) so a 'Usage' block discloses byte-identically to 'usage'; red-first differential pair: two journals identical except key case assert EQUAL disclosure sets (both disclose, including the nested message.Usage variant); non-object usage values keep failing soft into the estimator WITH their disclosure mint; full suite green + conductor validation digest recorded in the shipping PR
- evidence: campaign-recorded in agenttrace ROADMAP.md

### Claude stream fold counts non-usage semantics per re-emission (turns, tools, events, last-ts)
- id: `rm-834` | track: reliability | priority: 74.0 | status: in_progress
- acceptance: fold becomes (message-id, per-metric-max) for usage AND a per-message-id pass for turns/tool_events/tool names/last-emission ts — with-tools corpus reports turns 1 / tools 1 / Bash:1, tokens_per_turn divides by truth turns, --overview emits no slow-tool/retries anomaly on the PoC corpus, claude-stream-growing.jsonl reports duration 7.0 / end 10:00:07Z, and the landed rm-601 fixtures stay green.
- evidence: campaign-recorded in agenttrace ROADMAP.md

### rm-855
- id: `rm-855` | track: reliability | priority: 74.0 | status: in_progress
- acceptance: csv grows subagents,subagent_cost,parent_session (cost formatted {:.4} like the cost column), README column contract updated in lockstep; TSV unchanged; determinism pin csv_output_is_byte_deterministic_across_runs keeps holding
- evidence: campaign-recorded in agenttrace ROADMAP.md

### Codex head-classification probes bytes and substrings, not parsed JSON
- id: `rm-047` | track: reliability | priority: 73.0 | status: in_progress
- acceptance: classification parses the leading JSON object and switches on the parsed `type` field (or anchors matches to field boundaries); a fixture whose content quotes the marker substrings is classified by its true type; skip decisions are counted in parse diagnostics
- evidence: campaign-recorded in agenttrace ROADMAP.md

### Release workflow idempotency
- id: `rm-169` | track: reliability | priority: 73.0 | status: candidate
- acceptance: the release job is safe to re-run (existence checks before upload/publish; no duplicate release assets); a dry-run or simulated re-run demonstrates idempotency
- evidence: campaign-recorded in agenttrace ROADMAP.md

### Statusline journal reads are unbounded (whole-file fs::read_to_string before the bounded rewrite)
- id: `rm-898` | track: reliability | priority: 73.0 | status: in_progress
- acceptance: all three read sites bound the read (metadata().len() gate + capped reader, the read_journal_capped + capped_away_bytes pattern); a >cap journal yields a disclosed capped-away count, never an OOM-scale load; locking semantics stay out of scope (rm-166's arm); red-first: a test feeds an oversized journal and pins the capped read plus its disclosure
- evidence: campaign-recorded in agenttrace ROADMAP.md

### Mirror upstream's CI governance wave on the fork
- id: `rm-018` | track: reliability | priority: 72.0 | status: candidate
- acceptance: the four governance additions run green on the fork (cargo-deny advisories+licenses config committed, Scorecard or equivalent badge lane, coverage reporting wired, git-cliff changelog generated on release); no existing gate weakened
- evidence: campaign-recorded in agenttrace ROADMAP.md

### Fix --range today UTC boundary
- id: `rm-040` | track: reliability | priority: 72.0 | status: in_progress
- acceptance: today range anchors to local midnight with documented timezone handling; a test pinning the boundary around a fixed offset passes
- evidence: campaign-recorded in agenttrace ROADMAP.md

### Wire the adversarial-fixture pytest suite into CI
- id: `rm-177` | track: reliability | priority: 72.0 | status: candidate
- acceptance: a CI job runs python3 -m pytest -q scripts/fixtures/ on every PR/push and is green at HEAD; sqlite-version coupling is either pinned in the job or the byte-equality assertions are converted to schema+row-dump comparisons per the fleet lesson (sqlite stamps writer version into file header bytes 96-99)
- evidence: campaign-recorded in agenttrace ROADMAP.md

### Fork/resume-aware rollups (parentSession linkage, replayed-prefix dedup)
- id: `rm-252` | track: reliability | priority: 72.0 | status: candidate
- acceptance: child sessions link to parents (pi parentSession + Claude replayed-prefix same-message-id detection) and disclose the fork chain in the session report; cross-session rollups (overview/governance/insights totals) dedup replayed-prefix messages across files; per-session totals UNCHANGED (only rollups dedup) — pinned by a fixture pair parent+child sharing a prefix; the linkage rule documented per format
- evidence: campaign-recorded in agenttrace ROADMAP.md

### pi journal format-contract suite — a silent-drift tripwire against a private, fast-moving upstream
- id: `rm-423` | track: reliability | priority: 72.0 | status: in_progress
- acceptance: golden fixtures pinning every sniffed field shape and usage-extraction field per pi-family variant (pi, oh-my-pi, senpi/omo) asserted in CI, with a drift failure that NAMES the offending key; --doctor or a dedicated gate reports the journal version markers observed vs the contract's known versions; a suite run against a real 1.0.2 journal recorded as the baseline
- evidence: campaign-recorded in agenttrace ROADMAP.md

### Spend-cap guard hooks (warn/stop thresholds riding the statusline channel)
- id: `rm-454` | track: reliability | priority: 72.0 | status: candidate
- acceptance: an agenttrace guard mode emits spend-threshold warnings and (configurable) stop signals from the hook surface users already install for statusline; thresholds configurable per project; docs cover install for Claude Code and statusline-compatible hosts; tests pin warn/stop ordering and per-window reset semantics; no behavior change when unset
- evidence: campaign-recorded in agenttrace ROADMAP.md

### Governance audit cost-recompute blind to recorded-cost semantics
- id: `rm-520` | track: reliability | priority: 72.0 | status: in_progress
- acceptance: the audit's current-cost arm threads recorded-cost provenance — a session whose stored estimate already includes upstream-recorded cost is either excluded from the drift comparison or compared against a catalog+recorded recompute (never catalog-only vs recorded-inclusive); a golden fixture reproducing the PoC (a pi usage entry with 1M input tokens and recorded cost $5.00, catalog current $1.25) asserts NO drift note fires for the recorded-cost session while a genuinely stale-catalog session still trips it; --recommend and the TUI audit views inherit the corrected basis; deterministic (no wall-clock inputs); boundary — the drift-note MECHANISM for catalog-staleness stays (it is rm-419's companion disclosure), only its recorded-cost blindness is this item's scope
- evidence: campaign-recorded in agenttrace ROADMAP.md

### -o report artifacts are session-derived files written umask-default
- id: `rm-525` | track: reliability | priority: 72.0 | status: in_progress
- acceptance: every file class the CLI creates from session data (-o artifacts across json/csv/markdown/html writers) is created 0600 (or umask-tightened-only) via the landed private-file helper family; a mode-assertion test covers the -o artifact class beside the existing rm-208 five-site test; PRIVACY.md's blanket sentence is made true — either the fix or the enumeration is corrected to name report artifacts; boundary — landed rm-208 (done) owns the cache/statusline/history creation sites; the unlanded "unified render-safety boundary" sibling row owns in-band BYTES, this row owns the FILE MODE class
- evidence: campaign-recorded in agenttrace ROADMAP.md

### Date-split alias pricing for codex-auto-review (server-routed alias never names its model)
- id: `rm-530` | track: reliability | priority: 72.0 | status: in_progress
- acceptance: a pricingModelAt(model, timestamp)-equivalent resolves the codex-auto-review alias date-split (gpt-5.4 rates before 2026-07-30T00:00Z, gpt-5.6-luna after — both present in the bundle: pricing.rs:1222 hand-entry / snapshot gpt-5.6 entry); red-first fixture: a codex rollout naming codex-auto-review prices $0/fallback today and date-splits correctly after; precedence preserved exactly (recorded model > user override > alias resolution, per codeburn's shipped precedence); post-30-Jul unrated credits disclose an incomplete-credits label at provenance (rm-176 discipline); every cost site routes through the resolver (overview, statusline, reports, doctor, audit recompute, session-cache rehydration); known-model pricing byte-identical (no default change); targeted + full batteries green
- evidence: campaign-recorded in agenttrace ROADMAP.md

### Codex post-compaction in-envelope growth — last_token_usage counts zero under the high-water
- id: `rm-554` | track: reliability | priority: 72.0 | status: in_progress
- acceptance: post-rewind growth counts (last-per-distinct-total, or an equivalent proven against rm-035/rm-162 pinned fixtures); the four pinned compaction tests (rm-162/rm-401 families) stay green WITHOUT weakening their pins — any pin that conflicts with the new truth is resolved explicitly with a dated note, never silently; regression fixture codex-reset.jsonl pins 1700/700 (red today); serial / rewind-then-new-work / no-compaction shapes all pinned
- evidence: campaign-recorded in agenttrace ROADMAP.md

### Mistral Vibe provider arm (~/.vibe/logs/session/unified/)
- id: `rm-689` | track: reliability | priority: 72.0 | status: in_progress
- acceptance: discovery home + parser arm for the unified Vibe journal format; the WSL path variant covered; fixtures derived from the codeburn thread corpora; README provider-table row added
- evidence: campaign-recorded in agenttrace ROADMAP.md

### rm-696
- id: `rm-696` | track: reliability | priority: 72.0 | status: candidate
- acceptance: exactly one emission on stdout for '-o /dev/stdout'/'-o /dev/stderr' (alias lane suppresses the echo); plain '-o file' keeps file + stdout echo unchanged; upstream ports cleanly (upstream write_output is plain fs::write + the same unconditional echo — inherited design, verified at upstream/master main.rs)
- evidence: campaign-recorded in agenttrace ROADMAP.md

### SQLite snapshot cache collapses multiple databases to one entry — cross-DB numbers go stale
- id: `rm-713` | track: reliability | priority: 72.0 | status: candidate
- acceptance: (a) cache ledger keys entries per DB path (e.g. BTreeMap<db-path,entry>) — a 2-DB HOME persists 2 entries; (b) touching DB #2 alone invalidates only its entry (per-DB mtime/fingerprint); (c) fixture test pins N-DB HOME round-trip
- evidence: campaign-recorded in agenttrace ROADMAP.md

### Top-level JSON array journals bypass the usage-disclosure mint
- id: `rm-757` | track: reliability | priority: 72.0 | status: in_progress
- acceptance: array-lane journals produce the same usage-disclosure mint the line-oriented lane produces (shared helper — not a second classifier); red-first pair: an array journal and its object-lane equivalent assert EQUAL disclosure sets; the lib.rs:931 estimator arm becomes unreachable for usage the array lane can see; full suite green + digest recorded
- evidence: campaign-recorded in agenttrace ROADMAP.md

### rm-801
- id: `rm-801` | track: reliability | priority: 72.0 | status: in_progress
- acceptance: README --sessions flag row + csv section state the corpus scope of the rollups, the truncation disclosure (json wrapper + csv marker + stderr note), and the orphan stderr warning; the --help --sessions entry is extended; check-docs-commands.sh parity keeps holding (README rows == help flags, no blanks)
- evidence: campaign-recorded in agenttrace ROADMAP.md

### agenttrace mcp tools/call: non-object `arguments` must fail -32602, not execute with defaults
- id: `rm-840` | track: reliability | priority: 72.0 | status: in_progress
- acceptance: a tools/call whose `arguments` is not a JSON object returns a -32602 error naming the tool and the violated constraint; absent `arguments` stays legal per spec and keeps the {} default; the tool never executes on rejected input; a regression test pins the garbage-arguments case (the assess PoC shape: arguments "garbage-not-object")
- evidence: campaign-recorded in agenttrace ROADMAP.md

### Statusline journal lock-vs-rename race is unguarded on Windows (guard returns true unconditionally)
- id: `rm-920` | track: reliability | priority: 72.0 | status: candidate
- acceptance: (a) non-unix arm compares file identity (Windows file_index via MetadataExt, or equivalent) OR compaction refuses the rename-based protocol on non-unix; (b) the arm's comment matches the code it annotates; (c) red-first: a cfg-gated test pinning the guard semantics where compilable, plus compile coverage via the cross-OS matrix port (see the linux-only-tests section fold, upstream #303 source 36b943f); (d) full suite green on the primary lane
- evidence: campaign-recorded in agenttrace ROADMAP.md

### Refresh ureq and rusqlite majors
- id: `rm-044` | track: reliability | priority: 71.0 | status: in_progress
- acceptance: pricing transport migrated to ureq 3.x with identical request behavior locked by existing pricing-fetch tests; hermes/opencode sqlite reads migrated to current rusqlite with golden db fixtures passing
- evidence: campaign-recorded in agenttrace ROADMAP.md

### Truthful MSRV policy (rust-version vs edition-2024 dependency floor)
- id: `rm-386` | track: reliability | priority: 71.0 | status: candidate
- acceptance: rust-version is raised to the true build floor (dep-greatest, documented) or the edition-2024 deps are pinned back; a CI lane (or the existing toolchain pin) proves the declared MSRV actually builds — no advertised floor without a proving lane; documented in README/install docs
- evidence: campaign-recorded in agenttrace ROADMAP.md

### `-o` destination colliding with an input journal silently destroys the source transcript
- id: `rm-534` | track: reliability | priority: 71.0 | status: in_progress
- acceptance: write_output (or its callers) rejects a destination resolving to a file in the loaded session set (explicit paths AND discovery dirs) with an explicit error naming both roles ("output path is an input journal"), before any write; -o /dev/full-class behavior stays per the sibling sanitizer lane's pins; regression test pins collision rc!=0 and untouched journal bytes; the assess PoC flips to a loud error
- evidence: campaign-recorded in agenttrace ROADMAP.md

### Adopt upstream release-engineering wave (crt-static, git-cliff CHANGELOG, republish workflows)
- id: `rm-045` | track: reliability | priority: 70.0 | status: candidate
- acceptance: Windows release binaries link the static CRT (no VC redist requirement, verified by dumpbin or a CI artifact check); CHANGELOG generated by git-cliff with existing history preserved as prior context; release republish steps pinned and exercised once
- evidence: campaign-recorded in agenttrace ROADMAP.md

### Shell completions and man page via clap_complete/clap_mangen
- id: `rm-088` | track: reliability | priority: 70.0 | status: candidate
- acceptance: `agenttrace completions <shell>` emits bash/zsh/fish completions and a man page renders via clap_mangen; generation runs in CI so artifacts cannot drift from --help; outputs are wired into the existing homebrew/winget/npm channels where each has a convention; a CI step exercises generation end to end
- evidence: campaign-recorded in agenttrace ROADMAP.md

### Extend baseline gates to quality metrics
- id: `rm-159` | track: reliability | priority: 70.0 | status: candidate
- acceptance: baseline gates accept waste/loop/health metrics with stored baselines and delta-percent flags alongside the cost family; a regression vs stored baseline exits nonzero with a named metric; golden tests cover one passing and one regressed baseline per metric class
- evidence: campaign-recorded in agenttrace ROADMAP.md

### Parser-corpus breadth: next agent homes (Droid, Zcode, Kiro)
- id: `rm-203` | track: reliability | priority: 70.0 | status: candidate
- acceptance: discovery+parser support at least one of the three, chosen on evidence of real corpora (a corpus-donation issue or documented path shapes precedes implementation); source labels render and are pinned by parser tests; the discovery contract test covers the new homes
- evidence: campaign-recorded in agenttrace ROADMAP.md

### Disclose discovery-walk failures and fix the -d <file> error message
- id: `rm-249` | track: reliability | priority: 70.0 | status: candidate
- acceptance: unreadable directories are reported (count + path, non-fatal, on stderr and in the load report's skip disclosure) instead of silently narrowing coverage; `-d <unreadable-root>` errors distinguish permission denial from absence with a check-permissions hint; `-d <file.jsonl>` either scans that file or errors that -d takes a directory (pointing at the positional form); fixtures pin the chmod-000-subdir and -d-file cases
- evidence: campaign-recorded in agenttrace ROADMAP.md

### Installer release identity: pin the download or verify embedded digests
- id: `rm-340` | track: reliability | priority: 70.0 | status: candidate
- acceptance: the default installer download is pinned to an exact release tag (explicit opt-in escape hatch for latest), OR digests are embedded at render time so verification is independent of any sidecar fetch; the three-installer parity test family (all_three_installers_refuse_when_the_sidecar_cannot_be_verified, crates/agenttrace-core/src/lib.rs) extends to the pinning/embedding behavior; clean-env install smoke per OS stays green
- evidence: campaign-recorded in agenttrace ROADMAP.md

### pi model_change reads the wrong wire key (modelId) — dead handler, stale attribution
- id: `rm-438` | track: reliability | priority: 70.0 | status: in_progress
- acceptance: the model_change arm accepts modelId (and uses provider for context), keeping `model` as a legacy fallback; a golden fixture pins the transition (assistant-on-A → model_change{provider,modelId:B} → model-less assistant → tokens attributed to B and priced via B); PoC re-run attributes the post-switch 370 tokens to gpt-4o instead of pricing them as claude-sonnet-4-5
- evidence: campaign-recorded in agenttrace ROADMAP.md

### New-format parsers: crush (SQLite) and goose sessions
- id: `rm-456` | track: reliability | priority: 70.0 | status: candidate
- acceptance: crush sessions read from crush.db via a read-only schema-versioned query with zero writes, tokens/cost attributed, doctor row added; goose storage path pinned from source, parser + doctor row; both behind discovery registration with auto-discovery unchanged elsewhere; redacted fixtures from real corpus samples; targeted + full suites green
- evidence: campaign-recorded in agenttrace ROADMAP.md

### Bound the upstream-status subprocess waits
- id: `rm-543` | track: reliability | priority: 70.0 | status: in_progress
- acceptance: both subprocess paths run under a stated deadline; expiry produces a terminal error naming the deadline and the git operation that timed out (never a silent partial report); the timeout constant is pinned by a unit test and documented in docs/guides/upstream-status.md; scripts/upstream-delta --fetch inherits the same bound or documents its dev-tool exemption
- evidence: campaign-recorded in agenttrace ROADMAP.md

### opencode usage parsing drops the tokens.reasoning subtree when usage.total is 0
- id: `rm-619` | track: reliability | priority: 70.0 | status: in_progress
- acceptance: tokens.reasoning folds into the session's token/cost accounting when the total-style usage is 0 (or the drop fires a disclosure counter in the rm-408 family); golden opencode fixture with total 0 + non-zero tokens.reasoning red-first; full suite green
- evidence: campaign-recorded in agenttrace ROADMAP.md

### Range/scope windows admit sessions by START only — overnight sessions invisible to --range today
- id: `rm-694` | track: reliability | priority: 70.0 | status: in_progress
- acceptance: window membership admits a session when its START or LAST ACTIVITY falls in the window (span overlap), overview window end = max(session_end) over admitted sessions, latest_session_at = max(last activity), the overnight corpus reports $0.0158 under --range today, README documents span semantics, and --range all output is byte-identical to today's.
- evidence: campaign-recorded in agenttrace ROADMAP.md

### rm-695
- id: `rm-695` | track: reliability | priority: 70.0 | status: candidate
- acceptance: caps and tier floors re-derived so the red tier is reachable by real degenerate corpora (e.g. floors ≤ sum of caps, or explicit severity escalation), with a fixture that actually lands in red
- evidence: campaign-recorded in agenttrace ROADMAP.md

### rm-904
- id: `rm-904` | track: reliability | priority: 70.0 | status: in_progress
- acceptance: recorded from agenttrace ROADMAP.md by campaign ingest
- evidence: campaign-recorded in agenttrace ROADMAP.md

### rm-912
- id: `rm-912` | track: reliability | priority: 70.0 | status: in_progress
- acceptance: the -o collision check covers every path the loader ATTEMPTED, not only the sessions it produced — parse-failure paths from the LoadReport join the membership set and the run refuses BEFORE any write (non-zero rc, error naming the input transcript); red-first with the mixed-corpus PoC as fixture (good.jsonl + UTF-16 bad.jsonl, -o at bad.jsonl → refusal + byte-identical file); the all-failed bail is regression-pinned; rm-534's landed refusal for parseable inputs unchanged
- evidence: campaign-recorded in agenttrace ROADMAP.md

### statusline_insights limit-crossings loop is O(n*k) with a JSON clone+parse per probe — sparse journals degrade quadratically
- id: `rm-921` | track: reliability | priority: 70.0 | status: in_progress
- acceptance: (a) single precomputation pass parses each capture's window state once; crossings derived by a sorted sweep (O(n log n) or better) — no per-probe from_value; (b) boundary before/after selected by captured_at (max-before/min-after); (c) red-first sparse-shape regression test pinning the bound (wall-time or step-count proxy) proven red against unfixed source per the red-first provenance rule; (d) --statusline-report / --doctor / TUI governance outputs byte-identical on the benign corpus before vs after; (e) full suite green
- evidence: campaign-recorded in agenttrace ROADMAP.md

### Hermeticize the core insights project-decode test (shared-TMPDIR dependence)
- id: `rm-241` | track: reliability | priority: 69.0 | status: candidate
- acceptance: the fixture roots under a per-test temp directory the test exclusively owns (injected base path or per-test env), and a checked-in regression fixture placing a hostile agenttrace/ sibling beside the fixture root proves the test stays green; full suite green on hosts with populated shared TMPDIRs; extends rm-204's hermeticity class from the tui crate into core
- evidence: campaign-recorded in agenttrace ROADMAP.md

### Strict SQLite failure-marker semantics (LIKE false positives)
- id: `rm-387` | track: reliability | priority: 69.0 | status: candidate
- acceptance: failure classification uses structured evidence (result status/is_error fields where the schema provides them) with the LIKE heuristic demoted to a labeled fallback; fixtures pin (a) genuine failure counted, (b) success echoing the marker not counted, (c) fallback flagged in output when used; health/gate outputs unchanged for clean corpora
- evidence: campaign-recorded in agenttrace ROADMAP.md

### Unicode-aware case folding in CLI search and TUI filters
- id: `rm-016` | track: reliability | priority: 68.0 | status: candidate
- acceptance: case-insensitive matching in the CLI search path and TUI filters uses Unicode-aware folding; tests with non-ASCII case-variant queries (e.g. full-width Latin, Cyrillic, Turkish dotted-I documented as out-of-scope or handled) pass on both paths
- evidence: campaign-recorded in agenttrace ROADMAP.md

### Hermetic TUI tests (statusline journal isolation)
- id: `rm-204` | track: reliability | priority: 68.0 | status: candidate
- acceptance: the statusline journal path resolves inside each test's temp dir (injected or env-set) so no test reads $HOME; the full suite is green with AND without AGENTTRACE_SESSION_CACHE_DIR set; no per-host flake remains
- evidence: campaign-recorded in agenttrace ROADMAP.md

### Flat-pair id namespace hardening (synthesized-id collision + nameless results)
- id: `rm-300` | track: reliability | priority: 68.0 | status: candidate
- acceptance: synthesized ids cannot collide with journal-supplied ids (reserved-namespace escape, or disambiguation keyed on (name, ordinal, index) outside the id space); a result lacking tool_name pairs via a global positional fallback OR is disclosed as unmatched-with-reason, never silently dropped; both PoC fixtures imported as regression tests — collide reports one unmatched call, nameless pairs or discloses
- evidence: campaign-recorded in agenttrace ROADMAP.md

### Cache-load sweep must match agenttrace's own temp suffix, not any '.tmp.' substring
- id: `rm-368` | track: reliability | priority: 68.0 | status: candidate
- acceptance: the sweep predicate matches only files agenttrace itself could have created — exact numeric .json.tmp.<pid>.<seq> suffix (or an equivalent namespace-scoped form); red-first regression test plants myown.tmp.v1 aged past ORPHAN_TEMP_MAX_AGE inside the cache dir and pins its survival across a load_session_cache sweep while a genuine stale <name>.json.tmp.<pid>.<seq> temp is still removed
- evidence: campaign-recorded in agenttrace ROADMAP.md

### workbuddy input_tokens basis is an unstated assumption; a basis mismatch clamps input to silent zero (upstream #310)
- id: `rm-450` | track: reliability | priority: 68.0 | status: in_progress
- acceptance: the basis assumption is disclosed whenever the subtraction fires (counter or provenance note in the rm-400-class disclosure family); a basis-mismatch fixture produces a visible disclosure, not a silent zero; the existing negative-input survival test still passes; arithmetic re-pinned once upstream resolves #310 (watch recorded on this row)
- evidence: campaign-recorded in agenttrace ROADMAP.md

### pi message-level cost authority: reconcile or disclose the vendor divergence
- id: `rm-522` | track: reliability | priority: 68.0 | status: candidate
- acceptance: EITHER (a) message-level recorded cost becomes authoritative for cost totals (catalog fallback when the message carries no cost) with the pi_journal_contract goldens updated in the same delta and a recorded-vs-catalog disclosure line in provenance — vendor parity, or (b) the divergence is made explicit: the pi provenance line states message-level costs are catalog-estimated while vendor totals use recorded cost, and the pi journal docs guide documents why with the 0.0428/0.0075 fixture as the worked example — decision recorded on this row with a spec citation either way; a fixture carrying BOTH message-level recorded cost and a usage entry asserts whichever semantics ships; the golden may not change silently — its contract test updates in the same delta
- evidence: campaign-recorded in agenttrace ROADMAP.md

### Copilot metrics-less shutdown timestamp — duration loses the activity tail
- id: `rm-556` | track: reliability | priority: 68.0 | status: in_progress
- acceptance: the shutdown record's timestamp propagates to the credit event or a terminal marker whenever it is the latest activity, so session_end/duration_sec reflect it; regression fixture one-p2 pins duration 20.0 (red today); sessions whose checkpoint is LATER than the shutdown keep the checkpoint timestamp (no regression for open sessions)
- evidence: campaign-recorded in agenttrace ROADMAP.md

### rm-705
- id: `rm-705` | track: reliability | priority: 68.0 | status: candidate
- acceptance: discovery + parser lane for Command Code journals: per-message usage folded, checkpoints siblings skipped, costUsd presence-checked (costFromBilling), doctor reports the provider; live-billing quota optional arm
- evidence: campaign-recorded in agenttrace ROADMAP.md

### Per-session cost anatomy: explain why one session cost what it did
- id: `rm-729` | track: reliability | priority: 68.0 | status: in_progress
- acceptance: a one-shot surface (e.g. `agenttrace --why <session-identity>` or a named-session arm on --waste/--report) names the session it analyzed (human-readable title + stable stem per rm-407) and itemizes its cost — by model, by hour/phase buckets, cache-read vs fresh-input split, and top contributing turns — reusing the rm-245 aggregation at session scope; ambiguous or unmatched selection fails loudly (no silent latest pick: any implicit selection is disclosed on stderr and carried in JSON); all title/text rendering routes through the sanitizer family (rm-403 boundary); offline by default, no network
- evidence: campaign-recorded in agenttrace ROADMAP.md

### Decide the gemini_cli parser lane: retire with the Antigravity transition or fix the single-doc claiming hole
- id: `rm-758` | track: reliability | priority: 68.0 | status: candidate
- acceptance: DECISION-GATED — record the decision on this row with its landing, either arm: (retire) the gemini lane is deleted with a CHANGELOG note and journals that today claim as gemini_cli parse under their true source or disclose unknown-source — the assess F3 fixture pair pins the flip red-first; or (fix) single-doc claiming requires a gemini-shaped discriminator before the claim, unknown usage keys disclosed not dropped, red-first low-usage.jsonl golden; the decision cites upstream #312 + #236 + Google's announcement and stays visible at the fork-maintenance wave-port gate (gemini-lane survival gates how much parser code the port carries); full suite green + digest recorded
- evidence: campaign-recorded in agenttrace ROADMAP.md

### OpenCode v1 subagent lineage (parent_id) is invisible to the sqlite lane
- id: `rm-791` | track: reliability | priority: 68.0 | status: in_progress
- acceptance: (1) the sqlite lane reads parent_id and exposes parent linkage on ingested sessions (child.parent_session, the subagents.rs field family); (2) lineage-aware rendering: children roll up under their parent with parent-own vs children-aggregate distinguishable; (3) planted fixture: crafted opencode.db with 1 parent + 2 children ingests as 1 lineage group, totals = parent-own + Σ children; (4) parentless sessions (208 on this host) unaffected
- evidence: campaign-recorded in agenttrace ROADMAP.md

### Upstream drift report trusts FETCH_HEAD without disclosing its age
- id: `rm-048` | track: reliability | priority: 67.0 | status: candidate
- acceptance: FETCH_HEAD-derived answers carry the ref's age and a downgraded authority; when no fresh-enough ref exists the report prints an explicit stale/unknown marker instead of an unqualified value
- evidence: campaign-recorded in agenttrace ROADMAP.md

### Bound the delivery-evidence git subprocess
- id: `rm-242` | track: reliability | priority: 67.0 | status: in_progress
- acceptance: the git invocation runs under a bounded timeout consistent with the pricing-fetch cap class; on timeout the evidence record degrades soft (level recorded with an explicit timeout annotation, report still completes, exit code unchanged for non-CI lanes); a stand-in sleeping-git test pins the bound; the per-root subprocess budget is documented in the governance guide alongside the existing read-only disclosure
- evidence: campaign-recorded in agenttrace ROADMAP.md

### Sessions-list truncation disclosure and true count
- id: `rm-388` | track: reliability | priority: 67.0 | status: candidate
- acceptance: --sessions JSON gains a total-count field (or an envelope) and prints a stderr truncation note when capped, mirroring the governance branch; --search count reports true matches alongside returned-limit; a contract test pins both
- evidence: campaign-recorded in agenttrace ROADMAP.md

### Archived and relocated projects collapse into one "unknown" attribution bucket
- id: `rm-049` | track: reliability | priority: 66.0 | status: candidate
- acceptance: fallback identities are disambiguated (parent-dir or path-hash suffix) so distinct unknown projects never merge; genuinely unidentifiable sessions keep a single explicit bucket or have it explicitly retired; tests pin two distinct non-git directories yielding distinct buckets
- evidence: campaign-recorded in agenttrace ROADMAP.md

### Make -m truthful on --compare and --test-match
- id: `rm-089` | track: reliability | priority: 66.0 | status: candidate
- acceptance: -m on --compare either filters sessions to the named model or prices the comparison against it, with the chosen semantics documented; -m on --test-match targets the named model's lookup (the flag's evident purpose) or the combination exits with an explicit unsupported-flag error; tests pin both actions with a -m argument present
- evidence: campaign-recorded in agenttrace ROADMAP.md

### Windows USERPROFILE fallback for artifact roots
- id: `rm-205` | track: reliability | priority: 66.0 | status: candidate
- acceptance: directory resolution honors USERPROFILE before HOME on Windows (or the documented equivalent); a unit test sets USERPROFILE and unsets HOME and asserts the resolved root; all existing path tests stay green on Linux
- evidence: campaign-recorded in agenttrace ROADMAP.md

### Stop accepting armv7l the release lane never builds
- id: `rm-236` | track: reliability | priority: 66.0 | status: candidate
- acceptance: either an armv7 release lane exists or install.sh exits with an explicit unsupported-platform message naming the supported set; the arch map and the release target matrix are locked together by a check so they cannot drift apart again
- evidence: campaign-recorded in agenttrace ROADMAP.md

### Early-exit output contract: honor -o/-f or reject loudly; make --baseline x --compare coherent
- id: `rm-341` | track: reliability | priority: 66.0 | status: in_progress
- acceptance: every early-exit path (--test-match, --list-models; --statusline-report already compliant) honors -o and -f json OR exits rc!=0 with an explicit unsupported-flag error naming the flag — never silent; --demo honors --lang or rejects it; the --baseline x --compare pair is made coherent (either --compare consumes --baseline or the first error stops advising a combination the second error forbids); --compare documented in README/docs including its gating; golden tests pin each early-exit path's output contract
- evidence: campaign-recorded in agenttrace ROADMAP.md

### Emit OpenTelemetry GenAI-conformant spans per session (OTLP-JSON export lane)
- id: `rm-493` | track: reliability | priority: 66.0 | status: candidate
- acceptance: `agenttrace export --format otel <session-selector>` writes an OTLP-JSON FILE (spans per session + per assistant turn) carrying gen_ai.system, gen_ai.request.model, gen_ai.usage.input_tokens/output_tokens (per the semconv release current at implementation), and an agenttrace-namespaced cost attribute with the pricing_source string; offline-first — no network, honors -o conventions (special-file safety rebinds to the -o lane if it lands first); a golden fixture pins the attribute names with a comment naming the semconv commit date; boundaries: file export ONLY — no collector/OTLP-HTTP sender (network send stays out per PRIVACY.md posture), and this item does not touch the report renderers' existing arms
- evidence: campaign-recorded in agenttrace ROADMAP.md

### Cost provenance should ride the report lanes, not just doctor
- id: `rm-514` | track: reliability | priority: 66.0 | status: in_progress
- acceptance: --overview and --sessions text/markdown render a per-model rate-provenance summary (at minimum one footnote line per distinct source actually used: bundled catalog, user override, override+deprecated, fallback-unpriced); CSV gains a source column; -f json gains a lossless per-model `pricing_source` field on the versioned schema; NUMBERS UNCHANGED (disclosure, not repricing); byte-deterministic output (provenance strings derive from catalog state only — no wall clock; compose rm-419 discipline); golden tests pin all four provenance classes incl. the deprecated-override compound arm of pricing.rs:320-415; --audit and --doctor output unchanged
- evidence: campaign-recorded in agenttrace ROADMAP.md

### Opt-in cursor.com usage sync arm (recorded-cost provenance for the Cursor lane)
- id: `rm-539` | track: reliability | priority: 66.0 | status: candidate
- acceptance: an opt-in, OFF-by-default network arm (rm-302's opt-in live-quota pattern) that pulls the user's cursor.com usage export and feeds recorded-cost rows into the Cursor lane with explicit recorded-vs-estimated provenance; PRIVACY.md's network-touch enumeration extended (cookie reuse is a scope decision the privacy contract must name); runs only from report surfaces, never doctor/audit/discovery; a capability flag documents the cadence cap (≤1/hour + jitter); composes with the export parser arm (parser.rs:3307) without duplicating it
- evidence: campaign-recorded in agenttrace ROADMAP.md

### The upstream status report must honor -o like every report action
- id: `rm-605` | track: reliability | priority: 66.0 | status: in_progress
- acceptance: `agenttrace -o <file> upstream` writes the report to the file (stdout silent unless `-o -` or no -o) exactly like the other report actions, OR the combination is rejected loudly at parse time naming the flag (rc=2) — never a silent no-op; end-to-end test asserts the output file EXISTS and contains the report's bytes after `-o <tmpfile> upstream` against a fixture repo, and that `-o -` upstream writes stdout-only; rm-572's flag-x-target matrix (unlanded) gains the `-o x upstream` cell when it lands
- evidence: campaign-recorded in agenttrace ROADMAP.md

### Event model identity and usage silently drop to text estimation when model_used rides a non-payload line
- id: `rm-616` | track: reliability | priority: 66.0 | status: in_progress
- acceptance: a journal line carrying model_used (either casing) with top-level usage attributes to the true model with true tokens — golden fixture from /tmp/at-assess-4502/corpus/c2.json red-first (model 'default' 1/1 → gpt-5 7/3) — OR the fallback discloses the drop (a dropped-lines/estimation counter increments when a rejected line carries model_used or usage keys); a casing-parity test pins both ModelUsed and model_used at whichever layer owns the alias; full suite green
- evidence: campaign-recorded in agenttrace ROADMAP.md

### Antigravity standalone app: count cache reads, price app models
- id: `rm-690` | track: reliability | priority: 66.0 | status: in_progress
- acceptance: antigravity app transcripts contribute cache_read/cache_write token classes with a disclosure when the store shape differs from the CLI's; app-variant model names resolve to priced SKUs or route to the DISCLOSED fallback (never silently to $0); fixture from the #1655 store shape
- evidence: campaign-recorded in agenttrace ROADMAP.md

### rm-707
- id: `rm-707` | track: reliability | priority: 66.0 | status: candidate
- acceptance: generation-speed metrics: output tokens/sec per model and per session in analyze output + a summary view; div-by-zero and sub-second-window edge cases disclosed
- evidence: campaign-recorded in agenttrace ROADMAP.md

### Agent-format expansion wave — bootstrap new provider parsers from codeburn's per-format docs
- id: `rm-776` | track: reliability | priority: 66.0 | status: in_progress
- acceptance: wave-lane strategy — this row is the umbrella, per-format rows split out at prioritize time; each format = parser arm + golden corpus fixture + --audit disclosure of skipped/unparsed records + README/docs row; formats with SQLite or packed schemas fail loud on schema drift, never silently zero-count; full suite green per lane; no format admitted on doc evidence alone — codeburn's docs/providers/<tool>.md bootstrap docs (zed.md, goose.md fetched as spec evidence) must be confirmed against a real local corpus or a synthetic corpus generated to the documented schema before the parser lands
- evidence: campaign-recorded in agenttrace ROADMAP.md

### rm-785
- id: `rm-785` | track: reliability | priority: 66.0 | status: in_progress
- acceptance: --sort recent (main.rs), --latest selection (newer_session_order), and the TUI SortKey::Recent comparator all order by parsed instants (the lib.rs canonical_sessions parse_ts basis) via one shared comparator; regression fixture pins a mixed-offset corpus (+09:00 vs Z) so any future non-normalizing producer fails loudly
- evidence: campaign-recorded in agenttrace ROADMAP.md

### Discovery-lane parse errors swallowed at .ok(); exhausted sets bail with a misleading message
- id: `rm-835` | track: reliability | priority: 66.0 | status: in_progress
- acceptance: discovery counts parse failures and, when the admitted set is empty but failures > 0, reports "N files matched but failed to parse (first error: …)" with rc1 on the discovery lane; --doctor's parsed/failed split unchanged; a regression test with a mixed readable+corrupt corpus keeps the readable sessions visible.
- evidence: campaign-recorded in agenttrace ROADMAP.md

### Portable redacted session bundles (export/import)
- id: `rm-160` | track: reliability | priority: 65.0 | status: candidate
- acceptance: `agenttrace export` writes a single-file bundle (sessions + pricing snapshot + manifest, compressed) with redaction applied, and `import`/analysis consumes it without the original local stores; round-trip test proves bundle-in equals store-derived output; a canary fixture generated from a real bundle passes the format tracker
- evidence: campaign-recorded in agenttrace ROADMAP.md

### Gate scripts must not default to a shared /tmp binary
- id: `rm-377` | track: reliability | priority: 65.0 | status: candidate
- acceptance: the three scripts default to the repository's own release binary (repo_root via git rev-parse or script-relative path, mirroring check-rust-real-cli-smoke.sh:7) with the AGENTTRACE_BIN override retained; each fails cleanly with a named error when no built binary exists (never silently falls back to /tmp); a poison-stub proof (stub planted at /tmp/agenttrace, env unset) shows none of the three executes it, red-path recorded
- evidence: campaign-recorded in agenttrace ROADMAP.md

### Fix or remove dead cost_per_output_token trend field
- id: `rm-532` | track: reliability | priority: 65.0 | status: in_progress
- acceptance: (1) field replaced by a per-million derivation matching the totals twin (consistent with any existing per-million input field) or removed outright; (2) if replaced: text+json show non-zero on a live corpus and both agree with totals; (3) pinned test asserts projects[] and totals derivations are consistent for the same fixture; (4) suite green
- evidence: campaign-recorded in agenttrace ROADMAP.md

### Honor -f json for the waste report
- id: `rm-544` | track: reliability | priority: 65.0 | status: in_progress
- acceptance: `--waste -f json` emits the waste report as a JSON document (schema added to docs alongside the other json surfaces) and `--waste -f markdown`/`-f html` either render or reject per the guard's stated contract — one consistent rule for every machine format; a format-matrix test (text/json/markdown x waste) pins rc + machine-parseability per cell
- evidence: campaign-recorded in agenttrace ROADMAP.md

### Make the sessions TSV honestly requestable: explicit `-f tsv` + announce off stdout
- id: `rm-937` | track: reliability | priority: 65.0 | status: in_progress
- acceptance: tsv accepted as an explicit -f value, byte-identical to the default text lane (same renderer, same truncation markers; README:368 flag table gains tsv); announce side-effects routed to stderr for the sessions lane whether tsv is defaulted or explicit — `--clear-cache --sessions > rows.tsv` is table-only bytes, pinned by a red-first stdout-purity test alongside CU3's; a byte-equality test explicit `-f tsv` vs default lane; --help text updated
- evidence: campaign-recorded in agenttrace ROADMAP.md

### Widen the lockfile gate to all lockfile-mutating cargo verbs
- id: `rm-178` | track: reliability | priority: 64.0 | status: candidate
- acceptance: the gate scans all lockfile-mutating cargo verbs across workflows and scripts; a seeded workflow containing a bare `cargo run` fails the gate locally; the passing case stays green
- evidence: campaign-recorded in agenttrace ROADMAP.md

### Bound every git subprocess invocation
- id: `rm-206` | track: reliability | priority: 64.0 | status: candidate
- acceptance: every git subprocess runs under an explicit timeout with degraded-mode behavior (skip + annotate) instead of hanging; governance's per-root git log result is cached; a stub slow-git test pins the timeout path for both call sites
- evidence: campaign-recorded in agenttrace ROADMAP.md

### Static-context attribution: skills, MCP definitions, system prompt
- id: `rm-237` | track: reliability | priority: 64.0 | status: candidate
- acceptance: a static-vs-dynamic context split is reported per session wherever the data exists (tool schemas appear in transcripts; skill and MCP definition sizes when recorded); unattributable slices degrade honestly (marked unattributed, never estimated); docs name exactly what is measured
- evidence: campaign-recorded in agenttrace ROADMAP.md

### String-typed usage numbers: coerce or disclose — never silently drop
- id: `rm-342` | track: reliability | priority: 64.0 | status: in_progress
- acceptance: integer-valued string usage fields coerce (lenient parse) OR are counted as a disclosed skip class visible in --doctor; a PoC session (input 100 as string + output 50 as number) reports TOKENS 150 (or 50 plus an explicit dropped-fields disclosure) — never silently; a regression fixture pins the behavior
- evidence: campaign-recorded in agenttrace ROADMAP.md

### Extra positional session paths are silently ignored (single args.path)
- id: `rm-439` | track: reliability | priority: 64.0 | status: candidate
- acceptance: extra positional session paths fail LOUDLY (rm-247 house rule: name the dropped operand and exit nonzero) OR are honored (multi-file --compare is the natural reading of "Multi-Session Comparison"); whichever way, the pinning test is updated to the new contract and a CLI probe asserts the chosen behavior; --compare with two files must never silently audit one
- evidence: campaign-recorded in agenttrace ROADMAP.md

### Naive-ISO timestamps parse for --overview but vanish from --sessions and --diagnostics
- id: `rm-502` | track: reliability | priority: 64.0 | status: in_progress
- acceptance: (a) a naive-ISO timestamp (e.g. 2026-10-05T12:00:00 — no offset, no Z) is handled identically at EVERY consumer: --sessions lists the session, --diagnostics derives its tool_latencies, --overview keeps counting it — pinned by a committed fixture pair exercising both timestamp arms end-to-end, red-first on the divergent arm; (b) OR the lenient arm is removed and --overview refuses naive stamps with a stderr disclosure naming the file and its offending timestamp — one semantic either way, with the chosen direction locked by a round-trip test on both corpora; (c) the second lenient copy at parser.rs:3720-3730 is unified with lib.rs's parse_ts (single source of timestamp truth, unit test proving both call sites agree on naive, rfc3339, and garbage inputs); (d) any semantic change to the accepted-timestamp space bumps SESSION_CACHE_SCHEMA_VERSION per the rm-230/rm-009 regeneration rule (22 at this base — re-verify the live value at implement time, fleet lanes carry in-flight 23 bumps)
- evidence: campaign-recorded in agenttrace ROADMAP.md

### Exclude opencode fork-copied history from usage reports
- id: `rm-548` | track: reliability | priority: 64.0 | status: in_progress
- acceptance: survey-first per the rm-456 crush precedent — pin the opencode fork marker from the storage schema (forkedFrom/parent field name verified against opencode source at implement time) with a fixture doc BEFORE any parser arm; forked copies are excluded from aggregated usage (discovery diagnostics disclose the exclusion count per session, never silent); explicit-path loads of a forked copy still render (the exclusion is discovery-aggregation only); golden corpus parent+fork+independent asserting each transcript counts exactly once
- evidence: campaign-recorded in agenttrace ROADMAP.md

### Statusline journal append-vs-compaction lock — close the lost-append window
- id: `rm-686` | track: reliability | priority: 64.0 | status: in_progress
- acceptance: a cross-process lock (or an atomic append-only design) closes the append-vs-compaction window: an interleaved append survives compaction; a concurrency test runs a writer against a compactor and proves zero lost entries; a crash mid-compact leaves the journal readable (previous inode intact until rename)
- evidence: campaign-recorded in agenttrace ROADMAP.md

### delivery_evidence disclaimer repeats ~155 chars per row — half the overview JSON is boilerplate
- id: `rm-714` | track: reliability | priority: 64.0 | status: in_progress
- acceptance: (a) per-row disclaimer hoisted to a single document-level field or capped per row (the :585-590 precedent) — doc shrinks ≥40% on the 24.5k-row corpus with zero information loss; (b) schema bump only if the JSON shape changes for consumers; (c) golden-size test pins the shrink
- evidence: campaign-recorded in agenttrace ROADMAP.md

### Rank delivery-evidence levels explicitly
- id: `rm-243` | track: reliability | priority: 63.0 | status: in_progress
- acceptance: evidence records ordered by an explicit total rank (strong > medium > weak > non_code > none, or a documented order) shared by text and json lanes; a golden test with one record at every level pins the ordering
- evidence: campaign-recorded in agenttrace ROADMAP.md

### Route single-JSON-object journals through the JSONL probe block (workbuddy/kimi one-liners classify generic)
- id: `rm-586` | track: reliability | priority: 63.0 | status: candidate
- acceptance: 1) single-object JSONL journals enter the same probe dispatch as multi-line ones (or a dedicated
- evidence: campaign-recorded in agenttrace ROADMAP.md

### Render UNPROVABLE — not zero — where evidence is absent
- id: `rm-054` | track: reliability | priority: 62.0 | status: candidate
- acceptance: surfaces whose inputs are absent render an explicit UNPROVABLE (or equivalent) marker in text plus an additive JSON field; no numeric zero is printed where evidence was never captured; golden tests pin both branches
- evidence: campaign-recorded in agenttrace ROADMAP.md

### Stop the nightly from cancelling in-flight master runs
- id: `rm-179` | track: reliability | priority: 62.0 | status: candidate
- acceptance: schedule and push(master) use distinct concurrency groups, or cancel-in-progress is scoped to PR refs only; a nightly run can no longer cancel a master run's upload; the policy is stated in a workflow comment
- evidence: campaign-recorded in agenttrace ROADMAP.md

### Session-id scope flag across report modes
- id: `rm-238` | track: reliability | priority: 62.0 | status: candidate
- acceptance: a --session <id|path-substring> scope flag composes with the existing report modes and --format lanes; help documents it; a no-match run errors naming what was searched
- acceptance: Analysis accumulates a per-model token ledger keyed on per-event model_used; session cost is the sum over models of lookup_price(model) x that model's tokens; report --overview, JSON output, and the TUI session detail view render a per-model breakdown (model, tokens in/out, rate source, cost); provenance.cost becomes a per-model enumeration or an explicit mixed-model marker; a golden test pins a two-model fixture's expected per-model costs; the sqlite_sessions.rs multi-model acknowledgment test is updated to assert the new exact behavior
- acceptance: Every report footer, JSON output, and TUI about/overview surface embeds the pricing snapshot date, model count, catalog content hash, and the override-file fingerprint when AGENTTRACE_PRICING_FILE is active; two artifacts priced against different snapshots are distinguishable without repo access; JSON changes are additive only; a test pins the stamp format
- acceptance: Release tarballs and installers are cosign-signed; the build workflow emits SLSA provenance attestations; an SBOM ships with each release (cargo-auditable or syft); install.sh and npm/scripts/install.js verify a signature rather than only the sidecar hash; README documents verification including a Windows path; verification failure refuses install
- acceptance: The explorer's session index (filtered/sorted views, project resolution) is materialized into a SQLite sidecar keyed by session path + mtime; TUI launch appends/updates incrementally instead of enumerating and parsing every file; explorer_indices() remains the in-memory shape over indexed rows; comparator project resolutions are cached; the schema carries a version field for migration; full suite green
- acceptance: `agenttrace upstream status` compares the running build against upstream main, the GitHub release channel, and the npm package - commits ahead/behind, PR-level delta grouped by area (parser/diagnostics/CI), advisory drift, distribution-channel state; offline by default with an explicit fetch flag; output schema documented and stable for scripting
- acceptance: `agenttrace watch [paths]` tails session files on append, re-analyzes only the changed file incrementally, and re-renders the live overview/sessions view (cost so far, waste signals, loop detection); implementation stays file-tail re-analysis of written events and never stream interception, honoring the recorded non-goal; cost stays linear in changed files via rm-023's index or a per-file cache; the notify dependency is evaluated against the static-build profile
- acceptance: Sessions with no parseable timestamp are excluded from health-trend window bucketing (or bucketed into an explicit unknown bucket); direction and regression verdicts no longer change when untimestamped sessions are present; a golden test pins a mixed fixture with and without timestamps
- acceptance: Width-aware truncation and padding (unicode-width) for label and name columns in text reports; CJK content renders within the budgeted width; parity with the TUI's existing unicode-width approach; golden tests pin CJK-label fixtures in the affected columns
- evidence: campaign-recorded in agenttrace ROADMAP.md

### Disclose present-but-zero usage blocks instead of counting them as truth
- id: `rm-408` | track: reliability | priority: 62.0 | status: in_progress
- acceptance: ONE stated rule for present-and-recognized-but-all-zero usage, not per-format accidents — EITHER count it with a provenance flag naming the affected-event count (extending the rm-054 provenance taxonomy) OR route it through the existing text-estimate fallback WITH disclosure; a contrast fixture pair pins the absent case (→ estimated_from_text) against the present-zero case (→ the new contract); no silent $0/0 undercount — a corpus of present-zero sessions surfaces the affected share in doctor and JSON, never reports clean zeros silently
- evidence: campaign-recorded in agenttrace ROADMAP.md

### Read-only MCP server over the session cache
- id: `rm-455` | track: reliability | priority: 62.0 | status: in_progress
- acceptance: `agenttrace mcp` exposes read-only tools (session list/detail, spend summaries, doctor verdict) over stdio via rmcp; no write surface and no network; local-only transport documented; tests pin tool schemas; docs page includes a `claude mcp add` example
- evidence: campaign-recorded in agenttrace ROADMAP.md

### Meta-only sessions must not render an empty agent bucket
- id: `rm-490` | track: reliability | priority: 62.0 | status: in_progress
- acceptance: session-level source_tool falls back to the detected format/family when no event carries one; by_agent never renders an empty name (falls back to the family label, e.g. 'codex', or a named 'unknown' bucket); regression fixture = the pure token_usage_record codex journal (and the copilot metrics-only shape, composing with rm-485)
- evidence: campaign-recorded in agenttrace ROADMAP.md

### pi 1.0.x cacheWrite1h and the reported cost dict vanish in oh_my_pi_usage
- id: `rm-499` | track: reliability | priority: 62.0 | status: candidate
- acceptance: oh_my_pi_usage reads cacheWrite1h (own canonical class, or folded into cache_write with a disclosure counter — decision recorded at implement); forward-pinning red-first: a synthetic fixture with cacheWrite1h>0 either counts it (disclosed) or discloses the drop via counter — silent zero is the defect class; pi's reported cost dict gets a consume-or-disclose decision (provenance note when agenttrace's computed cost diverges from a non-zero reported cost)
- evidence: campaign-recorded in agenttrace ROADMAP.md

### `agenttrace update` self-updater + distribution divergence decisions (upstream #301)
- id: `rm-509` | track: reliability | priority: 62.0 | status: in_progress
- acceptance: update subcommand is network-opt-in under the rm-404 disclosure family (no silent network), verifies the release checksum before swap (rm-234 pins), swaps atomically (temp+rename, safe while running), refuses or redirects package-manager installs with a pointer to the owning manager; the WinGet/armv7 divergence decided-and-documented either way (align by dropping, or keep-and-document as deliberate fork policy in README/docs); CHANGELOG entry under Unreleased at landing
- evidence: campaign-recorded in agenttrace ROADMAP.md

### Disclose unparseable JSONL lines in format parsers
- id: `rm-526` | track: reliability | priority: 62.0 | status: in_progress
- acceptance: format parsers count parse-failed lines per file (thread a counters map the way parse_codex_rollout_jsonl already threads its own) and feed data_health.line_skips; a torn-tail fixture (truncated mid-object last line) discloses N unparseable lines in --doctor and JSON provenance and adjusts data_health confidence per the disclosure policy; boundary — rm-406 owns unrecognized KIND arms inside successfully-parsed lines, this row owns the JSON-PARSE-FAILURE channel upstream of them, and the codex custom-tools sibling row owns codex wire shapes; torn-write class = the same hazard rm-250 hardened the WRITERS against
- evidence: campaign-recorded in agenttrace ROADMAP.md

### Align OTel emission to the dedicated GenAI semconv repo — and emit reasoning tokens
- id: `rm-546` | track: reliability | priority: 62.0 | status: candidate
- acceptance: cache attrs emit the semconv names (`gen_ai.usage.cache_read.input_tokens`, `gen_ai.usage.cache_write.input_tokens`) with the legacy agenttrace.session.* names retained one deprecation cycle (documented in CHANGELOG per the rm-301-family additive policy — attribute rename is a consumer-visible contract change, so it must not be silent); NEW: `gen_ai.usage.reasoning.output_tokens` emitted whenever the session recorded reasoning output (data already tracked as reasoning blocks); a contract test pins the exact emitted attr-name set against a golden OTLP-JSON document; docs table maps every emitted key to its semconv definition and notes what remains custom; modality-scoped variants recorded as an explicit non-goal or implemented with per-modality fixtures
- evidence: campaign-recorded in agenttrace ROADMAP.md

### Date-windowed pricing — historical sessions at the rates in force at session time
- id: `rm-571` | track: reliability | priority: 62.0 | status: candidate
- acceptance: when the catalog source provides dated rate information, a session is priced at the rates in force at the session's own date and the pricing stamp/note states when historical rates were applied vs a current-rate fallback; when no dated information exists current behavior is preserved with an explicit 'priced at current rates' note (no silent retroactive repricing); --audit reflects the same basis
- evidence: campaign-recorded in agenttrace ROADMAP.md

### Shareable usage card output (deterministic SVG, no network)
- id: `rm-576` | track: reliability | priority: 62.0 | status: in_progress
- acceptance: a --card (or report -f svg) surface renders a static SVG usage card (period total, per-model/per-project bars, recent-days sparkline, session/call counts) from the same aggregation pass as --overview; output is byte-deterministic for a fixed corpus+range (golden test); every session-derived string embedded in the SVG is XML-escaped under the rm-540 CSV-guard discipline (no raw <, &, ", control bytes); zero network at render time; period/theme controlled by documented flags
- evidence: campaign-recorded in agenttrace ROADMAP.md

### Loop-waste dollar figures are pricing-independent constants wearing a $ label
- id: `rm-754` | track: reliability | priority: 62.0 | status: in_progress
- acceptance: per-retry and per-consecutive dollar figures derive from the session's own price table (rates × the loop's token mass) OR render with an explicit synthetic label in BOTH the governance recommendation text and the diagnostics JSON field naming; severity_from_cost tiers either consume real dollars or are disclosed as count-based heuristics; golden fixture pins a known-rate loop (mini-class vs opus-class rates diverge by the real ratio, not constants); red-first: on a mini-class fixture the current constants produce an opus-class-magnitude figure
- evidence: campaign-recorded in agenttrace ROADMAP.md

### rm-784
- id: `rm-784` | track: reliability | priority: 62.0 | status: in_progress
- acceptance: EITHER `-o` refuses a path whose parent does not exist with an error naming the missing directory, OR the mkdir -p behavior is documented in `--help` and README including the /-rooted-path EACCES case; decision recorded on this row before implement; test pins the chosen behavior
- evidence: campaign-recorded in agenttrace ROADMAP.md

### Repo-identity project grouping — one row per git repository across worktrees, clones, deletions
- id: `rm-832` | track: reliability | priority: 62.0 | status: candidate
- acceptance: a repo-identity grouping mode resolves each transcript's project dir to its git repository (toplevel/origin, with a disclosed fallback for non-repo dirs) and collapses N worktrees/clones of one repository into one project row; deleted-sibling clones stop double-counting; flag-off default preserves current by_project output byte-for-byte
- evidence: campaign-recorded in agenttrace ROADMAP.md

### Plan-tier (subscription) model pricing needs a second source — models.dev carries plan-scoped rates LiteLLM lacks
- id: `rm-845` | track: reliability | priority: 62.0 | status: candidate
- acceptance: (a) the snapshot schema gains a plan-scope class with per-entry provenance (litellm | models.dev); (b) a models.dev ingestion arm unions entries into the snapshot per the rm-588 never-drop contract — add-only merge, provenance-tagged, no key clobber without a recorded rate mutation; (c) plan-included entries (cost 0) report cost 0 with a plan-tier disclosure — never fallback estimation — and per-token plan entries price exactly; (d) a golden fixture pins the four swept entries end-to-end (--sessions cost + disclosure phrase); (e) full suite green
- evidence: campaign-recorded in agenttrace ROADMAP.md

### rm-891
- id: `rm-891` | track: reliability | priority: 62.0 | status: in_progress
- acceptance: per-field merge on re-emission — the union of thinking blocks survives across snapshots for the same message id (or an explicit disclosed-drop counter, never silent); non-monotonic fixture (thinking dropped in the final re-emission) pinning blocks=1 / chars>0; existing monotonic-growth fixtures unchanged; empty tool_use_id results keyed by (message_id, ordinal) or disclosed; clippy -D warnings + fmt rc0.
- evidence: campaign-recorded in agenttrace ROADMAP.md

### Reject --range without a report action loudly
- id: `rm-244` | track: reliability | priority: 61.0 | status: in_progress
- acceptance: --range without a report action exits non-zero with a message naming the flag and the actions it composes with, consistent with the other applicability rejections; help updated; a test pins rc and message
- evidence: campaign-recorded in agenttrace ROADMAP.md

### Add an `agenttrace gate` CI exit-code capability riding the post-#286 triage model
- id: `rm-019` | track: reliability | priority: 60.0 | status: candidate
- acceptance: `agenttrace gate --max-cost USD --max-fail-rate R <session-dir>` exits 0/1 listing the offending sessions, thresholds documented and defaulting to the post-#286 triage constants; golden tests cover pass/fail boundary cases; parked until rm-012 lands so the gate rides the stable triage model
- evidence: campaign-recorded in agenttrace ROADMAP.md

### Opt-in live-quota percentage from the user's own credentials (rm-201 rider)
- id: `rm-302` | track: reliability | priority: 60.0 | status: candidate
- acceptance: an opt-in, default-off surface (e.g. `agenttrace quota --live`) reports current 5h/weekly utilization percentage fetched via Anthropic's Usage API using credentials the user already holds (env var / credentials file / keychain per platform); tokens are never persisted and never sent anywhere but the API host; any failure degrades offline to rm-201's journal-derived numbers with a provenance field distinguishing live vs derived; a hermetic test pins the request shape and the offline fallback (no network in CI)
- evidence: campaign-recorded in agenttrace ROADMAP.md

### mtime prefilter for range-bounded discovery (byte-identical output)
- id: `rm-418` | track: reliability | priority: 60.0 | status: candidate
- acceptance: when a range lower bound exists, files whose stat (or cached) mtime precedes the bound minus a documented slack are skipped pre-parse; ranged output stays byte-identical on a golden corpus; unknown-start sessions living in skipped files are still counted (cache or bounded fallback) per the N7 comment at discovery.rs:347; data_health discloses the skip count; before/after latency recorded on a large corpus
- evidence: campaign-recorded in agenttrace ROADMAP.md

### Idle-aware hanging-anomaly classification — wall-clock idle must not read as agent latency
- id: `rm-422` | track: reliability | priority: 60.0 | status: candidate
- acceptance: gap-based anomalies separate idle (user-absent wall-clock) from latency (in-flight spans such as tool-call→tool-result pairs): "hanging" fires only on in-flight spans, or its detail names the idle window explicitly; a weekend-idle fixture yields no HIGH anomaly (or a distinct idle class); out-of-order timestamps surface as their own diagnostic, never folded into latency; golden fixtures pin both arms
- evidence: campaign-recorded in agenttrace ROADMAP.md

### CLI errors print one line — anyhow cause chains are discarded at the handler
- id: `rm-610` | track: reliability | priority: 60.0 | status: in_progress
- acceptance: the top-level handler prints the full chain (Debug format or explicit chain walk — e.g. 'Error: writing report output file: Is a directory (os error 21)'); ENOSPC vs EISDIR vs permission-denied produce distinguishable stderr through the SAME context site; exit codes unchanged; a pinning test drives two failure kinds through one context site and asserts distinct cause suffixes; no new disclosure beyond io error kinds
- evidence: campaign-recorded in agenttrace ROADMAP.md

### Single-pass budget view and a real first-writer contract test
- id: `rm-684` | track: reliability | priority: 60.0 | status: in_progress
- acceptance: render_budget_view performs exactly ONE journal read+aggregate shared by both output arms (existing budget-view pins stay green; a counting wrapper or single-read refactor proves the count); runtime_config test rewritten to pin the contract unconditionally — a second set() returns Ok(false) with the FIRST values persisting, asserted in any test order (not gated on installation order)
- evidence: campaign-recorded in agenttrace ROADMAP.md

### Read-only storage-footprint report
- id: `rm-692` | track: reliability | priority: 60.0 | status: in_progress
- acceptance: a documented read-only command (e.g. agenttrace footprint / --footprint) walks the cache roots and reports per-artifact-class file counts, bytes, and oldest/newest, in table and stable JSON; mutates nothing; honors every cache-root env (compose with the rm-575 lane); schema pinned by a golden test
- evidence: campaign-recorded in agenttrace ROADMAP.md

### rm-786
- id: `rm-786` | track: reliability | priority: 60.0 | status: in_progress
- acceptance: `--help` and README document that a bare numeric --cost/--health value filters cost/health >= value (Gte), with the =v, >v, <v forms shown; a doc-examples gate or unit test asserts the documented forms parse as written
- evidence: campaign-recorded in agenttrace ROADMAP.md

### Waste surfaces spend rm-754's basis contract — priced and synthetic loop dollars mix unlabelled
- id: `rm-857` | track: reliability | priority: 60.0 | status: in_progress
- acceptance: every dollar figure the waste surface renders or serializes carries the same basis contract rm-754 landed on governance advice and diagnostics JSON — (1) waste.rs summary/action strings tag synthetic figures the way governance advice does (or the mixed total decomposes into priced + synthetic lines); (2) agenttrace.waste.v1 gains a cost_basis field mirroring LoopCost.cost_basis (additive; the implementing lane records the additive-field vs version-bump decision); (3) the TUI loop-analysis line renders the same marker the CLI governance advice emits; (4) docs/guides/governance-reports.md's waste.v1 field list gains the basis story; red-first: a synthetic-basis fixture fails on today's missing-basis JSON/text/TUI legs before the fix; text + JSON + TUI-presentation goldens updated in the same unit; full suite green; conductor validation digest recorded at the gates
- evidence: campaign-recorded in agenttrace ROADMAP.md

### Amp (Sourcegraph) session lane — fixture-first
- id: `rm-881` | track: reliability | priority: 60.0 | status: in_progress
- acceptance: fixture-first — annotated fixtures built from the ccusage adapter contract BEFORE any discovery code; discovery registers ~/.local/share/amp/threads; parser meters per-message usage by model; ledger credits surfaced as a disclosed side-channel (never silently added to token totals); absent install → zero sessions, no error
- evidence: campaign-recorded in agenttrace ROADMAP.md

### rm-903
- id: `rm-903` | track: reliability | priority: 60.0 | status: in_progress
- acceptance: recorded from agenttrace ROADMAP.md by campaign ingest
- evidence: campaign-recorded in agenttrace ROADMAP.md

### Shareable usage summary card export (deterministic SVG / single-file markdown)
- id: `rm-841` | track: reliability | priority: 59.0 | status: candidate
- acceptance: `agenttrace card -o usage.svg` (and/or `--format markdown`) renders a deterministic single-file summary (period, tokens, cost, top project/model) from the existing reports.rs/pricing surfaces; fully offline (no network path added); a golden-file test pins the SVG skeleton; README documents the flag
- evidence: campaign-recorded in agenttrace ROADMAP.md

### Upstream-status labels conflate absent-probe with registry failure and rank unknown authority as least-fresh
- id: `rm-052` | track: reliability | priority: 58.0 | status: candidate
- acceptance: npm channel state distinguishes curl-absent from HTTP failure from unparseable payload; authority ranking presents unknown explicitly rather than as least-fresh; JSON schema changes additive only
- evidence: campaign-recorded in agenttrace ROADMAP.md

### Manual republish lane for release channels
- id: `rm-180` | track: reliability | priority: 58.0 | status: candidate
- acceptance: a workflow_dispatch lane re-renders and republishes channels for an existing tag without re-tagging, gated on manual approval, logging what changed, with a dry-run mode; rm-156's digest pinning is preserved in the republish path
- evidence: campaign-recorded in agenttrace ROADMAP.md

### Wire the pytest suite into CI
- id: `rm-209` | track: reliability | priority: 58.0 | status: candidate
- acceptance: a CI lane (or a step in the existing test job) runs `python -m pytest scripts/fixtures -q` on every PR with a pinned Python version; the sqlite-version coupling of committed fixtures (header bytes 96-99, reconciled portable in conflict case 55e6e239) is asserted or documented so the lane is host-portable
- evidence: campaign-recorded in agenttrace ROADMAP.md

### Cache-root misconfiguration: a non-directory cache root must be loud, not silent
- id: `rm-343` | track: reliability | priority: 58.0 | status: candidate
- acceptance: a cache root that is not a directory (or cannot be created/used) emits at minimum a one-line stderr warning, and --doctor reports the cache as disabled-with-reason rather than absent; a fixture pins the warning and the doctor row
- evidence: campaign-recorded in agenttrace ROADMAP.md

### Unknown-event-kind census: disclose silently dropped transcript line classes
- id: `rm-406` | track: reliability | priority: 58.0 | status: candidate
- acceptance: per-session unrecognized-kind census (BTreeMap<String,usize>) surfaced in --doctor and JSON provenance; a fixture pinning an unknown kind keeps the disclosure honest; drift driver: CC 2.1.289's agent.spawn teammates + idle/waiting states land as new line classes — the census must name them rather than swallow; '0 unknown kinds == full known-coverage' documented
- evidence: campaign-recorded in agenttrace ROADMAP.md

### CI example runner-trust: untrusted pull_request revisions on a self-hosted runner
- id: `rm-417` | track: reliability | priority: 58.0 | status: candidate
- acceptance: the example defaults to a GitHub-hosted runner, or gates `pull_request` to same-repo/`workflow_dispatch` filters with a comment that `self-hosted` + fork PRs requires private-repos-only; a check script or review checklist pins the example against regressing to pull_request+self-hosted; boundary — this item owns the runner-trust arm (:4/:9/:15) only: rm-405 (LANDED done via 33cc9bb/30f531a — superseded this row's :4 trigger and :15 install arms at integration, see the signal correction) owned the same file's mutable action tags, unpinned-install checksum, and `|| true` self-defeating-gate arms, and sibling lane 6557b823's unlanded rm-415 owns repackaging the gate as a versioned composite action (merge by title at integrate); landed rm-195 owns channel identity and rm-017 owns install-source pinning — neither covers workload isolation
- evidence: campaign-recorded in agenttrace ROADMAP.md

### Property-based parser testing for the untrusted wire formats
- id: `rm-452` | track: reliability | priority: 58.0 | status: in_progress
- acceptance: proptest (or equivalent) wired for the highest-risk invariants: the parser never panics on arbitrary bytes/JSON values (shrinkable failures), usage extraction is invariant under object key permutation, and detector lanes are mutually exclusive on adversarial mixed-key objects; bounded CI lane; any findings filed as follow-up rows
- evidence: campaign-recorded in agenttrace ROADMAP.md

### claude_code assistant string content must count as a turn
- id: `rm-491` | track: reliability | priority: 58.0 | status: candidate
- acceptance: assistant arm accepts plain-string content as a single text block (mirror of the user arm); golden fixture: assistant_turns increments with string content; array-content behavior unchanged
- evidence: campaign-recorded in agenttrace ROADMAP.md

### Add a charmbracelet/crush session-format lane (survey-first)
- id: `rm-494` | track: reliability | priority: 58.0 | status: candidate
- acceptance: SURVEY FIRST — the item's first deliverable is an on-disk format-contract fixture doc (versioned shape, usage fields, model ids, where sessions live) built from a real crush corpus, mirroring rm-423's pi tripwire pattern; only then a probe lane + parser arm with usage extraction and a golden fixture; walls: discovery stays path-based (no crush-specific special-casing outside the probe), and if the format proves unstable/private the adjudication is recorded and the item closed rather than shipping a brittle lane
- evidence: campaign-recorded in agenttrace ROADMAP.md

### Stdin session ingestion — `-` reads one session stream, no temp files
- id: `rm-503` | track: reliability | priority: 58.0 | status: in_progress
- acceptance: `-` (or an explicit --stdin alias documented as its synonym) reads exactly ONE session stream from stdin through the same parser path as file-backed sessions; cache bypass disclosed (stdin sessions are ephemeral — no session_cache write, or an opt-in documented); `-o` output paths work as for files; empty stdin fails with the existing "empty session" error class; a pinning test feeds a fixture via stdin and asserts byte-identical --overview output vs the same fixture passed as a file path; --help documents the form; boundary: multi-session discovery (-d) is OUT of scope — single stream only
- evidence: campaign-recorded in agenttrace ROADMAP.md

### Bound retained history growth
- id: `rm-527` | track: reliability | priority: 58.0 | status: candidate
- acceptance: retained history carries a documented size/count cap with oldest-first retention (reuse the statusline compaction pattern); truncation is disclosed — doctor's history stats name the cap and the retained count so silent data loss is impossible; a synthetic >cap history golden pins the cap and the disclosure; boundary — the statusline journal rows (rm-166 family) own the journal; this row owns history.json only
- evidence: campaign-recorded in agenttrace ROADMAP.md

### One cache-root env relocates every at-rest artifact
- id: `rm-575` | track: reliability | priority: 58.0 | status: candidate
- acceptance: ONE documented env (AGENTTRACE_SESSION_CACHE_DIR or an equivalently named cache-root var) relocates EVERY at-rest artifact the binary writes (session cache, pricing cache, statusline journal); AGENTTRACE_CACHE_DIR-style unknown vars get a --doctor warning or a documented rejection — never silent inertness; PRIVACY.md's artifact table and the env's help text stay in sync
- evidence: campaign-recorded in agenttrace ROADMAP.md

### Sweep skills/ CLI claims in the docs-commands gate
- id: `rm-598` | track: reliability | priority: 58.0 | status: in_progress
- acceptance: check-docs-commands.sh extracts agenttrace invocations from skills/*/SKILL.md and fails on unknown flags/commands exactly as it does for README/docs; a mutated scratch copy with a bogus flag trips the gate (red-green proof)
- evidence: campaign-recorded in agenttrace ROADMAP.md

### Port upstream #306 time-bucketed reports (--daily/--weekly/--monthly --tz --blocks)
- id: `rm-604` | track: reliability | priority: 58.0 | status: in_progress
- acceptance: (1) --daily/--weekly/--monthly produce bucketed overview output honoring --tz (UTC default); (2) 5-hour --blocks match upstream semantics; (3) requestless/journal-replay usage entries are deduped scoped-by-bucket so daily sums equal weekly/monthly sums over the same window (ccusage #1799 lesson — cite unlanded rm-549's divergence work); (4) format guards apply to the bucketed surfaces (no silent text fallback — unlanded rm-574 family).
- evidence: campaign-recorded in agenttrace ROADMAP.md

### rm-698
- id: `rm-698` | track: reliability | priority: 58.0 | status: candidate
- acceptance: cache save is guarded (lockfile or merge-on-conflict) so two concurrent scans converge instead of clobbering; documented concurrency contract
- evidence: campaign-recorded in agenttrace ROADMAP.md

### Watch: gemini-cli v0.63.0+ reworks the transcript layer on the fork-solo Gemini lane
- id: `rm-715` | track: reliability | priority: 58.0 | status: candidate
- acceptance: (a) IF a run lands gemini-cli (re)adoption, its parser baselines against v0.63.0+ fixtures (delta-patched history, collision-safe filenames, windowed transcripts); (b) discovery filename patterns accept the new collision-suffix scheme; (c) watch trigger: revisit this row when gemini-cli ships a format-breaking release
- evidence: campaign-recorded in agenttrace ROADMAP.md

### Event serde aliases accept only PascalCase model_used (generic-lane attribution drops to "default")
- id: `rm-718` | track: reliability | priority: 58.0 | status: in_progress
- acceptance: alias lists extended (or a generic-lane-only case-normalization layer; format-specific lanes byte-unchanged) so the PoC fixture reports the carried model name with catalog-or-unknown pricing provenance, never "default"; red-first golden test pinning the lowercase fixture; the same sweep covers or explicitly defers the F4 cachedContentTokenCount→cache_read alias gap (still present at 1c5edd1)
- evidence: campaign-recorded in agenttrace ROADMAP.md

### Plan/quota awareness — a user-declared plan drives plan-usage % and month-pace
- id: `rm-777` | track: reliability | priority: 58.0 | status: in_progress
- acceptance: a user-declared plan (config file + flag override, never auto-detected) feeds plan-usage % and month-pace projections into --overview across all four formats (text/markdown/html/json); with no plan configured the output stays byte-identical to today (locked by an A/B test); live quota reading from signed-in tools is OUT of scope unless a strictly read-only source is identified and the decision recorded in the lane; golden fixtures pin pace math at month boundaries and partial days; README contract row added
- evidence: campaign-recorded in agenttrace ROADMAP.md

### Claude server_tool_use (web_search/web_fetch) is an unpriced cost dimension
- id: `rm-793` | track: reliability | priority: 58.0 | status: in_progress
- acceptance: (1) pricing schema gains optional per-tool-request rates (web_search_request, web_fetch_request) with unpriced tool use DISCLOSED, never silently $0; (2) the claude lane extracts server_tool_use request counts into the usage record; (3) fixture: a session with 3 web_search blocks prices base+3×rate when rated and discloses "unpriced tool use: web_search ×3" when not; (4) snapshot/schema version bump rides the change per session-cache discipline
- evidence: campaign-recorded in agenttrace ROADMAP.md

### --demo --preserve-history writes fiction into the user's history.json
- id: `rm-836` | track: reliability | priority: 58.0 | status: in_progress
- acceptance: --demo never appends to history.json (composition test: --demo --preserve-history on a fresh HOME asserts history.json unchanged or absent); --preserve-history still records real -d scans; README documents --demo as side-effect-free.
- evidence: campaign-recorded in agenttrace ROADMAP.md

### Session cwd is first-wins — mid-session project switches misattribute cost
- id: `rm-859` | track: reliability | priority: 58.0 | status: in_progress
- acceptance: per-event cwd captured where the journal carries it (claude_code per-line cwd; oh_my_pi per-header), and the attribution surface consumes a RECORDED policy — dominant-cwd, last-cwd, or split accounting — with the decision written into the implementing lane; a golden fixture with a mid-session cwd switch (session opens in worktree A, later calls carry worktree B) pins the chosen policy's attribution; single-cwd sessions render byte-identical to today (A/B pin); the README attribution-contract sentence and docs/guides/governance-reports.md account for the policy; full suite green; conductor validation digest recorded at the gates
- acceptance: control bytes (C0/C1 minus the documented LF/CR/TAB allowlist) are neutralized at ONE choke point — the output-dispatch boundary covering stdout AND every -o file write — for every non-JSON format lane; JSON lanes stay escaped-by-design and are excluded with a comment saying why; a format×report MATRIX test drives the poisoned-name corpus (OSC-52 in BOTH its BEL- and ST-terminated forms, CSI clear, BEL, DEL, C1 CSI) through every render-bearing lane — {overview, sessions, audit, recommend, diagnostics, mcp-governance, context-trends, delivery-evidence, waste, compare, statusline-report} × {text, markdown, md, html, csv} — and asserts zero 0x1b/0x07 output bytes including -o file writes. CORRECTION (review fix, supersedes the original mint wording): `-f tsv` on --sessions is NOT a CLI value (clap rejects it, rc 2; tab-separated output is reached only through --delivery-evidence), so the original `tsv` matrix column was an over-spec — the matrix now pins that rejection instead of pretending a lane exists; and the `upstream` host lane, while it renders through the same choke point, takes environment-derived bytes (repo name, remote URL, ahead/behind counts) rather than transcript-derived ones, so it is pinned by a sanitized-site test rather than a poisoned corpus cell. The sanitizer is idempotent and tab-honest: 0x09 is layout for the document sanitizer and survives it, while the stricter line sanitizer (status-line cells) replaces it — pinned in BOTH composition orders; markdown_cell/html_escape keep their entity escapes as defense-in-depth with the :2833 merge note made true; the matrix test was RED on the assess corpus before the fix (implement-phase RED-FIRST record: overview markdown 10 ESC/6 BEL pre-fix → 0/0 post)
- evidence: campaign-recorded in agenttrace ROADMAP.md

### rm-892
- id: `rm-892` | track: reliability | priority: 58.0 | status: in_progress
- acceptance: a HashMap keyed by (path, source_tool, session_key) built once per load (O(n) total); key-links output byte-identical over the subagent_attribution fixture corpus; a scale test (e.g. 2,000 synthetic sessions x 200 children) completes within the suite's existing budget; clippy -D warnings + fmt rc0.
- evidence: campaign-recorded in agenttrace ROADMAP.md

### --budget trailing window counts sampled days, not calendar days
- id: `rm-917` | track: reliability | priority: 58.0 | status: in_progress
- acceptance: the budget window is CALENDAR-anchored to the trailing 7 calendar days ending today (zero-fill missing days, today's partial day included, empty-window -0.0 guard from review 5b9a9470 preserved); a journal with zero samples inside the calendar window renders the existing "(no cost samples in the window)" line and never drives an OVER/remaining verdict off stale samples; goldens pin (1) the sparse 13-day-span PoC journal → exactly 7 calendar dates, stale days excluded from the sum with staleness disclosed, (2) a dense recent journal unchanged, (3) the 30-day statusline-report arm anchoring identically; README's budget sentences updated if they name the semantics; full suite green; conductor validation digest recorded at the gates
- evidence: campaign-recorded in agenttrace ROADMAP.md

### Statusline journal read path is uncapped and one invalid UTF-8 byte wedges insights + retention permanently
- id: `rm-922` | track: reliability | priority: 58.0 | status: in_progress
- acceptance: (a) read path bounded (cap + truncate with a disclosed truncation flag in the stats lane); (b) invalid UTF-8 recovers — lossy per-line decode or quarantine-and-restart with a stderr note — so insights disclose rather than zero and retention resumes; (c) red-first tests: oversized planted journal (bound respected on read), bad-byte journal followed by a valid append (retention repairs, cap enforced again); (d) full suite green
- evidence: campaign-recorded in agenttrace ROADMAP.md

### Close check-locked-cargo.sh blind spots
- id: `rm-210` | track: reliability | priority: 56.0 | status: candidate
- acceptance: the gate scans *.yml and *.yaml, resolves wrapped invocations (line-continuation-aware or yaml-parsed), and either gates every dependency-resolving cargo verb (test/build/run/clippy/fetch/install/tree) or documents the exact accepted set; a synthetic wrapped-yaml violation fixture trips it
- evidence: campaign-recorded in agenttrace ROADMAP.md

### The OnceLock write-once contract has no honest regression net
- id: `rm-800` | track: reliability | priority: 56.0 | status: in_progress
- acceptance: the write-once contract is pinned by a test that cannot pass vacuously — EITHER resolution is extracted into a pure function over an injectable table trait (both writer orders unit-testable), OR the test runs in a fresh-process cargo target where installed_first is deterministic; the vacuous assertions are removed or replaced with assertions that can fail; red-first: a deliberate second-writer-wins mutation flips the new test red while the current test stays green (demonstrating the closed gap)
- evidence: campaign-recorded in agenttrace ROADMAP.md

### Open the agenttrace-core library lane
- id: `rm-161` | track: reliability | priority: 55.0 | status: candidate
- acceptance: publish = false dropped (or scoped to a stable subset crate); semver + API-stability policy documented in CONTRIBUTING; docs.rs renders the crate; the public API surface carries no TUI/runtime dependencies
- evidence: campaign-recorded in agenttrace ROADMAP.md

### pi recorded-cost precedence + turn semantics (decision item)
- id: `rm-253` | track: reliability | priority: 55.0 | status: candidate
- acceptance: a documented decision + implementation: recorded-vs-computed cost precedence for pi sessions with the chosen side surfaced via provenance (composes with rm-175's cost_provenance); queued-prompt turn semantics decided with a fixture; golden tests pin both decisions
- evidence: campaign-recorded in agenttrace ROADMAP.md

### Ship a CSV/statement export format (-f csv)
- id: `rm-409` | track: reliability | priority: 55.0 | status: in_progress
- acceptance: -f csv emits session rows plus the composable overview tables under a documented column contract in README; RFC 4180 quoting (quote-doubling, CRLF line endings) with every cell routed through ONE escaping helper (the markdown_cell lesson applied day one) — formula-injection hardening composes with the 278b2bda/6e96bed5 TSV/CSV-cell-injection lane rather than redefining it; byte-deterministic ordering with the deterministic-output gate extended to csv; the "markdown and html formats require --overview" bail (main.rs:35) extended coherently to csv's composable set; JSON documented as the lossless format, csv as a projection
- evidence: campaign-recorded in agenttrace ROADMAP.md

### Plan-aware effective-cost mode (subscription plans vs API rates)
- id: `rm-457` | track: reliability | priority: 55.0 | status: candidate
- acceptance: an optional plan configuration (plan type + renewal date) switches reports to effective-cost mode (quota burn + overage at API rate), clearly labeled, with API-rate totals still available; pricing-provenance labels distinguish plan-adjusted from catalog numbers (rm-176 discipline); tests pin both modes' math on a fixed corpus; no default behavior change
- evidence: campaign-recorded in agenttrace ROADMAP.md

### Bound pricing overrides against absurd-but-finite rates
- id: `rm-492` | track: reliability | priority: 55.0 | status: candidate
- acceptance: override validation rejects (or loudly discloses at load AND on the report surface) rates above a sane per-Mtok ceiling with a message naming the likely unit error; a 1e308-rate file never silently yields 1e+304 totals in any report; loud-rejection tests for negative (existing behavior pinned) and absurd-positive (new)
- evidence: campaign-recorded in agenttrace ROADMAP.md

### Lift lru 0.18.1 → ≥0.18.2 in Cargo.lock (RUSTSEC-2026-0253 in range, deny lane silent)
- id: `rm-552` | track: reliability | priority: 55.0 | status: in_progress
- acceptance: `cargo update -p lru` lands ≥0.18.2 (expected 0.18.5); the Cargo.lock diff moves ONLY lru (no unrelated crate churn); full workspace suite + cargo deny + fmt/clippy green on the bumped tree; the advisory range re-checked gone (OSV query or cargo audit) and recorded; CHANGELOG Unreleased line
- evidence: campaign-recorded in agenttrace ROADMAP.md

### TUI-ingested report flags must act or fail loudly — no silent no-ops
- id: `rm-572` | track: reliability | priority: 55.0 | status: candidate
- acceptance: every report-selection flag that cannot affect the TUI errors at parse time with a message naming the flag and why it is inapplicable (rc=2), OR is explicitly honored — no silent no-op; the loud-set matches the documented flag table in README/docs; --range keeps its current loud behavior
- evidence: campaign-recorded in agenttrace ROADMAP.md

### rm-654
- id: `rm-654` | track: reliability | priority: 55.0 | status: in_progress
- acceptance: recorded from agenttrace ROADMAP.md by campaign ingest
- evidence: campaign-recorded in agenttrace ROADMAP.md

### Codex session titles from session_index.jsonl
- id: `rm-691` | track: reliability | priority: 55.0 | status: in_progress
- acceptance: session_index.jsonl parsed alongside rollout transcripts; titles surface in --overview session lists and --search output where a name exists; a missing index file degrades silently (no error, empty title)
- evidence: campaign-recorded in agenttrace ROADMAP.md

### Aider lane silently drops DST-ambiguous session starts (unwrap_or_default on .single())
- id: `rm-899` | track: reliability | priority: 55.0 | status: in_progress
- acceptance: ambiguous local times resolve deterministically via LocalResult::earliest() (dated comment names the policy); a unit test pins a known fall-back-ambiguous local timestamp (repeat-Sunday 01:30 shape) resolving to the earlier arm, and a second asserts no valid aider timestamp yields a zero/empty start
- evidence: campaign-recorded in agenttrace ROADMAP.md

### Period-grouped usage reporting — calendar daily/weekly/monthly + rolling 5-hour blocks + explicit timezone (upstream #306 parity)
- id: `rm-916` | track: reliability | priority: 55.0 | status: in_progress
- acceptance: (a) period-grouped report surfaces (CLI + TUI) with calendar-grouped daily/weekly/monthly buckets AND rolling 5-hour block windows (Anthropic rate-window convention), timezone-explicit with DST-tested bucket boundaries (the landed rm-166/167/168 hour-bucket suite is the in-tree tz precedent), or a documented in-row rejection with reasoning; (b) the fixed 7d/30d windows keep their names — no silent month→30d aliasing in the new surfaces; (c) stable JSON shape pinned by a golden test; README documents the fixed-window vs calendar semantics
- evidence: campaign-recorded in agenttrace ROADMAP.md

### Disclose discovery partial failures instead of silently shrinking the corpus
- id: `rm-934` | track: reliability | priority: 55.0 | status: in_progress
- acceptance: an unreadable path is DISCLOSED at every layer — a stderr warning naming path + errno at discovery time, a --doctor provider/row note, and a warnings array in -f json (exit stays 0: the undercount becomes visible, the run is not killed); a regression fixture (chmod-000 subtree or an error-injecting wrapper) pins the warning emission AND the correct session count for the readable remainder; the live PoC flips from silent to disclosed
- evidence: campaign-recorded in agenttrace ROADMAP.md

### Small-hygiene bundle: TLS 1.2, npm timeout, git-root memo, map cardinality
- id: `rm-211` | track: reliability | priority: 54.0 | status: candidate
- acceptance: install.ps1 enables TLS 1.2 before the first request (parse-asserted); the npm installer bounds each download attempt (stub-server test); the git-root memo is invalidated or safely rekeyed on miss; usage maps carry a total-cardinality cap with oldest-eviction (overflow fixture test); each of the four fixes pinned by a targeted test
- evidence: campaign-recorded in agenttrace ROADMAP.md

### Pricing override file validation + loud failure
- id: `rm-305` | track: reliability | priority: 54.0 | status: candidate
- acceptance: override parsing rejects negative, non-finite, and unit-insane (per-token > 1e6) prices LOUDLY — rc1 naming the file, key, and offending value — instead of silently applying them; an unparseable or partially-typed override file fails loudly (or warns on stderr AND is flagged by --doctor) rather than no-oping; precedence (override > provider-reported > catalog) asserted by tests on a three-source fixture; a hostile-override regression fixture pins the loud rejection end to end
- evidence: campaign-recorded in agenttrace ROADMAP.md

### rm-702
- id: `rm-702` | track: reliability | priority: 54.0 | status: candidate
- acceptance: diagnostics severity mapping matches the in-tree rm-528-fixed semantics on the public API path
- evidence: campaign-recorded in agenttrace ROADMAP.md

### data_health confidence gates low on informational alias disclosures (corpus-wide)
- id: `rm-719` | track: reliability | priority: 54.0 | status: in_progress
- acceptance: confidence semantics pinned by golden tests: parse-loss counters (unparseable_line, non_object_line) keep degrading confidence; informational/disclosure counters never do — either migrate the alias family to disclosure_counters (preferred, single disclosure plane, rm-538 channel) or gate confidence on parse-loss counters only; kimi-alias corpus reports confidence high with the alias counters still visible in --doctor; rm-400's prevention rule (every alias match ships a counter) preserved and its tests updated to the new channel; composes with the unlanded sibling band's rm-710 (inverse flip: warm cache forgets line_skips → confidence wrongly high)
- evidence: campaign-recorded in agenttrace ROADMAP.md

### Documented limits surface — enumerate what transcripts can never tell us
- id: `rm-833` | track: reliability | priority: 54.0 | status: candidate
- acceptance: a LIMITS.md (or README section) enumerating known-invisible usage classes per agent format, each row citing the disclosure counter or first-hand absence that proves it; cross-referenced from --doctor output; kept honest by a docs gate that fails when a new disclosure counter lands without its LIMITS row
- evidence: campaign-recorded in agenttrace ROADMAP.md

### Docs/discovery small-hygiene bundle: zh README flag parity + case-insensitive session files
- id: `rm-390` | track: reliability | priority: 53.0 | status: candidate
- acceptance: a doc-coverage check pins flag-set equality between README.md and README.zh-CN.md (extending the rm-207 flag-documentation gate idea to both languages); session-file matching accepts .jsonl case-insensitively (or documents the exact contract) with a fixture pinning UPPER.JSONL discovery; both locked by tests
- evidence: campaign-recorded in agenttrace ROADMAP.md

### Versioned machine contract — schema_version on every -f json document
- id: `rm-504` | track: reliability | priority: 52.0 | status: candidate
- acceptance: every -f json renderer emits a top-level schema_version (e.g. "1"); a test pins presence on each document kind (overview / sessions / diagnostics / waste / upstream / list-models / baseline-compare); --baseline reads its artifact's schema_version and warns on mismatch instead of silently mis-reading; ci-integration.md states the additive-only policy with schema_version as its discriminator; bump discipline documented (additive = no bump; removal / rename / type-change = bump + CHANGELOG entry)
- evidence: campaign-recorded in agenttrace ROADMAP.md

### Governance slow-tool severity inversion
- id: `rm-528` | track: reliability | priority: 52.0 | status: in_progress
- acceptance: unmatched evidence alone cannot outrank measured latency — gate severity high on p95 ≥ threshold, or split the recommendation title so unmatched and measured rows never compete in the dedupe; a fixture pairing an unmatched-only session against a measured-slow session asserts the ordering; upstream reference: #296 "separate unmatched from timeouts" (merged into upstream master in the v0.10.x band) — adopt its taxonomy when the wave port lane lands; the rationale string already discloses the dual trigger, so this is a ranking defect, not a truthfulness one
- evidence: campaign-recorded in agenttrace ROADMAP.md

### upstream default ref should follow the remote's HEAD, not hardcode "master"
- id: `rm-559` | track: reliability | priority: 52.0 | status: candidate
- acceptance: the default ref derives from the remote (git symbolic-ref refs/remotes/upstream/HEAD, falling back to master only when HEAD is unset); the no-ref error names the detected HEAD and the UPSTREAM_REF knob; upstream --help (rm-505's route) documents the knob; regression: a repo whose upstream/HEAD → main resolves with zero configuration, and the knob still overrides everything
- evidence: campaign-recorded in agenttrace ROADMAP.md

### rm-579
- id: `rm-579` | track: reliability | priority: 52.0 | status: in_progress
- acceptance: with git absent from PATH, `agenttrace upstream` exits with a spawn-failure diagnosis naming the git binary — not a repo/remote/ref verdict — in both the human and -f json renderings; all three probes route Spawn distinctly from probe-run failures; regression pin = env -i PATH=/nonexistent ... upstream asserting the spawn-class error string and rc
- evidence: campaign-recorded in agenttrace ROADMAP.md

### rm-653
- id: `rm-653` | track: reliability | priority: 52.0 | status: in_progress
- acceptance: recorded from agenttrace ROADMAP.md by campaign ingest
- evidence: campaign-recorded in agenttrace ROADMAP.md

### Copilot usage fold: one collection arm, true max-per-field semantics
- id: `rm-687` | track: reliability | priority: 52.0 | status: in_progress
- acceptance: the two collection arms collapse to one shared helper keyed by event kind; per-model and per-field folding takes the max across re-emissions (rm-601 semantics) or documents replace-on-name as deliberate with a pinning fixture; comments match the implemented semantics
- evidence: campaign-recorded in agenttrace ROADMAP.md

### rm-699
- id: `rm-699` | track: reliability | priority: 52.0 | status: candidate
- acceptance: flat-lane events fold usage when present; corpus with usage-bearing flat transcripts reports nonzero totals
- evidence: campaign-recorded in agenttrace ROADMAP.md

### Codex `cache_write_input_tokens` and its context-window/credit riders are unhandled (latent)
- id: `rm-792` | track: reliability | priority: 52.0 | status: in_progress
- acceptance: (1) parser handles cache_write_input_tokens (cache-creation disclosure bucket or a distinct cache-write counter) with unknown usage keys provenance-disclosed per the existing disclosure family, never dropped; (2) fixture with non-zero cache_write_input_tokens pins extraction and proves no double-count against input_tokens; (3) riders dispositioned: model_context_window surfaced (context-utilization on report lanes) or recorded out-of-scope in-row; rate_limits.credits same; (4) all-zero live corpus behavior unchanged (no new noise)
- evidence: campaign-recorded in agenttrace ROADMAP.md

### Isolate the release-local gate script (shared CI_OUT wipe + hardcoded bin path)
- id: `rm-391` | track: reliability | priority: 51.0 | status: in_progress
- acceptance: the script writes only under its own unique subdirectory (mktemp-style) and honors AGENTTRACE_BIN when set, matching the other gates' contract; a sandbox test runs two gates concurrently and asserts both logs survive
- evidence: campaign-recorded in agenttrace ROADMAP.md

### Invoice true-up export (monthly per-model/provider cross-check)
- id: `rm-458` | track: reliability | priority: 50.0 | status: candidate
- acceptance: a monthly export (per provider x model: tokens, sessions, computed cost with pricing provenance) in a reconciliation-friendly format (CSV/JSON) with a stable column contract; a sample invoice cross-check documented end-to-end in docs; export is pure read, no network
- evidence: campaign-recorded in agenttrace ROADMAP.md

### "multiple"-model audit note misattribution after per-block pricing
- id: `rm-521` | track: reliability | priority: 50.0 | status: in_progress
- acceptance: the note branches on pricing source/provenance — sessions priced exactly per-block get an accurate label (e.g. "multiple models, priced per usage block; per-model totals exact") and only genuinely SQLite-aggregated sessions keep the legacy no-exact-price note; a golden fixture (the pi multi-model journal) asserts the corrected string; JSON/CSV/TUI render the same corrected text; boundary — rm-020 owns per-model LEDGER rendering; this item owns only the note's mechanism claim
- evidence: campaign-recorded in agenttrace ROADMAP.md

### rm-581
- id: `rm-581` | track: reliability | priority: 50.0 | status: candidate
- acceptance: decision-first row: capture a real 0.25.0 /export JSONL and a --json-file sidecar fixture, diff their wire shapes against the verified-local-history contract; if ingestion is feasible, qwen export files parse with correct per-model attribution plus a disclosure counter for shape drift; if not feasible, README/docs stop implying export coverage and the negative lands on this row
- evidence: campaign-recorded in agenttrace ROADMAP.md

### rm-701
- id: `rm-701` | track: reliability | priority: 50.0 | status: candidate
- acceptance: scope bounds derive from first/last observed events when session_start is absent, with a disclosed fallback marker
- evidence: campaign-recorded in agenttrace ROADMAP.md

### Slow-tool advice cites the first slow entry's p95, not the worst
- id: `rm-755` | track: reliability | priority: 50.0 | status: in_progress
- acceptance: the slow-tool recommendation selects the max-p95 entry among is_slow entries; fixture with two slow tools (max_sec leader p95 31s vs second entry p95 45s) pins the evidence citing 45s; red-first on the current first-match ordering
- evidence: campaign-recorded in agenttrace ROADMAP.md

### CHANGELOG section for every tag merged into HEAD
- id: `rm-303` | track: reliability | priority: 48.0 | status: in_progress
- acceptance: the v0.8.1 section is backfilled (or explicitly annotated inherited-upstream with rationale); the version gate (or a sibling check) fails when a ^v semver tag merged into HEAD has no CHANGELOG section, red-path proven on a synthetic missing-section fixture and green on the reconciled tree
- evidence: campaign-recorded in agenttrace ROADMAP.md

### New-agent-source coverage wave, Devin first (demand-gated)
- id: `rm-306` | track: reliability | priority: 48.0 | status: candidate
- acceptance: demand-gated — a source lands only after a dependent records need or a real corpus appears on a fleet host (the demand verdict is itself recorded evidence); one source per increment, Devin first: discovery root + parser + fixture corpus + README table row + source label in reports per increment; extends the landed rm-084 pi-family structural-discovery mechanism rather than growing a parallel registry
- evidence: campaign-recorded in agenttrace ROADMAP.md

### Hermes profiles: sqlite lane admits `~/.hermes/profiles/*/state.db` while doctor keys only the canonical path
- id: `rm-535` | track: reliability | priority: 48.0 | status: candidate
- acceptance: doctor provider rows enumerate every DB path actually loaded (profiles included, one row each or a count-bearing label); skip_sqlite_backed_file_dir symmetric with the load set (or the load set disclosed) so a -d walk over ~/.hermes cannot double-count; fixture with a synthetic profile state.db + sessions dir pins single-count and a truthful doctor row
- evidence: campaign-recorded in agenttrace ROADMAP.md

### Codex per-account dimension — multi-account spend cannot be split
- id: `rm-560` | track: reliability | priority: 48.0 | status: candidate
- acceptance: survey-first leg (rm-304's verify-first pattern): pin where codex account identity lives in the wild at the target base, recorded with a real or annotated-synthetic corpus; if proven — sessions carry account_id and --sessions/--overview break spend per account with zero behavior change for single-account users; if refuted — the item closes with the corpus and the verdict
- evidence: campaign-recorded in agenttrace ROADMAP.md

### upstream lane: honor -f json only (loud otherwise) and resolve the default ref symbolically
- id: `rm-574` | track: reliability | priority: 48.0 | status: candidate
- acceptance: `upstream` either honors -f json only and errors loudly on other formats or renders each supported format truthfully — never silently renders text; DEFAULT_REF resolution detects the remote's symbolic HEAD (refs/remotes/origin/HEAD) and falls back to 'master' with a note when unavailable; divergence verdicts name the ref actually used
- evidence: campaign-recorded in agenttrace ROADMAP.md

### Prometheus text-exposition output for ops stacks
- id: `rm-577` | track: reliability | priority: 48.0 | status: candidate
- acceptance: a -f prometheus (or --metrics) surface emits the Prometheus text exposition format (sessions_total, cost_usd_total, tokens_total broken down by model/window) from the same aggregation pass; metric and label values sanitized per the format spec (no raw newlines/quotes/backslashes in labels — rm-540 discipline); byte-deterministic for a fixed corpus; documented as one-shot scrape-to-file, NOT a daemon (the live-watch negative guard from the rm-385 family carries)
- acceptance: `agenttrace statusline --help` and `agenttrace upstream --help` print per-command help (usage, flags, sample line format and exit-path hints for statusline; flag semantics + the PRIVACY/network disclosure note for upstream) and exit 0; `-h` equals `--help`; an unknown flag after a keyword prints that keyword's usage with exit 2 (never the session-path error); pinning tests cover both keywords × {--help, -h, bad flag}; the root --help keyword descriptions match the per-command help text
- evidence: campaign-recorded in agenttrace ROADMAP.md

### rm-651
- id: `rm-651` | track: reliability | priority: 48.0 | status: in_progress
- acceptance: recorded from agenttrace ROADMAP.md by campaign ingest
- evidence: campaign-recorded in agenttrace ROADMAP.md

### Codex pre-Sept-2026 rollouts report zero usage with no distinct verdict
- id: `rm-716` | track: reliability | priority: 48.0 | status: in_progress
- acceptance: a codex rollout with zero buffered usage records emits a distinct disclosure ("rollout carries no usage rows" class) via line_skips or disclosure_counters plus provenance — usage is never fabricated; --doctor/--overview render the verdict; golden fixture from the #1405 envelope shape; a test pins that a post-Sept rollout WITH usage rows reports no verdict
- evidence: campaign-recorded in agenttrace ROADMAP.md

### Pricing alias resolution is user-override-only — glm-5.3 reports unpriced while priced siblings ship
- id: `rm-794` | track: reliability | priority: 48.0 | status: in_progress
- acceptance: (1) a built-in alias table resolves known family aliases (glm-5.3 → coding-glm-5.3 rates) with the fallback DISCLOSED in output ("priced via alias X"); (2) alias entries stay current across snapshot refreshes (curated table riding --update-pricing, or deterministic derivation from snapshot names); (3) fixture: a glm-5.3 session prices via alias with disclosure; a model with no alias and no price still discloses unpriced — no fabrication; (4) directly-priced models unchanged
- evidence: campaign-recorded in agenttrace ROADMAP.md

### TUI has no branch surface (no :branch filter key, no sort key, no detail field)
- id: `rm-900` | track: reliability | priority: 48.0 | status: candidate
- acceptance: TUI gains :branch <text> filter + branch sort key + a detail-view branch field riding the shared-filter architecture (the rm-389/784-786 dialect pattern); a TUI fixture filters by branch and slices the by-branch rollup; text/CLI surfaces unchanged
- evidence: campaign-recorded in agenttrace ROADMAP.md

### 0-byte session files: doctor "failed" vs data_health "skipped" — one disclosed class
- id: `rm-440` | track: reliability | priority: 46.0 | status: candidate
- acceptance: empty session files get ONE consistent, disclosed class (e.g. empty, distinct from parse-failed) in BOTH doctor and data_health; doctor's failed count no longer includes them; a fixture pins a 0-byte file next to a parseable one in both views; live host re-run: the only failure on a 21,186-session corpus disappears from failed and appears under the empty disclosure
- evidence: campaign-recorded in agenttrace ROADMAP.md

### rm-580
- id: `rm-580` | track: reliability | priority: 46.0 | status: candidate
- acceptance: concurrent runs on one cache dir converge without silent entry loss (lockfile or merge-on-write, chosen deliberately and recorded on the row); sweep_orphaned_temps never reaps a live writer's temp (pid-liveness check or writer-held marker); regression test = two interleaved save_sessions sequences asserting the union survives; sweep-bound stress pin
- evidence: campaign-recorded in agenttrace ROADMAP.md

### rm-652
- id: `rm-652` | track: reliability | priority: 46.0 | status: in_progress
- acceptance: recorded from agenttrace ROADMAP.md by campaign ingest
- evidence: campaign-recorded in agenttrace ROADMAP.md

### rm-700
- id: `rm-700` | track: reliability | priority: 46.0 | status: candidate
- acceptance: session reads are size-bounded (cap + disclosed skip counter) consistent with the discovery admission caps
- evidence: campaign-recorded in agenttrace ROADMAP.md

### Cache journal append lane races a planted symlink between inspection and reopen — O_NOFOLLOW the reopen
- id: `rm-843` | track: reliability | priority: 46.0 | status: candidate
- acceptance: (a) the reopen arm passes O_NOFOLLOW (custom_flags on unix) so a symlink at the path fails the open, surfaced through the existing loud 'refusing to append' InvalidInput family — never a raw ELOOP panic, never a silent follow; (b) a racing-plant test pins the fix: a fixture that passes the symlink_metadata inspection, plants a symlink, then reopens must observe refusal post-fix (red pre-fix); (c) the -o char-device create arm gets the same flag or a documented same-basis carve-out naming why char devices are exempt; (d) the non-unix arm documented unchanged; (e) full suite green
- evidence: campaign-recorded in agenttrace ROADMAP.md

### Torn-read hardening for the scanner
- id: `rm-254` | track: reliability | priority: 45.0 | status: candidate
- acceptance: on parse failure the scanner re-reads once when mtime changed during the read (or reads to end then verifies stable mtime before fingerprinting); a fixture that rewrites the file between stat and parse parses cleanly on the retry; the cache never fingerprints a file whose mtime moved during the read
- evidence: campaign-recorded in agenttrace ROADMAP.md

### Human-readable session titles (summary-line / first-user-message) for search, lists, reports
- id: `rm-407` | track: reliability | priority: 45.0 | status: candidate
- acceptance: session identity shows a human-readable title with the stem retained as the stable key (title from the Claude Code `type:"summary"` line when present, first-user-message fallback, deterministic truncation); title source disclosed in JSON; CONFIDENCE CAVEAT — zero `summary` lines exist in this host's 330-transcript census and none in repo testdata: pin the summary format from a compacted corpus BEFORE implementing (run 555a174d's N3 cleared find-by-name as a gap — this item is the naming layer, not a second search mechanism)
- evidence: campaign-recorded in agenttrace ROADMAP.md

### check-docs-commands.sh doctor leg must bound its runtime (HOME scan)
- id: `rm-444` | track: reliability | priority: 45.0 | status: in_progress
- acceptance: the doctor leg runs against an explicit `-d` fixture corpus (or equivalent scoped input) so the whole gate completes in bounded time (target <60s) with a debug binary; every documented flag stays exercised (the commands-coverage contract); gate stays green under the rm-369 bin default
- evidence: campaign-recorded in agenttrace ROADMAP.md

### Split parser.rs (6,077 lines) into per-format modules
- id: `rm-453` | track: reliability | priority: 45.0 | status: candidate
- acceptance: parser.rs decomposed into per-format (or format-family) child modules behind the existing public API (zero API churn, callers unchanged); test count identical pre/post; fmt + clippy -D warnings clean; the split is mechanical (no logic edits) verified by a review pass with zero non-move hunks
- evidence: campaign-recorded in agenttrace ROADMAP.md

### Shareable usage card: deterministic SVG for GitHub profiles and READMEs
- id: `rm-536` | track: reliability | priority: 45.0 | status: candidate
- acceptance: deterministic SVG (byte-stable under the check-deterministic-output discipline) rendering bounded usage totals through the same sanitized render choke point as reports (rides rm-239's helper, never raw strings); no host state consulted under --demo; README + zh parity docs (rm-390 section rule)
- evidence: campaign-recorded in agenttrace ROADMAP.md

### Claude Cowork discovery — a competitor-covered activity family we cannot see
- id: `rm-553` | track: reliability | priority: 45.0 | status: candidate
- acceptance: a discovery probe documents WHERE Cowork persists locally (macOS paths, schemas, cloud-export surface) with first-hand evidence, then dispositions: EITHER a tracked SOURCE arm with attribution + zero-usage contract (rm-408 family) + a discovery-contract test, OR a recorded defer with the probe results on this row so the next cycle does not re-research blind; title-disjoint from rm-488 (qwen detection) and the rm-400/401 provider arms
- evidence: campaign-recorded in agenttrace ROADMAP.md

### Claude Code now exposes session usage to MCP servers and hooks — a second ingestion class
- id: `rm-515` | track: reliability | priority: 44.0 | status: in_progress
- acceptance: DECISION-FIRST — a research note (spool) states whether CC hooks/MCP usage events carry data the disk transcripts lack (live pre-refresh totals, non-transcribed tool usage) and names the capture surface (SessionEnd hook payload or MCP resource) before ANY parser work; only if adopted: an ingestion arm (documented hook config or `agenttrace hooks` receive mode) that records usage events into the session cache without breaking offline-first defaults, with a golden fixture of a captured event stream and a schema-versioned event record
- evidence: campaign-recorded in agenttrace ROADMAP.md

### Cross-bucket totals must agree — daily == weekly == monthly invariant
- id: `rm-549` | track: reliability | priority: 44.0 | status: candidate
- acceptance: an invariant test asserts that daily, weekly, and monthly totals over the SAME corpus agree exactly (tokens, cost, session count) whenever those bucketings exist; a red fixture demonstrates the failure mode (an entry whose dedupe key differs across bucket boundaries — boundary-timestamp duplicate requestless entries) and the test would catch it; the invariant documented next to the bucketing implementation as a rule for future dedupe lanes
- evidence: campaign-recorded in agenttrace ROADMAP.md

### sessions.json read-modify-write has no inter-process lock (last-writer-wins whole-map)
- id: `rm-837` | track: reliability | priority: 44.0 | status: in_progress
- acceptance: save() takes an advisory lock (flock or lock file) around read-modify-write; a two-process interleaving test asserts both processes' rows survive; single-process path behavior unchanged and schema stays 32 (this is transport, not format).
- evidence: campaign-recorded in agenttrace ROADMAP.md

### Codex seen-totals ledger cap-clear re-counts a repeat cumulative total after 1024 distincts
- id: `rm-844` | track: reliability | priority: 44.0 | status: candidate
- acceptance: (a) red-first synthetic journal: ≥1025 distinct cumulative totals followed by a re-emission of total #1 carrying a fresh last-snapshot delta — session tokens exceed corpus truth by exactly the double-counted amount pre-fix, match truth post-fix; (b) the fix preserves rm-711's bounded-memory goal (evict-oldest ring / bounded window over last N totals — not an unbounded set, not a whole-clear); (c) rm-554's consecutive-duplicate rate-limit guard unaffected (it compares prev, not set membership); (d) full suite green
- evidence: campaign-recorded in agenttrace ROADMAP.md

### rm-893
- id: `rm-893` | track: reliability | priority: 44.0 | status: in_progress
- acceptance: each scan surfaces per-scan dropped-row counters through the existing disclosure channel (the rm-753 shape) or propagates the error; a hostile fixture with one malformed row mid-scan pins counter >= 1 AND the surviving rows' counts; clippy -D warnings + fmt rc0.
- evidence: campaign-recorded in agenttrace ROADMAP.md

### rm-582
- id: `rm-582` | track: reliability | priority: 42.0 | status: candidate
- acceptance: decide retire-or-keep with the evidence ON the row: if retire, one lane removes the adapter + README source entry + fixtures and pins a discovery_contract regression that gemini files fail loudly (never silently); if keep, record the demand evidence (fresh local gemini corpora since 2026-10-06) and the watch trigger that re-opens the decision (Antigravity CLI local artifacts appearing in host corpora)
- acceptance: span (and trace) id derivation can never emit an all-zero id (W3C/OTLP invalid — collectors drop or reject such spans); ids unique within an export by design rather than by accident of ordinals; golden tests assert non-zero AND determinism; SEMCONV_SNAPSHOT_DATE untouched; nothing outside id derivation changes in the renderer
- acceptance: gen_ai.system derives from metrics.source_tool (already parsed) with path markers only as a fallback for unknown families; the same journal exported from differently-named parent dirs produces byte-identical gen_ai.system; a fixture pins the source_tool↔system agreement; golden tests extended alongside rm-605's
- acceptance: sqlite-backed sessions dedup by (source_tool, session id) across all matched dbs, or the glob narrows to the canonical db with a loud warning listing ignored extras; red-first regression fixture: two dbs sharing session ids assert single-counted totals; --doctor discloses extra dbs; the repro is cold-cache (the snapshot cache must not mask the defect)
- acceptance: --doctor -d <nonexistent> exits rc=2 with the same guidance shape as overview/sessions; --doctor -d <file> exits non-zero with a directory-required error (or documents why doctor alone accepts files); overview/sessions behavior unchanged; entrypoint tests cover both doctor -d shapes red-first
- acceptance: --overview (text + -f json) gains a month-pace line: month-to-date cost, linear projection to month end, and an optional budget threshold (flag or env) with over/under indicator; math pinned by a leap/month-length-aware fixture; derived purely from existing aggregates, no new storage
- evidence: campaign-recorded in agenttrace ROADMAP.md

### update-snapshot.sh executes its fetch+rewrite on ANY argv (--help silently rewrote the tracked snapshot)
- id: `rm-901` | track: reliability | priority: 41.0 | status: candidate
- acceptance: --help/-h/usage print usage and exit rc0 with ZERO side effects (no fetch, no write); a --check mode reports upstream-vs-vendored drift (today 3,743 vs 3,099) without writing anything; the default no-arg refresh lane is unchanged; a smoke check (or docs-gate hook) pins the no-side-effect guarantee
- evidence: campaign-recorded in agenttrace ROADMAP.md

### Run-acceptance (trust disposition) ledger — watch item, promotion-gated
- id: `rm-090` | track: reliability | priority: 40.0 | status: candidate
- acceptance: promotion gate — a second independent demand signal must be recorded before any implementation spend; if promoted: `agenttrace --attest <session> --verdict ok|reject --note ...` appends to a local journal reusing the statusline journal mechanics and the terminal-sanitization rules already hardened this cycle, and --delivery-evidence plus governance surfaces render recorded dispositions with reviewer and timestamp
- evidence: campaign-recorded in agenttrace ROADMAP.md

### Competitive watch: ccusage now ships pi and hermes adapters reading this fork's home corpus
- id: `rm-344` | track: reliability | priority: 40.0 | status: candidate
- acceptance: watch-only — no implementation; re-verify each research pass; escalation trigger is a concrete capability gap appearing in their pi/hermes coverage that our users ask for (e.g. their named-store config model or pricingOverrides UX), at which point a parity/counter item is minted with evidence
- evidence: campaign-recorded in agenttrace ROADMAP.md

### check-docs-commands.sh must be hermetic (stale /tmp/agenttrace default + fixed /tmp captures)
- id: `rm-369` | track: reliability | priority: 40.0 | status: in_progress
- acceptance: bin resolution defaults to the repo's target/release/agenttrace (or the script fails loudly when neither it nor AGENTTRACE_BIN is present); every artifact the script writes lands under out_dir (mktemp-derived names under AGENTTRACE_CI_OUT for the two captures); a run with a poisoned /tmp/agenttrace still resolves the correct binary and produces correct gate results
- evidence: campaign-recorded in agenttrace ROADMAP.md

### Make TUI keybindings configurable (documented override file)
- id: `rm-410` | track: reliability | priority: 40.0 | status: candidate
- acceptance: a documented key-override file (e.g. ~/.config/agenttrace/keybindings.toml) remaps TUI actions; absent file = byte-identical behavior pinned by a test; invalid overrides fail loudly with named keys, never silently ignored; the Go-parity default keymap stays the baseline (the tests.rs:1078 pin passes unchanged)
- evidence: campaign-recorded in agenttrace ROADMAP.md

### Keyword commands need a help route — statusline/upstream mislabel --help as a dropped flag
- id: `rm-505` | track: reliability | priority: 40.0 | status: in_progress
- acceptance: recorded from agenttrace ROADMAP.md by campaign ingest
- evidence: campaign-recorded in agenttrace ROADMAP.md

### Decide CLI localization: adopt upstream's rust-i18n en/zh layer or record the decline
- id: `rm-759` | track: reliability | priority: 40.0 | status: candidate
- acceptance: DECISION-GATED — record the decision on this row with sources: (adopt) locale files + rust-i18n wiring land with the #297-299 wave port plus a zh golden render test; or (decline) the rationale (user base, maintenance cost) is recorded so the fork-maintenance merge preflight can DROP locale files deliberately instead of conflict-resolving them blind; either arm resolves the i18n line of the wave-port hazard map (dossier R8)
- evidence: campaign-recorded in agenttrace ROADMAP.md

### Vendor-stored vs recomputed cost divergence has no oracle or diagnostic
- id: `rm-795` | track: reliability | priority: 40.0 | status: in_progress
- acceptance: (1) a divergence diagnostic (doctor arm or report flag) lists sessions/files where |vendor − recomputed| exceeds a stated threshold, showing both numbers and each side's pricing basis; (2) threshold and aggregation documented in-row; (3) fixture with a known-divergent vendor cost appears in the diagnostic with both values; (4) existing outputs' PREFERENCE unchanged (diagnostic only — any policy flip is a later row)
- evidence: campaign-recorded in agenttrace ROADMAP.md

### Doctor's sqlite3 repair hint interpolates a raw path into a copy-paste shell command
- id: `rm-858` | track: reliability | priority: 40.0 | status: in_progress
- acceptance: the rendered repair hint shell-quotes the interpolated path (POSIX single-quote wrapping with '\'' escaping for embedded quotes, or equivalent lossless quoting) so the pasted command runs verbatim for paths containing spaces, single quotes, and shell metacharacters; a unit fixture pins the rendered string for a space-containing and a quote-containing path (red on today's raw interpolation); no other doctor output changes; if a Windows paste posture is wanted the implementing lane records that decision; full suite green; conductor validation digest recorded at the gates
- evidence: campaign-recorded in agenttrace ROADMAP.md

### MCP stdio server reads unbounded input lines while the statusline host caps at 1 MiB — same-host asymmetry
- id: `rm-923` | track: reliability | priority: 40.0 | status: in_progress
- acceptance: (a) bounded per-message read with a JSON-RPC error response past the cap (mirror the statusline host's disclosed-refusal pattern); (b) red-first test: an over-cap single line gets the error, not unbounded buffering; (c) if the MCP guide documents transport limits it names the cap (check-docs gate stays green); (d) full suite green
- evidence: campaign-recorded in agenttrace ROADMAP.md

### CI step-summary output lane — per-PR agent spend inside the checks UI
- id: `rm-938` | track: reliability | priority: 40.0 | status: candidate
- acceptance: `-f gha` renders the existing overview/sessions report as GitHub step-summary markdown (## headings, GFM tables, no raw HTML) on stdout, appending to $GITHUB_STEP_SUMMARY when the env var is set (one code path, env-gated); the lane joins the CU3 stderr-announce purity set; a golden fixture pins the rendered bytes; README gains a CI usage snippet; SCHEDULE BEHIND demand evidence (a real user running agents in CI) per the capability-gate discipline — do not implement speculatively
- evidence: campaign-recorded in agenttrace ROADMAP.md

### Windows installer parity — PSReadLine-proof arch detection and the CMD decommission
- id: `rm-550` | track: reliability | priority: 38.0 | status: candidate
- acceptance: arch resolution routes through a single resolver with the PSReadLine-shadow fallback (mirroring #314) unit-testable in isolation; the CMD installer is removed or loudly documented unsupported with its deprecation in CHANGELOG (mirroring #315); version pinning on Windows matches install.sh's pinned-REF semantics; verification documented for Linux-only hosts (pwsh -NoProfile parse check where pwsh exists, else a recorded manual step)
- evidence: campaign-recorded in agenttrace ROADMAP.md

### Dependency bump wave (upstream #309) — hygiene only, no advisories open
- id: `rm-561` | track: reliability | priority: 38.0 | status: in_progress
- acceptance: bumps land behind the full suite green; rusqlite 0.32→0.40 spans 8 minors (bundler/API churn is the risk surface — session-cache schema tests must stay green with ZERO schema bump); crossterm 0.29 checked against the TUI pty smoke lane; zero behavior changes expected — any behavior change found is a defect to fix before landing
- evidence: campaign-recorded in agenttrace ROADMAP.md

### TUI report-context underline sizes by UTF-8 bytes, not display width
- id: `rm-611` | track: reliability | priority: 36.0 | status: in_progress
- acceptance: underline length = UnicodeWidthStr::width(raw_title) at both sites (unicode-width is already in the workspace via the core column-width lane); a pinning test with a CJK + emoji-mixed title asserts dash count == display width; pure-ASCII titles render byte-identical before/after
- evidence: campaign-recorded in agenttrace ROADMAP.md

### CherryStudio SQLite usage-ledger provider (tier-3, demand-gated)
- id: `rm-717` | track: reliability | priority: 36.0 | status: candidate
- acceptance: demand-gated — first a demand census leg (install-base evidence: GitHub stars/issues, tokscale/ccusage references, user reports) recorded on this row; only if demand clears does a provider arm land: explicit -d/--provider discovery of the CherryStudio home DB, read-only opens, golden fixtures over the ledger tables, disclosure counters for unread/undecodable rows, no auto-discovery before the gate clears; a declined-with-evidence decision recorded on this row closes it either way
- evidence: campaign-recorded in agenttrace ROADMAP.md

### presentation.rs is a 3,899-line rendering monolith — split before it calcifies
- id: `rm-838` | track: reliability | priority: 36.0 | status: in_progress
- acceptance: split into per-report modules under presentation/ (overview.rs, sessions.rs, …) behind a facade; every CLI report path byte-identical over the repo's landed fixture corpus; clippy -D warnings + fmt clean; no public API change.
- evidence: campaign-recorded in agenttrace ROADMAP.md

### Custom-dir walks must not admit npm/package manifests as session candidates
- id: `rm-370` | track: reliability | priority: 35.0 | status: in_progress
- acceptance: the custom-dir walk extends the dir skip-list (npm and cache-style agent-dir children) and/or adds a small filename blocklist (package-lock.json, other *.lock, models-store.json); doctor failure_samples on the ~/.pi corpus contain no manifest paths — fixture-equivalent: a dir holding package-lock.json + models-store.json + one real session yields exactly 1 parseable session and 0 manifest failures; auto-discovery results byte-identical before/after
- evidence: campaign-recorded in agenttrace ROADMAP.md

### Report filters: --exclude and --session-id
- id: `rm-589` | track: reliability | priority: 35.0 | status: candidate
- acceptance: (1) --exclude inverts every existing filter lane (project/source/model) across overview/sessions/waste/audit; (2) --session-id selects a single session by id across formats; (3) docs-gate covers both flags.
- evidence: campaign-recorded in agenttrace ROADMAP.md

### De-duplicate the cfg(unix)/cfg(not(unix)) admit_session_path bodies
- id: `rm-733` | track: reliability | priority: 35.0 | status: in_progress
- acceptance: the body is written once, platform-generic, with only the special-file classifier behind cfg (per the repo's own pairing convention at doctor.rs:718/724 and session_cache.rs ×5); a shimmed non-unix cargo check (x86_64-pc-windows-gnu) stays green and unix behavior is pinned by the existing bin-suite tests; zero behavioral change
- evidence: campaign-recorded in agenttrace ROADMAP.md

### Tests execute only on Linux while release ships darwin/windows binaries
- id: `rm-501` | track: reliability | priority: 34.0 | status: candidate
- acceptance: EITHER at least one non-Linux lane (windows-latest first — both windows assets already build there natively) runs the full workspace suite in ci.yml, OR the workflow file carries an explicit comment + job name stating 'build+smoke only; the suite executes on Linux only' so coverage claims stop implying cross-OS correctness; if the test lane lands, a red-first canary (deliberate cfg(windows)-only compile break on a scratch branch) proves the lane catches it before the canary is dropped; suite duration on the new lane measured and recorded
- evidence: campaign-recorded in agenttrace ROADMAP.md

### Crash-truncated journal tails are classified as generic unparseable lines
- id: `rm-796` | track: reliability | priority: 34.0 | status: in_progress
- acceptance: (1) a final-line partial JSON record (unterminated object / EOF without terminal newline) classifies as truncated-tail, distinct from generic unparseable_line; (2) doctor reports truncated_tails separately from unparseable_line; (3) fixture: valid session + truncated last line ingests all complete records, discloses 1 truncated tail, 0 generic failures; (4) mid-file bad lines keep the generic classification (real corruption never masked)
- evidence: campaign-recorded in agenttrace ROADMAP.md

### Config parser truncates unquoted values at an embedded #
- id: `rm-918` | track: reliability | priority: 32.0 | status: in_progress
- acceptance: the implementing lane decides and records ONE of: (a) `#` opens a comment only at line start or after whitespace — an embedded `#` stays part of a bare value; or (b) values containing `#` must be quoted and an unquoted bare scalar containing `#` is rejected LOUDLY (file:line + the truncated-vs-written value) per the loud-failure promise; goldens pin an embedded-`#` config end-to-end through --doctor/pricing resolution (accepted value under (a), rejection message under (b)); full suite green; conductor validation digest recorded at the gates
- evidence: campaign-recorded in agenttrace ROADMAP.md

### rm-894
- id: `rm-894` | track: reliability | priority: 30.0 | status: in_progress
- acceptance: one truth — either correct the help sentence to match behavior (range applies to every report) or make corpus actions deliberately ignore since and say so; the chosen semantics pinned in the docs-commands gate (check-docs-commands.sh rc0); --overview's own help updated if the semantics change.
- evidence: campaign-recorded in agenttrace ROADMAP.md

### Codex Flex usage accounting
- id: `rm-927` | track: reliability | priority: 4.0 | status: in_progress
- acceptance: (1) flex-flagged codex usage rows price with flex rates or a disclosed fallback note; (2) fixture with flex-marked rollout entries; (3) usage-basis documentation names the flex lane.
- evidence: campaign-recorded in agenttrace ROADMAP.md

### Qwen export and dual-output session surfaces
- id: `rm-928` | track: reliability | priority: 4.0 | status: in_progress
- acceptance: (1) qwen export-format sessions parse or are skipped-with-disclosure, never misattributed to the transcripts lane; (2) dual-output sessions do not double-count turns or tokens; (3) fixture from a qwen 0.25.0 export sample.
- evidence: campaign-recorded in agenttrace ROADMAP.md

### Duration rendering clamp and sentinel-duration disclosure
- id: `rm-932` | track: reliability | priority: 4.0 | status: in_progress
- acceptance: (1) fmt_duration gains day/year units or a documented cap; (2) corpus totals clamp or flag sentinel durations with a disclosure counter instead of propagating raw floats; (3) fixture with a 9999-12-31 session asserts a disclosed bounded rendering.
- evidence: campaign-recorded in agenttrace ROADMAP.md

### Length-tiered pricing representation (context-length rate tiers)
- id: `rm-924` | track: reliability | priority: 3.0 | status: in_progress
- acceptance: (1) pricing snapshot schema + convert_litellm retain per-tier rate sets keyed by context-length threshold; (2) pricing resolution emits the tier actually applied plus a carrier note; (3) drift-check keep-filter accounts tier fields instead of silently excluding them; (4) fixture pricing-snapshot with a tiered model (claude-haiku-5.5) asserts both rate sets and the >100K switch; (5) report lane shows the applied tier for tiered sessions.
- evidence: campaign-recorded in agenttrace ROADMAP.md

### Antigravity model resolution and unpriced-model fallback
- id: `rm-925` | track: reliability | priority: 3.0 | status: in_progress
- acceptance: (1) model resolution survives legacy .pb cascade directory shapes without panic; (2) placeholder model ids map or disclose, never silently mislabel; (3) unpriced app models get a disclosed priced-fallback basis (per-session disclosure, otel.rs pattern); (4) regression fixture with legacy-cascade + placeholder + unpriced corpus passes.
- evidence: campaign-recorded in agenttrace ROADMAP.md

### Copilot session-credits and modelMetrics coverage
- id: `rm-926` | track: reliability | priority: 3.0 | status: in_progress
- acceptance: (1) copilot session-state + otel lanes reconcile per-session usage when modelMetrics is partial, with a coverage disclosure; (2) session-credit rows surface as disclosed fallback when absent from modelMetrics; (3) copilot model catalog maps claude-haiku-5.5 and claude-opus-4.7 rates; (4) fixture with partial-coverage and credit-bearing sessions.
- evidence: campaign-recorded in agenttrace ROADMAP.md

### README zh-CN parity regeneration
- id: `rm-931` | track: reliability | priority: 3.0 | status: in_progress
- acceptance: (1) zh-CN regenerated to structural parity (headings, tables, flag rows) or explicitly marked machine-translated with a dated parity note; (2) docs CI or a dated parity line keeps EN↔zh drift visible; (3) verify leg: heading/flag-row counts within a stated tolerance.
- evidence: campaign-recorded in agenttrace ROADMAP.md

### Local-timezone day bucketing for budget and statusline series
- id: `rm-929` | track: reliability | priority: 2.0 | status: in_progress
- acceptance: (1) budget/statusline series and `today`-range dashboards share one day-bucket basis (local midnight) or a consistently documented UTC basis with disclosure in both surfaces; (2) fixture session at 23:30 local crossing UTC midnight lands wholly in one bucket; (3) basis stated in the series docs and tools_list copy.
- evidence: campaign-recorded in agenttrace ROADMAP.md

### Release-channel integrity (owner decision required)
- id: `rm-930` | track: reliability | priority: 1.0 | status: in_progress
- acceptance: (1) OWNER DECISION recorded in-row first: single canonical channel (re-point installers to origin and cut a release there, or adopt upstream as the release home with a documented sync lane) — channel ownership is a business call the prioritize phase must surface, not a code call; (2) installers and SECURITY.md advisory routing point at the decided channel; (3) check-install-ref-drift gains a freshness leg (published latest ≥ changelog newest within a bound) or a documented exemption; (4) CHANGELOG backfills v0.9.1/v0.10.x; (5) verify leg: fresh `curl|sh` install of the decided channel lands a binary whose `--version` matches the changelog.
- evidence: campaign-recorded in agenttrace ROADMAP.md

## Closed items

- `rm-656` rm-656 — done
- `rm-583` Upstream fetch timeout can still hang when a grandchild holds the pipes — done
- `rm-584` Codex journals: two silent-drop holes in the disclosure contract — done
- `rm-585` Spend by branch / worktree from the unused gitBranch wire field — done
- `rm-587` Deny lane allows unsound advisories: "advisories ok" is a false clean — done
- `rm-588` Pricing snapshot refresh must never drop a model (union contract) — done
- `rm-590` Positional path lane folds not-a-regular-file into does-not-exist — done
- `rm-596` Gate the doctor demo lane onto the demo corpus the sqlite lane already uses — done
- `rm-688` --latest tiebreak memoizes file stats — no stat storm on legacy corpora — done
- `rm-693` rm-693 — done
- `rm-697` rm-697 — done
- `rm-703` rm-703 — done
- `rm-704` rm-704 — done
- `rm-720` Antigravity usage and cache-read extraction, standalone-app model pricing — done
- `rm-721` Copilot agent-host request-class counting audit (seat-vs-metered follow-through) — done
- `rm-730` rm-730 — done
- `rm-880` Codex 0.160.1 persists identity, lineage, and quota wire the fork drops unread — done
- `rm-778` Journal-derived cwd drives an unbounded O(n²) ancestor walk — cheap hostile-journal DoS, warm cache included — done
- `rm-779` --project filter is an undocumented case-insensitive substring over id+display_name+root — done
- `rm-831` Antigravity fold arithmetic is not hostile-value safe — plain += wraps and negatives ride unfiltered — done

<!-- managed by hermes-roadmap render; do not edit by hand -->
