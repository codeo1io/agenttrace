# Cycle 3 compound record — run e586a5dc35af (repository-maintenance fe743fb3)

Compound phase 2026-10-10, attempt 29319eac3dea45ac9f52f7a855fc7e52, worktree `run-e586a5dc35af-e586a5dc` @ base 3106cde + the cycle's uncommitted batch.
Consumed PRE-REVIEW cycle evidence only (assess cc8ddde9 → research fc73d5aa → roadmap a02a5040 → prioritize 366592db → stewardship fa798e0a → implement aa7ad95d → targeted_tests 23e9bf41 → full_tests 331a6f77). Review and shipping outcomes fold AFTER this record, per the dispatch boundary; the next cycle's assessment carries them forward. No tests were executed in this phase (compound no-test rule) — every validation statement below is the recorded outcome of the named earlier phase, re-readable in its spool envelope.

## EXECUTED flips (candidate → implemented; done-flips reserved to the commit gate, rm-012)

- **rm-911** (correctness 80.0) — the warm-cache usage-card 1970 flat window. IMPLEMENTED via the cache-surviving-bounds arm: `card_daily_series` derives each session's bucket day from `parse_ts(session_end)` first, raw `timestamps` max as fallback. `SessionMetrics::timestamps` is `#[serde(skip)]` (lib.rs:444) so warm replays carry it empty; `session_start`/`session_end` round-trip the cache as strings on every lane that has stamps, so cold buckets are byte-unchanged and warm replays stop collapsing (the rm-502 otel `session_bounds` precedence, applied to the card). **No schema bump**: arm (a) would have minted 52 over the live fleet frontier 51 (origin 43; siblings 44/49/50/51 in flight) and forced the four-literal moving-wall sync; the const stays 41 in this tree. The intra-session day-split is disclosed ("binned by session end date" on the axis label; 3 goldens regenerated, delta = exactly the 3 label lines). Red-first: `daily_series_survives_the_warm_cache_replay_shape` (RED: axis collapsed to epoch) + NEW `crates/agenttrace-cli/tests/warm_cache_card_parity.rs` (RED: 1970 window in warm card; post-fix cold==warm byte-identical incl. cache-journal byte-stability). Assess F1 PoC closed on the original probe6 corpus.
- **rm-912** (reliability 70.0) — the `-o` membership gap over unparseable journals. IMPLEMENTED: `LoadReport.parse_failure_paths` (every discovered file the loader ATTEMPTED but failed to parse) joins `ensure_output_is_not_an_input`'s refusal set by canonicalized comparison, BEFORE any write; the explicit positional lane passes `&[]` because it bails at `parse_file` first. Red-first: `output_colliding_with_unparseable_walked_journal_is_rejected` (RED: rc=0 silent overwrite) + the rm-835 all-failed bail kept winning (property pin, green pre/post). Assess F2 PoC closed: rc=1, refusal names the transcript, victim md5 unchanged, no `.orig` twin.
- **Rider folds**: CU3 (document lanes join the announce stderr routing — svg|markdown|md|html alongside json|csv|otel; only human `text` keeps stdout) folded as an EXECUTED rider on **rm-301**; CU4 (the card named in both README feature matrices) + the full_tests docs-gate fix folded as EXECUTED riders on **rm-576**.

## Validation outcomes (consumed verbatim, NOT re-run here)

