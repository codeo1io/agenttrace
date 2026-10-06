# Hermetic test markers must live inside the planted HOME — not in sandbox/machinery path names

**Workflow issue · recorded 2026-10-06 · run bd46a7d3 cycle 2 (rm-596) · compound attempt 24ba0106**

## Symptom

A binary-level hermetic test asserting "the tool's output must not contain the
planted secrecy marker" failed with the marker present in the output — but the
product under test had NOT leaked any operator state. The marker appeared in
*machinery* path strings, not in any discovered session data.

## Root cause

The test built its sandbox like this (name simplified):

```
$TMPDIR/at-<suite>-<MARKER>-<pid>/          <- marker in the sandbox ROOT name
├── home/            <- planted HOME (the "operator" corpus lives here)
└── cache/           <- XDG_CACHE_HOME, redirected OUTSIDE the planted home
```

Two facts combine into the trap:

1. On this host the delegate `$TMPDIR` is itself under the operator tree
   (`~/.hermes/tmp/delegate/...`), and the tool legitimately derives cache /
   statusline-capture paths from `XDG_CACHE_HOME` — which pointed *inside the
   sandbox root, next to `home/`*, not inside `home/` itself.
2. The secrecy marker was embedded in the **sandbox root's directory name**, so
   every machinery path derived under that root (`…/at-<MARKER>-<pid>/cache/…`)
   carried the marker into otherwise-correct output.

The assertion then tripped on machinery paths — a false positive that looks
exactly like the leak the test exists to catch.

## Rule

- Plant secrecy markers **only inside the planted HOME** (or any directory the
  assertion treats as operator state: session corpora, config dirs, project
  dirs). The thing you want to prove is "operator *content* never reaches the
  output" — so the marker must ride operator content, not test scaffolding.
- Keep sandbox root names, cache dirs, temp dirs and every other
  machinery-path component **marker-free** (bland names like
  `at-rmNNN-cli-<pid>/`).
- When asserting absence of a home path, assert on the **canonical home path
  string** (`$HOME` value) and on content markers planted beneath it — never
  on the sandbox root name.
- Control arms (run without the flag under test and assert the marker DOES
  appear) distinguish "gate works" from "marker was never plantable"; keep
  them.

## Detection / prevention

- If a no-leak assertion fails, grep the offending output for the marker and
  classify every hit: content leak (product bug) vs machinery path (test bug).
- Quick self-check before writing the test: `env | grep -i xdg` and echo the
  sandbox layout; confirm the only marker-bearing paths are the ones the
  product is supposed to read as operator state.

## Precedent

rm-596 (doctor demo gate): first run of the cli `doctor_demo_contract` test
failed on exactly this pattern; the fix was renaming the sandbox root
(marker-free) while keeping the marker inside the planted
`~/.claude/projects/<marker>/` corpus. Product code was correct throughout.
