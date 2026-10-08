# Cycle-1 compound record — run 749cd29820f34d53980488c59783504b

- **Run:** 749cd29820f34d53980488c59783504b (repository-maintenance dda534730da4402c848059a7ceaf4e5f, cycle 1)
- **Base:** a6a1f26f643311d3efb39c08169f307ef4dea989, worktree `run-749cd29820f3-749cd298` (branch lineage per work order; no commits made this run — the commit gate owns landing)
- **Compound attempt:** 52243a6cdde04c42a1f09666df6df008 (2026-10-08, after full_tests 2152ef02)
- **Status at compound:** implemented, pre-review. This record consumes RECORDED pre-review
  evidence only — no test/validation command was executed at compound, per the compound contract.

## Prior-attempt forensics (this run)

Four attempts died of infrastructure (provider) failures; TWO of them left real, verified,
adoptable work — the first run in the fleet ledger where a reaped attempt's *detached
runner*, not just its tree drift, outlived the reap:

| Phase | Dead attempt | Disposition |
|---|---|---|
| prioritize | 51984e11 | provider-dead at message 4; event log = pings only, no envelope, no scratch; REDONE from scratch (1ccf9f77) |
| implement | 44a88914 | provider-reaped at 24 messages, envelope absent — but left the COMPLETE code delta in the worktree (main.rs +~160 dispatch/config reorder, mcp.rs +~300 wire/protocol hardening); drift verified against the work-order lineage then **ADOPTED** by a0ef8a86 (73fe8e1e adoption rule: lineage identity is the gate, the tree trail is only the candidate) |
| full_tests | cd85a837 | provider-reaped 05:58:36Z, envelope absent — but its backgrounded runner survived the reap and finished ALL 23 ci.yml lanes rc=0 at 05:56Z, two minutes BEFORE the reap; **ADOPTED** by 2152ef02 after five verification legs (script identity / independent lane re-derivation / tree immobility / log genuineness / digest re-derivation). Same-day fleet confirmation: f4df3f3f (run c0f141d1, identical pattern), original 9a4d37af |
| compound | 6394648a | provider-dead at 3 messages; event log 4 events = heartbeats; envelope absent; tree byte-identical to the full_tests handoff (whole-diff sha256 ec2e9985608b5f073ecc3d90b33c8373181570c3b3f0a1df250f6ea072b5a9de); zero durable work — REDONE from scratch (52243a6c, this attempt) |

The reap-triage doc is extended with a dated rider for the two adoption classes:
`docs/solutions/workflow-issues/provider-reaped-delegate-attempts-redo-from-scratch-on-census-match.md`.

## What shipped this cycle (pre-review, uncommitted)

**Batch 'MCP surface hardening'** (prioritize 1ccf9f77; stewardship 575d89d3; ONE change-unit):

- **rm-781 LEAD (correctness 84.0) — dispatch-seam parity.** The `mcp` dispatch arm
  (crates/agenttrace-cli/src/main.rs:339, behind the rm-573 `!statusline_report` guard) now
  runs `config::resolve` + `set_runtime_config` (:361-362) BEFORE the server starts, so
  file-layer config (user/project/`--config` pricing overrides, `--history-dir`) reaches the
  MCP tools byte-identically to the CLI. Flags placed BEFORE the `mcp` keyword are honored
  (`-d/--dir` plus `--config`/`--history-dir`/`--pricing-file`) or refused at rc2
  (`refuse_inapplicable_mcp_flags`, main.rs:959) — previously clap accepted them and the
  dispatch arm silently dropped them. The empty-vs-range-excluded distinction mirrors the
  CLI's two messages (discovery miss vs filter miss naming the discovery count). All five
  falsified parity claims rewritten true across README.md / README.zh-CN.md /
  docs/guides/mcp-server.md / CHANGELOG.md.
