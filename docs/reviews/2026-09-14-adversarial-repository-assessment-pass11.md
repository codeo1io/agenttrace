# Adversarial repository assessment — pass 11

- run: 125bf93302aa4e308cb0739b67f16f33
- phase: assess (action `assess:assess`), attempt 6002593cdffe4732b5ecfa6a378389e6
- tree: HEAD df3b621 (`docs: record cycle-6 closures and pass-10 findings in the roadmap`), working tree clean
- date: 2026-09-14

## Method

Fresh pass, not a restatement of passes 1–10. Baseline re-verified first, then
under-examined surfaces (install-channel modes, cache-bound accounting under
duplicate keys, TUI thread teardown, roadmap-vs-code traceability), then
hostile-input probes, then re-verification of every still-open prior finding
with live reproductions rather than citations.

## Baseline (all re-run this pass)

- `cargo test --workspace`: 213/213 pass.
- `cargo clippy --workspace --all-targets`: exit 0, no warnings. `cargo fmt --check`: clean.
- All 10 `scripts/ci/check-*.sh` exit OK (logs under `/tmp/adv11/ci-out/`).
- Release binary rebuilt from HEAD and used for every live probe below.

## Verified-clean surfaces (no finding)

- Old findings confirmed fixed and still fixed: F8-1 sampling disclosure
  (`audited_sessions`/`excluded_reason`), F8-3 `prune_dead_entries`,
  F8-5 `json_float→null`, F8-6 single percentile, F8-7 docs schema,
  F8-8 README "Flags go before the session path" warning (README.md:154).
- Hostile SQLite (BLOB values, `i64::MAX` rows, zero timestamps) → parsed with
  unknown-time bucketing, no panic, no wraparound.
- Hostile JSONL (`<script>`, `onerror`, quotes) → HTML reports escape `&<>"'`
  consistently (reports.rs `escape_html`).
- `npm/scripts/install.js` platform/arch mapping, checksum gate, and 0o755
  chmod logic correct; codex plugin version pinned to CHANGELOG by
  `check-plugin-version.sh`; winget/homebrew manifests consistent with
  CHANGELOG version.
- TUI math uses saturating arithmetic throughout (app.rs, explorer.rs,
  presentation.rs); `waste.rs` divisions guarded; sqlite SQL built via
  `format!` only interpolates validated column names.

## New findings this pass

### A11-1 — MEDIUM — cycle-4 F2 fell out of the roadmap tracking pipeline while its bug class was being fixed

`docs/stewardship/2026-09-02-cycle-4-final-validation.md:56` dispositioned
review findings F2–F7 as "inputs for the next cycle's roadmap/prioritize
passes — not silently dropped". F2 (the `--no-baseline-gate` value-flag
misregistration, A11-5 below) never reached the ROADMAP: the "CLI surface
polish" item (ROADMAP.md:722-740) tracks P3-7, P4-2, P4-3, and the N7
residue, but not F2. Meanwhile cycle 5 fixed the *same failure class* for
`--sample` and added a shim unit test
(`docs/stewardship/2026-09-03-cycle5-implementation-record.md:48-52`) without
fixing the original instance — so the exact known bug the class-fix was
modeled on is still live at `crates/agenttrace-cli/src/main.rs:726`. A
live-reproduced MEDIUM from a recorded review is currently untracked, which
is the exact "silently dropped" outcome the final-validation record claimed
to prevent. Fix: remove `"--no-baseline-gate"` from `flag_takes_value` (it is
a bool — `main.rs:99-100`), extend the existing shim unit test, and add F2 to
the roadmap item (or its closure note) so the paper trail is complete.

### A11-2 — MEDIUM — `enforce_byte_bound` double-counts entries present in both cache maps; over-evicts near the bound

`session_cache.rs:652-664` builds the eviction size list from **both**
`raw_entries` and `entries` by serializing each value. A path can live in
both maps at once: `cached_entry` (`session_cache.rs:549-564`) decodes a raw
value into `cache.entries` and leaves the `raw_entries` copy in place. Any
save that happens after cache hits therefore counts those paths twice —
typically ~2× the true payload once most entries have been decoded. The
eviction loop subtracts only one copy's bytes per dropped path
(`session_cache.rs:691-695`), so with true size T near the 64 MiB bound the
loop evicts until `2T − Σdropped ≤ max`, i.e. it can evict roughly twice the
intended amount — up to evicting nearly the whole cache when T ≈ max. Today's
operator cache (918 entries, ~15.5 MB) stays far from the bound, so impact is
latent, but the "hard bound" contract is violated in the over-eviction
direction, and this compounds cycle-6 review F2 (which found the bound
*under*-counts envelope/`dirs` overhead in the other direction). Same root
cause distorts `enforce_entry_bound` (`session_cache.rs:627-641`): duplicate
paths consume `take(drop)` slots, and the second `remove` no-ops, so a single
pass can drop fewer entries than the bound requires (self-corrects only on
the next save). Fix: iterate over the union of deduplicated paths, counting
`max(raw_size, entry_size)` per path, and drop-by-path once for both maps;
pin both behaviors with unit tests on a cache where a path exists in both
maps.

