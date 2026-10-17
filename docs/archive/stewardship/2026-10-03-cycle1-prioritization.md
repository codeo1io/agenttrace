---
artifact_contract: "ce-handoff/v1"
created_at: "2026-10-03T00:35:00Z"
title: "Cycle 1 prioritization (run 364aa3be) — the cache obeys its own contract, and the pipe stays pure"
summary: "Scores the open board at fork/master fd5532f after this run's assess (c22757c9: net-new A1 cache-bound violation live at 108.9% on the real corpus, A2/A3 flat-pair namespace gaps, A4 CHANGELOG tag gap; known-live N1 stdout pollution, N2 gate flags) and research (c52c2cc3: pass-12, C71..C75) passes — the wall at 113 items (78 candidate / 21 done / 14 implemented) plus this run's uncommitted rm-298..rm-303 mint — by impact, risk-if-deferred, effort, dependency, and strategic value; selects the cycle-1 batch rm-298 byte-true session-cache bound + first dirs bound (lead, M) + rm-301 stdout purity for -f json under side-effect flags (rider, S) + rm-303 CHANGELOG section for every merged tag (rider, XS), with rejected-alternative analysis (rm-300 deferred to cycle-2 lead on two-M landing discipline, rm-299 on in-flight rm-286 overlap and dependency ordering behind rm-298, rm-302 on the unlanded rm-201 dependency, assess N2 EXCLUDED as sibling-claimed rm-246), merge-safety verified against the three live uncommitted code lanes (none touches session_cache.rs), and a red-to-green execution order."
keywords: ["agenttrace", "cycle-1", "prioritization", "session-cache", "byte-bound", "dirs-bound", "stdout-purity", "machine-readable", "changelog-coverage", "plugin-version-gate"]
cwd: "/home/agent/.hermes/conductor-worktrees/agenttrace-80c75f65b7/run-364aa3be00f6-364aa3be"
repository: "codeo1io/agenttrace"
repo_root_sha: "fd5532fcd4024f85a4a708db9dd3bbc20bc7b259"
branch: "conductor/run-364aa3be00f6"
head: "fd5532f"
---

# Cycle 1 prioritization (run 364aa3be, attempts e03c70cf + audit b8cfcdc2)

- run: `364aa3be00f6433eab47846195b9134c`, attempt
  `e03c70cfda51414a9f7491077c88a5d4` (authoring), audited and adopted
  unchanged by attempt `b8cfcdc20cff421bbde7d392a0142268` (§7)
- intent: `prioritize_repository_maintenance`
  (repository-maintenance `dde7875c`, cycle 1)
- grounding: HEAD `fd5532f` (porcelain: only this run's roadmap-phase
  edit to `ROADMAP.md`, +41/-0 uncommitted, commit-gate-owned, and
  nothing else). Envelope at this HEAD re-verified green by the assess
  phase (fmt / clippy `-D warnings` / tests / release build / CI gates
  once `AGENTTRACE_BIN` is provided).
