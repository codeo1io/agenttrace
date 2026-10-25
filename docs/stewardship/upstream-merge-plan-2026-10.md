# Upstream merge-hazard plan — luoyuctl/agenttrace → codeo1io/agenttrace

- Version: 1 (written 2026-10-23, run a356ccb78fc1 cycle 2, batch CU-2 —
  rm-513 arms 1 and 3, rider rm-2363). This is a merge-time plan: re-anchor
  every version number below against the live tips before executing, then bump
  this document to version 2 recording what actually happened at merge.
- Upstream snapshot (verified 2026-10-23, run a356ccb78fc1 research phase):
  `luoyuctl/agenttrace` master `15ed07f2` (2026-10-06, frozen since), release
  `v0.10.1` (2026-10-05); open PRs `#320` (Cargo.lock +7/−6, dependabot lane)
  and `#321` (actions bumps, incl. `actions/checkout` 7.0.0).
- Fork snapshot (this worktree at plan time): `codeo1io/agenttrace` `87f18ebc`,
  session-cache rung 73 (`crates/agenttrace-core/src/session_cache.rs:11`),
  pricing snapshot date 2026-10-17 (`crates/agenttrace-core/src/pricing.rs:21`),
  Cargo.lock rustls-webpki 0.103.16 / webpki-roots 1.0.10.

## 1. Pricing architecture (rm-513 arm 1 — the decision, EXPLICIT)

DECISION: keep the fork's bundled-snapshot architecture; REJECT any upstream
runtime-fetch merged into the pricing seam.

- One architecture, not a hybrid (the row's "the diff showing one
  architecture"): model pricing resolves offline from the bundled snapshot
  (`crates/agenttrace-core/src/pricing_snapshot.json`, vendored and reviewed),
  with the fork's two operator-controlled refresh layers on top —
  `--update-pricing` (downloaded cache, preferred when one is served) and
  `AGENTTRACE_PRICING_FILE` (operator override row source). This IS the
  row's "layer runtime refresh over the bundle" option; the upstream
  fetch-at-report-time model is not adopted.
- Offline-first rationale (recorded here as the row demands): reports stay
  deterministic and hermetic — the svg-card goldens, the conformance pack
  (`testdata/conformance/manifest.json`) and the contract suites all pin exact
  costs against the bundle; no network call sits on the report hot path; and
  snapshot freshness is governed by the drift gate
  (`scripts/pricing/drift-check.sh`, weekly cron lane) instead of silently
  drifting at query time.
- Merge mechanics: take upstream pricing hunks only as DATA (snapshot rows,
  per-model facts), never as code paths — regenerate the fork snapshot via
  `scripts/pricing/update-snapshot.sh` plus the sanctioned golden-refresh
  channel (`UPDATE_CARD_GOLDEN=1`), and let `drift-check.sh --selftest` (9
  offline arms) arbitrate before landing.
- Fleet sequencing guard: the pricing seam stays frozen while the
  pricing-refresh batch (run 6f8eb19a lineage) lands, and any leaf-crate split
  (R2 candidate) precedes the merge — never rebase the pricing seam mid-flight.

## 2. Session-cache schema (rm-513 arm 2)

- Fork rung at plan time: 73. At merge, renumber ABOVE max(fork, upstream) —
  never below 26 (the row's floor anchored on the historical fork-era rungs).
- Acceptance to carry at merge: a forced-reparse test on fixture caches pinned
  at the historical rungs v22 and v7 — both must be treated as stale and force
  a full reparse of raw sessions (no partial reuse across the schema boundary).
- The merge CHANGELOG must disclose the #312 accounting deltas (CC/Codex
  lower, WorkBuddy higher) with WorkBuddy reconciled to upstream #311
  (sum-every-record) and #310 closed-fixed.

## 3. Path remaps (rm-513 arm 3)

Fork-side path anchors that break if upstream renames files. Enumerate the
renames at merge time with
`git diff --name-status --find-renames 15ed07f2..<fork tip>`, then re-anchor
every hit below.

Anchors verified at plan time:

- `install.sh:19` `REPO="luoyuctl/agenttrace"` (usage line :5, release/source
  URLs :73-:78, clone hints :89/:233). Open decision at merge: keep the
  luoyuctl release channel (both repos currently publish v0.10.1 with
  identical asset sets) or repoint to the codeo1io channel once it owns
  release automation. Record the choice in this document's version 2.
- `npm/package.json` — `homepage`, `repository.url`, and the issues URL all
  point at `luoyuctl/agenttrace` (moves in lockstep with the install.sh
  decision above).
- `.github/workflows/*` — actions are SHA-pinned (checkout@`3d3c42e5…`,
  dtolnay/rust-toolchain@`29eef336…`, Swatinem/rust-cache@`6323deb1…`,
  dependency-review-action@`a1d282b3…`, git-cliff-action@`a9a95522…`,
  create-pull-request@`5f6978fa…`); see §4 for the #321 conflict map.
- `scripts/pricing/drift-check.sh:50-51` and
  `scripts/pricing/update-snapshot.sh` anchor
  `crates/agenttrace-core/src/{pricing_snapshot.json,pricing.rs}` and the
  svg-card fixture directories.
- Positional fixture-path contracts under `crates/*/tests/**` and
  `testdata/**` (CLI arms pass fixture paths as positional args).
- ROADMAP rider line-pins go stale on ANY upstream diff touching ROADMAP.md —
  re-derive riders by `rm-XXXX` title, never by line number, during the merge
  (fleet convention).

## 4. PR #321 workflow-conflict map

- Upstream #321 bumps actions to floating majors (checkout 7.0.0 et al.).
- Fork policy: SHA-pinned actions for provenance; refresh deliberately via
  bump-the-SHA commits, never by taking upstream's floating bumps.
- Resolution at merge: KEEP ours; discharge #321's overlap by recording the
  deliberate SHA-refresh decision (or a fresh SHA bump) in the merge
  CHANGELOG entry. Zero `.github/workflows` hunks are accepted from upstream
  without an explicit re-pin.

## 5. PR #320 lockfile-drift walk (dependabot fog)

- Ours: Cargo.lock carries the fork's dependency lanes (rustls-webpki
  0.103.16 / webpki-roots 1.0.10 at plan time), one lane ahead of upstream's
  graph.
- Theirs: #320 is a dependabot sweep (+7/−6) against upstream's older graph.
- Walk order at merge: land the merge on OUR tree (lockfile included), then
  refresh the fork's lanes through the sanctioned lock step, and treat
  upstream dependabot PRs as fogged — do not cherry-pick #320.

## 6. Merge acceptance (rm-513 arm 4)

- Merged head: full suite green (the 95-suite / 1487-test class at plan
  time), every wall/CI reference to renamed paths updated, and this document
  bumped to version 2 with the actual renumber and decision records.
