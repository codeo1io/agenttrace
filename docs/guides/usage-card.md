# Usage card (shareable SVG)

The usage card is a single static SVG file summarizing your usage — the same
aggregation pass as every other `--overview` renderer, rendered as a graphic
you can drop into a README, a PR comment, or a profile page. It is
deterministic: a fixed corpus + range + theme always renders byte-identical
bytes, so the output is diffable, cacheable, and trustworthy to commit.

```shell
# card over all discovered sessions
agenttrace --overview -f svg > usage.svg

# a windowed card (today / last 7 days / last 30 days / yesterday)
agenttrace --overview --range today -f svg -o usage-today.svg

# color scheme: auto (default) | dark | light
agenttrace --overview -f svg --card-theme dark -o usage.svg
```

Like the other rich formats, the card requires `--overview` (or a governance
report action) — `-f svg` on its own is an error, never a silently empty file.

## What the card shows

- headline stats: total cost (estimated), session count, tool calls, total
  tokens (input+output) over the selected range;
- top models and top projects by cost (top five each, horizontal bars);
- daily spend sparkline over the trailing 14 days of the corpus's own clock;
- an honesty footer: the covered window, the pricing source in effect, and
  the data-health confidence for the corpus — the same disclosure the text,
  JSON, HTML, and markdown overviews carry, so a shareable card cannot hide
  an incomplete corpus behind a pretty picture.

## Truth model

- The sparkline is anchored to the latest event timestamp in the corpus, not
  the wall clock, which is exactly why reruns are byte-identical: the card
  describes the data, not the moment you rendered it.
- Costs are the same estimates the other renderers report (see
  [pricing](../pricing.md) for the snapshot model). The footer states the
  pricing source so a stale snapshot is visible at a glance.
- Model, project, and scope strings are control-byte sanitized and entity
  escaped before they reach the document — a session named
  `<script>alert(1)</script>` renders as visible text, not markup. The card
  embeds no scripts, no external references, and no network calls.

## Theme semantics

`auto` ships the light palette as presentation attributes plus a
`prefers-color-scheme: dark` style block, so the same static file adapts to
the viewer wherever SVG media queries are honored; where they are not (or a
sanitizer strips `<style>`), the card remains a correct light card. `dark`
and `light` are fixed palettes with no style block at all.

## Determinism contract

`tests/svg_card_contract.rs` pins the demo corpus's bytes for all three
themes (regenerate deliberately with `UPDATE_CARD_GOLDEN=1 cargo test -p
agenttrace-core --test svg_card_contract`), asserts rerenders are
byte-identical, and proves hostile names render inert.
`scripts/ci/check-output-contract.sh` additionally checks the CLI path:
saved-vs-stdout behavior, the saved-file status line on stderr, and that
two runs of the same corpus+theme compare equal with `cmp`.
