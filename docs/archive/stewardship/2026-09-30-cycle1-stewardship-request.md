---
artifact_contract: "ce-handoff/v1"
created_at: "2026-09-30T07:14:07Z"
title: "Stewardship request — campaign 25898d03 cycle 1: trustworthy strings, trustworthy tokens"
summary: "Hands the conductor the selected rm-012 + rm-013 batch as two repository change units with live file:line surfaces, overlap facts, dirty state to preserve, and separation hints; makes no Git-topology decisions."
keywords: ["agenttrace", "stewardship-request", "terminal-injection", "codex-high-water", "change-units", "rm-012", "rm-013"]
cwd: "/home/agent/.hermes/conductor-worktrees/agenttrace-80c75f65b7/run-88feec46e4fa-88feec46"
resume_focus: "Establish the stewardship contract for the rm-012 + rm-013 cycle-1 batch: inventory this repository and the campaign worktree, detect overlap (PR #15 conflicts on sibling files; uncommitted ROADMAP.md roadmap delta; untracked cycle docs), split unrelated concerns (two independent units in disjoint files), preserve the dirty state listed below, and plan branches/worktrees before implementation begins."
repository: "codeo1io/agenttrace"
repo_root_sha: "7bb4dcbe40dbb78b5043a0f47f7d05191f9381ac"
branch: "master"
head: "7bb4dcb"
---

# Stewardship request

This document is a **request**, not a contract. Per the conductor work order
(run `88feec46e4fa49f4949fcc86a3eab277`, phase stewardship, attempt
`a5ffb75db664412cbf2527103ea25970`), it describes the selected maintenance
batch and stops there: **it chooses no branch, worktree, commit order, or
any other Git topology.** Those decisions belong to the conductor.

Routing note: no ce-* skill is installed in this delegate's pi session
(only `agent-reach` is exposed), so per the standing precedent this request
was produced directly on the work-order contract, mirroring the
`ce-handoff/v1` frontmatter and pointer-first body of the prior
`docs/stewardship/2026-09-02-cycle-2-stewardship-request.md`. Harness
disclosure: this delegate session has no subagent surface; everything here
was produced in-thread.

## Title

agenttrace campaign `25898d036bf4457c88b319e9bddb1f94` cycle 1 —
"trustworthy strings, trustworthy tokens" (`rm-012` + `rm-013`).

## Summary

Implement the two-unit batch selected by the prioritize phase
(`docs/stewardship/2026-09-30-cycle1-prioritization.md`):

- **Unit A — `rm-012`, statusline-report sanitization (security, 89.0).**
  `agenttrace --statusline-report` prints journal-derived strings raw:
  `session_id` at `statusline.rs:615`, joined `last_miss_causes` at `:628`,
  and `miss_causes` keys at `:636-644`. The render path sanitizes exactly
  this class via `sanitize_line_segment` (`:263-268`), and ingestion
  (`append_statusline_capture`, `:275`) persists hostile payloads verbatim,
  so a crafted statusline stdin once captured emits ANSI SGR / OSC-52
  (clipboard exfiltration vector) on every later report run. Live-reproven
  this campaign (assess attempt `6c039674`): crafted journal payload →
  `agenttrace --statusline-report | cat -v` showed raw `^[[31m` and
  `^[]52;c;...^G`. The TUI panel is numeric-only
  (`presentation.rs:1302` context) and the JSON path serde-escapes; both
  are unaffected.
