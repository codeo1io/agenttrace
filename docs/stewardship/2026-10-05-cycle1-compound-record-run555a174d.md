# Cycle-1 compound record — run 555a174d (agenttrace)

Compound fold of repository-maintenance cycle 1 (repository-maintenance:0937d5fb, worktree
run-555a174d07b2, base at dispatch 7e17ac1, tree re-anchored to ceiling ea5c41e during
implement). Batch: **rm-408** (disclose present-but-zero usage blocks) + **rm-409**
(-f csv statement export); rm-410 deferred. Written at compound attempt f3fcafd5
(2026-10-05), after the provider-dead attempt d6d6d4b1 was forensically cleared
(event log = status pings + `session_reap_failed` at +311s, no typed artifact, no scratch
dir — zero durable work; phase redone from scratch and declared in the ROADMAP banner).

All outcomes below are **pre-review** evidence transcribed from the recorded phases —
NO test execution at compound, per the compound contract. Review and shipping outcomes
happen after this phase; the next cycle's assessment carries them.

## Cycle outcome (pre-review evidence chain)

- assess c65d9cfb (fresh adversarial re-derivation at 7e17ac1 after dead-transport
  8e1a3a6e) → A1 markdown-HTML injection HIGH (folded to rm-389 with live paths), A2
  NaN/Inf view-filters + CLI-vs-TUI dialect divergence (folded to rm-195), A3 health-gate
  example takeover recipe, A4 SECURITY.md rider.
