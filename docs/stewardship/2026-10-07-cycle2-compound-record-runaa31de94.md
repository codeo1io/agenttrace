# Cycle 2 compound record — run aa31de94ea9f (2026-10-07, pre-review)

- **Run:** `aa31de94ea9f4083bbf93415e0fafb6e` (repository-maintenance `fad1cbf4f7684a158371444e113be579`, cycle 2)
- **Base:** HEAD `be2428849964cd1283ee1069c670eee72503251b` (unchanged all cycle; everything below is an uncommitted worktree delta in `run-aa31de94ea9f-aa31de94`)
- **Batch:** "time-truthful statusline" — rm-628 (lead, correctness/79 calendar cutoff) + rm-630 (reliability/61 single journal parse) + rm-165 (reliability/78 timezone-labeled resets_at); one crate, one file cluster (statusline.rs budget/reset rendering), one theme: what the statusline says about time is true.
- **Status at compound:** implemented, pre-review. Review and shipping happen after this step; the next cycle's assessment carries them forward. Done-flips reserved to the commit gate (rm-012 precedent).

## Attempts and forensics

| Phase | Attempt | Note |
| --- | --- | --- |
| assess | fff62475 | adversarial sandbox /tmp/at-assess-aa31 — PoCs poc-budget (stale-window), poc-bidi, neg.toml; wall-titles census (204 defs) |
| research | 12096915 | upstream/ecosystem memo (fork 124 behind; #311/#312; OTel; ccusage flex; codeburn) — /tmp/at-research-1209/memo.md |
| roadmap | e2b3b493 | minted rm-628..rm-631 + 6 dated riders (+36 ROADMAP lines), id-frontier sweep |
| prioritize | 17e5c47a (dead) → 7b506491 | 17e5c47a reaped with zero durable work (4 events: start + ping + reap + failed) → batch selection redone |
| stewardship | 14550a34 | change-unit contract, candidate checkout = this worktree @be24288 |
| implement | 7aeda2c7 (dead) → a946887c | 7aeda2c7 reaped AFTER writing the real uncommitted statusline.rs delta — adopted with live verification against the rm-628/rm-630 acceptance rows, then extended by a946887c |
| targeted_tests | 4f9b000a | core 346/0 + cli 109/0 (statusline module 20/0); digest re-derived == dispatch token |
| full_tests | 5f6fee30 (dead) → 4dfbff2d | 5f6fee30 reaped 26s in (event log = start + 1 ping + reap + failed; /tmp window scan empty) → zero durable, redone from scratch |
| compound | b0476222 | this record |

**Dead-attempt pattern (recurring, now 5+ fleet instances):** reaping proves nothing in either direction. This run had three reaped attempts and ONE of them (implement 7aeda2c7) carried the entire durable delta. The reliable triage: (1) read the event log — pings-only with no work-shaped events is the zero-durable signature; (2) check the typed envelope; (3) sweep /tmp for scratch in the attempt's time window (event timestamps bound it); (4) diff the worktree against the previous phase's recorded census. Adopt only after live verification against the owning rows' acceptance lines; redo from scratch and declare it otherwise.

## What was implemented (uncommitted delta, 4 files +325/−32)

- **rm-628 calendar cutoff** — `crates/agenttrace-core/src/statusline.rs`: `statusline_budget_series` gained a `now_epoch` parameter; the `split_off` trailing-days-WITH-SAMPLES window became a calendar-day retain with cutoff `utc_day(now − (days−1)·86400)`; rises still computed over full history (monotone %-of-budget). Help (cli main.rs) + `docs/guides/statusline-capture.md` state the cutoff semantics. Red-green: stale corpus (2026-09-16/17 captures, reported 2026-10-06/07) golden `$5.00 / 50%` → patched `$0.00 / 100%` with `(no cost samples in the window)`; JSON `daily[]`, `spend_7d_usd 0.0`, `remaining_usd 10.0`.
- **rm-630 single journal parse** — `render_budget_view` text path parsed statusline.jsonl three times (empty-gate stats read, a discarded series read, the final series read); now one CAPTURES parse per invocation — the empty-gate reads `statusline_journal_stats`, the line-count read the JSON output contract requires regardless (journal read twice, captures parsed once; review fix F3 wording). Byte-identity proven on a fresh corpus: `--budget` text/json + `--statusline-report` json all IDENTICAL golden-vs-patched.
- **rm-165 timezone-labeled resets_at** — `format_epoch` resets_at rendering appends the host-local UTC offset. Probes (resets_at 1760052600): golden TZ-blind `until 23:30` → patched `23:30+0000` (UTC), `08:30+0900` (Tokyo, midnight crossed), `18:30-0500` (Lima). The insights.rs daily-bucket half of this row's signal stays rm-040's per the 2026-10-03 integration note.

## Recorded validation outcomes (pre-review; consumed at compound, NOT re-run here)

- implement a946887c: core 311/0 (statusline 20/0, 6 new/updated), cli 109/0, clippy `--workspace --all-targets --locked -D warnings` rc0, fmt rc0, docs gate rc0 — evidence `/tmp/at-implement-a946/evidence.md`, digest.txt (both mirrored, below).
- targeted 4f9b000a: core 346/0, cli 109/0, statusline module 20/0 (183 filtered), fmt/clippy/docs rc0; digest re-derived byte-identical to the dispatch token.
- full 4dfbff2d: ci.yml full+deny mirror verbatim (dispatch `full_command` empty), 23 runnable steps rc0 — cargo test 502/0 across 21 result lines, release build rc0 (39.84s), entrypoints 33/0, all 8 env-gated scripts rc0 (private `AGENTTRACE_CI_OUT`), ruby/npm-test/manifests/plugin-version/script-syntax/locked-cargo rc0, `cargo deny --all-features check` ok (advisories/bans/licenses/sources); MSRV floor (schedule/dispatch-only) and TUI real-smoke (repo var unset) skipped per their own CI conditions. Companion: `/tmp/at-full-4dfb/COMPANION.md` (mirrored, below).
- Digest lineage: `validation:v1:b630f4a53f556d797678c72764a92c7ed7666c3cd13fbc905dba496903c056bc` stable targeted → full, re-derived byte-identical after the full run; compound touched ROADMAP.md + this doc only (non-executable).

## Prevention rules (fleet-reusable)

- **PR-1 Dead-attempt triage cuts both ways.** Reaped ≠ nothing happened (implement 7aeda2c7 carried the whole delta); reaped ≠ happened (5f6fee30, 17e5c47a carried nothing). Triage = event-log shape + envelope + /tmp window bounded by event timestamps + worktree census; adopt only on live verification against the owning acceptance rows.
- **PR-2 Engine classifier blindspot (crates/** layouts).** `validation_policy.classify_surface` matches root prefixes only (`src/ tests/ scripts/ lib/ bench/ .github/workflows/`) — zero `crates/**` Rust files classify executable, so `changed_testable_surfaces=[]` and `required_scope` collapses. Declare the full true change-set in `validation_evidence.changed_surfaces`; treat module-filtered cargo runs as the real targeted evidence. (Fleet rule #16653, re-confirmed both at targeted and full this run.)
- **PR-3 check-docs-commands.sh probes target/release/agenttrace.** Build the release binary BEFORE the docs gate or read rc=1 as a missing-probe false alarm (hit at implement, avoided at full).
- **PR-4 cargo-deny 0.20.2 argument order.** Local invocation is `cargo deny --all-features check` (feature flags precede the subcommand); the CI action's literal `check --all-features` order errors rc=2 locally. Plain `cargo deny check` is equivalent when no features are needed. (Refines the 250cfd64 note: the flag exists, it is an ordering constraint.)
- **PR-5 CI-conditional lanes must be recorded as conditional skips.** MSRV floor (schedule/dispatch-only) and TUI real-smoke (`AGENTTRACE_TUI_REAL_DIR` repo var) never run on default CI or a local mirror — cite the gate's own `if` as proof; never omit silently.
- **PR-6 Empty validation.full_command ⇒ ci.yml is the suite authority.** The `full` + `deny` jobs enumerate the authoritative steps; mirror them verbatim with private `AGENTTRACE_CI_OUT` and env vars as CI sets them.
- **PR-7 Time-label truth bug class.** When a rendered label asserts time semantics ("last 7 days", "until HH:MM"), pin a test that crosses the semantic boundary: a journal whose newest sample is older than the window, and a TZ change that must move the rendered offset. The pre-fix suite was green because no fixture crossed either boundary — the defect class is invisible to in-window fixtures.
- **PR-8 /tmp sweep: mirror evidence at the END of each producing phase, not at compound.** /tmp/at-assess-aa31 was swept between implement and compound (poc-budget + poc-bidi gone); only the implement-phase mirror survived. Preserve PoC corpora, goldens, and memos into the delegate spool scratch immediately after the phase that produces them records green.

## Mirrored evidence (delegate spool scratch, md5-verified copies)

`/home/agent/.hermes/conductor-delegate-spool/delegate/b0476222927f47feae6b3a1fd18ac246-scratch/`:
- `corpora-mirror/` — corpus-stale/ + corpus-fresh/ (statusline.jsonl fixtures), stale/fresh golden+patched .txt/.json/-report.json, `evidence.md`, `digest.txt`, step03-tests.log
- `research-memo-1209.md` — the research phase's memo (upstream drift, #311/#312, OTel, ccusage, codeburn)
- `full-tests-companion-4dfbff2d.md` — the full-suite companion (23 steps, rc's)

Swept and NOT recoverable from /tmp: the assess-phase PoCs (poc-budget reproduced by corpus-stale; poc-bidi re-derivable from rm-629's byte spec), neg.toml, wall-titles.txt (regenerable by grep).

## Next-cycle context

1. **rm-629 (security/65) — designated cycle-3 lead.** Cf/bidi/zero-width neutralization at every render boundary (sanitize_line_segment covers C0 only; U+202A-202E, U+2066-2069, ZW*, FEFF pass); composes with rm-540's shared render-boundary helper; TSV formula-guard lane stays unguarded by design but bidi is orthogonal. PoC swept — re-derive from the row's byte spec (session named `invoice<U+202E>txt.exe`).
2. **rm-631 (upstream-sync/74).** Four accounting arms + gemini keep-vs-drop; REAL codex rollout fixture before the math change; schema rung past 28 coordinated with rm-042; reconcile by subject vs sibling unlanded arms. Memo mirrored (above).
3. **Watch items:** upstream v0.10.2 absent as of 2026-10-06; OTel semconv-genai still 0 tags (rm-229 parked); rm-042 usage_points port remains the structural fix class for window bucketing.
4. **Review-surfaced candidates (round 1, F4/F5 — low, deferred per the review's own disposition):** F4 statusline resets_at offset should be derived at the reset instant, not render time (DST-boundary case: TZ=Europe/Berlin resets_at 2026-10-26T12:00Z renders 14:00+0200 vs wall-clock 13:00+0100; natural home beside rm-042's --tz plumbing); F5 future-dated captures in the budget window remain unclaimed (rm-628's acceptance covers "older than the cutoff NEVER count"; a symmetric upper bound + a stats-only pre-filter for wide journals were the reviewer's other halves — no test relies on them today, no verdict owed).

## Commit-gate seams

- 4 modified tracked files (ROADMAP.md, statusline.rs, cli main.rs, statusline-capture.md) + the UNTRACKED stewardship record (this file) — stage explicitly; `git commit -am` would drop the record.
- ROADMAP conflict seams: the cycle-2 compound banner + 3 status flips + 2 riders vs sibling bands at `## Open items` — union-resolve in fleet order, then this compounded postimage.
- CONTENTION CORRECTION (review round 1, F1 — re-verified live at fix time): the prioritize-phase zero-overlap sweep was true when taken (~2026-10-06 22:30Z) but a review-time re-sweep found 3 sibling worktrees dirty on statusline.rs — run-7e00d9cbe20a @be24288 (SAME base, repository-maintenance e74cb714 c1) carries PARALLEL claims of the same two defects (their rm-402 = this rm-628, windowing before rise attribution; their rm-684 = this rm-630) + rm-683 (XDG empty-env guard, disjoint); run-52465b9e @ce27969 and run-d6432dd5 @700a67c have no id claims. Integration reconciles BY SUBJECT (merge-by-title; this lane first-in-time, full-tests-validated, review-approved code; sibling lane not yet reviewed).
- CHANGELOG.md Unreleased riders REQUIRED in the landing commit (review round 1, F6): budget-window entry (rm-628: stale-journal weeks no longer count as "last 7 days" — the $5.00→$0.00 class) + statusline offset-label entry (rm-165: "until 23:30" → "until 23:30+0900"); no sibling CHANGELOG text on disk at fix time (run-7e00d9cb unstaged+staged both empty) but reconcile by subject at integration.
- Done-flips for rm-628/rm-630/rm-165 reserved to this gate after review (rm-012 precedent); rm-165 is arm-clean after its review-fix F2 re-homing rider (configurable-tz arm → rm-042).

## Review round 1 disposition (2026-10-07, review 638aaaa6 → fix 8cf8dfae)

Verdict NEEDS_CHANGES — record-level only; the code change was approved with zero code defects. All fixes applied this turn are record-level (ROADMAP riders/banner clauses + this record); NO executable surface was touched and NO test was run (dispatch validation block: required_scope=none, changed_testable_surfaces=[] — the crates/** classifier blindspot, PR-2).

- **F1 (medium) stale-false single-claimant claim** — corrected in the ROADMAP banner's COMMIT-GATE SEAMS clause and the seams section above; contention facts re-verified live at fix time (3 siblings dirty on statusline.rs, incl. same-base parallel claims run-7e00d9cb rm-402/rm-684; their CHANGELOG text absent from disk, unstaged+staged both empty).
- **F2 (medium) rm-165 configurable-tz arm homeless** — dated rider on rm-165 re-homes "insights daily buckets honor a configurable timezone" to rm-042 (upstream #306 --tz port); row arm-clean, implemented flip + commit-gate done-flip honest.
- **F3 (low) rm-630 EXECUTED wording overstatement** — wording corrected on the row and in "What was implemented" above; the deviation from the literal acceptance documented as deliberate design bounded by the byte-identity proof.
- **F6 (medium) CHANGELOG disclosure gap** — CHANGELOG seam added to the gate list (banner + seams); entries to be written at the landing commit, reconciled by subject.
- **F4/F5 (low)** — deferred to next cycle per the review's own disposition; recorded in Next-cycle context #4.

Digest at fix exit: unchanged `validation:v1:b630f4a53f556d797678c72764a92c7ed7666c3cd13fbc905dba496903c056bc` (executable-only digest — record-level fixes move nothing).
