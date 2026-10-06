# Cycle-1 compound record — run db6b76260a11 (agenttrace)

Compound fold of repository-maintenance cycle 1 (repository-maintenance:b5aab57c,
worktree run-db6b76260a11-db6b7626, base 1511547 = origin/master at this run's roadmap
census). Batch: **rm-525** (LEAD, security: `-o` artifacts 0664 vs PRIVACY.md:20's
blanket 0600) + **rm-526** (torn-tail unparseable-line disclosure) + **rm-528**
(governance slow-tool severity inversion); rm-527 deferred. Written at compound attempt
2cae1fce (2026-10-06). This cycle's full_tests attempt survived a provider-dead prior
attempt (560b0821, reaped at message 3 — event log holds turn_started/progress/
session_reap_failed only, no typed artifact, zero durable work; forensically cleared and
redone as 0597e89a).

All outcomes below are **pre-review** evidence transcribed from the recorded phases —
NO test execution at compound, per the compound contract. Review and shipping outcomes
happen after this phase; the next cycle's assessment carries them.

## Cycle outcome (pre-review evidence chain)

- assess 248c0eb8 (fresh adversarial assessment at base 1511547, clean porcelain,
  157-title known-findings wall as dedup filter; 425/0 tests debug+release, 10 gates
  rc0) → A1 `-o` artifact modes 0664 with verbatim prompt text (→ rm-525), A2
  malformed-JSONL lines silently dropped by every format parser — torn-tail PoC
  undercounts usage with `skipped=0` (→ rm-526), A3 retained history unbounded
  (→ rm-527), A4 governance slow-tool severity inversion `p95=0.0s unmatched=1` → P1/high
  (→ rm-528).
- research 08d7fa55 → upstream moved: merge-base be25c4c = v0.9.0, tip 706bf58 = 24
  commits through v0.10.0, **#311 + #312 merged** (six-agent token-accounting wave;
  their verification: Claude input over-counted ~1.9x on 100/155 files, Codex
  +11.6%/+64%/+17%), #305 subagent attribution, #306 time-bucket reporting; live PoCs:
  CC parent+subagent sidechain journals parse as 2 phantom sessions; codex
  `custom_tool_call`/`_output` response_items (6 pairs, real 2026-10-05 journal)
  invisible at parser.rs:2647 **and unfixed upstream too**; LiteLLM live 4,473 entries
  vs bundled ~3,100 → 637 token-priced models absent.
- roadmap 22a6a5ad → three-home census (origin ceiling 1511547 = this base, landed wall
  max rm-444; sibling uncommitted bands moving mid-sweep), folded research into sibling
  rows, minted rm-525..rm-528 at the file tail; uncommitted `M ROADMAP.md` left for the
  commit gate.
- prioritize → ranked the open wall (74 rows; top open rm-320@100, rm-407@96, rm-322@88,
  rm-411@80, rm-412@76, rm-417@74; mints rm-525@72 / rm-526@62 / rm-527@58 / rm-528@52);
  sibling collision check clean; selected the 3-row batch, deferred rm-527.
- stewardship 7b0ee2fc → re-verified every surface anchor LIVE at base (write_output
  main.rs:1057, jsonl_objects parser.rs:4576, count_skip lib.rs:516, governance.rs:528,
  session_cache write_private precedent, PRIVACY.md:20) rather than trusting the rows.
- implement a29ea245 → 8-file tree delta (7 M + new tests/parse_torn_tail_contract.rs,
  +260/−14 at handoff): cli-side `write_private` sibling for `-o` artifacts (mode set on
  the TEMP file because rm-250's rename hands the destination a fresh inode → rewrites
  re-tighten), `jsonl_objects_counted` threaded through every probe-path finish +
  `--doctor` fold, governance severity gated on measured `is_slow` with
  measured-beats-unmatched dedupe preference.
- targeted_tests b8e3d432 → validation-only (porcelain byte-identical before/after):
  core 303/0 across 12 result lines (lib 174/0 incl.
  `unmatched_tool_evidence_never_outranks_measured_slow_tool_latency`;
  parse_torn_tail_contract 3/3), cli 80/0 across 5 suites (incl.
  `write_output_creates_owner_only_artifacts`), fmt rc0, clippy `--workspace --locked
  --all-targets -D warnings` rc0. First core attempt hit host lld thread-spawn
  starvation (`std::system_error: Resource temporarily unavailable` at link) — retried
  serialized green; see PR-6.
