---
artifact_contract: "ce-review/v1"
created_at: "2026-09-02T16:05:00Z"
title: "Independent adversarial review — cycle 4 batch CU-6..CU-10 (truthful reads, truthful gates, durable records)"
summary: "Verdict: pass_with_findings. All five change units implement what the cycle records claim, and every load-bearing claim reproduces on this machine: 189/189 debug tests, fmt and clippy -D warnings clean, 4/4 runnable CI check scripts, and live release-binary reproductions of the P7-1 lone-surrogate recovery, the P7-2 BOM parse and UTF-16 named error, the P7-3 exit-2 baseline gate, the history quarantine, and the orphan-temp sweep. Seven findings, none blocking: (1) CU-6's acceptance is only partially met — single-JSON-object files carrying a usage key are claimed by parse_gemini_value before the generic fallback and still silently drop the whole message (the recorded pass-7 reproducers evusage.jsonl and strusage.jsonl still report 'Messages: 0 user'), a disclosed residual whose root cause this review pins; (2) --no-baseline-gate is misregistered as a value flag in the Go-compat shim (main.rs:570), so any invocation with the flag before other flags silently drops them — live-reproduced; (3) the temp sweep deletes any file whose name merely contains '.tmp.' rather than the writer's exact pattern; (4) history's own orphan temps are never swept; (5) repeated corruption overwrites the previous .corrupt quarantine; (6) the dropped-lines signal is format-asymmetric (claude-format detectors still skip silently while the generic path counts and flags confidence low); (7) the scoped fixture evidence (per-family BOM/UTF-16 fixtures, generator script) was not delivered and the deviation is undisclosed in the record."
keywords: ["agenttrace", "independent-review", "cycle-4", "baseline-gate", "bom", "utf-16", "line-skips", "orphan-sweep", "quarantine"]
run: "2a15625945fc40419fc4691c59b42a7b"
attempt: "83be37e96e2148249e2cabb490b5f7f6"
repo_head: "66320145ae38163bce90b45668e3e4afd95d3c2a"
tree_state: "dirty (cycle-4 implementation uncommitted on HEAD 6632014; nothing committed/pushed, per delegation policy)"
---

# Independent adversarial review — cycle 4 (CU-6..CU-10)

Reviewed against: the cycle goals in
`docs/stewardship/2026-09-02-cycle4-prioritization.md` and the cycle-4
stewardship request; the implementation record
(`docs/stewardship/2026-09-02-cycle-4-implementation-record.md`) and its
verification matrix; `ROADMAP.md` acceptance criteria for the batch
items (P7-1 no-silent-data-loss extension, P7-2 BOM at every parse
entry, P7-3 baseline thresholds gating the exit code, the
cache/history durability entry extended by P7-5, and the cycle-3
residuals entry — hardening lane); security boundaries (offline by
default, no new dependencies, untrusted-input containment); durability
and recovery (atomic writes, quarantine, orphan sweep, snapshot schema
versioning); and the test evidence claimed by the implement,
targeted-tests, and full-tests phases.

Routing disclosure: narrowest installed skill for this phase is
**ce-code-review**; this harness exposes no subagent surface, so all
personas ran in-thread as lenses of one reviewer, every claim
re-executed first-hand (no finding rests on the records alone). The
cross-model pass was skipped: host-family attestation is unavailable in
this environment (same disclosure as cycles 1–3). Roster actually run:
correctness, testing, maintainability, security, reliability,
adversarial, api-contract. Project-standards persona skipped (no
CLAUDE.md/AGENTS.md in repo); learnings phase skipped (no
docs/solutions corpus yet). Run artifact dir:
`/tmp/compound-engineering-1000/ce-code-review/20260902-151502-f3c6a556`.

## Scope

- BASE = HEAD `6632014` (workflows-only atop pass-7 `93aaf05`); the
  reviewed diff is the entire uncommitted working tree: 15 files,
  +883/−110, 657 executable lines, plus untracked
  `testdata/generated/adversarial/generic-loss.jsonl`,
  `docs/guides/ci-integration.md` changes, and the cycle-4 stewardship
  records themselves (records reviewed for accuracy, not re-reviewed as
  code).
