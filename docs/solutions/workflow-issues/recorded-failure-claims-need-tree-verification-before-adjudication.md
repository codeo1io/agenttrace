# Recorded failure claims need tree verification before adjudication

- Minted: 2026-10-11, run 5a04ae3b0b9f (repository-maintenance 62052d7689d04bafab23fbef59139aea cycle 3, compound 396cfce2), from targeted_tests 3cef9400 + implement e0a29b1b evidence.
- Applies to: any phase that consumes a RECORDED test outcome (targeted/full validation envelopes, dispatch digests, prior-attempt forensics) instead of running the suite itself — compound phases above all.

## Rule

A recorded "FAILED" line is a claim, not a fact. Before adjudicating it against the tree (attributing a regression, gating a flip, or reopening a row), verify it against the tree the claim is about:

1. **Grep the named test.** `grep -rn '<test_name>' crates/ src/` — if the test does not exist anywhere in the tree, the failure entry is a transport/phantom artifact of the recording pipeline, not a signal about this code.
2. **Check the surface it implicates.** If the named failure lives in a file this run never touched (`git status --porcelain`, diff against base), an alleged regression there cannot come from the batch.
3. **Transient single-failure shape:** one failing test in an otherwise-green battery (especially with cache-dependent harnesses) — re-run that test in isolation, and scan for probe/cache pollution, before treating it as signal.
4. **Record the adjudication in the phase envelope** with the exact grep/re-run evidence, so downstream phases don't re-litigate it.

## Instances (this cycle)

- **Phantom FAILED in the targeted digest:** the dispatch validation digest carried a FAILED entry naming `sanitize_line_segment_rejects_control_chars` — `grep -rn` over `crates/` returned ZERO hits (the test does not exist in this tree) and `statusline.rs` was untouched by the run. Adjudicated as a phantom transport artifact; had it been trusted it would have mis-attributed a statusline regression to an untouched surface.
- **Transient cache race mid-implement:** one `cargo test -p agenttrace-core` failure in an otherwise-green full-crate run; the isolated re-run of the named test passed and a cache probe-pollution scan found none. Treated as a cargo test-cache race, recorded, NOT as batch signal — the full-suite leg later passed 874/0.

## Failure mode if skipped

Compound/commit phases consume recorded outcomes by contract (no re-runs). An unverified phantom failure propagates into roadmap rows and review findings as a regression that never existed — the cheapest possible check (one grep) kills it.
