# Cycle 2 compound record — repository-maintenance 205f84bb, run 614624d7 (2026-10-07)

- **Run**: 614624d710f14283b45986af911ceb4d · repository-maintenance 205f84bbe9ad4847af3a2297f4bbc54d · cycle 2
- **Worktree**: `run-614624d710f1-614624d7` @ base `6b03087` (cycle-1 base was `2a024b6`), carrying the uncommitted 9-file cycle delta (8 authored + ROADMAP.md).
- **Compound attempt**: 29149e8509544cb98d1c84d653096acb (first attempt; no prior compound attempt trail).
- **Contract honored**: pre-review evidence only — every gate outcome below was RECORDED by the targeted_tests (7807) and full_tests (09fa) phases and is cited, not re-run, per the compound phase contract.
- **Durable artifacts of this phase**: the cycle-2 compound banner on ROADMAP.md, the four candidate→implemented flips + evidence lines on the batch rows, and this record.

## 1. Cycle shape and attempt forensics

| phase | attempt | prior-attempt forensics |
|---|---|---|
| assess | 72d5d236 | prior 5775ef0f provider-reaped at message 3, zero durable work → redone from scratch |
| research | 43d277ee | — |
| roadmap | 3e23d1ca | prior 1f9af6b3 reaped at ~25s (10 messages, session_reaped), zero durable work → redone |
| prioritize | 8416c9d0 | — |
| stewardship | e210c4fa | — |
| implement | cc7af0fb | prior d5d05946 absent (no typed artifact; event log = 1 heartbeat) → redone |
| targeted_tests | 7807… | — |
| full_tests | 09fa… | — |
| compound | 29149e85 | — |

Three phases lost a prior attempt to provider reaping with ZERO durable work. The forensics
pattern (typed result absent + event log progress-pings/heartbeat-only + porcelain check →
redo from scratch, never adopt) is the complement of the reap-race harvest rule: a reaped
attempt whose result JSON exists in the spool IS durable work and must be adopted, not redone.
Both rules now proven in this fleet within 24h.

## 2. Batch and delta — "Green gate, truthful entrypoints"

Selected by prioritize 8416c9d0 from the 118 open candidates on the 209-def wall; theme: the
tool must be GREEN on its own gates and TRUTHFUL about what it did with what the user asked.
Implement order rm-592 → rm-248 → rm-439 → rm-593, all landed in the uncommitted delta:

1. **rm-592 (LEAD, reliability 76.0)** — re-green the ci.yml Lint lane. `UsageCheckpointSnapshot`
   type alias at the `checkpoint_snapshot` binding (parser.rs; clippy type_complexity, net-new in
   f59a67d) + three needless_borrow drops `governance_inspect_flag(&args)` → `(args)` at
   main.rs:680/:686/:689. Lane rc=101 → rc=0. Process-guard arm landed as the new **"Merge &
   Integration Validation"** rule in `docs/maintainers/agentops-prompt-rules.md` (verbatim CI
   clippy lane AND `--workspace --all-targets` before declaring any merge round green).
2. **rm-248 (compatibility 74.0)** — relative `XDG_CACHE_HOME` ignored per the basedir spec:
   `cache.is_absolute()` gate at every private `user_cache_dir()` XDG arm
   (session_cache.rs/doctor.rs/pricing.rs/statusline.rs); relative ⇒ unset ⇒ `$HOME/.cache`.
   Pin test `relative_xdg_cache_home_is_ignored_per_spec` (3 public constructors +
   unset-equivalence); PRIVACY.md cache-root paragraph made truthful.
3. **rm-439 (correctness 64.0)** — extra positional session paths rejected loudly (rm-247 house
   rule): rc=2, names the dropped operand and the honest route (`-d <dir> --compare`), keyword
   arm for statusline/upstream; tolerates-pin REPLACED by two rejects-pins.
4. **rm-593 (developer-experience 46.0)** — validated report-shaping flags shape or reject:
   `validate_sample_applicability` + `validate_history_applicability` (rm-244 `--range` sibling
   shape) wired after the range guard; `sample_consumed()` predicate; --help scoping widened.

