# Cycle-1 compound record — run 66e75e39edfe (agenttrace)

Date: 2026-10-06 · Phase: compound (attempt 6b2f8f1a) · Lane: run worktree
`run-66e75e39edfe-66e75e39` @ 1511547 (run's dispatched base), 7 modified files
(batch rm-529) + this compound record (new, untracked) + this compound's ROADMAP
edits, all uncommitted, review/shipping pending (they land after this phase; the next
cycle's assessment carries them).

Prior-attempt forensics for THIS phase: attempt 93ffacc7 (provider-dead) cleared in
three commands — event log 4 lines with `message_count: 2` then `session_reap`/`failed`,
no typed result artifact, no scratch dir under /tmp — zero durable work; phase redone
from scratch and declared in the ROADMAP banner.

All outcomes below are **pre-review** evidence transcribed from the recorded phases —
NO test execution at compound, per the compound contract.

## Cycle outcome (pre-review evidence chain)

Batch: **rm-529 — "Clamp cache counts to cache-inclusive input (port upstream open PR
#316)"** (cited by TITLE; numeral double-claimed, see landscape below), selected at
prioritize (lead 78.0) over the run's second mint rm-530 (72.0, deferred — the cycle-2
lead). One change unit, one repository, one worktree (stewardship record
`/tmp/at-stewardship-66e75e39/stewardship-request.md`).

- **assess 342af0ee** — fresh adversarial re-derivation at base 1511547: baseline gates
  green (425/0 workspace all-targets, doc-tests rc0, fmt rc0, clippy -D rc0,
  docs-commands gate rc0); live PoCs: hardlink double-count (one file hardlinked across
  two projects → `Total Sessions: 2`, in 200/out 100) and fullwidth-formula CSV
  (`＝1+1` emitted bare past `guard_formula`) — both folded to existing unlanded lanes
  at roadmap, not re-minted.
- **research 0e9ad984** (redo after provider-dead 8eed7ffc, event log 14 messages, no
  artifact) — dossier `/tmp/at-research-66e75e39/research-dossier.md`; candidates 71/72
  became the mints; claim landscape moved under the run (origin 1511547 → e389f1a
  landing run 66a7d797's rm-448..rm-458 band; unlanded agenttrace bands swept on disk
  to rm-528).
- **roadmap 246f9441** — minted rm-529/rm-530 at the tail past the then-live frontier;
  delivered as patch artifact `roadmap-run-66e75e39-cycle1.patch`
  (sha256 667bb61a253d63b80ccf6586ed7458faf9bd8f374a98c6077540a1447fbc2231) with the
  worktree restored clean per that dispatch's hygiene clause; the implement phase
  carried the patch into the change-unit.
- **implement** — 7-file change-unit at base 1511547 (+124/−22): shared
  `subtract_cached_input` in parser.rs clamping each cache count to the remaining
  cache-inclusive input (`remaining` starts at `input.max(0)` so adversarial
  i64::MIN saturations clamp to 0 instead of underflowing — the fork's
  saturation-hardening family KEPT, a deliberate divergence from upstream's plain
  shape, documented in CHANGELOG); WorkBuddy arm adopts it; unit pair beside
  `workbuddy_usage_survives_negative_input_with_cache_read`; port of upstream's
  `rust_workbuddy_clamps_cache_read_above_input_without_inflating_total` +
  schema assertions in discovery_contract.rs; tui tests.rs fixture bump; session
  cache schema 24→26 (25 already taken at the landed ceiling by rm-450 — re-derived
  LIVE from origin/master, not from base); governance-reports.md ladder sentence
  realigned (check-docs-commands.sh greps the live const); CHANGELOG Fixed bullet.
  Red-first proven: parser.rs reverted alone → new contract test RED
  `left: 150, right: 100` → parked patch restored. Live PoC:
  hostile ledger (input 100 / cache_read 150 / output 20) → total 120, cache_read 100.
  Prior attempt 09e3dd17 died mid-turn but left a real 3-file delta — parked as
  `/tmp/at-impl-66e75e39/parser-fix.patch`, audited, adopted for the red-first
  roundtrip, re-pinned by the new tests.
- **targeted_tests** — gate run VERBATIM from the worktree
  (`local_validation_gate.py --shell-command 'cargo test'`) → GATE_RC=0 (workers 1,
  44.0s, no starvation escape; authoritative envelope
  `/home/agent/.hermes/local-validation-gate/results/result-1684246-374376046.json`);
  427 passed / 0 failed across 18 ok binaries; fmt rc0; clippy `-p agenttrace-core
  -p agenttrace-tui --all-targets -D warnings` rc0; digest
  `validation:v1:0ca09f019eb…` reproduced locally == dispatch digest.
- **full_tests** (attempt 2; attempt 43b07522 was rejected by the fold gate SOLELY for
  an annotated `validation_evidence.command` string — see PR-2) — gate run VERBATIM →
  GATE_RC=0 (envelope `result-3828516-…json`); 427/0 across 18 binaries; fmt rc0;
  doc-tests rc0; clippy `--workspace --all-targets -D warnings` rc0
  (`/tmp/at-full-66e75e39/attempt2/static-aux.log`); digest re-derived == dispatch
  VERBATIM; HEAD 1511547; porcelain exactly the 7 implement files.

## Roadmap accounting

- rm-529 stays `status: candidate` in its def row — **done-flip reserved for the commit
  gate after independent review** (fleet convention); the per-item `implemented`
  bullet appended at this compound carries the full pre-review evidence line,
  including the concrete conductor digest `validation:v1:0ca09f019eb888c756ff8e98926c120bbfb2067d89136e1652957f6b62522158`
  (the def row's `<sha>` placeholder is superseded by the bullet).
- rm-530 stays candidate untouched — the ready cycle-2 lead (deferral note appended).
- No new mints at compound: nothing net-new and repo-defect-shaped surfaced in
  implement/tests this cycle (the conductor-protocol lessons below are not repo
  defects), so the id frontier is not consumed.

## Integration landscape — re-swept live at compound (2026-10-06)

The landscape moved TWICE under this run and the compound banner records the current
state for the commit/review gates:

1. **origin/master: e389f1a → 8991144** (fetched this phase; 15 commits past our base,
   was 10 at roadmap mint). Landed ROADMAP def ceiling **rm-458 → rm-545** (198 def
   rows). The landed wall defines NEITHER rm-529 NOR rm-530 (grep over
   `origin/master:ROADMAP.md` empty) — our mint band now sits numerically BELOW the
   landed ceiling, exactly the "campaign-local until merge; renumber by TITLE"
   discipline the mint banner declared.
2. **The #316 clamp has NOT landed**: at 8991144 the workbuddy arm (now parser.rs:1013)
   is still the subtract-only shape with `cache_read_input_tokens` returned unclamped;
   `subtract_cached_input` appears nowhere on the wall. Our change-unit remains
   net-new vs origin.
3. **The clamp SUBJECT is triple-claimed across unlanded lanes**: this run's rm-529 +
   99d1c79c's lane (rm-541/542 candidates carry #316-adjacent claims) + 9873fc06's
   lane (rm-545/547). Only this run's claim carries green pre-review validation
   evidence and a recorded acceptance; integration merges by TITLE and folds the
   others.
4. **Numeral double-claims recorded at prioritize, re-verified live**: sibling 933058's
   unlanded rm-529 = "Saturate report-layer token accumulations (overview/variants)"
   (different content); sibling e486dc1a's unlanded rm-530 = service-tier-aware
   pricing (different content). Cite both subjects by TITLE at integration.
5. **Landed schema ladder is now 25/26/27** (rm-450 workbuddy basis-disclosure,
   rm-485 copilot credits, rm-542 codex custom-tools). Our delta's 24→26 bump —
   correct against the ceiling at implement time (25 was taken) — must REBASE to
   **27→28 at integration**, and the three places that pin the number move with it:
   the session_cache.rs const + comment, the governance-reports.md ladder sentence,
   and the schema assertions in discovery_contract.rs / agenttrace-tui tests.rs
   (check-docs-commands.sh greps the live const, so the sentence cannot lag).
6. **Adjacent landed subject**: rm-450 (upstream #310, basis disclosure) sits on the
   same workbuddy arm and holds an explicit arithmetic watch ("re-pinned once upstream
   resolves #310"). Our clamp composes with it (it only bounds counts by input) but if
   #310 pins a cache-EXCLUSIVE basis the subtraction itself changes — re-pin both then.
   Do not conflate the two rows.
7. **Id frontier**: unlanded worktree bands swept live — max rm-550 (9873fc06), then
   2d92ee95 rm-540, 4ffc4fbb rm-544, 99d1c79c rm-542 … → **next free rm-551** for any
   cycle-2 mint.

## Prevention rules (reusable, fleet-scoped)

- **PR-1 — Bump schema/version constants from the LIVE ceiling, never the dispatched
  base.** Base said 24; landed ceiling said 25 (rm-450); implement correctly bumped
  24→26 by re-fetching origin FIRST. By compound the ceiling was 27 — so even a
  correct-at-implement bump goes stale before integration. Every phase that touches a
  ladder constant re-derives it from a fresh fetch, and the docs sentence + pinning
  fixtures are part of the bump (the docs gate greps the live const).
- **PR-2 — `validation_evidence.command` is a byte-exact contract.** Full_tests
  attempt 43b07522 appended parenthetical annotations INSIDE the command string
  ("… cargo test' (validation.full_command VERBATIM …)") and the fold gate's
  exact-match rejected it — zero validation defect, one dead attempt. Auxiliaries go
  in `evidence_refs`/notes fields, never inside the command string.
- **PR-3 — Dead-attempt triage is three commands, then adopt-or-redo.** Event-log
  `message_count` + typed-artifact existence + worktree porcelain/diff census, before
  any redo: 8eed7ffc (research, 14 messages, nothing on disk → redo), 93ffacc7
  (compound, 2 messages → redo), 09e3dd17 (implement, died mid-turn but left a real
  compiling 3-file delta → PARK the delta as a patch, audit it line-by-line, use it
  as the red-first base, re-pin with new tests). A dead attempt's uncommitted trail is
  evidence, never a base.
- **PR-4 — Prove red-first with a parked patch, not a rebuild.** Revert ONLY the src
  file, run the NEW test to RED with exact assertion values (`left: 150, right: 100`
  is the citable proof the fix moves the number), restore the parked patch, then hand
  the tree to validation — validation phases never see a pre-fix tree, so their green
  is unambiguous.
- **PR-5 — The id frontier moves while you work; every minting phase re-censuses.**
  origin went 1511547→e389f1a between assess and research, and e389f1a→8991144
  between roadmap and compound (ceiling rm-458→rm-545). Census = landed wall
  (`git show origin/master:ROADMAP.md`) + ALL unlanded worktree diffs + spool sweep;
  numerals stay campaign-local and renumber BY TITLE at integration; subjects can be
  multi-claimed across lanes (the #316 clamp ×3) — the lane with the recorded
  acceptance and green validation owns the merge, the rest fold.
- **PR-6 — An OPEN upstream PR is a moving semantics source: snapshot it.** Body+files
  saved live at research (`upstream-pr316.json`) because the PR can change or merge
  out from under the port; port onto the fork's hardening family DELIBERATELY (we
  kept saturating arithmetic where upstream's plain shape would underflow on
  adversarial magnitudes — divergence documented in the CHANGELOG bullet), and pin
  upstream's own contract test by name plus the codex parity contract so a future
  upstream merge stays a reviewable diff.

## Residuals and next-cycle context

- **Cycle-2 lead: rm-530 — "Date-split alias pricing for codex-auto-review"** (72.0,
  correctness): `pricingModelAt(model, timestamp)`-equivalent at every cost site; both
  rate rows already in the bundle (pricing.rs:1222 hand-entry / snapshot gpt-5.6).
  Evidence lives under /tmp (sweeps between phases): re-fetch codeburn #1641 via
  `gh api repos/getagentseal/codeburn/pulls/1641` if
  `/tmp/at-research-66e75e39/codeburn-1641.json` is gone; the pricing.rs:106-120 basis
  read is re-derivable at any tree.
- **Copilot arm of `subtract_cached_input` waits on the #312-class basis port**
  (sibling d6432dd5's unlanded rm-509; the landed rm-485 covered credits, not basis) —
  recorded in rm-529's acceptance; adopt the shared helper there when that lands,
  do not mint a new row for it.
- **Assess findings that folded** (do not re-mint): hardlink double-count → 250cfd64's
  unlanded rm-511; fullwidth-formula CSV → the landed rm-540 sanitization lane
  (CSV statement cells route through `sanitize_line_segment` + first-non-whitespace
  formula guard — verify the fullwidth `＝` arm is covered by its hostile-corpus
  tests before assuming the fold is total).
- **Watch**: upstream PR #316 (OPEN, updated 2026-10-05T17:22Z) — if it MERGES, the
  port converges and integration diffs against upstream's shape; upstream #310 basis
  question (rm-450's arithmetic watch); v0.10.2 absent; bare `agenttrace` npm name
  still free; @zack78/agenttrace 0.10.1 still lockstep (rm-017 addendum).

## Commit-gate handoff (later phases — not this one)

- ONE commit lands the whole pre-review delta: the 7 modified files + this record
  (new untracked) + nothing else; KTD13 re-declares for the grown delta (`.md` files
  are non-executable, partial declaration stays legal).
- Schema/ladder rebase 26→28 onto the landed ceiling 27 at integration (PR-1) —
  session_cache.rs const+comment, governance-reports.md sentence, and both schema
  pinning tests move together.
- ROADMAP tail merges BY TITLE against the landed rm-449..rm-545 band (our numerals
  are undefined on the wall; renumber per house discipline if the wall claims them
  first) and against the unlanded double-claim lanes (933058 rm-529, e486dc1a rm-530,
  and the two sibling #316 lanes).
- Done-flip rm-529 candidate→implemented at the commit gate, after independent review.
- This record is a NEW untracked file — it enters the validation digest for phases
  dispatched after this turn, so later dispatch digests differ from
  `validation:v1:0ca09f01…` BY DESIGN; each later phase consumes its own dispatch
  token (fleet convention).
