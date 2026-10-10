# Implement batches must run the fmt lane before delivering

**Rule:** an implement (or review-fix) phase that edits Rust must run `cargo fmt --check`
before declaring its batch delivered — the clippy + docs-gate pair is NOT sufficient.
If a fmt fix happens at any later phase, the batch's tree pins (per-file md5s, `git diff`
sha256) must be RE-EMITTED post-fix, and every downstream gate must consume the post-fmt
values.

**Incident (2026-10-10, agenttrace run d02291d0efbb, repository-maintenance cycle 1):**
the 'honest weekly envelope' batch (rm-917 calendar-anchored budget window + rm-918
embedded-`#` config semantics) was implemented red-first with a green targeted battery —
`cargo test -p agenttrace-core --lib` 279/0, `--bin agenttrace config::tests` 9/0,
`cargo clippy -p agenttrace-core -p agenttrace --all-targets --locked -- -D warnings` rc0,
`scripts/ci/check-docs-commands.sh` rc0 — and was only then caught by the FULL suite's
lane 01 (`cargo fmt --all --check`) with **5 rustfmt violations in the batch's new test
code** (statusline.rs :998 :1048 :1094 :1208; config.rs :639), turning the first full run
red.

**Why it slipped:** the targeted/clippy battery does not include a fmt lane, and the
violations were all in code that clippy accepts (rustfmt layout only: line-wrapping).

**Fix + proof:** canonical `cargo fmt`, layout-only proven by whitespace-normalized diff
of pre/post files being IDENTICAL (27 changed lines, all wrapping); `cargo fmt --check`
rc0; the ENTIRE 22-lane suite re-ran green from lane 01 after.

**The pin-movement corollary (the part that bites gates):** an fmt fix changes the batch
diff. This cycle the pins moved — `git diff` sha256
`d5a212ea…` → `acb9b8cb…`; statusline.rs md5 `8b4a8438…` → `cb4d02c6…`; config.rs md5
`0247a64d…` → `6315e032…` (README untouched). A commit gate that consumed the
implement-phase pins would have failed or, worse, validated the wrong tree. Full-suite
phases that fix fmt must publish the post-fix pin table and name the pre-fmt values
superseded.

**Prevention:**

1. Implement phases: run `cargo fmt --check` (or `cargo fmt` then re-diff) as the last
   lane of the delivery battery, after clippy.
2. Any phase that runs fmt: re-emit `changed-files.md5` + diff sha256 immediately, and
   state in the record which earlier pins are superseded.
3. Commit gates: consume the NEWEST pin table; verify `git diff | sha256sum` against it
   before staging.