**Exit-code convention (sibling-consistent)**: applicability guards exit 1 (rm-244); go_flag
shim errors exit 2 (rm-247). **No schema bump** — SESSION_CACHE_SCHEMA_VERSION stays 26 (cache
format unchanged; only root resolution). Users who relied on a relative XDG root take a one-time
cold rescan from `$HOME/.cache` (fallback semantics, not an invalidation event).

## 3. Recorded validation outcomes (pre-review; NOT re-run at compound)

**Targeted (7807…)** — fmt rc0 · clippy ci-lane (`-p core -p tui -p cli -- -D warnings`) rc0 ·
clippy `--workspace --all-targets --locked` rc0 · core lib 182/0 (incl. the rm-248 pin) · cli
94/0 over 6 targets (bin 42 + csv 7 + entrypoints 30 + launch_guards 4 + upstream 9 +
warm_cache_pricing 2) · tui lib 47/0 · porcelain identical 9-file M before/after, no residue.

**Full (09fa…)** — ci.yml jobs `full` + `deny` mirrored verbatim (runner `/tmp/at-full-09fa/run-full.sh`):
**21/21 lanes rc=0 on FIRST PASS, zero fixes.** 458 passed / 0 failed across 21 suites; release
build rc0; all `scripts/ci/*` gate scripts rc0 with AGENTTRACE_BIN pinned; ruby -c homebrew OK;
npm test rc0; cargo deny (all four categories) ok. Documented divergences (env-var artifact
dirs, repo-var-gated TUI-real-smoke skipped, no `git fetch --tags` from a delegate worktree,
cargo-deny flag-order gotcha) — none semantic.

**Validation digest**: `validation:v1:7d172c525350fbc3488453aa947e6ff62f019deb144ca0c5c5dd98545166d6e9`
— declared VERBATIM at both validation turns; re-derived byte-identical with the engine's own
`validation_policy` before and after each battery.

**Note on the dispatch scope**: both validation work orders derived empty scope
(`changed_testable_surfaces=[]`, `full_command=''`) — the documented hermes-conductor
`classify_surface` blind spot for agenttrace's `crates/**` layout (root-prefix matcher).
Workaround used, per fleet precedent (db6b76260a11): targeted phase declared scope `targeted`
from the REAL git delta; full phase ran the repository's authoritative ci.yml full+deny jobs
verbatim. **Prevention: an empty derived scope for this repo means "derive from the real delta",
never "no validation needed".**

## 4. Live red→green probes (implement cc7af0fb, debug binary; red sides recorded by assess/prioritize)

- `--compare multi-shutdown.jsonl shutdown-only.jsonl`: rc0 "(auditing 1 of 1 sessions)" → **rc=2** naming the dropped `shutdown-only.jsonl` + the `-d <dir> --compare` route.
- `--demo --overview --sample 1 -f json`: cmp-byte-identical to unsampled rc0 → **rc=1** naming `--sample`, the ignoring lane, and the consuming lanes.
- `--include-history` alone (TUI lane): silent no-op → **rc=1** naming the pair + actions; controls (`--demo --audit -f json --sample 2`, `--demo --overview --include-history -f json`) rc0.
- `XDG_CACHE_HOME=rel-cache` from a scratch cwd: created `./rel-cache/agenttrace/{sessions.json,hermes-sqlite.json,opencode-sqlite.json}` → **rc0 with nothing under ./rel-cache**, artifacts at `$HOME/.cache/agenttrace`.

## 5. Durable lessons / prevention rules (this cycle's additions)

1. **Lint-lane rot root cause**: every integration/merge round since f59a67d ran `cargo check` +
   `fmt` only, so four clippy `-D warnings` errors (type_complexity + 3 needless_borrow) landed
   undetected and the ci.yml Lint fast lane went RED between cycles. Guard now in-repo:
   the agentops-prompt-rules "Merge & Integration Validation" rule. Always run the VERBATIM
   ci.yml lane form (`-p agenttrace-core -p agenttrace-tui -p agenttrace`), not an ad-hoc
   clippy invocation — the package set is the contract.
2. **Row-anchor drift**: rm-439's 2026-10-04 anchors (main.rs L945/L952/L1737) had drifted to
   main.rs:1042 (load site) / :873 (shim comment) / :1892-1900 (pinning test) by base 6b03087.
   Stewardship re-verified every anchor live before implement. Rule: row anchors are historical
   evidence, not current coordinates — re-verify at the base you implement on (echoes the
   never-reconstruct-anchors lesson).
