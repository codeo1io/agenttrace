# Extensions research — 2026-09-14 (ninth pass)

Run: `58910360ca80424aa2e3c8c9820e6ee5` · phase `research` · attempt `7eeb18b1cf5c49b3ad7ff7cef71f4ad8`
Head: `df3b621`. Channels: `gh` (upstream repo, issues, PRs, discussions) and Exa web
search/fetch. Companion artifact of this pass:
`.conductor/research/2026-09-14-ecosystem-candidates.md` (working notes; the durable
record is this file plus the ROADMAP lane entries).

---

## Upstream census (2026-09-14)

- Upstream `luoyuctl/agenttrace` is active again after a quiet spell: `pushedAt`
  2026-09-11, latest release **v0.8.1** (2026-09-06), 131 stars.
- Exactly **one** upstream commit postdates our fork HEAD: `6848aa1` = PR **#284**
  ("fix(parser): skip leading non-session lines in Oh My Pi JSONL", merged 2026-09-11).
  The fork does not have it, and the defect reproduces locally: an Oh My Pi JSONL whose
  first line is `{"type":"title",...}` fails with `Error: oh_my_pi: missing session
  header` (`parser.rs:1325`); upstream's fix skips lines until the first
  `type == "session"` object. Filed in the hardening lane as the F11 companion port item.
- Open upstream issues (3): radar **#236** (Antigravity transition), radar **#237**
  (Qwen export / dual-output), and **#103** — *promoted* 2026-07-19 from radar to an
  implementation task for provider/cost provenance with written acceptance criteria;
  upstream #280 (2026-08-22) landed the per-model `pricing_status` half our tree shares.
- Upstream's dependency queue is stale: dependabot **#279** (crossterm 0.28.1→0.29.0,
  rusqlite 0.32.1→0.40.2, clap 4.6.2→4.6.3, serde/serde_json/thiserror patches) open
  since 2026-08-10, plus #278 (`actions/attest-build-provenance`) and #259
  (`actions/checkout` 6→7, open since June). Feeds candidate 23.
