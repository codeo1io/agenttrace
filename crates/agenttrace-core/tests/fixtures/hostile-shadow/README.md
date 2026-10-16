# Hostile shadow fixture (rm-241)

This directory is a *checked-in regression fixture*: a directory literally
named `agenttrace/`, placed beside a decode-test fixture root, that makes
the hostile ancestor as realistic as a live shared TMPDIR would be —
plausible-looking session content instead of an empty `create_dir_all`
stub (an empty shadow cannot catch a decoder that only looks at
non-empty trees).

The `rm-382` shadow-trap regression and the `rm-241` TMPDIR-inside-git
regression both copy this tree beside their fixture root before running
the decoder, so the goldens prove BOTH hostilities at once:

1. a sibling `agenttrace` directory that a greedy decoder could mistake
   for the first path component, and
2. an enclosing git repository *above* the fixture's boundary (bounded
   by `GIT_CEILING_DIRECTORIES` — see `find_git_root`).

The two files inside `agenttrace/` are inert: the insights decoder never
parses them, they only need to exist with plausible names.
