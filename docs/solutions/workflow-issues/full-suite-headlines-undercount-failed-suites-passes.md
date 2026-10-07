---
title: Full-suite failure headlines undercount executed tests
date: 2026-10-09
run: feb16bba31d64504869b4db782b0c8d3
repo: agenttrace
discovered-at: full_tests attempt 00ee2c3760bb4b73ab272663c37a4d21
---

## Problem

When a prior run reports a full-suite result as "N passed / M failed" and
the number came from grepping `test result: ok. N passed` lines, N is
UNDER the true count of executed passing tests: the failed suite's line
reads `test result: FAILED. 11 passed; 1 failed; ...` — its passing
tests exist and ran but never match an ok-grep. Any arithmetic built on
that headline ("baseline + my new tests = expected total") is wrong by
exactly the failed suite's passing count.

## Impact

In run feb16bba31d6 cycle 2, the assess phase recorded
`cargo test --workspace --no-fail-fast → 605 passed / 1 failed`.
The full_tests phase therefore predicted 605 + 2 new pins + 1 healed =
608. The actual green total was **619**. The 11-test gap was the failed
`hostile_journal_disclosure` suite's passing tests (11/12 suites-worth
inside it were green), dropped by the ok-only grep. A reviewer or gate
comparing 619 against a "608 expected" note would have flagged a
phantom discrepancy — or worse, accepted a wrong reconciliation.

## Detection

Machine-sum BOTH result shapes, then reconcile SUITE LISTS, not
headlines:

```sh
grep -oE 'test result: (ok|FAILED)\. [0-9]+ passed' full.log \
  | grep -oE '[0-9]+' | awk '{s+=$1} END{print s}'
```

and diff per-suite (name → passed count) between the baseline log and
the new log. The suite diff is the real proof: it shows exactly which
suites grew (here: `generic_model_usage_truth` 9→11, the two new pins)
and which healed (`hostile_journal_disclosure` 11-passed/1-FAILED →
12/0), and that nothing else moved.

## Prevention

- When recording a RED full-suite result, record the machine-sum of
  executed tests (ok + FAILED lines) and say so; never let an ok-only
  grep be the headline a later phase does arithmetic on.
- When a later phase reconciles against a recorded headline, first
  re-derive the executed count from the underlying log if it still
  exists; if it does not, treat the headline as a lower bound.
- Suite-count differences (e.g. 30 vs 29 suites) can hide 0-test
  doc-test suites — the suite-list diff catches those too.

## Incident history

- 2026-10-09, run feb16bba31d6 cycle 2: predicted 608 vs actual 619;
  reconciled per-suite from `delegate/ac7d1d53…-scratch/full-nff.log`
  vs `delegate/00ee2c37…-scratch/logs/test-trio.log`; executed
  baseline was 616, +2 pins +1 healed = 619 exactly.
