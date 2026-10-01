# Prevention rule: a formatting gate goes red fleet-wide the moment the toolchain floats

- **Class:** CI / toolchain reliability
- **Observed:** 2026-10-01, campaign `6a10ae64` cycle 1 (run `3c3e933d`), at master HEAD `ec8acdc`
- **Cost:** `cargo fmt --check` exit 1 with 8 diff sites at master, enforced by
  `.github/workflows/ci.yml` (PR lane and push-master + nightly), so **every PR and master
  push failed step 1** until fixed — gating delivery of all other work. Root-causing required
  proving the drift was rustfmt-version-relative (not author error) by bisecting it to the
  `2083a8d` landing.

## What happened

The tree was formatted by one stable rustfmt and checked by a newer one (1.9.0,
`48a229ceae 2026-09-01`). The CI lanes used `dtolnay/rust-toolchain` with
`toolchain: stable`, and no `rust-toolchain.toml` existed in-tree — so both local and CI
rustfmt silently floated across stable releases. The 8 flagged sites
(`crates/agenttrace-core/src/session_cache.rs` L264+L1792,
`crates/agenttrace-core/tests/pi_family_discovery.rs` L14/L53/L61/L69/L125/L133) were valid
under the formatting toolchain that produced them; nothing any author did wrong.

## Prevention rules

1. **Pin in the same commit as the gate.** Any repo that turns on a `cargo fmt --check` (or
   equivalent) gate commits a `rust-toolchain.toml` pinning the channel whose formatter
   rendered the tree — never "later". A gate plus a floating toolchain is a time bomb with a
   release-cadence fuse.
2. **CI toolchain inputs must match the pin file.** `ci.yml`/`release.yml` `toolchain:`
   inputs cite the same version as `rust-toolchain.toml` (here both `1.98.1`). A floating
   `stable` input next to a pin file disagrees silently and re-introduces the drift in CI.
3. **Diff the toolchain before diffing the code.** When a fmt gate goes red fleet-wide, first
   compare formatter versions; then prove the fix is whitespace-only with a token-stream
   comparison (`tr -d '[:space:]' <file | md5sum` old vs new) before treating the reformat as
   a content change.
4. **Minimal CI profiles do not install rustfmt/clippy.** A `dtolnay/rust-toolchain` install
   with the minimal profile ships no `bin/rustfmt` (verified by uninstall/install probe this
   cycle). With a `rust-toolchain.toml` present, `cargo fmt`/`cargo clippy` auto-provision
   the components per-directory — prefer the file over action inputs.

## Detection history

- Found 2026-10-01 by adversarial assessment, run `3c3e933d` (artifact
  `assess-27a31ef4…/2026-10-01-assessment.md`, finding N-F1 HIGH) and independently by the
  prioritize phase as the top-scoring wall item (rm-173, P90).
- Fixed same cycle by the implement phase (rm-173): staged 6-file batch — the 8-site
  reformat + `rust-toolchain.toml` (`channel = "1.98.1"`) + matching `toolchain:` inputs in
  `ci.yml`/`release.yml`; `cargo fmt --check` exit 0; full suite 272/272 under the
  validation gate (tree digest `validation:v1:2416e891…`).
