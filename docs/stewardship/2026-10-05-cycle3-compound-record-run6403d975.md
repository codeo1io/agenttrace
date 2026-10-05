# Cycle-3 compound record — run 6403d975 (agenttrace)

Date: 2026-10-05 · Phase: compound (attempt 98fa060e) · Lane: run worktree
`run-6403d9753320-6403d975` @ c032f33 (run's dispatched base), 8 M files + 1 untracked
test (batch rm-436/437/438) + this compound's ROADMAP/doc edits, all uncommitted.

## Cycle outcome (pre-review evidence chain)

Batch "pi journal accounting correctness": rm-436 (lead, 88.0 — pi `type:"usage"` entry
accounting incl. cache_warm + recorded-cost passthrough) + rm-437 FIRST CUT (85.0 —
pi v2/v3 tree-journal branch disclosure, `pi_branches:N`) + rm-438 (70.0 — model_change
wire key `modelId`), implemented at base c032f33 (attempt 4454cae0) after prioritization
(5a5663aa) and stewardship (c8deeccd) chose the run worktree. All three came from the
research pass-12 triangulation of the newly-PUBLIC pi upstream (spec
`packages/coding-agent/docs/session-format.md` × npm 1.0.2 dist writer × live PoCs).

Recorded outcomes, consumed at compound and NOT re-run (compound contract):

- **targeted d6643fb5** — `cargo fmt --check` rc0; `cargo clippy -p agenttrace-core
  --all-targets -- -D warnings` rc0; `cargo test -p agenttrace-core --test
  pi_usage_tree_accounting` 9/0; `cargo test -p agenttrace-core` 253/0 (8 result lines);
  `cargo test -p agenttrace` 67/0 (4 result lines) — **329/0 across 13 result lines**;
  porcelain identical before and after (validation-only turn).
- **full 50efdbc1** — local step-for-step mirror of `.github/workflows/ci.yml` jobs
  full+deny (fleet precedent a0407d88 → d08e4a6f → 2a1feaa4), mirror fidelity verified
  first (`git diff ef8e2de9 c032f33 -- .github/workflows/ci.yml` empty); **20/20 steps
  rc0**, `ALL-RUNNABLE-STEPS-PASS`, S03 `cargo test --locked` core+tui+cli **366/0**, npm
  test / ruby -c / bash -n / check-locked-cargo / cargo-deny all rc0, S13 TUI smoke
  skipped-by-condition exactly as CI does. Logs `/tmp/at-full-50efdbc1/logs/`.
- **implement-phase live PoCs (stand as recorded)** — poc-usage 150 tokens/$0.00 →
  50,150/$0.0161; poc-modelchange MODEL=multiple, $0.0025 per-block (was all-claude
  $0.0026); poc-branch `pi_branches=2` disclosed on a WARM cache (GoMetrics round-trip
  proven). Fixtures `/tmp/at-research-6403/poc/` — verified present at compound.
- **digest lineage** — `validation:v1:c11a345d…` declared VERBATIM at targeted and full.
  Engine ground truth (derived with `hermes_conductor.validation_policy` against this
  worktree): `changed_surfaces` = the 9 delta paths, `testable_surfaces` = () — the delta
  is `crates/**` + `.md` only, never executable-classified, so the digest could not move
  between folds and the verbatim declaration was legitimate, not stale.

## Roadmap accounting

- Mint (8c5d63c3, base c032f33, porcelain 0) minted **rm-436..rm-440** past the then-live
  claim frontier (rm-435); implement (4454cae0) flipped rm-436/437/438
  candidate→implemented with per-item evidence lines; compound (98fa060e, this record)
  appends the `compound c3` header block + three per-item pre-review outcome lines.
- rm-439 / rm-440 stay **candidates** untouched (this cycle's next-cycle context).
- **Done-flips for rm-436/437/438 are reserved for the commit gate** (fleet convention;
  review/shipping happen after this phase).
- Post-compound census (this tree): 156 def rows (142 new-format `id:` lines + 14
  old-format), 98 candidate / 26 done / 32 implemented, 0 duplicate ids, managed footer
  intact. Committed-wall max at base is rm-391; this run's mint band tops at rm-440.

## Prevention rules (reusable, fleet-scoped)

- **PR-A — A dead attempt's uncommitted trail is evidence, never a base.** Prior attempt
  aace1d82 (provider-dead, no typed result) left a parser.rs+lib.rs trail covering only
  core mechanics: it did NOT compile (missing new `Metrics` fields at the session_cache
  reconstruction), PANICKED on a header-without-cwd journal (`expect` → needed
  synthesize), and counted the session header's id as a branch leaf (pi_branches
  off-by-one). Audit line-by-line, re-derive compile+tests, and only then adopt pieces.
  Everything adopted was re-pinned by this run's 9-test contract file.
- **PR-B — Wire contracts come from the upstream WRITER, not from any reader.** All three
  defects were key-shape mismatches invisible to reader-side reasoning: `modelId` vs
  `model` (dead handler), `type:"usage"` vs the `_ => {}` catch-all, `id`/`parentId`
  tree shape vs linear parse. When upstream is public, triangulate spec × shipped dist
  writer (`appendUsage(kind, provider, model, usage)`, `appendModelChange(provider,
  modelId)`) × live PoC. One repo going public (pi, 2026-10-04, versioned session-format
  spec) repriced an entire research lane.
- **PR-C — Derive validation-phase shape from the engine's own code, first.** For a pure
  Rust+docs delta: `crates/**` and `.md` are never executable-classified →
  `testable_surfaces` = (), the covered_surfaces exact-match inference is moot, and the
  digest is immobile across folds (verbatim declaration is legitimate). Declare exactly
  the paths actually changed (KTD13); never declare paths you didn't touch.
- **PR-D — Mirror reuse needs a fidelity check and a pre-created scratch tree.** Before
  reusing the sibling ci.yml mirror at a new base, diff the workflow across the base span
  (empty = run bodies byte-verbatim). Pre-create
  `mkdir -p $SC/{logs,home,tmp,ci-artifacts}` or S03 loses one test through
  TMPDIR/`std::env::temp_dir()`.
- **PR-E — /tmp is phase-scoped; verify before citing.** Recorded artifacts under /tmp
  must be existence-checked at every later phase that cites them (here: PoC fixtures and
  mirror logs were present; the 6557b823 assess-scratch sweep loss did not recur). Durable
  records must carry re-derivation instructions for anything they cite under /tmp — the
  rm-437 second-cut line does exactly that.
- **PR-F — Numeral double-mints can arrive WITHOUT a ROADMAP mint.** Sibling 71f666e8's
  uncommitted delta cites `(rm-436)`/`(rm-437)` in its CHANGELOG and code comments
  (governance.rs:513 measured-numerator, statusline.rs:1131 bounded statusline numerics)
  for completely different content, and carries no ROADMAP rows — a diff-only ROADMAP
  sweep shows nothing. Frontier sweeps must also grep sibling CHANGELOG diffs and code
  comments. Resolution stays merge-by-TITLE: the lane without ROADMAP rows renumbers its
  citations at its landing; this run's minted definitions keep the numerals.

## Residuals and next-cycle context

- **Cycle-4 lead: rm-437 SECOND CUT** — active-branch replay (turns/health/timeline
  follow the leaf reachable from the last user turn per spec §v2/v3 navigation) while
  spend keeps all branches disclosed. First-cut shapes + fixtures are pinned in
  `crates/agenttrace-core/tests/pi_usage_tree_accounting.rs`; re-derive PoC fixtures from
  the signals if `/tmp/at-research-6403/poc/` is swept.
- **rm-439** (extra positional session paths silently ignored — `--compare a b` audits
  one file, rc0) and **rm-440** (0-byte session files: doctor `failed` vs data_health
  `skipped`) are minted candidates with on-disk PoC evidence recorded in their rows.
- **rm-423 amendment at ITS landing** (16bbd3ae lane): pi upstream is PUBLIC with a
  versioned session-format spec and a live releases API (1.0.2, 2026-10-04) — that item's
  "repo PRIVATE (404)" premise is dead; amend there, do not double-mint.
- **Id frontier at compound**: landed wall max rm-402 @ ea5c41e (origin re-fetched this
  phase, unchanged); unlanded live bands rm-408..410 + rm-444 (555a174d), rm-411..416
  (6557b823), rm-417..419 (3c24960c), rm-420..423 (16bbd3ae), rm-424..428 (fb22927c),
  rm-429..435 (5bec3c93), rm-436..440 (this run), rm-441..443 (e97ae6c9 — claims stand
  per repeated re-sweeps), rm-445..447 (2d37535d @ ea5c41e) → **next free rm-448**.
- **Watch**: pi 1.0.3+ release trigger untripped; bare `agenttrace` npm name still free;
  upstream luoyuctl quiet (tip 52ab2cd, v0.9.0 latest); dependabot #279/#278 bump-only
  (clap 4.6.3, attest action).

## Commit-gate handoff (later phases — not this one)

- One commit lands the whole pre-review delta: 8 modified tracked files + the untracked
  test + this compound's ROADMAP/doc additions; KTD13 re-declares for the grown delta
  (the doc is `.md` — non-executable, partial declaration stays legal).
- This record is a NEW untracked file, so it enters the validation digest for phases
  dispatched after this turn — the commit/final gates' dispatch digests will differ from
  `validation:v1:c11a345d…` by design; consume each phase's own dispatch token.
- Done-flips rm-436/437/438 → done at the commit gate, after independent_review.
- Integration must merge-by-TITLE against the 71f666e8 lane (PR-F) and against sibling
  parser.rs/diagnostics bands (41263f58 rm-403..407, 16bbd3ae rm-420..423) — the mint
  banner's per-band map is the entry point.
