---
ce-handoff: v1
cwd: /home/agent/.hermes/conductor-worktrees/agenttrace-80c75f65b7/run-792ef47bdeaf-792ef47b
repository: /work/projects/agenttrace
repo_root_sha: 9d88b36750a991bd1436dbbc91b4579c39003067
branch: conductor/run-792ef47bdeaf
head: 9d88b36750a991bd1436dbbc91b4579c39003067
---

# Cycle 1 implementation record — count every session, disclose every artifact

- **Date**: 2026-10-01
- **Cycle**: 1, campaign `1f5ad3cf` (batch: rm-084 lead + rm-086 rider)
- **Base**: `9d88b36` (pre-existing uncommitted in worktree at start: `ROADMAP.md` +45 roadmap-phase mint, preserved)
- **Worktree**: `conductor-worktrees/agenttrace-80c75f65b7/run-792ef47bdeaf-792ef47b`
- **Selection**: `docs/stewardship/2026-10-01-cycle1-prioritization.md`
- **Status**: implemented; uncommitted worktree state (commit/push/PR deliberately out of scope)

## What shipped

| # | Item | Lineage | State |
|---|------|---------|-------|
| 1 | rm-084 pi-fork + pi-profile session-home discovery | research 5b2d04a1 RC-1 (+ cad2c25d profile fold) | done, live-verified |
| 2 | rm-086 at-rest artifact disclosure + `--clear-cache` parity | assess 5d6eb40c F2 (+ ce360ed1 F3 fold) | done, live-verified |

## rm-084 — Discover pi-fork and pi-profile session homes

Root cause: `discovery.rs` registered exactly three pi-family homes (`~/.pi/agent/sessions`, `~/.config/pi/agent/sessions`, `~/.omp/agent/sessions`), so fork homes (`~/.senpi`, `~/.omo`) and profile-relocated agent dirs (`~/.pi/<profile>/sessions`, e.g. `PI_CODING_AGENT_DIR=~/.pi/agent-cliproxy-only`) were invisible to default discovery; and `parser.rs::pi_source_for_path` labelled every non-default root `oh_my_pi`.

Changes:

- `crates/agenttrace-core/src/discovery.rs` — new `pi_family_known_session_dirs(&home)` enumerates every pi-family home structurally: for each root (`~/.pi`, `~/.omp`, `~/.senpi`, `~/.omo`) it registers `<root>/agent/sessions`, `<root>/sessions` (fork homes whose agent dir is the root), and every `<root>/<child>/sessions` for children present on disk (profile/agent-dir variants), each labelled `Pi`, `Oh My Pi`, `Pi (senpi)`, `Pi (omo)` — child dirs append `(name)`. Dedup is path-based *before* the sessions suffix is stripped, so `<root>/sessions` and a literal `<root>/agent` child never collapse; per-dir session discovery is unchanged (same `sessions` layout contract as `~/.pi`). `known_session_dirs()` composes it via the existing `dirs.extend` pattern; `.pi` and `.omp` roots are handled by the new fn, so the old literal entries are gone.
- `crates/agenttrace-core/src/parser.rs` — `pi_source_for_path` now decides by the actual home root in the path (`.pi` root → `pi`, senpi/omp roots → their ids) instead of matching the single default suffix, fixing the XDG-`.pi` mislabel (`pi` was labelled `oh_my_pi`).
- `crates/agenttrace-core/src/reports.rs` — `tool_display_name` renders the new ids (`Pi (senpi)`, `Pi (omo)`).
- `crates/agenttrace-cli/src/main.rs` — `-d <DIR>` gained its missing doc comment, so `--help` no longer shows a blank line.
- `README.md` — supported-homes line names the fork homes.
- `crates/agenttrace-core/src/statusline.rs` — doc comment only: `statusline_capture_path` now states that `--clear-cache` removes the journal by explicit path via the session-cache artifact registry. No `.pi`-comment existed to correct; no behavior change.
- New golden test `crates/agenttrace-core/tests/pi_family_discovery.rs`: a 10-home fixture corpus (`~/.pi` default+profile, XDG `~/.config/pi`, `~/.omp` default+profile, `~/.senpi` both shapes, `~/.omo` agent/agent-dir/root-sessions), pinned for discovery (all 10 found) and attribution (XDG → `pi` not `oh_my_pi`; senpi → `pi_senpi`; omo → `pi_omo`), with an env-locking `with_home` helper modeled on `discovery_contract`'s (HOME + XDG_CONFIG_HOME + XDG_CACHE_HOME + XDG_DATA_HOME + AGENTTRACE_SESSION_CACHE_DIR under one serializing lock). **Proven RED first** against pre-change code: 3-of-10 discovered + XDG mislabel + senpi/omo mislabel.