- inputs: `ROADMAP.md` (this worktree: committed wall rm-001..rm-238 +
  this run's uncommitted rm-298..rm-303), assess ledger
  (`/tmp/at-assess-c22757c9/assessment-report.md`), research ledger
  (`/tmp/at-research-c52c2cc3/research-report.md`), the prior-cycle
  prioritization convention (`docs/stewardship/2026-10-01-cycle1-prioritization.md`),
  and a fresh sweep of every sibling worktree's uncommitted diff (§4).
- fresh evidence re-verified this phase: the assess PoC cache artifact
  `/tmp/at-assess-c22757c9/capcache/sessions.json` re-measured on disk
  at **72,262,482 B = 108.9 % of the 67,108,864 B cap** (the assess
  run recorded 72,257,626 B; both over — the violation is stable
  across rewrites, not a one-off); `grep -n '^## v' CHANGELOG.md`
  confirms the heading jump `v0.9.0` → `v0.7.1` with no `v0.8.x`
  section (the v0.9.0 body carries a backfill *annotation*, so
  rm-303's residual is the missing per-tag gate arm, not missing
  prose); `git worktree list` + per-worktree `git status --porcelain`
  census re-run this phase (three live code lanes, five ROADMAP-only
  claimers — §4).

**Skill note.** Same disclosure as every prior cycle in this lineage:
no compound-engineering router is installed in this delegate's pi
session (only `agent-reach`, an internet-research skill — used by the
research phase, not here). Scoring ran in-thread: every open item
scored on impact (1–5), risk-if-deferred (1–5), effort (XS–L),
external dependency, and strategic value to the product's two jobs
(cross-agent session analytics; truthful cost/token reporting), under
the hard constraints that the batch be completable and verifiable
**end to end on this host** (offline, no CI execution in this phase),
**merge-safe** against the live uncommitted fleet, and landable as a
reviewable unit (one M lead + small riders, the discipline the
2026-10-01 pass codified).

---

## 1. Verdict — cycle-1 batch: "The cache obeys its own contract, and the pipe stays pure"

> **Selected:** **rm-298** byte-true session-cache bound and a first
> dirs bound (lead, M) + **rm-301** stdout purity for `-f json` under
> side-effect flags (rider, S) + **rm-303** CHANGELOG section for
> every tag merged into HEAD (rider, XS).

Three change units, one theme — *the tool's own outputs are
truthful* — zero external dependencies, every acceptance criterion
verifiable offline on this host. This is the direct continuation of
the repo's lineage ("trustworthy strings on untrusted input" →
"truthful reads, truthful gates, durable records" → "truthful
counters, bounded waits, private artifacts, pinned installs"): the
on-disk artifact obeys its documented 64 MiB hard bound byte-exactly,
machine-mode stdout is a single parseable document even when
side-effect flags fire, and the public record covers every shipped
tag.

Why this batch wins every test applied to it:

- **The lead is the previous campaign's own designated successor.**
  The wall's compound-c1 record (run a9e73293, commit note at
  `ROADMAP.md` header) names *"F3 session-cache 64MiB cap violated
  live (~106 %)"* as a **cycle-2 lead** — and this run's assess
  sharpened it from "~106 %" folklore to a byte-decomposed,
  re-measured 108.9 %: entries-only values 100.1 % of cap, dirs map
  3,586,620 B / 2,783 entries (5.3 %) entirely uncounted *and
  unbounded*, path keys + punctuation the rest. The estimator
  (`session_cache.rs:836-863` over `cache_paths_sized_once`) and the
  writer (`save_session_cache` :874-921) were both read again this
  phase and match the assess mechanism line-for-line. Nothing else on
  the board combines a documented-contract violation, a live real-
  corpus reproduction, and a designated-lead endorsement.
- **It is the only impact-5 defect whose harm compounds on a proven
  curve.** codeburn's published `perf-cache-fix.md` (11,309 ★,
  same-class tool) documents where this architecture ends: a 386 MB
  monolithic JSON cache, ~20 % CPU re-parse per fresh process. Our
  72 MB / 108.9 % is on the same curve, the corpus grows daily, and
  the in-repo docs still describe the overflow as *"latent"* — three
  stale claims (decision doc :47, independent-review :116) the fix
  corrects in the same change.
- **Zero live code contention.** The full worktree census (§4) shows
  exactly three lanes with uncommitted code, none touching
  `session_cache.rs`, `CHANGELOG.md`, or the version gate; the two
  lanes touching `main.rs` sit on bases 2–3 integration merges stale.
  The roadmap phase separately verified rm-301 unclaimed across the
  landed wall *and* the full rm-239..rm-297 in-flight claim set
  before minting.
- **Every acceptance is mechanically checkable on this host:** the
  A1 PoC already demonstrates the red path (isolated
  `AGENTTRACE_SESSION_CACHE_DIR`, fresh-corpus run, written file
  byte size vs cap), the over-capacity fixture is synthetic
  (long-path keys + many dirs), and the riders' red paths are a
  first-stdout-byte probe (rm-301) and a synthetic missing-section
  fixture (rm-303).
- **Effort shape matches the landing discipline:** one M lead + one
  S + one XS rider ≈ the a9e73293 cycle's proven shape, and stays
  under the two-M rejection line the 2026-10-01 pass set.

## 2. Scoring — the open board

