# CONCEPTS

Project vocabulary captured from maintenance cycles. Seed terms from the cycle-1 learning `docs/solutions/workflow-issues/fork-upstream-drift-status-subcommand.md` (2026-09-30).

## Terms

- **Drift** — the fork's divergence from its upstream across commits, tags, and distribution channels at a point in time; surfaced as ahead/behind counts, diverged files, new upstream releases, and unported commits by `agenttrace upstream`.
- **Last sync** — the merge-base commit anchoring drift measurement: the most recent upstream change carried in the fork (currently upstream PR #284, 2026-09-11). Older tracking refs mean a staler drift view; the report self-describes its age.
- **Unported commit** — an upstream commit after the last sync that is not yet applied to the fork; counted and grouped by area (parser / diagnostics / pricing / reports / core / tui / cli / ci-release / npm-install / docs-testdata / other) so the next maintenance cycle can see which surfaces it is about to touch.
