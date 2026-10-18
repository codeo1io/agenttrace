# agenttrace — Roadmap

> Autonomously maintained by the roadmap sync (reliability-first). Items cite reproducible codebase signals; acceptance is proven by cited evidence.

**Vision**: A reliable, customer-friendly repository advanced by evidence-cited roadmap cycles owned by the autonomy loop

**Pillars**: reliability work outranks customer-experience work; every roadmap item cites reproducible codebase signals; acceptance is proven by cited evidence, never claimed

## Fleet context

- dependents (changes here affect): (host), agent, maestro
- graph: evidence-derived (imports/refs/deploy surfaces); advisory

## Open items

### Add test coverage for 1 untested module(s)
- id: `rm-002` | track: reliability | priority: 83.0 | status: candidate
- signals: reliability.no_tests:scripts/fixtures/make-adversarial-sqlite.py
- acceptance: Every module in ['scripts/fixtures/make-adversarial-sqlite.py'] has a corresponding test file with at least one passing test
- evidence: full suite green (python -m pytest -q) at HEAD; conductor validation digest validation:v1:<sha> recorded in the shipping PR

### Refactor 3 high-complexity function(s)
- id: `rm-001` | track: reliability | priority: 79.0 | status: candidate
- signals: reliability.complexity_hot:npm/scripts/install.js::L24, reliability.complexity_hot:npm/scripts/install.js::L28, reliability.complexity_hot:npm/scripts/install.js::L40
- acceptance: Each flagged function is decomposed below the branch threshold with behavior locked by characterization tests
- evidence: ast-based branch-count check passes at HEAD (full suite green; conductor validation digest validation:v1:<sha> recorded in the shipping PR)

### Decode Codex zstd rollouts in-process

- id: rm-003
- track: compatibility
- priority: high
- status: candidate
- signals: compatibility.rejected_format:crates/agenttrace-core/src/parser.rs::L32, compatibility.version_drift:codex-cli >=0.152 zstd rollout frames
- acceptance: A zstd-framed Codex rollout fixture (magic 28 B5 2F FD) parses transparently with no manual decompress step; decoding uses a pure-Rust crate (e.g. ruzstd) so static MSVC/MUSL builds gain no C toolchain dependency; doctor reports the Codex format acceptance rate; existing JSONL fixtures unchanged and suite green
- evidence: crates/agenttrace-core/src/parser.rs:32-40 bails with "decompress it to JSONL first" at HEAD 90a4ef5; Codex latest release rust-v0.159.2 published 2026-09-29 (GitHub API, fetched 2026-09-30, research pass 7); local codex 0.147.0 still writes JSONL, so the defect is forward compatibility for mainstream-current CLI users

### Make diagnostics and waste reporting truthful

- id: rm-004
- track: correctness
- priority: high
- status: implemented
- signals: correctness.over_reporting:crates/agenttrace-core/src/diagnostics.rs::L660, correctness.baseline_poisoning:crates/agenttrace-core/src/diagnostics.rs::L428, correctness.bucket_miscount:crates/agenttrace-core/src/diagnostics.rs::L730, correctness.weak_dedup_key:crates/agenttrace-core/src/diagnostics.rs::L866, correctness.double_count:crates/agenttrace-core/src/waste.rs::L54, correctness.cost_fabrication:crates/agenttrace-core/src/waste.rs::L239
- acceptance: Each of the six assessed defects is fixed or its display explicitly annotated as an estimate - loop_groups reports an actual group count; predict_cost_anomaly excludes zero-cost sessions from its baseline or labels the result; tool_latencies stops counting unmatched calls as timeouts and no longer drops calls >=3600s; repeated_response keys on full response identity, not a 50-char prefix; stuck detection counts each long-gap pattern once; per-tool cost comes from tool-share data or renders as a session-level estimate, never fabricated per-tool currency; every changed diagnostic carries a golden-value test pinning expected output on a fixture session
- evidence: assess phase b2488fec at HEAD 90a4ef5 (9 findings, 6 in this cluster); anchors re-verified with fresh pinned greps (diagnostics.rs:660, :428-441, :727-733, :866-877; waste.rs:54 and :56-70, :239-247 rendered at :324-334; consumers presentation.rs:2398 and :2412-2424); CYCLE-1 IMPLEMENTED pre-review (run cc2f32d5, implement attempt f7c2990d, worktree run-cc2f32d56918-cc2f32d5 @ 90a4ef5): all six defects fixed - loop_groups counts distinct consecutive-tool runs >=3 (diagnostics.rs:631/:644/:655/:679), predict_cost_anomaly baseline excludes cost_estimated==0.0 sessions (:432), the >=3600s latency cap is removed and the field renamed timeouts->unmatched (:66; consumers governance.rs, explorer.rs, presentation.rs), repeated_response keys on full message content (no 50-char prefix), stuck long-gaps scored exactly once (waste.rs:63-68), per-tool bloat cost renders as a labeled call-count allocation of WasteReport.session_cost (waste.rs:245, :351 'N% of session'); six golden-value tests added in-module (loop_groups_counts_distinct_runs_not_a_flag, cost_anomaly_baseline_ignores_unpriced_sessions, tool_latencies_keep_long_calls_and_report_unmatched, repeated_response_requires_identical_full_content, stuck_long_gaps_scored_once, per_tool_cost_is_allocated_share_of_session_cost); validation: targeted gate 243/243 (result-3832098-324310608.json) and full-command gate 243/243 returncode 0 (result-3934489-324343777.json); changes uncommitted in the worktree awaiting the commit phase