### A11-3 — LOW — `install.sh` installs the binary as mode 0711, not 0755

`install.sh:53-54` does `TMP=$(mktemp)` (creates 0600) then only
`chmod +x "$TMP"`, so the installed file lands as `rwx--x--x` (0711 here;
umask-dependent but group/other read is never granted) after the
`mv "$TMP" "$DEST"` at `install.sh:66`. The npm channel installs 0o755
(`npm/scripts/install.js`), so the two official channels disagree. An
executable-without-read binary works on local filesystems but breaks or
misbehaves on NFS-mounts, some container overlay setups, and prevents
`strip`/debugging by non-owners — a poor default for `/usr/local/bin`.
Fix: `chmod 0755 "$TMP"` before the move (and note that the roadmap's open
install.sh checksum item, P5-6/F8-9, is still unfixed in the same script).

### A11-4 — LOW — TUI delivery-evidence thread can end silently empty with no surfaced error

`app.rs:1513-1521` spawns the governance Delivery worker as
`thread::spawn(move || { let _ = tx.send(delivery_evidence_with_git(&sessions)); })`.
`poll_governance_delivery` (`app.rs:1536-1553`) treats `RecvError`
(Disconnected) as completion and leaves `snapshot.delivery = None`; the panel
then renders its empty state. If the worker panics before `send` (or the
channel is torn down), the user sees "no delivery evidence" with zero
diagnostic — indistinguishable from a project without git history. Low
probability today (`delivery_evidence_with_git` returns `DeliveryEvidence`,
not `Result`), but it is the only TUI async surface with no failure path.
Fix: send a `Result`/error variant through the channel and render it in the
panel (or at least trace it).

## Re-verified still-open findings (live evidence this pass)

- **A11-5 — MEDIUM — `--no-baseline-gate` misregistered as a value-taking
  flag** (cycle-4 IR F2; now also untracked, see A11-1).
  `crates/agenttrace-cli/src/main.rs:726` lists the bool flag
  (`main.rs:99-100`) in `flag_takes_value`, so any placement before other
  flags silently drops them. Live (release binary, cache sandboxed):
  - `agenttrace --no-baseline-gate -f json --overview` → drops both flags →
    TUI path → "stdout is not a terminal", exit 1.
  - `agenttrace --no-baseline-gate --baseline b.json --overview -f json` →
    drops `--overview -f json`, then **misleads**: `Error: --baseline
    requires --overview -f json`, exit 1 — telling the user to add exactly
    the flags they passed. `docs/guides/ci-integration.md:124-127` tells
    users to "Add --no-baseline-gate" to that very command line.
  - Inversion: `--no-baseline-gate file.jsonl -f json` honors `-f json`
    *after* the positional, contradicting the documented Go-flag contract
    that post-positional flags are ignored.
- **A11-6 — HIGH — discovery is `HOME`-only; every Windows channel ships a
  build that discovers zero sessions** (roadmap P3-1, still open).
  `crates/agenttrace-core/src/discovery.rs:52-54` requires `HOME`; grep finds
  no `USERPROFILE`/`cfg(windows)` anywhere in `crates/`. Live:
  `env -u HOME … --overview -f json` → `Error: No session files found in `
  (note trailing space), and `--doctor` shows an empty Providers list with no
  hint. Siblings: TUI `language_preference_path` (app.rs:1558-1567) can never
  save on Windows; history/cache dirs fall back to temp. Yet
  `.github/workflows/release.yml:58-66` publishes windows-amd64/arm64
  assets, README.md:73-79 advertises `npm install -g` and
  `winget install Luoyuctl.AgentTrace`, and npm install.js supports win32.
- **A11-7 — known, unchanged — delivery-evidence cost** (pass-2 N10):
  `governance.rs:779-789` still runs serial, unbounded-time `git -C <root>
  log --all` per root; root derives from untrusted session `cwd` fields.
- **A11-8 — known, unchanged — dead CI step** (N5/P4-5): the PTY TUI smoke
  remains gated on `env.AGENTTRACE_TUI_REAL_DIR != ''`, which is set nowhere.
- **A11-9 — known, unchanged — control characters** (P3-4) and
  **install.sh without published checksum verification** (P5-6/F8-9) remain
  open as recorded in the roadmap.

## Answers this pass contributes

- Prior-findings drift: none of the *closed* items regressed; the drift is in
  the open direction only (A11-1 shows one recorded finding escaping the
  tracking pipeline entirely).
- The `flag_takes_value` list is complete and correct for every flag except
  `--no-baseline-gate` (checked against all `#[arg]` definitions).

## Suggested next actions (priority order)

1. One-line fix + shim test for A11-5, with the roadmap note (A11-1).
2. Deduplicated-path accounting in `enforce_byte_bound`/`enforce_entry_bound`
   (A11-2) with tests for both directions of the bound.
3. Windows home resolver (A11-6) — biggest user-facing gap; acceptance is
   already written in the roadmap.
4. `chmod 0755` + published-checksum verification in install.sh (A11-3,
   A11-9).

No commits, pushes, PRs, or CI runs were made this pass.