Live verification (debug binary, this worktree): default `--overview` on this host went **5700 sessions / $4.4K / `Pi 199`** → **15338 sessions / $15.7K / `Pi 8.4K $8.0K` + `Pi (senpi) 1.4K $3.5K` + `Pi (omo) 34 $62.73`** (fresh cache under `AGENTTRACE_SESSION_CACHE_DIR`); `agenttrace -d ~/.senpi/agent-cliproxy-only/sessions --overview` now attributes `Pi (senpi)` (was `Oh My Pi`). No parser change was needed — the corpora are pi `version:3` transcripts.

## rm-086 — Disclose at-rest artifacts and align `--clear-cache`

Root cause: `PRIVACY.md` (7 lines) never named `~/.cache/agenttrace`, while the tool writes five artifact classes there; `clear_session_cache` removed only `sessions.json` + the two SQLite snapshots, skipping the statusline journal (`statusline.jsonl`, 10 MiB bound) and the pricing catalog (`pricing.json`).

Changes:

- `crates/agenttrace-core/src/session_cache.rs` — new `cache_artifact_paths()` is the single registry of what `--clear-cache` removes, built from the same constructors that write the files (`session_cache_path()`, `sqlite_snapshot_path("hermes"|"opencode")`, `statusline::statusline_capture_path()`, `pricing::pricing_cache_path()`) so each artifact is cleared where its own env-aware constructor says it lives; `clear_session_cache()` consumes it via `remove_cache_artifacts`. `pricing.json` was deliberately included (public LiteLLM metadata, no personal data, re-downloadable via `--update-pricing`; removal falls back to the bundled snapshot, no forced network).
- `PRIVACY.md` — rewritten: local-first statement, artifact table (file, data class incl. conversation-derived names/CWDs/touched-file paths, purge command), env relocation (`XDG_CACHE_HOME`, `AGENTTRACE_SESSION_CACHE_DIR`), and an explicit preserved-not-cleared row for `--preserve-history`'s `history.json` (user data, not a cache) plus the offline-by-default/`--update-pricing` network statement.
- Two unit tests in `session_cache.rs`: `clear_cache_removes_every_artifact_and_only_those` (registry length 5, every artifact removed, bystander file kept, second clear is a no-op) and `privacy_disclosure_lists_every_artifact` (PRIVACY.md must contain every artifact file name derived from the same constructors + `history.json` — a new store cannot ship undisclosed).

Live verification (sandboxed `HOME`): a `~/.cache/agenttrace` holding all five artifacts + `keep-me.json` → `agenttrace --clear-cache` empties it except the bystander.

## Gates recorded (focused/impacted only, per phase budget)

- `cargo test -p agenttrace-core --lib` → **101/0** (includes both new rm-086 tests)
- `cargo test -p agenttrace-core --test pi_family_discovery` → **2/0**
- `cargo test -p agenttrace-core --test discovery_contract` → **72/0**
- `cargo test -p agenttrace` (CLI crate: lib + entrypoints + launch_guards + upstream) → **41/0**
- `cargo clippy --workspace --all-targets -- -D warnings` → clean
- Host-state note: `discovery_contract`'s `rust_discovers_only_opencode_storage_session_files_like_go` and `default_discovery_uses_opencode_sqlite_when_present` fail if the *test process* carries `XDG_DATA_HOME` (the file's `with_home` helper pins HOME/XDG_CONFIG/XDG_CACHE but not XDG_DATA_HOME). Verified pre-existing at stashed HEAD 9d88b36 — not caused by this batch; both green when the runner env omits `XDG_DATA_HOME`. Full-suite (repository-wide) validation is left to the reserved full-tests gate.

