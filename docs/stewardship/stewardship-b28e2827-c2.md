# Stewardship request — repository-maintenance b28e2827, cycle 2
Run 95e25b56f61549cdb344033458b11ce4 · stewardship attempt 9933c95582df4e4bb1f0e7b574591431 · 2026-10-05
Base: worktree e1d31c4 (porcelain ` M ROADMAP.md` = roadmap phase deliverable; this doc untracked). Canonical /work/projects/agenttrace @1511547 porcelain 0.

## stewardship_request

```json
{
  "title": "Upstream v0.10.1 truth-alignment: token-accounting port (rm-517) + OTEL dual-spelling (rm-519a) + installer honesty (rm-516)",
  "summary": "One coherent batch aligning the fork's core value — trustworthy per-agent token totals — with upstream's just-shipped v0.10.1 reference semantics, plus two small honesty riders. LEAD rm-517: port upstream #312 (merged 10:00:36Z cb625d73, shipped v0.10.1 10:06:38Z) onto this base's single-file parser — CC one-usage-per-message-id max-per-field; Codex sum-last-once-per-distinct-total + delta fallback; Kimi TokenUsage map single-count; Copilot last-shutdown-per-model + cache-subtract (OTEL dual-spelling rides here as rm-519a); Qwen promptTokenCount-minus-cache-reads; OpenCode tokens.reasoning=output — plus WorkBuddy reconcile (parser.rs:828 keeps FIRST via .or(latest_usage); upstream now sums every record — recommend follow-upstream, disclose); cache-schema renumber (fork SESSION_CACHE_SCHEMA_VERSION 22 vs upstream 26, SKIP 23 = claimed by the 700a67c lineage; SQLITE_SNAPSHOT_SCHEMA_VERSION fork 7 vs upstream 6 = explicit decision + force-re-parse on first run); Gemini truth fix — README.md:31 advertises 'Gemini CLI' but src/ has ZERO gemini code (verified: only two dead untested testdata fixtures), so remove the claim + delete the fixtures, matching upstream's v0.10.1 removal; README states the expected-total changes (CC/Codex LOWER, WorkBuddy HIGHER per upstream release notes). RIDER rm-516: install.sh:13 REF default v0.9.0 (two majors stale vs npm 0.10.1) → default ≤ latest tag with SHA-pinning preserved (keep refuses-unverified-master-tip); install.ps1:14 ProcessArchitecture switch gains the PROCESSOR_ARCHITEW6432/PROCESSOR_ARCHITECTURE string fallback accepting X64/AMD64 (exact PSReadLine-2.0.0-shadowing bug class upstream #314 fixed 12:02:37Z); version-pinning env per upstream #315 documented for sh+ps1; NEW scripts/ci/check-install-drift.sh pins REF ≤ latest tag. Acceptance centerpiece: totals reconcile against upstream v0.10.1 numbers on a golden corpus using ported upstream testdata fixtures (incl the NEW testdata/opencode/storage/message fixture).",
  "repositories": [
    "/work/projects/agenttrace",
    "/home/agent/.hermes/conductor-worktrees/agenttrace-80c75f65b7/run-95e25b56f615-95e25b56"
  ],
  "surfaces": [
    "crates/agenttrace-core/src/parser.rs::L800-807 (usage_totals dispatch) + L809 (fn usage_totals) + L828 (workbuddy keep-first .or(latest_usage)) + L2004-2022 (parse_details/otlp — Qwen arm + gen_ai.* dual-spelling point); all six per-agent arms ported in situ",
    "crates/agenttrace-core/src/session_cache.rs::L8 (SESSION_CACHE_SCHEMA_VERSION 22 → renumber, skip 23) + L9 (SQLITE_SNAPSHOT_SCHEMA_VERSION 7 — explicit decision vs upstream 6, force-re-parse)",
    "crates/agenttrace-core/testdata/ (NEW per-agent fixtures ported from upstream v0.10.1 testdata incl testdata/opencode/storage/message/ses_abc/msg_assistant.json; REMOVE dead gemini-checkpoint.json + gemini-current-chat.json)",
    "README.md::L31 (false 'Gemini CLI' sources claim — zero gemini code in src/) + totals-disclosure paragraph for expected-total changes",
    "install.sh::L13 (REF default v0.9.0)",
    "install.ps1::L14 (arch detect, no fallback) + pinning-env block",
    "scripts/ci/check-install-drift.sh (NEW gate: REF ≤ latest tag; ps1/sh defaults consistent)",
    "docs/stewardship/stewardship-b28e2827-c2.md (this request, untracked)"
  ],
  "must_remain_separate": [
    ["crates/agenttrace-core/src/parser.rs usage-accounting arms (this batch)", "crates/agenttrace-core/src/parser.rs disclosure/unknown-kind arms (66a7d797 lineage @700a67c, review-approved uncommitted)"],
    ["crates/agenttrace-core/src/session_cache.rs schema renumber (this batch)", "crates/agenttrace-core/src/discovery.rs walker admission + DIR_LISTING_WALK_VERSION (d6432dd5 lineage @700a67c, complete uncommitted; holds SESSION_CACHE=23)"],
    ["install.sh", "install.ps1"],
    ["README.md sources/truth paragraphs (this batch)", "crates/agenttrace-core/src/pricing.rs + crates/agenttrace-core/src/doctor.rs (4a688257 same-base riders)"],
    ["scripts/ci/check-install-drift.sh (NEW)", "scripts/ci/check-docs-commands.sh (untouched by this batch)"]
  ]
}
```

