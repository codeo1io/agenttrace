---
artifact_contract: "ce-review/v1"
created_at: "2026-09-02T12:55:00Z"
title: "Independent adversarial review — cycle 3 batch CU-1..CU-5 (trustworthy strings on untrusted input) plus the full-tests determinism fix"
summary: "Verdict: pass_with_findings. All five change units are implemented as claimed and every load-bearing piece of the implement/targeted/full-test evidence reproduces on this machine (180/180 debug and release, fmt/clippy clean, 8/10 check scripts with only the pre-existing absent-expect gap, fuzz sweep of 9,000 hostile lines with zero non-standard exits, cross-process cache-race probe with zero temp orphans, live-DB naming census 215 provider_title / 12 provider:placeholder). Three residual defects found inside the batch's own new surface: the CU-5 naming-semantics change did not bump the SQLite snapshot schema version, so warm pre-CU-5 v5 snapshots keep serving verbatim placeholder names indefinitely (reproduced); the lone-surrogate repair treats every backslash-u byte pair as an escape without escaped-backslash awareness and can silently rewrite literal text on already-corrupt lines (reproduced end to end); and per-writer temp suffixes are never swept, so crashed writers leak orphans the old fixed name used to self-heal. None blocks the cycle goals."
keywords: ["agenttrace", "independent-review", "cycle-3", "utf-16-repair", "cjk", "placeholder-title", "cache-schema"]
run: "5d025d55b1194dd1a4dd8784146dfeeb"
attempt: "03e800929c854cafbf2b51544b693af9"
repo_root_sha: "e0059522b4fc74d53824f0e7ea7e4ac94d1465bb"
tree_state: "dirty (cycle-1 + cycle-2 + cycle-3 uncommitted; nothing committed/pushed, per delegation policy)"
---

# Independent adversarial review — cycle 3 (CU-1..CU-5 + determinism fix)

Reviewed against: the cycle goals in
`docs/stewardship/2026-09-02-cycle3-prioritization.md` and the cycle-3
stewardship request; `ROADMAP.md` acceptance criteria for the batch items
(P6-1 UTF-16 escape repair, P6-4 honest fallback token accounting, P6-3
unique cache temp suffix, P6-2 `--version` claim integrity, candidate 34
placeholder-title gate — hardening lane entries and capability-lane
candidate 34); security boundaries (read-only SQLite, offline-by-default,
no new dependencies, untrusted-input containment); durability/recovery
(SQLite snapshot schema v5, WAL/SHM fingerprinting, atomic cache writes,
concurrent writers); and the test evidence claimed by the implement,
targeted_tests, and full_tests phases, including the in-phase
deterministic-output regression fix.

Routing disclosure: narrowest installed skill for this phase is
**ce-code-review** (in-thread execution; no subagent surface in this
delegate environment). The review ran every check itself against the
current dirty tree; nothing below is transcribed from earlier artifacts.

## Verdict

**pass_with_findings.**

The batch does what the cycle selected. The last reproducible-panic HIGH
(P6-1) is closed with its own committed corpus, both crash sites are gone
from the code, and the fix is byte-oriented exactly as the acceptance
required — a 9,000-line randomized sweep of backslash/multibyte/hex soup
produced zero non-standard exits across three actions on both the debug
and release binaries. CU-2's estimator is character-correct end to end
(8 CJK chars → 8 tokens in, was 6; `reasoning_chars` pinned to
characters). CU-3's hoist makes the CHANGELOG claim true and is pinned in
both flag orders. CU-4 survives a 10-process cross-process race with zero
temp orphans — stronger than the committed 8-thread in-process test. CU-5
verifies against the live database (215 `provider_title`, 12
`provider:placeholder`, message-derived names visible). The in-phase
determinism fix holds under 5x stress.

Three residual defects sit inside the newly delivered surface (M1, L2, L3
below) plus four info items. M1 is the one that matters: the CU-5
naming-semantics change shipped without a snapshot-schema bump, so a warm
cache written by the pre-CU-5 binary keeps serving exactly the junk names
the unit was supposed to eliminate — for quiet databases, indefinitely.
None of the three is a crash, a panic, a negative number, a network touch,
or a durability regression relative to HEAD; none falsifies an acceptance
criterion as literally written (the fixture tests all pass on fresh
paths); M1 undercuts CU-5's user-visible goal for warm caches and should
lead the follow-up queue.

## Evidence re-verification (all run this phase, dirty tree at e005952)