## Files touched

Modified: `PRIVACY.md`, `README.md`, `ROADMAP.md` (status flips + compound bullets), `crates/agenttrace-cli/src/main.rs`, `crates/agenttrace-core/src/discovery.rs`, `crates/agenttrace-core/src/parser.rs`, `crates/agenttrace-core/src/reports.rs`, `crates/agenttrace-core/src/session_cache.rs`, `crates/agenttrace-core/src/statusline.rs` (comment).
New: `crates/agenttrace-core/tests/pi_family_discovery.rs`, `docs/stewardship/2026-10-01-cycle1-implementation-record.md`.

Fleet overlap note for the review/commit gates: sibling worktree `run-83642957` carries uncommitted additive-only `discovery.rs`/`parser.rs`/`statusline.rs` edits from the rm-055..064 lane — expect a merge conflict in `discovery.rs` (both add pi-family registration) and the `statusline.rs` comment; fold-time should keep this batch's structural enumeration and reconcile the sibling's additive entries against it.

---

<!-- INTEGRATION NOTE (2026-10-21, conflict case 4869dd99): both sides of this path are cycle-1 implementation records from DIFFERENT runs on 2026-10-01, each added at this one path (both-added). The record above is run 792ef47bdeaf's (rm-084 + rm-086, landed on this wall's lineage). The record below is run e602bb692466's (rm-079 + rm-080 + rm-081, the candidate merged by this case), preserved verbatim as that run's historical record. Its rm-079/rm-081 mechanics are superseded at this merge by the wall's later landed decisions — cache serving stays as-is regardless of age (the merged tree's stale_cache_is_served_as_is_without_download_or_rewrite pin), and the npm probe rides the wall's bounded-subprocess/curl machinery (UPSTREAM_* bounds, rm-052) — while rm-080's disclosure intent survives through the wall's rm-798 vocabulary (matched_sessions/returned_sessions/truncated/limit) — the record's own latest_with_coverage_json was likewise candidate-tree code, not landed HEAD, and `--latest` keeps the wall's landed design (the "No sessions match the requested filters" context), the match-set disclosure staying owned by the wall's open rm-011 lane. See the ROADMAP integration banner for the full seam-by-seam adjudication. -->

# Implementation Record — Cycle 1: "Truthful sources, disclosed views, honest transport"

- **Run**: `e602bb6924664c24add4c5c453d72cdd` (repository-maintenance, cycle 1)
- **Phase**: implement (attempt `aae0d83bc6af4d9181c30c1a2f1b2838`; prior envelope attempts died on provider 429s — this record covers the full batch at one working-tree state)
- **Batch**: rm-079 + rm-080 + rm-081 (selected by prioritize `a01e3f43`, contract in stewardship request `240d8960`)
- **Base**: HEAD `9d88b36750a991bd1436dbbc91b4579c39003067`, tree left uncommitted for the commit gate.
- **Post-review correction** (2026-10-01, review:fix `e440865d`, per independent_review `3b4a8e1e` findings 1/2/3/5/6): the rm-079/rm-080/rm-081 sections below were rewritten to name only APIs, flags, counts, and quotes that exist in the shipped tree. The first draft credited a `PricingCacheMeta` sidecar type, a `PRICING_STALE_CACHE_BYPASSES` counter, `pricing_source_note()`, a shared `append_audit_coverage()` trailer helper, a `--sessions-limit` flag, and a stderr trailer — none of which exist — plus a `--doctor` quote and test count that were never produced. Every anchor below was re-verified live at HEAD `9d88b36` (binary md5 `916f91d4…`).

