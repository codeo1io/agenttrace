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