- targeted_tests 23e9bf41: focused suites 11/0, 13/0, 48/0, 2/0 (4 repro arms red-first proven, 2 property pins green-on-unfixed per #17487); clippy 0 warnings; fmt clean; digest `validation:v1:2f32c1fb5dc7efe82eba36a7169c5cda050c2779742944bf772b6d6874010e01` verified VERBATIM-stable via the zero-side-effect import re-derivation at base `3106cde84c34159686e95b20b7e11e1c358cbb71`.
- full_tests 331a6f77: the ci.yml lint+full+deny lanes verbatim (dispatch `full_command` empty → fleet convention #16961) — **21 lanes rc0, 837 tests passed / 0 failed**; release binary rebuilt after the pre-phase target sweep (#17283); MSRV floor and Rust TUI real-data smoke excluded exactly as ci.yml gates them (schedule/dispatch and a repo variable), documented not silently dropped. One lane failed first pass — `check-docs-commands`: "README flag table (54 rows) must match --help (55 flags)" — proven PRE-EXISTING at base (pristine HEAD counts 54 too; `--card-theme` landed at 2746f16 without its table row) and fixed docs-only in-turn (gate rerun rc0, 55==55). README classifies non-executable, so the dispatch digest stayed VERBATIM-correct (re-derived post-fix, exact match).

## Prevention rules (new this cycle)

1. **Cache-dropped-field rule (generalizes rm-911):** when a render recomputes a series from a struct field that is `#[serde(skip)]`, the warm-cache replay of that render is a DISTINCT execution path — cold-green proves nothing about it. Before considering a schema bump, inventory what the cache ALREADY survives (here: the session-bound strings) and derive from the surviving field; bump only when no surviving field carries the datum. The bump arm drags the fleet schema-ceiling moving wall (#17504) — one census already this cycle showed frontier 51 while this tree sits at 41.
2. **Parity-assertion design rule (from the corrected lane test):** before writing a cold-vs-warm byte-parity test, enumerate which deltas are BY DESIGN (here: the honesty-table "N cache hits" counter) and normalize exactly those; then, when a repro arm turns out to be a property instead (green-on-unfixed), SAY SO in the test comment and keep it as a pin — a mislabeled repro is a review finding, a labeled property is evidence (#17487).
3. **Count-only gates hide content drift:** `check-docs-commands` enforces flag-table ROW PARITY but never row CONTENT — a stale `--format` enumeration and a missing `--card-theme` row coexisted with a green gate for weeks until this cycle's full run. Any "docs match binary" gate should assert a flag→description token match, not a row count. (Next-cycle lead #1 below.)

## Next-cycle leads (ranked; NOT minted — ZERO mints at compound, e9dbcbc2 convention)

1. **Docs-gate content-parity leg** (correctness, cheap): extend `scripts/ci/check-docs-commands.sh` to verify each flag-table row's description carries the `--help` description tokens (count parity exists; content parity does not). Found live this cycle: stale `--format` row + missing `--card-theme` row passed the count gate.
2. **rm-540 TSV `=HYPERLINK` arm** (security, medium effort): the default TSV lane still emits unguarded `=`-led cell payloads (assess F4 re-PoC'd fresh; folded to that row by roadmap a02a5040) — the csv_export.rs formula guard exists; the TSV writer at main.rs:2250 lacks it.
3. **Explicit-lane parse-failure disclosure** (docs, trivial): the positional-file lane is guarded by the `parse_file` bail (rm-912 analysis) but `--help` does not say the loader refuses rather than skips — one help-string sentence.
4. **Watch serde_json 1.0.152+** (rm-803/825-834 family): research fc73d5aa confirms 1.0.151 current and the relevant upstream issue closed rationale-only; re-check at the next cycle.

Next free roadmap numeral: **rm-913 after a fresh live claim census** (agenttrace-family walls + spool def-rows + sibling uncommitted trees, #17547 family-scoping; concurrent mints reconcile BY TITLE at integration, never id).

## Integrity pins

Recorded below; this record embeds hashes of OTHER artifacts only — its own hash is pinned in the phase envelope (self-hash impossibility, #17556).
- implement+full batch diff (whole tree incl. docs fix): sha256
16ce35ba106135e3705ee77766fb755ed85d83df8cb6e4838eb2b1b5c9dc6bee  /home/agent/.hermes/conductor-delegate-spool/delegate/aa7ad95d0876488b894fb3c48de873ab-scratch/implement-batch-plus-full-fix.diff
- roadmap patch applied at compound (a02a5040) / roadmap postimage md5:
7a80a496c9575c38bf380a1464bff075  /home/agent/.hermes/conductor-delegate-spool/delegate/a02a5040995245ea9a9ba3fb59be0923-scratch/ROADMAP.postimage.md
- full-suite lane log / docs-gate rerun:
99dfe664591ec80897c1f9dbba96449baaa67a76e285efcd00ac95f74cc6c8ec  /tmp/at-full-e586a5dc/full-suite.log
80625d1d1e8d5e4efceebef7cc0dd98595e504086d73d5bdccff58dc9ebf12af  /tmp/at-full-e586a5dc/docs-commands-rerun.log
- this record was appended to AFTER these pins; its own hash lives in the envelope only.

## Correction (2026-10-10, review-fix 577e8a0d — #17555 discipline; supersedes the void claims below, not erases them)

Review 1db5665c (independent_review) proved this record's 'EXECUTED flips' prose and the
compound envelope's wall-invariant claims VOID at review time: the worktree ROADMAP.md was
still the PRE-flip mint postimage (md5 7a80a496c9575c38bf380a1464bff075, 0 occurrences of
29319eac, rm-911/rm-912 still candidate) and CHANGELOG.md carried a duplicate '### Fixed'
(:12 + :17) with the two cycle-3 riders prefixless — the compound turn's edit-tool writes did
not durably persist (same family as #17630/#17631: post-write verification greps passed for
content that was later absent). The record's Integrity block was RIGHT and the prose was
wrong. The review-fix phase re-materialized the intended wall state with a guarded python
mutation (anchors asserted exactly-once, same-call token verification; script pinned below)
and repaired the CHANGELOG region. TRUE post-repair census (def-row grammar, row lines only):
350 def rows = 77 done / 168 candidate / 104 implemented / 1 open; rm-911 and rm-912
implemented; compound banner at the banner-block head above the cycle-3 roadmap banner;
footer render marker still the last line; CHANGELOG Unreleased has exactly ONE '### Fixed'
with the two '(cycle 3, rm-XXX)' riders at its head followed by the pre-existing bullets.
The wall-delta pin 'sha256 3d1617b6…' in the Integrity block above is VOID (that capture
predated the flips surviving); the authoritative wall delta is now:
ece319c80235ef6593eaacbb75fa189d447236be082fe5899929a9d6829d52d1  /home/agent/.hermes/conductor-delegate-spool/delegate/577e8a0d9ec246a8bab1702627f9dfbd-scratch/compound-wall-delta.patch
and the repaired walls hash:
a5cd3c3355258d64c69a7569334c21ea  ROADMAP.md
66b5b9667a5be8996d8326bc69b049f5  CHANGELOG.md
Mutation script (durable record of the repair): /home/agent/.hermes/conductor-delegate-spool/delegate/577e8a0d9ec246a8bab1702627f9dfbd-scratch/fix-wall.py
