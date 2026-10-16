# Decision: cycle-4 maintenance batch — zstd rollout truth end-to-end + signal truth

- **Status:** implemented + validated (pre-review, uncommitted); record minted at compound
- **Run:** 1e4f6b95c08d430882982f8c32718db2 · repository-maintenance bae598722582490d89cd79109fb6a692 cycle 4
- **Base:** 16dcc9a044d891b0a599848ea01b40c4d65a0c4b (== origin/master at dispatch)
- **Selection artifact:** prioritize aa16a7d51d5244f59366c54c1fee5202 · stewardship 4a6a13619716423bab6e5c85b6ac99fc
- **Record minted by:** compound 7490ba2153394e8186abd7f5508bdd60 (2026-10-16)

## Decision

Implement the 4-item batch "zstd rollout truth end-to-end + signal truth" —
rm-1516 (LEAD, per-frame zstd decode to true EOF), rm-1518 (user-facing zstd
docs + CHANGELOG truth), the rm-012 install-ref gate fix (rider, not a mint),
and rm-1519 (gemini perf-ratio arm re-stabilization) — deferring the six
catalogued alternatives (see Prioritized alternatives below).

## Method

Selected from this run's own upstream artifacts (assess 07edad83 F1–F9,
research 7f41f39d RC1–RC10, roadmap band 532349b1 mints rm-1516..rm-1519),
each item re-verified at base 16dcc9a and cross-checked against live
origin/master before admission. Selection bias: reliability-first (rm-1516 is
a silent half-journal billing error on the format our own error guidance
points users at), then truth-of-record (rm-1518, rm-012 rider), then
signal-truth of the test battery itself (rm-1519).

## Outcome (pre-review; validation consumed at compound, not re-run)

| Item | Change | Validation |
| --- | --- | --- |
| rm-1516 LEAD | `decode_zstd_rollout_bounded` recreates the ruzstd decoder per frame to true EOF; 64 MiB cap bounds the TOTAL across frames; trailing bytes refused with offset; skippable frames skipped; corrupt frames name frame+offset; +4 tests, +2 fixtures + fixture README | red-first 19/38 events pre-fix; `zstd -dc \| cmp -` parity byte-identical; CLI --overview identical via path and stdin; zstd_rollout_decode 7/0; full 65 suites 1147/0 |
| rm-1518 | README "### Compressed (zstd) Codex rollouts" (:338+); CHANGELOG rm-992 entry corrected (at research time no 0.163 stable; 0.163.0 went stable 2026-10-15 with the write flag still off — re-synced at review fix; reader tolerance since ≥0.152) + dated entries | check-docs-commands RC=0; full battery green |
| rm-012 rider | check-install-runtime.sh derives the pinned ref from install.sh (AGENTTRACE_SOURCE_REF extraction); test-D receipt + test-E pins assert the derived value | RC=0 "derived pinned default ref: v0.10.1"; ref-drift RC=0; the only standing gate red is dead |
| rm-1519 | gemini ratio arm: large side min-of-three, floor 40→60 from 6 measured probe runs at fleet load 50–58 (healthy 7.5–24.9x) | gemini_chats_contract 9/0; discovery_contract 104/0; full 65 suites 1147/0 |

Batch tree: 6 M + 3 ?? uncommitted in worktree run-1e4f6b95c08d-1e4f6b95;
tracked diff md5 29af8b2d0c168a6eb6cdbc1d51baad4c. Full validation reproduced
the ci.yml job matrix job-by-job (65 suites 1147/0; clippy 0 warnings; msrv
+1.89.0 check/fmt green with the 3 known version-lints pristine; conformance
13 cases asserted, 1 xfail-pinned; actionlint exactly the one standing SC2001
at ci.yml:192, ci.yml byte-identical to base).

## Reusable lessons minted this cycle

- `docs/solutions/reliability/decoder-contract-must-match-the-writers-own-reader.md` (L1)
- `docs/solutions/ci-hygiene/gate-pins-must-derive-from-the-source-of-truth.md` (L2)
- Measured-floor rule for self-calibrating perf asserts on shared fleet hosts (L3)
- KTD13 recovery re-verified: prior implement attempt cd63bb62 folded without
  `validation_evidence.changed_surfaces`; tree verified byte-intact, work
  adopted, only the attestation re-emitted — never redo verified work.
- Swept-evidence recovery: quarantine/scratches/&lt;ts&gt;/&lt;attempt-id&gt;/
  + work-order-embedded records, verified byte-exact against recorded md5s
  (band round-trip re-proven: 1f1ab384 → 33c255d3 → 902106d7).

