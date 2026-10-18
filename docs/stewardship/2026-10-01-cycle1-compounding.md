# Compounding Record — Cycle 1: "Truthful sources, disclosed views, honest transport"

- **Run**: `e602bb6924664c24add4c5c453d72cdd` (repository-maintenance, cycle 1)
- **Phase**: compound (attempt `a227da7bba83403aa1a10069bcf29567`)
- **Base**: HEAD `9d88b36750a991bd1436dbbc91b4579c39003067`; batch left uncommitted for the commit gate.
- **Scope note**: PRE-REVIEW snapshot by design — review, commit, push/PR/CI outcomes happen after this phase; the next cycle's assessment carries them forward. Everything below is built only from this cycle's assessment, research, roadmap, prioritization, implementation, and test outcomes.
- **Post-review correction** (2026-10-01, review:fix `e440865d`, per independent_review `3b4a8e1e` findings 1/2/6): lessons 1–3, the §2 F3 row, and §5 item 3 were corrected to name the mechanisms that actually shipped — the first draft referenced `PRICING_STALE_CACHE_BYPASSES` + `pricing_source_note()` (never implemented) and described `append_audit_coverage()` as a shipped shared trailer helper (it does not exist; the pre-existing `attach_audit_coverage` is the JSON governance-coverage helper).

## 1. Cycle outcome at a glance

| Item | Title | Status |
|---|---|---|
| rm-079 | Prefer the fresher of cached vs bundled pricing catalog | implemented (EXECUTED bullet appended) |
| rm-080 | Disclose bounded-list truncation in sessions-style reports | implemented (EXECUTED bullet appended) |
| rm-081 | Probe npm registry in-process; diagnose missing git correctly | implemented (EXECUTED bullet appended) |
| rm-082 | Catch up statusline schema wave 2 | candidate (minted) |
| rm-083 | Stream session parsing with bounded buffers | candidate (minted) |

Validation record (pre-review): `cargo clippy --workspace --all-targets -- -D warnings` clean; targeted gate fell back to authoritative full validation and passed; full gate `cargo test` **276/276 across 10 suites** (targeted-tests `3f7bbeac` + full_tests `0e4834f4`, envelope `result-3481121-331711820.json`); dispatch digest `validation:v1:a1fea75b…` verified unchanged pre- and post-gate. Assessed baseline was 264; the +12 delta is this cycle's tests (pricing precedence suite, truncation-disclosure suites, upstream transport/preflight suites).

## 2. Assess-findings disposition (assess `a1d45e89`)

| Finding | Disposition |
|---|---|
| F1 — TUI suite non-hermetic: `efficiency_panel…` reads the real `~/.cache/agenttrace/statusline.jsonl` under real HOME (44/45 through the gate) | **Fixed** in targeted_tests (warm–clear–re-render in `crates/agenttrace-tui/src/tests.rs`) + compounding rule → **Rule 6** |
| F2 — `upstream status` misdiagnoses git-absent as "not inside a git repository" (`upstream.rs:93`), npm probe shells to curl (`:316`) | **rm-081 EXECUTED** (in-process ureq probe + `repository_preflight()` split) |
| F3 — `--sessions` silently truncates at limit 20, bare array, no N-of-M (`main.rs` render path) | **rm-080 EXECUTED** (JSON `total_matches`/`truncated` + inline N-of-M trailer at three call sites; extracting the shared helper is deferred to the rm-011 lane) |
| F4 — raw hostile strings in `--delivery-evidence` / `--latest` text arms | **Not this lane** — class owner is the shared-sanitization lane (run-52465b9e, `ce27969`, IMPLEMENTED with `--sessions` re-verification); folded there, do not re-open independently |

## 3. Reusable lessons

1. **A staleness label without a comparison is a lie waiting to happen** (rm-079). Printing a cache's age was not disclosure: the bundled snapshot was newer. Fix pattern worth copying: persist the date *with* the data (`_snapshot.date` stamped into the payload on write — `stamp_snapshot_date`, no sidecar file), compare at every load (`freshest_catalog`: the bundle wins only on strictly-older; an equal-date cache still wins), and make the precedence decision observable from user-facing surfaces (`--doctor` via `pricing_provenance_line()` printing both dates; `--list-models` via the `bundled-newer:<date>` source label) — the bypass is disclosed, not counted. Applies directly to rm-082 (schema `dateModified`) and any future "how old is this?" surface.
2. **One wording, every bounded view — helper still to be extracted** (rm-080). rm-080 shipped identical N-of-M wording at three call sites (inline trailer in `render_session_list`, `latest_with_coverage_json`, the `(latest of N matching sessions)` lead); there is deliberately **no shared trailer helper yet** (`append_audit_coverage()` never existed; the pre-existing `attach_audit_coverage` is the JSON governance-coverage helper), and `--delivery-evidence` gained no coverage line this cycle. Future bounded lists — search-JSON disclosure (rm-011), per-tool tables — should extract that one helper and reuse it rather than minting new phrasing; in-repo consumers (like `scripts/record-real-marketing.sh`) now *fail loudly* when a disclosure key is absent, which turns contract drift into a CI error instead of silent data loss.
3. **In-process transport, and never conflate "tool absent" with "state absent"** (rm-081). The binary already links ureq; shelling to curl was a portability defect. And the git-absent/not-a-repo conflation cost the assessment phase a wrong-path debugging session — spawn-failure taxonomy (`repository_preflight()`) must precede any state claim. Both arms now have hermetic tests (the nonexistent-binary + synthetic-error arm is portable to every platform; the PATH-scrubbed spawn arm is `#[cfg(unix)]`).
4. **Host-state hermeticity is a test-design rule, not an env-var ritual** (→ Rule 6). The gate runs `bash -c 'cargo test'` under the caller's real `HOME`; a suite that only passes with an isolated cache env is not green. Rule 6's warm–clear–re-render pattern makes absence assertions deterministic *without* mutating process-global env (so Rule 1's lock stays untouched).
5. **Partial-work recovery after mid-flight envelope deaths.** Two phases this cycle (roadmap `d0721c50`, implement `aae0d83b`) had first attempts die on provider 429s after landing sound partial work. The audit-then-complete pattern — verify the landed edits' soundness, keep them, complete the remainder, add a chain-of-custody note naming both attempts — is cheaper and safer than redo-from-zero, and it is now the recorded precedent (see the roadmap-phase comment in ROADMAP.md and the implementation record's header).
6. **Minting discipline that paid for itself.** The dedup-wall across all sibling worktrees plus the deliberately-not-minted ledger (ROADMAP.md, 2026-10-01 comment block) prevented double-claiming the integration lane's cross-session/fork dedupe work and kept rm-052/rm-072 boundaries crisp inside rm-081's claim. Keep both artifacts in the mint workflow.