Imp = impact (1–5) · Risk = risk if deferred · Eff = effort · Dep =
external dependency · Strat = value to the two product jobs. The
board is the 78-candidate wall plus this run's fresh mints; rows cite
title + this run's mint ID (or assess label where the item is
sibling-claimed/unminted).

| Item (title · mint/label) | Imp | Risk | Eff | Dep | Strat | Disposition |
|---|---|---|---|---|---|---|
| Byte-true session-cache bound + dirs bound · rm-298 | 5 | 5 | M | none | 5 | **batch (lead)** |
| Flat-pair id namespace hardening · rm-300 | 4 | 4 | M | none | 4 | cycle-2 lead (first in-cycle alternate, §3) |
| stdout purity for -f json under side-effect flags · rm-301 | 3 | 3 | S | none | 4 | **batch (rider)** |
| CHANGELOG section for every tag merged into HEAD · rm-303 | 2 | 2 | XS–S | none | 2 | **batch (rider)** |
| Warm-path status snapshot keyed by stat-only fingerprints · rm-299 | 4 | 3 | M+/L | none | 4 | defer — in-flight overlap + ordering (§3) |
| Opt-in live-quota % from user's own credentials · rm-302 | 3 | 2 | M | rm-201 unlanded | 4 | defer — dependency (§3) |
| Gate flags enforced only under --overview · assess N2 = sibling rm-246 | 4 | 3 | S | none | 4 | **EXCLUDED — claimed** (§3) |
| Statusline journal integrity (locking + growth) · wall (N3) | 2 | 3 | M | none | 2 | defer (stale lane holds statusline.rs) |
| hit_rate >100 % provider semantics · wall/assess B | 2 | 2 | S | none | 2 | defer (standing) |
| History-id/session-admission micro-hardening · rm-212 | 2 | 2 | S | none | 2 | defer (verify-first lane) |
| Timezone-truthful resets + daily buckets · wall | 3 | 2 | M | none | 3 | defer (standing) |
| Markdown pipe-only cell escaping · wall (assess E) | 2 | 2 | S | none | 2 | defer (reports.rs stale-lane history) |
| 5-hour billing block analytics · rm-042 | 4 | 2 | M+ | none | 5 | standing cycle-3 capability lead |
| Conformance fixture pack (AgentMeasure vectors) · rm-053 refresh | 3 | 2 | M | none | 4 | standing (evidence refreshed this run) |
| Pricing snapshot cadence + drift · rm-006 | 3 | 2 | S–M | network at refresh | 3 | defer (drift evidence refreshed; cadence ownership) |
| Codex zstd rollouts in-process · rm-003 | 3 | 3 | M | crate vs static-build | 3 | defer (standing) |

Not tabled: the rest of the 78-candidate wall retains its standing
dispositions (dependency wave rm-007, API prune rm-008, npm refactor
rm-001, promotion-gated watches) — none outrank the selected trio
under reliability-first, and none gained fresh evidence this run.

## 3. Standing dispositions and exclusions

- **rm-300 (flat-pair namespace hardening) is the designated cycle-2
  lead, and the first in-cycle alternate.** It is the next-highest
  fresh correctness item (one journal-controlled id can satisfy two
  calls — the exact masking shape landed rm-230 was written to
  prevent — plus the nameless-result pairing gap and an overstated
  CHANGELOG Unreleased claim). Deferred this cycle on landing
  discipline: pairing it with rm-298 makes a two-M batch, and
  `parser.rs` just absorbed a +322-line landing (cbe30a9c); letting
  that settle one cycle lowers fold risk. Its two PoC fixtures
  (`collide.jsonl`, `nameless.jsonl`) are already on disk, so cycle 2
  starts red-first for free. If the trio lands green with budget
  left, rm-300 is the only sanctioned overflow item.
- **rm-299 (warm-path snapshot) deferred for two reasons, both
  recorded in its own mint.** (a) *In-flight overlap*: sibling run
  6e96bed5's uncommitted rm-286 ("Warm-start parse economics") arms
  (a) tail-incremental re-parse and (b) per-fingerprint sqlite
  snapshot cover this lane; the mint says "fold at integration,
  whichever lands first absorbs the other" — implementing into a
  claimed lane is recorded fold debt, the exact third-wrapper problem
  the 2026-10-01 pass rejects. (b) *Dependency ordering*: the
  snapshot work should build on a byte-true-bounded cache, not beside
  a violated one — rm-298 this cycle, rm-299/rm-286 resolved next.
