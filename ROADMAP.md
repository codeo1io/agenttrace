# agenttrace — Roadmap

> Autonomously maintained by the roadmap sync (reliability-first). Items cite reproducible codebase signals; acceptance is proven by cited evidence.

**Vision**: A reliable, customer-friendly repository advanced by evidence-cited roadmap cycles owned by the autonomy loop

**Pillars**: reliability work outranks customer-experience work; every roadmap item cites reproducible codebase signals; acceptance is proven by cited evidence, never claimed

## Fleet context

- dependents (changes here affect): (host), agent, maestro
- graph: evidence-derived (imports/refs/deploy surfaces); advisory

## Open items

> **cycle-1 roadmap extension (2026-10-01, run 266b6e2b, attempt 2c29316c):** minted rm-091..rm-099 from this run's assess ledger (delegate-spool `assess-a29abc450efd4861b0d649d0bbd37c2a/findings.md`, 11 findings, 2 live repros) and research ledger (delegate-spool `266b6e2b-research-candidates-a84f6fe5.md`, candidates C1-C10). Campaign base 90a4ef5 is 17 commits behind fork master 9d88b36; assess F1-F4 (Codex partial-rewind double-count, statusline-report raw ESC/OSC-52, absent advisory gate, unpinned install fallback) are all already fixed at master (07eba81 rm-012/rm-013, b898caa rm-010, 35cf7fb rm-009, fallback pin rm-051) and are therefore NOT minted here — rm-091 owns delivering them via integration. IDs minted past the live fleet wall rm-090 (grep of all 33 sibling worktrees under conductor-worktrees/agenttrace-80c75f65b7 this phase); the wall already carries two double-mint generations (rm-068..074 in run-1766ab4e vs run-40208f3d, rm-084..089 in run-b8d7db05 vs run-792ef47b) — reconcile at integration per docs/solutions/workflow-issues/roadmap-campaign-id-collision-at-integration.md, do not renumber here. NOT minted (claimed elsewhere, verified against the live wall): OTel GenAI semconv export (run-40208f3d rm-077 — note its watch target must repoint: upstream moved GenAI semconv to the semantic-conventions-genai repo), upstream-offline marker semantics at upstream.rs:63/:157 (rm-052, per e602bb69 rm-081's explicit scope note), dead ci.yml cargo-cache block and the 5h-block/dependency-refresh waves (master-era items). rm-002's evidence line below was annotated for truthfulness at this base (assess finding 6).

### Add test coverage for 1 untested module(s)
- id: `rm-002` | track: reliability | priority: 83.0 | status: candidate
- signals: reliability.no_tests:scripts/fixtures/make-adversarial-sqlite.py
- acceptance: Every module in ['scripts/fixtures/make-adversarial-sqlite.py'] has a corresponding test file with at least one passing test
- evidence: full suite green (python -m pytest -q) at HEAD; conductor validation digest validation:v1:<sha> recorded in the shipping PR
  - cycle-1 correction (2026-10-01, run 266b6e2b assess finding 6): the pytest suite does NOT exist at this base — `python3 -m pytest -q` reports "no tests ran in 0.09s" and no conftest.py/test_*.py is in-tree; the suite (scripts/fixtures/test_make_adversarial_sqlite.py) lands with fe8b316, which sits inside the 17-commit gap owned by rm-091. Re-establish this evidence after rm-091 merges; do not cite it for work shipped from 90a4ef5 alone.

### Refactor 3 high-complexity function(s)
- id: `rm-001` | track: reliability | priority: 79.0 | status: candidate
- signals: reliability.complexity_hot:npm/scripts/install.js::L24, reliability.complexity_hot:npm/scripts/install.js::L28, reliability.complexity_hot:npm/scripts/install.js::L40
- acceptance: Each flagged function is decomposed below the branch threshold with behavior locked by characterization tests
- evidence: ast-based branch-count check passes at HEAD (full suite green; conductor validation digest validation:v1:<sha> recorded in the shipping PR)

