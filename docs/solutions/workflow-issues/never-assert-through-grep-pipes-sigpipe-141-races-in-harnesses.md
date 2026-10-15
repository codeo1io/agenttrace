# Prevention rule: assert on captured outputs, never through grep pipes — SIGPIPE 141 races in harnesses

- **Class:** release/CI harness robustness (test infrastructure, not product code)
- **Observed:** 2026-10-15, repository-maintenance 205f84bb cycle 5 (run 85c7546d53ef), implement attempt 72f14637, hardening `scripts/release/idempotency-selftest.sh` (rm-169).
- **Cost:** the release-idempotency gate failed 6 of 30 runs on fully-correct output — flaky reds that erode trust in the gate, invite rerun-until-green habits, and mask real regressions during release-week contention.

## The rule

1. **Never let a pipeline stage's exit code be the verdict.** Capture first (`out=$(cmd)` or redirect to a file), then assert on the captured artifact. A `grep` reading a regular file cannot SIGPIPE; a `grep` reading a pipe can.
2. **Prefer here-string asserts** — `grep -q pattern <<<"$out"` — over inline `cmd | grep -q pattern` pipes, especially under `set -o pipefail`, where one SIGPIPE'd stage fails the whole chain.
3. **`grep -q` is the worst offender**: it exits at the first match, instantly closing the read end while the producer is still writing. `echo "$out" | grep -q …` under `set -o pipefail` SIGPIPEs the `echo` writer roughly 3.5% of the time under load (138/4000 in the isolated reproduction, `race-proof/which-race.sh`).
4. **If a pipeline is unavoidable**, inspect `PIPESTATUS[*]` explicitly and treat `141` (128+SIGPIPE) as infrastructure noise with a bounded retry — never as a test verdict.
5. **Prove no-op/idempotency by CONTENT, not mtimes**: snapshot the artifact (e.g. `md5sum`), run the operation again, byte-compare. Mtime-based no-op proofs break under any touch-happy tooling.

## Why this shape keeps recurring

Assertion-through-a-pipe is the shortest shell idiom for "does the output contain X", so it reads as obviously correct in review and passes every quiet-machine run. The failure is purely timing-dependent (writer exit vs. reader early-exit under scheduler load), so it appears only under CI contention or fleet oversubscription — exactly when a red gate is most likely to be believed. Recorded live: 6/30 gate runs red on correct output pre-fix; 0/90 post-fix with here-string asserts (same 10-branch harness, same machine, same load window).

## Related

- `scripts/release/idempotency-selftest.sh` (this cycle's hardening, all asserts here-string or file-based)
- `scripts/release/publish-release.sh` (existence-probed release publish — asserts on captured `gh` output)
- Cycle record: `docs/stewardship/2026-10-15-cycle5-compound-record-run85c7546d53ef.md`
- Fleet family: measuring exit codes after `cmd | grep` reports grep's rc — same class, recorded by this run's assess phase
