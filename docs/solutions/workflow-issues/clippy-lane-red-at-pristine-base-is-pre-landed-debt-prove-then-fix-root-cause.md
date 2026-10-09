# Clippy lane red at a pristine base is pre-landed debt — prove it with a detached worktree, then fix root-cause (never `-A` allowances)

**Summary:** `cargo clippy -D warnings` under a pinned toolchain goes red on code the current batch never touched, and the error you see first is not the only error in the tree — cargo aborts at the first failing crate, masking every later crate's lint findings.

## Symptoms

- The `ci.yml` clippy lane fails during implement/full_tests validation while the batch's own diff does not touch the failing file.
- Fixing the visible error "reveals" a brand-new error in a different crate (the mask lifting), making the delta look like it is regressing repeatedly.
- A green `cargo test` run coexists with a red clippy gate — local test batteries never exercise it.

## Diagnosis

- Conductor-salvage landings reach the main line through commit gates that may never run GitHub Actions, so lint debt accumulates invisibly at bases; the next validation phase is the first thing to run the lane at the pinned toolchain.
- `cargo clippy -p a -p b -p c -- -D warnings` stops compiling at the first failing crate; every later crate's findings stay hidden until the earlier one is clean (observed live: a `parser.rs:240` type_complexity in `agenttrace-core` hid three `needless_borrow` sites in the CLI bin at `main.rs:694/:700/:703`).

## Fix

1. Run the CI-form clippy lane EARLY in any implement/full_tests turn — it is seconds on a warm build and converts a late surprise into an early one.
2. If red in code the batch did not touch, prove provenance BEFORE editing: `git worktree add --detach /tmp/<probe> <base>` and rerun the same lane there. Identical error at the detached base ⇒ pre-landed debt, not a batch regression (record the probe rc in the phase result).
3. Fix ROOT-CAUSE in the tree: type aliases for complex types, drop redundant borrows, restructure per the lint's own suggestion. Type-level fixes change zero behavior — they ride the batch delta safely.
4. NEVER silence with `-A clippy::…` allowances in a validation run: allowances hide the debt class and keep the first-failing-crate mask in place (a targeted-tests battery ran green with `-A type_complexity -A needless_borrow` while both debt classes stayed in the tree).
5. Close by rerunning `cargo clippy --workspace --all-targets -- -D warnings` with NO allowances — the strict superset — and record both lanes' rc.
6. Declare the lint-fix hunks explicitly in the phase result and the compound record so the commit gate knows their provenance (they are base repairs, not batch features).

## Prevention

- The repo-level hardening (gate the test-code lint class too) is standing roadmap row rm-654 — `ci.yml`'s clippy lane lacks `--all-targets`, so the MF3 test-code lint class escapes the gate; land that row rather than re-reporting the class.
- Keep the pinned toolchain authoritative locally (`rustup` default == `rust-toolchain.toml` pin); a drifting clippy version fires different rules than CI will.

## Checklist

- [ ] Clippy lane run early in the turn, before the full battery.
- [ ] Red-at-base proven by detached-worktree rerun before any edit.
- [ ] Root-cause fixes only; zero `-A` lint allowances left behind.
- [ ] `--workspace --all-targets -D warnings` green with no allowances at close.
- [ ] Lint-hunk provenance declared in the phase result + compound banner.

**Evidence:**
- run 7eae74ea full_tests c55a0715 (2026-10-06): pristine-base proof at 6b03087 (`parser.rs:240` type_complexity, rc=101 identical in the detached worktree), masked `main.rs` borrows, root-cause fix (`type ModelUsageSnapshot` alias; 3 borrow drops), both lanes green, 22/22 ci.yml lanes rc0.
- run 7eae74ea targeted_tests 0b126b00: the counterexample — allowances made a battery green while the debt stayed in the tree.