- **rm-780 (compatibility 71.0) — protocol-version currency.**
  `SUPPORTED_PROTOCOL_VERSIONS = ["2026-07-28", "2025-11-25", "2025-06-18"]`
  (crates/agenttrace-cli/src/mcp.rs:78), negotiated newest-mutual in `initialize_result`
  (:367); unknown client versions fall back to newest-supported with a one-line stderr
  disclosure (:372-373) — never a blind echo (assess F5a: `1999-99-99` was echoed
  verbatim as "supported").
- **rm-782 (reliability 63.0) — wire discipline.** Byte-level `read_wire_line`
  (mcp.rs:204) over `WireLine::{Bytes, Oversized, Eof}` with a 1-MiB `MAX_LINE` cap:
  non-UTF-8 and oversized lines answer -32700 and the server stays up (previously a
  non-UTF-8 line was an rc1 fatality with no response). Request ids validated to
  String|Number|Null (others -32600 with `id: null`); non-object `params`/`arguments`
  answer -32602 instead of behaving like absent values; the `inputSchema` enum and the
  -32602 message publish the parser's full `--range` alias set verbatim, and
  schema-vs-parser equivalence is locked by test.

Test census: 9 added `#[test]` = `mcp_lane_refusal_passes_the_honored_flag_family`
(main.rs unit) + 8 e2e arms in tests/mcp_server.rs
(`protocol_version_is_negotiated_never_echoed`, `hostile_wire_lines_answer_32700_and_stay_up`,
`oversized_line_answers_32700_and_stays_up`, `invalid_request_shapes_answer_spec_errors`,
`range_alias_schema_matches_the_enforced_parser`,
`inapplicable_flags_before_the_keyword_are_refused`, `dir_flag_scopes_the_server_corpus`,
`config_layer_prices_the_mcp_tools_identically_to_the_cli`) + 3 re-pins riding the rebuilt
harness. Zero new dependencies (rm-455's hand-rolled decision stands; rmcp adoption stays
open as evolution on rm-455's rider).

Worktree delta at compound: **8 M** (CHANGELOG.md, README.md, README.zh-CN.md, ROADMAP.md,
crates/agenttrace-cli/src/main.rs, crates/agenttrace-cli/src/mcp.rs,
crates/agenttrace-cli/tests/mcp_server.rs, docs/guides/mcp-server.md) + this record and the
reap-doc rider as new/modified untracked-era paths (docs/**). +1153/−121 tracked before
compound's own ROADMAP/docs additions. No commits — the commit gate owns landing.

## Recorded validation outcomes (NOT re-run at compound)

| Phase | Attempt | Recorded outcome |
|---|---|---|
| implement | a0ef8a86 (adopting 44a88914) | cargo test -p agenttrace 141/0 (63 unit + 7 csv_export + 1 launch_guards-family + 43 entrypoints + 4 mcp_server-suite-baseline + 12 upstream + 9 warm_cache_pricing + 2 +9-new — final suite line 141 passed / 0 failed); fmt rc0; clippy -p agenttrace --all-targets -- -D warnings rc0; release build rc0 + docs gate rc0; PoC replays on the release binary ALL green: CLI total_cost 0.003 == MCP 0.003 under the same --config override; initialize 1999-99-99 → 2026-07-28 + one-line stderr disclosure; \xff\xfe line → -32700 then next ping served; `--overview mcp` → rc2 refusal |
| targeted_tests | b2624f8b | focused battery over the changed surfaces, 141/0 ×2 independent runs; fmt rc0; clippy rc0; docs gate rc0 on a fresh release build; porcelain identical 8 M in/out — zero fixes, zero executable changes this turn, digest copied VERBATIM from the work order per the emission-time rule. SCOPE DECISION recorded: engine-derived `changed_testable_surfaces=[]`/`required_scope=none` contradicted the tree; battery ran over the real delta per 2854c56d/73fe8e1e precedent (third fleet confirmation) |
| full_tests | 2152ef02 (adopting cd85a837's runner) | ALL 23 ci.yml push-surface lanes rc=0 (lanes independently re-derived from .github/workflows/ci.yml at adoption; TUI-real lane is var-gated out in ci.yml itself): workspace trio 611/0 = assess pristine 602 + exactly the 9 added tests (count reconcile exact); -p agenttrace 141/0 == implement final; entrypoints 43/0; cargo deny 4/4 ok; MSRV 1.88 lane rc0; release build + docs gate rc0; runner finished 05:56Z, reap 05:58:36Z |

Digest lineage: `validation:v1:026515da726c99fef393bf66371c25fa64ba23e84fc2caf29f4c00f6de903ba0`
re-derived live at full_tests == dispatch VERBATIM; porcelain census 8 M stable
implement → targeted → full → compound. Compound touches ROADMAP.md + docs/ only —
non-executable under the engine classifier (validation_policy.py:35-42 floors crates/**)
— so the digest is immobile across this phase and stays current for final_validation.

## Id accounting (live sweep at compound, 2026-10-08)

- ZERO ids minted at compound. This run's campaign-local band: rm-780 / rm-781 / rm-782
  (roadmap cb510878 @ a6a1f26).
- Landed ceiling moved during the run: origin/master a6a1f26 → 96b528e → 6e83d9a → … →
  **65f9f63**; landed wall def-max **rm-800** — but the rm-780..rm-782 numerals are STILL
  UNCONSUMED on the landed wall (def-row greps for all three return empty; the fleet's 783+
  bands numbered past this band at mint time). This band therefore lands as-is unless a
  same-numeral landing races; if one does, reconcile by TITLE (880a7b9e discipline), never
  by numeral.
- Unlanded worktree ceilings (live): rm-840 (run-c0f141d1), rm-839 (run-fabd9fb8), rm-833
  (run-aa41d9b5), plus lower bands (rm-826, rm-824 ×2). Spool frontier **rm-842** → next
  free rm-843 after a fresh live census.
- Wall accounting after the compound flips: 256 one-line def rows → **130 candidate /
  69 implemented / 57 done** (rm-780/781/782 candidate→implemented with EXECUTED addenda;
  done flips reserved to the commit gate, rm-012 precedent).

## Commit-gate seams (re-verify live at landing)

1. **Stage explicitly (`git add -A`; `git commit -am` DROPS untracked paths)** — the delta
   is the 8 M files + `docs/stewardship/2026-10-08-cycle1-compound-record-run749cd29820f3.md`
   + the dated rider on
   `docs/solutions/workflow-issues/provider-reaped-delegate-attempts-redo-from-scratch-on-census-match.md`.
2. **Rebase onto 65f9f63+** and re-anchor: `mcp.rs` carries rm-548 threading at origin
   :293-306 over this base (title-disjoint — sequence, don't blind-merge);
   `main.rs` dispatch arm + rm-573 guard literals have moved in landed lanes (origin :339
   dispatch / :379-380 install region / :3422 self-test at stewardship-time tip).
3. **Integration sequences the mcp.rs seam against unlanded rm-789** (run-652a6461's MCP
   2026-07-28 server/discover feature) BY TITLE.
4. Done-flips for rm-780/781/782 at the gate (rm-012 convention); CHANGELOG already
   rewritten in-batch (no gate-time bullets owed for this batch).
5. Re-verify the landed ceiling live before anything next mints (frontier ≥ rm-843).

## Next-cycle context (cycle 2)

- **Lead candidates: rm-455's remaining tool-scope arms** (session list/detail + doctor
  verdict) on the now-hardened seam — designated by prioritize 1ccf9f77; the parity,
  refusal and wire harness built this cycle (tests/mcp_server.rs) is the fixture base.
- **rmcp SDK adoption** stays open on rm-455's rider: under rmcp, spec bumps are
  dependency bumps; hand-rolled, every protocol revision is a code change rm-780 owns.
  Either way rm-780's negotiation semantics are now spec-correct and pinned.
- **Watches carried** (research f7f08c69 pass 11): upstream luoyectl tip 15ed07f
  (2026-10-06T14:49Z, 30 commits ahead at fetch time); npm @zack78/agenttrace v0.10.1
  (bare name still 404); LiteLLM 4,486 keys (2026-10-08) vs the bundled 3,100-row snapshot
  (rm-006's refresh lane); ccusage #1771 open; OTel semconv-genai 0 tags (rm-229 trigger
  untripped); serde_json 1.0.150 shortest-round-trip float-parse defect is fleet-recorded
  by run 9bf79afa (warm≠cold --sessions --format json) — candidate 65-adjacent, NOT this
  band.
- **Cede ledger unchanged** from prioritize 1ccf9f77 (rm-195 / rm-251 / rm-421 / rm-239 /
  rm-448 / rm-232 / rm-053, each with its reason at the prioritize banner).
- Review and shipping outcomes deliberately absent here — they land after this phase; the
  next cycle's assessment carries them forward.

## Prevention rules (fleet-lettered; continuing from PR-T..PR-V in
docs/stewardship/2026-10-07-cycle3-compound-record-run90f54faf.md)

- **PR-W — every new entrypoint arm earns runtime-config parity and a seeded-home A/B
  test.** The F1 class: an arm dispatched before `config::resolve`/`set_runtime_config`
  silently starves every runtime-config consumer (pricing layers, history dir) while clap
  still accepts the flags — the failure is invisible to every unit test that doesn't load
  config. Rule: a new keyword arm installs runtime config before doing anything else,
  honors the flags that CAN apply, refuses the rest at rc2 (rm-573 family), and its e2e
  harness pins parity with the CLI under a config-layer override in a seeded HOME
  (`config_layer_prices_the_mcp_tools_identically_to_the_cli` is the template). Doc-surface
  parity claims are falsifiable the same way — five were falsified here and rewritten.
- **PR-X — wire/stdio readers on untrusted streams read BYTES, not lines.** `BufRead::
  lines()` over hostile stdin is two defects in one call: a non-UTF-8 byte fatality (rc1,
  no response, host hangs on a dead server) and an unbounded-length allocation. Rule: any
  reader on an untrusted stream returns bytes with a decode fallback to a protocol error
  plus a hard length cap (`read_wire_line`/`WireLine::{Bytes, Oversized}`/1-MiB MAX_LINE is
  the in-tree template). Extends
  docs/solutions/reliability/untrusted-inputs-need-budgets-before-they-need-parsing.md from
  budgets to byte-level decode discipline.
- **PR-Y — reaped ≠ unstarted: triage tree drift AND detached artifacts before redoing.**
  Third fleet confirmation of the survivor pattern (9a4d37af; cd85a837 here; f4df3f3f
  same-day on run c0f141d1): a provider reap kills the session, not the detached runner —
  hour-scale batteries must run backgrounded with logs and CI_OUT under /tmp named for the
  attempt, and a successor must SWEEP for completed survivors before re-running. The twin
  case is tree drift (44a88914 here, 05563e2a on 73fe8e1e): a reaped attempt's uncommitted
  delta can be complete work — adopt only after work-order-lineage verification, and
  declare the legs. Triage order: typed envelope → scratch dir → event tail → tree drift →
  detached /tmp artifacts. Recorded in the dated rider on
  docs/solutions/workflow-issues/provider-reaped-delegate-attempts-redo-from-scratch-on-census-match.md.

## Verification appendix (how to check this record)

- `grep -n 'compound c1 (2026-10-08, run 749cd29820f3' ROADMAP.md` → the banner,
  newest-first above the prioritize designation banner; `grep -c '^- id: `rm-' ROADMAP.md`
  → 256; def-row status census → 130 candidate / 69 implemented / 57 done; duplicate-id
  check and managed-footer-last check as ever.
- `git status --porcelain` → the 8 M batch files + ROADMAP.md + the two docs riders
  (this file untracked, the workflow-issues file modified); `git diff --stat` →
  +1153/−121 pre-compound plus compound's ROADMAP/docs additions.
- Recorded outcomes: delegate envelopes on the spool (a0ef8a86, b2624f8b, 2152ef02) and
  their scratch records; full-suite lane logs mirrored at
  `delegate/2152ef023bb1420eb44f807ccf903a7c-scratch/lanes/` (originals /tmp/at-full-cd85,
  sweep-volatile).
