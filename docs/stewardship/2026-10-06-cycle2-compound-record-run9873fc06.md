# Cycle 2 compound record — run 9873fc06 (repository-maintenance 059e84c3)

Status: pre-review compound (learnings recorded after implementation + full validation,
before review/shipping). Written 2026-10-06, attempt ae5a3a95. Base: 40f0ae8 →
stewardship-mandated merge-forward to 8991144; tree porcelain at authoring = batch
files + ROADMAP delta + governance-guide schema pin, uncommitted (commit gate owns landing).

## What the cycle did

- **rm-625 (security 88.0; minted campaign-locally as rm-545, rebound at review-fix 2026-10-06) — terminal-safe output dispatch: one control-byte choke
  point (`dispatch_sanitize` at all 12 report emission sites incl. `-o` writes) over a
  new document-level sanitizer (`sanitize_output_document`: LF/CR/TAB preserved, all
  other control bytes → U+FFFD, idempotent so it composes with the rm-383/rm-540 cell
  sanitizers instead of corrupting them); JSON lanes excluded escaped-by-design;
  `output_safety_matrix.rs` drives the assessment's poisoned corpus (OSC-52/BEL/CSI in
  user text, tool name, model id) through every report × non-JSON format and asserts
  zero 0x1b/0x07 bytes on stdout AND `-o` files. Red-first: overview markdown was
  10 ESC / 6 BEL, now 0/0 everywhere.
- **rm-548 (correctness 64.0)** — opencode fork-copied history excluded from usage
  aggregation, both lanes (JSON storage `parentID`; sqlite `session.parent_id`,
  feature-detected), counts disclosed through the rm-436/437 disclosures channel;
  explicit-path loads keep forks rendering; SQLITE_SNAPSHOT_SCHEMA_VERSION 7→8;
  survey doc `docs/guides/opencode-fork-marker.md` pins the marker from opencode dev
  source (info.ts:19 / sql.ts `session_parent_idx`) + ccusage #1782 provenance.
- Full suite after both: **471/0 across 22 targets**, all 14 CI gate scripts green,
  `cargo deny --all-features check` green (see full_tests phase evidence,
  /tmp/at-full-9873/).

## Prevention rules (PR) — new this cycle

- **PR-A (renderer onboarding)**: any NEW output format or report renderer must
  (a) emit through the same `dispatch_sanitize` boundary and (b) add its cell to
  `crates/agenttrace-cli/tests/output_safety_matrix.rs`. The rm-383 family failed
  because sanitizers were composed per-renderer; the choke point only holds if new
  lanes are born inside it. (Owner row: rm-625, the rebound campaign-local rm-545.)
- **PR-B (schema-pin coupling)**: any bump to `SQLITE_SNAPSHOT_SCHEMA_VERSION` (or
  the session-cache schema constant) must move `docs/guides/governance-reports.md`'s
  pinned sentence in the SAME change — `scripts/ci/check-docs-commands.sh` greps the
  live constant against the guide and fails otherwise. Caught by targeted_tests this
  cycle; cheap to prevent. (Second occurrence of the class 99d1c79c hit on the
  session-cache schema.)
- **PR-C (survey-before-exclude)**: cross-tool semantics (fork markers, dedup keys)
  must be pinned from upstream SOURCE in a `docs/guides/*.md` fixture doc before any
  exclusion code lands (rm-456 precedent, reused here as
  `docs/guides/opencode-fork-marker.md`). The exclusion count must always ride a
  disclosure channel — never a silent drop.
- **PR-D (explicit-path immunity)**: aggregation-level exclusions (duplicates, forks,
  sidechains) must never block explicit-path loads — a user pointing at a file gets
  that file rendered. Encoded as a standing test contract
  (`opencode_explicit_dir_still_renders_forked_copy`).

## Coordination ledger (for integration)

- **Id collision**: origin/master (via b1ff12f8 landing) minted its own `rm-545`
  (upstream-sync #305 subagent attribution, 74.0) after our campaign banner reserved
  the id. RESOLVED AT SOURCE (independent-review fix, 2026-10-06): our security row
  is renumbered `rm-545 → rm-625` NOW — def row, code/test/doc comments, this
  record — with the rebound on its own id line, minted from a fresh live fleet
  census (highest in-flight agenttrace claim rm-624, origin/master landed ceiling
  rm-600). Theirs keeps the id; renumber-by-title at integration is now a no-op
  for the code (fleet precedence: mint-above-the-high-water + renumber-by-title).
- **rm-239 fold**: rm-625's choke point delivers rm-239's shared-helper acceptance —
  fold rm-239 into rm-625 by title at integration; rider sites (compare session names,
  waste tool names, governance plain values) are covered by the boundary.
- **rm-543/rm-544** remain reserved for run 4ffc4fbb's recorded band (pi compaction/
  branch_summary parser arms — active sibling lane, intentionally not touched).