- **Unit B — `rm-013`, Codex cumulative-token high-water mark
  (correctness, 88.0).** `parser.rs:2257`
  (`codex_token_count_usage`) computes
  `token_usage_delta(&total, prev_total)` on the raw cumulative
  `total_token_usage`. Codex rewinds that cumulative after compaction; when
  it climbs back, the rebound is counted a second time — inflating token
  totals and estimated costs for every compacted Codex session. Upstream
  fixed this in `be25c4c` (PR #286, merged 2026-09-30, present in this
  repository's local object store as `git show be25c4c`): a
  `token_usage_high_water` helper so rewound totals are never re-counted.
  Our fork point `6848aa1` predates it; verified absent at HEAD
  (`grep 'high_water' crates/agenttrace-core/src/parser.rs` → no matches).

Both units are defect-class reliability work, provable offline on this
host, with no dependency between them and no dependency on the open,
conflicting sibling PR #15.

## Change units and surfaces

### Unit A — rm-012 (statusline-report sanitization)

Surfaces (all in `crates/agenttrace-core/src/statusline.rs`):

- `:263-268` — `sanitize_line_segment`: existing control-character
  sanitizer to reuse (shared helper; do not fork a second
  sanitizer).
- `:615`, `:628`, `:636-644` — the three raw print sites in
  `render_statusline_report` to route through sanitization.
- `:650` (`mod tests`) — home for the regression test; existing
  count 7.
- Context (out of Unit A's mutation scope, documented for the
  implementer): `:275` `append_statusline_capture` keeps storing raw
  payloads — the journal is data, sanitization belongs at every print
  site.

Acceptance flavor: a journal payload containing ESC/OSC sequences appears
sanitized in `--statusline-report` output (`cat -v` shows no raw `^[`);
JSON output unchanged (serde escapes); render path unchanged.

### Unit B — rm-013 (Codex high-water mark)

Surfaces (all in `crates/agenttrace-core/src/parser.rs`):

- `:2257` — `codex_token_count_usage` delta computation: replace
  raw-total delta with high-water accounting (reference: `git show
  be25c4c`, function `token_usage_high_water`).
- `:2090`, `:2137-2139` — `prev_token_total` threading sites that
  carry the previous cumulative; the high-water state rides this
  existing plumbing.
- `:4426` (`mod tests`) — home for the rewind fixture regression test;
  existing count 13.

Acceptance flavor: a fixture whose `total_token_usage` rises, rewinds
(compaction), and rises again yields exactly the sum of true per-event
deltas — single-counted.

## Overlap facts (verified this campaign)

- **Sibling campaign cc2f32d5 / PR #15** (`conductor/run-cc2f32d56918`,
  head `9eba8cf`, OPEN + CONFLICTING vs master):
  `git diff --name-only 90a4ef5..9eba8cf` → 8 files — `diagnostics.rs`,
  `governance.rs`, `lib.rs`, `waste.rs`, `presentation.rs`,
  `explorer.rs`, `install.ps1`, `ROADMAP.md`. **Neither `statusline.rs`
  nor `parser.rs` is among them** — both units are merge-clean against
  PR #15.
- **Worktree dirty state (must be preserved, commit-gate owned):**
  ` M ROADMAP.md` (roadmap-phase deliverable, +74/-0, items
  rm-012..rm-023 added) and untracked
  `docs/stewardship/2026-09-30-cycle1-prioritization.md` plus this
  document. Implementation phases must not revert, restage, or fold
  these away.
- **ROADMAP.md entries** `rm-012` at `:54` and `rm-013` at `:60` remain
  `status: candidate`. Status flips belong to the commit gate, not to
  implementation.

## Separation hints (must_remain_separate)

1. `rm-012` unit (`crates/agenttrace-core/src/statusline.rs` and its
   inline `mod tests`) vs `rm-013` unit
   (`crates/agenttrace-core/src/parser.rs` and its inline `mod tests`):
   disjoint files, independently reviewable, revertable, and landable.
   Do not merge them into one commit-sized blob.
2. Batch surfaces vs PR #15 surfaces (`diagnostics.rs`, `governance.rs`,
   `lib.rs`, `waste.rs`, `presentation.rs`, `explorer.rs`,
   `install.ps1`): keep the cycle off those files entirely. In
   particular, put new tests in each unit's inline `mod tests`, **not**
   in `lib.rs` (PR #15's test module lives there).
3. Implementation edits vs `ROADMAP.md` status fields: implementers do
   not flip `rm-012`/`rm-013` to done; the commit gate owns flips.
4. Live repro scratch (crafted statusline journals, fixture corpora)
   belongs under `/tmp` or disposable cache roots — never inside the
   repository and never in the operator's real
   `~/.cache/agenttrace/statusline.jsonl` (the assess-phase probe entry
   was already removed; tests must use env-overridable cache roots).

## Out of scope (recorded, not selected)

- `rm-014` (parse-once JSONL probing) deliberately sequenced after
  `rm-013` — landing both in one cycle would muddy `rm-013`'s
  before/after fixture evidence (same file, same fixtures).
- PR #15 merge/resolve window: a commit-gate concern, surfaced as
  provisional future work, not part of this batch.
