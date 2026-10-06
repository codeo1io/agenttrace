# rm -rf under a repository path needs a git ls-files guard first

## Rule

Before any `rm -rf` on a path inside a repository working tree — in test cleanup, in scratch
sweeps, in an interactive shell — run `git ls-files <path>` and treat **any** tracked entry as a
hard stop. A directory that *contains* untracked build residue can still be (or sit under) a
tracked fixture tree; "looks like scratch" is not evidence. Tests that need a cleanable directory
must create it themselves (a `tempfile::tempdir()` or a dedicated scratch root outside the repo),
never delete a path they did not exclusively create.

## Incident (agenttrace, 2026-10-06, run de96d4cc implement ed48face)

A new `discovery_contract` test created a `mkfifo` fixture under `testdata/generated/` and its
cleanup needed to remove it. The operator line used to sweep the residue was `rm -rf testdata` —
read as "untracked scratch", but `testdata/` is a **tracked** fixture tree (18 tracked files under
`testdata/generated/` alone). The tree was restored immediately via
`git checkout -- testdata/` and the final porcelain was verified to contain only the intended
files, but the class is worth a standing rule: the misread cost a restore cycle and could have
cost fixture content that is not fully reconstructible from git (locally-modified or ignored
data under a tracked parent).

The sibling failure mode from the same implement pass — a mangled edit anchor landing a guard in
`walk_session_files_cached`'s cline-task branch, where a green test could not distinguish the
right site from the wrong one until the collector-level mkfifo contract exposed it — is the same
lesson from the other side: a placement claim needs its own red→green proof, not just a green.

## Checklist

1. `git ls-files <path>` — non-empty output ⇒ stop; the path is repo content.
2. `git check-ignore -v <path>` — ignored ⇒ still prefer a targeted delete over a tree-wide `rm -rf`.
3. Test fixtures: create under a self-owned temp dir; assert cleanup removes only what the test created.
4. After any sweep in a repo: `git status --porcelain` and, if anything unexpected vanished,
   `git checkout -- <path>` immediately, before any other command can build on the loss.
