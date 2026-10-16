# Cycle 2 compound record — run 7fd83ab2fd7f (repository-maintenance 86b80027)

Date: 2026-10-16 · Base: e130845c3b57154a608c525c275a29b34b21a3ef (master tip, unchanged all cycle) · Compound attempt: 4a21788a (pre-review, redo of transport-dead 1291939a)

## Batch

Theme: *evidence-lane paste-safety + claude-lane format-compat* (prioritize 3d3090af selection, stewardship 48309680 request — both consumed, not redone).

| id | title | role | state after compound |
|---|---|---|---|
| rm-540 | `--search` evidence rows: guard the VALUE, not the joined line (assess cbf457f4 F1, live-proven) | lead (UNIT A) | **in_progress, search-lane arm EXECUTED + VALIDATED** (dated rider on the row; compare-lane sibling-join sweep + shared-helper enumeration open; done-flip reserved to the commit gate, rm-012) |
| rm-1522 | Claude Code v2.1.300 request-reproduction records disclosed, never silently skipped (research f4bdc13e C2) | unit (UNIT B) | **candidate, arms (1)–(4) EXECUTED + VALIDATED** (live-2.1.300 re-capture arm open when the host upgrades past 2.0.76; done-flip gate-reserved) |

Uncommitted worktree delta carried to the commit gate: 4 M + 2 ?? from implement
(diff md5 `69988a89b46d3efc4d3c3cd1ad53137e` — `CHANGELOG.md`,
`crates/agenttrace-core/src/parser.rs`, `src/search.rs`,
`tests/formula_lane_contract.rs`, NEW `tests/claude_v2_1_300_request_reproduction.rs`
and `tests/fixtures/claude-code-2-1-300/request-reproduction.jsonl`) **plus this
compound's `ROADMAP.md` (band + banner + 2 riders) and three untracked docs**
(this record and the two prevention rules below). Porcelain at compound finish:
exactly 5 M + 5 `??`.

## UNIT A mechanics (implement 87c78056)

The defect was guard granularity, not guard absence: `add_match` joins
`label: value` (`search.rs:246` pre-fix) and the render loop guarded the joined
item, while `guard_formula_cell` (`reports.rs:3691`) only neutralizes an
introducer at the first non-whitespace character — dead behind every label. The
label-less summary row (`search.rs:146`) fired; that contrast localized the
seam to join order. Fix: `guarded_evidence_item` (`search.rs:141-146`) splits
the fixed label off (`split_once(": ")`) and routes the value through the
unchanged rm-985 stack; `reports.rs` untouched; `-f json` byte-lossless
(pinned). Red-first:
`search_evidence_rows_carry_the_value_paste_marker` FAILED on the joined-line
guard, then green (formula_lane_contract 6/0, upgraded to the strict
every-occurrence-marker-preceded form). Live flips with the tree's binary:
`--search cmd` on `tests/fixtures/formula-lane/hostile-tool.jsonl` →
`- tool: '+cmd|' /C calc'!A0'` paste-marked (cat -A); `--search review` →
`- name: '=HYPERLINK("http://evil.example","click-me") please review'` marked.

## UNIT B mechanics (implement 87c78056)

Upstream Claude Code 2.1.300 (CHANGELOG line 66, 2026-10-15) records each API
request's system prompt, tools, and parameters; the host runs claude-code
2.0.76, so no live capture exists — the vendored fixture is **hand-built from
the upstream contract** (provenance-pinned, known lines pinned against the
inline control corpus). `parse_claude_code_jsonl` now counts unmatched wire
kinds into `claude_unmatched_type:{kind}` (`parser.rs:5019-5028`, `:5265-5274`)
on a **content-gated** one-carrier meta event — the `!events.is_empty()` gate
is the cycle's live regression catch: the ungated carrier stole contentless
claude-detected journals from the generic lane
(`discovery_contract::rust_generic_jsonl_accepts_legacy_type_events_without_role`
FAILED live, then pinned green, 104/0). Tests 7/7 red-first (6 FAILED with the
counter absent): parse+disclose, known-shape identity with/without the records,
unarmed-known-kinds disclose too, 2,000-duplicate bounded-by-construction, era
paste-marker, `--doctor` counter surfacing, control-corpus byte pin.
`--doctor` discloses `claude_unmatched_type:request_reproduction=3` end to
end; the fixture's hostile payloads ride UNIT A's paste-safe lane
(`--search click` → `- tool: '=HYPERLINK(…`, `--search calc` →
`- tool argument: '+cmd|…`).

## Attempt trail (all reaps adjudicated)

- assess cbf457f4 → research f4bdc13e — clean; assess scratch swept to
  quarantine but its envelope text survived (the work-order record is the
  evidence channel).
- roadmap 07f38635 — prior attempt f097dfe6 REJECTED/redo (typed artifact
  absent; event log 2 messages/~27s, terminal failed(provider), zero tool
  calls — no work existed); phase redone from scratch; delivered SPOOL-SIDE.
- prioritize 3d3090af → stewardship 48309680 — clean.
- implement 87c78056 — clean (red-first both units; discovery_contract
  regression caught and gated in-turn).
