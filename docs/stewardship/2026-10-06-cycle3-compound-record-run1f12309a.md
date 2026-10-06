# Cycle-3 compound record — run 1f12309adc31 (repository-maintenance dde7875c, 2026-10-06)

Attempt lineage (three attempts, one phase): (1) 692d476c77eb42a597a5b45db28bc425
provider-reaped at message_count 3 in ~23 s, zero durable artifacts, nothing
to adopt; (2) d06314ff930d437091acefad388086ce provider-reaped mid-flight
(~16 messages, session_reaped) AFTER authoring this record, the solutions
rule, and a complete assert-guarded wall builder — it died before running
the builder or shipping any patch; the completing attempt verified its
content first-hand against tree 511fb01 and the recorded phase reports,
found and fixed three defects (R018 anchor non-unique ×3 rows, R367 anchor
non-unique ×2 rows, stale `kimi_usage_alias` :153→:163 after the salvage
shifted parser.rs) and ADOPTED the substance rather than redoing it;
(3) 9d0f19d60b554ab6b92c7fd409653a1c — completed the phase: ran the
corrected builder, shipped postimage + both patches, wrote the PhaseResult.
Compound consumes ONLY pre-review cycle evidence (assess
425d2301, research 97480e45, roadmap 3ba2e6d0, prioritize 5ec011a3, stewardship
1c7bc707, implement 8ede7a65, targeted 99c5062a, full a26322ab). No test
execution at compound; every validation number below is TRANSCRIBED from the
recorded phase outcomes, not re-run.

## What this cycle delivered (tree state)

