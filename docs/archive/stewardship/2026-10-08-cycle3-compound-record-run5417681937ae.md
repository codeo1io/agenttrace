# Cycle 3 compound record — run 5417681937ae (repository-maintenance 853a4f32)

Compound phase, attempt 510a968c, 2026-10-08. This record compounds the cycle's pre-review
learnings into durable artifacts. Review and shipping outcomes happen AFTER this step and are
deliberately absent here; the next cycle's assessment carries them forward.

**Prior-attempt forensics (this phase):** compound attempt 375973306928437fb0474ba82296e7dc
died of a provider abort 30 s after dispatch (2026-10-07T16:56:04→16:56:34Z). Durable trail
swept live: the events log holds 4 lines (started / progress 1-msg / progress 6-msgs-then-
reaped-failed / completed), the delegate result JSON is absent, and no `375973306…-scratch/`
directory exists. The worktree's changed-file mtimes (12:28–12:39) predate the attempt window
and the porcelain fingerprint still equals full-tests' `dfccce1e…`, so zero durable or
transient work existed to adopt. **Phase redone from scratch; nothing adopted.** This is the
cycle's fourth provider-dead reap (96bad5fef roadmap, df44633c stewardship, 1d3c0e34
targeted_tests, 375973306 compound) — every one left zero durable work and every one was
detected by the same sweep: events log + delegate JSON + scratch dir + tree fingerprint.

## Batch

Selected by prioritize 14bc885a from the roadmap-16cffdbe wall (232 def rows):
**"Disclosure-plane honesty — attribution, confidence, absence"** =
rm-718 (LEAD, correctness 58.0) + rm-719 (54.0) + rm-716 (48.0). rm-717 (CherryStudio
provider, 36.0) stays candidate — tier-3 demand-gated by its own row. The batch is the
assess-7c8a6446 N2-alias/N3 half plus research-821b3a65 R2/R3, with the other assess halves
(N1 symlink, N2-usage, N4, N5, R1) folding as title-twins into the unlanded 4c34ecaa band
(rm-693/694/699/700/705) rather than being re-minted.

Implemented by d34a7d65 at base 1c5edd1 as an uncommitted 16-surface delta: 8 M files
(+250/−19: lib.rs, parser.rs, session_cache.rs, disclosure_channel.rs,
fixtures/kimi-cli/README.md, tui tests.rs, docs/guides/governance-reports.md,
CHANGELOG.md) + 8 new files (+355 lines: the 10-test `disclosure_plane_honesty`
suite, three new disclosure_channel tests, `tests/fixtures/generic/` and
`tests/fixtures/codex-legacy/` with sha256-pinned READMEs).
Porcelain 11 entries; rm-717 untouched; main.rs (cli) untouched — the batch is core+TUI only.
(Correction recorded at the commit gate: an earlier draft of this paragraph
named pricing.rs/tui main.rs — the authoritative 8-M list is the one above,
matching the validated diff byte-for-byte.)

## Recorded outcomes (pre-review, consumed here — NOT re-run at compound)

Per the phase contract, compound executes no tests. Outcomes as recorded by the cycle:

- **implement d34a7d65 (red-first):** pre-implementation, 9 of 13 new tests failed exactly as
  the assess PoCs predicted (honesty 6/10: model `default`, tokens_cache_r 0 vs 500, verdict
  missing; channel 3/6: counters still in line_skips, confidence still low). Post:
  `cargo test --workspace --locked` → 546 passed / 0 failed / 27 suites = assess baseline
  533 + exactly 13 census-added. Live release-binary PoCs flipped all four targeted
  behaviors: by_model `claude-sonnet-4-5-20250929` / by_provider `anthropic` (was
  `default`/`unknown`); tokens_cache_r 500 / total_tokens 1600 / $0.0047 on the
  camel-alias fixture; `Disclosed facts: codex_rollout_no_usage_rows=1` on the
  #1405-envelope fixture with the with-usage control clean; all four `kimi_usage_alias:*`
  counters ×15 under Disclosed facts with no "Dropped lines" row.
