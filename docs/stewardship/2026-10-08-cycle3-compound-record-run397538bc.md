# Cycle 3 compound record — run 397538bc (repository-maintenance 6277fbeb, cycle 3)

Attempt: compound c3bb9ddc69874597bbb0256cb0e7f05c (2026-10-08), after
provider-reaped 3636613afc8c4b59b05c56e3a1f5f255. Base e9e8fd9 (worktree
run-397538bc5110-397538bc). Review and shipping outcomes are deliberately
absent — they land after this phase; the next cycle's assessment carries
them forward. No test was executed at compound; every validation number
below is transcribed from the recorded phase evidence.

## Batch: "honest intake, honest diagnostics"

- **rm-751** (LEAD, correctness 56.0) — negative rates reach costing
  through the LiteLLM catalog intake while the override lane rejects
  them. Implemented.
- **rm-249** (correctness 70.0) — unreadable directories silently narrow
  coverage; `-d` mislabels permission-denied as absence. Implemented.
- **rm-752** (ci-integrity 34.0) — `--doctor` defines no failure exit.
  Implemented.

Selection: prioritize b54603c6 (batch dossier
`delegate/b54603c659bb4ff491d180a10c616479-scratch/prioritization.md`);
stewardship contracts: 5bdac7ab (3rd attempt; see forensics).
Wall: this run's spool roadmap postimage 82e59be2 (rows rm-751 :2356,
rm-249 :1412, rm-752 :2364, all `candidate` at selection).

Delta: 8 tracked files, +780/−78, uncommitted at e9e8fd9 —
crates/agenttrace-core/src/{pricing.rs, discovery.rs, doctor.rs, lib.rs},
crates/agenttrace-cli/src/main.rs, crates/agenttrace-cli/tests/entrypoints.rs,
CHANGELOG.md, README.md. No session_cache.rs / SESSION_CACHE_SCHEMA_VERSION
move. Implement attempt: c2dfa71a.

What landed (code-read of the delta, anchors for the commit gate):

- **rm-751**: `convert_litellm` enforces finite AND non-negative per-field
  (input/output/cache-write/cache-read) on every catalog lane — bundled
  snapshot, `--update-pricing` download, cache read; hostile rows skip
  whole → disclosed fallback lane (`data_health: fallback_pricing`) +
  hostile-row report names them. Cache loader quarantines a poisoned
  catalog (`.hostile` rename beside itself, bytes + provenance stamp
  preserved, stderr disclosure at the moment it happens, falls back to
  the bundled snapshot). `--update-pricing` refuses to persist a hostile
  download (named error, previous cache stays).
- **rm-249**: walk stays infallible but returns the unreadable-directory
  skip set (`find_session_files_reported` / `collect_session_files_reported`
  + cached variants; `LoadReport.unreadable_dirs`); CLI announces
  "N session directories not readable — coverage narrowed; check
  permissions (sample paths, +N more)" on stderr, stdout pure; `-d`
  validated up front — missing / file / not-readable = usage error exit 2
  naming the denial and the positional alternative; `-d <dir> <file>`
  refused (closes review 3fb3e39d F5); mid-walk vanish ≠ denial.
- **rm-752**: `DoctorReport.failed_checks` (serde-skipped when empty);
  build/render split (`doctor_report_render`); `--doctor` exits 3 when
  any check fails, 0 healthy, documented in `--help`; code 3 chosen
  distinct from clap usage 2 and action-failure 1.

## Recorded validation outcomes (NOT re-run at compound)

- **implement c2dfa71a**: `cargo test -p agenttrace-core` 209 lib + all
  integration suites, 0 failed (discovery_contract 90/0); `-p agenttrace`
  61 + 7 + 37 + 4 + 9 + 2 (entrypoints incl. the 3 adopted red-first
  pins); `-p agenttrace-tui` 47/0; `cargo fmt --all --check` clean;
  `cargo clippy --workspace --all-targets -- -D warnings` clean.
- **targeted 8a13cdb3** (battery log
  `delegate/8a13cdb3c73c4940a5abce95234543ac-scratch/targeted-battery.log`):
  declared scope **targeted**, overriding the dispatch's derived
  `required_scope='none'` — the tree held the implement delta's 8 modified
  files (6 executable Rust surfaces); declaring 'none' would have
  misrepresented the tree the fold gate inspects. Core 386/0 across 18
  result lines (209 lib incl. `convert_litellm_rejects_negative_rates` +
  `load_pricing_cache_quarantines_a_poisoned_cache`; discovery_contract
  90/0); cli 120/0 across 6 lines (entrypoints 37/0; each batch pin
  green); tui 47/0; fmt + clippy clean. **556 passed / 0 failed across
  26 'test result: ok' lines.**
