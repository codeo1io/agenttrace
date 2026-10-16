# Gate pins must derive from the source of truth, never hardcode a version

**Class:** CI-hygiene prevention · **First fleet instance recorded:** run 1e4f6b95 cycle 4 (2026-10-16) · **Prevention owner:** every delegate touching scripts/ci/* or adding release-asserting gates

## Failure shape

`scripts/ci/check-install-runtime.sh` asserted the installer's pinned default
ref as the literal `v0.9.0` in three places (a comment, the test-D receipt
pin `^ref=v0.9.0$`, and test-E's `--branch v0.9.0` / `pinned ref v0.9.0`
echoes). Two release bumps later (install.sh pins `v0.10.1`), the gate went
red and stayed red — a false alarm on every run — while its sibling
`check-install-ref-drift.sh` stayed green the whole time.

## Root cause mechanics

- Both gates assert the same fact ("what ref does install.sh install by
  default?"), but only `check-install-ref-drift.sh` **derives** it — it
  extracts `AGENTTRACE_SOURCE_REF` from install.sh and compares against the
  newest CHANGELOG v-heading, so a release bump moves the expectation with
  the code.
- `check-install-runtime.sh` instead **duplicated the value** as string
  literals. Any duplicated constant rots at exactly the cadence releases
  ship; the gate then fails on healthy trees, which trains people to ignore
  gate output.
- The asymmetry was invisible to drift checks because the rot is *in* the
  gate, not in the thing the gate measures.

## Prevention rules

1. **A gate that asserts a value another file owns must read that value at
   run time from the owning file** — with an extraction ANCHORED to the
   owning line, not a first-match-anywhere grep. The landed shape is
   `sed -n 's/^REF="${AGENTTRACE_SOURCE_REF:-\([^"]*\)}"$/\1/p' install.sh`
   (anchored to the whole pin line, so a mere mention of
   `AGENTTRACE_SOURCE_REF:-` elsewhere in install.sh can never win; the
   `grep -m1 -oE 'AGENTTRACE_SOURCE_REF:-[^}"]+'` shape was this fix's
   first draft — and is still what check-install-ref-drift.sh uses today —
   but it was reconciled at review to the anchored form already landed on
   master by run a64735b75a46), so bumps
   cannot desynchronize. Deriving ≠ duplicating.
2. When two gates cover adjacent halves of one invariant, make them share
   the derivation pattern — or one of them will rot while the other lulls
   you (ref-drift green / runtime red was exactly this).
3. A standing red in a *gate* is a defect in the gate until proven
   otherwise; pin its pristineness (byte-compare to base) before calling
   it pre-existing, then fix the pin, not the release.

## Fleet instance (evidence)

- ROADMAP `rm-012` fix rider (run 1e4f6b95 cycle 4): the script now derives
  `install_ref` once at the top, prints `install-runtime: derived pinned
  default ref: v0.10.1`, and both pins assert the derived variable;
  targeted RC=0, full battery green. The only standing gate red at that
  base is dead.
- Original rot recorded in assess 07edad83 F3 (`:152 greps ^ref=v0.9.0$`
  vs install.sh `REF=v0.10.1`), with the fix shape already visible in
  `check-install-ref-drift.sh:35-38`.
