# opencode v2.1.x storage fixture bank (rm-507 / rm-1021)

Synthetic fixture store in the opencode **v2.1.2** storage layout
(released 2026-10-21). No real user data: every file was authored for
this bank; session ids, account paths, and corpora are invented.

## Provenance

- The v2.1 field shapes come from the upstream breaking-change record
  `anomalyco/opencode` PR #57260 ("refactor: v2.1 minor breaking
  changes", merged 2026-10-20), verified against the PR body on
  2026-10-23 while banking this fixture:
  - text/reasoning parts: provider replay `state` → `native` (a tool's
    execution `state` is a DIFFERENT field and keeps its name —
    `pr_a3_tool.json` therefore still carries `state.status`);
  - assistant / tool / compaction messages: `providerState` → `native`;
  - tool call + result replay data share one shallow-merged
    `tool.native`; tool status and readable output stay in `state`;
  - compaction: `recent` removed, replaced by `metadata.keepID`
    (the first message retained verbatim beside the summary);
  - provider compaction windows live in `native.messages`;
  - the unused transitional `session.compacted` event is removed.
- The agenttrace storage-session parser reads only
  `id` / `message` / `tokens` / `meta` surfaces
  (`parse_opencode_storage_session`), so the renames touch no field it
  reads; this fixture proves that per revision instead of assuming it.
- Layout mirrors the existing fixture banks
  (`opencode-{agent-summary,hostile,summary-diffs}/`,
  `opencode-v2-store.db`): `storage/session/**`,
  `storage/message/<sessionID>/**`, `storage/part/<messageID>/**`,
  `storage/project/**`.

## Corpora

Two sessions under one `opencode/storage` root:

- `ses_v21_alpha` (project `proj_orbit`, directory on the session doc):
  user turn, assistant turn with message-level tokens
  (`msg_a2`: in 1200 / out 340 / cache 800r+100w / reasoning 64) whose
  step-finish part carries a POISONED 9999/9999/5 block that must never
  fold over the authoritative message tokens; a tool round-trip
  (`msg_a3` → `read` completed); a reasoning-only message
  (`msg_a4`: reasoning 48) whose step-finish part (700/210) must fold
  through exactly once (rm-619 both arms); a v2.1 compaction
  (`msg_a5`: `metadata.keepID`, `native.messages` window).
  Totals locked by `opencode_v2_1_x::v21_alpha_locks_full_attribution_and_token_folds`:
  input 1900, output 550, cache_r 800, cache_w 100, reasoning 112,
  one reasoning block, one completed `read` tool call, all usage owned
  by `anthropic/claude-haiku-4-5`.
- `ses_v21_beta` (project `proj_north`, no `directory` on the session
  doc → resolved through `storage/project/proj_north.json`): one
  assistant message (`msg_b1`: in 500 / out 150, sonnet) with a text
  part. Totals locked by
  `opencode_v2_1_x::v21_beta_resolves_directory_through_project_doc`.
- Discovery end-to-end (fixture copied to a scratch
  `~/.local/share/opencode`) is locked by
  `opencode_v2_1_x::v21_store_auto_discovers_with_attribution`.
