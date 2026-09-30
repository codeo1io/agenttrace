---
title: "Mechanize fork-vs-upstream drift checks with a status subcommand"
date: 2026-09-30
category: workflow-issues
module: "agenttrace-cli / fork maintenance"
problem_type: workflow_issue
component: development_workflow
severity: medium
applies_when:
  - "Maintaining a long-lived fork whose cadence is slower than upstream's merge cadence"
  - "Re-deriving fork-vs-upstream divergence by hand at the start of each maintenance cycle"
  - "Distributing through a channel outside git (npm registry) whose state is invisible to git alone"
  - "Needing machine-readable drift for scripts or CI rather than a human-only report"
tags: [fork-maintenance, upstream-sync, drift, offline-first, cli-tooling]
---

# Mechanize fork-vs-upstream drift checks with a status subcommand

## Context

agenttrace is maintained as a fork (codeo1io/agenttrace) tracking
luoyuctl/agenttrace (remote `upstream`, fetch-only by design), and is
additionally distributed through the npm registry as `@zack78/agenttrace`.
Every maintenance cycle used to begin by re-deriving the fork's position by
hand: ahead/behind counts, the last sync point, upstream commits not yet
ported, upstream release tags the fork does not contain, and the npm channel
state. That derivation lived in ad-hoc shell run once per cycle, so it (a)
cost time every cycle, (b) varied in shape from run to run, and (c) was not
available to fork users at all.

The cost is not hypothetical. While the 2026-09-30 maintenance cycle was in
flight, upstream merged four maintenance PRs within an hour
(#287 CodeRabbit/Codecov, #288 cargo-deny + OpenSSF Scorecard + rustls
advisory patch, #289 git-cliff CHANGELOG generation, #290 coverage/Scorecard
badges — all merged upstream 2026-09-30 02:53–03:48 UTC) while the fork's
last sync point stayed at upstream PR #284 (2026-09-11): 8 unported commits
(#285–#292), one unreleased tag (v0.9.0), and 124 diverged files. Drift that
requires a hand-run script to see gets discovered late.

## Guidance

Ship the drift report as a first-class, offline-by-default CLI subcommand
rather than a hand-run script or a network-dependent tool. Roadmap item
rm-024 added `agenttrace upstream` (crates/agenttrace-cli/src/upstream.rs,
cycle 1 batch "Drift made visible"):

- **Offline by contract.** Every number derives from local remote-tracking
  refs (`git merge-base`, `git rev-list --count`, `git diff --name-only`,
  `git tag --merged`) — the command never touches the network on its own.
- **Explicit network opt-in.** `agenttrace --fetch upstream` runs
  `git fetch upstream --quiet` first, then probes the npm registry (a `curl`
  subprocess capped at 15 s) for the distribution channel; a failed probe
  degrades the npm line to `unavailable (...)` instead of failing the report.
- **Self-describing staleness.** The report prints the age of its own
  remote-tracking refs (`refs_age_seconds`: tracking-ref mtime with a
  FETCH_HEAD fallback), so a stale offline view says "2h old" instead of
  silently passing as current.
- **Machine-readable drift.** `-f json` renders a stable, append-only schema
  (command/mode/remote/local/last_sync/ahead/behind/diverged_files/
  new_upstream_releases/unported_commits/unported_areas/refs_age_seconds/npm),
  documented in docs/guides/upstream-status.md. The text view caps the
  unported list at 20; JSON is uncapped and oldest-first.
- **Area grouping.** Files touched by unported commits are grouped into
  named areas (parser, diagnostics, pricing, reports, core, tui, cli,
  ci-release, npm-install, docs-testdata, other; the exact output labels are
  defined by `classify_area` in upstream.rs), turning
  "8 unported commits" into "which surfaces am I about to conflict with".
- **No new crate dependencies.** `git` (and optionally `curl`) run as
  subprocesses, so the CLI crate stays network-free and the lockfile does not
  move. `UPSTREAM_REMOTE` / `UPSTREAM_REF` env overrides keep wrapper tooling
  pointed at other remote/ref pairs.

## Why This Matters

- Drift that is one flag away gets looked at every cycle; drift that costs a
  hand-run script gets looked at when something breaks. Upstream's
  2026-09-30 burst shows the pace a fork can silently fall behind at.
- Offline-by-default keeps the command safe to run anywhere (CI sandboxes,
  air-gapped hosts) while the refs-age line keeps the offline view honest
  about its own freshness — better than a "fresh" view that silently needed a
  network round-trip nobody made.
- A stable JSON schema makes drift consumable by the maintenance tooling
  itself: prioritization, assess, and reconcile phases can read the same
  numbers the maintainer sees, with no parsing of human-formatted text.
- Grouping by area turns the unported list into the direct input the next
  cycle's batch selection needs (a `ci/release`-heavy delta and a
  `parser`-heavy delta imply very different risk).

