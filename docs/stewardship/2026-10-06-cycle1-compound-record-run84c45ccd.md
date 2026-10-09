# Cycle-1 compound record — run 84c45ccdbbeb (campaign 66430025)

- **Date:** 2026-10-06 (pre-review; review/shipping outcomes land after this record by design)
- **Base:** e1d31c4 (worktree run-84c45ccdbbeb-84c45ccd; `merge: autonomously integrate run 3c24960c` — cycle 3 of the prior campaign)
- **Batch:** one change unit, one theme — *provable pricing-catalog freshness + provenance honesty* — lead `rm-176`, riders: assess-A1 (PRICING_SNAPSHOT_DATE/_snapshot.date desync) closure, `rm-006` disclosure arm, `rm-513` three-postures fold.
- **Delta (5 files, uncommitted):** NEW `scripts/pricing/drift-check.sh` · NEW `.github/workflows/pricing-drift.yml` · M `ROADMAP.md` · M `CHANGELOG.md` (Added bullet + gate marker block) · NEW `docs/stewardship/2026-10-05-cycle1-implementation-record-run84c45ccd.md`.

## Phase / attempt trail (all pre-review)

| phase | attempt | outcome |
|---|---|---|
| assess | (cycle base) | 395/0/15 green baseline at e1d31c4; A1 gate gap confirmed |
| research | caa1350a | 10 candidates, drift-basis correction (raw-total = false alarm), three-postures map |
| roadmap | ee5960de | +31/-0: minted rm-513/514/515, 4 folds, id sweep → next free rm-513 (then) |
| prioritize | 622579db | selected the batch; collision rules vs 14 sibling lanes |
| stewardship | 076b8b03 | stewardship request + three-postures decision (KEEP bundled / REJECT runtime-fetch / REJECT hourly auto-push) |
| implement | 109d3b4f → eb4c9035 | 109d3b4f completed the work but the fold was rejected for ONE defect (missing KTD13 changed-surfaces attestation); eb4c9035 re-folded with zero file changes and the attestation fixed (5 surfaces declared, engine cross-check) |
| targeted_tests | a2d985b2 | 7-command battery rc-conformant + 1 in-scope hardening (checkout `@v4` → SHA pin `3d3c42e5…`, matching all 8 sibling workflow refs) |
| full_tests | ee44b89a → c127bdb3 | ee44b89a reaped-failed 46s in (provider family, zero durable work); c127bdb3 redone: ci.yml lane-for-lane local mirror, 22 lanes rc0 |

## Recorded validation outcomes (consumed as evidence; NOT re-run at compound)

- **Targeted (a2d985b2):** `bash -n` + `shellcheck` clean · `--selftest` rc0 (4 offline arms + keep-filter control) · `--live` vs the 12:45Z research cache rc1 **true positive**: 3,099/3,099 keep-filter parity, 11 rate-changed (openrouter/deepseek/qwen/kimi), 2 deprecation-new (azure_ai), removal_guard clear, no DESYNC · 4 red arms per the rc contract (removal→guard SET rc1 / new-costed rc1 / missing pricing.rs rc2 / forced desync rc1). Digest moved to the emission value by the SHA pin (only executable-content change post-implement).
- **Full (c127bdb3):** `cargo fmt --check` · syntax lane (bash -n / sh -n / `ruby -c`) · locked-cargo · manifests · workflow YAML · **check-plugin-version rc0 post-remediation** (pre-remediation red proven batch-independent — identical 7 missing sections at `git show HEAD`; fleet-sanctioned 19-line no-changelog-section marker block copied byte-identically, region sha256 `a7dbd255…`, from carrier run-4a68825724aa) · `cargo clippy --locked -D warnings` · **`cargo test --locked` 395/0/15, byte-consistent with the assess baseline at this base — zero regressions** · release build · entrypoints 25/0 · docs-commands 19m09s under a private `AGENTTRACE_CI_OUT` · real-cli-smoke · npm 4/0 · `cargo deny --all-features check` (corrected flag order) · shellcheck · `--selftest`.
- **Digest lineage:** dispatch `validation:v1:6e8840…` → emission `validation:v1:da358ce281c82af0194690723c3e70046182ffc5673938dcdb31affe8b8d5d0f` (targeted SHA pin), held through the full fold (marker block is non-executable).

## Wall accounting + live id re-sweep (2026-10-06)

