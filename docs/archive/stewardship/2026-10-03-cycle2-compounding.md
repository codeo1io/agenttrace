# Compounding Record — repository-maintenance `4ae787ecdb724aabb8c17a0240f8efcf` cycle 2

Run `0d487394649847ac88e657bb30b4c154` · phase compound · attempt
`0eb699eb44fe4dbd92ae41b8a2a25982` · 2026-10-03 · base `5ef66c0`
(porcelain: the implement batch's 7 M + 1 new file, untouched by this
phase — this phase added ZERO in-tree changes; all compound artifacts are
spool-side for the commit gate).

> Scope note: this record compounds PRE-REVIEW cycle evidence only
> (assessment, research, roadmap, prioritization, stewardship,
> implementation, targeted/full test outcomes). Review and shipping
> outcomes are NOT included by design — the next cycle's assessment
> carries them forward. No tests were executed in this phase; every
> validation claim below is consumed from the recorded outcomes of
> implement `b320bd3f`, targeted `2f587f05`, and full `04ec205d`.
>
> AMENDED 2026-10-03 by review-fix attempt `e6f22fad` after independent
> review `26382697` returned NEEDS_CHANGES (code batch approved; two
> factual errors in these artifacts): (1) the rm-243 mechanism text had
> the rank direction INVERTED and anchors wrong — actual:
> `sort_delivery_records()` governance.rs:852-860 sorts ASCENDING by
> `delivery_level_rank()` :862-869 (strong=0…catch-all=4; `none` ties
> unrecognized in the weakest band), not "strong=4…unrecognized=−1,
> descending" at :904-919; (2) the rm-245 test count was 72 (the
> pre-existing discovery_contract suite) — attribution_dimensions.rs
> has 8 tests; (3) the "post-validation drift" framing is withdrawn:
> the +1/−1 guide edit was full_tests' own disclosed mid-suite fix, so
> 798/42 IS the validated state. Same corrections applied to
> ROADMAP.compounded.md and both regenerated diffs.
>
> AMENDED 2026-10-03 by review attempt `89691b9b` after independent
> review `a8322627` returned NEEDS_CHANGES (code batch approved in full
> on fresh evidence; one residual factual error here plus advisory
> counts — worktree still needs ZERO changes): (1) lesson L4's
> "72-test suite" corrected — the attribution invariants are pinned by
> `tests/attribution_dimensions.rs` (8 tests;
> `task_type_tokens_match_by_model_baseline` at :241); 72 is the
> unrelated pre-existing `tests/discovery_contract.rs` suite (both
> counted and run fresh by the review: 8 passed / 72 present). The
> e6f22fad sweep assertion "no stale (72 tests)/72-test strings remain"
> had been scoped to ROADMAP.compounded.md only — lesson: scope
> artifact-sweep assertions to the exact files swept, or sweep every
> deliverable in the set and list each. (2) The §5 size claims are
> re-measured from the artifacts on disk: mint wave +41/-0 over in-tree
> (the mint record's own "+58 added lines" is wrong for the patch on
> disk — 41 insertions, 99-line file); compound delta is 12 changed
> lines over the mint wave (8 added / 4 removed; single-line bullets);
> `roadmap-full.diff` = 103-line file, 45 added / 0 removed. (3) §4's
> rm-003 /tmp PoC pointers are dead (swept before review) — re-derive
> the PoC from the rm-003 entry's signals before claiming; durable
> evidence lives in repo/spool, never /tmp. Deliberately NOT changed:
> ROADMAP.compounded.md and both diffs stay byte-stable (sha256
> 8dd66e7a…/4f5a00b4…/7780eeba…); a8322627's low finding on the
> rm-244 bullet's loose anchor ("main.rs:252-258 area" — the guard
> actually fires via `validate_range_applicability()` invoked at
> main.rs:203, fn :1175-1182; :252-258 is the protected fallthrough) is
> recorded here for the commit gate rather than churning the
> twice-certified diff payload for a hedged anchor.

Suggested landing (commit gate): `docs/stewardship/2026-10-03-cycle2-compounding.md`
(per the repo convention established by
`docs/stewardship/2026-10-01-cycle1-compounding.md`, sibling campaign).