- Probes ran against `target/release/agenttrace` (rebuilt and confirmed
  current) under an isolated `HOME`/`XDG_DATA_HOME`/`XDG_CACHE_HOME`
  sandbox at `/tmp/ir4`, with the pass-7 reproducer set `/tmp/pri4`
  (still present on this host) replayed against the fixed binary.

## What was independently verified (all reproduced first-hand)

| Claim in the cycle records | Method | Result |
|---|---|---|
| 189/189 debug tests | `cargo test -p agenttrace-core -p agenttrace-tui -p agenttrace` fresh run | **189 passed, 0 failed** (12+3+2+64+7+61+40) |
| fmt / clippy clean | `cargo fmt --all -- --check`; `cargo clippy --workspace --all-targets -- -D warnings` | Clean / clean |
| CI check scripts | `check-output-contract`, `check-report-semantics`, `check-docs-commands`, `check-deterministic-output` with `AGENTTRACE_BIN` | 4/4 PASS |
| P7-1 lone-surrogate recovery | `/tmp/pri4/mix3.jsonl` → `--latest` | **3 user messages** (was 2); no `line_skips`; unrecoverable-line sibling reports `Dropped lines: unparseable_line=1` + confidence low |
| P7-1 usage coercion | multi-line generic files with Event-typed / string-typed / flat usage; committed fixture | messages recovered; meta-line usage counted (`tokens: 7`); message-role usage stays text-estimated — matches the recorded pre-existing `meta`-only accounting contract (SAMPLE_JSONL) |
| P7-2 BOM | BOM'd vs BOM-less identical claude-shape file, full JSON diff | Parses; only diff is the directory name — **byte-equivalent output** |
| P7-2 UTF-16 | `/tmp/pri4/utf16.jsonl` | exit 1, `Error: session file utf16.jsonl is UTF-16 encoded; convert it to UTF-8 and retry` |
| P7-3 gate | forged `base-zero.json` + `--baseline-max-token-delta-pct 1` | exit 2 after the report JSON is written; stderr names thresholds and the opt-out; `--no-baseline-gate` (trailing) restores exit 0; no-baseline run exits 0 |
| CU-9 quarantine | torn `history.json` + `--include-history` | renamed `history.json.corrupt`, visible warning, history starts empty (bytes preserved) |
| CU-9 sweep | backdated orphans in cache dir | `sessions.json.tmp.999.0`, `pricing.json.tmp.999.0` removed; fresh `sessions.json` untouched |
| CU-10 backslash parity | logic trace of the guard + in-repo unit tests; `\\ud800` literal preserved, adjacent real escape still repaired, odd/even backslash runs, trailing backslash | Correct; unit-pinned |
| CU-10 schema 5→6 | version test in tree; v5 snapshots rejected and regenerated | As recorded |
| No perf regression on the new lenient fallback | 40k-line generic JSONL with 40 lone-surrogate lines (2.1 MB) | full `--overview` in **0.49 s** |
| `1e400` hostile float | single-line file with `"input_tokens": 1e400` | whole-file skip with the standard message, no panic, no hang |

## Findings

### F1 — MEDIUM — CU-6 acceptance only partially met: single-object files with a `usage` key still silently lose the message

The roadmap acceptance for the P7-1 extension reads: "the pass-7
reproducers (lone-surrogate line, string-typed usage, Event-typed usage)
**each counted in `data_health` with a reason** instead of dropped."
Two of the three recorded reproducers are still dropped, silently:

- `/tmp/pri4/evusage.jsonl` (single line
  `{"role":"user","message":"ev","usage":{"input":{"tokens":5}}}`) →
  fixed binary prints **`Messages: 0 user`**, session named from the
  filename, `source_tool: ""`, `line_skips` absent, `skipped: 0` —
  parsed-healthy on the surface, message gone.
- `/tmp/pri4/strusage.jsonl` (single line, string-typed usage) → same.
- Any single-line JSONL whose object carries a `usage`,
  `usageMetadata`, or `tokenUsage` key of object shape hits this:
  `{"role":"user","content":"x","usage":{"input_tokens":7}}` as a
  one-line file yields 0 user messages (verified with several shapes).