3. **Reaped-with-zero-durable-work forensics** (3-for-3 this cycle): typed result absent +
   event log pings/heartbeat-only + porcelain check ⇒ redo from scratch, declare the redo. The
   reap-race harvest rule (complete result JSON in spool = adopt) is the complement; do not
   conflate them.
4. **classify_surface crates/** blind spot** (above, §3 note).
5. **/tmp fixture reaping hits within one cycle**: BOTH this cycle's corroborating copilot PoC
   fixtures (`/tmp/at-research-43d2/sessions/multi-shutdown.jsonl`,
   `/tmp/at-assess-6146/probe/sessions/partial-shutdown.jsonl`) were reaped before compound.
   The red-first RECREATION RECIPES are pinned in §7 so the rm-584 implementer never needs the
   files. Recipes > files: write the recipe into the durable record, not just the path.
6. **Timeout seam**: release-build-dependent gate scripts (`check-report-semantics.sh` etc.)
   can exceed the 900s bash timeout when run mid-implement before a release build exists —
   run them in full_tests after the release-build lane (done: lanes 06-08 rc0), not during
   implement.
7. **Mid-cycle ceiling motion**: origin/master moved twice past this run's base during the
   cycle (8991144 → 05d5016d at stewardship → 1c5edd1 at compound, ls-remote live). Assume the
   base is stale by commit time: rebase, re-anchor, re-clippy at the tip.

## 6. Status accounting (this compound)

- Wall: 209 def rows, max def rm-593, zero duplicate ids, managed footer still last line.
- Status: 129 candidate / 37 done / 43 implemented → **125 / 37 / 47** (four candidate→implemented
  flips made at this compound with evidence lines: rm-592, rm-248, rm-439, rm-593).
- `done` flips stay reserved to the commit gate (rm-012 convention).
- Campaign-local ids: this wall mints through rm-593; other fleet streams minted well past that
  concurrently — at integration, renumber BY TITLE, never by numeral.

## 7. Next-cycle context — candidates, folds, recipes

**Natural lead (from prioritize 8416c9d0's deferral rationale)**: **rm-249** (correctness 70.0,
discovery-walk failure disclosure + truthful `-d` errors) with its P13 rider (`-d <dir> <file>`
silently ignores `-d`) — same entry-surface-honesty family as this batch; a report-surface
change that fits one cycle.

**Named candidates**:
- rm-017 (74.0) install-surface refresh — pinned-release + checksum + clean-room install per OS.
- rm-451 (74.0) first-scan peak memory — streaming rework, its own cycle.
- rm-044 + rm-386 — upstream #318 turns the pending dep wave into a ureq-3 API migration
  (`Agent::config_builder().timeout_recv_response(5000)`, pricing body API; do NOT port
  upstream's 10MB→64MB body raise — the fork's own 32MiB `read_body_capped` cap must stay the
  binding limit) + MSRV truth (upstream advertises 1.89, fork 1.80 at Cargo.toml:12 vs the
  1.98.1 toolchain pin). Fresh evidence appended to both rows this cycle.
- Multi-cycle programs: rm-421 / rm-251 / rm-195 (90.0 band) — fork-identity and parser
  programs needing their own campaigns.

**Folds — implement via the OWNING unlanded rows, do not re-mint**:
- **rm-584** (fb1addd5 band) copilot multi-snapshot token reconciliation — this cycle added TWO
  corroborating live PoCs to rm-485's boundary note. Fix sketch (assess N2): track the
  shutdown-emitted model set and emit checkpoint entries whose model is not in it
  (parser.rs:303-331 per-model loop; :310 sets the flag only on successful parse).
  Cache-bump decision rides the rm-230 convention (reported tokens change on unchanged files).
- rm-574 (de96d4cc) keyword-dispatch swallows (--statusline-report statusline runs the host,
  main.rs:192 vs :298; -f csv/markdown on upstream render text, :199 vs :213/:220) +
  upstream.rs:32 DEFAULT_REF.
- rm-590 (d65f72c7) /dev/null path conflation; rm-587 (d65f72c7) lru unsound-class OSV policy
  gap (deny says ok, RUSTSEC-2026-0253 listing exists).