### Verify downloaded artifacts in install.ps1

- id: rm-005
- track: security
- priority: high
- status: implemented
- signals: security.unverified_download:install.ps1::L51, supply_chain.parity_gap:install.ps1 vs install.sh and npm/scripts/install.js
- acceptance: install.ps1 fetches the .sha256 sidecar emitted by release.yml for the Windows asset, compares via Get-FileHash, and refuses to install on mismatch or missing sidecar; a parity check names the sidecar each of the three installers verifies (install.sh, install.ps1, npm/scripts/install.js)
- evidence: install.ps1:37-51 downloads via Invoke-WebRequest with no hash check and only probes at :62; install.sh:57-72 verifies the sidecar and names the parity intent; npm/scripts/install.js verifies SHA-256 pre-install; release.yml emits .sha256 for every asset including Windows (assess A11-9, re-verified at HEAD 90a4ef5); upstream distributes npm-only with zero GitHub releases or tags, making the sidecar channel this fork's differentiating trust anchor; CYCLE-1 IMPLEMENTED pre-review (run cc2f32d5, implement attempt f7c2990d): install.ps1:62-71 fetches $url.sha256 into a temp sidecar, validates the digest is a 64-hex token, compares via Get-FileHash, and refuses the install (throw -> catch -> exit 1) on missing, malformed, or mismatched sidecar BEFORE Move-Item, sidecar temp cleaned in finally (pwsh Parser::ParseFile PARSE OK); parity locked by agenttrace-core test all_three_installers_verify_sha256_sidecars (lib.rs:1620) asserting the sidecar mechanics of install.sh, install.ps1, and npm/scripts/install.js together; validation: full-command gate 243/243 returncode 0 (result-3934489-324343777.json); changes uncommitted in the worktree awaiting the commit phase (Windows-side smoke of the gate still owed at final_validation)

### Establish pricing snapshot refresh cadence and drift surfacing

- id: rm-006
- track: data-freshness
- priority: medium
- status: candidate
- signals: data_freshness.stale_snapshot:crates/agenttrace-core/src/pricing.rs::L16, data_freshness.catalog_drift:LiteLLM main vs bundled snapshot
- acceptance: The bundled snapshot refreshes on a stated cadence (per maintenance cycle at minimum) using the landed refresh mechanism; doctor reports snapshot age and entry delta against upstream LiteLLM; --overview notes pricing confidence when the snapshot is older than the cadence allows
- evidence: pricing.rs:16 pins the snapshot at 2026-09-13 with 2,755 chat-priced entries at HEAD 90a4ef5; LiteLLM main counted 4,436 entries / 3,680 costed on 2026-09-30 (research pass 7); the refresh tooling already landed per CHANGELOG pass 9 - the gap is cadence ownership and user-visible drift, not mechanism

### Stage a dependency refresh wave

- id: rm-007
- track: dependencies
- priority: medium
- status: candidate
- signals: dependency.drift:Cargo.toml rusqlite 0.32 to 0.40.1, ratatui 0.30 to 0.31.0, crossterm 0.28 to 0.29.0, ureq 2.12 to 3.4.2, clap to 4.5.48
- acceptance: Refresh lands in stages - minors first (clap, crossterm, ratatui), then rusqlite 0.40 with the full adversarial-sqlite fixture regression, then the ureq 3 rewrite last with its API migration; each stage pins exact versions re-verified against the registry at bump time (the 0.40.2 and 4.6.7 pins from the prior research doc were not reproducible on 2026-09-30 - registry yank churn); static build profile unchanged and suite green per stage
- evidence: crates.io max_versions fetched 2026-09-30 (research pass 7): rusqlite 0.40.1, ureq 3.4.2, clap 4.5.48, crossterm 0.29.0, ratatui 0.31.0; repo pins at HEAD 90a4ef5: rusqlite 0.32, ureq 2.12, crossterm 0.28, ratatui 0.30

