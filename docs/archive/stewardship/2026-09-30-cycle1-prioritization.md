---
artifact_contract: "ce-handoff/v1"
created_at: "2026-09-30T07:09:47Z"
title: "Cycle 1 prioritization (campaign 25898d03) — trustworthy strings, trustworthy tokens"
summary: "Scores all 15 open ROADMAP.md items after this campaign's assess pass (7 findings F1-F7 at HEAD 7bb4dcb), research pass (8 candidates RC-1..RC-8), and roadmap extension (rm-012..rm-023) by impact, risk-if-deferred, effort, dependency, and strategic value; selects the cycle-1 batch rm-012 statusline-report terminal-injection sanitization + rm-013 Codex token high-water fix, with rejected-alternative analysis (rm-011 deferred a third time, rm-014 sequenced behind rm-013, rm-015+rm-016 as the cycle-2 pair), merge-safety against sibling PR #15 verified by file-set diff, and a red-to-green execution order."
keywords: ["agenttrace", "cycle-1", "prioritization", "terminal-injection", "codex-high-water", "statusline", "token-accounting"]
cwd: "/home/agent/.hermes/conductor-worktrees/agenttrace-80c75f65b7/run-88feec46e4fa-88feec46"
repository: "codeo1io/agenttrace"
repo_root_sha: "7bb4dcbe40dbb78b5043a0f47f7d05191f9381ac"
branch: "conductor/run-88feec46e4fa"
head: "7bb4dcb"
---

# Cycle 1 prioritization (run 88feec46, attempt 51899064)

- run: `88feec46e4fa49f4949fcc86a3eab277`, attempt
  `51899064dede499dabdbfff28fd68de3`
- intent: `prioritize_repository_maintenance`
- grounding: HEAD `7bb4dcb` (PR #14 merge: rm-009 rustls lockfile patch +
  rm-010 cargo-deny lane, committed). Working tree carries this run's
  roadmap-phase edit to `ROADMAP.md` (+74/-0, 12 new candidate items,
  uncommitted, commit-gate-owned) and nothing else. Baseline re-verified
  this campaign: 236/236 tests, clippy `-D warnings` clean, fmt clean,
  all 12 check scripts green. The headline reproducer (F1) was
  demonstrated live this campaign on the release binary.
- inputs: `ROADMAP.md` (17 items: 2 done, 15 open), assess pass
  (`88feec46-assess-findings.md`, F1–F7 + the spool JSON), research
  pass (`88feec46-research-candidates.md`, RC-1..RC-8 + the spool
  JSON), the cc2f32d5 sibling campaign's PR #15 state, live probes
  (`gh pr view`, file-set diffs).

**Skill note.** Same disclosure as every prior cycle in this lineage:
no compound-engineering router is installed in this delegate's pi
session (only `agent-reach`, an internet-research skill — used by the
research phase, not here), so the nearest historical match (`ce-plan`)
would be a heavier implementation-plan workflow than a scoring and
selection pass. Scoring ran in-thread: every open item scored on
impact (1–5), risk-if-deferred (1–5), effort (XS–L), external
dependency, and strategic value to the product's two jobs (cross-agent
session analytics; truthful cost/token reporting), under the hard
constraint that the batch be completable and verifiable **end to end
on this host** (offline Linux, no Windows runtime, no CI execution in
this phase).

---

## 1. Verdict — cycle-1 batch: "Trustworthy strings, trustworthy tokens"

> **Selected:** **rm-012** statusline-report terminal-control-injection
> sanitization + **rm-013** Codex cumulative-token high-water fix.

Two change units, one theme, zero external dependencies, every
acceptance criterion verifiable offline on this host. The theme is the
direct continuation of this repo's cycles 2–4 lineage ("trustworthy
strings on untrusted input", "truthful reads, truthful gates"):
**rm-012** closes a live-proven escape of hostile strings into a
terminal (ANSI SGR + OSC-52 clipboard exfil through
`agenttrace --statusline-report`), and **rm-013** closes silently
wrong numbers on the product's flagship surface (Codex sessions that
compact get their rewound token totals re-counted on rebound,
inflating token counts and estimated costs).

Why this batch wins every test applied to it:

- **Both units are the top two priorities on the board** (89.0 and
  88.0 — the only items above rm-011's 81.0 and rm-002's 83.0 that are
  also small). Both are defect-class, reliability-track; neither is a
  capability want.