## 4. Prevention rules landed this cycle

- **Rule 6 — "Assert absence by construction, never by host luck"** in `docs/maintainers/test-flake-prevention.md`, with the cycle-8 case study (44/45 through the gate on a host with a real 2.4 MB journal; 45/45 × 5 + 276/276 after the fix) and the operational corollary: green claims must come from a run under the gate's real environment, not only the isolated env the tests were written in.

## 5. Next-cycle candidates and context (ordered by readiness)

1. **rm-082 — statusline schema wave 2** (candidate). Schema extraction artifact is already on disk and dated (`statusline-schema-2026-10-01.json`, 21 keys, upstream `dateModified` 2026-09-29); wall grep verified zero sibling claims on `pr.number`/`worktree.name`/`vim.mode`/`review_state`. Raw payload retention at `statusline.rs:86` means no data is lost — this is modeling + disclosure work, and it composes with (not duplicates) the OTel-export and subagent-attribution lanes via `prompt_id`/`agent.name`.
2. **rm-083 — streamed parsing with bounded buffers** (candidate). Technique references fetched live from ccusage main (BufReader line streaming, 128 KiB buffer). Be honest in prioritization: this host's largest transcript is 1.9 MB — the win is worst-case robustness and RAM ceilings (16 workers × whole-file × retained events), not typical-case latency.
3. **rm-011 — report-contract truthfulness batch** (candidate, explicit cycle-2 anchor from prioritization `a818550b`/`a01e3f43`): search-JSON truncation disclosure (reuse the `# showing N of M` wording — extracting the still-nonexistent shared trailer helper belongs in this item), `rates_per_million_usd.total` computed-or-removed, unknown-reason fallback labels in every locale. Estimated ≈ a full session; three sub-defects each need golden tests.
4. **GATED — cross-session / fork-copied usage dedupe** (research RC-1, P3): claimed by the unmerged integration lane `integration-9c6b6f4e1107` (their rm-048). Do **not** mint or implement without reconciling with that lane. Fold-in evidence preserved in the ROADMAP not-minted note: 61 % of opencode input tokens appear in both parent and fork rows; forks carry 96 % of reported opencode cost while parents are cost-zeroed; ccusage shipped the exclusion as #1782; and this host's GLM-proxy fleet reuses `msg_stub_1` across 9 responses, so a dedupe key must bind content/timestamp and copies must be *disclosed*, not silently merged.
5. **rm-007 — pricing snapshot cadence ownership** (candidate): mechanism landed cycles ago; the gap is cadence + user-visible drift. Fresh drift datum already appended to its evidence line: codex ≥0.159.1's default model `gpt-6.1-sol` is absent from the bundled 2026-09-13 snapshot, so current-stable codex default-model sessions price at the $3/$15 built-in default (≈1.5× overstatement vs live $2/$10).
6. **Environment/lane context for the next assessment**: upstream frozen at 41ac231 (4th consecutive probe, issues #279/#278/#272/#259/#237/#236/#103 unchanged); sibling shared-sanitization lane owns hostile-string rendering (F4); this run's rm-IDs are campaign-local until merge — ID reconciliation is the commit gate's concern (renumber precedents: `c7dca75d`, `c028a7e5`).

## 6. Evidence index

Spool (delegate artifacts): assess `assess-a1d45e8966ec402394efa195ffa0ee09/2026-10-01-cycle1-adversarial-assessment.md` · research `research-6dc2900d57e84d1b9264169f9fab07e4/` (candidates, statusline schema, ccusage patch, probe log) · roadmap `roadmap-d0721c50b3f8414583dd273d848450df/` · prioritize `prioritize-a01e3f43bfd44d20ba529ff9ba20ba88/` · stewardship `stewardship-240d8960884246048c01d42f62776df1/` · implement `implement-aae0d83bc6af4d9181c30c1a2f1b2838/` · targeted-tests `targeted-tests-3f7bbeacf54d41d89e0ff86c15e2e6d8/` · full-tests `full-tests-0e4834f498ea4cee9ae8af669e144047/` (run log + gate envelope).

In-repo: `docs/stewardship/2026-10-01-cycle1-implementation-record.md` (batch detail) · `docs/maintainers/test-flake-prevention.md` (Rule 6) · `ROADMAP.md` (EXECUTED bullets, not-minted ledger, rm-082/083 candidates) · `README.md` / `docs/guides/governance-reports.md` / `docs/guides/upstream-status.md` (contract updates shipped with the batch).
