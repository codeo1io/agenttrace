# CONCEPTS

Project vocabulary captured from maintenance cycles. Seed terms from the cycle-1 learning `docs/solutions/workflow-issues/fork-upstream-drift-status-subcommand.md` (2026-09-30).

## Terms

- **Drift** — the fork's divergence from its upstream across commits, tags, and distribution channels at a point in time; surfaced as ahead/behind counts, diverged files, new upstream releases, and unported commits by `agenttrace upstream`.
- **Last sync** — the merge-base commit anchoring drift measurement: the most recent upstream change carried in the fork. At seed time (2026-09-30) that was upstream PR #284, 2026-09-11; the same day's 02993de2 integration ported upstream v0.9.0 onto master, moving it to upstream #286 (`be25c4c9`, 2026-09-30) — the parenthetical drifts with every port, which is the point of measuring. Older tracking refs mean a staler drift view; the report self-describes its age.
- **Unported commit** — an upstream commit after the last sync that is not yet applied to the fork; counted and grouped by area (parser / diagnostics / pricing / reports / core / tui / cli / ci-release / npm-install / docs-testdata / other) so the next maintenance cycle can see which surfaces it is about to touch.
- **Validation digest** — a digest binding a recorded validation outcome to the exact tree state it ran against: the content of the policy-classified executable surfaces, folded with a base commit. Two digests for one tree are both legitimate whenever their base commits or covered surfaces differ, so digest inequality alone is never evidence of staleness; a change confined to non-executable surfaces leaves the digest byte-identical by design.
- **Gate envelope** — the local result record a validation gate writes for one run: outcome, timestamps, and its own validation digest derived against a base the local gate cannot know. It is run bookkeeping and evidence of execution; the declaration authority for a digest is a derivation against the dispatch base commit, never the envelope's value.