## Deliverables

### rm-079 — cached-vs-bundled pricing precedence (P3)

**Defect**: `load_pricing_cache` (crates/agenttrace-core/src/pricing.rs) served the on-disk LiteLLM cache regardless of its age, and never compared it to the bundled snapshot date. A stale cache silently outranked a newer bundle: a 2026-08-01 cache kept serving `$9.9/M` input for `claude-sonnet-4-5` (3.3× the bundled `$3/M`) forever, and `--doctor` reported the stale date as the truth.

**Fix** (crates/agenttrace-core/src/pricing.rs — real mechanisms, anchors re-verified live at HEAD `9d88b36`):
- `stamp_snapshot_date()` (pricing.rs:433) stamps `_snapshot.date` (today, UTC) **into the downloaded payload itself** before `write_pricing_cache` persists it (`update_pricing`, pricing.rs:214; the write stays atomic temp+rename, pricing.rs:478-490). There is **no sidecar meta file and no `PricingCacheMeta` type** — the date lives inside `pricing.json`.
- `cached_snapshot_date()` (pricing.rs:415) reads the date from the payload's `_snapshot.date`, falling back to the file's **mtime date** for pre-rm-079 caches that carry no meta (the live LiteLLM registry embeds none — that is why agenttrace stamps its own).
- `freshest_catalog()` (pricing.rs:340) enforces precedence: the bundle is served only when the cache's snapshot date is **strictly older** (`if cache_date < bundled_date`, pricing.rs:346); an **equal-date cache still wins** — live-confirmed with a stamped 2026-09-13 cache served over the identical bundled date. There is **no bypass counter** (`PRICING_STALE_CACHE_BYPASSES` and `pricing_source_note()` were never implemented): the bypass is *disclosed*, not counted.
- `catalog_source()` (private, pricing.rs:158) labels the bypass `bundled-newer:<date>`, rendered as `LiteLLM snapshot 2026-09-13 (bundled; newer than the cached catalog from <date> — run --update-pricing to refresh)`; labels stay clock-free (deterministic-output rule).
- `pricing_provenance_line(demo)` (new, exported, pricing.rs:105) renders the doctor-facing `Pricing snapshot:` line and discloses **both** dates when cache and bundle disagree (`cached catalog snapshot X vs bundled Y — the fresher one is served`). `--doctor` (crates/agenttrace-core/src/doctor.rs:133, rendered at :363) now uses it, and `--demo` consults no host state at all (live output: `serving bundled snapshot 2026-09-13 (2755 models, 18 days old) (demo; host cache not consulted)`).
- Live quotes (review:fix `e440865d`, scratch HOME, binary md5 `916f91d4…`): backdated undated cache mtime 2026-08-01 → `Pricing snapshot: serving LiteLLM snapshot 2026-09-13 (bundled; newer than the cached catalog from 2026-08-01 — run --update-pricing to refresh); bundled snapshot 2026-09-13 (2755 models, 18 days old); cached catalog snapshot 2026-08-01 vs bundled 2026-09-13 — the fresher one is served`; stamped 2026-09-20 cache → `serving LiteLLM (cached catalog); …; cached catalog snapshot 2026-09-20 vs bundled 2026-09-13 — the fresher one is served`.

**Tests** (pricing.rs `mod tests`, **13 total**; a `pricing`-filter count of 15 additionally sweeps 2 governance tests elsewhere in the core lib): both disagreement directions (`bundled_snapshot_beats_older_cached_catalog` — 10-day-backdated cache loses; `stamped_snapshot_date_beats_mtime_in_precedence` — a stamped date outranks a backdated mtime); stamp additivity (`stamp_snapshot_date_adds_meta_without_disturbing_models`); both-dates disclosure (`provenance_line_prints_both_dates_when_catalogs_disagree`); clock-free labels (`pricing_source_labels_carry_no_wall_clock`); legacy undated-cache tolerance (`stale_cache_is_served_as_is_without_download_or_rewrite`, 7-day mtime); pinned bundle date (`pricing_snapshot_date_is_pinned_to_the_bundled_payload`); no-cache arm (`catalog_uses_bundled_snapshot_without_cache_and_never_writes_one`).