### Prune the API surface before any 1.0 lock

- id: rm-008
- track: api-hygiene
- priority: medium
- status: candidate
- signals: api_hygiene.dead_exports:crates/agenttrace-core/src/lib.rs::L33, api_hygiene.suppressed_lint:crates/agenttrace-tui dead_code allow, api_hygiene.dead_code:crates/agenttrace-tui/src/doctor.rs::L191
- acceptance: The five dead re-exports in lib.rs are consumed or removed; the TUI dead_code allow is removed and the workspace compiles clean under clippy -D dead_code; doctor.rs count_by_root is consumed or deleted; removals land before a 1.0 semver lock (removals after it are breaking)
- evidence: assess A11-7 and A11-8 at HEAD 90a4ef5, re-verified: lib.rs:33 re-export block exports five never-consumed symbols; doctor.rs:191 count_by_root written-never-read; the TUI crate carries a crate-level dead_code suppression

<!-- New items 2026-09-30 (run 2326f88e research, attempt 3c3a6d95): numbered rm-009+ to stay disjoint from sibling campaign cc2f32d5's uncommitted rm-003..rm-008 -->

### Patch RUSTSEC-2026-0285: ship rustls >= 0.23.45 in release binaries
- id: `rm-009` | track: reliability | priority: 88.0 | status: done
- signals: security.advisory:RUSTSEC-2026-0285:Cargo.lock::L1633 (rustls 0.23.42 locked, reached via ureq 2.12.1 pricing transport; upstream fix commit 56df4c47 "cargo update -p rustls 0.23.42 -> 0.23.45" merged 2026-09-30T02:58Z)
- acceptance: Cargo.lock resolves rustls >= 0.23.45 with no other dependency regressions; cargo tree -i rustls still shows the single pricing-fetch path; release artifacts rebuilt against the patched lock
- evidence: cargo audit (or cargo-deny advisories check) reports zero RUSTSEC-2026-0285 findings at HEAD; full suite green; conductor validation digest validation:v1:<sha> recorded in the shipping PR
- cycle-1 (2026-09-30, run 2326f88ec8b4): implemented uncommitted in worktree run-2326f88ec8b4-2326f88e — rustls 0.23.42→0.23.45 (plus companion webpki 0.103.13→0.103.15, 8-line lock diff); full suite green (core 52, tui 45, entrypoints 10, doc-tests clean, digest validation:v1:403a28e2…); local cargo-deny advisories ok on the patched lock; learning recorded in docs/solutions/security-issues/rustsec-advisory-shipped-via-ungated-lockfile.md; status flipped to done by the commit gate