- Landed ceiling: origin/master **1511547 → 67dfdb5a**; wall max **rm-505**, 204 id-rows.
- Canonical `/work/projects/agenttrace`: still **mid-merge** (MERGE_HEAD `e1e2c7e`, 51 staged files — the upstream v0.10.x wave: NEW changelog.yml / publish-channels.yml / .coderabbit.yaml / codecov.yml / cliff.toml, M ci.yml; `scripts/pricing/**` absent from the staged set).
- Unlanded dirty ROADMAP maxima: rm-550 (9873fc06) · 542 (99d1c79c) · 540 (2d92ee95) · 539 (cb38b958) · 532 (933058dc) · 530 (66a75e39) · 528 (db6b7626) · 512 (250cfd64, 4a688257) · 515 (this run's mint band).
- Spool clusters above 550 re-verified foreign-lineage: 3ee2775f's rm-608 is dashboard-project prose inside an agenttrace roadmap scratch that mints only through rm-423; babaf41c's rm-558 belongs to an origin/main project.
- **NEXT FREE = rm-551** (above every landed id and live dirty claim).
- No duplicate ids on this wall (`grep -oE 'id: \`rm-[0-9]+' | sort | uniq -d` empty).

## Durable lessons / prevention rules from this cycle

1. **Drift-basis rule:** any pricing-drift comparison must run on the builder keep-filter basis (`mode=="chat" AND input-or-output cost > 0`) and only on fields the bundle pins. Raw-total comparisons false-alarm (4,473 raw vs 3,099 kept); comparing builder-dropped fields (max_output_tokens) produced 2,802 phantom diffs. Encoded in `drift-check.sh`'s header and rm-176's evidence line.
2. **Mutable-ref rule:** every new workflow must pin `uses:` to a full 40-hex SHA (house convention; `check-example-workflows.sh` enforces it only for the example file). This cycle's targeted pass caught `checkout@v4` in the new workflow and pinned it; the enforcement GAP (non-example workflows ungated) is recorded below as a cycle-2 rider.
3. **Fold-attestation rule (fleet):** succeeded implement folds need `validation_evidence.changed_surfaces` declaring every changed surface (KTD13). Cost this run one rejected attempt (109d3b4f → eb4c9035 re-fold). Fleet memory carries the rule; recorded here for the repo-side trail.
4. **Tag-namespace hazard:** check-plugin-version's per-tag arm fails on tag-complete environments until the 7 inherited upstream tags carry CHANGELOG markers; the marker block is the fleet-sanctioned remedy and is now in this tree (see seam 1).

## Commit-gate seams

1. **CHANGELOG marker block dedupe** — byte-identical to sibling carrier 4a688257's block (region sha256 `a7dbd255…`). At integrate, dedupe by content; never append twice.
2. **Upstream wave merge** — the canonical's mid-merge brings upstream pricing.rs (runtime-fetch, no bundle, no scripts/pricing/). Apply the Rider-B checklist (`delegate/109d3b4f-rider-b-posture.md`): reject upstream pricing.rs wholesale, keep `scripts/pricing/`, re-run `drift-check.sh --selftest` post-merge. Upstream session-cache schema 26 vs fork 22 also rides this wave (rm-292-class stranding risk).
3. **ROADMAP delta** — this note + the rm-513/514/515 mint band + the rm-176 implemented flip; the done-flip stays reserved for the commit gate (rm-012 precedent).
4. **No new supply-chain decisions** — pricing-drift.yml's checkout pin equals ci.yml's existing pin.

## Cycle-2 leads (concrete)

- **Snapshot regeneration** — the gate's true positive (11 rate-changed + 2 deprecations vs the 2026-10-04 bundle) is queued for the scheduler's first post-merge run by design; an uncontested cycle may regenerate deliberately (`update-snapshot.sh` + const bump) and close rm-176's "first automated green run" evidence leg.
- **models.dev second source** — per rm-513's posture table; fills cache_read/cache_write + limit.context/limit.output gaps; LiteLLM stays primary.
- **CI glob rider** — extend ci.yml "Validate helper scripts" (`bash -n scripts/record-demo.sh scripts/ci/*.sh`) to `scripts/**/*.sh` so `scripts/pricing/` is syntax-gated in CI.
- **rm-006 --overview arm** — the item's sole remaining scope.
- **Upstream 27-commit wave merge** — rm-517, claimed by sibling 95e25b56; reconcile by title.

## Watch

- Upstream luoyuctl: 4 releases in 5 days (v0.9.1/v0.10.0/v0.10.1) + post-release #313/#314/#315; pushed 2026-10-05T12:16Z.
- OTel GenAI semconv (rm-229).
- npm identity window: bare `agenttrace` unclaimed at research time.

## Review fix (2026-10-06, independent_review 6d90c387 → independent_review:fix 825ca17f)

Review verdict: **NEEDS_CHANGES (2 medium + 3 low)** — every recorded claim otherwise
reproduced first-hand, the batch core sound. All five findings fixed and revalidated this
attempt; prior fix attempt 0aea3f9a was reaped at turn start (infra, zero durable work —
verified against its event log before redoing).

| # | severity | finding (pre-fix file:line) | fix (post-fix anchor) | proof |
|---|---|---|---|---|
| F1 | medium | desync gate fails OPEN on absent inputs — pricing.rs without the token or bundle without `_snapshot.date` → green rc 0 with `<none>` (drift-check.sh:120 + `\|\| true` at :162) | absence is rc 2 INFRA, never "clean": bash guard at drift-check.sh:197 (const declaration missing), python guard at :112 (`_snapshot.date` missing); `desync` simplified at :130 with both sides proven present | red arms standalone: missing const → rc 2, missing meta → rc 2; selftest arm 5 pins both |
| F2 | medium | duplicated CHANGELOG content line `for public demo and install surfaces. (#115, #121)` ×2 (CHANGELOG.md:302-303), introduced by the marker-block append | one copy dropped (now :302, single occurrence on both sides of the diff) | `check-plugin-version.sh` rc 0; CHANGELOG diff vs HEAD = pure +19/−0 marker append |
| F3 | low | `const_date_of` first-match anchoring poisonable — dated doc comment above the const → false DESYNC rc 1 (drift-check.sh:161-162) | read anchored on the DECLARATION `const PRICING_SNAPSHOT_DATE: &str = "<date>"` (drift-check.sh:171), same shape the workflow's const-bump sed targets | red arm C: poison fixture reads 2026-10-04, no DESYNC; selftest arm 6 (parity fixtures → rc 0); real pricing.rs reads 2026-10-04 in every live run |
| F4 | low | regen fetches live then verification fetches live AGAIN — TOCTOU: LiteLLM main moving mid-job fails the refreshed pair's own gate (pricing-drift.yml:85 vs :96) | ONE fetch per job (curl at :49) shared by gate (`--live pricing-live.json`), regen (`update-snapshot.sh pricing-live.json` — new optional pre-fetched-catalog arg, default fetch unchanged, update-snapshot.sh:19), and verify (:106) | whole-workflow grep: exactly one `curl`; sandbox simulation: regen byte-deterministic (3101 @ 2026-10-06), post-bump verify **rc 0 on the same fetch** |
| F5 | low | PR branch `chore/pricing-snapshot-$(date -u +%F)` — same-day cron+dispatch or re-run collides non-fast-forward (pricing-drift.yml:81) | branch carries `-${GITHUB_RUN_ID}.${GITHUB_RUN_ATTEMPT}` (pricing-drift.yml:89) | structural assert on the YAML; unique per run attempt by construction |

**Revalidation (targeted scope, dispatch-authoritative):** `bash -n` + `shellcheck` clean on
both scripts · `--selftest` rc 0, six arms + keep-filter control · the review's three red arms
standalone, all now behaving per contract · live gate vs fresh 2026-10-06 fetch (3,051,942 B,
4,480 entries): rc 1 true positive identical in shape to the review's own fresh-data run —
3099 bundled @2026-10-04 / 3101 live, new_costed 2 (`global/us.zai.glm-5.3`), rate_changed 11
(same openrouter deepseek/qwen/kimi/llama families), deprecation_new 2 (azure_ai), retired 0,
guard clear, no DESYNC, const read 2026-10-04 from the real pricing.rs via the new anchor ·
update-snapshot.sh override mode byte-deterministic across runs, missing-input rc 2 · YAML
parse + 9 structural assertions (single fetch, shared `--live`, run-id branch, SHA pin, no
`@v4`) · check-plugin-version rc 0.

**Digest lineage update:** dispatch `6e8840…` → targeted-pin emission `da358ce2…` (held through
review) → **review-fix emission `validation:v1:68faba1d0cbfaf375287d715d43089405a4ed1b6caf7b5c46abff71e48340cdf`**
(executable surfaces changed this turn: drift-check.sh, update-snapshot.sh — now a changed
testable surface per the engine's own derivation — and pricing-drift.yml). The sha the
shipping PR records per rm-176's acceptance line is now the 68faba1d one.

**Delta is now 7 files** (was 5 at implement, 6 after the marker block):
`scripts/pricing/drift-check.sh` · `.github/workflows/pricing-drift.yml` ·
`scripts/pricing/update-snapshot.sh` (M — optional-arg only) · `CHANGELOG.md` · `ROADMAP.md` ·
two `docs/stewardship/` records (implementation record carries a review-fix addendum).

## Review fix 2 (2026-10-06, independent_review cac54820 NEEDS_CHANGES 1M+3L → a53b451b)

Second adversarial re-review graded the post-825ca17f batch NEEDS_CHANGES on four findings;
all four were verified first-hand this attempt (no file had changed since — worktree mtimes
pre-date the review), fixed, and revalidated at targeted scope.

| # | severity | finding (pre-fix file:line) | fix (post-fix anchor) | proof |
|---|---|---|---|---|
| F1 | medium | pricing-drift.yml `gh pr create --label "pricing"` — the repo carries no `pricing` label (live label list = GitHub's 11 defaults), and gh resolves label names BEFORE creating the PR, so the first bump run fails PR creation deterministically | label flag dropped with an in-file rationale comment (labeling would also need the label created on the repo first + an `issues:write` grant) | `gh api repos/codeo1io/agenttrace/labels` re-verified live; YAML assert: no `--label` string anywhere in the file; the residual PR-create failure path is the documented permissions warning |
| F2 | low | `--json` stdout was summary prose + JSON — unparseable by `json.tool` (fails on line 1), breaking the documented machine-report contract (drift-check.sh analyze) | summary routes to stderr when `--json` (`out = sys.stderr if emit_json else sys.stdout`); stdout is now PURE JSON; no-`--json` runs keep the summary on stdout (the workflow's regen counts-grep depends on that) | `--json \| python3 -m json.tool` rc 0, first stdout byte `{`; stderr carries the summary; no-json stdout `grep -c 'costed-chat models'` → 2 |
| F3 | low | malformed catalog SHAPES crashed with tracebacks at rc 1 (list-typed `input_cost_per_token` → TypeError in `kept()`; `_snapshot` as a string → AttributeError) — a parse failure mislabeled as actionable drift, steering the workflow into bump mode with an empty report | the analysis computation is wrapped fail-closed: any malformed shape → `infra error: malformed catalog content` on stderr, rc 2, EMPTY stdout; selftest arm 7 pins both sub-shapes | red arms: badtype rc 2 / stdout 0 B; badmeta rc 2 / stdout 0 B; `--selftest` arm 7 PASS (now seven arms) |
| F4 | low | CHANGELOG "four offline selftest arms" stale (six existed by the prior fix round) | corrected to "seven" (post-arm-7 truth) | grep; `check-plugin-version.sh` rc 0 |

**Revalidation (targeted scope):** `bash -n` + `shellcheck` clean on both scripts · `--selftest`
rc 0, seven arms + keep-filter control · prior-round red arms still per contract (absent const
rc 2, absent `_snapshot.date` rc 2, dated-comment poison reads 2026-10-04) · live gate vs a
fresh 09:14Z fetch (3,053,287 B / 4,480 entries): rc 1 true positive identical in shape to the
review's own fresh-data run — 3099 bundled @2026-10-04 / 3101 live, new_costed 2
(`global/us.zai.glm-5.3`), rate_changed 11 (openrouter deepseek/qwen/kimi/llama),
deprecation_new 2 (azure_ai), retired 0, guard clear, no DESYNC, const 2026-10-04 · `--json`
output parses · the workflow's regen loop simulated end-to-end in a sandbox on that single
fetch: regen writes 3101 stamped 2026-10-06 → sed const bump → post-regen verify rc 0,
counts-grep fed, byte-deterministic regen (bundle sha 8a0f9a67…) · YAML structural asserts
(3 steps, SHA-pinned checkout, no label flag, exactly 1 curl, dated+run-id branch, shared
`--live` across gate and verify, gate greps match the pure-JSON report) · check-plugin-version
rc 0.

**Digest lineage update:** review-fix emission `68faba1d…` → **review-fix-2 emission
`validation:v1:395fef176f5805c15e5824b128c2e0ed08ca448bd9ebe3eb4679deca4b2b159c`** (executable
surfaces changed this turn: drift-check.sh, pricing-drift.yml; CHANGELOG/ROADMAP/records are
non-executable). The sha the shipping PR records per rm-176's acceptance line is now the
395fef17 one. Delta stays 7 files (no new paths).