- full_tests 0597e89a → 22 lanes ALL rc0 mirroring `.github/workflows/ci.yml`
  `full`+`deny` verbatim (dispatch `full_command` empty; fleet precedent f093b7e5 /
  a0407d88): **430 passed / 0 failed in BOTH debug and release** across 17 suites
  (+5 vs assess 425 = exactly the batch's new tests), 9 artifact gates green against the
  release binary built from this tree, `cargo deny --all-features check` rc0. ONE
  in-turn fix (see PR-5): check-plugin-version's per-tag arm was red at this base —
  the shared canonical gitdir's tag namespace (inherited upstream tags imported
  2026-10-06 00:51Z by another run's fetch) exposes merged v0.7.2..v0.8.0 as section-less
  while base 1511547 predates origin/master's landed marker reconciliation (HEAD is an
  ancestor of origin/master 8991144); fixed by porting the landed 18-line
  `no-changelog-section` block VERBATIM (CHANGELOG.md:311-327, tail byte-identical to
  origin/master). No tags fetched, no tags deleted, no marker invented.

### Live PoC flips (recorded at implement, standing at compound)

Re-derive before citing: the `/tmp/db6b-*` corpora are /tmp and NOT durable across
phases (recipes in the assess/research phase results and this record).

- rm-525: `--overview -o` and `--search -o` artifacts land **600** under umask 0002
  (was 664); rewrite over an aged 0664 file returns to 600.
- rm-526: torn-tail corpus → `-f json` `data_health.line_skips
  {unparseable_line: 1}` + confidence low (was null/silent); `--doctor` → "Journal
  disclosures: unparseable_line=1" (was silent).
- rm-528: same corpus → slow-tool recommendation **P2/medium** (was P1/high), evidence
  line `tool=Bash p95=0.0s unmatched=1`.

### Digest lineage