## 1. Cycle outcome — roadmap status flips (spool artifact
`ROADMAP.compounded.md` + `roadmap-compound.diff` + `roadmap-full.diff`)

All four batch items flipped `status: candidate → implemented` with a
`cycle-2 (2026-10-03, run 0d487394…, pre-review): EXECUTED` bullet
appended (mechanism + line anchors + validation counts), matching the
in-repo convention. Census after compounding: 96 items = 68 candidate +
14 implemented + 14 done (was 72/10/14 in the prioritize census — exactly
the batch applied). `done` flips remain the commit gate's.

| Item | Outcome |
|---|---|
| `rm-242` bound the delivery-evidence git subprocess | implemented — `GIT_PROBE_TIMEOUT = Duration::from_secs(10)` + `wait_child_bounded()` (try_wait + kill on deadline, helper-thread stdout drain so a full pipe can't fake a timeout); soft-degrade via the existing `Option` path + methodology disclosure "bounded at 10 seconds per repository root…"; sleeping-git stand-in: report completes real 0m10.025s rc0 degraded. Umbrella remainder (upstream.rs:185/:201, rm-050) still open |
| `rm-243` rank delivery-evidence levels explicitly | implemented — `sort_delivery_records()` governance.rs:852-860 sorts ASCENDING by `delivery_level_rank()` :862-869 (strong=0/medium=1/weak=2/non_code=3/catch-all=4 — smaller rank is stronger evidence; `none` and unrecognized labels share the weakest band, so an unknown label never outranks a known one; mutual order by session-name tie-break); shared by text+json; golden tests at every level (governance inline suite 6→13); ordering contract now documented |
| `rm-244` reject `--range` without a report action loudly | implemented — guard at the no-action fallthrough, rc1 + "Error: --range requires a session report action (for example --overview, --sessions, --diagnostics, --waste, --search QUERY, --compare, or --audit); the interactive and utility views ignore it"; `--demo --range all --overview` composes rc0; pinned in launch_guards.rs |
| `rm-245` attribution dimensions (STRATEGIC LEAD) | implemented — `by_provider` via `pricing::provider_for(model_used)` (same catalog row that prices the model — vendor and price claims can never disagree; unknown → explicit bucket, never dropped) + `by_task_type` via `infer_task_type()` (deterministic 3-way cascade over parsed aggregates only: debugging = ≥1 failed call AND a ≥25% session fail rate, or a tool-failures anomaly; coding = execution authority (external_publish/git_write/write_files/package_install/test_or_build); planning = rest) + top-cost-drivers; all five render surfaces (json/md/text/human/html); 8-test suite incl. reconciliation to baselines; docs +40 lines |

Validation envelope (recorded, not rerun): fmt rc0 · clippy `-D warnings`
rc0 · cargo test core 119+8+7+72+2 / cli 22+10+4+9 / tui 46 = **299 passed,
0 failed** (full suite mirrored verbatim from `.github/workflows/ci.yml`
job `full`) · release build rc0 · gates output-contract /
deterministic-output / report-semantics / release-surfaces /
install-runtime all rc0 under CI-faithful isolation.

**PROVENANCE OF THE +1/−1 GUIDE EDIT (resolved at review):** `git
diff --stat` reads 798/42 — +1/−1 versus the 797/41 that implement and
targeted certified — because the full_tests phase itself applied one
mid-suite docs fix and disclosed it in its own record ("ONE FIX WAS
REQUIRED AND APPLIED … guide line 71 schema 6 → schema 7"; guide mtime
08:21:36 falls inside full_tests' window — release build 08:27:16,
record 08:31:31): the recorded pre-existing docs-schema drift line
`the SQLite snapshot is schema 6` → `schema 7`, and the docs gate ran
rc0 AFTER the fix (step 11). So **798/42 IS the fully validated
state** — there is no unaccounted mutation and no re-run owed; this
record's earlier "post-validation drift, commit gate must act"
framing was wrong and is withdrawn (independently re-corroborated by
review 26382697: docs gate rc0 with the batch binary). This phase
changed no tracked file; all compound artifacts are spool-side.

NOT flipped (deliberately): `rm-239` (sanitization), `rm-240`
(backtracking decode), `rm-241` (hermetic decode test) — their CONTENT is
sibling campaign 6a10ae64's cycle-2 batch (their numbering rm-232/230/231;
stewardship 6810e9f2 succeeded, implement not started). This run mints
the ids, their lane flips their statuses. **Numbering warning for the
commit gate:** this run's uncommitted wave rm-239..rm-245 numerically
collides with 6a10ae64's rm-230..rm-240 (committed ceiling in
fork/master = rm-238) and with b099a3a5's telegraphed rm-241..rm-243
(different content). Content is unique fleet-wide; renumber at
integration per the record-not-renumber convention; next free id after
all recorded waves ≈ rm-246.

