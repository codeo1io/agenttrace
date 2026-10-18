---
artifact_contract: "ce-handoff/v1"
created_at: "2026-09-30T21:45:00Z"
title: "Cycle 1 prioritization (campaign 1f5ad3cf) — count every session, disclose every artifact"
summary: "Scores the open ROADMAP.md board at HEAD 9d88b36 after this campaign's assess (5d6eb40c, F1-F4) and research (5b2d04a1, RC-1..RC-4) passes — 7 newly minted items rm-084..rm-090 plus the two inherited item generations — by impact, risk-if-deferred, effort, dependency, and strategic value; selects the cycle-1 batch rm-084 pi-fork + pi-profile session-home discovery (lead) + rm-086 PRIVACY.md at-rest disclosure and --clear-cache parity (rider), with rejected-alternative analysis (rm-085+rm-087 deferred as the cycle-2 pair on reports.rs lane contention, rm-020 as the cycle-2/3 anchor on size, rm-011 deferred a fourth time, hermetic-tui excluded as duplicate sibling work), merge-safety verified against five uncommitted sibling worktree file sets, and a red-to-green execution order with the sandboxed-HOME gate trap and .rs-only digest mechanics baked in."
keywords: ["agenttrace", "cycle-1", "prioritization", "discovery", "pi-fork", "senpi", "privacy", "clear-cache", "cache-artifacts"]
cwd: "/home/agent/.hermes/conductor-worktrees/agenttrace-80c75f65b7/run-792ef47bdeaf-792ef47b"
repository: "codeo1io/agenttrace"
repo_root_sha: "9d88b36750a991bd1436dbbc91b4579c39003067"
branch: "conductor/run-792ef47bdeaf"
head: "9d88b36"
---

# Cycle 1 prioritization (run 792ef47b, attempt bd0a53e2)

- run: `792ef47bdeaf42a0a76d02ac170473c4`, attempt
  `bd0a53e2e9934cebb1e16091b95a21e2`
- intent: `prioritize_repository_maintenance` (campaign `1f5ad3cf`,
  cycle 1)
- grounding: HEAD `9d88b36` (porcelain clean except this run's
  roadmap-phase edit to `ROADMAP.md`, +45/-0 uncommitted,
  commit-gate-owned, and nothing else). Baseline re-verified this
  campaign by the assess phase: full workspace suite 264/0 under a
  sandboxed HOME, `cargo clippy --workspace -- -D warnings` clean,
  debug binary built at this HEAD (`target/debug/agenttrace`).