Root cause, pinned (the implementation record discloses the behavior at
implementation-record.md:74/:82 but not the mechanism): a
single-object file is successfully strict-parsed as a `serde_json`
`Value`, so `parse_raw_session` runs the value-detector chain
(crates/agenttrace-core/src/parser.rs:145) **before** the generic
fallback (parser.rs:165). `parse_gemini_value` (parser.rs:152) claims
any object that has one of the three usage keys
(crates/agenttrace-core/src/parser.rs:2727), and `gemini_usage`
(crates/agenttrace-core/src/parser.rs:3538) returns `Some(usage)`
**unconditionally** for any object — even an all-zero map — so the
claim always succeeds. `parse_gemini_object` then emits only a `meta`
event from the usage block; the `role`/`content`/`message` fields on
that same line are discarded with no skip accounting (the generic
fallback, where CU-6 added accounting, is never reached).

Multi-line equivalents recover correctly (CU-6's own fixture and my
two-line probes), which is why the new contract test passes — the
fixture is multi-line by construction.

Why MEDIUM and not higher: the residual is **accurately disclosed** in
the implementation record (decision 5 + provisional-work entry) and is
out of the batch's declared surface; the detectors' ordering is
pre-existing code the cycle did not touch. Why not lower: the cycle's
acceptance sentence is literally unmet for 2 of 3 named reproducers,
the loss is silent (no `line_skips`, `skipped: 0`), and today the
residual lives only in the implementation record's provisional-work
list — it must be pinned into the roadmap's open hardening lane at
close-out, with the `gemini_usage` unconditional-`Some` mechanism
noted so the next pass doesn't re-derive it. Suggested fix direction
for the next cycle: require `gemini_usage` to return `None` for
all-zero maps (`non_empty_usage` already exists, parser.rs:3630), or
prefer the generic fallback when the object also carries `role` +
`content`/`message`.

### F2 — MEDIUM — `--no-baseline-gate` misregistered as a value flag; preceding placements silently drop the rest of the command line

`crates/agenttrace-cli/src/main.rs:570` lists `"--no-baseline-gate"` in
`flag_takes_value`, whose every other entry is a genuine value flag
(`-f`, `-d`, `--baseline`, …). The Go-flag-compat shim therefore
consumes the **next token** as this boolean's value. Live-reproduced on
the release binary (isolated env):

- `agenttrace --no-baseline-gate -d <dir> --overview -f json -o out.json`
  → the shim eats `-d`; `<dir>` becomes the first positional, the loop
  stops, and **`--overview -f json -o` are all silently dropped**;
  the TUI default action then runs and dies with `Error: stdout is not
  a terminal`, exit 1. No output file, no hint why.
- `agenttrace --no-baseline-gate --baseline X -d <dir> --overview …` →
  `--baseline` eaten, `X` becomes positional → `Error: --baseline
  requires --overview -f json`, exit 1.
- Same flags with `--no-baseline-gate` last → works (17 sessions).
- Control without the flag → works, gate fires with exit 2 as designed.

This is the flag the gate's own error message advertises as the remedy
(`opt out: --no-baseline-gate`), and "boolean first, then the command"
is the most natural ordering — a user following the error message into
a rewritten command has a good chance of landing here. Introduced by
CU-8 (all other entries pre-exist). Fix is one line: remove
`"--no-baseline-gate"` from the `flag_takes_value` match, and add a
CLI test placing a boolean flag before a value flag. (The records'
note that the TUI/lang matrix couldn't run on this host — `expect`
missing — is exactly the surface that would have caught this.)

### F3 — LOW/MEDIUM — the orphan sweep deletes by substring, not by the writer's pattern

`crates/agenttrace-core/src/session_cache.rs:286` sweeps any
directory entry whose **name merely contains `".tmp."`** and is older
than 1 h. Live-reproduced: `keep.tmp.old` and `notes.tmp.bak` — files
with no relationship to agenttrace's `<name>.json.tmp.<pid>.<seq>`
temp scheme — were deleted from the cache directory on the next cache
load. The CHANGELOG (CHANGELOG.md:10) claims only
"`<name>.json.tmp.<pid>.<seq>` siblings are swept", i.e. the code is
destructively broader than the documented and tested behavior (the
record's evidence used `unrelated.txt`, which contains no `.tmp.` and
therefore survives — the probe that would have caught this). Blast
radius is bounded by the directory being agenttrace-owned by default,
but `AGENTTRACE_SESSION_CACHE_DIR` can point anywhere the user
chooses, and the sweep runs over the whole directory on every load.
Recommend matching the writer's exact shape (prefix
`<file>.json.tmp.` with numeric pid/seq suffix), which also makes the
CHANGELOG sentence true.