**Warm-host cost caveat** (independent_review `3b4a8e1e` finding 4): the `$0.0690→$0.0210 (3.3×)` correction was observed on a session-cache-COLD fixture only. The session cache's freshness check keys on source mtime+size alone (`is_fresh`, crates/agenttrace-core/src/session_cache.rs:1041-1047, and the header check at :556), so on a host with a warm `sessions.json` previously-analyzed sessions keep serving their frozen `CostEstimated` after a precedence flip or `--update-pricing` until `--clear-cache` (review live-repro: warm cache + catalog flip kept 0.0105 until `--clear-cache` → 0.0594). Follow-up candidate for a later cycle: fold the pricing-catalog identity (source label + snapshot date) into session-cache entry validity, or recompute cost at render from cached tokens.

### rm-080 — `--sessions` truncation disclosure (P4)

**Defect**: `render_session_list` (crates/agenttrace-cli/src/main.rs) emitted `serde_json::to_string_pretty(&sessions[..limit])` — a bare array with no way to tell 20-of-20 from 20-of-5 688. The default limit (20) silently hid thousands of sessions, and `-f json` consumers could not detect truncation programmatically. `--latest` (single-session view) never said whether it was the true latest match or the newest within a truncated window.

**Fix** (crates/agenttrace-cli/src/main.rs — real mechanisms, anchors re-verified live at HEAD `9d88b36`):
- `render_session_list` (main.rs:1084) now: JSON → object `{"count": shown, "sessions": [...], "total_matches": N, "truncated": bool}`; text → an **inline stdout trailer** appended after the table when truncated (main.rs:1119-1124): `# showing {shown} of {total} matching sessions (--limit {limit}; raise --limit for the full view)` — present only when `total_matches > limit`. The real flag is `--limit` (main.rs:137, default 20; **there is no `--sessions-limit`**), and nothing moves to stderr.
- `--latest` discloses the same facts at two more call sites: `latest_with_coverage_json` (main.rs:1128-1140) inserts `total_matches`/`truncated` **additively** into the JSON report object (every original key survives), and the text arm leads with an inline `(latest of N matching sessions)` line (main.rs:395-398).
- **No shared trailer helper was extracted this cycle** (`append_audit_coverage()` as described in the first draft does not exist): the pre-existing `attach_audit_coverage` (main.rs:654) is the JSON governance-coverage field helper (`audited_sessions`/`total_sessions`/`excluded_reason`, used by `--delivery-evidence`/`--compare --sample`) and was untouched; `--delivery-evidence` (main.rs:305) gained no coverage line. Extracting one shared N-of-M helper remains open work for the bounded-view family (rm-011 lane).
- The doc surfaces are the README `--limit` paragraph (extended in place with the "Bounded views disclose their bound" sentences) and the existing coverage paragraph in docs/guides/governance-reports.md (extended in place to fold in the `--sessions`/`--latest` disclosure) — there is **no** new "Truncation disclosure" section and **no** `--sessions`/`--latest` contract-table row pair (round-2 review F2: those two named structures never existed); scripts/record-real-marketing.sh (the only in-repo JSON consumer) now reads `total_matches` and fails loudly if the key is absent.

**Tests** (bin suite **28** = 18 in main.rs `mod tests` + 10 in upstream.rs): `session_list_json_discloses_total_matches_and_truncated` (object shape, `--limit 20` → count 20 / total_matches 25 / truncated true; limit 50 → truncated false, all 25 present); `session_list_text_discloses_truncation_with_n_of_m_line` (trailer `# showing 20 of 25 matching sessions (--limit 20` present when truncated — header + 20 rows + trailer = 22 lines — and absent when not); `latest_view_discloses_match_count_in_both_arms` (additivity + the `(latest of N matching sessions)` lead); `audit_coverage_fields_are_pinned_for_bounded_views` (pins the pre-existing governance coverage fields, including bare-array wrapping).

