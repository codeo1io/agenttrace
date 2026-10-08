# Compound record — cycle 3 — run `ec762a618372` (2026-10-08, pre-review + same-cycle review-fix addendum)

Repository-maintenance `fad1cbf4` cycle 3. Base `92149bd` (worktree `run-ec762a618372-ec762a61`), campaign span 2026-10-07→08. This record compounds the cycle's pre-review evidence, plus the same-cycle review-fix outcome appended below ("Review fix"); shipping outcomes land at the commit gate.

## Batch

**"Honest usage truth on every claiming lane"** — `rm-760` (lead, correctness 74.0) + `rm-449`'s F3/F4 fold arms, selected by `23db236e` (PRIORITIZE) from the unlanded pool against this cycle's fresh mints `rm-760..763` and the sibling-ownership sweep. One change-unit, one classifier seam; evidence chain assess `984bcea3` F1-F4 (live PoCs at `92149bd` on a fresh release binary) → roadmap `7621cfd3` (mints) → stewardship `9205dff6` (contract with pinned line refs).

What landed (implement `5c6710d9`, pre-review/uncommitted):

- The gemini lane's bare `usage` probe no longer claims single-object journals carrying the journal grammar's `type`+`role` pair — token truth and tool attribution no longer flip on line count (the f4-alias pair: single-object was `gemini_cli 100/50`, byte-identical 2-line was `hermes_jsonl 0/0` + alias disclosures; now both report identically).
- The disclosure classifier case-matches `Usage`/`usage` (the spelling `Event` itself deserializes via rename+alias) at all three container probes, and the tier engine (`usage_unknown_key` / `usage_alias_unmapped` / `usage_unconsumed_location` + new `usage_key_non_numeric` / `usage_non_object`) runs on the whole-JSON hermes and gemini document lanes' success paths — every claiming lane now accounts or discloses the usage containers its scan pinned: root containers, capital twins, and message arrays. (Review `e747789a` found the pre-review scan did NOT yet reach nested wrapper objects or shadowed duplicate aliases — both closed in the review-fix pass below, which extended the same tier engine to every object level that claims usage.)
- All-noncanonical maps ride `rm-408`'s `zero_usage_reported` rider instead of upgrading to clean `reported_by_agent`; non-object `usage` values disclose `usage_non_object:<kind>` and mark the text estimate `+usage_unusable:<N>` ("— usage present but unusable", En/Zh text + TUI).
- `SESSION_CACHE_SCHEMA_VERSION` 32→33 (rm-230 convention: parser-semantics changes that alter the served report for unchanged files bump the schema); the TUI planted warm-cache fixture and the governance-guide schema sentence ride the bump.
- 8-test lane suite + 12-fixture corpus (`tests/usage_truth_lanes.rs`, `tests/fixtures/usage-truth/`) mirroring the assess RED corpus (review-fix pass below: 11 tests / 17 fixtures at cycle close).

## Review fix (2026-10-08, post-review)

Independent review `e747789a` returned **NEEDS_CHANGES** — 1 medium + 3 low — and every finding was fixed in the same cycle by `dfbb49b2` (third reap-with-work this cycle: `c47bcc81` wrote the complete fix drift, was reaped after ~40 min, and was adopted after leg-by-leg verification):

