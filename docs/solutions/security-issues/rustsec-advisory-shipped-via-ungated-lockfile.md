---
title: RUSTSEC advisory shipped silently through an ungated lockfile
date: 2026-09-30
category: security-issues
module: agenttrace-core pricing transport (ureq/rustls dependency path)
problem_type: security_issue
component: infrastructure
symptoms:
  - "Cargo.lock pinned rustls 0.23.42, inside the RUSTSEC-2026-0285 affected range (introduced 0.23.30, fixed 0.23.45)"
  - "Release binaries shipped the vulnerable TLS stack through the repository's only network path, the ureq pricing fetch in agenttrace-core"
  - "No repository gate surfaced the advisory; it was found by external ecosystem research, not by any check the repo runs"
root_cause: missing_tooling
resolution_type: dependency_update
severity: high
tags: [rustsec, cargo-deny, rustls, supply-chain, cargo-lock, advisory-gate]
---

# RUSTSEC advisory shipped silently through an ungated lockfile

## Problem

The workspace lockfile pinned rustls 0.23.42, a version inside RUSTSEC-2026-0285's affected range ("TLS 1.3 handshake messages incorrectly accepted across encryption level boundaries"; introduced 0.23.30, fixed 0.23.45 per osv.dev). rustls is reached through exactly one dependency path (cargo tree -i rustls → rustls ← ureq 2.12.1 ← agenttrace-core), the transport that fetches model pricing. Every release binary therefore shipped the vulnerable TLS stack, and no gate the repository ran — build, lint, or test — had any way to notice.

## Symptoms

- Cargo.lock pinned rustls 0.23.42 while the fix (0.23.45) had already been published to crates.io.
- The vulnerable stack rode into release binaries through the pricing-fetch path with zero code involved.
- The advisory was discovered by external research (osv.dev + upstream commit history), not by any repository check.

## What Didn't Work

- The repository's existing dependency-review.yml workflow did not surface the advisory at any point in this campaign; the actionable gap was not its specific configuration but the absence of any gate that fails when the lockfile carries a known advisory.
- Direct crates.io index fetches for advisory/dependency research failed until the client sent a User-Agent header; osv.dev served the advisory detail directly and is the better single-source lookup for RUSTSEC records.

## Solution

Two-part fix, both implemented in this cycle's worktree (left uncommitted for the commit gate as of this writing):

1. Precise lock bump — `cargo update -p rustls --precise 0.23.45`. The lockfile diff is exactly two crates: rustls 0.23.42 → 0.23.45 and companion rustls-webpki 0.103.13 → 0.103.15. Minimal, fully reviewable, no transitive churn.
2. Standing advisory gate — a new deny.toml at the repo root (yanked = "deny", license allow-list with a webpki-roots exception, wildcards = "deny", unknown sources denied) plus a third CI job `deny` in .github/workflows/ci.yml running cargo-deny-action with --all-features on every trigger.

Validation recorded by this cycle: workspace cargo test green (core 52, tui 45, cli entrypoints 10, doc-tests clean) and a local cargo-deny 0.20.2 --all-features check on the patched lock reporting advisories/bans/licenses/sources all ok.

The pattern is upstream's: the same advisory was fixed upstream hours earlier (upstream PR #288, merged 2026-09-30T02:58Z), and the deny.toml/CI-lane shape was taken from that merged upstream configuration rather than reinvented.

## Why This Works

rustls has a single reverse-dependency path into this workspace, so a lockfile-only bump removes the entire exposure without touching product code. The precise flag (--precise) is what keeps the diff to two crates instead of a broad refresh that would drown review. The deny gate is the durable half: it converts "advisory present in lock" from a silent state into a red CI run, which is the failure mode that allowed this one to ship undetected in the first place.

## Prevention

- Run `cargo deny check --all-features` locally whenever Cargo.lock changes; treat an advisories finding as release-blocking, not informational.
- The CI deny job re-checks every push and PR — a newly published RUSTSEC now fails the build instead of riding into a release.
- Watch upstream supply-chain commits: this advisory was patched upstream hours before the fork noticed. Diffing the fork's Cargo.lock against upstream master after their hardening merges is a cheap early-warning signal.
- OpenSSF Scorecard was deferred while the fork's GitHub remote was believed private (Scorecard only scores public repositories) — that belief was wrong: codeo1io/agenttrace is public, verified live 2026-09-30 via `gh repo view --json visibility,isPrivate` (corrected under rm-155, cycle 1, alongside this doc and the deny.toml note). The scorecard lane now lives in `.github/workflows/scorecard.yml`.

## Related Issues

- ROADMAP.md rm-009 (the rustls lock bump) and rm-010 (the supply-chain gate) — the two change units this learning documents.
- Upstream PR #288 — the merged upstream fix and gate pattern this followed.
- docs/decisions/2026-09-02-adversarial-repository-assessment.md — the repository's assessment ledger this cycle contributed to.
