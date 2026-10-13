# Cycle 3 compound record — run 45d3f3ed3e434249b784edfb411a24df

repository-maintenance 0bcd596a · cycle 3 · base 3e90993 (== origin/master at cycle start) · 2026-10-14
Compound pass: attempt fd610df5 · this document + ROADMAP compound band (spool chain, lands at the commit gate)

## What this cycle shipped

**OTel conformance lane** (prioritize fb7744d9 → stewardship 8aff8675 → implement f7ba5407 → targeted 43a8b6da → full_tests 36d713cc):

- **rm-546 → implemented.** The OTLP-JSON export's attribute vocabulary is now pinned to an exact upstream tree: `SEMCONV_SNAPSHOT_COMMIT = 07794021` + `SEMCONV_SNAPSHOT_DATE = 2026-10-14` (committer date). semconv-genai publishes no releases/tags — a date alone cannot identify a tree, so the pin is commit-anchored and both constants move together, golden-pinned. The commit id rides every export as `agenttrace.export.semconv_snapshot_commit` next to the existing date attr. New golden test `emitted_attribute_vocabulary_is_pinned_rm546` pins the EXACT sorted attribute-key set per span shape (16-key core + `gen_ai.usage.reasoning.output_tokens` as the lone conditional), resource attrs, and span-name truth (`session <name>`, `gen_ai.system` carries the tool). Red-first proven by mutation: a rogue `gen_ai.request.provider` emission fails with an exact-set diff; reverted; 9/9 green. `docs/otel-attributes.md` documents every emitted key → semconv standing, the recorded divergence (delta input basis vs the registry's cache-inclusive SHOULD, disclosed per-span), and non-goals (modality buckets, histograms, `gen_ai.client.inference.duration` adopted at naming policy only).
- **rm-983 → implemented.** The row's upstream citation was WRONG on the wall ('#312' resolves to 2025 data-loader PRs in BOTH ccusage repos). The real delta set is **ccusage/ccusage#1858** (merged 2026-10-10). Named decisions: `inputTokens`-cache-subtraction → fork-supersedes (fork has emitted the delta basis with a per-span disclosure attr since inception); `reasoningTokens`-subset → fork-supersedes (rm-863 lineage); count-OTel-calls-missing-from-shutdowns → **declined this cycle** (occupied parser seam — sibling rm-992 zstd lane) → recorded as next-cycle candidate subject. Copilot chat-span usage-summation truth (checkpoints SUM per model; shutdown REPLACEs) pinned on the wire: 1_050_000 in / 105_000 out across `model-rotation.jsonl`. rm-485 credit pins zero-regression (9/9).

Code delta in worktree: `CHANGELOG.md`, `crates/agenttrace-core/src/lib.rs` (re-export), `crates/agenttrace-core/src/otel.rs`, `crates/agenttrace-core/tests/otel_export.rs` (4 M) + `docs/otel-attributes.md` (new), +263/−10, left uncommitted for the commit gate.

## Validation record (pre-review evidence, consumed — not re-run here)

- targeted 43a8b6da: tree proven byte-identical to implement finish, then core lib 399/0, otel_export 9/9, copilot_checkpoint_reconciliation 9/9, fmt clean, clippy `-D warnings` clean, changelog + docs-commands gates rc0. Digest `validation:v1:694c7e1f…` declared verbatim (nothing executable changed that turn).
- full_tests 36d713cc (redo of provider-dead af7a8955): ci.yml lanes verbatim per lane — core 49 suites 824/0 (scoped HOME; one fleet-flake `disclosure_case_fold::single_line_renders_bound` failed once under load 33, all reruns green, forensically disclosed), tui 2/60, cli 11/236, fmt+clippy rc0 (after stale-rlib `cargo clean -p` repair), msrv `+1.89` rc0, release build rc0 with `strings`-grep confirming the 07794021 constant in the shipped binary.

## Roadmap chain (compound band)

Wall ROADMAP.md md5 `1d59adda…` (392 rows, def-max rm-1373) at base 3e90993.