- **rm-302 (live quota) blocked on rm-201 (still `candidate`).** A
  network + credentials surface cannot be responsibly designed atop
  an unlanded offline surface in the same cycle; rm-201 remains the
  strategic lead for a later cycle with rm-302 as its rider. Demand
  evidence was refreshed (two more same-week entrants), keeping the
  thesis warm without spending implementation.
- **Assess N2 (gate flags under `--overview` only) is EXCLUDED, not
  deferred — it is sibling-claimed.** The roadmap phase verified the
  content IS run ca0b284f's uncommitted rm-246 patch artifact and
  deliberately did not re-mint it. This batch touches nothing in that
  lane; re-selection here would mint a duplicate claim.
- **rm-303 scoped down honestly.** The v0.9.0 section already carries
  a backfill annotation for v0.8.0/v0.8.1, so the prose gap is
  half-closed; the residual is the one-directional (`>=`) version
  gate letting mid-range merged tags stay sectionless forever
  (`check-plugin-version.sh:9-10` re-read this phase). Selected as an
  XS rider for gate red/green, not for the backfill alone — if the
  gate arm grows beyond XS at implement time, drop the rider rather
  than grow the batch.
- **Standing watches unchanged:** OTel GenAI (rm-229) still closed;
  upstream zero movement since pass-11 (residual #294/#291 lanes
  only); npm identity flat; clap 4.6.7 minor bump rides rm-007's
  staging.
- **Board integrity:** this phase selects; it does not edit
  `ROADMAP.md` (the uncommitted +41/-0 is the roadmap phase's
  commit-gate-owned deliverable). Status flips belong to the
  implement/commit phases per the house rule.

## 4. Merge-safety — the live uncommitted fleet (diffed this phase)

| Worktree | Base | Uncommitted file set | Overlap with this batch |
|---|---|---|---|
| run-52465b9e | ce27969 (2 merges stale) | main.rs, lib.rs, reports.rs, search.rs, statusline.rs, discovery_contract.rs | **rm-301**: main.rs only — the announcement lines are a stable, isolated region; routing them to stderr under `-f json` merges trivially over a stale base |
| run-cf755698 | 5ef66c0 (1 merge stale) | insights.rs only | none |
| run-e602bb69 | 9d88b36 (3 merges stale) | README.md, main.rs, upstream.rs, doctor.rs, lib.rs, pricing.rs | **rm-301**: main.rs, same isolated region; same trivial fold |

ROADMAP-only claimers (no code, no overlap): run-75ae7fb6
(rm-304..306 @fd5532f), run-6e96bed5 (rm-284..297 @ec8acdc — holds
the rm-286 overlap that defers rm-299), run-980eb773 (rm-272..283
@1806e18), run-278b2bda + run-e741e661 (per the roadmap mint's
census), and the ca0b284f patch artifact (rm-246/247). Also noted:
worktree registration `run-d675a17700e` points at a directory absent
from disk (stale registration, harmless, no claims). Same-base
fd5532f siblings (2c2db6f5, 27fdc908, 1cb61083, f22ff7d1) are clean;
no sibling prioritization artifact exists for 2026-10-02/03, so there
is no selection-level collision.

## 5. Rejected alternatives

- **rm-298 + rm-300 (top-two fresh findings):** two M units blows the
  one-M-lead landing discipline; each needs red-first fixtures and
  its own real-corpus/gate verification; reviewer load doubles for no
  dependency gain (their files are disjoint, so sequencing costs
  nothing).
- **rm-299 as lead:** highest performance value on the board but
  M+/L, into a lane a sibling already claims (rm-286), building on a
  cache whose size contract is currently false — wrong order even
  absent the claim.
- **rm-302 as rider:** its dependency rm-201 is unlanded; a
  credentials-handling surface rushed as a rider is the opposite of
  the fail-closed discipline this repo just landed (install.sh).
