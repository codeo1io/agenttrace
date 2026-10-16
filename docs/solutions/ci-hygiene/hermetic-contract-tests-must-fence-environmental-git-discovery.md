---
title: Hermetic contract tests must fence environmental git discovery
date: 2026-10-16
category: ci-hygiene
module: agenttrace-cli (tests/upstream.rs)
problem_type: environment_dependent_precondition
component: tests
symptoms:
  - "outside_a_git_repository_fails flipped from 'not inside a git repository' to 'no 'upstream' remote configured' (rc101) when TMPDIR lived inside a git worktree"
  - "the flip was host-class-dependent: green on CI runners and neutral /tmp, red on any fleet whose temp dir sits inside a repo"
  - "the same host class intermittently broke unrelated in-process fixture walkers (insights decode / plausible_ancestors)"
root_cause: unfenced_git_discovery
resolution_type: discovery_fence
severity: medium
tags: [git, GIT_CEILING_DIRECTORIES, temp_dir, hermeticity, flaky-tests, TMPDIR]
---

# Hermetic contract tests must fence environmental git discovery

## Problem

A test that asserts "this directory is NOT inside a git repository" plants
its fixture under `std::env::temp_dir()` — but git discovery walks UP.
On any host whose temp dir lives inside a git worktree (this fleet's
delegate TMPDIR is `/home/agent/.hermes/tmp/delegate`, inside the
`/home/agent/.hermes` repo), discovery escapes the fixture namespace, finds
a HOST ancestor repository, and the precondition is silently false: the
contract under test flips from "not inside a git repository" to the next
prerequisite's error. The test then passes or fails depending on where the
CI host keeps its temp files.

## What Didn't Work

- Hoping for neutral TMPDIRs: the failure class is invisible on CI runners
  and intermittent on hosted fleets — the worst kind of red.
- Borrowing the host's repo layout for the "repo-enclosed" case: it
  reproduces the flip only on hosts that happen to have the layout, and it
  makes the test depend on host state it does not control.

## Solution

Two moves, both required:

1. **Fence**: every not-inside-a-repo expectation runs with
   `GIT_CEILING_DIRECTORIES` set to a STRICT ANCESTOR of the fixture cwd.
   Git never chdirs up into a ceiling, so discovery is pinned inside the
   fixture namespace on every host. Semantics that cost us an hour:
   the ceiling must be a strict ancestor — a ceiling equal to cwd does NOT
   fence (the ceiling itself is still searched from, and ascending past it
   is allowed).
2. **Plant, don't borrow**: the red-first arm constructs the repo-enclosed
   host class deterministically — `git init` an ancestor, nest a plain
   dir inside it, and pin BOTH directions: unfenced, the ancestor is
   discovered (the exact live flip); fenced, the run reports "not inside a
   git repository". Every host now exercises both classes in one suite,
   which is strictly stronger than a CI matrix split over TMPDIR classes
   and needs no workflow change.

## Prevention rules

- Any test asserting repository *absence* must fence git discovery
  (`GIT_CEILING_DIRECTORIES` = strict ancestor of cwd). `temp_dir()` alone
  is never a hermetic namespace.
- Reproduce host-class dependencies by planting the class in fixtures, not
  by borrowing the host's layout — determinism on every host beats
  authenticity on one.
- When sweeping siblings, distinguish assertions that require repository
  ABSENCE (need the fence) from ones that assert fixture-repo RESOLUTION
  (their own nearest `.git` governs; host ancestors are irrelevant) —
  fencing the latter class is a different row of work.

## Vocabulary

- **strict-ancestor ceiling**: a `GIT_CEILING_DIRECTORIES` entry that is a
  proper ancestor of cwd; git stops ascending before entering it.