1. **Regeneration disclosure**: the roadmap phase's spool scratch AND its /tmp mirror were reaped before this compound pass. The band (1 banner + 7 evidence riders, zero mints, zero flips, zero modified lines) was regenerated from the recovered patch text; content and placement verified rider-by-rider against each row's evidence section. The roadmap phase's recorded postimage figures (md5 `f73f62bc…`, 2549 lines) do NOT reconcile with the 2538-line wall + 23 insertions = 2561, so those figures are treated as transcription drift in the reaped phase's summary, not ground truth; the authoritative artifacts for the commit gate are the ones below, roundtrip-proven.
2. **Compound band** (this pass): 1 compound banner line-block + **2 flips** (rm-546, rm-983: candidate → implemented, each with a dated VALIDATED rider citing implement/targeted/full evidence) + **zero mints**. Next free after this cycle: **rm-1510** (fresh fleet census: def-row ceiling rm-1509 = run-5c144613 dirty wall + b3f93f10 postimage; mention-level fog reaches further but is not def-rows).

Chain artifacts (mirrors in spool scratch + /tmp/at-compound-fd610df5/): `roadmap.regen.patch` (wall → intermediate, 49 lines), `ROADMAP.compound.patch` (intermediate → final, 37 lines), `ROADMAP.final.md` (md5 `262b6281…`), `intermediate.md` (md5 `bea7d659…`). Roundtrip proven byte-exact via `git apply` in a throwaway repo at both stages (STAGE1-BYTE-EXACT / STAGE2-BYTE-EXACT).

## Reusable lessons (prevention rules)

1. **Verify upstream citations live before implementing** — the wall's '#312' was a 2025 data-loader PR; the real deltas were #1858. Any roadmap row citing an upstream number gets a live resolve at implement time, with the correction recorded back onto the row.
2. **Commit-pin tagless upstreams** — semconv-genai has no releases; `SEMCONV_SNAPSHOT_COMMIT` (+ the commit riding the export) is the citable tree. Dates drift silently; commits don't.
3. **Zero-context patch hunks relocate** — `git apply` on `-U0` pure-insertion hunks applied them at EOF (offset +2532) instead of their anchors. Spool chain patches must carry context (`-U1` minimum); the fleet's own `-F3` convention exists for exactly this. Verify every chain stage with `cmp`/md5, never by exit code alone.
4. **Cite scoped flake forensics** — a single >60s-stretched test failure under box load 33 was disclosed with rerun matrix (solo/binary/full), not silently retried.
5. **Fog census by def-rows, not mentions** — mention-level `rm-XXXX` greps over the spool overstate the ceiling (narrative references to cycle-2's rm-1509s etc.); only def-rows in live postimages/dirty walls bind numerals.
6. **Rust ergonomics** — closure returning borrowed `Vec<&str>` failed lifetime inference; converting to a nested `fn` inside the test fixed it without restructure.

## Next-cycle candidates (context handoff)

- **Copilot ingest-truth arm** (declined from rm-983): count OTel calls missing from shutdowns + dedupe re-exports — parser seam occupied this cycle by rm-992 (zstd); natural pairing once that lands.
- **rm-933 lineage TUI sanitization** slices are in flight fleet-wide (slices 2–3 observed); rm-014 dual-renderer remains the strategic follow-up but must re-census the seam first.
- **`gen_ai.client.inference.duration`** adoption (4f85037 rename class): fork emits no duration attribute today; if one lands, use the new spelling and extend the golden set + docs table atomically.
- **Fleet-flake hardening** (temp_dir+pid fixture reaping under load): recurring across cycles; a shared fixture-root helper is a candidate infrastructure row.
- Snapshot-staleness watch: pricing bundle still `_snapshot=2026-10-08` (rm-540 rider records the refresh protocol incl. UPDATE_CARD_GOLDEN=1 svg-card trap); semconv pin 07794021 is fresh as of 2026-10-14T11:58Z.
