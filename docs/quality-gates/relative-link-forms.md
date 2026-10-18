# Relative-link forms the docs gate lints

This document is the standing fixture for `scripts/ci/check-docs-links.sh`
(rm-1894): it exercises every link FORM the gate must resolve, in the
tree the gate actually lints. If any form below stops being covered,
this file's own links are what the gate fails on.

The gate checks two syntaxes (written here in words — this fixture
must not contain the literal shapes themselves, or it would be its
own broken link):

- **Inline links** — a label in square brackets with the destination
  in parentheses immediately after it, as in the [quality-gates
  index](../quality-gates) directory target or
  [the project README](../../README.md).
- **Reference-style definitions** — a `[label]: destination` line at
  the start of a line, used by the anchors below.

It also honors three semantics:

- A target may be a **directory** (the walk lands on `index`-style
  pages or a directory listing), not only a file.
- A `#fragment` suffix addresses a section of an existing file and is
  stripped before the existence check.
- Remote schemes (`http://`, `https://`, `mailto:`) are owned by the
  online docs-links workflow, not this gate.

[contributing]: ../../CONTRIBUTING.md
[quality-gates dir]: ./
[changelog head]: ../../CHANGELOG.md#unreleased