### F4 — LOW — history's own orphan temps are never swept

CU-9 is scoped "atomic writes + orphan sweep (pricing/history)", but
`sweep_orphaned_temps` is only invoked from `load_session_cache`
(session_cache.rs:327), i.e. over the session-cache directory. Pricing
temps happen to share that directory and are covered; `history.json`
lives in the data dir (`XDG_DATA_HOME`/`AGENTTRACE_HISTORY_DIR`) and
its `<name>.tmp.<pid>.<seq>` orphans from crashed writers are
**permanent** — live-verified: a backdated `history.json.tmp.999.0`
survives every subsequent run indefinitely. The CHANGELOG sentence
"swept whenever the session cache loads" is technically precise about
the trigger but the CU-9 unit framing ("pricing/history") overstates
coverage. Low because the orphans are inert garbage, not corruption.
Recommend a sweep call at the history read/write site or documenting
the cache-dir-only scope explicitly.

### F5 — LOW — repeated corruption overwrites the previous quarantine

`crates/agenttrace-core/src/history.rs:83` renames a torn
`history.json` to the fixed name `history.json.corrupt`. Live: a
second torn write **replaced** the first quarantine's bytes
(`{"torn": ` destroyed by `{"torn2": `). Decision 6 in the
implementation record says "the bytes are evidence; deleting them
would repeat the silent-loss mistake" — but the fixed name deletes
exactly that evidence on the second occurrence. Recommend a unique
quarantine name (timestamp/counter suffix).

### F6 — LOW — the dropped-lines signal is format-asymmetric and flags well-formed files

Two directions of the same asymmetry, both live-verified:

- Claude-code-format files (e.g. `/tmp/pri4/bom.jsonl`'s family) still
  skip non-matching object lines inside
  `parse_claude_code_jsonl` (crates/agenttrace-core/src/parser.rs:2142)
  with **no** `line_skips` accounting — the same content shape that
  the generic path now reports.
- Conversely, hermes/generic files with legitimate metadata lines now
  count them as `non_event` (lib.rs:457) and cap `confidence` at
  `low` (insights.rs:346-348): a well-formed 3-line hermes-shaped file
  whose only "anomaly" is a `{"summary":…}` line reports
  `Dropped lines: non_event=1 | confidence: low`. Truthful per-line,
  but a healthy file now reads as degraded, and which of the two
  signals you get depends purely on which detector claims the file.

Not blocking — the cycle's truthfulness bar is met per-path — but the
next pass should either extend counting to the claude-format detectors
or exempt `session_meta`/known metadata shapes from the confidence
cap, so the signal means the same thing across formats.

### F7 — LOW — scoped fixture evidence not delivered, deviation undisclosed

The stewardship request scopes CU-6 fixtures as "generator sibling of
`scripts/fixtures/make-adversarial-sqlite.py`"
(docs/stewardship/2026-09-02-cycle-4-stewardship-request.md:122) and
CU-7 evidence as "fixture variants per corpus family" (:162); the
roadmap BOM entry's evidence bar is "a UTF-8-BOM variant of one
committed corpus per family and a UTF-16LE transcript". Delivered: one
hand-written `testdata/generated/adversarial/generic-loss.jsonl`, with
BOM/UTF-16 cases constructed inline in unit tests rather than as
committed per-family fixtures, and no generator script. The
**behavioral** acceptance (single strip at offset 0, BOM≡BOM-less
output — which I verified byte-for-byte — and a named UTF-16 error) is
met and pinned by tests, so this is a plan-conformance gap, not a
correctness one; but the implementation record does not disclose the
deviation, which matters in a loop whose currency is record accuracy.
Close-out should either commit the per-family variants + generator or
record the substitution explicitly.

### Handoff notes (not defects)

- `testdata/generated/adversarial/generic-loss.jsonl` is untracked;
  `discovery_contract` depends on it — the commit step must include it
  or CI goes red on a clean checkout.
- `docs/guides/ci-integration.md:126` correctly documents exit-2 gate
  semantics; `check-docs-commands` passes.
- The no-baseline "labeled skip" wording in the roadmap acceptance is
  implemented as "no comparison, no gate" (exit 0, nothing emitted) —
  defensible reading, worth one sentence at close-out.
- The record's honesty correction on pass 7 (string-typed usage never
  reached the P7-1 drop surface) checks out against the code — with
  the caveat that the surface it *did* reach drops the whole message
  (F1), which is the stronger fact and should lead the next pass.

