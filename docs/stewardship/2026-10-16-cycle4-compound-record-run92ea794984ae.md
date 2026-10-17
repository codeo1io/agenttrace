# Cycle 4 compound record — run `92ea794984ae40f4ad8c73836efe21a6`

- **run:** 92ea794984ae40f4ad8c73836efe21a6 (worktree `run-92ea794984ae-92ea7949`)
- **requirement:** repository-maintenance `d44cb46181004f10a9bc2ca76825a9c4` cycle 4
- **base:** `c4aa3a0` (batch base); at compound the cycle content sits in salvage commit `00a87028ad60ecbd8d4f4cd10ad3f9a4bed2f382` (2026-10-16 04:39:08Z, "conductor-salvage", 18 paths +1327/−34, content-preservation only, **no validation claim**)
- **phase chain (attempt ids):** assess 7fb648a0 · research b1e8b617 · roadmap 797b8bf9 · prioritize eb812bab · stewardship 8b0ec9de · implement 7795001f (redo of provider-dead 2568b53d, zero durable delta) · targeted_tests c71b7fc7 · full_tests 7988981d · compound c29326e9 (provider-dead mid-turn **after** durable work — salvaged) → **compound 8e946adb (this record, completing the phase)**
- **status at compound handoff:** pre-review; the batch, the rebuilt ROADMAP band and the cycle docs are preserved in the salvage commit above (the terminal landing preserved the dead compound attempt's in-tree work — it is not the review, not the commit gate, and carries no validation); review/commit/CI of the landing happen after this phase.

## Outcome

Cycle 4 implemented and validated the two-row batch **"silent lies in attribution lanes"** (LEAD rm-1532 second-catalog first-party pricing + PAIR rm-1534 future-admission bound) red-first, and compounded the cycle back into the wall as the rebuilt-and-extended ROADMAP.md band (compound banner + rebuilt roadmap banner + 5 evidence-refresh riders + 4 mints rm-1532..rm-1535, +42/−0 on ROADMAP.md) plus four docs and this record. No new validation was executed at compound — every test outcome below was **consumed from the recorded targeted/full artifacts**, per the phase contract.

## 1. What the cycle shipped (18 paths, +1327/−34, preserved in 00a8702)

| Surface | Δ | Content |
|---|---|---|
| `crates/agenttrace-core/src/models_dev_snapshot.json` | new (28) | embedded first-party extract: 3 dated Anthropic rows (`claude-sonnet-4-20250514`, `claude-3-7-sonnet-20250219`, `claude-opus-4-1-20250805`) with provenance note — none of the three exists in ANY form in live LiteLLM (research b1e8b617), so a snapshot refresh could never fix the openrouter/vercel_ai_gateway misattribution |
| `crates/agenttrace-core/src/pricing.rs` | +243/−24 | `serving_catalog` seam with 4 consumers (resolve/pricing_source/provider_for/pricing_source_for_catalog); first-party rows served from the second catalog with disclosed provenance |
| `crates/agenttrace-core/src/insights.rs` | ~+69 | `session_admitted_since` gains an upper bound — far-future sessions (year-99999 class) no longer admitted into every forward `--range` |
| `crates/agenttrace-core/src/lib.rs` | ~+65 | `future_timestamp` anomaly class + `disclosures.future_timestamp_s` counter (floor-only admission was a silent-lie class) |
| `crates/agenttrace-core/src/discovery.rs` | ~+31 | `future_excluded_sessions` surfaced in discovery counts |
| `crates/agenttrace-core/src/sqlite_sessions.rs` | ~+15 | admission pre-filter superset fix (range queries and the DB lane stay consistent) |
| `crates/agenttrace-core/src/doctor.rs` | ~+70 | doctor census row for the second catalog + future-timestamp disclosure |
| `agenttrace-cli/src/main.rs`, `crates/agenttrace/src/mcp.rs` | +4/+3 | callers wired |
| `crates/agenttrace-core/tests/first_party_pricing_extract_contract.rs` | new (205) | 9 pins |
| `crates/agenttrace-core/tests/future_timestamp_bound_contract.rs` | new (271) | 8 pins |
| `crates/agenttrace/tests/warm_cache_pricing.rs`, `…/discovery_contract.rs` | +18/+7 | updated to the seam |
| docs (4) | new (54/92/95/49) | batch-selection decision record; two reliability solutions (upper-bound, second-catalog); one workflow prevention doc (target-dir headroom — see L2) |
| `ROADMAP.md` | +42 | the compound band (this record's §3) |

**Red-first proof recorded at implement:** base binary vs `fixtures/dated` → by_provider openrouter/vercel_ai_gateway; fixed binary → `by_provider [{name: anthropic, sessions: 3, cost: 43.05}]` + provenance string. Far-future: base `--range today` admits the year-99999 session (anomalies 0); fixed → parsed 1/2, 1 outside range/filters, `disclosures.future_timestamp_s`.

## 2. Validation (consumed, not re-run at compound)

- **targeted (c71b7fc7):** fmt rc0; clippy `--workspace --all-targets` 0 warnings; `first_party_pricing_extract_contract` **9/0**; `future_timestamp_bound_contract` **8/0**; `warm_cache_pricing` **3/0**; core lib **411/0**; bins **110/0**. Zero fixes needed — no executable surface changed in that phase.
- **full (7988981d):** `cargo test --workspace --no-fail-fast` → **69 suites, 1176 passed / 0 failed**, battery rc0; fmt clean; clippy 0 warnings — parity with implement's pre-dispatch 67 suites/1159 (**+2 suites, +17 tests**, the two new contract binaries). First attempt died rc=101 (`failed to parse process output`) with the private target on /var/tmp while `/` sat at 98%/2.3G free — cured by relocating to /home/agent/.cache + `TMPDIR=/tmp`, same tree rc0 (see L2 + the minted prevention doc).
- **standing reds at base, all delta-untouched:** rustls 0.23.45 lock advisories (RUSTSEC family); `check-install-runtime` test D (`^ref=v0.9.0$` vs `install.sh` REF — the rm-051 arc).

## 3. Landing provenance — reap, rebuild, salvage adoption (verified this attempt)

The roadmap phase (797b8bf9) composed its band spool-side, but the scratch AND its `/home/agent/.cache/at-roadmap-797b8` mirror were reaped before landing. Per the spool-substrate-gone precedent, compound rebuilt the band **in-tree** from the folded typed result (`results/797b8bf94b3340419afd4b55a8ecd3d4.json`) — titles/rider subjects/evidence verbatim, priorities re-derived; disclosed in the compound banner.

Compound attempt **c29326e9** then died provider-side (`session_reaped`) at ~20 min — **after** writing ROADMAP.md, the headroom doc and the banners, **before** minting this record or its envelope. An earlier compound attempt **f1b1db32** left zero durable work (lifecycle-only log, no scratch, tree unchanged) — redone. The terminal landing salvage-committed `00a8702` preserving c29326e9's in-tree work.

**Adoption verification executed by THIS attempt (8e946adb) against the salvage:**

1. c29326e9's spool `ROADMAP.postimage.md` md5 `1ad840c724b4a383aa11a68468602167` == landed in-tree ROADMAP.md **byte-identical**;
2. its `ROADMAP.base.md` md5 `caaefc742c407a9ae31191b3db28c48e` == the pristine wall recorded at implement and full_tests;
3. round-trip: `patch` of its `ROADMAP.compound.patch` onto that base reproduces `1ad840c7` exactly (7/7 hunks);
4. base→landed delta is **+42 insertions / 0 deletions**, matching the salvage stat — pure band insertion, no silent edits;
5. census on the landed wall: **407 def rows / 0 duplicate ids / def-max rm-1535 / managed footer last**; 5 evidence-refresh riders (rm-176, rm-164, rm-1053, rm-1437, rm-1402); rm-1532 + rm-1534 `minted-implemented` with EXECUTED/VALIDATED arms; rm-1533 + rm-1535 candidates.

This record is the only compound deliverable the dead attempt never minted; completing it closes the phase.

## 4. Wall drift and fog (commit-gate duties)

- **Numerals are campaign-local:** fleet frontier observed at rm-1876 by prioritize eb812bab; this band's rm-1532..rm-1535 are numeral-stale but **title-authoritative** (rm-708 convention — reconcile by title, never numeral, never renumber post-hoc).
- **Live wall:** origin/master has moved past the compose-time 5b94464 (now 2dbd749 at this writing) — hunk-by-hunk title reconciliation is owed at integration, newest-landed-first.
- **Corroboration to CREDIT at landing (not a twin):** sibling 43e5489f's independent far-future-timestamp finding; and the unbounded-read assess family stays fog-owned (rm-1871 LEAD claim) — deliberately not minted here.
- **Done-flips** for rm-1532/rm-1534 are reserved to the commit gate (rm-012 convention).
- **CHANGELOG:** the batch carries no entry — additive `### Fixed` bullets for rm-1532/rm-1534 are owed at review/commit if the changelog gate demands.
- **Stage list:** this record (untracked) — every other cycle path is already preserved in 00a8702.

## 5. Reusable lessons / prevention rules

- **L1 — dead-attempt triage, extended: durable work can outlive its attempt.** A lifecycle-only event log does NOT mean no work happened when the scratch carries a full kit AND the tree carries a delta: verify identity (md5 anchors base/postimage/landed), round-trip the patch, re-run the census, then **adopt and disclose**. Blind redo would have discarded a valid band; blind adoption would have skipped the verification legs.
- **L2 — rc=101 `failed to parse process output` is an environment signal first** (root-FS-full starving the private target dir), not a code defect: check `df` before heavy lanes; a private target must ALSO sit on a filesystem with headroom. Prevention doc minted: `docs/solutions/workflow-issues/…target-dirs-must-live-on-a-filesystem-with-headroom.md`.
- **L3 — the typed envelope is the only durable channel.** Spool scratches and cache mirrors were swept within hours; this cycle's band rebuild was possible only because the roadmap envelope carried census, titles and md5s. Phase summaries must carry rebuild-grade content.
- **L4 — upstream absence ≠ staleness.** Before "refresh the snapshot" as a cure, prove the rows ever existed upstream: live LiteLLM has zero rows (any form) for the three dated Anthropic ids — the second-catalog pattern (`docs/solutions/reliability/…upstream-catalog-omits-need-a-second-catalog.md`) is the cure class.
- **L5 — floor-only admission is a silent-lie class.** Every `since`/`after` admission needs an upper bound plus an anomaly + disclosed exclusion (`docs/solutions/reliability/admission-predicates-need-an-upper-bound-not-just-a-floor.md`); HEALTH 100 with zero anomalies on year-99999 data was the tell.

## 6. Next-cycle starters

1. **rm-1533** antigravity-numeric ids — fog-blocked this cycle (rm-1864 claimed ×2 unlanded + 477a0c5c; "no third mint"); verify which claims landed before minting; upstream semantics (#1894/#1897) still converging.
2. **rm-1535** CJK/TUI column alignment (unicode-width) — declined for a dedicated i18n/DX cycle; assess fixtures banked in 7fb648a0's scratch.
3. **rm-1532 residual arm** — extract self-retirement pin once a real rm-176 snapshot refresh lands; per-family row-survival drift report (rm-176 rider) at that landing.
4. **CHANGELOG bullets** for rm-1532/rm-1534 if the changelog gate demands them at review.
5. **Standing reds:** rustls 0.23.45 advisories (lock bump, sibling-owned); `check-install-runtime` test D (rm-051 arc, one-line fix).
6. **Fog-held:** unbounded-read family (rm-1871 LEAD) — re-adjudicate after its owner lands or dies.

---

*Record minted 2026-10-16 by compound attempt 8e946adb26e84c109e2f3b339ec68157 — the redo-of-record completing the provider-dead c29326e9 (whose durable ROADMAP/headroom work was verified and adopted per §3, not redone).*
