# cline-sessv fixtures (rm-1032)

Synthetic corpus for the two Cline storage generations, built from the
documented shapes in ccusage PR #1642 (closed-by-gate 2026-08-31; the
author's audit trail called the closure unreflective of the work —
agenttrace implements the format on its own evidence). No real
operator data; tokens and costs are invented.

- `sess-v/sess-v.messages.json` — the sess-v generation: a per-session
  transcript named after its directory, with per-assistant-message
  `metrics` (`inputTokens`/`outputTokens`/`cacheReadTokens`/
  `cacheWriteTokens`/`cost`) and `ts` epoch milliseconds. Includes:
  a user message with no `modelInfo` (skipped — not a usage-bearing
  turn), two same-timestamp assistant messages (dedup-safety probe:
  both must count), a second model (multi-model attribution), and an
  empty-metrics message (skipped).
- `legacy-task/` — the task-dir generation (`api_conversation_history`
  + `task_metadata`), no per-message usage; text-estimated tokens only.

sha256 pins (rm-542 convention — update these if the fixtures change):

    833682961e8b357c77d122a80ca98b2d65f7abad374c7e793485d0782e621ee7  sess-v/sess-v.messages.json
    fe03fae2005ab4e84cba6791265109291e6aeee0d64ef574dfd1468a435d17a7  legacy-task/api_conversation_history.json
    1c943e1c927798f886590c2b79260261fde6075092c0bfd4bf46aecba50256a3  legacy-task/task_metadata.json