`validation:v1:0ca09f019eb888c756ff8e98926c120bbfb2067d89136e1652957f6b62522158`
declared VERBATIM at targeted_tests and full_tests: the entire cycle delta is
crates/** + root docs (CHANGELOG/PRIVACY/ROADMAP), never executable-classified, so the
digest could not move between folds (see PR-7).

## Prevention rules (reusable, fleet-scoped)

- **PR-1 — Blanket privacy sentences are executable contracts.** PRIVACY.md:20's
  "every session-derived artifact … 0600" was live-false for report artifacts (0664
  measured). Rule: every new file-writing surface for session-derived data routes
  through the `write_private` helper family (rm-208's five sites + rm-525's cli-side
  sibling), a mode-assertion test rides beside the writer, and either the helper or the
  PRIVACY.md enumeration is amended in the SAME change.
- **PR-2 — A counter that never reaches a renderer is a silent drop.** rm-526's gap was
  not missing machinery: the fallback parser already counted `unparseable_line`,
  reports.rs already rendered `line_skips`, doctor already had a disclosures fold — only
  the format-parsers' thread-through was missing. Rule: when adding or discovering a
  skip/census counter, thread it end-to-end (data_health + --doctor + JSON provenance)
  in the same change; grep for the renderer before declaring a disclosure done.
- **PR-3 — Severity gates on measured evidence only.** rm-528: `if slow.unmatched > 0
  { "high" }` let a zero-latency phantom outrank a measured p95 and displace it in the
  (category,title) dedupe. Rule: severity/priority expressions read a MEASURED
  predicate (p95, count, bytes); presence-only evidence discloses at medium and never
  wins a dedupe against a measured row.
- **PR-4 — Set file modes on the temp file, before the rename.** rm-250's staged write
  renames a temp sibling onto the destination, handing it a fresh inode — chmod after
  rename re-tightens but leaves a window and misses nothing on rewrites only if
  re-run. rm-525 sets 0600 on the TEMP file, so every landing (fresh write and rewrite
  over an aged loose-mode file) is born tight.
- **PR-5 — plugin-version's per-tag arm keys the SHARED gitdir's tag namespace.**
  `git tag --merged HEAD` sees tags ANY lane fetched into /work/projects/agenttrace/.git;
  a base predating the marker reconciliation goes red the moment another run imports the
  inherited tag set (observed: 00:51Z 2026-10-06 import → red at base 1511547; the
  assess-phase green predates the import). Never `git fetch --tags` from a gate; never
  delete tags; the fix is porting origin/master's landed `no-changelog-section` block
  VERBATIM (CHANGELOG.md:311-327) so the lane's eventual merge is content-identical on
  both sides.
- **PR-6 — Empty dispatch `full_command` → ci.yml `full`+`deny` is the suite.** Mirror
  steps command-verbatim with CI's gate env (AGENTTRACE_BIN / AGENTTRACE_CI_OUT /
  AGENTTRACE_REAL_CLI_*), route ALL gate output to the delegate spool (repo-root
  `ci-artifacts/` is a hygiene trap even when gitignored), run cargo test lanes `-j 2`
  under host load (lld thread-spawn starvation now observed twice fleet-wide —
  `std::system_error: Resource temporarily unavailable` at link; serialized retry
  green), and use the corrected deny order `cargo deny --all-features check`.
- **PR-7 — Digest discipline.** Root docs + crates/** deltas are never
  executable-classified under this repo's classifier, so a dispatch digest stays
  VERBATIM across folds — prove it with porcelain plus a name-only classify
  (`scripts/**`, `.github/**`), and never substitute a locally re-derived digest for a
  dispatch stamp (the 0ca09f01 divergence forensics stand).
- **PR-8 — Provider-dead attempts: read the event log tail before redoing.** This cycle
  cleared one (560b0821, reaped at message 3). Typed artifact absent + event log showing
  early `session_reap_failed` = transport death, zero durable work; declare the
  forensics, redo the phase. An absent envelope is not evidence no work happened — but
  this anatomy is.

## Next-cycle candidates and context

- **Ready lead: rm-527 (reliability, 58.0)** — bound retained history growth
  (history.rs merge-then-whole-file rewrite, no cap constant at base 1511547; anchors
  re-verified live at stewardship 7b0ee2fc). Reuse the statusline compact-to-half
  pattern per its acceptance; its truncation disclosure composes with rm-526's
  data_health plumbing — sequence them through the same disclosure surface.
- **Top of the open wall** (prioritize's def-row parse, 74 rows): rm-320@100 (cli),
  rm-407@96 (docs), rm-322@88 (docs, in-band provenance disclosure — boundary-checked
  against rm-525: no overlap), rm-411@80, rm-412@76, rm-417@74.
- **Research-fused candidates** (folded to sibling rows at this cycle's roadmap — do
  not re-mint): upstream #312 six-agent token-accounting wave (sibling "Arbitrate
  upstream PR #312" row; SUPERSEDES the #286 high-water approach for codex — rm-035's
  supersession watch line records the re-verification requirement), #305 subagent
  attribution (three sibling rows; PoC /tmp/db6b-res-cc is ephemeral — re-derive:
  CC 2.0.76 same-dir `agent-*.jsonl` with isSidechain+shared sessionId must key to the
  parent, NOT upstream's `subagents/` subdir shape), codex `custom_tool_call` response
  items (sibling rm-450..452 rows; net-new in BOTH lineages — upstream-contributable),
  #306 time buckets, #311 WorkBuddy (now adopt-the-patch), #301 self-updater.
- **Watch list:** upstream is no longer quiet (post-v0.10.1 wave #313/#314/#315); host
  CC 2.0.76 vs npm 2.1.289 (rm-406's drift driver); LiteLLM 637-model drift (rm-006's
  2026-10-06 refresh line records the census); bare `agenttrace` npm name still free;
  PR-5's tag-namespace hazard for any stale-base lane.
- **Numeral discipline:** no numerals minted at compound. This lane's roadmap banner
  recorded "next free rm-529" AT ITS MINT; the fleet census (~16:00Z 2026-10-06) shows
  sibling bands through rm-550 → fleet next-free rm-551. Next cycle's roadmap phase
  MUST re-census live (lanes land concurrently) and reconcile by title at integration
  per the 880a7b9e discipline.

## Commit-gate handoff

ONE commit for the whole batch, still uncommitted in the worktree: 7 modified
(CHANGELOG.md now +21 — 3 batch entries + the 18-line ported marker block, byte-identical
to origin/master's tail; PRIVACY.md; ROADMAP.md grown by roadmap + compound; main.rs;
doctor.rs; governance.rs; parser.rs) + 2 new (crates/agenttrace-core/tests/
parse_torn_tail_contract.rs; this record). Do NOT `git fetch --tags` before or during
the plugin-version gate (PR-5). Done-flips for rm-525/rm-526/rm-528 are reserved for
the commit gate; review/shipping outcomes deliberately absent here.