### rm-081 — npm probe via in-process ureq + git-absent diagnosis split (P4)

**Defect** (crates/agenttrace-cli/src/upstream.rs): the `upstream status` npm freshness probe shelled out to `curl` (`Command::new("curl")`), so on hosts without curl the probe failed with a misleading transport error; worse, when git itself was missing, `upstream status` reported `Error: not inside a git repository` on a perfectly valid checkout — a false diagnosis that sent the assess phase down the wrong path (assess F2).

**Fix** (upstream.rs):
- npm registry probe rewritten on the in-process `ureq` client already used for `--update-pricing` via `agenttrace_core::http_get_text` (`crates/agenttrace-cli/src/upstream.rs:366-367`, **15 s** timeout), removing the curl dependency entirely; module doc updated to match.
- `repository_preflight()` (new) is called before any git work and distinguishes: git binary absent (bails with the actionable `git is not available ({err}); 'agenttrace upstream' drives the git CLI — install git or extend PATH`) vs not-a-repo (bails with the plain `not inside a git repository`). (Round-2 review F3: the earlier draft's "install git or run from a machine that has it" / "run inside the agenttrace checkout" wordings were never the shipped strings.) The conflated `not inside a git repository` message can no longer be produced by a missing binary.
- npm state strings (`npm: vX.Y.Z (N minor versions behind)`) unchanged, so downstream parsing is untouched.

**Tests** (upstream.rs `mod tests`): `repo_probe_classifies_spawn_failure_as_git_unavailable` drives the spawn-failure arm **portably** (nonexistent binary + synthetic raw `io::Error` — upstream.rs:654-671); `repo_probe_classifies_git_exit_failure_as_not_a_repository` (real git on a pristine temp dir — only this outcome may say "not inside a git repository"); `git_absent_from_path_is_not_reported_as_not_a_repository` exercises a real PATH-scrubbed spawn on a valid checkout, reproducing assess F2 — `#[cfg(unix)]` (upstream.rs:701), so the unix-gated arm does not run on Windows while the two portable arms do; `npm_version_from_body_parses_registry_documents` (hermetic parsing half); plus npm state renderers and preflight message wording.

## Sibling-lane boundaries respected

- The `--sessions` output change stays inside the render layer; the shared-sanitization lane (run-52465b9e, `ce27969`) owns escape/neutralization and its `--sessions` re-verification passes were against text mode, which keeps its payload layout here.
- rm-052 (label split for npm state) is untouched — only the transport underneath changed.
- rm-072 (upstream divergence framing) untouched; `--fetch upstream` messaging beyond the git-absent arm is unchanged.

## Verification

- `cargo clippy --workspace --all-targets -- -D warnings` — clean at final state.
- `cargo test -p agenttrace-core --lib` — pricing filter 15/15 (13 in pricing.rs `mod tests` + 2 governance tests matching the filter), all green (no doctor failures).
- `cargo test -p agenttrace-cli` — 28 bin + 10 integration + 2 + 9, all green.
- Live probes (binary in `target/debug`): older-than-bundled cache loses on `--doctor`/`--list-models` with both dates disclosed (rm-079); 25-session fixture shows `# showing 20 of 25 matching sessions (--limit 20; raise --limit for the full view)` + JSON `{"count": 20, "total_matches": 25, "truncated": true}` (rm-080); `PATH=/tmp/…-nobin agenttrace upstream status` now names git as absent, and `--fetch upstream` completes its npm probe with **no curl on PATH** (rm-081).
- Determinism/CI scripts: `scripts/record-real-marketing.sh` updated (reads the new key); `scripts/check-*.sh` untouched surfaces confirmed by grep; `real-cli-smoke` requires only valid JSON + rc 0, which the object shape satisfies.
