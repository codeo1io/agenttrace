# Codex pre-Sept-2026 rollout fixtures (rm-716)

Golden corpus synthesized from the tokscale #1405 envelope (open issue,
2026-10-06; repro on Mar–Aug 2026 rollouts): sessions the codex CLI of
that era wrote with the event set `session_meta` / `task_started` /
`turn_context` / `response_item` / `event_msg(item_completed)` and **no
`token_count` events at all**. agenttrace's codex lane buffers all of
its usage from `token_count` snapshots and `token_usage_record`
compaction rows, so those rollouts parsed green with zero usage and no
distinct verdict — the silent-zero this batch's rm-716 closes with the
`codex_rollout_no_usage_rows` disclosure.

- `rollout-no-usage.jsonl` — the legacy envelope verbatim (model carried
  on `turn_context`, as the era's rollouts did). Parses to a session
  with the absence verdict and NO fabricated usage (token provenance
  stays `estimated_from_text`).
- `rollout-with-usage.jsonl` — the control: identical envelope plus one
  `event_msg`/`token_count` snapshot. Reports the counted usage and NO
  absence verdict.

Synthetic (shape from the issue text + the fork's codex lane tests), not
recordings. Do not edit; the goldens in
`tests/disclosure_plane_honesty.rs` pin both directions.
