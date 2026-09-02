---
type: stewardship-record
id: cycle6-independent-review
cycle: 6
review-pass: 10
date: 2026-09-03
run: a24bcf084cf049208c75d2cb4f3a3755
attempt: 254503c297a44dedb929ee71c969b3d5
base_commit: 696206f
reviewed_tree: uncommitted working tree (+1098/−41 across 14 files, 3 untracked code/test assets)
status: completed
---

# Cycle-6 independent adversarial review (pass 10)

Independent re-verification of the CU-17..CU-22 implementation against the
cycle goals (prioritization doc), the security boundaries (AGENTS.md rules
1-4), durability/recovery requirements (cache bounds, schema compatibility,
atomic writes), and the test evidence claimed by the implement / targeted_tests
/ full_tests phases. Nothing from prior phases was trusted without re-running
or re-reading it; every behavioral claim below was reproduced live this pass.
No code was modified — findings are recorded for the commit gate.

## Verdict

**PASS WITH FINDINGS — one pre-commit fix required (F1), the rest
non-blocking.** The batch is substantively true to its goals: CU-17 fully
retires the CRITICAL runner contamination and adds a working guard; CU-18,
CU-20 and CU-21 were verified end-to-end live; CU-22 works but its "hard
bound" is approximate (F2). F1 is a one-line field-case defect in the new
CU-19 parser that silently drops real Antigravity VIEW_FILE steps — it does
not regress any pre-existing capability (the conversations root is new this
cycle), but the implementation record's claim about VIEW_FILE is falsified by
live evidence and no test covers it. It should be fixed before the commit
phase lands CU-19.

## Re-verified live this pass (evidence)

- `cargo test --workspace` → 212 passed, 0 failed (9 result-ok suites;
  `grep -c '^test .* ok$'` = 212). Matches the implement claim exactly.
- `cargo fmt --check` → clean. `cargo clippy -p agenttrace-core -p
  agenttrace-tui -p agenttrace -- -D warnings` → clean (CI invocation).
- `bash scripts/ci/check-no-self-hosted.sh` → exit 0; `bash -n` parses.
- `grep -n runs-on .github/workflows/*.yml` → exactly five hits, all
  `ubuntu-latest` (ci.yml:21, dependency-review.yml:16, release.yml:13/40/117).
  `git config branch.master.remote` → `fork`; `git remote -v` → origin =
  luoyuctl/agenttrace (upstream), fork = codeo1io/agenttrace. CU-17 acceptance
  met; the repoint is uncommitted local state (AGENTS.md rule 3 honored —
  `git diff` contains zero local-path/LAN/remote leakage: 0 matches for
  `/home/agent|/work/projects|tailscale|192.168.|codeo1io`).
- `--demo --audit -f json --sample 2 --sort cost --order asc` →
  `excluded_reason` = "sampled first 2 of 3 sessions in the --sort cost
  --order asc view (--sample 2)". CU-21 truthfulness confirmed, both strings
  (main.rs:251, main.rs:294) share the new wording and the entrypoints test
  pins both.
