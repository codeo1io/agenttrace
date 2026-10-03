# Cycle-1 implementation record — run 364aa3be (repository-maintenance dde7875c)

- **Phase:** implement (attempt `cfee1fa4`), 2026-10-03
- **Worktree:** `run-364aa3be00f6-364aa3be` at `fd5532f`, starting from the verified
  stewardship CU-0 state (`M ROADMAP.md` + untracked `docs/stewardship/2026-10-03-cycle1-prioritization.md`,
  porcelain otherwise 0 — re-verified with `git status --porcelain` at phase start)
- **Batch** (selected by prioritize `e03c70cf`/`b8cfcdc2`, stewardship `9f7c4806`):
  "The cache obeys its own contract, and the pipe stays pure" — lead `rm-298` (M) +
  riders `rm-301` (S) + `rm-303` (XS)
- **Do-not-commit:** nothing was committed or pushed; all changes are uncommitted
  worktree drift awaiting the commit phase, per the phase contract.

## CU-1 — rm-298: byte-true session-cache bound + first dirs bound

**Root cause (assess `c22757c9` A1, re-measured live this run):**
`enforce_byte_bound` summed only the per-entry serialized VALUES, while
`save_session_cache` also writes per-path keys (~80 B/entry), JSON punctuation, the
top-level fields, and the whole `dirs` map. On the real corpus the fresh cache wrote
72,262,482 B = **108.9%** of the documented 64 MiB "hard bound"
(`MAX_SESSION_CACHE_BYTES`, session_cache.rs doc contract) while the estimator called
it in-bounds. `dirs` additionally had no bound of any kind.

**Changes** (`crates/agenttrace-core/src/session_cache.rs`):

- New helpers, all `rm-298`-commented: `json_key_len` (key with quotes + escapes),
  `json_object_len` (braces + member bytes + inter-member commas), `dirs_member_bytes`,
  `doc_frame_len` (fixed top-level prefix `{"schema_version":N,"dir_listing_version":M,"entries":`
  + entries object + optional `,"dirs":{...}` + closing brace), and
  `serialized_doc_size(cache)` — the byte-true projection of the file
  `save_session_cache` writes.
- `cache_paths_sized_once` now returns each path's WRITTEN-form size (the decoded copy
  overwrites the raw copy for the same path at save time — modeled exactly, and pinned
  by the byte-exact test below). The previous pass-11 "larger of the two forms"
  behavior is superseded: it over-counted what the writer emits.
- `enforce_byte_bound` enforces the cap over `serialized_doc_size`: fast path (one
  projection, no member bookkeeping — the common in-bounds save), slow path evicts
  oldest-mtime entries first, decrementing the projected total by each member's
  `key:value` bytes plus one comma (`usize::from(count >= 2)` guard for the last
  member). Headerless entries remain `i64::MIN` (oldest, F5-5 preserved); the
  deduplicated path union counts once (pass-11 A11-2 preserved).
- New `enforce_dirs_bound` + `MAX_SESSION_CACHE_DIRS = 20_000` +
  `MAX_SESSION_CACHE_DIR_BYTES = MAX_SESSION_CACHE_BYTES / 8`: listings evict oldest
  directory mtime first until both budgets hold. The 1/8 sub-budget guarantees the
  entries map can always reach the overall cap (the dirs block can never crowd entries
  out of it).
- `save_session_cache` now runs entry-count → dirs count/bytes → byte-true bound, in
  that order, before serializing.

**Tests** (all in the `session_cache` test module):

- `byte_bound_covers_the_written_document_not_a_model` — **red-first**: with the
  ceiling set to the pre-fix model's own in-bounds value, the old code wrote
  16,910 B against a 13,112 B ceiling (129% — same defect class as the live 108.9%);
  post-fix the WRITTEN FILE fits, oldest-first eviction and dir-preservation asserted.
- `serialized_doc_size_predicts_the_written_file_exactly` — pins the projection
  byte-for-byte against the file on disk, including a key needing JSON escapes
  (`probe "quoted-1"\slash/...`), a path living in BOTH maps with a stray-field raw
  copy (the decoded form is what reaches the file), and a non-empty `dirs` map.
- `dirs_map_gains_count_and_byte_bounds_of_its_own` — count-bound eviction, then a
  one-byte-under budget dropping exactly the oldest listing.
- The two pre-existing byte-bound tests were updated from values-only ceilings to
  byte-true ceilings (`serialized_doc_size - oldest members`), preserving their
  "drops exactly the two oldest" assertions.

**Live acceptance (release binary, fresh isolated cache on the real corpus):**

```
AGENTTRACE_SESSION_CACHE_DIR=<fresh> target/release/agenttrace --overview -f json
sessions.json = 67,107,729 B ≤ 67,108,864 B cap  (99.998%; pre-fix 72,262,482 B = 108.9%)
entries kept: 4,375 of 5,293 (oldest-first eviction to fit the cap — disclosed behavior)
dirs: 2,856 listings, 4,091,271 B ≤ 8,388,608 B sub-budget
```

A second (warm) run's overview differs from the first only in `generated_at` and the
live-growing `pi`/`hermes_db` token totals — the corpus is live (this very session
appends to it), not a determinism defect; the `--demo` determinism surface stays
pinned byte-identical by the `check-deterministic-output` gate (rc0).

**Stale-doc riders (same change):** `docs/decisions/2026-09-14-cycle-7-batch-selection.md`
item 4 "Latent today (15.5 MB vs 64 MiB)" → no-longer-latent with the live 108.9%
measurement and the rm-298 correction; `docs/stewardship/2026-09-03-cycle6-independent-review.md`
F2 gained a dated "[Resolved …]" annotation noting both suggested fixes landed, the
predicted failure mode occurring in between, and the drifted line citations.