- **Assess N2 (rm-246 content) as rider:** excluded on claim
  collision (§3); the cheap-looking S effort is exactly how duplicate
  implementations become fold debt.
- **rm-303 as the only rider (dropping rm-301):** rm-301 carries a
  MED live PoC breaking downstream `jq` today; rm-303's residual is a
  LOW gate-completeness gap. Both fit; dropping either wastes cycle
  capacity.
- **rm-042 / rm-006 / rm-003 as lead:** all standing M+ candidates
  with no fresh red path on this host this run; the board's only
  live-proven contract violation wins.

## 6. Execution order (red-to-green, for the implement phase)

1. **rm-298 first, fixture-red first.** Build the synthetic
   over-capacity fixture (many entries with long path keys + a large
   dirs map) and assert the WRITTEN file's byte size ≤ cap — red on
   the current binary (the estimator under-counts keys, punctuation,
   top-level fields, and the whole dirs map). Then: budget exactly
   the bytes the writer emits (values + path keys + structural JSON +
   dirs — measure the serialized document or model it exactly),
   give `dirs` its own count and byte bound, keep eviction
   deterministic (oldest fingerprint first, pinned by the test), and
   correct the three "latent" doc claims
   (`docs/decisions/2026-09-14-cycle-7-batch-selection.md:47`,
   `docs/stewardship/2026-09-03-cycle6-independent-review.md:116`,
   and any sibling wording) in the same change. Live gate: fresh
   isolated-`AGENTTRACE_SESSION_CACHE_DIR` run on the real corpus →
   sessions.json ≤ 67,108,864 B byte-exact (baseline red: 72.2 MB).
   Budget note: cold scan is ~54 s — do not mistake it for a hang.
2. **rm-301 second.** Stdout-purity test first: sandboxed cache dir,
   pipe `--clear-cache --overview -f json`, assert first stdout byte
   is `{` and the stream is one JSON document — red today (stdout
   starts `Session cache cleared.`). Then route the
   `--clear-cache`/`--update-pricing` announcements to stderr
   whenever a machine format is requested (human path unchanged),
   covering main.rs:206-218. Verify the deterministic-output and
   output-contract gates stay green (they pin stdout shape).
3. **rm-303 third (XS arm only).** Add the per-tag section arm to
   `check-plugin-version.sh` (or a sibling check): every `^v` semver
   tag merged into HEAD must have a CHANGELOG section or an explicit
   inherited/annotated marker; red on a synthetic missing-section
   fixture, green on the reconciled tree (v0.8.0/v0.8.1 get sections
   or an explicit annotation upgrade). Drop the rider if it grows
   past XS (§3).
4. **Gates (all units):** `cargo fmt --all --check`;
   `cargo clippy --all-targets --all-features -- -D warnings`;
   `cargo test --all-features` (isolated TMPDIR — the known
   non-existent-TMPDIR trap); `cargo build --release`; CI gate
   scripts with `AGENTTRACE_BIN` + `AGENTTRACE_CI_OUT` set
   (deterministic-output, docs-commands, output-contract,
   report-semantics at minimum).
