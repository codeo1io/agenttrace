# Cycle 1 batch selection — run 96287e100a56469babf0aae93fd0e9ed (repository-maintenance bf1270b018564442ac460476a0b0fd4f)

**date**: 2026-10-15/16 · **base**: 445450b (== origin/master, the e4725315 cycle-1 landing) · **worktree**: run-96287e100a56-96287e10

## Batch selected

"pricing & release-channel reliability hardening" — three separable units, one theme:

1. **rm-1514** (mint, reliability 84.0) — the weekly pricing-drift auto-PR staged only `pricing_snapshot.json`+`pricing.rs` with no golden regen; its GITHUB_TOKEN PR runs no CI, so every actionable drift PR carried a `svg_card_contract` red that surfaced on master at merge. Hard deadline: the 2026-10-19 06:00 UTC cron (drift was live, 4,116 vs 3,131 keys). → fix + drill; see `docs/solutions/ci-hygiene/automation-generated-artifacts-must-regenerate-their-pinned-goldens.md`.
2. **rm-164 rider** — tiered pricing matched exact-case `Some("flex")/Some("ultrafast")` over a verbatim capture; mixed-case journals silently repriced standard, and opencode v1.19.0 enabling the tier made real traffic guaranteed. → case-fold + red-first contract; see `docs/solutions/reliability/external-tier-strings-must-fold-case-before-exact-match.md`.
3. **rm-1515** (mint, reliability 38.0) — installer downloads unbounded (install.sh :60/:78 `curl -fsSL`, npm install.js unbounded `https.get`). → **BOTH halves real and fixed**: install.sh's release and checksum fetches bounded with `--connect-timeout 15 --max-time 120`, install.js per-request 30s socket-inactivity destroy + error routing. **Error recovered at compound time**: mid-implement this was briefly recorded as an install.sh *false positive* ("curls already carried the bounds") — `git show HEAD:install.sh` at compound time proved both curls UNBOUNDED at HEAD; the observation had been the checker's own uncommitted edit (working-tree contamination), not the shipped tree. The false-positive wording propagated into two phase envelopes before being caught; corrected here, in the CHANGELOG entry, and in the rm-1515 roadmap flip.

## Rationale

- Deadline + failure-class ranking: automation shipping a master CI red weekly (rm-1514) > billing correctness on freshly landed code (rm-164) > first-run hang (rm-1515).
- Collision discipline: open PR #22 (rm-051 pin, rustls 0.23.46, doctor formula guard) and PR #23 (quota -f) own disjoint file sets; the batch implemented nothing they carry — merge coordination only.
- Feasibility: disk recovered (228M → 20G free) between research and implement, restoring compile-verified paths; 5 warm cargo target dirs.

## What validation showed (pre-review)

- Targeted (7a3120e2): 4 contract suites 23/0/0 (incl. the new case-insensitivity contract), `--lib pricing` 52/0/0, fmt/sh/node/ruby-YAML/changelog gates rc0.
- Full (ce41e2e9): `cargo test --locked -p agenttrace-core -p agenttrace-tui -p agenttrace` (ci.yml:134 authoritative selection) → 64 suites, 1136/0/0, green first run, zero fixes.
- rm-1514 drill (a5c25056, adopted): live-catalog refresh moved bundle+goldens together, second regen re-pinned byte-stable, goldens restored byte-identical to HEAD (mechanism lands; the weekly automation owns refreshes).

## Lessons recorded

- **Prove claims about HEAD against HEAD (`git show HEAD:<path>`), never a dirty working tree**: assess F5's install.sh claim was CORRECT — both curls were unbounded at HEAD. A mid-implement re-check observed the checker's own uncommitted edit and produced a false "false positive" that propagated into two phase envelopes before `git show HEAD:install.sh` caught it at compound time. A working-tree read on a dirty tree proves nothing about what shipped; the re-read-at-mint rule stands (cite lines at mint time), but the decisive check is against the commit, not the checkout.
- **Golden-coupled automation** must regenerate goldens in the same job (see the ci-hygiene solution doc); a token-authored PR has no CI to catch the omission.
- **Reaper recovery**: the roadmap-phase scratch (band seam) was reaper-swept between phases; recovered from `quarantine/scratches/20261015T081607Z/69a48cfb…/` with a byte-exact round-trip (patch → pristine wall → postimage md5 eb55b31d…). Commit gate: the seam is TWO patches in order (band `roadmap-delta-69a48cfb.patch`, then `ROADMAP.compound.patch`).

## Context for the next maintenance cycle

- Standing reds at this base (owned elsewhere): install-runtime test D pin (PR #22), rustls advisory lanes (PR #22), SC2001 ci.yml:204 (fog claim rm-1504), quota -f naming (PR #23).
- Unflipped rows the next cycle may want: rm-991 (by_agent display/vocab), rm-598 (flag-table identity gate); triggers pending: rm-507 (first opencode v2 tag), rm-421 (codex compression flags + NEW Chronicle/request-body-zstd), rm-713 (gemini 0.65.0 stable), rm-195 (registry state #6), rm-720/831 (Antigravity radar).
- Next free numeral after this cycle's mints: rm-1516 (derive from a LIVE wall census at mint time, never memory).