## When to Apply

- Any fork whose maintenance cadence is slower than upstream's merge cadence.
- Whenever a cycle's assess or research phase re-derives the same upstream
  numbers more than once — that is the signal to mechanize the derivation.
- When distribution channels (npm, GitHub releases) can drift independently
  of git history; surface them in the same report rather than as separate
  manual checks.
- When the drift check must run in environments without network access; the
  offline default plus explicit `--fetch` opt-in is the shape that permits
  that without lying about freshness.

## Examples

Cycle-1 verification (2026-09-30 maintenance run at the committed base, raw
git re-derivation cross-checked against the subcommand):

```
$ agenttrace upstream
agenttrace upstream status — offline view (remote-tracking refs …)
  upstream:         upstream/master (https://github.com/luoyuctl/agenttrace)
  last sync:        6848aa10df9d (2026-09-11) — fix(parser): skip leading
                    non-session lines in Oh My Pi JSONL (#284)
  ahead:            37 commit(s)
  behind:           8 commit(s)
  diverged files:   124
  new releases:     1 (v0.9.0)
  unported commits: 8   (#285–#292)
  unported areas:   ci/release 7, core 5, docs/testdata 4, other 4, tui 3,
                    diagnostics 1, parser 1, reports 1
  npm:              unknown (offline; agenttrace --fetch upstream) | …

$ agenttrace --fetch upstream
  … post-fetch view; npm line becomes '@zack78/agenttrace@0.9.0 (registry)'

$ agenttrace -f json upstream
  … full append-only schema for scripting
```

Exit behavior: 0 for any rendered report (drift itself is informational);
1 with a setup hint for operational failure — not inside a git repository,
no `upstream` remote configured, no remote-tracking ref yet, or no common
history.

Coverage: 6 in-module unit tests (area classification, log parsing, render
shapes) plus 9 hermetic integration tests (crates/agenttrace-cli/tests/upstream.rs)
over a fixture pair — a local bare "upstream" and a fork clone whose remote
is named `upstream` — so the `git fetch` path is exercised against the local
bare remote with zero network. Recorded outcomes for the cycle: targeted
suite 5 passed (python fixture tests), full workspace suite 251 passed / 0
failed.

## Related

- In-tree: docs/guides/upstream-status.md (schema, exit codes, env
  overrides); crates/agenttrace-cli/src/upstream.rs (implementation);
  crates/agenttrace-cli/tests/upstream.rs (integration coverage);
  ROADMAP.md rm-024 (origin item — numbered rm-021 campaign-locally until
  the 2026-09-30 integration renumbered it past 02993de2's landed
  rm-017..rm-019, per the roadmap's collision note).
- Upstream PRs merged after the fork's last sync, merge-state verified via
  gh on 2026-09-30: #287 (CodeRabbit review config + Codecov), #288
  (cargo-deny + OpenSSF Scorecard; rustls advisory patch), #289 (git-cliff
  CHANGELOG after each release), #290 (coverage and Scorecard badges);
  last-sync marker upstream PR #284 (merged 2026-09-11).
- A bounded GitHub issue search over both the fork and the upstream repo
  found no related issues (the fork has issues disabled; the upstream search
  returned one unrelated growth-lane item), so this doc cites
  upstream-PR/advisory evidence rather than issue links.
- The report shape mirrors the upstream-delta shell prototype developed in a
  sibling maintenance lane under rm-012; that script landed on master with
  the 2026-09-30 02993de2 integration (after this doc was written), so both
  forms are in-tree — `agenttrace upstream` is the durable, shipped form of
  the same view.