### Integrate the 17-commit fork-master gap before any new implementation

- id: rm-091
- track: reliability
- priority: 90.0
- status: candidate
- signals: process.stale_base:git log --oneline HEAD..origin/master = 17 commits at 90a4ef5, incl. be25c4c (#286 Codex double-count fix), 07eba81 (rm-012/rm-013 statusline sanitize + token high-water), b898caa (rm-010 cargo-deny gate), 35cf7fb (rm-009 rustls RUSTSEC-2026-0285 lock), fe8b316 (rm-021 upstream-drift subcommand + adversarial-sqlite pytest), 083963b (#285 crt-static), 9eba8cf (rm-004/rm-005 truthful diagnostics); integration.conflict_ahead:.github/workflows/ci.yml ubuntu-22.04 glibc pin duplicates upstream 9421e3a (#13); process.reimplementation_risk: the rm-092/rm-093/rm-094 defect sites exist only at master
- acceptance: campaign branch merges origin/master (9d88b36 or later) with zero manual re-implementations of already-landed fixes (verify `git merge-base --is-ancestor be25c4c HEAD` etc. for be25c4c, 07eba81, b898caa, 35cf7fb after the merge); the ci.yml glibc-baseline duplication resolves to one canonical lane; at the merged HEAD the assess R1 mixed-rewind Codex fixture reports input tokens 100000 (observed 160000 at this base) and the assess R2 poisoned statusline journal produces zero raw ESC bytes in `--statusline-report` output; full workspace suite green at the merge result; the ROADMAP.md merge records ID-collision reconciliation for the double-minted ranges
- evidence: assess-a29abc45 finding 5 (P2) with the live `git log HEAD..origin/master` gap at 90a4ef5; repro artifacts codex-rewind-repro.jsonl/.out.json and statusline-report-output-with-raw-esc.txt double as the merge's regression probes

### Stop codex_line_is_ignorable dropping lines whose nested values contain ignorable type strings

- id: rm-092
- track: correctness
- priority: 86.0
- status: candidate
- signals: correctness.predecode_drop:crates/agenttrace-core/src/parser.rs::L2261-2272 at master 9d88b36 (verified via `git show origin/master:`, filter call at :2094) — the pre-decode skip tests a 160-byte head window with substring `contains` (e.g. `head.contains(r#""type":"event_msg""#)`), so any Codex line carrying a NESTED `"type":"event_msg"`/`"type":"compacted"` key value in its first 160 bytes is dropped before JSON decode; introduced by be25c4c (#286); absent at this base — gated on rm-091
- acceptance: a line whose first 160 bytes contain a nested ignorable-type value (e.g. a tool payload echoing `{"type":"event_msg"}` as data) is still decoded and counted; the skip decision keys on the decoded top-level `type` (or an exact-byte anchored match), not substring-in-window; a regression fixture reproduces the observed silent undercount (input tokens 8 counted as 4) and stays green; suite green
- evidence: research candidate C1 (HIGH) citing the same-day independent aa9c4fd6 live repro (ledger assess-2d8f159a5c55483cab662e226a6ab5f6, input tokens 8→4, silent); wall grep 2026-10-01: unclaimed; master-only defect — do not implement before rm-091 lands be25c4c

### Sanitize the upstream subcommand's commit-derived terminal output

- id: rm-093
- track: security
- priority: 82.0
- status: candidate
- signals: security.terminal_injection:crates/agenttrace-cli/src/upstream.rs::L365-369 (base_subject) and :L385-388 (commit.subject in the unported-commits list) render via bare `format!` with no sanitize at master 9d88b36 (verified via `git show origin/master:`); the site was added by rm-024 (fe8b316) and missed the rm-012 sanitizer class; a same-day sibling live-proved an OSC-52 clipboard-write sequence (`ESC]52;c;c2hlbGw=BEL`) in a commit subject reaching the terminal via a hermetic file:// upstream remote (aa9c4fd6 ledger N2); the file does not exist at this base — gated on rm-091
- acceptance: every commit-derived string rendered by `agenttrace upstream` (base_subject, unported commit subjects) routes through the same sanitizer family as rm-012's sanitize_line_segment; a poisoned-remote fixture with SGR + OSC-52 payloads produces zero raw ESC bytes in captured output (cat -v clean); a unit test pins both render sites; suite green
- evidence: research candidate C2 (HIGH) + this run's assess R2 proving the identical defect class end-to-end in the statusline-report path at this base (artifacts statusline-poisoned-journal.jsonl, statusline-report-output-with-raw-esc.txt); boundary: run-1766ab4e rm-072 owns divergence semantics and trailing-arg handling, not injection — no overlap

### Restore project resolution for cwd-less transcripts lost with decode_agent_project_dir

- id: rm-094
- track: correctness
- priority: 76.0
- status: candidate
- signals: correctness.cwdless_unknown:crates/agenttrace-core/src/insights.rs::L166-176 — at master 9d88b36 resolve_project handles only session.cwd and decode_agent_project_dir(path) before yielding "unknown" (verified via `git show origin/master:`); the parent-dir fallback present at this base (Path::new(&session.path).parent(), insights.rs:171-176 at 90a4ef5, live-read this phase) was dropped, so transcripts with no cwd field (aider writes none) and no agent-project-dir-shaped path resolve to unattributed/unknown, breaking off-host and aggregate analysis; gated on rm-091
- acceptance: a cwd-less session whose path is a plain filesystem path resolves to that path's project root (or an explicit, tested resolution note) instead of "unknown"; the regression fixture is an aider-shaped transcript (no cwd, plain path); grouping is identical from two different invocation cwds; suite green
- evidence: research candidate C3 (MED-HIGH) + aa9c4fd6 finding N3 (affected tool: aider, a supported session source); boundary: run-b8d7db05 rm-087 owns preserved-history re-attachment (history.rs:139) and the master-era memoization item owns perf — this item owns the live-session cwd-less fallback axis only; wall grep 2026-10-01: unclaimed

### Bound the npm install path: download timeout/retry and verify-runtime spawnSync timeout

- id: rm-095
- track: reliability
- priority: 72.0
- status: candidate
- signals: reliability.unbounded_download:npm/scripts/install.js::L40-72 — download() has no timeout (node https default: none) and no retry, so a stalled CDN connection hangs `npm install` forever; reliability.unbounded_spawn:npm/scripts/verify-runtime.js::L18 — spawnSync(binaryPath, ["--version"], {encoding:"utf8"}) without a `timeout`, so a hung (vs failing) binary hangs install indefinitely; both read live at this base 90a4ef5; flagged independently twice on 2026-09-30 (this run's assess finding 8 + aa9c4fd6 N6)
- acceptance: download() enforces a bounded timeout with at least one retry and surfaces a typed error naming the URL on abort; verify-runtime passes an explicit spawnSync timeout and maps expiry to a clear failure message; a simulated-stall test (local http server that never responds) proves the install path aborts in bounded time instead of hanging; offline install checks stay green
- evidence: research candidate C4 + assess finding 8 (P3); wall grep 2026-10-01: unclaimed on the npm-scripts axis (run-e602bb69 rm-081 owns the Rust upstream.rs registry probe — different file and language; master-era rm-050/rm-051 own sha256/pinning axes)

### Split CI concurrency so the nightly schedule cannot cancel push runs

- id: rm-096
- track: reliability
- priority: 64.0
- status: candidate
- signals: reliability.shared_concurrency:.github/workflows/ci.yml::L14-16 — `concurrency: group: ci-${{ github.workflow }}-${{ github.ref }}` with `cancel-in-progress: true` is shared by push(master) and the nightly schedule on the same ref (verified identical at this base and at master 9d88b36 via `git show`), so a scheduled run cancels an in-flight push run and vice versa
- acceptance: schedule events get a distinct concurrency-group infix (e.g. `ci-nightly-${{ github.ref }}`); push runs are never cancelled by schedule and vice versa; an actionlint/CI-visible check pins the two groups; no other concurrency behavior changes
- evidence: this run's assess finding 11 (P4 ops) + research candidate C7; wall grep 2026-10-01: unclaimed; defect identical at base and master — fix forward after rm-091

### Execute the real install.sh source-build fallback in an offline-sandboxed test

- id: rm-097
- track: reliability
- priority: 62.0
- status: candidate
- signals: test.fallback_never_run:scripts/ci/check-install-runtime.sh::L106-150 — test D replaces git/cargo with no-op shims that plant a stub at target/release/agenttrace, so the real `git clone --depth 1 && cargo build --release -p agenttrace` fallback — the newest install.sh code and the axis master-era rm-051 pins — is never executed by any test (`sh -n` is its only guard); verified identical at this base and at master
- acceptance: an offline sandbox test drives the real fallback with a `file://` bundle remote and a pinned ref and asserts the built binary passes the runtime probe; test D's shims remain for the pure-script arms; the lane runs in CI without network; suite green
- evidence: this run's assess finding 9 (P3 test gap) + research candidate C8; complementary to rm-051 (pinning) and the PR-lane restoration item; wall grep 2026-10-01: unclaimed

### Ingest Qwen Code /export and Dual Output --json-file transcripts once corpora exist

- id: rm-098
- track: compatibility
- priority: 60.0
- status: candidate
- signals: ecosystem.open_upstream_radar:luoyuctl/agenttrace issue #237 (OPEN, verified via gh 2026-09-30) — Qwen Code now exposes /export (Markdown/JSONL/HTML; HTML default since 2026-05-14) and Dual Output structured JSON event streams captured from --json-file; docs evidence cited in the issue body (qwenlm.github.io/qwen-code-docs: weekly-update-2026-02-09, weekly-update-2026-05-14, users/features/dual-output); user value: distinguish verified local Qwen history vs exported artifacts vs sidecar event streams when reporting cost/tool failures/health
- acceptance: corpus-gated exactly like the Antigravity item (rm-067): NO parser work until a real fixture lands; first deliverable is a committed fixture plus a format-acceptance report stating what agenttrace can and cannot read today; then parse /export JSONL and --json-file event streams with tokens/cost extracted and a doctor format-acceptance line; fixtures committed under scripts/fixtures/
- evidence: research candidate C5 (MED) citing `gh issue view 237`; extends the corpus-gated ingest family (rm-067 upstream issue #236, run-1766ab4e rm-074 Grok/Antigravity) with the unclaimed Qwen axis; wall grep 2026-10-01: unclaimed

### install.ps1 parity: pre-install runtime probe and recovery path

- id: rm-099
- track: customer-experience
- priority: 54.0
- status: candidate
- signals: ux.post_install_only:install.ps1::L50-72 — runtime verification is post-install only: the binary is Move-Item'd into place (:55), then probed (:60-66); on failure it is removed and the user is told to build from source (advice only, :71), while install.sh has both a pre-install probe and a (to-be-pinned) source-build fallback — Windows users get strictly worse failure modes; read live at this base
- acceptance: install.ps1 probes the artifact BEFORE moving it into place; on probe failure nothing is left half-installed and the script offers the same recovery path as install.sh (pinned source build) or a precise next-step message; a PowerShell harness test simulating a broken artifact asserts no partial install; offline checks green
- evidence: this run's assess finding 10 (P4 parity) + research candidate C9; different axis from the master-era ps1 checksum-verify item (rm-005); wall grep 2026-10-01: unclaimed

<!-- managed by hermes-roadmap render; do not edit by hand -->
