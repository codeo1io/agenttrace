# Upstream status

This fork tracks `luoyuctl/agenttrace` (remote name `upstream`) and
re-derives its drift from upstream main, the GitHub release channel, and
the npm package on every maintenance cycle. `agenttrace upstream` makes
that drift visible with one command instead of a hand-run script
(roadmap rm-024; it mirrors the standing shell report
`scripts/upstream-delta` from rm-012).

```
$ agenttrace upstream
agenttrace upstream status — offline view (remote-tracking refs 2h old)
  upstream:         upstream/master (git@github.com:luoyuctl/agenttrace.git)
  local:            conductor/run-3099db98be2b @ 90a4ef5a9ed9
  last sync:        748d1d2312 (2026-09-02) — Merge pull request #280 from luoyuctl/release
  ahead:            3 commit(s)
  behind:           4 commit(s)
  diverged files:   12
  new releases:     2 (v0.9.1 v0.9.2)
  unported commits: 4
    e54831e2 parser: handle adversarial token counts (#287)
    ...
  unported areas:   parser 2, ci/release 3, docs/testdata 1
  npm:              unknown (offline; agenttrace --fetch upstream) | running agenttrace v0.0.0-dev
```

## Offline by default

The command never touches the network on its own. Every number is
computed from **local remote-tracking refs** (`git merge-base`,
`git rev-list`, `git tag --merged`) and stamped with the age of those
refs, so a stale view says so. The npm distribution line reads
`unknown (offline; ...)` until you opt into a fetch.

`--fetch` opts into the network explicitly:

```
$ agenttrace --fetch upstream
```

It runs `git fetch upstream --quiet` first (refreshing the
remote-tracking refs — the upstream remote is fetch-only by design) and
additionally probes the npm registry for `@zack78/agenttrace`, reporting
the published version against the running build. If the registry probe
fails (no `curl`, offline, bad response) the report degrades to
`unavailable (...)` instead of failing.

Every `git` subprocess this command spawns runs under a stated
deadline, mirroring the governance `git log` probe's bound (rm-543):
local probes (rev-parse, merge-base, rev-list, diff, tag, log) are
capped at ten seconds — the same local class as governance's
`GIT_PROBE_TIMEOUT` — and the opt-in `git fetch` takes thirty seconds,
the network class the pricing-catalog download uses. A subprocess that
overruns its deadline is killed and the command fails with a terminal
error naming the git operation and the deadline; it never renders a
report from a partial repository, and a slow `git` can never masquerade
as "not inside a git repository" — only a probe that actually ran and
failed counts as a missing prerequisite.

Like the rest of the CLI, flags follow Go-style placement: they must
precede the positional command, so `agenttrace -f json upstream` and
`agenttrace --fetch upstream` parse, while anything after `upstream` is
treated as trailing arguments.

## Output schema

`-f json` renders a stable, append-only schema for scripting:

```json
{
  "command": "agenttrace upstream",
  "mode": "offline" | "fetched",
  "remote": "upstream/master",
  "remote_url": "...",
  "local": { "head": "<full sha>", "branch": "<name> | null" },
  "last_sync": { "sha": "<merge-base>", "date": "YYYY-MM-DD", "subject": "..." },
  "ahead": 3,
  "behind": 4,
  "diverged_files": 12,
  "new_upstream_releases": ["v0.9.1"],
  "unported_commits": [ { "sha": "...", "subject": "...", "files": ["..."] } ],
  "unported_areas": { "parser": 2 },
  "refs_age_seconds": 7200,
  "npm": { "package": "@zack78/agenttrace", "state": "...", "running_version": "0.0.0-dev" }
}
```

Notes:

- `ahead`/`behind` count commits between `HEAD` and the remote-tracking
  ref via the merge-base, exactly like `scripts/upstream-delta`.
- `new_upstream_releases` lists upstream tags reachable from
  `upstream/master` but not from `HEAD` — the release channel drift.
- `unported_commits` is oldest-first and uncapped; the text view lists
  at most 20 and prints `… and N more`.
- `unported_areas` groups the files touched by unported commits into
  named areas (`parser`, `diagnostics`, `pricing`, `reports`, `core`,
  `tui`, `cli`, `ci/release`, `npm/install`, `docs/testdata`, `other`)
  — the PR-level delta by area.
- `refs_age_seconds` is `null` when the ref age cannot be determined
  (e.g. fully packed refs).

## Exit codes

- `0` — the report rendered (drift itself is informational, not a
  failure).
- `1` — operational failure: not inside a git repository, no `upstream`
  remote configured (`git remote add upstream <url>`), no remote-tracking
  ref yet (run `agenttrace --fetch upstream`), or no common history.

## Environment

- `UPSTREAM_REMOTE` — remote to report against (default `upstream`).
- `UPSTREAM_REF` — upstream branch to compare with (default `master`).

Both mirror `scripts/upstream-delta`'s overrides, so wrapper tooling can
point either at a different remote/ref pair.