- **targeted_tests e7e4687d:** 8 lanes rc0 (core lib 203/0, honesty 10/0, channel 6/0, tui
  47/0, fmt, clippy −D warnings 32.69 s, docs-gate, install-ref-drift) with the 16
  engine-derived surfaces mapped lane-by-lane; validation digest
  `validation:v1:42740398213dc95635d30f0a4789c61f608a162429ff6c31dfd38d8ff3ffdc8f`
  re-derived with the deployed validation_policy, byte-identical pre and post — the Rust-blind
  classifier makes the crates/** delta digest-immobile (fleet constraint).
- **full_tests cd0d047a:** 22 lanes rc0 mirroring `.github/workflows/ci.yml` @1c5edd1
  lane-for-lane (03-tests 546/0 across 27 result lines == implement == assess+13;
  entrypoints 33/0; output-contract 227 s known-slow multi-tenant scan; real-cli-smoke
  sampled_files=20; cargo-deny all-ok; plugin-version green). Tree fingerprint byte-identical
  pre/post (HEAD 1c5edd1, porcelain sha `dfccce1e…`, untracked sha `b4ac358b…`).
- **Tree re-confirmed at compound:** `git status --porcelain` sha256 prefix still
  `dfccce1e46f51464`, 11 entries, HEAD 1c5edd1 — nothing drifted between full validation and
  this record.

## Schema-bump lesson (PR-1: the bump test is behavioral, not structural)

The stewardship contract predicted "no schema bump (SESSION_CACHE_SCHEMA_VERSION stays 32)".
Implement bumped 32→33 anyway — correctly: the alias intake, the counter→channel move and the
new codex verdict all change what a re-parse reports for unchanged journal files, and stale
warm caches would mask the fixes (rm-230 convention). No serde struct shape changed.

The discriminator for future cycles: **ask "does a warm cache now lie?", not "did the struct
change?"** Any output-visible value change (attribution, counters, verdicts, pricing inputs)
is a bump; struct-literal changes that preserve every cached output are not.

This cycle also broke the sentence-lags-bump failure class: the sweep (session_cache.rs
constant + slot-history comment, both discovery-contract pins, the TUI warm-cache fixture
literal at tests.rs:1670, and the governance-guide sentence at governance-reports.md:72) was
done proactively at implement, so check-docs-commands went green first try — versus three
prior fleet instances where the sentence lagged and a gate caught it. The rm-230 rider
(cargo-test-enforced guide sentence) remains the structural fix; until it lands, copy this
cycle's proactive-sweep behavior.

## Prevention rules carried forward

- **PR-2 (/tmp fixture volatility — vendored by design):** the stewardship contract predicted
  `/tmp/at-assess-7c8a/corpus/*.jsonl` could vanish between phases. It survived this cycle,
  but only by luck (same host, no /tmp sweep). Implement vendored every PoC corpus into
  `tests/fixtures/{generic,codex-legacy}/` with sha256 pins in per-directory READMEs and
  synthesized-shape disclosure. Rule: a PoC that gates a batch must be vendored in the same
  change-unit that implements it; /tmp paths remain evidence, never load-bearing state.
- **PR-3 (prior-attempt forensics is cheap and load-bearing):** four provider-dead reaps this
  cycle, all correctly adjudicated by the same 4-leg sweep (events log tail, delegate JSON,
  scratch dir, tree fingerprint + mtime-vs-window attribution). Keep sweeping BEFORE redoing;
  an absent envelope is not evidence work never happened, and a present one is not proof it
  is valid.
- **PR-4 (title-twin discipline at the mint, not at landing):** the roadmap phase folded five
  assess findings into the unlanded 4c34ecaa band by title instead of re-minting, and pinned
  the sibling boundary in tests (`usage_on_non_meta_rows_stays_estimated_from_text` keeps
  rm-694's usage-BLOCK half red-by-design). Boundary tests that STAY red are the cheapest
  guard against a sibling port silently claiming more than it implemented.

## Next-cycle leads (concrete)

1. **rm-717** (CherryStudio SQLite usage-ledger provider) is this campaign's only unclaimed
   mint; it is tier-3 demand-gated — run the demand census (any CherryStudio journals on the
   host corpus?) before spending an implement slot.
2. **The 4c34ecaa band** (rm-693 N1 symlink temp-staging HIGH security, rm-694, rm-699,
   rm-700, rm-705) is still the hottest external band. Re-census whether it landed before
   doing any N1-class work; if it stays unlanded, N1 is the strongest cycle-4 LEAD candidate
   (severity HIGH with a live PoC that still reproduces at 1c5edd1).
3. **Schema 32→33 sibling collision:** the parallel campaign's rm-710 (inverse warm-cache
   confidence flip) anticipates the same bump. Whichever lands second rebases; the
   governance-guide sentence names the const once, so the same-unit set moves together.
4. **#318 ureq-3 port source:** MAX_PRICING_BYTES 64 MiB via `.limit()` plus a 10-min
   recv-body timeout is now pinned upstream text (research 821b3a65) and is the ready port
   for rm-044/rm-561 — fleet constraint #16993 (fork's 32 MiB read cap) stays binding.
   COMPOSITION WARNING from the roadmap banner: sibling 91d44ce8/82ccc520 already carries
   the manifest wave uncommitted — reconcile by title before porting twice.
5. **Pricing snapshot drift:** bundled snapshot 3,100 rows @2026-10-04 vs live LiteLLM 4,480
   (+7 day-over-day in the 10-07 census). The mutation-class key-delta trigger (rm-176 rider)
   remains the automation signal to build when prioritized.

## Commit-gate handoff

The compound delta ships spool-side (this campaign's c873ef96/57af4bbc convention), stacking
on the roadmap delta in a 3-patch chain applied IN ORDER against base 1c5edd1:

1. `delegate/16cffdbe93d14a799dc681e56e3b5619-scratch/roadmap-16cffdbe.patch`
   (banner + 4 minted rows + 6 riders + evidence refreshes)
2. `delegate/510a968c969f4378b526df58a2510409-scratch/compound-510a968c.patch`
   (this cycle's compound ROADMAP delta: newest-first banner + rm-716/718/719
   candidate→implemented with EXECUTED lines + digest rounding)
3. `delegate/510a968c969f4378b526df58a2510409-scratch/compound-record-510a968c.new-file.patch`
   (THIS file, new at docs/stewardship/)

Then: stage the record file EXPLICITLY (`git commit -am` drops untracked); flip
rm-716/rm-718/rm-719 implemented→done at landing (rm-012 reserves done-flips to the commit
gate); renumber campaign-local ids by TITLE at integration (880a7b9e discipline). Banner-slot
union with the sibling e64d963c patch chain is expected to conflict by design (e43bb8f3
precedent): union banners newest-first, concatenate rider lists chronologically, interleave
tail bands by id. Post-chain census: 232 def rows = 127 candidate / 62 implemented / 43 done,
zero duplicate ids, footer line last.

## Worktree census at compound end

`run-5417681937ae-54176819` @ HEAD 1c5edd1, porcelain 11 entries (8 M + 3 untracked paths;
16 files under `-uall`), byte-identical to the full-tests fingerprint — compound touched no
tracked file in the worktree (its delta is spool-side). Release binary
`target/release/agenttrace` (built 2026-10-07 by the docs gate) left in place for the review
phase. Zero ids minted at compound.