## Standing reds at this base (remote CI; all pristine, out-of-batch)

1. actionlint SC2001 ci.yml:192 (cosmetic echo|grep; ci.yml byte-identical base==worktree)
2. msrv +1.89.0 clippy: 3 version-lints (parser.rs:7405/:7434, pricing.rs:345) — the 1.98.1 toolchain does not flag them
3. cargo-deny: rustls 0.23.45 RUSTSEC-2026-0346..0349 — cure 0.23.46 published 2026-10-14; sibling PR #22 owns the bump

## Next-cycle leads (recorded, not decided)

1. **rm-451** (designated lead): file-size admission bound for the bare-path
   parse + doctor skip counter + RSS corpus pre/post (assess-F2 residual:
   stdin 256 MiB / zstd 64 MiB / bare file unbounded).
2. Watches: rm-1517 paginated migration (trigger = codex flag default flip or
   first stable noting it); rm-421 compressed-by-default re-sync when 0.163
   ships stable; rm-715 trigger re-arm to "first stable ≥ 0.65".
3. Slack: rm-1502 ASCII case-fold decision; rm-992 CSV churn leg; the SC2001
   cosmetic rewrite.

## Prioritized alternatives (deferred with rationale, from the selection pass)

- **rustls 0.23.46 repin (rm-018)** — plain `cargo update -p rustls`, but PR
  #22 already open upstream; merging never re-implements. ACT ONLY IF STALLED.
- **rm-1502 ASCII case-fold** — needs a decision (fold engine choice), not a
  mechanical fix; slack this cycle.
- **rm-992 CSV churn leg** — drift, not breakage; slack.
- **rm-1517 paginated rollout** — upstream flag still
  `UnderDevelopment/default_enabled:false`; no actionable seam on a stable
  channel yet.
- **SC2001 cosmetic rewrite** — byte-identical to base; the standing red is
  upstream-of-batch, not batch-owned.
- **rm-1517's admission arm** (RSS/corpus work) — SAME WORK as rm-451's
  admission arm; rm-451 owns it.

## Provenance note

Four phases of this run survived provider-transport deaths of prior attempts
(roadmap e137734d, prioritize 7441827f, stewardship 37b6e75f, compound
d93f4e4d — each a 4-event reap log, nothing durable, each redone from scratch
and declared). The spool reaper later swept the assess/research/roadmap/
prioritize/stewardship envelopes + scratch dirs; their records live on in the
run's work-order trail and this cycle's ROADMAP riders, and the roadmap band
was recovered byte-exact from quarantine (round-trip md5-proven at compound).

## Review fix (2026-10-16, independent_review 2d3b7ff3 -> independent_review:fix e41fec74)

Independent adversarial review returned NEEDS_CHANGES with four findings;
dispositions (evidence in delegate/e41fec74…-scratch/):

1. (medium) "no 0.163 stable has shipped" went false between the research
   snapshot (2026-10-15) and review: codex rust-v0.163.0 shipped stable
   2026-10-15T21:30:38Z. FIXED — the CHANGELOG (rm-992 + rm-1516 entries),
   README :341, and the rm-1518 row above now time-stamp the claim and
   record that the write flag is STILL off at 0.163.0 (verified at tag:
   Stage::UnderDevelopment, default_enabled:false). The rm-421 watch
   trigger ("re-sync docs when 0.163 ships stable") has fired; a dated
   review rider lands on the wall (append-only, spool-side patch — the
   minted banners are never rewritten).
2. (medium) the rm-012 install-ref fix duplicates work ALREADY LANDED on
   master by run a64735b75a46 (2026-10-14, ANCHORED sed extraction; its
   comment also cites parallel sibling 966bd5f9). FIXED by title-fold:
   scripts/ci/check-install-runtime.sh now carries master's bytes verbatim
   (md5-identical to origin/master — the anchored variant, not the weaker
   first-match-anywhere grep -oE this batch first wrote), and this batch's
   duplicate CHANGELOG entry was REMOVED (one landing, one entry). The
   commit-gate fog-list correction rides the wall rider.
3. (low) parser.rs `8 + length as usize` could wrap on 32-bit release
   targets before the truncation bail ran. FIXED — the guard now compares
   in body-space: `length as usize > rest.len().saturating_sub(8)`.
4. (low) skippable-frame coverage is tail-only (no between-frames case,
   no truncated-skippable-header test). DEFERRED by the reviewer's own
   text ("add in a later pass") — both paths verified correct by
   inspection at review; recorded as next-cycle work.