- **rm-547 saturation**: the #316 cache-clamp lane stays triple-claimed fleet-wide
  (rm-529/542/547) and unlanded on fork origin (`subtract_cached_input` grep = 0) —
  do not add a fourth claim; reconcile by landing order.

## Next-cycle context

- **OTel-completion batch** (recommended single cycle): rm-493's unowned CLI half
  (`-f otel` value_parser + dispatch) + rm-546 (semconv-genai dialect + reasoning
  tokens) + the rm-541/542 transport fixes from run 99d1c79c — one file family
  (otel.rs + main.rs), no sibling collisions.
- **C8 (Cursor adapter, ccusage #1771)** is now UNBLOCKED: rm-548's survey + discovery
  pattern is the template; seed from `cursor-import.md` notes + ccusage's adapter.
- **C7 (--id filter, ccusage #1777)** still pooled; cheap CLI additive.
- **rm-549** stays gated: no daily/weekly/monthly bucketing lanes exist in-fork yet
  (verified at 8991144); revisit after the rm-042 rider lands.
- **TUI real-smoke** is CI-excluded (repo-vars opt-in) — if TUI coverage beyond the
  unit suite is wanted, run `scripts/ci/check-rust-tui-real-smoke.sh` with a seeded
  pty in final_validation.
- Persisted sandboxes for the next assessor: /tmp/at-9873-assess/, /tmp/at-9873-research/,
  /tmp/at-9873-poc/ (poisoned corpus + fuzz corpus), /tmp/at-impl-9873/ (batch diff
  + roadmap delta), /tmp/at-full-9873/ (full-suite logs).

## Fleet notes (durable)

- Merge-forward with preserved dirty ROADMAP: extract rows → `git checkout --` →
  `git merge --ff-only origin/master` → re-append before the footer. The shared
  canonical repo's stash namespace belongs to other runs — never stash.
- Full-suite derivation when `validation.full_command` is empty: `.github/workflows/
  ci.yml` (lint + full + dependency-policy jobs) IS the authoritative suite; CI-exact
  env for real-cli-smoke is `AGENTTRACE_REAL_CLI_DIR=testdata QUERY=internal/ws
  FILE_LIMIT=20` (the script ignores AGENTTRACE_SEED_HOME — that is a different
  step's variable; an empty REAL_CLI_DIR silently defaults the source to
  `$HOME/.pi/agent/sessions`).
- Local cargo-deny 0.20.2 flag order: `cargo deny --all-features check`.

## Independent-review fix addendum (2026-10-06, attempt 7c705e38)

The compound phase's ROADMAP delta (banner outcome line + status flips) was lost
from the working tree before it could land; the review-fix turn re-applied it with
corrections, plus every actionable review finding:

- ROADMAP: rm-625/rm-548 flipped `candidate → implemented` with dated landed notes,
  the cycle-2 outcome banner re-applied with the id-collision resolution, the
  rm-625 acceptance corrected (the `tsv` matrix column was an over-spec — `-f tsv`
  is a clap rejection on --sessions, pinned as such; upstream is a site pin), and
  the rm-548 landed note records the review-fix deltas (doctor filter + disclosure,
  legacy sqlite wrapper deletion, fork-probe memoization).
- Code: doctor inventory fork filter + `opencode_fork_excluded_sessions`
  disclosure (auto-discovery only); `load_sqlite_backed_sessions` deleted (the
  count-dropping legacy wrapper) with `load_sqlite_backed_sessions_since(None)` as
  the single entry point; fork column read leniently (non-TEXT degrades to
  not-a-fork); fork probe memoized on the cached dir listing (fingerprint-keyed,
  rewrite always re-probes); matrix widened (+audit/recommend markdown/md/html,
  +compare, +statusline-report) with tsv-rejection, upstream-site, OSC-52-ST and
  tab-honesty pins.