- **full 87c235c6** (record + per-lane logs
  `delegate/87c235c65fce457b9469272780a7037c-scratch/`): dispatch
  `full_command` EMPTY → this tree's `.github/workflows/ci.yml` full+deny
  job lanes ran VERBATIM as command authority (standing fleet precedent;
  lint job is PR-only, two event/var-gated lanes documented-excluded).
  **21 lanes, ALL rc0** — fmt, clippy --locked, tests 556/0 (26 result
  lines), release build, entrypoints 37/0, output-contract,
  deterministic-output, report-semantics, release-surfaces,
  example-workflows, install-runtime, docs-commands (lane 12 — its
  `--doctor` leg rc0 against a healthy custom dir reconciles with the new
  rc-3 contract), real-cli-smoke, homebrew, npm, cargo-manifests,
  plugin-version, helper-scripts + 18b install-sh, locked-cargo,
  `cargo deny --all-features check`.
- **Baseline reconciliation**: 556 = assess 33ff0892 baseline 551 + the
  batch's 5 new tests (2 unit core-lib + 3 entrypoints E2E) — exact.
- No validation digest token was recorded by this cycle's validation
  phases; none is claimed here.

## ROADMAP spool patch applied at compound

The roadmap phase (82e59be2) delivered its delta as a spool patch +
postimage with the worktree restored byte-clean. Compound applied it
(compound-gate duty, 8e983cf5 precedent):

- `git apply --check` green; `git apply` clean.
- Worktree `ROADMAP.md` after apply: sha256
  `a95b6698a203b0c96d2cfc93a62aeb8e2367f8ac68c8fd5def22992939cb6080`,
  `cmp`-verified BYTE-IDENTICAL to
  `delegate/82e59be2073b41019333a5947fcec1d5-scratch/ROADMAP.postimage.md`
  (base sha f72ae43f… matched the pre-apply tree exactly).
- On top of the postimage, compound added: the `compound c3` banner
  (newest-first at the banner-block top) and the three
  candidate→implemented flips with dated EXECUTED bullets. Post-edit
  census: 259 def rows (245 backtick + 14 legacy), **0 duplicate ids**,
  max def id rm-752, managed-render footer still the file's last line;
  status accounting 245 backtick rows = 132 candidate / 62 implemented /
  51 done.

## Dead-attempt forensics (this cycle's reap tally)

Five provider-infrastructure deaths across the cycle; zero lost work —
forensics-then-adopt-or-redo each time:

1. **roadmap 85fa8b66** — zero durable output (event log reading-phase
   pings only, no typed result, no tree drift). Redone from scratch
   (82e59be2); declared in the minted banner.
2. **stewardship 2d5b52a9** — envelope JSON structurally INVALID
   (duplicate `action_id` key). Content superseded by redo.
3. **stewardship 41534a7b** — content complete and verified live, but
   the session was reaped before the result was emitted. Redone (5bdac7ab)
   with every anchor re-verified.
4. **implement 8cc6f907 + b6aa0891** — zero typed results, but the
   durable trail held real work: 8cc6f907's red-first E2E layer in
   `crates/agenttrace-cli/tests/entrypoints.rs` (+258 lines, 3 tests,
   red at base per its spool `red-run1.log`) and b6aa0891's
   `contracts.json` mapping. Both ADOPTED by c2dfa71a; production code
   written fresh.
5. **compound 3636613af** — event log = dispatch + one progress ping +
   `session_reaped/failed` at ~25s; typed result absent; ZERO tree drift
   (ROADMAP.md sha f72ae43f… still the base at forensics time). Nothing
   to adopt; compound redone from scratch (this attempt), declared in the
   banner.

## Id accounting

- ZERO ids minted at compound.
- This wall's def-max after the roadmap phase's mint: **rm-752**
  (campaign-local until merge; renumber BY TITLE at integration per the
  880a7b9e discipline).
- Sibling unlanded spool claims observed this cycle: rm-748..rm-750
  (dcc04243 postimage, same e9e8fd9 base, title-disjoint:
  reseller-claiming / gate-drop / dead-SQL) and rm-753..rm-755 (5cf79d29
  @1c5edd1e: OnceLock-test title-twin / publisher-arg-drop / statusline
  journal-append). Fleet walls on other bases carry higher numerals —
  any future mint MUST re-run the live def-row census at mint time.
- Next free on this wall's numbering: rm-753 first use is CONTESTED by
  the sibling 5cf79d29 claim — mint at ≥rm-756 after a fresh census, or
  reconcile by title at integration.

## Prevention rules (reusable)