5. **ROADMAP status flips belong to later phases** (implement flips
   candidate→implemented with its compound bullet; done-flip is the
   commit gate's). This phase selected; it did not edit ROADMAP.md.
6. **Sanctioned overflow only:** if the trio is green with budget
   left, rm-300 is the sole sanctioned next unit (red-first with the
   two on-disk PoC fixtures); anything else waits for cycle 2.

Verification anchors: `ROADMAP.md` (rm-298/rm-301/rm-300/rm-303 in
this run's uncommitted mint block), the assess ledger
(`/tmp/at-assess-c22757c9/assessment-report.md` + its capcache/fx
artifacts), the research ledger
(`/tmp/at-research-c52c2cc3/research-report.md`), and the fresh
re-measurements recorded in this document's grounding section.

---

## 7. Audit addendum — attempt b8cfcdc2 (2026-10-03 ~01:05Z): selection ADOPTED unchanged

Forensics first: the authoring attempt e03c70cf **completed** (its typed
result JSON exists in the delegate spool; the events log records
`delegate_turn_completed` / `succeeded` at 00:35Z); the conductor
re-dispatched the phase ~29 min later as attempt b8cfcdc2. Per this run's
adopt-with-disclosure convention (roadmap phase, prior attempt 1ef3db41),
this attempt did NOT redo the selection — it re-verified every claim with
fresh commands and adopts the verdict. ROADMAP.md remains untouched by
this phase (its +41/-0 diff is byte-identical to the roadmap phase's spool
patch, re-checked this attempt).

**Audit result — all green, fresh this attempt:** `stat` on the assess PoC
cache = 72,262,482 B (108.9 % of the 67,108,864 B cap, stable across
rewrites); CHANGELOG headings jump `v0.9.0` (:64) → `v0.7.1` (:79) with no
`v0.8.x`; `session_cache.rs:44-49`'s hard-bound doc vs `enforce_byte_bound`'s
values-only sum vs `save_session_cache`'s schema_version +
dir_listing_version + path-keyed entries map re-read line-for-line;
main.rs:206-218 announcements re-read; check-plugin-version.sh's
one-directional `>=` tag anchor re-read; rm-201 still `candidate`; wall =
113 ids / 78 candidate / 21 done / 14 implemented; ROADMAP header :16 still
names "F3 session-cache 64MiB cap violated live" as the cycle-2 lead (the
designated-lead endorsement); `/tmp/at-roadmap-ca0b284f.patch` still holds
rm-246 ×2 (the N2 exclusion's basis); no sibling agenttrace prioritization
exists for 2026-10-02/03 (only dashboard-repo and old lineage files).

**Corrections — the live fleet moved in the ~30 min after §4 was written:**

- run-cf755698 (5ef66c0) grew from "insights.rs only" to nine modified
  files, including main.rs — but its main.rs diff touches only the import
  list (hunk `@@ -8`) plus render_session_list/tests; **no line of
  :206-218**. rm-301's isolated-region fold claim survives; §4's "none"
  overlap for this lane is corrected to "main.rs, non-overlapping hunks".
  Its CI-script touch is check-rust-release-local.sh, not the version gate.
- run-52465b9e's file set also grew (tui filters.rs, ROADMAP.md, untracked
  render.rs / CONCEPTS.md / a solutions doc) — still no session_cache.rs,
  CHANGELOG.md, or version-gate overlap, and still no :206-218 hunk.
- run-d675a177 (absent-from-disk at §4 time) is restored and porcelain-clean
  at fd5532f — its roadmap deliverable lives as a spool patch; no in-tree
  claims. run-a1cafb4c (5ef66c0) is a new ROADMAP-only claimer. run-e0f302fe
  (ec8acdc) is clean in-tree (patch-in-scratch deliverable).
- Net effect on the batch: **no dirty worktree — sibling lanes or this
  run's own — touches `session_cache.rs`, `CHANGELOG.md`, or
  `scripts/ci/check-plugin-version.sh`.** rm-298's zero-contention claim and
  rm-303's gate-arm claim both survive the fresh census; rm-301's overlap is
  now two stale-base main.rs lanes, both with non-overlapping hunks.

**New fact post-dating this document (00:36Z > 00:35Z):** sibling run
e0f302fe's roadmap attempt d828a642 delivered a patch (vs ROADMAP.md at
ec8acdc) whose ids **rm-298..rm-304 carry different content** (display-label
table, negative cache, CLI help blank rows, npm identity, line-resilient
parsing, local-Today, zh-CN parity) — an OPEN id collision with this run's
rm-298..rm-303, the same shape as the recorded 278b2bda/980eb773
rm-272..276 collision. House rule applies: record-not-renumber here;
whichever campaign's commit gate lands first keeps the ids and the other
renumbers at integration. The selection is content-keyed (mint title +
signals, not bare ids), so the verdict is unaffected — the implement phase
should reference the selected items by title + signals alongside ids, and
the commit gate owns any renumber.

**Verdict reaffirmed:** cycle-1 batch = rm-298 (lead, M) + rm-301 (rider, S)
+ rm-303 (rider, XS) on the theme "the cache obeys its own contract, and
the pipe stays pure" — §1–§6 stand as written with the §4 census
corrections above folded in.
