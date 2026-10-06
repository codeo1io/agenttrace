# Changelog

## Unreleased

### Fixed

- CSV statement export no longer emits hostile transcript-derived cells raw (rm-540, rm-409 residual): the rm-409 export fed transcript-derived group names straight into statement cells, so a model string carrying a raw OSC-52 clipboard-write sequence (`ESC ] 52 ; c ; pwn BEL`) landed verbatim in a by_model row — clipboard-write bytes that terminals act on — and a tab-prefixed `=HYPERLINK(...)` or ` =SUM(9+9)` payload rode past the formula guard, which matched only the literal first character (`chars().next()`), so tab- and space-prefixed formulas passed (the tab arm additionally defeats spreadsheet autodetect). Every statement cell now routes through `sanitize_line_segment` at the render boundary (control bytes become U+FFFD; composition sanitize, guard, then quote; idempotent for the already-sanitized sessions row) and `guard_formula` judges the first non-whitespace character, with the numeric-cell exemption re-based on `trim().parse::<f64>()` so padded numbers stay bare. CRLF row-forgery containment proven at assessment is retained and strengthened (a CRLF payload now lands as one quoted single-line cell). Pinned by unit plus end-to-end hostile-corpus regression tests, and the e2e suite now sandboxes `HOME`/`XDG_CACHE_HOME`/`AGENTTRACE_SESSION_CACHE_DIR` per test thread so it can never race the operator's real `~/.cache/agenttrace` or write fixture entries into it (same-batch reliability rider rm-444 likewise scopes the docs gate's `--doctor` leg off operator HOME).
- Context utilization divides by the model's real vendor window (rm-231): the denominator was guessed from model-name substrings (`claude` → 200k, unknown models silently 131k), so 1M-context models — the current Claude default class — reported utilization roughly 5x too high and tripped critical-pressure findings on healthy sessions. The pricing snapshot builder now carries LiteLLM's `max_input_tokens` (the source file is literally named `model_prices_and_context_window.json`; the field used to be the one thing the trim dropped), diagnostics resolves the window per session model from the active catalog, and models the catalog cannot resolve take the documented fallback ladder with the estimate disclosed as `window_source: "fallback"` instead of silently asserted. The bundled snapshot was refreshed from 2026-09-13 (2,755 models) to 2026-10-04 (3,099). The refresh retired upstream's first-party `claude-sonnet-4` row, which would have silently reattributed the demo corpus's Claude session to an openrouter gateway row (identical rates, wrong vendor lane), so the demo session now names `claude-sonnet-4-5` — the current first-party row, byte-identical rates, every demo cost unchanged.
- Cached per-session costs re-price when the pricing catalog changes (rm-196): the session cache's freshness fingerprint was only file mtime+size, so a snapshot bump on upgrade or a `--update-pricing` refresh left every cached per-session cost priced at the old catalog's rates. The cache journal and SQLite snapshot now stamp a 64-bit content-identity digest of the active catalog (stable across `cache`/`cache(stale)` relabeling); a stamped id that disagrees with the live catalog drops the entries exactly once so they re-price, an unstamped legacy journal is accepted as-is and stamped at the next save (upgrading alone never forces a rescan), and identical catalogs keep every hit. The override-swap leg is pinned end to end by `tests/warm_cache_pricing.rs` (landed at the run-27253dd5 integration): swapping `AGENTTRACE_PRICING_FILE` without touching the session artifact re-prices both the cost and the provenance label in both directions, and pure cache hits leave the cache journal bytes untouched.
- Markdown reports no longer pass transcript-derived HTML through verbatim (rm-403): `markdown_cell` escaped only `|` and newline, so a model or session string carrying `<img onerror=…>` / `<script>…` landed raw in `-f markdown` output, and every markdown renderer that allows inline HTML executed it (the same threat class as Claude Code 2.1.289's published-artifact `<script>` freezes). Plain cells now entity-escape `&`, `<`, `>` before the pipe/newline table escapes — mirroring the HTML arm's `html_escape` — with the policy documented at the function; the five raw Scope-row string fields (range, session-window timestamps, parse coverage, confidence) route through the same escape, while `markdown_inline_code` deliberately keeps code-span semantics (renderers escape code spans themselves, and pre-escaping would corrupt display). Pinned by a hostile-corpus regression suite (`tests/markdown_escape_contract.rs`: payloads arrive entity-escaped verbatim, `&`/`|`/newline stay display-faithful); the assess PoC corpus re-renders with zero raw `<img`/`<script>` tags. Boundary: the governance `--audit -f markdown` JSON fence is inert under CommonMark fence semantics (a mid-line triple backtick cannot close it) and is documented as a boundary rather than fixed.
- Every opt-in network touch is now disclosed truthfully, in PRIVACY.md and at the moment it happens (rm-404): PRIVACY.md's "the only exception is `--update-pricing`" claim was false — `agenttrace --fetch upstream` also touches the network (a `git fetch` against the configured upstream remote plus one HTTPS request to `registry.npmjs.org/@zack78%2fagenttrace/latest`, a live channel at 0.9.0). The paragraph now enumerates every opt-in touch with its trigger flag and exact URL, and the promise "announced on stderr at the moment it happens" is enforced in code: `--fetch upstream` prints a disclosure line built from the same constants the probe uses before any request goes out, and the `--update-pricing` download announcement moved from stdout to stderr so a redirected pipe cannot swallow it (review finding 1). Two constructor-derived pin tests (the rm-086 pattern) keep the document and the code from drifting apart.
- Copilot sessions report session-wide credit totals (cycle 1, rm-485): the Copilot CLI emits its real spend as cumulative session-scoped fields — `session.shutdown.totalNanoAiu` and freshest-per-model `session.usage_checkpoint` snapshots — that no per-event `modelMetrics` tally can reconstruct, so a resumed session that had already burned its credits reported `cost 0.00` (no modelMetrics rows at all) and a still-open session was invisible from the numbers. The parser now reads `totalNanoAiu` on shutdown and folds `session.usage_checkpoint` snapshots with MAX-semantics counters, so re-emitted checkpoints and a shutdown following a checkpoint never double-count (the ccusage #1824 reconciliation); `credit_usd` is reported per event and session alongside cost, and `cost` prices as `max(token estimate, credit)` with provenance `calculated_from_copilot_credits` so the reported number is never lower than what the agent actually paid (at integration the `max` folds over the rm-436/rm-438-refined token estimate — upstream-recorded costs priced once, per-block multi-model pricing — rather than replacing it). The session cache schema was bumped so previously cached sessions regenerate under the corrected totals (the rm-230 convention: a parser-semantics change that alters reported totals for unchanged files must invalidate warm caches, or the fix stays invisible behind stale entries) — landed as 25 → 26 at integration, re-basing the campaign's own 22 → 23 bump onto the ceiling already advanced by the zero-usage (23), pi-accounting (24) and workbuddy (25) landings (one invalidation either way) — and the governance guide's schema sentence, which `scripts/ci/check-docs-commands.sh` verifies against the live constant, was realigned at integration together with the test fixtures that pin the version (a regression test proves a stale-schema cache is dropped whole and rebuilt rather than served).
- Meta-only sessions attribute to their family (cycle 1, rm-490): Codex-family sessions whose only identifying record is a `session.meta` line (no rollout header, no message-level source marks) previously surfaced with an empty agent name, so `--overview`'s `by_agent` rows carried blank-name buckets that under-counted the family. The session-level `source_tool` fallback now attributes such sessions to their family (`Codex CLI`), pinned by a fixture that renders `by_agent` from a meta-only corpus.
- `--compare` honors the report gates (cycle 1, rm-486): `--fail-under-health` (and the sibling gate flags) were enforced on `--overview` and `--audit` but silently skipped on `--compare`, which exited 0 on a corpus a gated run would fail — the exact CI command the docs recommend for drift checks. Gate enforcement now routes through one shared `enforce_report_gates` used by the governance and compare branches alike (exit 2 with a single `Gate failed: …` stderr line), while ungated compares keep exit 0; pinned by an end-to-end entrypoints test sandboxed away from the operator's real session cache.
- Tool p95 and p95-gap now use the house percentile definition (cycle 3, rm-420): the tool-latency p95 in `--diagnostics` used nearest-rank (`ceil(len*0.95)-1`) semantics instead of the pinned `crate::percentile` (`trunc(len*p)` clamped), so at every sample size divisible by 20 the value read one rank low — a 19×2s+1×31s corpus reported `p95_sec=2.0` / `is_slow=false` beside `max_sec=31.0`, disarming the >30s slow-tool gate in `--diagnostics -f json` and the TUI's slow-tool filter while the same session's `--overview` latency p95 (already house semantics) disagreed. Both sites route through `crate::percentile` now (values were already sorted), and the percentile pin test scans `diagnostics.rs` alongside `reports.rs` and rejects any future inlined index arithmetic, so another local percentile copy cannot slip into the core crate unscanned (golden fixtures pin the 31s tail and the n=20 gap boundary). Scope note (review 21a6c6a5): three house-equivalent trunc copies remain in agenttrace-tui (presentation.rs, shared.rs) — unreachable while `percentile` is `pub(crate)` and claimed by twin lane rm-411 (run 6557b823, uncommitted), which routes them and widens the fn to `pub`; the core-crate pin is the cycle-3 scope.
- `trace_steps` durations no longer collapse to 0.0 beyond one hour (cycle 3, rm-004 residual arm): the per-step duration filter accepted only `0.0..3600.0` while `tool_latencies` (the landed rm-004 fix) keeps calls up to 24h, so a 2h job rendered as `dur=0.0` / `status=ok` with two-hour-apart timestamps in the same report that listed it at 7199s `is_slow=true`. Steps now accept the same 24h sanity bound, pinned by a fixture asserting steps and latency agree on one report.
- Flat-transcript sessions pair tool results with their calls (rm-230): the flat Claude-transcript parser arm dropped the `tool_use_id` → `tool_call_id` join key, so every tool call in that format was reported as `unmatched` and fed the high-severity latency-review filter even when its result was present. Explicit ids are now preserved verbatim on both sides and id-less entries pair positionally per tool, so a present result no longer counts as unmatched — and one tool's genuinely missing result is no longer masked by another tool's surplus result. The session cache schema was bumped (20 → 21) so previously cached sessions regenerate under the corrected pairing.
- Retry-loop detection keys on (tool name, argument identity) instead of the tool name alone (rm-233): a single assistant turn issuing several same-name calls with distinct arguments — the normal parallel batch — was misreported as a `*_loop` retry pattern and priced into loop waste. Arguments that differ only in key order still count as the same call, so reordered-key retries stay detected.
- The `install.sh` source-build fallback pins and verifies what it builds (extending rm-051, rm-235): `AGENTTRACE_SOURCE_REF` now also accepts a full commit id, which is fetched directly (a SHA cannot be resolved by `clone --branch`) and checked against the pin, `AGENTTRACE_SOURCE_URL` overrides the clone/fetch URL, an unresolvable pin refuses instead of building, and the install receipt records the exact commit built.
- A positional session path no longer hijacks explicit report actions (cycle 3, rm-246): the single-session report previously rendered whenever a path was present, so `agenttrace --overview --fail-under-health 100 session.jsonl` silently skipped the quality gate (exit 0) and `--search <term> session.jsonl` never searched. The default report now renders only when NO action is requested, so a positional path feeds `--overview`/`--search` like `-d` does — the gate fails identically in both forms (exit 2, `Gate failed: …`), pinned by an end-to-end test asserting an identical stderr verdict for the positional and `-d` delivery forms.
- Flags after the positional session path are a loud usage error instead of being silently dropped (cycle 3, rm-247; README §"Flags go before the session path" rewritten): the Go-flag shim stopped at the first positional and discarded the rest, so `session.jsonl -o out.txt` exited 0 without writing anything, `session.jsonl --clear-cache` no-op'ed, and gate flags after the path never reached the gate. Any flag-like token after the positional now exits 2 naming the dropped tail (`flag \`-o\` follows the positional session path and would be silently dropped …; place flags before the positional path`); plain extra positionals stay tolerated. A drift canary derives every value-taking flag from the clap command and asserts the shim's value-flag table covers each, so a future flag cannot silently move the truncation point. At integration the canary made its first catch: the landed `--dir` long form of `-d` (run d675a177, cycle-4 B2) was missing from the table, so `agenttrace --dir <path> <flags>` misread the directory as the positional and rejected the following flags with the wrong error; `--dir` now sits beside `-d` in the table and the value order is pinned end to end.
- `-o` report writes are atomic (cycle 3, rm-250): `write_output` stages content in a unique temp sibling (`<name>.tmp.<pid>.<seq>`, mirroring the session-cache and history writers) and renames into place, removing the temp on write or rename failure; a crash or Ctrl-C mid-write can no longer leave a truncated report at the destination, and the `Saved:` line prints only after a successful rename. Pinned by a test asserting correct content, a clean destination-is-directory failure, and zero `.tmp.` residue.
- The Go-flag shim no longer misregisters the boolean `--no-baseline-gate` as a value flag (pass-11 A11-5; cycle-4 review F2): it previously swallowed the next token and kept scanning, so the documented CI command `agenttrace --no-baseline-gate --baseline X --overview -f json` exited 1 with the misleading `Error: --baseline requires --overview -f json` and post-positional flags leaked past Go flag semantics. Leading and trailing placements now behave in both orders, pinned by shim unit tests and an end-to-end entrypoints test.
- Symlinked session directories are discovered (Codex `#42135`; Codex 0.153+ officially supports symlinked session roots): both discovery walks previously tested `entry.file_type().is_dir()`, which is false for symlinks, so sessions under a linked child directory were silently invisible. Links are resolved once and followed, a canonical-target visit set terminates cycles (self- and parent-links), stored directory listings from the pre-symlink walker are retired by a `dir_listing_version` bump (stale listings dropped exactly once, cache keys unchanged so dead-path pruning stays valid), and `--doctor` names followed links as `path (symlink -> target)` instead of implying a plain directory.
- Cache-bound accounting now sizes eviction over the deduplicated union of raw and decoded entries (pass-11 A11-2): a path present in both maps was counted twice, so near the 64 MiB byte bound the loop could evict up to ~2× the intended amount (near-total eviction as true size approached the bound) and the entry bound wasted drop slots on duplicate paths and under-dropped in one pass. Each path is now sized once (the larger of raw/decoded), both bounds reach their target in one pass, and headerless entries order oldest rather than sitting in the map as unevictable-or-misplaced (building on the F5-5 headerless-entry semantics, not reverting them).
- TUI delivery-panel worker failures render as diagnostics instead of empty states (pass-11 A11-4): the governance channel now carries a `Result`, worker panics are caught and reported (`delivery worker failed: …`), a dead worker sets `delivery worker exited without evidence` instead of looking identical to "no evidence", and the panel shows the message with a retry hint; a successful retry clears it. A retry (re-entering the panel) now also clears the stale error at spawn time, so the panel shows the scan in flight rather than the previous failure until the next poll (cycle-7 review F4).
- `agenttrace statusline` sanitizes payload strings before rendering (cycle-7 review F1): session and model names are user-authored, and control characters (newlines, CR, ESC/OSC/CSI, DEL, C1) previously passed through to the host terminal — breaking the one-line contract and enabling terminal-injection from a hostile chat title. They are now replaced with `U+FFFD`, pinned by a hostile-fixture test in both the session-name and model-name paths.
- `agenttrace statusline` no longer panics when stdout cannot be written (cycle-7 review F2): a closed pipe or full disk previously aborted with exit 101 (`failed printing to stdout`); the line is now written with the write error ignored (and stderr diagnostics use the same non-panicking form), so the exit-0-for-anything host contract holds for stdout conditions too, pinned by an end-to-end `/dev/full` test.
- `install.sh` now verifies the downloaded release asset against the `.sha256` sidecar the release workflow already publishes (pass-11 A11-3, hardened by rm-234): a sidecar that is missing, malformed, or mismatched aborts before anything is executed (naming expected and actual hashes on mismatch), as does a host with no sha256 tool — all three installer channels (`install.sh`, `install.ps1`, npm) now refuse identically instead of the POSIX channel warning and continuing, pinned by behavioral gates in `scripts/ci/check-install-runtime.sh`. The installed binary is forced to mode 0755 before the move, so a restrictive `umask` (verified under `077`) can no longer yield a group/other-unreadable binary the way `mktemp`'s 0600 plus `chmod +x` did.
- Per-format usage truthfulness: kimi_cli and Codex compaction turns stop reporting zero/under-counted tokens (rm-400/rm-401): kimi_cli `StatusUpdate.token_usage` arrives under wire keys (`input_other`, `output`, `input_cache_read`, `input_cache_creation`) the shared usage-alias table did not know, so every reported record was discarded by the empty→None gate and the session fell back to the text estimator (132 tokens on the official MoonshotAI/kimi-code fixture — shipped in-tree as a golden test — versus the reported 563,628). The aliases now map (canonical keys keep precedence and matching is numbers-based, so the bare `output` key cannot over-match a foreign string field), usage events keep wire arrival order (`insert(0)` removed), and every alias match is disclosed per session as `kimi_usage_alias:<key>` diagnostics counters, so a future key change surfaces as nonzero counters instead of silently-zero usage. Codex remote-compaction turns persist their provider usage as `token_usage_record` entries (and `compacted.payload.latest_token_usage_record`) that no cumulative `token_count` snapshot includes; the parser blanket-skipped `compacted` lines and never matched the record type, under-counting by the entire compaction turn (1,910 of 2,610 tokens on the research corpus). Records now pair with their compaction marker by `response_id` and count exactly once (the marker's embedded copy and the replayed top-level record are deduped and disclosed as `codex_compaction_usage_duplicate`), unpaired records stay visible as `codex_token_usage_record_unpaired` without being counted (their usage already lives in the snapshots), and counted usage never advances the rm-035/rm-162 high-water baseline. The rm-304 verify-first harness (`tests/codex_compaction_verdict.rs`) was flipped to the post-fix totals at integration — that inversion is the fix's regression gate. The session cache schema was bumped (21 → 22) at integration so previously cached kimi_cli and Codex sessions regenerate under the corrected accounting (the rm-230 convention: a parser-semantics change that alters reported totals for unchanged files must invalidate warm caches, or the fix stays silently invisible behind stale entries).
- Custom `-d`/`--dir` directory walks no longer admit npm/package manifests as session candidates (rm-370): the shared filename admission accepted `package-lock.json`, `models-store.json`, and any `*.lock`, and the custom-dir walk descended into `npm` roots, so `agenttrace -d <agent-home>` listed package manifests as sessions and `--doctor` rendered JSON parse failures for them (15 of 12,869 discovered files on the research corpus). `package-lock.json`, `models-store.json`, and `*.lock` are now denied for every walker and `npm` joins the directory skip-list beside `node_modules` — while the default discovery lane stays byte-identical (it registers `<root>/<child>/sessions` roots only, so it never meets a manifest), pinned by a test proving the deny-list is inert for the default lane and active for the custom walk. Warm journals are retired by a `dir_listing_version` bump (2 → 3, the cycle-7 symlink-walk convention): the cached replay extends a stored listing's `files` verbatim instead of re-running the admission predicates, and a v2 listing still names the manifests (and `npm` roots as children), so without the bump the blocklist would silently not apply on any host with a warm cache — the session-cache schema bump cannot cover it, because schema 22 already shipped with the pre-blocklist walker. Stale listings drop exactly once at load and regenerate; cache keys are unchanged, so dead-path pruning stays valid. Pinned red-first: a journal rewound to walk version 2 with a manifest in its listing keeps offering the manifest as a candidate session file (`discovered` grows and the manifest counts as skipped) until the bump retires the listing.
- The 21 → 22 session-cache schema bump's residual references were aligned at integration: the bump had moved only the constant in `session_cache.rs`, leaving three stale `21`s — the governance guide's "the session cache is schema 21" sentence (which `scripts/ci/check-docs-commands.sh` verifies against the live constant, so the gate was red), the TUI warm-cache fixture (its expected cache hit is impossible: a mismatched `schema_version` discards the whole cache), and two discovery-contract cache pins asserting `Some(21)`. All now read 22, and the guide's version-history parenthetical records why the cache moved.
- Present-but-zero usage blocks are now counted and disclosed instead of silently dropped from accounting (rm-408): sessions gain a `zero_usage_events` counter and the usage provenance string gains a `zero_usage_reported:<N>` suffix on `provenance.tokens` (extending the rm-054 taxonomy), `--doctor` reports the zero-usage session/event counts in its JSON `zero_usage` block (with up to six sanitized samples) and in the text census, plus an affected-session-share recommendation, over the discovered session-transcript census (SQLite-backed agent databases are not part of this census yet), the session cache round-trips the new field — the schema was bumped 22 → 23 at review fix because a warm pre-rm-408 entry still parsed as fresh while the `#[serde(default)]` counter read back as 0, reporting clean zeros for exactly the historical sessions the disclosure exists to flag (the rm-230 convention) — and the new CSV export carries a `zero_usage_events` column. The TUI data-quality/inspect label composes with the extended string: `provenance_label` translates the taxonomy base behind a `+key:value` suffix, so a flagged session keeps its "recorded by the agent" label instead of degrading to "source unknown" exactly when the disclosure flags it (integration review decfa879).
- pi journals count `type:"usage"` entries — cache-warm spend is no longer invisible (cycle 3, rm-436): the pi parser's catch-all dropped every `type:"usage"` journal entry, so a cache-warm block of 50,000 tokens with a $0.015 recorded cost vanished from all totals (research PoC reported 150 tokens / $0.00 where the spec-true answer was 50,150 tokens / catalog + $0.015). Each usage entry now contributes its input/output/cacheRead/cacheWrite tokens to the session totals, is attributed to the entry's own provider/model, and its self-recorded cost passes through as recorded cost (blocks with a recorded cost are not re-priced by the catalog, and the pricing source discloses `+ recorded cost`); unknown `kind`s count under their own name per the upstream spec. Facts the parser deliberately does not count — skipped entry types (`label`, `thinking_level_change`, …) and message roles without an accounting arm — surface as disclosure counters (`pi_usage_entry:<kind>`, `pi_entry_skipped:<type>`, `pi_message_role:<role>`) aggregated corpus-wide in `data_health`, `--doctor`, and every overview format instead of silently vanishing; clean corpora render byte-identically (skip-if-empty). The session cache schema was bumped so previously cached sessions regenerate under the corrected accounting instead of silently serving pre-fix totals (review F1) — landed as 23 → 24 at integration, because the kimi_cli/Codex (22) and zero-usage (23) landings had already advanced the schema past the batch's own 21 → 22 bump, and recorded costs / disclosure counters are set by the pi parser only — a foreign JSONL line can no longer inject them (review F5). Counter keys are sanitized at mint (control bytes → `U+FFFD`), so a hostile `kind`/`type`/`role` can carry neither terminal escape sequences nor line breaks onto any report surface, and an entry whose only signal is its recorded cost (all-zero token classes) still passes that cost through instead of being silently dropped (review F-A/F-C).
- pi tree journals disclose their branch shape (cycle 3, rm-437 first cut): v2/v3 pi sessions form a tree (`id`/`parentId` per entry; sibling branches written by `/tree` and `/fork`), but the parser read every line linearly and counted all branches in spend and turns with no hint. Branch ends are now counted over body entries (the session header's root id excluded) and disclosed as `pi_branches:N` in `data_health`, `--doctor`, and the overviews whenever a journal has more than one; the spend still intentionally counts all branches — active-branch-only replay is deferred to a second cut — and id-less v1 journals never disclose.
- pi `model_change` reads the wire key `modelId` (cycle 3, rm-438): the handler matched `model`, but the upstream writer (`appendModelChange(provider, modelId)`) serializes `modelId`, so the arm never fired — after a model switch, every model-less assistant message kept the pre-switch model for attribution and pricing (research PoC priced 370 gpt-4o tokens as claude-sonnet-4-5). The switch now applies (`model` stays honored as a legacy spelling), so post-switch usage attributes to the new model, mixed sessions price per usage block, and the session reports its model as `multiple`. Per-block multi-model pricing is parser-agnostic: any journal family whose usage-bearing events carry model attribution (not only pi) now prices each block at its own model instead of billing the whole session at the last-seen model, and the mixed-model pricing source keeps the `+ recorded cost` suffix when journal-recorded spend contributed (review F2/F3). The rm-423 pi format-contract suite's known-divergence pin for the dead `model` handler was flipped at integration — by intent, not drift, as that suite's header doc requires — so the contract now pins `modelId` end to end (a model-less post-switch assistant message attributes to the switched model) with `model` kept as the legacy spelling.
- workbuddy sessions disclose the `input_tokens` basis instead of clamping a mismatch to a silent zero (rm-450, upstream luoyuctl/agenttrace#310): the workbuddy lane subtracts `cache_read_input_tokens` from `input_tokens` on the assumption the input is reported cache-inclusive, so a transcript whose input is already cache-exclusive clamped to `tokens_input=0` with no counter anywhere — the silent-zero class the usage-truthfulness lineage exists to prevent. Every subtraction now counts `workbuddy_input_basis:cache_subtracted`, and a subtraction that zeroes the input additionally counts `workbuddy_input_basis:zeroed_suspected_mismatch` (riding the pass-7 `Metrics.line_skips` channel, so the counters aggregate into `data_health`, cap confidence at `low`, and surface in the text, Markdown, HTML and JSON overviews), and the Dropped-lines cell appends the assumption's provenance note ("input basis assumed cache-inclusive; a zeroed input suggests the transcript reports a cache-exclusive basis"). The subtraction arithmetic is deliberately unchanged until upstream pins the true basis (watch on the row). Clean corpora render byte-identically; the session cache schema was bumped so warm entries regenerate and surface the disclosure — landed as 24 → 25 at integration, re-basing the campaign's own 22 → 23 bump onto the ceiling already advanced by the zero-usage (23) and pi-accounting (24) landings (the rm-230 convention; one invalidation either way).
- `agenttrace <keyword> --help` is a real help route, not a dropped flag (rm-505, extending the rm-247 dropped-flag guard): `statusline` and `upstream` dispatch off the positional PATH slot, so clap never rendered help for them and `agenttrace statusline --help` / `agenttrace upstream --help` / `-h` / `--version` all exited 2 as a "flag `--help` follows the positional session path" mislabel - even though `agenttrace --help` itself teaches both keywords. Both keywords now print per-command help (usage, the no-flags-after contract with pointers to `--statusline-report` and the `--fetch`/`-f json` examples, and the guide references `docs/guides/statusline-capture.md` / `docs/guides/upstream-status.md`) and exit 0; a bad flag after a keyword keeps the rc2 dropped-flag discipline but the error is keyword-scoped ("flag `--bogus-flag` follows the `statusline` keyword ... run `agenttrace statusline --help`") instead of calling the keyword a session path. Pinned by entrypoints tests: `keyword_host_commands_have_real_help_routes` (rc0 + `-h` parity for both keywords) and `keyword_bad_flag_error_names_the_keyword_not_a_session_path`.

### Added

- `--doctor` and the pricing-source report line disclose vendor-deprecated models (rm-419): the snapshot ingestion now carries LiteLLM's `deprecation_date` when the source has one (463 of 3,099 entries in the 2026-10-04 refresh), the pricing provenance line discloses how many priced models are past their vendor deprecation date, and a session priced on a model the vendor has already retired says `model deprecated YYYY-MM-DD (rate unverified)` at its rate provenance instead of silently billing at catalog rates. The judgment anchors to the catalog's own vintage (the bundled snapshot's pinned date, or a cached catalog's fetch date) — never the wall clock — so report bytes stay stable for a given catalog.
- pi-family journal format-contract suite (cycle 3, rm-423): the pi upstream (`@earendil-works/pi-coding-agent`) is a private repository that shipped 1.0.0→1.0.2 within four days, so journal-format drift has no release-notes early warning — a silent key change would degrade discovery, labeling, and usage attribution without failing a test. Golden fixtures (one derived-redacted from a real 1.0.2 v3 journal observed live, one per sniff arm: title-preamble/versionless header, `parentSession` header) pin the header key set, a known-journal-version gate (`1|2|3`) that names the `version` key on drift, both usage alias families (`input/cacheRead/cacheWrite/totalTokens` and `input_tokens/cache_creation_input_tokens/...`), message-level model attribution, and the `model_change` wire key (`modelId`) end to end. Known-divergence pins (the dead `model` handler, documented; system-entry routing — `branch_summary`/`compaction` surfacing as assistant turns — pinned by a fixture assertion added at review) are recorded in the suite so those fixes must update the contract deliberately, not by drift.
- Statusline capture (research pass 9 candidate 53): `agenttrace statusline` is a host command for Claude Code's `statusLine` hook that renders the one-line status and tees the raw payload to a bounded local journal (`~/.cache/agenttrace/statusline.jsonl`, 10 MiB cap, newest-whole-lines compaction via temp-file rename, `AGENTTRACE_SESSION_CACHE_DIR` honored, torn lines skipped on read). It never fails the host — valid payload, malformed JSON, and empty stdin all exit 0 with exactly one stdout line, diagnostics to stderr only, stdin bounded at 1 MiB. `--statusline-report` (text and JSON) turns the journal into subscription limit-pressure windows (`5h`/`7d` usage, `resets_at` crossings evidenced by observations on both sides), deduplicated per-session prompt-cache analytics (`hit_ratio`, misses, `miss_causes` such as `tools_changed`), and peaks; `--doctor` reports the journal's health and the TUI Efficiency panel gains a "Subscription limits" block. Captures are deduplicated by exact payload with the count disclosed. Fixtures are schema-faithful to the documented payload contract, not recordings of a real host.
- `--doctor` discloses the offline pricing snapshot's date, model count, and age ("Pricing snapshot: LiteLLM snapshot 2026-10-04 (bundled, 3099 models, N days old)"), and the bundled snapshot was refreshed from 2026-09-02 (2,458 chat models) to 2026-09-13 (2,755) and again to 2026-10-04 (3,099).
- `docs/guides/statusline-capture.md` documents the host contract, journal schema and retention, and report semantics.
- `-f csv` statement export (rm-409): `--overview` and `--sessions` render CSV with per-format column sets (RFC-4180 CRLF rows, quote-doubling escaping, formula-injection guard); the sessions header carries `zero_usage_events`; the format is listed in `--help`; README documents the column contract; the deterministic-output gate covers the csv renders; and `-f csv` outside that composable set bails loudly (`csv format requires --overview or --sessions`, matching the markdown/html guard) instead of silently rendering the text report (review fix).
- Seeded property-invariant harness for the untrusted parser surface (rm-452): `crates/agenttrace-core/tests/parser_property_invariants.rs` drives randomized inputs through `parse_raw_session` under a deterministic xorshift64\* generator (128 cases by default; `AGENTTRACE_PROPERTY_SEED`/`AGENTTRACE_PROPERTY_CASES` replay or stress a failure exactly, with the seed printed on any panic) and pins three invariants — arbitrary and mutated bytes/JSON never panic, token accounting is invariant under object-key and line permutation, and detector dispatch is deterministic on adversarial mixed-key objects. Zero new dependencies (the registry was unreachable this cycle; swapping to proptest for shrinking is recorded as the row's residual leg). The `session_id` × `sessionId` dual-key class is deliberately excluded while the rm-449 port is in flight and lands with that fix.
- OTel GenAI OTLP-JSON export renderer (cycle 2, rm-493, renderer half): `agenttrace-core` gains a pure in-process exporter that renders parsed sessions as an OTLP-JSON `ExportTraceServiceRequest` (one INTERNAL span per session carrying `gen_ai.system`, `gen_ai.request.model`, `gen_ai.usage.input_tokens`, `gen_ai.usage.output_tokens` per the pinned semconv snapshot `SEMCONV_SNAPSHOT_DATE`, plus `agenttrace.session.*` attributes for cache read/write tokens, tool calls, cost, source tool, and the pricing provenance string resolved through `pricing::pricing_source_for`). The renderer is deliberately transport-free — no network code paths exist in the module (asserted by a source-scan tripwire test) — so export stays a local, privacy-preserving transformation like every other format; integration tests pin the envelope shape, per-agent attribute sets, and provenance presence. Scope note: this lands the library half only — the `-f otel` CLI arm is the cycle-3 follow-up, and the roadmap wall records the batch's two rider items (rm-389, rm-212) as re-dispatched, not delivered.
- `-` as the positional session path reads one session stream from stdin (rm-503): `agenttrace --overview -` previously failed with "session path does not exist: -" (rc 1) whether or not bytes were piped, and there was no `io::stdin` anywhere in the CLI crate - a pipe-friendly session report was impossible. `-` is now intercepted before the path-existence checks and routes the piped bytes through `parse_stdin_bytes` (the shared decode tail of `parse_file`: the same UTF-16-BOM and zstd-frame bails with `<stdin>` standing in for the path, so encoding failures stay actionable), then through the identical explicit-session tail - a piped session renders byte-identically to the same session named as a file (`stdin_dash_overview_is_byte_identical_to_the_same_file` pins the equality), `-o` output paths work as for files, and empty stdin fails with the existing "empty session" error class at rc 1 exactly like an empty named file. Stdin sessions are ephemeral by construction - like every explicit single-file load they never touch the session cache (neither read nor write), which `--help` discloses on the path argument. `-d` stays a separate multi-session lane.

### Changed

- The health-gate example workflow is supply-chain pinned and lets the gate gate (rm-405): `examples/github-actions/agenttrace-health-gate.yml` pinned `actions/checkout` and `actions/upload-artifact` to mutable major tags and installed the binary with `curl | sh` straight off the moving `master` ref with no checksum. Both actions are now SHA-pinned to their `v7.0.0` tags with version comments, the install step downloads the pinned `v0.9.0` release asset and verifies it against that release's published `checksums.txt` (platform detection mirrors `install.sh`; both `sha256sum` and `shasum` handled), and the gate step is documented as the job-verdict owner — the report-only steps stay `if: always()` explainers whose exit codes cannot mask a red gate. A new `scripts/ci/check-example-workflows.sh` gate (wired into CI after the release-surface-drift check) enforces the contract deny-by-default: every `uses:` must end in a full 40-hex commit SHA, a pinned release with checksum must be present, no moving-ref `curl | sh`, no `|| true` on the gate step, and `--fail-under-health` must appear — verified rc0 on the example and red on each mutation. The runner-execution arm (a sub-health corpus failing the example job on a real runner) stays open on the follow-up checklist; the static arms are verified. The report-only session-breakdown leg now runs `--sessions -f csv` instead of `--sessions -f markdown` — markdown requires `--overview`, so the taught command always bailed at the format guard and the step never produced output; the CSV statement export is inside `-f csv`'s composable set and machine-readable (integration review decfa879).
- CI: the never-true `AGENTTRACE_TUI_REAL_DIR` gate (pass-11 A11-8) on the PTY TUI smoke step now reads the repository variable of the same name and forwards it, making the opt-in reachable; dependency review skips fork PRs with an explanatory note instead of failing on the missing base snapshot (repo-owned runs unchanged).
- CI: `scripts/ci/check-docs-commands.sh` is hermetic (rm-369): the gate binary now resolves from the repository's own release build (`target/release/agenttrace`) instead of a days-old `/tmp/agenttrace` left over from a previous session, failing loudly when neither that nor `AGENTTRACE_BIN` exists, and the markdown/HTML stdout captures land under the same `AGENTTRACE_CI_OUT` directory instead of stray fixed `/tmp/agenttrace-docs-*.stdout` paths — the `AGENTTRACE_BIN`/`AGENTTRACE_CI_OUT` override contracts are unchanged.
- CI: `scripts/ci/check-install-ref-drift.sh` keeps the installer's source-build default honest (rm-017 rider): `install.sh`'s `AGENTTRACE_SOURCE_REF` default is compared against CHANGELOG.md's newest `## v` heading, so refreshing one without the other fails the build instead of drifting silently (the default sat two upstream releases behind with no automated signal). The gate enforces agreement, not freshness — a deliberate hold-back must touch the gate consciously. Wired into the `lint` job after lockfile enforcement; green live (`v0.9.0` == `v0.9.0`).

- The generic-JSONL fallback no longer silently drops recoverable lines (pass-7 P7-1): a lone-surrogate line is repaired via the same lenient machinery the format detectors use, an Event-typed usage block (`"usage": {"input": {"tokens": 5}}`) and numeric-string usage coerce instead of failing the whole event, and a lowercase `usage` key is no longer invisible next to `Usage`. Lines that still cannot parse are counted per reason and surfaced everywhere: per-session `metrics.line_skips`, aggregate `data_health.line_skips`, and a `Dropped lines` row in the text, Markdown, and HTML overviews; any lost line also caps `data_health.confidence` at `low`.
- Baseline regression thresholds now gate the exit code (pass-7 P7-3): `--baseline` with any `--baseline-max-*-delta-pct` breach fails the run with exit 2 (mirroring `--fail-under-health`) and names the breached thresholds on stderr, where the breach booleans previously sat unread in the report JSON while the process exited 0. `--no-baseline-gate` keeps the comparison in the report without failing the build.
- Session files written with a UTF-8 BOM now parse (one BOM is stripped at the shared parse entry, and nowhere else — a U+FEFF inside content survives), and UTF-16 files fail with a named, actionable error (`... is UTF-16 encoded; convert it to UTF-8 and retry`) instead of a generic read failure (pass-7 P7-2). PowerShell 5.1's default `>` redirection produces exactly this shape.
- The pricing-catalog cache and the derived-history file now stage through a unique temp sibling and rename into place (pass-7 P7-5), extending the pass-6 atomic-write hardening to the last two raw `fs::write` callers; a torn or corrupt `history.json` is quarantined as `history.json.corrupt` with a visible warning instead of silently wiping the durable record, and orphaned `<name>.json.tmp.<pid>.<seq>` siblings are swept (age > 1 hour) whenever the session cache loads.
- The lone-surrogate repair no longer rewrites literal `\\uXXXX` text: escaped-backslash pairs advance together, so `"\\ud800"` (an escaped backslash followed by `ud800`) keeps its literal bytes while a real lone surrogate in the same line still repairs (cycle-3 residual).
- The SQLite snapshot cache schema was bumped 5 → 6: cycle 3's placeholder-name rewrite shipped while the version stayed at 5, so v5 snapshots could carry stale names under new semantics; they now regenerate (cycle-3 residual).
- Fixed a `clippy::useless_format` finding in the TUI provider/model row label under the current toolchain.

### Added

- Extended the token and cost hardening to the SQLite ingestion paths (OpenCode `opencode.db` and Hermes `state.db`), which the first hardening pass had missed: adversarial token counts now saturate instead of overflowing the per-session accumulator (a crafted database previously crashed debug builds with exit 101) or wrapping negative (`u64::MAX` previously reported `"input": -1`), and negative token columns in Hermes databases are clamped to zero. The earlier entry below claimed repo-wide coverage that this path disproved; it is now true.
- SQLite-backed OpenCode sessions now prefer the authoritative totals recorded on the session row (`cost` and the five token columns) over message-derived aggregation when present, keep the derived path for older schemas, and disclose the choice: `provenance.tokens` reports `stored_session_totals`, the per-session `stored_totals_delta` exposes how far derived aggregation drifted, and `data_health` summarizes both (`stored_totals_sessions`, `stored_totals_delta_tokens`). The SQLite snapshot cache schema was bumped (v5) so cached entries regenerate under the new semantics.
- Sessions with an unknown start time (for example OpenCode rows with `time_created = 0`) are no longer silently dropped from `--range`/`--since` views; they stay visible in an unknown-time bucket and `data_health` reports the count as `unknown_time_sessions`.
- The default TUI now fails with a normal error (`stdout is not a terminal; ... use agenttrace --overview`) instead of panicking with exit 101 when stdout is not a terminal — the README quickstart previously crashed in every piped context (CI, cron, docker without `-t`, IDE consoles).
- `--version` now wins over argument validation, including action validation: `agenttrace --lang fr --version` and `agenttrace --overview --version` print the version instead of rejecting the unsupported language or the action combination first (pass-6 P6-2).
- Fixed a crash every non-interactive surface inherited from format detection: a single log line containing a `\u` escape followed by multi-byte UTF-8 (for example `{"prompt":"\u中文测试"}`) sliced mid-character inside the lone-surrogate repair path and killed `--overview`, `--doctor`, `--waste`, `--latest`, `--sessions`, `--diagnostics`, positional files, and directory scans with exit 101 in debug and release builds. Escape hex is now read from bytes, rejected escapes pass through untouched, and the adversarial corpus gained unicode-escape reproducers plus contract tests asserting the file degrades to "unsupported format" while clean neighbors keep loading (pass-6 P6-1).
- The fallback token estimate (used when a session records no usage block) is now CJK-aware: ASCII text keeps the classic four-characters-per-token rate while every non-ASCII character counts as roughly one token, where the previous bytes-divided-by-four heuristic under-counted CJK by 40-60%. `reasoning_chars` now counts characters rather than bytes, matching the unit its name promises, and a new `Naming` provenance distinguishes `first_user_request`, `file_name`, `provider_title`, `message_derived`, `session_id`, and `provider:placeholder` session names (pass-6 P6-4, research candidate 34).
- OpenCode sessions whose recorded `title` is a `New session - <timestamp>` placeholder (every session the provider does not summarize; 227/227 in the live census at research-census time) are now named from their first user message text instead of carrying the placeholder as their name; real provider titles still win, and the gate is disclosed through `provider:placeholder` naming provenance (research candidate 34).
- `--demo --overview -f json` now pins `scope.generated_at` to a fixed synthetic epoch (`2026-05-02T10:36:00Z`, just after the newest demo event) instead of the wall clock, so the deterministic-output check no longer flakes whenever two back-to-back runs straddle a second boundary; real (non-demo) reports still stamp the actual generation time.
- Session-cache and SQLite-snapshot writes now stage through a per-writer unique temp file (process id plus an in-process counter) before the same atomic rename, so two concurrent agenttrace processes can no longer race on a shared `<name>.json.tmp` and fail or tear a save (pass-6 P6-3).
- Governance cost audits no longer report `confidence: "high"` when any session in scope carries a negative token or cost component.
- Hardened token and cost aggregation against adversarial or corrupt session logs: token counts now saturate instead of overflowing, negative usage values are clamped, and reports can no longer print negative token totals or panicked and absurd costs.
- Made pricing fully offline by default: report and test paths never download anything. A dated LiteLLM snapshot (2,458 chat models, trimmed from the ~2 MB catalog to 533 KB) is bundled with the binary and used whenever no cached catalog exists; the network is touched only by the explicit `--update-pricing` command.
- Stabilized `pricing_source` labels: they no longer embed fetch or cache timestamps, so identical inputs produce byte-identical reports across runs and cache states.
- `--lang` now rejects unsupported values with an error instead of silently falling back to English.
- Removed stale `.gitignore` entries (`agentwaste`, `apps/desktop/...`) left over from an earlier layout that no longer exists in this tree, and ignore the local `.hermes/` harness-state directory.

### Added

- Committed generic-loss adversarial fixture (`testdata/generated/adversarial/generic-loss.jsonl`) pinning the pass-7 P7-1 shapes: a recovered lone-surrogate line, a coerced Event-typed usage line, and a counted unparseable line, asserted from file through `data_health`.
- A test pinning `PRICING_SNAPSHOT_DATE` to the bundled `pricing_snapshot.json` payload's `_snapshot.date`, so the const and the snapshot can no longer drift apart silently.
- Committed adversarial SQLite repro fixtures (`testdata/generated/adversarial/sqlite/`, regenerable via `scripts/fixtures/make-adversarial-sqlite.py`) with regression tests covering the overflow, wrap, negative-column, stored-totals, and unknown-time paths.

- Added `scripts/pricing/update-snapshot.sh` to regenerate the bundled pricing snapshot.
- Added `scripts/ci/check-plugin-version.sh` tying `.codex-plugin/plugin.json` to the latest CHANGELOG version so release drift is caught locally.

## v0.9.0 - 2026-09-30

Backfilled from the GitHub release notes for the tags that shipped without
changelog entries (`v0.8.0`, `v0.8.1`, `v0.9.0`; full changelogs:
[v0.8.1...v0.9.0](https://github.com/luoyuctl/agenttrace/compare/v0.8.1...v0.9.0)).

### Fixed

- Parser: skip leading non-session lines in Oh My Pi JSONL ([#284](https://github.com/luoyuctl/agenttrace/pull/284)).
- Windows: link the MSVC CRT statically so installed binaries no longer require the
  Visual C++ Redistributable ([#285](https://github.com/luoyuctl/agenttrace/pull/285)).
- Codex cost double counting fixed and TUI triage tightened ([#286](https://github.com/luoyuctl/agenttrace/pull/286));
  includes the total-usage rewind/high-water handling that `rm-162` ports into
  this fork's parser.

## v0.8.1 - 2026-09-06

Annotated retroactively (`rm-303`): this tag shipped without a changelog
section. Its delta is the TUI navigation/feedback/loading-progress work of
[#283](https://github.com/luoyuctl/agenttrace/pull/283) (`a34dea2`); the
v0.9.0 section above backfills the release notes for `v0.8.0`–`v0.9.0` in
one place
([release page](https://github.com/luoyuctl/agenttrace/releases/tag/v0.8.1)).

## v0.7.1 - 2026-07-20

### Changed

- Refreshed the redacted real-local-run GIF, TUI screenshots, and static HTML overview evidence for the v0.7.1 release surface.
- Updated release-facing metadata across the CLI, plugin, Homebrew Formula, README, Pages, and sample report.

## v0.7.0 - 2026-07-19

### Added

- Added local governance reports for cost audits, prioritized recommendations, observed MCP usage, cross-session context trends, and read-only Git delivery evidence.
- Added optional local model aliases and per-million-token pricing overrides through `AGENTTRACE_PRICING_FILE`.
- Added report scope, pricing confidence, project-root resolution, cache-aware doctor output, and governance appendices to overview exports.
- Added Action Center, Efficiency, and Delivery workspaces to the TUI, plus bilingual UI copy, live search, paste handling, and report scrollbars.

### Changed

- Updated the CLI, TUI, reports, plugin, Homebrew Formula, Pages assets, and release metadata to v0.7.0.
- Made CLI report actions mutually exclusive and hardened validation for gate thresholds and output formats.
- Made delivery and MCP output explicit about evidence limits: commit correlation is not authorship or merge proof, and invocation logs do not imply complete MCP inventory coverage.

### Validation

- Added governance, TUI interaction, discovery, report, and release-surface coverage to the Rust test and CI paths.

## v0.6.0 - 2026-07-19

### Added

- Replaced the Go implementation with a Rust workspace while preserving one
  `agenttrace` binary for both CLI reports and the terminal TUI.
- Added cached background session loading, Hermes/OpenCode SQLite sources,
  `Detailed`/`Aggregate`/`Limited` data capability labels, coverage reporting,
  privacy-safe tool-step metadata, actionable issue filters, and shared Core
  findings/comparison rules across CLI and TUI.
- Added deterministic generated parser fixtures and CI checks for provider
  coverage, data degradation, step redaction, and single-binary entrypoints.

### Changed

- Split the Rust TUI into state, presentation, filtering, and test modules and
  moved shared data-health/comparison logic into `agenttrace-core`.
- Streamed JSONL object parsing instead of retaining whole-file JSON trees,
  reducing peak memory by about 44% on a 2.53 GiB local session corpus while
  keeping report totals unchanged.
- Added `Ctrl+d`/`Ctrl+u` half-page movement and `G` end navigation to the TUI.
- Updated public docs, Pages, plugin, and Skill surfaces to describe the Rust
  implementation and honest per-source evidence limits.

### Fixed

- Invalidated SQLite session snapshots when the database, WAL, or SHM file
  changes.
- Hardened preserved-history loading against malformed short identifiers and
  stabilized the real-data CLI/TUI release smoke checks.

### Validation

- The Rust workspace, release binary, parser fixtures, CLI/TUI PTY entrypoints,
  report contracts, Homebrew syntax, and Pages artifact are covered by the
  local release gate.

## v0.5.4 - 2026-05-24

### Changed

- Published cross-platform command-line release assets and checksums.

## v0.5.3 - 2026-05-24

### Changed

- Refreshed release-facing artifacts for the v0.5 line.

## v0.5.2 - 2026-05-24

### Fixed

- Fixed Claude Code JSONL metrics for assistant messages that include thinking,
  text, and a parallel `tool_use` batch so the report keeps one assistant turn,
  multiple tool calls, cache token attribution, and failed `tool_result`
  counting aligned. (#243)

## v0.5.1 - 2026-05-19

### Fixed

- Clarified `agenttrace --doctor` cache-state wording so users can distinguish
  parsed session cache entries, entries reusable for the current scan, and
  cached directory listings. (#239)

## v0.5.0 - 2026-05-18

### Added

- Added local baseline comparison for overview reports so a later run can be
  checked against a saved local JSON baseline. (#203)
- Added incident timeline evidence to the TUI and report surfaces. (#204)
- Added tool authority summaries to HTML, Markdown, and text overview reports.
  (#210, #212, #214, #219, #221)

### Changed

- Improved overview report readability for Unicode text, incident rows, and
  terminal-readable authority summaries. (#216, #217, #219, #221)
- Aligned public README, docs, site metadata, and discovery surfaces with the
  current local coding-agent session coverage. (#197, #202, #228, #229, #230,
  #231, #232)
- Removed stale package-channel and launch-kit surfaces so release-facing
  install guidance stays limited to available channels. (#225, #226)

### Validation

- Added and refreshed release-surface, report-semantics, Pages artifact, and
  parser-coverage checks for the v0.5.0 release train. (#178, #182, #184, #205,
  #208)

## v0.4.6 - 2026-05-10

### Fixed

- Show sessions from `~/.pi/agent/sessions` as Pi while keeping
  legacy `~/.omp/agent/sessions` sessions labeled Oh My Pi.

## v0.4.5 - 2026-05-10

### Fixed

- Added PI auto-discovery for `~/.pi/agent/sessions` while keeping the legacy
  Oh My Pi `~/.omp/agent/sessions` path for compatibility.

## v0.4.4 - 2026-05-10

### Added

- Added a real local-data marketing refresh script for README and site assets.

### Changed

- Capped overview JSON anomaly details and added anomaly total/truncation metadata
  so large real histories stay readable for automation and promotional reports.
- Refreshed README and site screenshots from a real local run.

## v0.4.3 - 2026-05-10

### Changed

- Updated release surfaces for the v0.4.3 distribution.

## v0.4.2 - 2026-05-05

### Changed

- Refreshed README GIF and screenshots from a real local run with color enabled.
- Updated release surfaces for the v0.4.2 install paths.

### Fixed

- Kept Session List table values readable when terminal colors are enabled.

## v0.4.1 - 2026-05-04

### Changed

- Refreshed the README's real local-run screenshots and summary metrics from
  the latest TUI against local session logs.
- Updated release surfaces for the v0.4.1 distribution.

## v0.4.0 - 2026-05-04

### Changed

- Polished the first-run TUI demo path with clearer selected-session context,
  scan-friendly status text, refreshed demo assets, and an updated recording
  script. (#91)
- Improved TUI feedback around loading, empty diff states, and command-mode
  results so users get immediate guidance while navigating. (#122)
- Made `--waste` use the same latest-session selection behavior as `--latest`,
  reuse loaded diagnostics, and show clearer waste-report copy. (#95)

### Fixed

- Stabilized overview report ordering for recent sessions and anomaly tie
  breakers across JSON, Markdown, and HTML outputs. (#104)
- Aligned overview aggregate metrics with TUI discovery, including cache
  read/write tokens in the exported totals. (#114)
- Clamped loop waste so reported waste cannot exceed total session cost. (#116)
- Aligned TUI cache status wording with `agenttrace --doctor`. (#117)
- Isolated auto-discovery tests from runner-specific environment configuration.
  (#120)

### Validation

- Added repeatable CI gates for output contracts, deterministic demo output,
  report semantics, release surfaces, and Pages artifacts. (#118)
- Documented the launch-kit validation gates and release consistency checklist
  for public demo and install surfaces. (#115, #121)

<!--
  no-changelog-section markers (scripts/ci/check-plugin-version.sh, per-tag
  arm rm-303): releases shipped before that arm existed, whose tags are
  merged into every branch but whose changes never got a dedicated section
  at release time. Reasons below are taken from each tag's own commit
  (git log -1 <tag>). Recorded 2026-10-05, run 4a688257 full_tests
  b163e538: the gate began failing only after a wholesale tag fetch made
  these refs visible to `git tag --merged HEAD`; nothing about the releases
  themselves changed.
-->
<!-- no-changelog-section: v0.7.2: release only decoupled the Pages checks from the release version (3f6252a); CI wiring, no user-facing behavior to section -->
<!-- no-changelog-section: v0.7.3: release only fixed npm tarball publishing (739a6c3); packaging plumbing, no user-facing behavior to section -->
<!-- no-changelog-section: v0.7.4: release only configured npm auth before publishing (2462045); packaging plumbing, no user-facing behavior to section -->
<!-- no-changelog-section: v0.7.5: re-tag of v0.7.4's npm-auth fix on the same commit (2462045); packaging plumbing, no user-facing behavior to section -->
<!-- no-changelog-section: v0.7.6: release only published the npm launcher under the zack78 scope (e20a224); packaging plumbing, no user-facing behavior to section -->
<!-- no-changelog-section: v0.7.7: release only fixed release-channel script permissions (cd33203); CI plumbing, no user-facing behavior to section -->
<!-- no-changelog-section: v0.8.0: upstream merge "Improve pricing provenance and TUI session exploration (#280)" (b964a74) shipped without a dedicated section; its user-visible changes are covered by the v0.8.1 and v0.9.0 sections that follow it -->