## CU-2 — rm-301: stdout purity for `-f json` under side-effect flags

**Changes** (`crates/agenttrace-cli/src/main.rs`):

- New `write_stderr` mirroring `write_stdout`'s `BrokenPipe` → `Ok(())` tolerance.
- A format-aware `announce` fn-pointer next to the side-effect blocks: under
  `-f json` the four announcements (`Session cache cleared.`,
  `Downloading pricing from LiteLLM...`, `Loaded N model prices`, `Cache saved: <path>`)
  route to **stderr**; every other format (human default) keeps them on stdout,
  byte-identical to before.

**Test:** `crates/agenttrace-cli/tests/entrypoints.rs::clear_cache_json_stdout_is_a_single_json_document`
— **red-first** (pre-fix stdout began `"Session cache cleared.\n{"`); post-fix it
asserts stdout parses as one JSON document via `serde_json::from_str`, the
announcement is on stderr, and the human path still prints it on stdout. Sandboxed
HOME/XDG_CACHE_HOME/AGENTTRACE_SESSION_CACHE_DIR so `--clear-cache` cannot touch any
real cache artifact.

**Live PoC (release binary):** `--clear-cache --overview --demo -f json | python3 -c 'json.load(sys.stdin)'`
→ parses, version reported; announcement on stderr; human invocation prints it on
stdout.

## CU-3 — rm-303: CHANGELOG section for every tag merged into HEAD

**Changes:**

- `CHANGELOG.md`: backfilled `## v0.8.1 - 2026-09-06` as an explicitly
  annotated-retroactively section (delta = TUI navigation/feedback/loading progress,
  PR #283 / `a34dea2`; release-page link; cross-referenced to the existing v0.9.0
  backfill note that carries the full v0.8.0–v0.9.0 release notes).
- `scripts/ci/check-plugin-version.sh`: new per-tag arm — every `^v` semver tag
  `git tag --merged HEAD` must have a `## vX.Y.Z` heading (ERE-escaped tag,
  `( |$)` boundary so v0.8.1 never matches v0.8.10) or an explicit one-line
  `<!-- no-changelog-section: vX.Y.Z: reason -->` marker. Lineage-scoped and
  no-tag-fallback, matching the landed rm-014/rm-163 anchor's semantics
  (a clone without tags stays green; lineage-foreign tags are ignored).

**Red/green proof** (synthetic fixture `/tmp/at-impl-cfee1fa4-fx/fx`, script at its
required `scripts/ci/` path, both `v0.1.0`/`v0.2.0` merged, plugin 0.2.0):

- only a `## v0.2.0` section → **rc1** naming `v0.1.0` and both remedies;
- `<!-- no-changelog-section: v0.1.0: … -->` marker → **rc0**;
- real `## v0.1.0` section (newest-first ordering, as the pre-existing latest-heading
  rule pins) → **rc0**;
- the real tree (v0.8.1 + v0.9.0 both sectioned) → **rc0** with the standard message.

## Validation matrix (focused/impacted only, per the phase budget)

| Check | Result |
| --- | --- |
| `cargo fmt` (then `git diff` review) | clean |
| `cargo clippy --all-targets --all-features -- -D warnings` | rc0 |
| `cargo test -p agenttrace-core` | 128 + 7 green |
| `cargo test -p agenttrace` (cli, incl. new entrypoints test) | 72 + 20 + 11 + 9 + 2 green |
| `cargo build --release` | rc0 |
| `check-deterministic-output.sh` / `check-output-contract.sh` / `check-plugin-version.sh` (new binary) | all rc0 |
| Live corpus cache-cap gate | 67,107,729 B ≤ 67,108,864 B |

## Commit-gate adjacency (fresh census at phase end)

Lanes drifting the files this batch touches (`git worktree list` + per-lane
`diff --name-only`):

- `session_cache.rs`, `check-plugin-version.sh`, `CHANGELOG.md`: **no other lane** —
  this batch owns them outright this cycle. (Memory of run 16787d74's rm-368 targeting
  session_cache.rs writer/load-path lines refers to a lane not currently drifting in
  this worktree set; re-verify by TITLE at merge time if it resurfaces.)
- `crates/agenttrace-cli/src/main.rs`: 7 other lanes (`0d487394`, `52465b9e`,
  `5de3ef1c`, `a1cafb4c`, `cf755698`, `d675a177`, `e602bb69`). The stewardship
  request's hunk census showed none intersecting `:206-218` (the side-effect block);
  this batch's hunks are the `write_stderr` insertion above `write_stdout` and the
  `announce` pointer + four `announce(...)` call swaps inside that block. Re-check
  hunk adjacency at merge time by function, not line number.
- `crates/agenttrace-cli/tests/entrypoints.rs`: shared with `a1cafb4c` and
  `d675a177` — all known edits are whole-test appends at different units; this
  batch's test is one appended unit at file end.
- `ROADMAP.md`: this run's own uncommitted mint (rm-298..rm-303, from the roadmap
  phase) is the same file this phase flipped `candidate → implemented` on for
  rm-298/rm-301/rm-303 — they ship together; the sibling renumbering/merge rules in
  the mint's header note still apply at integration.
- Also dirty from CU-0 and untouched here: `docs/stewardship/2026-10-03-cycle1-prioritization.md`
  (untracked deliverable of the prioritize phase, rides along).

## ROADMAP bookkeeping

rm-298 / rm-301 / rm-303 flipped `candidate → implemented` with dated
"CYCLE-1 IMPLEMENTED pre-review" evidence appends (red-first results, live numbers,
validation matrix, and the uncommitted-awaiting-commit-phase status). No other ids
were touched; no new ids minted.
