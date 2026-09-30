# Adversarial repository assessment — pass 12

- **Date:** 2026-09-30
- **Assessed head:** `9d88b36` (== `origin/master`, clean tree; worktree `conductor/run-0a279c440c10`)
- **Campaign:** repository-maintenance `47e4432e`, cycle 1 (run `0a279c440c104741ae459246dc310a00`)
- **Method:** whole-repository adversarial review (`depth:full`), every finding verified against the live tree this pass — none repeated from prior review docs.
- **Independence disclosure:** the cross-model adversarial peer is structurally unavailable on the GLM host (router provider enum `{codex,claude,grok,composer}`; `unknown` family is refused and declaring a known family would be a false attestation). The adversarial lens therefore ran in-thread with reduced independence, compensated by direct file/line evidence and live reproductions for the headline findings.
- **Baseline at this head:** full suite 264 passed / 0 failed / 0 ignored; demo smoke OK.

## Findings

| # | Severity | Location | Finding |
|---|----------|----------|---------|
| F1 | high | `ROADMAP.md` (campaign block) | 12 doubly-defined ids `rm-012..rm-023`: merge `9d88b36` integrated the 88feec46 campaign without applying the renumber its own collision-record mandates. (Executed later this cycle — see the dated RENUMBER EXECUTED record in the same file.) |
| F2 | high | `crates/agenttrace-core/src/parser.rs` `add_usage`/`add_usage_value` | Plain `+=` accumulation over untrusted per-message token counts: debug builds panic (`attempt to add with overflow`), release builds wrap negative and the `>0` consumers silently drop the total (under-count to zero). Live repro with a crafted opencode journal (`input: 5 + i64::MAX`). |
| F3 | medium | `crates/agenttrace-core/src/parser.rs` codex head probe (~:2262) | Substring-based "ignorable line" head probe: journal-quoting lines whose marker sits beyond the 160-byte window (or payload keys non-adjacent) are silently dropped, uncounted. |
| F4 | medium | `crates/agenttrace-cli/src/upstream.rs` (~:294) | FETCH_HEAD fallback can report stale refs as fresh. |
| F5 | medium | `crates/agenttrace-core/src/insights.rs` (:178-179) | Archived projects collapse into a single "unknown" bucket. |
| F6 | medium | `crates/agenttrace-core/src/pricing.rs` (:348) | Unbounded `into_string()` on the pricing HTTP response. |
| F7 | medium | `install.sh` (~:118) | Source-build fallback clones an **unpinned** upstream tip after a failed runtime verify — outside the checksum-verified story and unlabelled in output. |
| F8 | low | `crates/agenttrace-core/src/insights.rs` (:231/:278/:281) | Unmemoized `is_dir` probes on the same hot paths as rm-038. |
| F9 | low | `upstream.rs` unknown-authority | Unknown registry authority ranks as lowest freshness rather than unknown. |
| F10 | low | `upstream.rs` curl-absent path | Missing `curl` reads as registry outage. |

## Verified-and-discarded suspicions

Two initial suspicions were re-verified against the tree and **discarded** (recorded so the next pass does not re-chase them):

1. "parse-failure panics may propagate through the discovery loader" — they do propagate (`std::thread::scope`), and parse failures are counted as skipped; a malicious journal corrupts only its own session row.
2. "sqlite snapshot invalidation may miss fingerprint inputs" — the fingerprint covers the relevant inputs; no gap found at this head.

## Disposition into the roadmap

F2→`rm-046`, F3→`rm-047` (cycle-1 batch, implemented this cycle); F4→`rm-048`, F5→`rm-049`, F6→`rm-050`, F7→`rm-051`, F9/F10→`rm-052`; F8→folded into the pre-existing `rm-038` session-identity memoization item (see its cross-campaign corroboration note on the is_dir probes); F1 executed as bookkeeping (see ROADMAP.md RENUMBER EXECUTED record). Research candidates 60–64 (docs/research pass 10) map to `rm-043`/`rm-044` (dependency refreshes) + `rm-045` (release-engineering wave; candidate 61 is moot in this lineage — rustls 0.23.45 already shipped here) and `rm-051`/`rm-053`/`rm-054`.