- research a9037c3c (repo-grounded ce-ideate in-thread; subagent dispatch unavailable) →
  N1 present-but-zero usage (= rm-408's source), N2 no CSV export (= rm-409), N3 TUI
  keybindings (= rm-410).
- roadmap e2292821 → minted rm-408..410 past a three-home census; deliverable was the
  uncommitted ` M ROADMAP.md` delta.
- prioritize 907a8193 (ce-plan) → selected rm-408 (LEAD, 62.0) + rm-409 (55.0); rm-410
  (40.0) excluded.
- stewardship 637b65764 → two change units, one repo, one worktree: U1 core
  (rm-408), U2 cli (rm-409); five must_remain_separate pairs.
- implement 349ab712 → **rejected by the fold gate for a missing KTD13
  changed-surfaces attestation only — zero code defect**; re-attested as 562afd0a with
  tree identity asserted, no code redone.
- targeted_tests 1b450631 → zero_usage_contract 3/0, csv_export 4/0, core lib 158/0,
  cli bin 34/0, fmt + clippy `-D warnings` + deterministic + docs-commands +
  plugin-version gates all rc0.
- full_tests f093b7e5 → ci.yml `full`+`deny` mirror (validation.full_command was empty;
  fleet precedent 71a7d5db / a0407d88): 20/20 runnable steps rc0, 380 passed / 0 failed
  across 15 ok binaries; ci.yml verified byte-identical since the sibling extraction.

## Prevention rules (reusable, fleet-scoped)

- **PR-1 — Emit the KTD13 attestation on EVERY implement fold.** This cycle's only
  rejected fold (349ab712) was rejected for a missing `validation_evidence.changed_surfaces`
  declaration with zero code defect; the re-attestation turn verified tree health and
  re-declared instead of redoing work. Declare repo-relative paths actually changed,
  against the DISPATCHED base (not current HEAD).
- **PR-2 — Dead-attempt forensics before redo.** A provider-dead attempt leaves no typed
  artifact; its event log (status pings + `session_reap_failed`) plus a porcelain/diff
  census of the worktree is the evidence. This compound's d6d6d4b1 check took three
  commands and prevented a phantom redo. An absent envelope is not evidence no work
  happened — check the tree; a present artifact is not proof it is valid — verify.
- **PR-3 — Read the gate script before diagnosing the gate.** The `>5 min` docs-commands
  "hang" in targeted was root-caused by reading scripts/ci/check-docs-commands.sh in
  full: the `--doctor` leg runs without `-d`, so it walks the operator's real HOME.
  Workaround (isolated HOME/XDG scratch + `timeout 240`); durable fix minted as **rm-444**
  (def-row sweep at mint in the ROADMAP provenance comment; next free rm-445).
- **PR-4 — Gate flake anatomy: reproduce before believing.** The full-suite S03 first
  read as flaky; 3x exact-command reruns (380/0 each), 5x standalone discovery_contract
  (78/0 each), and a single-test `--exact` probe showed green, and the fixture audit
  explained why: discovery_contract's SAMPLE_JSONL has no all-zero usage, so the new
  optional cache field stays absent via `skip_serializing_if` — optional-field additions
  do not move cache-byte tests on corpora that never exercise them. AMENDED at review
  fix 2bff960d: the writes-side reading was half the story. On READS the same
  `#[serde(default)]` lets a warm pre-field v22 cache serve the counter as 0 — clean
  zeros — which is why the fix bumped SESSION_CACHE_SCHEMA_VERSION 22 → 23 (rm-230
  convention, review 342a1349 F1) instead of trusting the default; pinned red-first by
  `stale_schema_22_cache_cannot_mask_the_disclosure`.
- **PR-5 — Empty `validation.full_command` → mirror ci.yml, but re-verify the mirror's
  basis.** The a0407d88 mirror-ci.sh extraction was reused only after confirming
  `git diff --stat b0e12d44..HEAD -- .github/workflows/ci.yml` is empty (byte-identical
  step list). A moved CI file silently invalidates a reused mirror.
- **PR-6 — The CLI crate's package name is `agenttrace`, not `agenttrace-cli`.**
  Test invocations must be `-p agenttrace` (the implement record self-corrected this);
  sibling fleets have hit the same transcription slip (41263f58 review 0145eeb5 finding 3).
- **PR-7 — ROADMAP deltas are base-anchored; verify content, not patch bytes.** When the
  worktree base moved mid-run (7e17ac1 → ea5c41e), the same roadmap delta produced a
  different `git diff` (+24/−0 became +22/−0, different blob ids and hash). Byte-compare
  the file against the phase's postimage artifact (ROADMAP.new.md), or re-anchor before
  hashing, before declaring drift.

## Residuals and next-cycle context

- **rm-410** (TUI keybindings, 40.0) is the ready cycle-2 lead — deferred at stewardship,
  anchors re-verified at ea5c41e.
- **rm-444** minted at compound (docs-gate doctor leg HOME scan) — small, script-only,
  pairs with landed rm-369.
- Assess A1/A2 live paths are pinned as addenda on rm-389 / rm-195; their IMPLEMENTATION
  remains open — A1 is HIGH severity and unclaimed by this cycle's batch.
- Research watch items, unchanged: upstream luoyuctl zero movement (3rd consecutive
  check, 52ab2cd8 / v0.9.0), the npm `agenttrace` identity window (REPO pin riders),
  OTel genai semconv motion (b9ecbaef entity, #558/#559).
- Fleet numbering at compound (live re-sweep, ceiling ea5c41e unmoved): uncommitted
  agenttrace lanes rm-403..440 on disk + e97ae6c9's recorded rm-441..443; this run's
  rm-408..410 band uncontested (zero ceiling defs, zero sibling def rows); next free
  rm-445 after the rm-444 mint. Ids campaign-local until merge — renumber by TITLE at
  integration.

## Evidence index

- Implement fold: spool 562afd0ac27d4cbfab4d8d64cc43112c (scratch tree-state.txt + t1..t10
  logs); rejected prior 349ab712 (KTD13 attestation-only).
- Targeted: spool 1b45063134da4c38919f28de4456aa13 (t1..t8 logs, RC markers inline).
- Full: spool f093b7e544b743c1aa9290b90dfbfba2 + /tmp/at-full-f093b7e5/logs/ (S01..S20).
- Prioritize/stewardship inputs: spool 907a819385574899815f5251775cf57a,
  637b65764d3c42caad8bb79e501fadf0.
- Dead compound attempt forensics: events/d6d6d4b1664e4e2f98cff6aa63a69235.jsonl;
  absent delegate/d6d6d4b1664e4e2f98cff6aa63a69235.json; ROADMAP.md untouched (mtime
  predates the attempt; porcelain unchanged).