The implement phase's uncommitted delta was landed by the engine mid-run as
salvage commit 511fb01be46777fa8b5b3ffc2503e248c8766f86 ("conductor-salvage:
run=1f12309adc3142558005df541c59f17c") on run base ea5c41e — 8 files:
`Cargo.lock`, `crates/agenttrace-core/src/parser.rs`,
`crates/agenttrace-core/src/reports.rs`,
`crates/agenttrace-core/tests/fixtures/workbuddy/{README.md,basis-clamped.jsonl,basis-includes.jsonl}`,
`crates/agenttrace-core/tests/workbuddy_usage_basis.rs`, `deny.toml`.
Targeted and full batteries pinned to that actual HEAD (511fb01), not the run
base. The batch was prioritize 5ec011a3's "Printed truth: finish the sanitizer
family, disclose the workbuddy clamp, close the advisory floor".

### rm-239 — render-boundary terminal-control sanitization (LEAD, 87.0 → implemented)

- RED proof at pristine ea5c41e reports.rs (release rebuild): hostile fixture
  model key `mod<ESC>]52;c;!<BEL>el` produced 1/1/2/0 raw-ESC lines across
  `--overview -f text/markdown/html/json` — the row's original acceptance
  clause ("md/html lanes remain byte-identical… they are already safe") was
  FALSIFIED at this base (assess F1; see the solutions doc below).
- Fix at 511fb01, all inside `reports.rs` at the render boundary:
  `text_cell` (:2708) sanitizes after the whitespace squash (the squash keeps
  column geometry but only the whitespace subset — ESC rode through);
  `markdown_cell` (:2810) and `markdown_inline_code` (:2819) sanitize AFTER the
  pipe/newline rewrites so `<br>` survives; By-Agent row label routes through
  `sanitize_line_segment(&tool_display_name(&agent))` (:930 — raw at base,
  found by the new unit test's offender diagnostic); By-Model/By-Provider/
  By-Task-Type raw row keys wrapped (:948/:965/:976, plus :292/:378/:445 lanes
  that print model/tool/anomaly strings).
- GREEN: 0/0/0/0 raw-ESC lines on the same fixture; sanitized label
  `mod<U+FFFD>]52;c;!<U+FFFD>el  1 Sessions` visible in the text lane; the
  hostile key stays exact in the json lane (losslessness pinned by
  `overview_json_lane_stays_lossless_for_control_bytes`).
- Pinned by 4 lane tests: `overview_text_lane_replaces_control_bytes_in_group_keys`,
  `overview_markdown_lane_replaces_control_bytes_in_group_keys`,
  `overview_html_lane_replaces_control_bytes_in_group_keys`,
  `overview_json_lane_stays_lossless_for_control_bytes` (lib suite 161→165 as
  recorded by implement 8ede7a65).

### rm-497 — workbuddy usage-basis clamp disclosure (88.0 → implemented)

- Both halves landed in `parser.rs`: the basis clamp counter
  `workbuddy_usage_basis_clamped` (counted into the parse-diagnostics
  `line_skips` map at the `parse_raw_session` workbuddy arm, format!-pattern
  matched to the `kimi_usage_alias` precedent at :163 — the :153 number
  was the ea5c41e-era ref; the salvage commit shifted parser.rs) and the
  read-or-disclose half — usage blocks on unconsulted lines now fire
  `workbuddy_usage_dropped:reasoning` / `workbuddy_usage_dropped:function_call_result`
  instead of vanishing.
- Live surfacing on the clamp PoC: `Parse: 1/1 parsed, 0 skipped |
  confidence: low` + `Dropped lines: workbuddy_usage_basis_clamped=1`
  (previously: input silently 0, cost $0.0014 priced on cache alone, zero
  counters — research XR-A).
- Committed fixture pair + contract tests:
  `tests/fixtures/workbuddy/{basis-clamped,basis-includes}.jsonl` (+ README) and
  `tests/workbuddy_usage_basis.rs` with `gross_basis_journal_nets_cache_and_stays_silent()`
  (netting contract control) and `basis_mismatch_clamps_input_and_discloses_it()`;
  `workbuddy_usage_on_unconsulted_lines_is_disclosed_dropped()` lives in the
  parser lib tests (parser.rs:6060).
- PoC-craft gotcha (on the rm-499/497 rows already): workbuddy fixtures must be
  ≥ 2 lines — one-line `.jsonl` files parse as single JSON documents and skip
  the JsonlProbe array (parser.rs:99-118 at mint time).

### rm-498 — advisory floor: lru bump + unsound-scope policy (78.0 → implemented)

- `Cargo.lock`: lru 0.18.1 → 0.18.5 (crates.io max_stable; RUSTSEC-2026-0253
  patched ≥ 0.18.2), diff-verified to change ONLY lru's version+checksum —
  the security-scoped single-crate move that composes without waiting on the
  staged rm-044 dependency wave (upstream PR #309's 8-crate jump is all-red
  and excludes lru/ratatui).
- `deny.toml` `[advisories]`: `unsound = "all"` with the rationale comment
  committed in-config — the repo's previous default silently exempted the
  advisory class RUSTSEC-2026-0253 belongs to (informational/unsound).
- Red-first deny legs (cargo-deny 0.18.4, advisory-db ef6173c):
  LEG_A old lock + old policy → rc0 "advisories ok" (the blindness);
  LEG_B old lock + unsound=all → rc1 "advisories FAILED" (policy proof);
  LEG_C new lock + unsound=all → rc0 (closed).
- Union note for integration: sibling unlanded band rm-329 (bca4cee0) had
  claimed the lru bump alone; this row owns the NET-NEW policy half —
  reconcile by title at the commit gate, do not double-bump.

### rm-499 — pi `cacheWrite1h` drift (62.0, NOT selected; standing next-cycle lead)

Stays candidate. This cycle's batch was scoped to printed-truth + advisory
floor by prioritize 5ec011a3; rm-499 remains the smallest forward-pinning
schema-churn item (live pi-1.0.2 census: key present 4/4, value > 0 in 0/60
journals). Next cycle should re-census first — pi released 1.0.0→1.0.2 within
4 days at research time, so the drift may have gone live since.

## Transcribed validation outcomes (pre-review; NOT re-run at compound)

Targeted 99c5062a @ 511fb01 (porcelain 0 before and after): `cargo fmt
--check` rc0; `cargo clippy --locked -p agenttrace-core -p agenttrace
--all-targets -- -D warnings` rc0; `cargo test --locked -p agenttrace-core`
rc0 — suites 165/8/2/7/78/2/2 passed, 0 failed (lib, attribution,
codex_compaction, demo_contract, discovery, pi_family, workbuddy_usage_basis);
`cargo test --locked -p agenttrace` rc0 — 29/25/4/9 passed, 0 failed; `cargo
deny check advisories` rc0 "advisories ok"; `cargo build --release -p
agenttrace` rc0 in 36.96 s.

Full a26322ab @ 511fb01: CI-faithful execution of the complete
`.github/workflows/ci.yml` full job (19 steps in CI order) + the deny job —
all 20 gate lines rc=0 except 13-tui-real-smoke rc=skipped (CI-conditional
gate, unset `AGENTTRACE_TUI_REAL_CI_DIR`); 03-tests lane: 377 passed / 0
failed across 14 `test result: ok` lines (assess baseline 368/0 on 13 suites
at ea5c41e); 20-cargo-deny: advisories/bans/licenses/sources all ok;
check-plugin-version rc0 against the working tree (marker backfill present —
see lesson 3). Envelope digest
`validation:v1:d6221bb9de7128ea5bafee1684039ee8794b044a65eb6214abbc5a81e937568`
replicated byte-exact post-battery by the engine's own `validation_policy`
code from the (ea5c41e FULL sha, worktree) pair — this run's engine stamp is
replica-reproducible.

## Lessons and prevention rules (numbered)

1. **Acceptance-clause exemptions go stale — re-PoC every exempted lane.**
   rm-239's original acceptance asserted md/html were "already safe"; the
   assess format×surface matrix (1/1/2/0) falsified it at ea5c41e and the
   implement had to cover markdown_cell, markdown_inline_code and the html
   lane too. Compounded as a reusable rule:
   `docs/solutions/workflow-issues/acceptance-exemptions-go-stale-re-poc-exempted-lanes.md`.
2. **Engine salvage can move HEAD between phases; pin batteries to the actual
   tree, and treat an EMPTY dispatch `full_command` as "derive from the CI
   workflow", never as "no validation required".** The implement delta became
   commit 511fb01 mid-run while the dispatch block still said
   `changed_testable_surfaces=[]`; `crates/**` layouts classify nothing
   executable, so full's `full_command` was empty and the authoritative suite
   was derived from `.github/workflows/ci.yml` (full+deny jobs). PCR-9
   pattern; rm-018 owns the standing process note.
3. **Tag-namespace hazard (re-encounter; rm-163/rm-018 own it).**
   `check-plugin-version.sh` at PRISTINE 511fb01 is rc1 ("merged tag v0.7.2
   has no CHANGELOG section") — delta-independent, inherited-tag gap. The
   gate is green only against the 18-line `<!-- no-changelog-section -->`
   marker backfill sitting as UNCOMMITTED `CHANGELOG.md` drift in this
   worktree (authored by run 4a688257's full_tests b163e538; tail
   byte-identity to sibling run-4a68825724aa verified). That backfill MUST
   ship with this cycle's landing or every tag-complete environment stays
   red. Never edit CHANGELOG inside a validation gate to un-red it, and never
   `git fetch --tags` from one.
4. **/tmp PoC sandboxes are swept between phases (re-encounter).**
   `/tmp/at-research-97480` and `/tmp/at-assess-1f12` are gone; the research
   ledger survives in spool (97480e45-scratch). rm-497's PoC is re-derivable
   from the now-committed fixtures `tests/fixtures/workbuddy/*.jsonl`;
   rm-239's hostile fixture is re-derivable from the committed lane tests
   (`hostile_key_overview()` builder). Future phases: preserve PoCs in the
   delegate spool, not /tmp.
5. **Deny-policy decisions must be explicit in-config.** The default advisory
   scope silently passed an unsound transitive advisory for the lock's whole
   life; `unsound = "all"` + an in-config rationale comment makes the policy
   legible at the exact place a reviewer will look (rm-498, deny.toml).

## Next-cycle context (concrete)

- **Lead:** rm-499 (re-census pi ≥ 1.0.2 for `cacheWrite1h` going live, then
  consume-or-disclose with the rm-400/rm-497 counter pattern).
- **Assess residuals this cycle left open (no new ids — all claimed rows):**
  F2 — `-d <npm-home> --doctor` lists `./package.json ./tsconfig.json
  ./real-session.jsonl` as FAILED session files ("Session files: 3"): noise
  on implemented rm-369/370's `-d`-walk surface; dated append left on the
  rm-369 row. F3 — `--doctor <explicit-file>` silently widens to
  auto-discovery (22,032 files) where `--overview` bails loudly
  (main.rs:957): fresh corroboration appended to rm-367 (stages (b)/(c)
  strengthen alongside (a)).
- **Sequencing for the commit gate:** land the salvage tree + the CHANGELOG
  marker backfill + this record and the solutions doc together behind the
  review outcome; apply the compound ROADMAP postimage (supersedes the
  roadmap-phase patch as a strict superset — apply ONE, not both; a delta
  patch postimage→compound ships alongside for the already-applied case):
  all three shipped in `9d0f19d6…-scratch/` —
  `ROADMAP.after-compound-9d0f19d6.md` (postimage, 1306 lines / 147 rows,
  sha256 388c976aa4530511b7109d223a97a3211b66f4c386527ad846e2a09fb5277490),
  `roadmap-compound-9d0f19d6-cumulative.patch` (pristine fe8ae30 → e4bc753,
  +39/−1, `git apply --check` CLEAN vs the pristine worktree file), and
  `roadmap-compound-9d0f19d6-delta-from-3ba2e6d0.patch` (3ba2e6d0 postimage
  a155cbb → compound, +14/−3, applied-and-cmp'd BYTE-IDENTICAL to the
  postimage — use only if the roadmap-phase patch already landed).
  In-tree wall is pristine 144 rows / ceiling rm-402; the compound wall is
  147 rows / ceiling rm-499. Fleet numbering at compound time: reservations
  run to rm-550 (run 9873fc06), so any NEW mint starts at rm-551 after a
  live census — this compound mints ZERO ids.