**Red-first recreation recipes (fixtures reaped from /tmp; shapes verified live this cycle)**:
- `multi-shutdown.jsonl` (research 43d277ee RC-1; mirrors ccusage #1823 resumed-session shape):
  one copilot session jsonl, two session.start/shutdown pairs, cumulative per-model modelMetrics
  100/50 then 250/125, totalNanoAiu 1.5e11 then 4.521e11 → current binary reports tokens
  350/175 (SUMS both snapshots) vs truth 250/125 at the final snapshot; cost 4.521 via the
  credit max (correct arm). With token-priced > credit models the inflated token arm would also
  win `cost = max(token, credit)`.
- `partial-shutdown.jsonl` (assess 72d5d236 N2): a session.usage_checkpoint carrying
  claude-4.5 {input 1000, output 500} + one session.shutdown whose modelMetrics carries ONLY
  gpt-5.4 {10,5}, totalNanoAiu 350000000000 → report tokens_input 10 / tokens_output 5
  (claude-4.5 dropped wholesale by the `!shutdown_metrics_emitted` gate at parser.rs:367),
  cost_estimated 3.5, provenance calculated_from_copilot_credits. No test covers partial
  shutdown today (discovery_contract.rs:733 = empty modelMetrics; :765 = checkpoint-only open).

**Watch items**: pi 1.0.3/1.0.4 drift check NEGATIVE (session-format.md byte-identical,
discharged on rm-423's row); LiteLLM 4,480 keys vs bundled 3,099 and models.dev 226 providers /
8,390 models — the rm-006 cadence/drift-disclosure gap keeps widening; ccusage #1823/#1824
(reconciliation contract: "gaps that a later shutdown attributes are not counted twice") and
upstream #317/#318 remain the live upstream references.

## 8. Commit-gate seams (for review/shipping, which happen AFTER this phase)

1. **PRIVACY.md is a protected surface** under docs/maintainers/agentops-prompt-rules.md
   (linked issue + maintainer approval): the shipping PR must cite the rm-248 wall row + this
   conductor batch as the coordination record.
2. **Rebase at the tip**: origin/master is at 1c5edd1 (live at compound), two moves past this
   base — re-verify the batch's file:line anchors and re-run the clippy ci lane at the tip.
3. **Renumber BY TITLE at integration** (campaign-local numerals; concurrent streams minted
   past rm-593); the managed footer must stay the last line of ROADMAP.md.
4. **rm-592 borrow-drop region** is byte-identical to d65f72c7's uncommitted targeted-allow
   repair — reconciles trivially; rm-592's row notes record the reconciliation.
5. **CHANGELOG rider** for the batch lands with the merge per lineage convention.
6. `done`-flips for rm-592/rm-248/rm-439/rm-593 are reserved to the commit gate.

## 9. Evidence ledger

- Assess dossier: `…/conductor-delegate-spool/72d5d236ac384e9e8e1dd735d50fc6da-scratch/assessment.md` (+ /tmp/at-assess-6146/ logs — reaped)
- Research memo + upstream fetches: `…/conductor-delegate-spool/delegate/43d277ee75ca49c5aebe5cc119bbe60e-scratch/`
- Roadmap delta (this run's roadmap phase): `…/delegate/3e23d1ca9e9142918724312ba7fee647-scratch/roadmap-delta-3e23d1ca.patch`
- Batch selection: `…/delegate/8416c9d03ca3493280ad5ddcfc6143f7-scratch/batch-selection.md`
- Stewardship request: `…/delegate/e210c4fa54b34379b23fdc4e6b3e143a-scratch/stewardship-request.md`
- Implement handoff: `…/delegate/cc7af0fb83a540978821cd849f5a73ee-scratch/handoff-memo.md`
- Targeted logs: `/tmp/at-tt-7807/{fmt,clippy-ci-lane,clippy-all-targets,core-lib,cli-pkg,tui-lib}.log`
- Full runner + 21 lane logs: `/tmp/at-full-09fa/` (run-full.sh, lane-results.txt, logs/01..21.log, ci-artifacts/)
- This phase's scratch: `…/delegate/29149e8509544cb98d1c84d653096acb-scratch/` (roadmap delta patch + this record's source copy)