| Claim (source phase) | Re-verified | Result |
|---|---|---|
| debug suite green | `cargo test --workspace` | **180 passed, 0 failed** (record's 179 + the full_tests determinism test) |
| release suite green | `cargo test --workspace --release` | **180 passed, 0 failed** |
| fmt/clippy clean | `cargo fmt --all --check`; `cargo clippy --workspace --all-targets` | clean / **0 warnings** |
| P6-1 no exit 101 anywhere | pass-6 reproducers rebuilt from scratch (`printf '{"prompt":"\\u中文测试"}'`, `\\ud800\\u中文测试`) through `--overview/--doctor/--waste/--latest/--sessions/--diagnostics/--audit/--context-trends`, positional and `-d` forms, release and debug | **no exit > 1**; positional hostile file = clean `Error: unsupported session format`; all-hostile dir = clean `No sessions match the requested filters`; mixed dir = **exit 0 on all six actions** with `--doctor` labeling both hostile files (`failed: repro1.jsonl`, `failed: repro2.jsonl`) while 8 clean neighbors load |
| P6-1 random-blinded panic hunt | 6 files x 500 random lines (seed 20260902; alphabet of `\`, `u`, hex, `中文`, `😀`, quotes, braces) x 3 actions x release binary | **9,000 hostile lines, zero exits outside {0,1,2}, zero panics** |
| P6-1 pairs still repair | committed unit tests + `parse_jsonl_value_lenient` end-to-end test | pass; lone `\ud800` → U+FFFD, `\ud83d\ude00` survives as 😀, `\uzzzz`/truncated pass through untouched |
| CU-2 CJK end-to-end | claude_code fixture `"中文测试中文测试"` (user) + `"好的"` (assistant), no usage block | **Input 8 tokens / Output 2 tokens** (chars; bytes/4 would give 6 and 1); `provenance.tokens = estimated_from_text` preserved |
| CU-2 reasoning unit | `reasoning_chars_counts_characters_not_bytes` | pass (4 chars, not 12 bytes) |
| CU-3 both orders | `--overview --version`, `--version --overview`, `--lang zz --version` | all **exit 0**, banner printed |
| CU-4 in-process race | committed `concurrent_persist_never_races_on_a_shared_temp_file` (8 writers) | pass |
| CU-4 cross-process race (beyond the committed test) | 10 parallel release binaries, isolated `AGENTTRACE_SESSION_CACHE_DIR`, `-d` mixed corpus, then a post-race load | **0 `.tmp.*` orphans**, cache valid JSON, post-race run exit 0 |
| CU-5 live census | snapshot histogram after regeneration | **215 `provider_title` / 12 `provider:placeholder`** of 227 — matches the implement record's census; message-derived names visible in `--sessions` output; `Naming` round-trips the v5 snapshot (PascalCase key) |
| CU-5 fixture test | `opencode_placeholder` contract test + placeholder/real/empty-title fixture | pass |
| Determinism fix | `check-deterministic-output.sh` x5 with sleep-1 gaps (the flaking pattern) | **5/5 PASS**; `--demo` JSON pins `2026-05-02T10:36:00Z`; a real-data run stamps wall clock (`2026-09-02T12:33:15Z`); text/markdown/html demo outputs contain no timestamp (html's "generated from" is static prose) |
| Check scripts | all ten with release binary + `AGENTTRACE_CI_OUT` | **8 pass**; the two failures are solely `command -v expect` (pre-existing P4-5) |
| No dependency motion | `git diff HEAD -- '**/Cargo.toml' Cargo.lock` | **empty** |
| Read-only SQLite | `open_sqlite_read_only` (`sqlite_sessions.rs:150-155`) | still `READ_ONLY\|NO_MUTEX`, unchanged |

## Findings

### M1 (medium) — CU-5 shipped without a snapshot-schema bump, so warm pre-CU-5 caches keep serving placeholder names

`SQLITE_SNAPSHOT_SCHEMA_VERSION` is still `5`
(`crates/agenttrace-core/src/session_cache.rs:9`), the version cycle 2
introduced. CU-5 changed the *meaning* of a snapshot's `Name` and added
`Provenance.Naming`, but snapshot freshness
(`session_cache.rs:184-201`) checks only schema version + db/wal/shm
fingerprints — none of which change on a binary upgrade. A v5 snapshot
written by the pre-CU-5 binary therefore remains fresh, and the entire
placeholder gate (`sqlite_sessions.rs:727-745`) never executes for its
sessions: they surface with verbatim `New session - <timestamp>` names
and empty `Naming` (the field deserializes as `""` via
`#[serde(default)]`, `lib.rs:243-245`).

Reproduced this phase: quiet-DB sandbox (`/tmp/at-review-c3/stale4/`,
db copied with mtime preserved, no wal/shm, snapshot planted with
placeholder `Name`s, no `Naming` key, schema 5, matching fingerprint) —
the current release binary **served `New session - 2026-08-16T19:59:18.000Z`
three times** from cache. The fix is one line: bump to 6 so pre-CU-5
snapshots regenerate (exactly the mechanism cycle 2 used for its stored-
totals semantics change). Mitigating factor, not a defense: on live WAL
databases the `-shm` mtime advances whenever opencode writes, which
eventually refreshes the snapshot — but on a quiet or checkpointed
database the stale names persist indefinitely, which is precisely the
user-visible defect C34 exists to remove.

### L2 (low) — the escape repair mis-parses `\\` + `udXXX` literal text on already-corrupt lines

`repair_lone_surrogates` (`crates/agenttrace-core/src/parser.rs:3796`)
advances byte-by-byte and treats **every** `\` followed by `u` as an
escape start (`parser.rs:3801`), without escaped-backslash awareness: in
JSON text `\\ud800` the first `\` is consumed as a character, and the
*second* `\` + `u` + `d800` then looks like a surrogate escape even
though textually it is the tail of an escaped backslash plus the literal
characters `ud800`. When the line is invalid JSON for a *different*
reason (the repair only runs after strict parse fails,
`parser.rs:3772-3775`), the repair rewrites that literal text:
`ud800` → `ufffd`.

Reproduced end to end through a real lenient parser
(`parse_claude_code_jsonl`, `parser.rs:2132`): a claude_code line whose
content is `A\ud800 B\\ud800 tail` (real lone surrogate + escaped
backslash + literal) surfaces with session name **`A? B\ufffd tail`** —
the literal `ud800` text became `ufffd`. `/tmp/at-review-c3/probe2/cc.jsonl`.
Impact is confined to lines that were already invalid, cannot crash, and
only mutates display text (and estimates derived from it), so low; but it
is a string-integrity defect inside the batch whose theme is string
integrity. Fix: track the preceding byte — skip the `\u` interpretation
when the backslash run before it is odd — or repair only the escapes
serde actually rejected.

### L3 (low) — crashed writers now leak unique temp orphans the fixed name used to self-heal

`unique_temp_path` (`crates/agenttrace-core/src/session_cache.rs:237-247`)
suffixed `<name>.tmp.<pid>.<seq>` fixed the concurrent-writer race (CU-4,
verified), but a writer killed between `fs::write` and `fs::rename`
(`session_cache.rs:226-228`, `:535-537`) leaves an orphan nobody cleans:
the old fixed `<name>.json.tmp` was overwritten by the next save, bounding
leakage at one file; per-writer names make it unbounded (one
snapshot-sized file per crash, e.g. ~330 KB each for opencode). Verified
no cleanup path exists (only the test asserts absence of leftovers on the
success path, `session_cache.rs:864-869`; current real cache dir has 0
orphans). Suggested: sweep same-pid stale temps at persist time, or prune
`.tmp.*` files whose owning pid is gone on load.

### I4 (info) — CHANGELOG census citation reads stronger than today's live state

`CHANGELOG.md:14` says the placeholder populates "227/227 in the live
census". That is accurate as a citation of research pass 5, but at
implementation time only 12/227 still carried the placeholder (215 have
provider summaries, which correctly win) — the implementation record
discloses the nuance the changelog doesn't. A parenthetical "(at census
time)" would keep the entry future-proof.

### I5 (info) — clap-level parse errors still precede `--version`

`--overview --gate 200 --version` and `--overview --format bogus
--version` exit **2** (clap's `value_parser` fires inside
`Args::parse_from`, `main.rs:141`, before the hoisted early-return at
`main.rs:149-153`). The P6-2 acceptance ("version early-return wins over
action validation, pinned by a CLI test") is met and the CHANGELOG's two
worked examples are true; unknown *values* are simply outside the
claim's scope. No action needed beyond awareness.

### I6 (info) — generic JSONL sessions still silently drop lone-surrogate lines

`parse_jsonl_session` (`lib.rs:383-387`) parses strictly and `continue`s
on failure, so a lone-surrogate line is dropped from a *generic* JSONL
session while the format-specific lenient parsers (claude_code, qwen,
oh-my-pi, copilot, codex rollouts — all `jsonl_objects` callers) repair
it. Pre-existing asymmetry; CU-1's acceptance (never fatal) is met, but
the "strings are trustworthy" story is only half-true on this path. Worth
folding into a future lenient-parse unification item.

### I7 (info) — roadmap retirement pending, by design

`ROADMAP.md` still narrates P6-1/P6-4/P6-3/P6-2/C34 as open items; the
implementation record explicitly defers retirement to a later roadmap
phase (same separation as cycle 2's I6). The final validation/commit gate
should fold closure status back so the next cycle doesn't re-select
closed items.

### I8 (info) — snapshot freshness can be defeated-or-refreshed by `-shm` mtime motion

On the live WAL database the `-shm` mtime advanced between a snapshot
write and a subsequent read-only run (observed twice this phase; a later
run left it stable). That is inherent to WAL fingerprinting and errs
toward re-querying rather than serving stale data — acceptable, but it is
also what makes M1 intermittent on active machines instead of obviously
broken, which is how it survived the implement phase's live-DB check
(that check ran after the snapshot had regenerated).

## What was checked and found sound

- **CU-1 boundary safety, by construction**: every slice in
  `hex_escape_u16`/`repair_lone_surrogates` is byte-indexed and
  length-guarded (`i+6 <= len`, pair `i+12 <= len` short-circuited);
  `i` only ever advances over ASCII bytes or whole `char`s, so `line[i..]`
  stays boundary-safe; the pair fast-path's `line[i..i+12]` is safe
  because both hex groups are ASCII by the time it runs. The `\\`-prefix
  case (L2) mutates content but cannot panic.
- **CU-2 arithmetic**: the estimator accumulates into saturating `u64`
  before a single `/4` — even `u64::MAX` scaled (≈4.6e18) cannot exceed
  `i64::MAX` on the cast; ASCII and mixed cases pinned exactly
  (`"12345678"`→2, `"café 中文"`→4); both estimate sites
  (`lib.rs:586`, `:611`) and the reasoning site (`:597-598`) route through
  the one function; no `len()/4` estimator remains anywhere in the
  workspace.
- **CU-3**: the hoist precedes `validate_primary_action`,
  `validate_gate_thresholds`, the markdown/html gating, and
  `report_language`; committed test pins both orders with the real
  binary.
- **CU-4**: both persist sites (`session_cache.rs:226`, `:535`) use the
  per-writer suffix; rename target and atomicity unchanged; per-process
  counter is `AtomicU64`, no cross-writer collision possible.
- **CU-5**: gate is prefix-exact (`starts_with("New session - ")`),
  real titles win, empty-title and placeholder both fall back
  `first_user_text → display_title_from_text → agg.id`; the
  part⋈message join (`sqlite_sessions.rs:462-507`) parses per-row and
  skips non-`user`/non-`text` rows without failing the load; naming
  provenance emits all four SQLite-side values and the JSONL side's
  `first_user_request`/`file_name` (`lib.rs:449-454`); live-db full load
  (`--overview --source opencode_db -f json`, 227 sessions, 144 MB db)
  runs in **0.39 s** — no measurable join regression, and the result is
  snapshot-cached.
- **Determinism fix**: pinned epoch const `demo.rs:35` re-exported and
  threaded only into the `--demo` JSON context path
  (`main.rs:350`, `reports.rs:494-501`); text/markdown/html renderers emit
  no generation timestamp; real runs stamp the wall clock; committed
  `demo_overview_json_is_byte_deterministic` test plus the CI script
  (5/5 under load).
- **Claim integrity sweep of the new CHANGELOG entries**: every
  behavior bullet checked against observed behavior this phase (escape
  repair, CJK rates, `--version` precedence, placeholder naming, unique
  temps, demo epoch) — all true as written except the I4 census nuance.

## Reproducers (kept for the follow-up implementer)

- `/tmp/at-review-c3/stale4/` — M1 (planted pre-CU-5 v5 snapshot, quiet
  DB, placeholder names served from cache)
- `/tmp/at-review-c3/probe2/cc.jsonl` — L2 (escaped-backslash literal
  rewritten in session name)
- `/tmp/at-review-c3/fuzz/f0..f5.jsonl` — randomized panic sweep inputs
- `/tmp/at-review-c3/repro1.jsonl`, `repro2.jsonl`, `mixed2/` — P6-1
  reproducers and mixed directory
- `/tmp/at-review-c3/conc/` — cross-process cache-race sandbox
- `/tmp/at-review-c3/home/`, `home2/` — sandboxed live-db copies (fresh
  vs WAL-bearing)

## Recommended follow-up ordering

1. **M1** — one-line schema bump to 6 (+ a stale-snapshot regression
   test planting a v5 placeholder snapshot, mirroring this review's
   reproducer). Highest value per line in the batch.
2. **L2** — backslash-run awareness in `repair_lone_surrogates` (+ unit
   case with `\\ud800` literal).
3. **L3** — same-pid temp sweep at persist time.
4. I4/I7 — wording + roadmap retirement at the next stewardship pass;
   I6 folds into the lenient-parse unification lane.