### Adopt upstream supply-chain hardening wave (cargo-deny + OpenSSF Scorecard)
- id: `rm-010` | track: reliability | priority: 85.0 | status: done
- signals: reliability.ci_gap:.github/workflows (no cargo-deny, deny.toml, or OpenSSF Scorecard lanes; the pattern landed upstream as PRs #287/#288/#289/#290, merged 2026-09-30T01:53Z-03:38Z: CodeRabbit+Codecov, cargo-deny+deny.toml+Scorecard, git-cliff, badges)
- acceptance: deny.toml enforces advisories + licenses + sources + bans; a cargo-deny CI job gates every PR and push; an OpenSSF Scorecard workflow publishes results (SARIF); the fork's own lock passes all deny checks
- evidence: CI green including the new deny job at HEAD; Scorecard run artifact published; full suite green; conductor validation digest validation:v1:<sha> recorded in the shipping PR
- cycle-1 (2026-09-30, run 2326f88ec8b4): deny.toml added (yanked=deny, wildcards=deny, unknown sources denied) + third CI job `deny` (cargo-deny-action pinned SHA, --all-features) in ci.yml — committed by the cycle-1 commit gate with rm-009; local cargo-deny 0.20.2 check all four categories ok; Scorecard DEFERRED with rationale (private fork remote — Scorecard scores public repos only), revisit if repo goes public; next unexercised step: the deny job's first remote run (CI gate)

### Report-contract truthfulness batch (search truncation disclosure, rates.total, unknown-reason labels)
- id: `rm-011` | track: reliability | priority: 81.0 | status: candidate
- signals: correctness.contract_gap:crates/agenttrace-core/src/search.rs::L203 (search JSON omits total_matches/truncated; per-session cap 8; --sort/--order unapplied on the search path, main.rs::L409-419), correctness.constant_field:crates/agenttrace-core/src/governance.rs::L97+L104 (rates_per_million_usd.total serialized as constant 0.0 while components.total is computed at L106; serialization sites L326/L434), correctness.fallback_mislabel:crates/agenttrace-tui/src/i18n.rs::L147-L155 (unknown inspect_reason values render as "ok"/"正常" while every sibling fallback says "unknown ...")
- acceptance: search JSON carries total_matches and truncated fields and either applies --sort/--order or exits with an explicit unsupported-flag error; rates_per_million_usd.total is computed from its components or the field is removed across all cost-audit serializations; inspect_reason fallback labels unknown reasons as unknown in every locale
- evidence: golden JSON tests covering search truncation disclosure, rates-total computation, and reason-fallback rendering; full suite green; conductor validation digest validation:v1:<sha> recorded in the shipping PR
- cycle-2 anchor: deliberately deferred this cycle (prioritization a818550b — three sub-defects × golden-test scaffolding ≈ a full session alone); untouched this cycle, anchors assess AF-1..AF-3

<!-- New items 2026-09-30 (run 52465b9e assess 9431a48f + research cea11f48, attempt 56eb6993): numbered rm-034+ to stay disjoint from every live claimant — origin/master 9d88b36 holds rm-001..027+rm-033 (PRs #16/#17/#18 all merged), this tree in rm-001..011; record-not-renumber: on integration, renumber campaign-local IDs to the next free range per this comment if any surface claims rm-034+ first -->

### Ship a unified render-safety boundary for session-derived strings

- id: rm-034
- track: security
- priority: high
- status: done
- done-evidence: implement 5d935c96 + review-fix 58d33778 at ce27969 — live probes re-run against the rebuilt binary over EVERY terminal-text sink incl. --sessions and --search (review cd9a253b F1/F2 closed: 0 raw ESC/BEL/C1, U+FFFD neutralization); html_escape regression test + TUI terminal_safe_report tests added (review F4); review 14f25886 F1 closed the last sibling sink: --doctor provider paths, symlink targets, and parse-failure sample file names now route through sanitize_line_segment with a doctor payload test
- signals: security.control_sequence_injection:crates/agenttrace-core/src/reports.rs::L290 and ::L891-893 (terminal-text paths interpolate untrusted session strings raw), security.false_safe:terminal_safe_report transliterates box-drawing but passes ESC verbatim (`_ => ch`), security.undocumented_guard:agenttrace-tui relies on ratatui zero-width grapheme dropping with no in-code note
- acceptance: one shared sanitization policy in agenttrace-core serves every consumer of session-derived strings (model, agent, session name, project path) across terminal-text, markdown, HTML, JSON, and TUI; the text and markdown report paths emit no raw ESC/OSC/CSI bytes originating from untrusted strings (golden test with an OSC-52 + SGR payload fixture run through --overview text, single-session text, and markdown); statusline's sanitizer folds into the shared module with its hostile-payload test carried over unchanged; the HTML sink keeps html_escape with a regression test; the TUI either routes through the policy or documents the ratatui reliance in-code; assess AF-1 anchors re-verified gone at the shipping HEAD
- evidence: assess phase 9431a48f at HEAD ce27969 — AF-1 live E2E: crafted session model `EVIL\u001b]52;c;aGVsbG8=\u0007\u001b[31mRED` emitted verbatim (raw ESC count 2) through `agenttrace --overview` and the single-session report (reports.rs:290, :891-893); HTML verified escaping at sink via live probe (html_escape reports.rs:2364); statusline sanitized with hostile test (statusline.rs:187, :873, rm-012 lineage); TUI proven safe only by ratatui width-0 drop via pty frame capture; research pass 11 candidate 71 — first 'sanitize' claim in the research ledger (grep across docs/ideation 2026-09-02 + docs/research passes 4-9 + spool passes 5/6/10)

### Close the CLI argument-surface contract gaps (missing long forms, silent --format drop)

- id: rm-035
- track: developer-experience
- priority: medium
- status: done
- done-evidence: implement 5d935c96 + review-fix 58d33778 at ce27969 — live probes re-run against the rebuilt binary; golden guard test over 7 argument orders added (review F4a). SUPERSESSION (review F6): the pre-batch Go-flag tolerance test's `-f json` case was deliberately rewritten (`go_flag_compatible_args_ignore_unknown_flags_after_positional_path` now drops an unknown legacy flag) because hoisting known value flags makes the old expectation obsolete by design. KNOWN HALF-OPEN: boolean (non-value) flags after the positional path are still silently dropped (within this item's value-bearing acceptance letter) — follow-up noted in the close comment
- signals: dx.missing_long_form:crates/agenttrace-cli/src/main.rs::L36/L58/L60 (-d/-m/-o have no --dir/--model/--output), dx.misleading_error:clap suggests `--delivery-evidence` when `--dir` is typed, dx.silent_drop:main.rs::L711-714 go-flag-compat shim breaks at the first positional discarding trailing value flags, dx.guard_bypass:main.rs::L174-182 markdown/html guard reads post-shim format and never fires when --format was dropped
- acceptance: --dir/--model/--output exist and behave identically to the short forms, with help text updated and the flag_takes_value allowlist (main.rs:727) kept in sync by a parity test against the arg declarations; a value-bearing flag after the positional path is either honored or rejected with an explicit error naming the dropped flag — never silently dropped (test: `agenttrace session.jsonl --format json` yields JSON or a hard error); the markdown/html format guard fires on every argument order (golden test); the existing Go-flag tolerance tests (main.rs:1318, :1336) stay green
- evidence: assess 9431a48f AF-2/AF-3 live at ce27969 — pty-captured `--dir /tmp` error suggesting `--delivery-evidence`; `agenttrace /tmp/agenttrace-ov-probe/m.jsonl --format json` returned the text report with exit 0; `--format markdown` and `--format html` identical (guard never fired); control `--latest -d <dir> --format json` returned valid JSON

### Emit a session source manifest (`agenttrace sources`)

- id: rm-036
- track: compatibility
- priority: medium
- status: candidate
- signals: compat.silent_format_drift:crates/agenttrace-core/src/discovery.rs::L215 (a mid-file parse error drops the whole session into `skipped` with only aggregate visibility), upstream demand: open issues #236 "[Radar] Track Gemini CLI to Antigravity CLI transition" and #237 "[Radar] Track Qwen Code export and dual-output session surfaces"
- acceptance: `agenttrace sources [--dir]` prints per detected format family: file count, parsed / skipped / downgraded-to-generic counts, first and last session timestamps, and a note when the generic fallback path was hit; output shape is stable for scripting; fully offline; golden test over a mixed-format fixture directory including one malformed file
- evidence: research pass 11 candidate 72 — upstream open issues #236/#237 (2026-05-19) show CLI transitions silently breaking session surfaces upstream; discovery.rs:215 skip path; no `sources` subcommand exists in main.rs; session_cache persists the detected source per session so the data is already available

### Accept session input on stdin (`agenttrace -`)

- id: rm-037
- track: developer-experience
- priority: low
- status: candidate
- signals: dx.no_stdin:crates/agenttrace-cli/src/main.rs has zero stdin handling (grep verified), workflow friction: piping a session from a remote host or smoke-testing a parser upgrade requires a temp file today
- acceptance: `-` (or `--stdin`) as the positional reads stdin to EOF, detects the format from content, and emits the standard report; stdin sessions are read-only (no cache writes unless explicitly enabled); documented in README; test pipes a fixture through and byte-compares key fields against the file-path invocation
- evidence: research pass 11 candidate 73 — `grep stdin crates/agenttrace-cli/src/main.rs` → 0 matches; the parser layer already operates in-memory over byte slices (jsonl_objects), so no structural change is needed below the CLI boundary; pipe-first UX is table stakes for CLI analyzers (jq, rippgrep)

### Reconcile with origin/master and port the remaining upstream batch (diagnostics collision first)

- id: rm-038
- track: compatibility
- priority: high
- status: candidate
- signals: compat.lineage_divergence:branch merge-base vs upstream/master = 6848aa1 (#284), behind 9 (#285..#293) while origin/master 9d88b36 is already past be25c4c (#286) and behind only 7 (#287..#293), compat.same_name_collision:fork diagnostics.rs:10 LoopFingerprint (9eba8cf counting semantics, 11 golden tests) vs upstream #286 LoopFingerprint (attention thresholds) — both sides of the lineage/master merge carry the name with divergent semantics
- acceptance: the lineage-to-master reconciliation resolves the LoopFingerprint collision explicitly (fork counting semantics preserved or consciously merged with #286's thresholds, decision recorded in the merge or PR body) and all existing golden diagnostics tests pass unmodified at the merged HEAD; #285 (MSVC static CRT) is evaluated on the Windows release lane with the build-profile outcome recorded; #287..#293 CI/docs deltas are folded without duplicating what rm-010 already landed here (deny.toml, badges); open upstream PRs #259/#272/#278/#279 are each tracked adopt-or-close with rationale
- evidence: research pass 11 ground facts — `git merge-base HEAD upstream/master` = 6848aa1 (#284), ahead 43 / behind 9; `git merge-base origin/master upstream/master` = be25c4c (#286), master behind 7; origin/master 9d88b36 ROADMAP holds rm-001..027+rm-033 (verified via gh pr diff on PRs #16/#17/#18, all MERGED)

### Fill empty agent labels in overview attribution

- id: rm-039
- track: correctness
- priority: medium
- status: done
- done-evidence: implement 5d935c96 + review-fix 58d33778 at ce27969 — live probes re-run against the rebuilt binary; golden test now pins the real claude-code fixture testdata/claude-code-preamble.jsonl rendering a named 'Claude Code' row (review F4d: discovery_contract claude_code_fixture_renders_a_named_by_agent_row)
- signals: correctness.empty_attribution:overview "By Agent" groups claude-code-family sessions under an empty name (live probe row `|  | 3 | $0.0033` at ce27969)
- acceptance: every By-Agent row carries a non-empty label derived from session metadata or an explicit localized 'unknown agent' bucket; golden test pins a claude-code fixture rendering a named row; no session lands in an unlabeled group
- evidence: research pass 11 PE-1 refresh row — live `--overview -f markdown` probe at ce27969 with three claude-code fixtures produced the empty-label row; attribution completeness was never claimed as a prior capability (ledger grep)

<!-- 2026-09-30 compound-c1 close (run 52465b9e, campaign 5cdbf259 cycle 1) — pre-review record.
     Batch rm-034 + rm-035 + rm-039 implemented uncommitted at ce27969; validated:
     targeted gate envelope result-3634171-325836455.json and full gate envelope
     result-3983123-325916121.json, both 251 passed / 0 failed / 0 ignored; digest
     validation:v1:669d910daa8cec0a72eb68da55eab39ac03c9e1fafa003f0fc06bb1e962262b9
     (base ce279697). Learning shipped: docs/solutions/security-issues/
     2026-09-30-untrusted-session-strings-render-boundary.md; CONCEPTS.md seeded.
     Done-flips above reflect implement+validation only — commit gate confirms after
     independent review. Next-cycle anchors: rm-036 (sources manifest), rm-037
     (stdin), rm-038 (lineage/master reconciliation + LoopFingerprint collision);
     research pass 11 candidates 71-73 (spool research-candidates-2026-09-30-
     run52465b9e-cea11f48.md).
     2026-09-30 independent review cd9a253b (NEEDS_CHANGES, 7 findings) → review-fix
     58d33778 (tree work from the prior 5c4b3e1d attempt whose envelope
     was lost to a provider 429, re-verified and completed here): F1/F2
     closed live (search text + session list now sanitize via
     render::sanitize_line_segment; 0 raw ESC/BEL over the hostile corpus), F4
     acceptance-test clauses shipped (guard golden test 7 orders, html_escape
     regression test, terminal_safe_report tests, claude-code fixture golden
     test), F3/F5 doc corrections. Post-fix full gate re-run at the review-fix
     tree: envelope result-943650-326456486.json, 256 passed / 0 failed /
     0 ignored (251 + 5 new review-fix tests), independently reproduced at
     the same tree by envelope result-3229376-328130582.json in the completing
     attempt; digest validation:v1:669d910d…
     re-derived at base ce279697 (unchanged — the review-fix delta touched only
     .rs/.md surfaces, none of the policy's 19 executable digest inputs).
     Follow-ups recorded, deliberately not in
     this batch: (1) boolean (non-value) flags after the positional path are
     still silently dropped by the go-flag shim (rm-035 was value-flag
     scoped); (2) html output still carries raw control bytes — markup-only
     escaping is the documented negative on html_escape, treat generated
     HTML as untrusted text; terminal-safe HTML variant would be a new item. -->

<!-- managed by hermes-roadmap render; do not edit by hand -->