- **F1 (medium)** — the reasoning fold consumed its winning wire key but never pushed it onto `matched_keys`, so a benign root `usage:{...,reasoning_tokens:5}` minted a standing false `usage_unconsumed_location:reasoning_tokens` (`parser.rs` `usage_from_value_with_keys`). Fixed: the fold pushes its winning key (first numeric, all four synonyms). The fix pass also closed the zero edge the first cut missed: consumption is numeracy-based like the class winners, so an explicit `reasoning_tokens: 0` counts as consumed on this lane exactly as on the JSONL lane (`USAGE_KEYS_CONSUMED`), instead of minting the same standing disclosure (`reasoning-fold-zero.json`).
- **F2 (low)** — `gemini_usage` folded the FIRST present alias but disclosed only the class winners, so a non-numeric first alias shadowed its numeric sibling silently (`parser.rs:5232` `first_number`); `gemini-shadowed-alias.json` (100/**0** was reported; now 100/50 + `usage_unconsumed_location:input_tokens`).
- **F3 (low)** — the gemini scan walked only top-level containers while `parse_gemini_object` also claims usage from nested `checkpoint`/`session`/`chat` wrappers, so a nested usage minted no disclosure (`parser.rs:5303`); the scan now recurses those wrappers (conservatively — a usage object it finds but the parser does not read still discloses) (`gemini-nested-usage.json`).
- **F4 (low)** — the universal claims above outran the pinned surface; scoped here, and the CHANGELOG wording updated to the 17-fixture corpus once the scan mirrored every object level that claims usage.

Re-validation (this pass, sequential): lanes 11/11, core 414/0 over 21 suites, tui 48/0, fmt clean, clippy `-D warnings` rc0; the review's own PoC files replay green (false disclosures gone, shadowed/nested now disclose); fixture blast-radius sweep — no tier disclosure on any pre-existing fixture; warm==cold with the cache engaged (`--overview -f json -d`, schema 33, 4 entries — session payloads byte-identical, only `cache_hits` differs by design). Digest re-derived with the engine's own derivation over the current tree: `validation:v1:58f2516eab516d394143c7076c368d37133a4890e9612a52e28bf67ab1bd1730` (unchanged — the fix touched no file the surface classifier counts, the known Rust blindness `#17194`).

## Attempts and forensics (reap-after-work ×3 in one cycle)

- stewardship: `515fc393` reaped with a COMPLETE typed artifact on disk → adopted by `9205dff6` after lineage verification (the artifact-present branch).
- implement: `ccaedd53` died of a provider 429 (523s) after writing a complete, contract-matching delta; its scratch logs showed lanes 8/8 + core/tui green but FMT/CLIPPY red. `5c6710d9` adopted the delta and closed the interrupted completion: clippy `iter_cloned_collect`, fmt drift, and the missed session-cache schema bump. The census-mismatch (adopt) branch of the reaped-attempts prevention rule is now documented: `docs/solutions/workflow-issues/provider-reaped-delegate-attempts-redo-from-scratch-on-census-match.md` (dated addendum).
- review-fix: `c47bcc81` reaped after ~40 min with the complete F1/F2/F3 code + fixtures + tests on the tree and no typed artifact; `dfbb49b2` verified the drift leg-by-leg (all six parser.rs hunks accounted for), adopted it, closed F4, and added the zero-reasoning edge the first cut missed. The drift-present ADOPT branch of the same prevention rule fired a third time.

## Validation (recorded outcomes, not re-run here)

- targeted (`a8449989`): lanes 8/8, core unit 209/0, tui 48/0 (incl. planted schema-33 warm-cache + provenance label pins), fmt clean, clippy `-D warnings` rc0.
- full (`12329992`): ci.yml push-event lanes mirrored verbatim — 20/20 green; authoritative lane `cargo test --locked -p agenttrace-core -p agenttrace-tui -p agenttrace` 577/0 over 30 suites; entrypoints 34/0; docs-gate verified the schema-33 sentence against the live const; `cargo deny --all-features check` rc0. Validation digest stable across all folds: `validation:v1:58f2516eab516d394143c7076c368d37133a4890e9612a52e28bf67ab1bd1730` (re-derived independently before and after each fold).

## Integration seams for the commit gate

1. **Stage the 2 untracked test paths explicitly** (`tests/fixtures/usage-truth/` + `tests/usage_truth_lanes.rs`) — a census with `git grep` alone undercounts them; `commit -am` would drop them.
2. **`SESSION_CACHE_SCHEMA_VERSION` 33 collision:** sibling run `32f3b7a1` has an uncommitted 32→33 delta of its own — the second lander re-bases onto the advanced ceiling (33→34) and merges the governance sentence (7f9c6d24 post-merge precedent).
3. **Title-twins `rm-756`/`rm-757`** (sibling `9ab0afad`, three failed implement attempts, nothing landed): reconcile by title at integration — `rm-760` is the superset seam (claiming-lane dispatch + case-matching + whole-JSON scan) and carries the combined acceptance.
4. **Done-flips stay reserved** to the integration gate (rm-012 / 7eae74eae ruling); this compound flips only candidate→implemented with dated EXECUTED addenda.
5. ROADMAP delta at the commit gate = the roadmap phase's +40 (mints) + this compound's banner/flips/addenda; nothing else moved.

## Research compounding

- `docs/research/pass-12-2026-10-08-antigravity-quota-zcode-and-pricing-census.md` (this cycle's `9a4ad8e0` dossier, first-hand probes only): R1 HIGH Antigravity 2.16 dual-shape quota (codeburn `#1667`), R2 ZCode/Z.ai demand 2-sourced in 24h with glm-5.3 pricing already bundled, R3 LiteLLM census (4,486 live; `_flex` growing; `gpt-6-sol` root-key correction; nested cost objects an intake-drop risk), R4 codex account dimension, R5 quiescence + OSV-clean pins.

## Next-cycle leads (concrete, unclaimed)

- `rm-239` P87 text-cell routing residual — strongest parked lead (bounded, unclaimed; PoCs need base re-verification).
- `rm-761` Antigravity dual-shape quota (pass-12 R1, fixture-first — no local install on this host).
- `rm-762` pricing-snapshot refresh under fixture-first discipline (pass-12 R3 arms; resolve `chatgpt/` alias normalization before trusting null-cost skips).
- `rm-763` codex per-account attribution (pass-12 R4; smallest).
- `rm-421` 90.0 epic keeps its "next cycle LEAD" note (since 10-04); `rm-195` 90.0 and `rm-448` 87.0 remain parked epics.
- Watch (no row yet): the gemini lane's retirement-vs-fix decision now has all three inputs — upstream deleted it (`#312`), Google is migrating the CLI (`#236`), and its claiming probe is fixed but its quota lane is two payload versions behind (`rm-761`).

## Wall state after compound

264 def rows, 0 duplicate ids; status census by inline token (`| status: <word>`): 134 candidate / 64 implemented / 52 done, with 14 id lines carrying no inline status token (rm-760 moved candidate→implemented this pass — the single flip this compound made). Managed footer still the file's last line; banners newest-first with the cycle-3 compound banner on top.