## Contract notes (binding on implement)

1. **Do NOT touch fleet-owned lanes**: `pricing.rs`, `doctor.rs` (4a688257's same-base riders, not yet started — leave free); `reports.rs`, `discovery.rs`, `insights.rs`, `csv_export.rs`, `main.rs` render arms (d6432dd5's complete-uncommitted "hardened reporting surfaces" @700a67c, +498/−23, 15 files + tests/fixtures/hostile-overview/). This batch needs none of them.
2. **Schema renumber discipline**: SESSION_CACHE 22 → **24** (23 is content-claimed by the 700a67c walker lineage); force-re-parse on first run must be tested. SQLITE_SNAPSHOT stays 7 OR renumbers — either way an explicit code comment + test documents the upstream-6 divergence.
3. **WorkBuddy semantics**: recommend following upstream sum-every-record (v0.10.1 shipped reference; maintainer-confirmed #310 fix). Whichever is chosen, disclose it in README alongside the expected-total changes.
4. **Gemini**: verified this phase — `grep -ri gemini crates/*/src/` = ZERO hits; only testdata/gemini-{checkpoint,current-chat}.json exist (referenced by no test). Honest fix = remove README.md:31 claim + delete both fixtures. No parser code change.
5. **Test baseline**: 395 passed / 0 failed across 15 targets at e1d31c4 (/tmp/at-assess-b481f5f9/tests.log). Gates: fmt, clippy -D warnings, check-docs-commands.sh (README claims are CI-gated), full workspace tests green with the ported fixtures.
6. **CI wiring** for check-install-drift.sh belongs to the ci phase, not implement (phase boundary).

## Verification legs (this phase)
- Sibling format precedent: ../run-4a68825724aa-4a688257/docs/stewardship/stewardship-0937d5fb-c2.md (read; same schema).
- Surface anchors grepped live: parser.rs L800-807/L809/L828/L2004-2022; session_cache.rs L8=22/L9=7; README.md L31 'Gemini CLI'; install.sh L13 REF v0.9.0; install.ps1 L14 ProcessArchitecture; scripts/ci/ = check-deterministic-output.sh + check-docs-commands.sh only (no drift gate exists).
- Gemini-absence proof: `grep -ril gemini crates/ | grep -v target` → only the two testdata JSONs; `grep -ri gemini crates/agenttrace-core/src/` → zero.
- Canonical state: /work/projects/agenttrace @1511547 porcelain 0 (dirty-state preservation is Conductor's; topology decision NOT made here).
- Collision state (from prioritize 26759cb, re-verified): d6432dd5 = M ROADMAP.md + 15 M source + 2 ?? fixtures (uncommitted, holds reporting/walker/session_cache-23 lanes); 4a688257 = M ROADMAP.md + ?? docs/stewardship only.