## 2. Reusable lessons (mechanism-anchored)

- **L1 — rank maps, not string sorts.** `rm-243`: sorting semantic enums
  by display string is a latent correctness bug (lexicographic rendered
  `medium, non_code, none, strong, weak` — zero-evidence above medium,
  weak above strong). Pattern landed: explicit rank fn + unrecognized
  labels collapsed into the weakest band alongside `none` (never
  outranking a known level; mutual order resolved by the session-name
  tie-break) + golden test with one record at every
  level. Apply wherever a total order over an enum is rendered.
- **L2 — every spawn carries the bound class.** `rm-242`: the repo's cap
  class (`download_pricing(Duration::from_secs(30))` → now
  `GIT_PROBE_TIMEOUT(10s)` via `wait_child_bounded`) = bound + soft-degrade
  on the existing Option path + disclosure string + stand-in test. The
  helper-thread stdout drain is the non-obvious half: without it a full
  pipe turns a FAST child into a spurious timeout. Remaining unbounded
  lanes: `upstream.rs:185/:201` (rm-206 umbrella), pricing body read
  (rm-050).
- **L3 — a flag with no consumer must reject, not fall through.**
  `rm-244`: the TUI fallthrough silently dropped `--range`; the fix
  reuses the established applicability style (`main.rs:201`/`:263`).
  Composability debt recorded: b099a3a5's telegraphed `--anomaly`/`-m`
  rider must share the helper at fold — do not fork a second mechanism.
- **L4 — every new aggregate reconciles to its sources.** `rm-245`:
  `tests/attribution_dimensions.rs` (8 tests) pins
  `by_provider`/`by_task_type` session sums ==
  `total_sessions` and task-type token sums == the by-model rollup
  baseline (`task_type_tokens_match_by_model_baseline`); unresolvable
  provider models bucket explicitly as `unknown`, never dropped. Vendor
  identity comes from the same catalog row that prices the model, so
  vendor claims and price claims cannot disagree. This is the pattern for
  every future dimension (and partially de-risks rm-011's
  report-contract decision).
- **L5 — the authoritative full suite is the repo's own CI, not an
  invented list.** When the work order's `full_command` is empty, mirror
  `.github/workflows/ci.yml` job `full` step-for-step (19 steps: fmt,
  clippy --locked, cargo test --locked over 3 crates, release build,
  entrypoints, output-contract/deterministic/report-semantics gates,
  release-surfaces, install-runtime) under CI-faithful isolation
  (scratch HOME/TMPDIR, pinned AGENTTRACE_BIN, AGENTTRACE_CI_OUT). That
  recipe produced 299/299 and is the recorded envelope for this tree.
- **L6 — the validation digest is blind to Rust.** Conductor's
  `classify_surface` digests only `.py`/`.pyi` + executable prefixes, so
  a pure-`.rs`/`.md` delta yields `changed_testable_surfaces=[]` with
  `underivable=false` — digest-invisible. Targeted phases must re-derive
  the tree digest (this batch:
  `validation:v1:ed7fc27031119b7016a55b56a385c882d068a`) and run the
  cargo suites regardless of what the engine's digest implies.
