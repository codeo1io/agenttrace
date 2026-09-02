# Cycle-6 prioritization (phase: prioritize)

Run `a24bcf084cf049208c75d2cb4f3a3755`, attempt `0bd3cd41967d4085baa48a0ec73bd2e7`,
2026-09-03. Inputs: roadmap state after the cycle-5/p9/r8 update (`ROADMAP.md`,
+265/−27 this run), assess pass 9
(`2026-09-03-cycle5-adversarial-assessment.md`), research pass 8
(`2026-09-03-research-extensions.md`), cycle-5 implementation record. Router
note: no `ce-*` compound-engineering router is installed in this environment
(consistent with the assess/research/roadmap phases this run); work proceeded
directly with file evidence.

Method: every open item (28 hardening-lane bullets plus the open capability
candidates) scored on impact (correctness/truthfulness/security/user-visible
capability), risk of staying open, effort in this codebase, dependencies, and
strategic value. Effort claims below were re-verified against current source
this pass (line cites checked live; stale cites corrected).

## Scoring matrix (top of the queue)

| Item | Impact | Risk open | Effort | Deps | Strategic | Score |
|---|---|---|---|---|---|---|
| CI revert + master tracking (pass-9 CRITICAL) | 5 | 5 (proven: PR #282 lost) | XS | none | 5 (unblocks any future upstream work) | **20** |
| Candidate 50a: `~/.gemini/tmp` discovery root | 4 (restores README claim) | 4 | S | none | 4 (radar #236; Gemini-family center of gravity) | **16** |
| Candidate 50b: Antigravity `conversations/` root | 3-4 | 3 | S-M | docs for shape | 4 | **14*** |
| Candidate 25: `thoughtsTokenCount` billed as output | 4 (cost truth) | 4 | S-M | none | 3 | **15** |
| Pass-9 `--sample` wording vs `--sort` | 2 | 2 | XS | none | 2 | 6 |
| Pass-9 cache byte ceiling | 3 | 3 | S-M | none | 1 | 8 |
| Pass-9 parse_file size cap | 3 | 3 | S | none | 1 | 7 |
| Pass-9 git-root sandbox + memoization | 3 | 3 | M (folds N10) | N10 | 2 | 8* (next cycle) |
| Pass-9 Hermes tool-failure signal | 2 | 2 | M (schema research) | state.db schema | 2 | 6* |
| Installer checksums (F8-9) | 3 | 3 | M (release coupling) | release artifacts | 2 | 8* |
| Candidate 51: pricing snapshot age | 2 | 2 | S | none | 2 | 6* |
| Candidate 3: budgets/pace | 4 | 3 | M-L | history infra (present) | 4 | 11* (after hardening) |
| Candidate 19: MCP server mode | 4 | 2 | M-L | none | 4 | 10* |
| Candidate 2: OTel export/ingest | 4 | 2 | L | none | 4 | 10* |
| P3-1 Windows/HOME parity | 4 | 3 | L | Windows env | 2 | 9* |
| Pass-9 TUI reload race | 1 | 1 | S-M | none | 1 | 3 (defer) |

(*) scored but not selected this cycle — rationale below.

## Selected batch — cycle 6: "coverage, cost, and capability tell the truth"

Theme: every user-facing claim (CI config, README's Gemini support, thinking-
model costs, sampling disclosure) becomes verifiably true. All items are
locally test-verifiable, independent of each other, and none touch topology
beyond one local, network-free git-config change.

- **CU-17 — Upstream-portable CI revert (CRITICAL stewardship; pass-9).**
  Revert `runs-on: self-hosted` → `ubuntu-latest` at `ci.yml:21`,
  `release.yml:13/40/117`, `dependency-review.yml:16` (AGENTS.md rule 3);
  repoint local master's tracking to the fork remote (`git config
  branch.master.remote fork` — local-only, no network, reversible) so a plain
  `git push` can never ship fork work to upstream; add a guard script grepping
  the three workflows on upstream-bound branches. Acceptance per roadmap item;
  evidence: grep shows zero `self-hosted`, `git config branch.master.remote`
  prints `fork`.
- **CU-18 — Gemini CLI `~/.gemini/tmp` discovery root (candidate 50a).**
  Format already verified live in research pass 8 (checkpoint parses
  positionally); helper predicates already expect the path
  (`discovery.rs:533`, `:584-590`). Two-line root entry + fixtures + tests +
  doctor output. Restores the README's claimed Gemini support.
- **CU-19 — Antigravity `…/antigravity-cli/conversations/` root (candidate
  50b).** Conditioned: land the root with a conversations fixture built from
  the officially documented shape (research §3: official migration doc + the
  hermes-agent skill reference). If the documented shape proves ambiguous
  mid-implementation, land the root + lenient detector with a fixture marked
  provisional and file the verified-fixture residual — never block the batch
  on external data. Note: this host has no `~/.gemini`, so no real corpus
  verification is possible this cycle (checked live).
- **CU-20 — Gemini-family thinking tokens (candidate 25).** Fold
  `usageMetadata.thoughtsTokenCount` into output billing with a reasoning
  breakdown. Verified this pass: the three usage sites are alias-driven —
  `qwen_usage` (parser.rs:1802), `gemini_usage` (parser.rs:3550), and the
  table-driven site (parser.rs:3885) — and none lists `thoughtsTokenCount`
  (grep: zero matches). Roadmap's old line cites (`:1779-1795` etc.) are
  stale; corrected here. Fixture + tests + `--audit` reasoning share +
  changelog note that baselines shift.
- **CU-21 — `--sample` disclosure names the active ordering (pass-9
  residual of closed F8-1).** Two format strings at `main.rs:250-252` and
  `:293-295` (verified live, identical text) say "sampled newest N of M"
  regardless of `--sort`; fix + CLI test.
- **CU-22 — Session-cache byte ceiling (pass-9, F8-3 follow-on), stretch
  item.** Byte cap enforced at save with oldest-source-mtime eviction (same
  policy as CU-13's count bound), surfaced in `--doctor`. Include only if
  CU-17..CU-21 close clean; otherwise it rolls to cycle 7 unchanged.

Batch shape matches cycle 5 (six items, 203-test baseline, no cross-item
dependencies). Order: CU-17 first (retires the standing leak hazard), then
CU-18/19 (one discovery.rs edit set), CU-20 (parser.rs), CU-21 (main.rs),
CU-22 (session_cache.rs) — each closes with its own tests; `cargo test
--workspace`, `cargo fmt --check`, `cargo clippy --all-targets` clean at the
end.

## Explicit deferrals (with reasons)

- **Candidate 51 (pricing age), parse_file size cap, installer checksums** —
  all worthwhile, all small-to-medium; not selected because the batch is full
  of higher-scoring truth items. Size cap and installer checksums pair
  naturally with the next hardening slice; candidate 51 pairs with candidate
  24 (pricing provenance) as one motion.
- **Git-root sandbox + memoization** — deliberately folded with the open N10
  cost-ceiling item (they share the bounded-pool/timeout acceptance); doing
  half now would leave the roadmap item half-closed again.
- **Hermes `state.db` tool failures** — blocked on schema research (which
  columns, if any, record failures); do the research as part of the slice,
  not before it.
- **Candidates 3/19/2, P3-1** — strategic capability lanes; the roadmap's
  standing rule is hardening precedes capability, and each is cycle-sized on
  its own.
- **TUI reload race (INFO)** — no user-data corruption possible; keep as a
  documented benign race until the loader-ownership refactor has a natural
  slot.
- **C8 cleaned upstream-only PR** — remains operator-gated per AGENTS.md
  rule 4; CU-17 is the precondition that makes any such PR reviewable.

## Verification of this phase

- Effort/site claims above re-checked live this pass: `grep -n runs-on
  .github/workflows/*.yml` (5 hits), absence of `~/.gemini` on this host,
  `rg thoughtsTokenCount` (zero matches; three usage sites located at
  parser.rs:1802/3550/3885), `--sample` strings at main.rs:250-252/293-295,
  28 hardening-lane bullets counted in ROADMAP.md:431-826.
- Batch acceptance criteria are lifted verbatim from the roadmap items filed
  in the roadmap phase this run; no new scope invented here.