- targeted_tests 5293d633 — clean, zero fixes (diff md5 in == out).
- full_tests 35937b27 — prior attempt 3fcebbdb REJECTED/redo (provider reap,
  event log turn_started + 1 progress + session_reaped; no scratch/envelope
  anywhere in spool, /tmp, /var/tmp, ~/.cache); phase executed from scratch;
  ci.yml:134 lane run VERBATIM with private TMPDIR + source touches against
  the shared-target staleness trap.
- compound 4a21788a (this record) — prior attempt 1291939a REJECTED/redo
  (typed artifact absent; event log 9 messages ~7 min of progress pings
  ending session_reaped; zero scratch anywhere in spool or quarantine —
  nothing durable, nothing adopted); redone from scratch and declared.

## Recorded outcomes (consumed as evidence at compound — NOT re-run)

- **targeted 5293d633**: formula_lane_contract 6/0,
  claude_v2_1_300_request_reproduction 7/0, discovery_contract 104/0,
  disclosure_case_fold 7/0, doctor_demo_contract 3/0, core lib 411/0; digest
  re-derived live == declared; `changed_surfaces.testable_surfaces == []`
  (every batch path is non-executable class).
- **full 35937b27**: `cargo test --locked -p agenttrace-core -p agenttrace-tui
  -p agenttrace` (ci.yml:134 lane, verbatim) rc0 — 67 suites, 1161 passed /
  0 failed / 0 ignored; batch suites in-lane (6/0 + 7/0); digest
  `validation:v1:04bee40f02cae6e46d476a8abc59aa409ec7cbc2c3c760d678e452f55cb37ef5`
  declared == re-derived, identical to targeted's token.
- **digest immobility at compound**: this phase changed only `ROADMAP.md` and
  `docs/*.md` — all non-executable class per the validation classifier — so
  the recorded digest carries unchanged; no validation command was invoked
  (compound phase contract).

## Cessions (adopt-not-reimplement; gate reconciles BY TITLE)

- **assess F2 (regional-twin pricing divergence, 496/2804 qualified snapshot
  rows)** — ceded in-flight to run 4cee65a1: their implement envelope
  (`results/48fff67f42c34948a885d4612493fb9b.json`) carries the
  `exact_retained_key` exact-first `lookup_context_window_in`/`pricing_source`
  ladder this run's F2 evidence pointed at. pricing.rs was the fleet's
  most-contended surface this cycle (SIX dirty siblings at the prioritize
  census) — never reimplement into that churn.
- **rm-018 rustls 0.23.46** — pre-verified this cycle (RUSTSEC-2026-0346..0349
  all patched ≥ 0.23.46, 0.23.46 un-yanked, `Cargo.lock` pins 0.23.45); sibling
  14fd8b0d carries the uncommitted lockfile bump — reconcile BY TITLE, never
  double-bump.

## Roadmap delivery + audit chain

The roadmap phase's band was recovered from quarantine
(`quarantine/scratches/20261015T193552Z/07f38635…/roadmap-c2-7fd83ab2.patch`)
and applied IN-TREE at compound: pristine + patch → recorded postimage
md5 `3b3b66b1cae0439514f083269124c213` byte-exact; the compound banner +
2 dated riders chain on top → in-tree `ROADMAP.md` md5
`7982026eac7a5c1d2b1831d8ffb3463d`, reproduced by
`roadmap-cumulative-4a21788a.patch` at fuzz=0 (round-trip byte-proof). Audit
copies: `delegate/4a21788aacde4277805d2c3de7ab78bb-scratch/`
(`ROADMAP.pristine.md`, band patch, `roadmap-compound-4a21788a.patch`,
`roadmap-cumulative-4a21788a.patch`, builder + banner/rider sources).
**The commit gate stages ROADMAP.md AS-IS — do NOT re-apply the spool-side
band patch** (the band is already in-tree; the patch remains the audit copy).

Census at compound: 398 def rows in/out, zero duplicate ids, def-max rm-1522,
managed footer last; ZERO ids minted at compound; next free rm-1523 after a
fresh live census.

## Prevention rules distilled this cycle

- `docs/solutions/security-issues/a-first-char-formula-guard-is-dead-behind-a-label-join.md`
  — guard the field at composition, never the joined line; a protection claim
  needs a red-first test that fails if the guard goes dead; localize seam
  defects by row contrast.
- `docs/solutions/workflow-issues/pin-upstream-format-contracts-ahead-of-host-availability.md`
  — hand-built fixture from the published contract (provenance-pinned) +
  shape-agnostic disclosure counter + content-gated carrier + bounded by
  construction; schedule the re-capture.

## Next-cycle starters (recorded, not decided)

1. F2 regional-pricing re-verify against 4cee65a1's landing (pricing.rs churn
   permitting).
2. rm-018 rustls 0.23.46 lockfile bump post-reconcile.
3. rm-540's remaining arms: compare-lane sibling-join sweep + shared-helper
   enumeration (search.rs churn was ZERO at this cycle's census — region clean).
4. rm-1522's live-2.1.300 re-capture when the host claude-code upgrades past
   2.0.76 (capture, diff against the hand-built fixture).
5. rm-992 codex zstd rollout decode; rm-1502 ASCII-only case folding
   (sibling-named starters — fresh census before minting).