- `-d testdata --audit -f json` (release binary, rebuilt this pass) →
  gemini-2.5-flash row `tokens {input:160, output:80, reasoning:40}`,
  `reasoning_share_pct 50.0`, `pricing catalog_estimate`, cost $0.0002.
  CU-20 fold + breakdown + share reproduce from real output. (Note: the
  implementation record's evidence string cites the single-fixture numbers
  80/60/40; the `-d testdata` aggregate is 160/80/40 because two gemini
  fixtures live there. Capability confirmed; record's citation imprecise.)
- `--doctor` (live corpus) → 1416 session files, 745 entries / 341 dir
  listings / 9,387,541 bytes, and the new disclosure line naming both bounds;
  directory list shows `Gemini CLI`, `Antigravity CLI`,
  `Antigravity CLI conversations` (all "missing" on this host — expected, no
  `~/.gemini` corpus exists).
- `AGENTTRACE_BIN=target/release/agenttrace … scripts/ci/check-output-contract.sh`
  → exit 0 (doctor.json schema survives the new fields).
- CU-19's shape source re-checked against the agy-reader reference clone at
  `/tmp/agy-check/internal/daemon/types.go`: `userResponse` (:85),
  `thinking`/`response`/`toolCalls[].argumentsJson` (:96/:94/:105),
  `commandLine`/`proposedCommandLine`/`exitCode *int` (:111/:112/:116),
  `metadata.createdAt` (:72), `CORTEX_STEP_TYPE_*` discriminators — all
  match the parser's mappings except one (F1 below).

## Findings

### F1 — CU-19 VIEW_FILE wire key case mismatch (fix before commit)

`crates/agenttrace-core/src/parser.rs:513` reads
`step.pointer("/viewFile/absolutePathURI")`, but the wire key is
`absolutePathUri` (reference: `/tmp/agy-check/internal/daemon/types.go:151`,
Go field `AbsolutePathURI` with tag `json:"absolutePathUri"`). Live repro
this pass: a sidecar whose only step is a real VIEW_FILE
(`"viewFile":{"absolutePathUri":"file:///x.rs","startLine":1,"endLine":2}`)
fails to parse entirely → "No sessions match the requested filters"
(`non_empty(events)` → None at parser.rs:536); the same file with the
parser's wrong-case `absolutePathURI` parses. Mixed sidecars silently lose
every VIEW_FILE step (empty path → `continue` at parser.rs:517). The
implementation record's "VIEW_FILE → tool event with line range" claim is
therefore false for real data, and the CU-18/19 contract test only exercises
USER_INPUT + PLANNER_RESPONSE
(crates/agenttrace-core/tests/discovery_contract.rs:1980-1996), so the suite
cannot catch it. Fix: accept `absolutePathUri` (keep `absolutePathURI` as a
tolerated alias if desired) + add a VIEW_FILE step to the sidecar fixture.
Severity within the release is contained (new capability, no regression to
existing formats), hence High-for-CU-19 / non-blocking for the other five
units — CU-19 is explicitly severable per the stewardship request's
must-remain-separate rule 3.

### F2 — CU-22 byte bound under-counts; "hard bound" is approximate

`enforce_byte_bound` (crates/agenttrace-core/src/session_cache.rs:652-690)
estimates only the per-entry value serialization. It omits (a) every entry
path key and the envelope written by `save_session_cache`
(session_cache.rs:702-746), and (b) the entire `dirs` section
(session_cache.rs:728-740) — directory listings have no byte bound of their
own, only dead-dir pruning (session_cache.rs:419-428). Measured on the live
cache: file 9,387,541 bytes vs bound-visible 9,112,447 bytes → 275,094 bytes
(2.9%) invisible today, of which the `dirs` section is 188,305 bytes and
grows with directory-tree breadth, not session count. The doc comment
(session_cache.rs:30-38, "Hard bound on the serialized `sessions.json`
size") and the doctor line ("hard bounds: … bytes<=67108864",
doctor.rs:98-105/:282-284) overstate the guarantee: the written file can
exceed 64 MiB by the uncounted overhead. Not data loss (atomic temp+rename
write is intact; eviction bounds the dominant term and sets `dirty` so it
persists). Suggest: include path keys + a `dirs` allowance in the estimate
(or bound `dirs`), or soften the doctor wording to "entries bound".
Severity: Medium.

### F3 — CU-20 alias aggregation inconsistent across the three fold sites

`qwen_usage` SUMS all four thinking aliases (parser.rs:1970-1977), while
`gemini_usage` (parser.rs:3742-3752) and `usage_from_value`
(parser.rs:4118-4141) take the FIRST alias found. A payload carrying two
aliases with the same value (e.g. `reasoning_tokens` + `thinking_tokens`)
double-counts in the Qwen path only. No known current source does this
(Gemini's `thoughtsTokenCount` and DeepSeek-style flat `reasoning_tokens`
are single-alias and excluded from the completion count, so the fold itself
is correct for the documented shapes), but the three sites should agree —
prefer first-found everywhere, matching the existing `first_number`
convention in `gemini_usage`. Severity: Low.

### F4 — CU-19 unmapped step types dropped without disclosure

The reference schema defines content-carrying payloads the new parser
ignores: `CodeAction`, `GrepSearch`, `ListDirectory`
(/tmp/agy-check/internal/daemon/types.go:63-68), plus the object-shaped
`errorMessage.error.{user,model}ErrorMessage` (:199-209) which the parser
renders as a raw JSON dump (parser.rs:520-526, `jsonish`). The
implementation record files only the `.db`/`.pb` store as residual; these
fidelity gaps are neither mapped nor filed. Severity: Low (transcript
completeness for a brand-new source, no cost/metric impact — steps carry no
usage).

### F5 — CU-19 dispatch double-parses every single-object JSON session

`parse_antigravity_trajectory(raw)` (parser.rs:403-408) re-runs
`serde_json::from_str` on the full raw text although `parse_raw_session`
already holds the parsed `value` (parser.rs:95-110), and it runs before the
gemini/kimi/opencode/cursor value parsers, so every non-qwen/openclaw/hermes
JSON object session pays one extra full parse on first scan. CU-18 newly
admits all of `~/.gemini/tmp` (chats/checkpoints), increasing exposure;
the session cache keeps it to a one-time cost per file. Trivial fix: accept
`&Value`. Severity: Low (performance).

### F6 — CHANGELOG not updated (disclosed deferral)

The CU-20 acceptance in the prioritization doc asked for a "changelog note
that baselines shift"; CHANGELOG.md's Unreleased section has no cycle-6
entry. The implementation record discloses the deferral to the commit phase
and the precedent holds (CHANGELOG last touched by 998ade8, cycle 4;
cycle-5's commit 696206f did not touch it either — verified via
`git log --oneline -- CHANGELOG.md`). Fold into the commit phase's message.
Severity: Info.

### F7 — stewardship records carry operator-local references

`docs/stewardship/2026-09-03-cycle6-stewardship-request.md` (2 hits) and
`...cycle6-implementation-record.md` (1 hit) reference `/work/projects` and
`codeo1io` remotes. Correct as fork-local campaign records; they must never
ride the operator-gated upstream PR (C8) — already covered by the
must-remain-separate hint 5 and AGENTS.md rules 3-4, restated here so the
commit phase's split sees it. Severity: Info.

## Security-boundary review (AGENTS.md)

- Rule 3 (no runner overrides / local config in committed content): the
  committed diff is clean — zero `runs-on:` changes beyond the five
  `ubuntu-latest` reverts, zero local paths/hostnames/remotes in
  `git diff` or the three new untracked code/test assets
  (`scripts/ci/check-no-self-hosted.sh`, `testdata/gemini-thoughts-checkpoint.json`
  checked; the guard's failure text mentions no hostnames). The git tracking
  repoint exists only in `.git/config`. The guard itself would not catch a
  bare LAN-hostname label (e.g. `runs-on: agent-lan`) — it matches
  `runs-on:.*self-hosted` only; acceptable for the known contamination class
  (and un-generalizable without an allowlist), noted here as a hardening
  residual, not a blocker.
- Rule 1/2/4 (fork-only push, fork-base PRs, operator-gated upstream PR):
  honored — tracking targets `fork`; nothing pushed/PR'd this cycle; CU-17's
  guard plus the repoint are exactly the precondition the roadmap demands
  before any C8 conversation.

## Durability / recovery review

- Atomic write path unchanged (temp + rename; orphan sweep intact).
- Schema 17 additive change verified: `GoMetrics.TokensReasoning` carries
  `#[serde(default)]` (session_cache.rs:139-141) and the legacy-JSON test
  pins that pre-field caches deserialize to 0 — no cache invalidation storm,
  `check-docs-commands.sh`'s pinned "schema 17" claim stays true.
- Eviction ordering (oldest source mtime first) matches `enforce_entry_bound`;
  dropped entries set `dirty` so the eviction persists; idempotence and
  in-bounds cleanliness asserted by the new test
  (session_cache.rs:1370-1443). Only F2's estimate gap tempers the
  guarantee.

## Test-evidence assessment

- 212/212 re-run green this pass, fmt/clippy clean, guard clean, contract
  scripts exit 0 — the implement/targeted_tests/full_tests claims are
  accurate. The +9 test inventory matches the record (parser ×4 incl. the
  three fold sites + e2e, discovery ×2, session_cache ×2, governance ×1).
- Coverage gaps found: no VIEW_FILE/GREP_SEARCH step in any fixture (F1's
  hiding place), and no test pins that a single-alias payload with two
  reasoning keys behaves identically across the three fold sites (F3's).
  The targeted_tests phase's one gap fix (--compare twin pin) is real and
  present (entrypoints.rs:231-247).

## Provisional follow-ups (for the commit gate and cycle 7)

1. Commit phase: fix F1 (one line + fixture step) before landing CU-19;
   CU-17 must land as its own policy-only commit per must-remain-separate 2.
2. Cycle 7 candidates: F2 estimate fix (count path keys + `dirs`), F3
   unification, F4 remaining step types + errorMessage extraction, F5
   `&Value` refactor — all sub-half-day.
3. Guard hardening residual: consider allowlisting exact runner labels
   (`ubuntu-latest`, `macos-latest`, `windows-latest`) instead of the
   self-hosted-only deny pattern.