- Discussion #2's feedback thread (incident timelines, CI regression JSON) is fully
  landed upstream (#203–#205) and present in the fork — no new ask there.

## Candidate 53 (new) — Calendar spend buckets and as-of-date pricing

Two coupled gaps, both ecosystem-validated this pass:

1. **Calendar buckets.** ccusage ships daily/weekly/monthly/session reports across its
   full 15-source matrix; RoninForge **claude-code-cost** ships `/by-day`, `/by-week`
   (Monday-based), `/by-month` (calendar month, UTC) views plus `/by-project`. agenttrace
   has `today|7d|30d|all` only (`main.rs:833-835`). This is the display-side sibling of
   candidate 13 (`--since`/`--until`/`--timezone`) and candidate 3's window math.
2. **Point-in-time pricing.** claude-code-cost prices every response at the rate in
   effect on its date, backed by a bundled `ai-price-index` dataset, and publishes the
   design argument ("why a price change should not restate your recorded history"). We
   price every historical session at the *current* catalog snapshot
   (`PRICING_SNAPSHOT_DATE` 2026-09-02), so each `--update-pricing` silently restates
   history and breaks baseline deltas taken weeks apart.

Acceptance: `--bucket day|week|month` tables (local-calendar aware, riding the P3-2 fix)
over the existing session set in text/JSON/Markdown; and a dated-pricing mode where each
session prices against the rate tier effective on its date, with the mode disclosed in
`data_health` (`pricing_basis: current|as-of-date`) and baseline deltas stable across a
catalog refresh under the as-of-date mode. Evidence: a fixture priced under two snapshot
dates asserting history does not move when the catalog refreshes; a bucket test pinning
Monday-week and UTC-month boundaries; a parity note against ccusage's day/week/month
output on a shared corpus.

## Candidate 54 (new) — Next-source coverage: Goose first, Amp explicitly legacy

ccusage's source matrix now reads Amp, Droid, Codebuff, Goose, Kilo, GitHub Copilot CLI,
Qwen, and Gemini — five we do not parse. Independent format documentation (the txcript
crate's published format notes) ranks the follow-up targets:

- **Amp** (`~/.local/share/amp/threads/*.json`) is documented as a **legacy** local
  store: current Amp CLI versions are server-authoritative and neither read nor write
  `threads/` (verified by bisection in txcript's notes). Weak target; do not claim.
- **Goose** sessions are the most fixture-able next source (local session JSONL with
  documented layout); **Kilo/Droid/Codebuff** remain census items until a corpus exists.

Acceptance: one new source per change unit, fixture-first (no docs-only support claims,
matching the #236/#237 discipline), README provider table updated in the same change,
`--doctor` reporting the new root, and the format-canary watch list (candidate 5)
extended. Evidence: synthetic-`HOME` discovery round trip; a hostile fixture proving
graceful degradation.

## Strengthening notes for existing candidates

- **Candidate 14 (sub-agent attribution):** RoninForge claude-code-cost reads *every*
  `*.jsonl` under `~/.claude/projects` including subagent logs and attributes their
  spend; Anthropic's `session-report` plugin features subagent/skill attribution (cited
  in upstream #103). Both treat sidechain rows as first-class cost rows — the
  acceptance's `by_sidechain` output now has two shipped reference implementations to
  parity-check against.
- **Candidate 16 (provider-recorded git branches):** claude-code-cost attributes spend
  per project *and per branch*, "both recorded by Claude Code itself" — direct
  ecosystem validation that the transcript-recorded `gitBranch` path outranks git
  timestamp correlation.
- **Candidate 4 (second pricing source, models.dev):** models.dev exposes per-model
  context-window limits through `api.json`; besides pricing redundancy this can replace
  the hardcoded window fallback in `context_utilization` (`diagnostics.rs:780`,
  131_072 default) — folding in F11-4's estimator unification.
- **Candidate 23 (dependency currency):** upstream dependabot #279/#278/#259 remain
  unmerged (oldest since June); our lockfile still pins rusqlite 0.32 / crossterm 0.28.
  The lane stands as written; the queue above is the current target set.
- **Candidate 24 (cost provenance):** upstream promoted #103 on 2026-07-19 with
  acceptance language nearly identical to this candidate (provider-scoped keys, additive
  provenance fields, no billing claims), and the community schema proposal there adds
  `cost_provenance.{cost_source, pricing_catalog_source, pricing_match_status,
  requested_model, matched_model, confidence}` plus `token_provenance`. Our tree has the
  per-model `pricing_status` (upstream #280 content) but `recent_sessions` still emits a
  scalar `cost` — the per-session additive object is the remaining half.
- **Candidate 52 (Antigravity store decode):** the txcript format notes document the
  store layout independently of agy — `conversations/<id>.db` with `trajectory_meta`
  (trajectory_id, cascade_id = session id), `steps` (one protobuf `gemini_coder.Step`
  per row: type tag, status, CortexStepMetadata envelope, per-kind payload), plus
  `trajectory_metadata_blob` (workspace, branch, created-at) — and the JSONL display
  mirrors under `brain/<id>/.system_generated/logs/transcript.jsonl`. Two independent
  descriptions agreeing lowers the reverse-engineering risk; the corpus prerequisite
  stands, and F11-1's accounting fix must land with or before any store decode so the
  new source does not inherit the clamp bug.
- **Candidate 2 (OTel GenAI):** re-checked the dedicated conventions repo — the
  agent/framework spans page (create/invoke agent, execute tool, plan) is still
  **Development** status with no stable tag; the tag-gated pin stands. Claude Code's
  native OTel three-signal surface (pass-8 note) remains the ingest-half producer.
- **Candidate 26 (Qwen dual-output):** no new evidence beyond pass 8's docs pin;
  upstream radar #237 discipline holds — wait for an artifact.

## User-need signals (no new lane)

Stanford Digital Economy Lab's "How Do AI Agents Spend Your Money?" (via #103) keeps
token-spend variance framed as a debugging concern; the fresh entrant set this pass
(claude-code-cost + its `goei-sync`/BudgetClaw ecosystem, txcript as a format-conversion
layer, coding_agent_session_search) all reinforce the local-first cross-tool
cost/token-visibility pattern the roadmap already lanes. The durable differentiator
recorded in pass 5 stands: diagnosis over accounting.

## Sources

- gh repo view / commits / issues / PRs / discussions on `luoyuctl/agenttrace`
  (2026-09-14); `gh pr diff 284`; `gh pr view 279`.
- github.com/ryoppippi/ccusage README (source matrix, daily/weekly/monthly/session).
- github.com/RoninForge/claude-code-cost README (branch attribution, subagent logs,
  dated ai-price-index pricing, by-day/by-week/by-month UTC buckets) and
  roninforge.org/data/ai-price-index/back-dating/.
- github.com/anomalyco/models.dev (api.json, TOML catalog, opencode consumer).
- docs.rs/crate/txcript — docs/formats/antigravity.md and docs/formats/amp.md.
- github.com/open-telemetry/semantic-conventions-genai — gen-ai-agent-spans.md
  (Status: Development).
- Local tree checks: `main.rs:833-835` (range presets), `pricing.rs:16` (snapshot date),
  `diagnostics.rs:771-800` (estimator + window fallback), `reports.rs` recent_sessions
  scalar cost, and the #284 repro at `/tmp/at-adv/omp.jsonl`.