## Requirements traceability

| Unit | Roadmap acceptance | Verdict |
|---|---|---|
| CU-6 (P7-1) | Fallback routes through lenient parser + coercion | **Met** (verified live, fixture + tests) |
| | Skip-reason counts surfaced | Met (`unparseable_line`/`event_schema`/`non_event`, JSON+text+MD+HTML) |
| | "each pass-7 reproducer counted instead of dropped" | **Partial** — lone-surrogate ✅, single-line Event-typed/string usage ❌ (F1, disclosed) |
| CU-7 (P7-2) | One BOM strip at shared entry, offset 0 only | Met |
| | UTF-8-BOM ≡ BOM-less | Met (byte-verified) |
| | UTF-16 named encoding error | Met |
| | Per-family committed fixtures | **Not delivered** (F7) |
| CU-8 (P7-3) | Breach → exit 2, report still printed | Met |
| | Opt-out flag | Met in substance; **shim bug** (F2) |
| | Guide shows failing exit; CLI tests pin | Met (ci-integration.md:126; entrypoints tests) |
| CU-9 (P7-5) | Atomic writes (pricing, history) | Met |
| | Torn history quarantined w/ visible warning | Met (repeat-overwrite nit F5) |
| | Orphan sweep | Partial — cache+pricing dir only (F3 scope, F4) |
| CU-10 (residuals) | Backslash-parity guard + corpus line | Met (unit-pinned; inline line, not committed corpus) |
| | Snapshot schema bump-or-compatible, version test | Met (5→6, v5 rejected, rationale comment) |
| | Orphan sweep on load | Met (with F3/F4 caveats) |

## Verdict

**pass_with_findings.** Every claim in the cycle records that this
review could execute reproduces; the five units do what they say, the
new tests are real red→green surface, and no security, durability, or
correctness defect was found inside the batch's named scope that would
block close-out. Two findings warrant action at or before close-out:
pin F1 into the roadmap's open hardening lane (acceptance sentence is
only 1-of-3 reproducers met on the recorded paths) and land the
one-line F2 fix (the flag the gate error message advertises is broken
in its most natural placement). F3–F7 are next-cycle material with
clear fixes.

## Resolved questions

1. Does CU-6 actually fix the recorded P7-1 reproducers? — Split
   verdict, established empirically: multi-line yes, single-object
   files with a `usage` key no (F1, mechanism pinned).
2. Is the "string-typed usage was already coerced" honesty correction
   in the record right? — Yes for tokens (detector-path coercion),
   but those single-line files lose the message itself, which is worse
   and should headline the residual.
3. Does the baseline gate really exit 2 with the report intact, and
   does the opt-out work? — Yes, verified (F2 is the opt-out's CLI
   placement bug, not a gate bug).
4. Does the sweep touch unrelated files? — Yes, anything
   `.tmp.`-named and >1 h old (F3); history-dir orphans never swept
   (F4).
5. Any perf cost to the lenient fallback? — None measurable (40k
   lines / 2.1 MB incl. 40 repairs in 0.49 s).
6. Do the records misstate anything material? — Only the F7
   fixture-evidence deviation (undisclosed); everything else checked
   out, including the disclosed residuals.

## Artifacts

- This document: `docs/reviews/2026-09-02-cycle4-independent-review.md`
- Run dir: `/tmp/compound-engineering-1000/ce-code-review/20260902-151502-f3c6a556/`
- Probe sandbox: `/tmp/ir4` (isolated HOME/data/cache; corpus of 17
  adversarial JSONL shapes); pass-7 reproducers: `/tmp/pri4`