- **PR-1 — Envelope structural validity is part of the deliverable.**
  2d5b52a9's result envelope parsed only loosely: a duplicate
  `action_id` key made it structurally invalid JSON for strict parsers.
  Rule: after writing the result JSON, re-read and strict-parse it
  (`python3 -c 'json.load(...)'` or equivalent) before finishing; a
  succeeded phase whose envelope cannot round-trip is a transport
  failure wearing a success flag.
- **PR-2 — Declare the tree you hold, not the dispatch's derivation.**
  The targeted dispatch derived `changed_testable_surfaces=[]` /
  `required_scope='none'` while the worktree carried 8 modified files.
  8a13cdb3 declared 'targeted' and ran the battery over every changed
  surface. Rule: the fold gate inspects the TREE; when the dispatch's
  derived scope understates the tree (the known classifier artifact
  around crates/**), declare the real scope and say why in the envelope.
- **PR-3 — Dead attempts are candidate work product until proven
  otherwise.** This cycle adopted a red-first test layer and a contracts
  mapping from two dead implement attempts (durable trail: tracked-file
  drift + spool scratch), while three other dead attempts (85fa8b66,
  3636613af, 41534a7b) held adoptable CONTENT only after claim-by-claim
  verification. Rule unchanged from fleet precedent, now with a
  structural checklist: typed artifact → event log → tree drift census
  (porcelain + sha of untouched sentinels) → spool scratch; adopt only
  what verifies, declare the rest redone.
- **PR-4 — The CLI exit-code space is now documented; keep it that way.**
  0 clean / 1 action failure / 2 clap usage + lane validation / 3
  diagnostic failed (`--doctor`). Any new exit semantics picks a code
  from this table (or extends the table in `--help` where it lives) —
  rm-019's rider (distinct from clap usage errors) is honored by 3.
- **PR-5 — PoC corpora: the spool mirror is the authority.** The assess
  PoCs lived under /tmp (sweep-volatile); the durable mirrors are
  `delegate/82e59be2073b41019333a5947fcec1d5-scratch/poc-mirror/`
  (xdg-neg poisoned cache, sessions-neg journal, main-model_prices.json,
  modelsdev.json, litellm-commits.json). Cite the mirrors, not /tmp.

## Commit-gate checklist (for the landing, not compound)

1. ONE commit for the whole batch: the 8 implement files + ROADMAP.md
   (spool patch + compound banner + 3 flips) + this record
   (`docs/stewardship/2026-10-08-cycle3-compound-record-run397538bc.md`,
   untracked — stage it EXPLICITLY).
2. Done-flips for rm-751 / rm-249 / rm-752 AT THE GATE, BY TITLE
   (rm-012 convention). Sibling lanes minted on the same base
   (dcc04243 rm-748..750, 5cf79d29 rm-753..755) may land first —
   reconcile by TITLE, never by numeral.
3. CHANGELOG Unreleased → Fixed already carries the batch bullets from
   implement; land or re-word at the gate. No compound rider was added.
4. No SESSION_CACHE_SCHEMA_VERSION move in this batch — schema-pin
   surfaces stay untouched; do not re-base session-cache pins.
5. If the gate rebuilds from a wiped target/, verify
   `target/release/agenttrace` exists before the release-binary gates
   (~2.5 min rebuild) — the known worktree target/ wipe recurrence.

## Next-cycle context

- **Deferred queue** (priority order from prioritize b54603c6, all still
  candidate): rm-421 / rm-251 / rm-195 @90.0 (verify-first + dependency
  deferrals recorded in the dossier) → rm-448 / rm-239 @87.0 →
  rm-240 / rm-232 @86.0 → rm-053 @84.0 → rm-540 / rm-449 / rm-367 /
  rm-175 / rm-085 / rm-014 @82.0.
- **Open seams this batch touched but did not claim**: `--doctor`'s
  failing-exit consumers beyond check-docs-commands.sh (docs and
  workflows invoking --doctor were reconciled green through lane 12, but
  new consumers should cite the rc-3 contract); the hostile-row report
  surface (`--update-pricing` names refused rows — a listing/list-pricing
  disclosure arm may deserve its own row if users need to enumerate
  quarantined caches).
- **Standing reap risk**: this cycle lost five attempts to provider
  infrastructure and one stewardship envelope to a duplicate-key defect;
  PR-1/PR-3 above are the countermeasures. Keep writing the result JSON
  FIRST, before the final message.
- **Fleet watch items carried forward** from the research dossier
  (447a9a7c): LiteLLM tier-suffixed rates now sourceable (rm-164 lane);
  npm bare-name window holds but the neighborhood is contested
  (rm-017/rm-195); bundle-vs-live drift 24 base-price changes + 3 dropped
  keys in one day (rm-176/rm-006 refresh triggers firing).