- **Neither rots if taken now; both compound if deferred.** F1's
  hostile payloads are *persisted verbatim* into the journal
  (`append_statusline_capture`, statusline.rs:275), so every day the
  fix waits, hostile strings accumulate in users' caches waiting to be
  replayed by the next report run. rm-013's over-counting widens with
  every compacted Codex session added to a corpus.
- **Both have their acceptance infrastructure already in hand.** The
  F1 repro harness was built and run this campaign (crafted journal →
  `--statusline-report` → `cat -v` shows raw ESC/OSC bytes); the fix
  is reuse of the tree's own `sanitize_line_segment`
  (statusline.rs:263-268) at the three raw print sites
  (:615/:628/:636-644) plus a regression test. rm-013's reference
  implementation exists **in the local git object store**
  (`git show be25c4c` — upstream #286, merged 2026-09-30); our tree's
  pre-fix pattern is pinned at parser.rs:2257 and the surrounding
  threading at :2090/:2137-2139, so the port is a ~15-line accounting
  change plus a rewind fixture.
- **Merge-safe against the sibling campaign.** `git diff --name-only
  90a4ef5..9eba8cf` (PR #15's full file set: ROADMAP.md,
  diagnostics.rs, governance.rs, lib.rs, waste.rs, explorer.rs,
  presentation.rs, install.ps1) touches **neither statusline.rs nor
  parser.rs**. The batch lands independently of PR #15's
  open-and-conflicting state. (rm-013's new tests go in the parser
  module, not lib.rs, to keep even test-file overlap at zero.)
- **It is simultaneously highest-value and end-to-end completable
  here** (see §4 for the alternatives that fail one of the two).

## 2. Scoring — the open board (15 items)

Imp = impact (1–5) · Risk = risk if deferred · Eff = effort · Dep =
external dependency · Strat = value to the two product jobs.

| # | Item | Imp | Risk | Eff | Dep | Strat | Disposition |
|---|---|---|---|---|---|---|---|
| rm-012 | `--statusline-report` prints journal-derived session_id/miss_causes raw (statusline.rs:615/628/636-644) while render path sanitizes (:263-268); live ANSI+OSC-52 repro; ingestion persists hostile payloads (:275) | 5 | 4 | S | none | 4 | **batch (lead)** |
| rm-013 | Codex cumulative total re-counted on rewind-rebound after compaction (parser.rs:2257); upstream fix be25c4c in local objects; estimated costs inflated for compacted sessions | 5 | 4 | S–M | none | 5 | **batch** |
| rm-011 | search truncation disclosure, rates.total constant, unknown-reason labels (search.rs:203, governance.rs:97/104, i18n.rs:147-155) | 3 | 3 | M | none | 3 | defer (3rd time, see §3) |
| rm-002 | test coverage for scripts/fixtures/make-adversarial-sqlite.py | 2 | 2 | S–M | none | 2 | defer, post-batch |
| rm-014 | parse-once JSONL across format probes (parser.rs probe chain; upstream wave) | 4 | 3 | M | none | 3 | **cycle 2, behind rm-013** (§3) |
| rm-015 | statusline schema catch-up: spend_limit window + workspace.repo identity (statusline.rs:215/405-502) | 3 | 3 | M | none | 4 | cycle-2 pair w/ rm-016 |
| rm-016 | resolve_project memoization in TUI hot paths (app.rs:1427-1429, explorer.rs:556-568, filters.rs:251-253) | 3 | 3 | M | none | 3 | cycle-2 pair w/ rm-015 |
| rm-017 | dead self-hosted cache steps on hosted runner (ci.yml:44-78) + dependency-review comment drift (:7) | 2 | 2 | S | CI lane to observe | 2 | defer (needs a CI run to prove) |
| rm-018 | `--range today` UTC boundary (insights.rs:65-70) | 2 | 2 | S | none | 2 | defer |
| rm-019 | discovery-cache lossy keys + mtime-only freshness (session_cache.rs:1031-1033/513) | 3 | 3 | S–M | none | 3 | defer |
| rm-020 | transcript-derived 5-hour billing blocks (ccusage 18.8k★ demand evidence) | 4 | 2 | M+ | none | 5 | cycle-3 lead (capability) |
| rm-021 | crossterm 0.28→0.29 unification (dual copy verified) | 3 | 2 | S–M | dep graph churn | 2 | defer, post-merge |
| rm-022 | ureq 3.x + rusqlite refresh | 3 | 3 | M | API migrations, MSRV | 2 | defer |
| rm-023 | upstream release-engineering wave (crt-static, git-cliff, republish) | 2 | 2 | M | release plumbing | 2 | defer |
| rm-001 | decompose 3 high-complexity npm/scripts/install.js functions | 2 | 2 | M | none | 2 | defer |

Sibling-campaign items rm-003..rm-008 are **not on this board**: they
are implemented on PR #15's branch (open, conflicting, awaiting its
own merge window) and are therefore merge-coordination work, not
selection candidates. This batch deliberately avoids their file set.

