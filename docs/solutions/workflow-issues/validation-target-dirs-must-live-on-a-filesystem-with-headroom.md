# Validation target dirs must live on a filesystem with headroom (df-first)

**Rule class:** workflow issue · **First proven:** run 92ea794984ae40f4ad8c73836efe21a6 full_tests attempt 7988981de9fc480997ee4e89b97e0c1a, 2026-10-16 · **Repo:** agenttrace

## The failure mode

The fleet rule "always override the shared inherited `CARGO_TARGET_DIR` with a private per-attempt
directory" names **privacy** as the invariant but not **placement**. Placement is not stable: on
2026-10-16 the root filesystem (`/`, which backs `/var/tmp`) hit 98% full / 2.3 G free while
`/home/agent` (`.cache`) had headroom — the reverse of the state that motivated the earlier
`/var/tmp` placement. A full `cargo test --workspace` battery whose private target dir sat on the
full filesystem died mid-compile with:

```
rustc exit 101 — error: failed to parse process output
```

on `agenttrace-core`'s lib test target — a **misleading** signature that reads like a source/parse
defect but is a disk-pressure artifact (the same tree compiles rc=0 the moment the target dir is
relocated). A delegate that trusts the signature will diagnose phantom code breakage, or worse,
"fix" a healthy tree.

## The rule

1. **df before every heavy lane.** Before any `cargo build/test/clippy --workspace` battery, check
   `df` on the candidate target filesystem and require comfortable headroom (hundreds of MB free at
   minimum; a full workspace battery's target dir runs >2 GB). Placement follows headroom, not
   habit: whichever of `/var/tmp` (root FS) and `/home/agent/.cache` has room hosts the private
   target for that attempt.
2. **Treat rc=101 "failed to parse process output" as an environment signal first.** Re-derive with
   `df` + a relocated `CARGO_TARGET_DIR` before reading it as a code defect. Only if it reproduces
   on a healthy filesystem is it a source problem.
3. **Private + headroom, both, always.** The two traps compose: the inherited shared
   `conductor-cargo-target` gives phantom E0432/E0425 (stale fingerprints across ~85 sibling
   worktrees); a private dir on a full FS gives phantom rustc parse failures. The cure for both is
   one motion: per-attempt `CARGO_TARGET_DIR` (and `TMPDIR=/tmp`) placed on a filesystem verified to
   have headroom.
4. **Record the relocation.** The battery record must state where the target lived and, if a
   relocation happened mid-validation, both locations and the df numbers that forced it — the
   reviewer must be able to distinguish an environment cure from a tree cure.

## Evidence

- Run 92ea794984ae40f4ad8c73836efe21a6 full_tests 7988981d (2026-10-16): first attempt
  `CARGO_TARGET_DIR=/var/tmp/.../target` with `df /` at 98% (2.3 G free) → rustc exit 101
  `failed to parse process output` on agenttrace-core lib tests; relocated the private target to
  `/home/agent/.cache/at-full-7988/target` (+ `TMPDIR=/tmp`), identical tree → rc=0, 69 suites
  1176/0; after cleanup root returned to 48% (56 G free). Log preserved in the phase's
  `full-battery.log` (spool) and folded result.
