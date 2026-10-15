# Shared CARGO_TARGET_DIR serves stale sibling test binaries (mtime shadowing)

**Discovered:** 2026-10-15, run `f084f50ee5f1` full_tests (attempt `ba29f2fc` triage; confirmed + remediated in redo `1d172e41`).

## Symptom

`cargo test --locked -p agenttrace-core -p agenttrace-tui -p agenttrace` failed 1 test with:

```
panicked at 'keyword host lanes must refuse, not silently drop, foreign flags at rc2'
```

citing `crates/agenttrace-cli/tests/entrypoints.rs:2962` — **a test that does not exist in this tree** (file is 2947 lines; `grep -rn keyword_host_lanes` → 0 matches; not on `origin/master` either).

## Cause

The conductor delegate environment exports `CARGO_TARGET_DIR=/home/agent/.cache/conductor-cargo-target` — an engine-shared 28G cache written by every fleet run. Cargo reuses a compiled test binary when its source is *mtime-older-or-equal* than the cached binary. A sibling run had compiled `entrypoints-e0b665cf9f0a5b7f` (mtime Oct 14 20:10, i.e. **newer than our older-mtime identical-path sources**) containing a foreign test set; cargo ran that binary against our tree. `strings` on the binary showed 10 references to the foreign test; our tree's own `target/` did not contain it.

Two compounding effects:

1. Phantom failures in tests absent from the tree (the red above).
2. Silently **missing suites**: only 4 of the expected 60+ test binaries ran, because most were satisfied from the cache against foreign fingerprints.

## Rule

- **Full/validation batteries always run with a private `CARGO_TARGET_DIR`** — the worktree's own `target/` or a `/var/tmp` path (the fleet's established private-target convention). Never inherit the engine-shared cache for anything you will report as evidence.
- When a failure cites a test that `grep` cannot find in the tree, **suspect the binary, not the code**: check `$CARGO_TARGET_DIR`, `strings <binary> | grep <test-name>` for foreign content, and rerun with a private target before drawing regression conclusions.
- The same shadowing applies to `cargo build --release` outputs referenced by gate scripts via `AGENTTRACE_BIN` — verify freshness (`strings` probe for a symbol you just added) when reusing a shared target.

## Related

- Fleet TMPDIR-inside-a-repo trap and /tmp disk-pressure guidance: see `tests/upstream.rs` hermeticity work (rm-241) and the cycle-3 compound record `2026-10-15-cycle3-compound-record-run-f084f50ee5f1.md`.
- Host oversubscription (fleet load 42–53 on 14 cores) stalls gate scripts past tool timeouts → reaping; serialize batteries with generous timeouts under contention.