- inputs: `ROADMAP.md` (this worktree: two inherited item generations
  rm-001..rm-027 with the known duplicate rm-012..rm-023 blocks, plus
  this campaign's uncommitted rm-084..rm-090), assess ledger
  (`assess-5d6eb40c…/2026-10-01-adversarial-assessment.md`), research
  ledger (`research-5b2d04a1…/2026-10-01-cycle1-research-candidates.md`),
  and a fresh sweep of every sibling worktree's uncommitted diff
  (file sets and claimed IDs, listed in §4).
- fresh evidence this phase (all commands re-run 2026-10-01 at HEAD
  `9d88b36`, debug binary): default `--overview` reports **5700
  sessions / $4.4K** with the By-Agent table showing **Pi 199
  Sessions / $307.43** and no senpi/omo/pi-profile line, while the
  host carries `~/.senpi/agent-cliproxy-only/sessions` **1404 jsonl**,
  `~/.omo` **150 jsonl**, and `~/.pi/agent-cliproxy-only/sessions`
  **8187 jsonl** (was 8010 at research time — the invisible corpus
  *grew by ~177 files in a day*); `~/.cache/agenttrace` holds **17
  files across 5 artifact classes at HEAD source** (sessions.json,
  hermes-sqlite.json, opencode-sqlite.json, pricing.json,
  statusline.jsonl journal 2.4MiB) plus stale `hermes-sqlite-v7-*.json`
  orphans from an unlanded sibling build; `PRIVACY.md` (7 lines)
  discloses only the pricing cache; `clear_session_cache`
  (session_cache.rs:216-229) removes exactly 3 of the 5+ artifact
  classes.

**Skill note.** Same disclosure as every prior cycle in this lineage:
no compound-engineering router is installed in this delegate's pi
session (only `agent-reach`, an internet-research skill — used by the
research phase, not here). The nearest historical match (`ce-plan`)
would be a heavier implementation-plan workflow than a scoring and
selection pass. Scoring ran in-thread: every open item scored on
impact (1–5), risk-if-deferred (1–5), effort (XS–L), external
dependency, and strategic value to the product's two jobs
(cross-agent session analytics; truthful cost/token reporting), under
the hard constraint that the batch be completable and verifiable
**end to end on this host** (offline Linux, no CI execution in this
phase) and **merge-safe** against the five sibling worktrees holding
uncommitted code right now.

---

## 1. Verdict — cycle-1 batch: "Count every session, disclose every artifact"

> **Selected:** **rm-084** pi-fork and pi-profile session-home
> discovery (lead, M) + **rm-086** PRIVACY.md at-rest disclosure and
> `--clear-cache` parity (rider, S).

Two change units, one theme, zero external dependencies, every
acceptance criterion verifiable offline on this host. The theme is the
direct continuation of this repo's lineage ("trustworthy strings on
untrusted input", "truthful reads, truthful gates"): **rm-084** closes
a live-proven completeness hole in the product's core job — default
discovery is blind to 9,741 pi-family jsonl files on the very host it
runs on (≈97.5% of the host's pi sessions, ≈$11K of spend) — and
**rm-086** closes the honesty gap that the lead *widens*: registering
new homes means more conversation-derived names/CWDs/file paths flow
into `sessions.json`, a cache `PRIVACY.md` never discloses and
`--clear-cache` cannot fully purge.

Why this batch wins every test applied to it:

- **The lead is the only impact-5 data-completeness defect on the
  board, with a live number no other item approaches.** The tool's
  headline promise is "see all your agents' sessions and spend";
  today it shows 5700 sessions/$4.4K where the host's truth is
  ≈15.4K sessions/≈$15K. Compare the strongest old-board rival,
  rm-020 per-model pricing (gen-1 :170): it improves *precision on
  sessions already seen*; rm-084 fixes *wholesale blindness*.
- **Both compound if deferred.** The invisible corpus grows every day
  (8187 vs 8010 profile jsonl in one day; senpi/omo mtimes run
  2026-08-16..09-18 and continue); each run also refreshes
  `sessions.json` while the journal accumulates conversation-derived
  strings at rest, undisclosed and unpurgeable through the tool.
- **The rider is the only zero-contention, high-trust-value unit
  available.** Its file set (PRIVACY.md, session_cache.rs, plus a
  ~5-line append-only path helper) overlaps **no** uncommitted
  sibling lane (verified by diffing all five, §4).
- **It is simultaneously highest-value and end-to-end completable
  here:** live corpora on this host for before/after proof
  (`~/.senpi`, `~/.omo`, `~/.pi/agent-cliproxy-only`), a fresh
  default-overview baseline recorded this phase (5700/$4.4K), fixture
  goldens for multi-home/multi-profile discovery, and a sandboxed-HOME
  clear-cache exercise for the rider.
- **Merge-safest high-value pair available.** rm-084's overlap with
  in-flight lanes is exactly one stale-base worktree (83642957,
  rm-055 canonical-root dedup at base 90a4ef5) whose concern
  *composes* with new-home registration; the batch deliberately
  avoids reports.rs (2 claiming lanes), pricing.rs (3 claiming
  lanes), and the whole TUI crate (2 hermeticity-fix lanes). See §4.

## 2. Scoring — the open board

Imp = impact (1–5) · Risk = risk if deferred · Eff = effort · Dep =
external dependency · Strat = value to the two product jobs. IDs are
campaign-local and this file carries two rm-012..rm-023 generations —
rows cite **title + ROADMAP line**; the ID is a handle only.

| Item (title · line · this campaign's ID) | Imp | Risk | Eff | Dep | Strat | Disposition |
|---|---|---|---|---|---|---|
| Discover pi-fork and pi-profile session homes · :339 · rm-084 | 5 | 5 | M | none | 5 | **batch (lead)** |
| Disclose at-rest artifacts in PRIVACY.md, align --clear-cache · :363 · rm-086 | 4 | 3 | S | none | 4 | **batch (rider)** |
| Honor --lang in every report renderer · :355 · rm-085 | 4 | 3 | S–M | none | 4 | cycle-2 pair w/ rm-087 |
| Absolute-time scoping --since/--until · :371 · rm-087 | 3 | 2 | M | none | 4 | cycle-2 pair w/ rm-085 |
| Price multi-model sessions per model · :170 · gen-1 rm-020 | 5 | 4 | M+ | none | 5 | cycle-2/3 anchor |
| Report-contract truthfulness batch · :105 · gen-1 rm-011 | 3 | 3 | M | none | 3 | defer, 4th time (§3) |
| Eliminate TUI dual-renderer test/prod binding · :127 · gen-1 rm-014 | 3 | 3 | M | none | 3 | defer (TUI lanes contended) |
| Parse JSONL once across probes · :269 · gen-2 rm-014 | 3 | 3 | M+ | none | 3 | defer (parser churn post-port) |
| Harden discovery-cache keys/freshness · :301 · gen-2 rm-019 | 3 | 3 | S–M | none | 3 | sequenced behind rm-084 (§3) |
| Statusline schema catch-up · :276 · gen-2 rm-015 | 3 | 3 | M | none | 3 | defer (statusline.rs ×3 lanes) |
| Codex zstd rollouts in-process · :29 · rm-003 | 3 | 3 | M | crate vs static-build profile | 3 | defer (forward-compat, parser contended) |
| Make -m truthful on --compare/--test-match · :379 · rm-089 | 3 | 2 | S–M | none | 3 | defer (pricing.rs ×3 lanes) |
| Pricing snapshot cadence + drift · :59 · rm-006 | 3 | 2 | S–M | network at refresh | 3 | defer (pricing.rs contended) |
| Pricing provenance stamped into artifacts · :181 · gen-1 rm-021 | 3 | 2 | S | rides rm-020 | 3 | cycle-2/3 rider w/ rm-020 |
| 5-hour billing block analytics · :307 · gen-2 rm-020 | 4 | 2 | M+ | none | 5 | cycle-3 capability lead (standing plan) |
| Display-width math for report columns (incl. compare padding anchor) · :243 · rm-027 | 2 | 2 | S–M | none | 2 | defer (reports.rs contended) |
| Shell completions/man via clap_complete · :377 · rm-088 | 2 | 2 | M | installer-channel conventions | 2 | defer |
| Trust-disposition ledger · :385 · rm-090 | 1 | 1 | M | promotion gate | 1 | watch, promotion-gated |

Not tabled (unchanged standing dispositions, all defer): rm-001 npm
refactor, rm-007 dependency wave, rm-008 API prune, gen-1 rm-015/016/
017/018/019/022/023/025/026, gen-2 rm-016/017/018/021/022/023 —
dependency churn, CI/release lanes needing a CI run to evidence,
perf/architecture work, and low-severity correctness riders, none of
which outrank the selected pair under reliability/compatibility
first.

## 3. Standing dispositions

- **rm-085 + rm-087 are the cycle-2 pair** ("meet the user: language
  and time"). rm-087's acceptance already states it composes with
  rm-085's i18n threading; one renderer-threading pass lands both.
  Deferred this cycle on lane contention: reports.rs is claimed by
  run-52465b9e (rm-034..039 @ ce27969) and run-aa9c4fd6 (@ 9d88b36).
- **rm-020 (gen-1, per-model pricing) is the cycle-2/3 anchor.**
  Highest strategic value on the old board (no competitor prices
  per-model; cad2c25d RC-3 proved one mixed-model row nulls
  cost_audit's headline `current_estimated_cost`), but M+ effort
  across lib.rs pricing core + three render surfaces + sqlite path;
  cannot ride this cycle without blowing the two-unit landing size,
  and lib.rs/reports.rs are both contended right now. rm-021 (gen-1,
  provenance stamping) rides it.
- **rm-011 (gen-1) deferred a fourth time — with a clock.** Full-session
  estimate stands (three sub-defects × golden scaffolding); its exact
  files (search.rs, reports.rs, governance surfaces) are mid-flight in
  run-52465b9e's lane. It becomes the cycle-2 fallback only if that
  lane lands first.
- **gen-2 rm-019 (discovery-cache keys) is sequenced behind rm-084,
  deliberately.** Widening discovery changes what flows into the
  discovery cache; hardening cache keys on the pre-widening shape is
  work on moving ground. Cycle 2 takes it against the widened corpus.
- **The hermetic TUI test fix is NOT selected and must not be
  re-implemented here.** Two sibling lanes already hold independent
  uncommitted fixes (run-aa9c4fd6: env-pin + serializing lock;
  run-e602bb69: warm-draw + clear-governance assertion), and the
  fold-time rule is to keep exactly one. This batch touches no
  `agenttrace-tui` file, adding a third wrapper is explicitly
  rejected (§4). Execution consequence: every gate run uses a
  sandboxed HOME.
- **rm-090 stays promotion-gated watch** (single four-month-old
  upstream demand signal, no momentum). No implementation spend.
- **Board integrity note for the fold.** At least three live
  double-mint generations now exist: (a) the in-file duplicate
  rm-012..rm-023 blocks (known, PR #17 renumber precedent); (b)
  run-1766ab4e vs run-52465b9e both claiming rm-034..rm-0xx; (c)
  run-b8d7db05 minting rm-084..rm-087 (mtime 2026-09-30T21:04Z) 16
  minutes before this campaign's roadmap phase minted rm-084..rm-090
  (mtime 21:20Z), with run-266b6e2b already minting rm-091..rm-094
  past the union (21:34Z). Nothing here is renumbered by a
  prioritization phase; items are cited by title at fold and the
  commit gate reconciles IDs per the PR #16/#17 precedent.
- **Cross-campaign scope note on rm-084.** The profile half
  (~/.pi/&lt;profile&gt;/sessions) was first proven by sibling campaign
  d7ca90df cycle 2's research (cad2c25d RC-1); that campaign's
  selection is not visible from here. Landing rm-084 first claims the
  work; if both lanes implement, the fold reconciles on the earlier
  merge and the losing diff is evidence, not waste.

## 4. Merge-safety — the live uncommitted fleet (diffed this phase)

| Worktree | Base | Uncommitted code file set (claimed IDs) | Overlap with this batch |
|---|---|---|---|
| run-83642957 | 90a4ef5 (stale, 2 merges behind) | discovery.rs, parser.rs, discovery_contract.rs, statusline.rs, lib.rs, main.rs, doctor.rs, history.rs, Cargo.toml (rm-055..rm-064: canonical-root dedup, oversized-file rejection, journal compaction, #286 port) | **rm-084**: discovery.rs + parser.rs. Composes: rm-055 rewrites *admission keying*; rm-084 adds *registry entries* and a profile-expansion step. Mitigation: additive edits only; new tests in a NEW file (not discovery_contract.rs) |
| run-52465b9e | ce27969 | main.rs, lib.rs, reports.rs, search.rs, statusline.rs, discovery_contract.rs, filters.rs (rm-034..rm-039) | none (batch avoids reports.rs/main.rs bulk; the 1-line `-d` help ride-along in main.rs is a trivially mergeable doc-string change) |
| run-aa9c4fd6 | 9d88b36 | insights.rs, lib.rs, pricing.rs, reports.rs, explorer.rs, presentation.rs, tui/tests.rs + flake-prevention doc | none |
| run-e602bb69 | 9d88b36 | main.rs, upstream.rs, doctor.rs, lib.rs, pricing.rs, tui/tests.rs, guides | none (this lane also owns the --fetch-timeout and hermeticity space — untouched here) |
| integration-58cb435705b8 | staged merge (c7dca75d/PR #19) | parser.rs, pricing.rs, ci.yml, CHANGELOG, ROADMAP, docs, scripts | **rm-084**: parser.rs only — `pi_source_for_path` is a 9-line isolated function; conflict surface minimal and this repo has landed parser.rs changes under open PRs twice before |

ROADMAP-only claimers (no code): run-1766ab4e (rm-034..041),
run-40208f3d (rm-068..), run-b8d7db05 (rm-084..087, colliding),
run-266b6e2b (rm-091..094), run-cad2c25d (clean, pre-roadmap).

## 5. Rejected alternatives

- **rm-084 + rm-085 (top-two-by-priority pairing):** rejected —
  rm-085's renderer family (reports.rs ×7 entry points) is claimed by
  two uncommitted lanes; a wide threading pass under that contention
  triples the fold burden, and rm-085 pairs better with rm-087 next
  cycle (§3).
- **rm-084 alone:** leaves S-effort cycle capacity unused and forgoes
  the only zero-contention trust win; the rider compounds with the
  lead (widened discovery → more undisclosed at-rest data) rather
  than merely coexisting.
- **rm-020 (gen-1) as lead:** M+ effort, contended files, and it
  fixes precision on already-seen sessions while the board's only
  impact-5 blindness defect stands; also too large to pair, breaking
  the two-unit landing size every prior cycle hit.
- **rm-011 (gen-1) as batch:** full-session size estimate plus active
  contention on its exact files (§3).
- **rm-088 as rider:** acceptance spans installer-channel conventions
  (homebrew/winget/npm) that cannot be exercised offline this cycle —
  it would buy claims we cannot close in-cycle (same reasoning that
  rejected rm-017-class riders in prior cycles).
- **The hermetic TUI test fix as a rider:** rejected — two
  independent fixes already sit uncommitted in sibling lanes; a third
  wrapper is recorded fold-time debt. The batch's no-TUI file set is
  the mitigation.

## 6. Execution order (red-to-green, for the implement phase)

1. **rm-084 first, fixture-red first.** Build the multi-home,
   multi-profile fixture corpus and observe the test RED against
   default `discover_session_dirs` (fork homes + profile variants
   absent). Then: extend the pi-family registry (senpi/omo-class
   homes; enumerate `<home>/<profile>/sessions` variants; respect
   PI_CODING_AGENT_DIR relocation), fix `pi_source_for_path`
   (parser.rs:1416-1424) to label by actual home root — including the
   XDG root currently mislabeled `oh_my_pi` — and give `-d` non-blank
   `--help` text (main.rs, 1 line). Keep `discover_session_dirs`
   edits additive so rm-055's canonical-keying rewrite composes at
   fold; put new integration tests in a **new** test file (not
   `discovery_contract.rs`); `pi_source_for_path` unit tests stay
   in-module beside it.
   Live gate: default `--overview` grows senpi/omo/pi-profile
   attribution vs the 5700-session/$4.4K baseline recorded this phase.
   Budget note: the first run is a **cold parse of ~9.7K new files**
   (they are not in the discovery cache) — minutes, not the ≈6s warm
   overview; use an isolated `AGENTTRACE_SESSION_CACHE_DIR` for the
   before/after pair and do not mistake the cold scan for a hang.
2. **rm-086 second.** Enumerate the artifact classes HEAD writes
   under the cache dir (sessions.json, hermes-sqlite.json,
   opencode-sqlite.json, pricing.json, statusline.jsonl); rewrite
   `PRIVACY.md` with a data-class + purge-command table; extend
   `clear_session_cache` to remove or explicitly report every listed
   artifact including the journal — prefer a small pub path helper
   (e.g. `statusline_journal_path()` in statusline.rs, ~5 lines,
   append-only) over duplicating path logic; add the registry test
   pinning PRIVACY text against the code's cache-path construction
   (this is what guards the v7-orphan drift already visible on this
   host). Live gate: sandboxed-HOME run → artifacts present →
   `--clear-cache` → directory empty of every disclosed class.
3. **Gates (both units):** `cargo fmt --all --check`;
   `cargo clippy --workspace -- -D warnings`;
   `cargo test --workspace` under a **sandboxed HOME with
   RUSTUP_HOME/CARGO_HOME exported** (the real-HOME run fails the tui
   hermeticity test on any host running the statusline hook — known
   trap; the batch does not fix that here); `scripts/ci/check-*.sh`.
   Reuse this worktree's `target/` (debug binary already current).
4. **Validation-digest mechanics:** both units' executable surface is
   `.rs`-only (PRIVACY.md is a document) — on this repo such turns do
   not move the validation token; declare the dispatch digest
   verbatim and re-derive the engine-format token via
   `validation_policy.validation_digest(base=HEAD sha, repo_path)`,
   with the usual append/revert sensitivity probe recorded.
5. **ROADMAP status flips belong to later phases** (compound flips
   candidate→implemented with its `- compound c1 <date>:` bullet; the
   done-flip is the commit gate's). This phase selects; it does not
   edit ROADMAP.md.

Verification anchors live in `ROADMAP.md` (rm-084 at :339, rm-086 at
:363) and this campaign's spool ledgers (assess `5d6eb40c`, research
`5b2d04a1`); the fresh baseline numbers in this document's grounding
section are re-derivable with the commands in its evidence list.