## 3. Standing dispositions

- **rm-011 deferred a third time — with an explicit clock.** The
  cycle-4-era anchor note (three sub-defects × golden-test scaffolding
  ≈ a full session alone) still holds; the new items outrank it on
  both severity (HIGH-class security + billing-truth vs
  contract-disclosure gaps) and size. It is the designated cycle-2
  fallback if the cycle-2 pair slips.
- **rm-014 is sequenced *behind* rm-013, deliberately.** Both touch
  the parser, but rm-014 refactors the very probe chain whose output
  rm-013's fixture asserts. Landing rm-013 first gives it a clean
  before/after (rewind fixture red on double-count, green on
  high-water); landing rm-014 first would muddy that evidence and
  enlarge the diff under review. Cycle 2 takes rm-014 with rm-013
  already green beneath it.
- **rm-015 + rm-016 are the cycle-2 pair.** They are synergistic
  (journal-side `workspace.repo` identity reduces the very walk
  pressure rm-016 memoizes) and both medium; together they are a
  coherent "identity without walking" theme.
- **rm-020 is the cycle-3 capability lead** (blocks analytics), with
  rm-017/rm-019/rm-021 as reliability riders by priority order.
- **PR #15's merge window is a commit-phase concern, not a
  prioritization output.** The standing recipe (fmt fix + ROADMAP
  union merge of origin/master + one push) is recorded in the
  campaign memory; the cycle-1 commit gate lands *this* batch and
  leaves PR #15 to its own lane.

## 4. Rejected alternatives

- **rm-011 as the batch (or as a rider):** fails the size test as
  lead (recorded as a full session), and as a rider it would stack
  three golden-test scaffolds on top of two defect fixes, tripling
  the review surface for no compounding benefit.
- **rm-012 + rm-013 + rm-014 (whole-parser batch):** rejected —
  rm-014's probe-chain refactor drags upstream's 1,249-line wave's
  risk into the cycle and muddies rm-013's evidence (§3). The
  two-item batch matches both prior cycles' landing size (rm-009+010,
  rm-004+005).
- **rm-015 + rm-016 this cycle:** both M-effort, CE-track; the
  roadmap's own rule (reliability outranks customer-experience)
  bars them while two reliability defects of higher priority stand.
- **rm-017 as a cheap rider:** its acceptance ("CI run on the changed
  workflow shows the lint lane green without the tar step") can only
  be evidenced in the CI lane of a later phase-of-record; adding it
  now buys a claim we cannot close inside this cycle's evidence.

## 5. Execution order (red-to-green, for the implement phase)

1. **rm-012 first.** Write the regression test red (payload with
   ESC/OSC sequences must appear sanitized in report output), then
   route the three print sites through the shared sanitizer; rerun
   the campaign's live repro (crafted journal →
   `agenttrace --statusline-report | cat -v`) — must show zero raw
   ESC bytes. TUI panel asserted unchanged (numeric-only).
2. **rm-013 second.** Build the rewind fixture first and observe the
   double-count (red), then port the high-water mark from
   `be25c4c` (parser.rs, tests in the parser module), fixture green;
   existing parser goldens must stay byte-identical in expectation.
3. **Gate:** `cargo test --workspace` (236 + the new tests), `cargo
   clippy --workspace -- -D warnings`, `cargo fmt --all --check`
   (pre-commit, per the cycle's fmt landmine), all 12
   `scripts/ci/check-*.sh`. Digest recorded by the validation phase
   into the shipping PR.

Verification anchors for the implementer live in `ROADMAP.md` (rm-012,
rm-013 signal/acceptance/evidence blocks) and in this campaign's spool
records (`88feec46-assess-findings.md`,
`88feec46-research-candidates.md`).