- **L7 — provider-death recovery is cheap when artifacts landed.**
  This cycle lost two attempts to provider deaths: prioritize `bca23159`
  (nothing durable — redo from scratch) and stewardship `af1f80d0`
  (companion + JSON landed — attempt d4c34acc ADOPTED the companion and
  re-verified every anchor instead of redoing). Decision rule: check
  `delegate/<attempt>.json` + `events/<attempt>.jsonl` + scratch dir;
  adopt-and-reverify when a typed artifact exists, redo when not.

## 3. Prevention rule (spool artifact
`test-flake-prevention-rule7.diff`)

**Rule 7 — budget or isolate host-state-reading commands in gates**
(2026-10-03, cycle-2, run 0d487394). `--doctor -f json` (and any lane
walking the session store) stalls >120 s when HOME points at the grown
real store (~/.claude ≈253 MB + sibling roots) but completes rc0 <30 s
under an isolated HOME. Re-proved to be a host condition, not a
regression: this cycle stashed the batch, rebuilt BASE `5ef66c0`, and the
base binary also times out (45 s, rc124 under `timeout`) — so a gate red
here indicts the environment, never the batch. Same family as Rule 1's
TMPDIR shadow; kin to Rule 6's host-state pinning. Recipe: every gate
invoking `--doctor` (`check-docs-commands.sh` step 2,
`check-rust-release-local.sh`) runs under scratch `HOME` + `TMPDIR`,
pinned `AGENTTRACE_BIN`, redirected `AGENTTRACE_CI_OUT`, and gates that
need the real CLI/TUI fixtures set `AGENTTRACE_REAL_CLI_DIR` /
`AGENTTRACE_TUI_REAL_DIR` (an isolated HOME hides the default paths —
that bit this cycle's assess sweep). Cross-campaign rule-number
collisions renumber at fold (as Rule 6's note records).

## 4. Next-cycle candidates and context (concrete, evidence-backed)

1. **Lead — `rm-003` in-process zstd decode of Codex cold rollouts.**
   Research HIGH: upstream codex#25089 merged 2026-06-01 ("Compress cold
   local rollouts": writers plain .jsonl, cold = .jsonl.zst) — every cold
   rollout since is a silent coverage hole; PoCs WERE at
   `/tmp/at-assess-0c01b29f/zstd-poc/` + `z1-out.txt`/`z2-out.txt`
   (swept from /tmp before review — re-derive from the rm-003 entry's
   signals before claiming; durable evidence lives in repo/spool).
   Needs dependency-review headroom (ruzstd vs deny.toml) — pair with a
   cargo-deny gate pass (upstream added cargo-deny/Scorecard workflows,
   #287-#295, all CI/docs: rebase stays cheap).
2. **Alternate lead — `rm-042` 5h billing blocks** (ccusage
   18,843★/480,510 dl-mo anchors the window model; pairs rm-201/rm-087).
3. **Riders:** `rm-206` umbrella completion — `upstream.rs:185/:201` now
   have `wait_child_bounded` as the exact template; `rm-050` bounded
   pricing body read (same template); `rm-211` hygiene bundle.
4. **Pricing family as one coherent batch:** rm-196/175/176
   (identity/provenance/freshness; 24h etag drift recorded).
5. **Decisions pending:** `rm-011` report-contract truthfulness (needs
   the consumer-contract decision; rm-245's reconciliation pattern is the
   template once decided); `rm-053` consume the AgentMeasure #27 pack.
6. **Watch:** upstream #294 (generated_at ignore) — b099a3a5's telegraphed
   territory, coordinate before claiming; #103/#236/#237 still open
   (#103's attribution wedge now partially served by rm-245);
   dependency freshness deltas recorded this cycle (crossterm 0.29.0 vs
   ours 0.28.1 · clap 4.6.7 vs 4.6.2 · serde_json 1.0.151 vs 1.0.150 ·
   ratatui 0.30.2 current).
7. **Fleet:** sibling 6a10ae64 holds rm-239/240/241 content (their
   rm-232/230/231) — next assess must consume their landed state first,
   not re-derive it. This batch's review outcome lands after compound by
   design; next assess carries it.

## 5. Evidence index

- Batch implementation (uncommitted, this worktree): `git status` 7 M +
  1 ?? (governance.rs, lib.rs, pricing.rs, reports.rs, main.rs,
  launch_guards.rs, governance-reports.md + tests/attribution_dimensions.rs),
  +798/−42 = the full_tests-validated state (the +1/−1 guide schema fix
  was applied by full_tests mid-suite and disclosed in its record — see
  §1's provenance note) — mechanisms re-verified read-only this phase:
  `sort_delivery_records` governance.rs:852-860, `delivery_level_rank`
  :862-869, `GIT_PROBE_TIMEOUT`/`wait_child_bounded`,
  `by_provider`/`by_task_type`/`infer_task_type`/`pricing::provider_for`,
  `task_type_tokens_match_by_model_baseline`.
- Implement `b320bd3f` (PoCs: sleeping-git real 0m10.025s rc0; guard rc1
  ×4 + compose rc0 69 lines; overview by_provider/by_task_type/
  top_cost_drivers across md/text/html; docs gate only pre-existing
  schema red; base-stash re-proof of the `--doctor` host stall 45s
  rc124).
- Targeted `2f587f05`: porcelain/diff-stat unchanged certification;
  digest re-derivation `validation:v1:ed7fc270…`; fmt/clippy rc0; core
  119+8+7+72+2, cli 22+10+4+9; guard stderr captured at
  `/tmp/at-tgt-2f587f05/guard.err`.
- Full `04ec205d`: ci.yml job `full` mirrored (19 steps); 299/0; release
  build 2m13s (12,681,352 B); 5 gates rc0; logs under
  `/tmp/at-full-04ec205d/logs/`.
- Roadmap wave: `ba284996…-scratch/ROADMAP.updated.md` (+41/−0 over
  in-tree, measured; mint rm-239..rm-245 — the mint record's "+58
  added lines" is wrong for the patch on disk, which has 41 insertions
  in a 99-line file) — compounded by this phase into
  `ROADMAP.compounded.md` (12 changed lines over the mint wave:
  4 status flips + 4 single-line EXECUTED bullets;
  `roadmap-compound.diff` = 41-line file);
  `roadmap-full.diff` (in-tree → compounded: 45 added / 0 removed,
  103-line file) is the commit gate's single apply artifact.

## COMMIT-GATE NOTE (2026-10-04, commit attempt a89c2680)

Final_validation 876f437a flagged two integrity items for this gate:

1. **Record bytes mutated after approval.** The approving review 89691b9b
   recorded this record's post-fix sha256 as `08ff30eb8492ba596082b…`
   (12:45/13:02 evidence). The file's mtime is 2026-10-03 13:38:04 —
   after that review's last recorded write — and it hashes to
   `2e0e9e913b0e3ab3…`, i.e. the pre-approval value a8322627 recorded;
   the 13:38 writer is unidentified and the approved bytes were never
   captured, so byte-restoration was impossible.
2. **Count discrepancies** (§5a/§5b of the release-integrity note): the
   record's mint figure "+58/−0" (propagated from the roadmap phase's own
   claim) vs the measured +41/−0; compound-diff line figures.

**Disposition (per the note's instruction to "verify the residual
content manually"):** this gate re-read the entire record and re-verified
its factual claims against ground truth — every code anchor re-grepped
(governance.rs :852-860/:862-869/:787/:794; reports.rs
:527-529/:942/:956/:1104/:1117/:1362/:1183; pricing.rs
:48-49/:88/:141/:170/:483; main.rs :201/:252-258/:263/:766/:871-877;
docs :66→:71 drift explanation), test counts re-counted
(attribution_dimensions 8, governance inline 13/6-at-base,
discovery_contract 72, envelope 299/0), censuses re-derived (96 = 68+14+14;
ids 82 backticked, no dups, footer intact), and all diff figures
RE-MEASURED (mint +41/−0/51 lines; compounding +8/−4/41 lines;
roadmap-full +45/−0/103 lines). Corrections applied this turn: the two
stale §1/§5 compounding figures and the §1 docs-gate sentence (final
state rc0 green). The current record supersedes every earlier byte state
as the tree of record for this artifact; its sha256 is recorded in the
commit-gate result JSON.
