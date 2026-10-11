# Cursor Import

Cursor surfaces agent history in two places, and agenttrace reads both:

- **Live chat stores (read automatically).** The IDE agent's chats live in sqlite outside the transcript walk: the app-state database's `cursorDiskKV` table (`…/Cursor/User/globalStorage/state.vscdb`, key spellings `composerData:<id>` and `bubbleId:<composerId>:<bubbleId>`; `CURSOR_HOME` relocates the root) and the per-chat `~/.cursor/chats/**/store.db` files. agenttrace opens both READ-ONLY — never writes, never creates WAL/journal sidecars — merges them with the `~/.cursor/projects/**/agent-transcripts/*.jsonl` walk, dedups a conversation visible in both spellings so it counts once, and applies per-bubble `tokenCount` usage (a recorded `costUSD` wins over catalog estimates and prices exactly its own block; a chat with no per-bubble usage falls back ONCE to the composer's `promptTokenBreakdown.totalUsedTokens`, disclosed in the report's "Disclosed facts"). A corrupt or unrecognized store is disclosed through `agenttrace --doctor` ("Cursor (live stores)" row) — never silently treated as an absent corpus. When the live stores carry chat rows, the default walk skips the project transcripts as re-emitted copies; an explicit `-d <dir>` scan always reads them.
- **Workspace exports (manual, unchanged).** The older VS Code-style workspace storage (`workspaceStorage/<workspace-id>/state.vscdb`) keeps `aiService.*` / `composer.composerData` keys; agenttrace still does not open those per-workspace databases — export them to JSON as below.

## Export a Workspace

macOS default path:

```bash
db="$HOME/Library/Application Support/Cursor/User/workspaceStorage/<workspace-id>/state.vscdb"
sqlite3 "$db" "
select json_group_object(key, json(value))
from ItemTable
where key in (
  'aiService.prompts',
  'aiService.generations',
  'composer.composerData'
);
" > cursor-export.json
```

Then run:

```bash
agenttrace cursor-export.json
agenttrace --overview -d .
```

## Supported Cursor Shapes

- `aiService.prompts`: user prompt entries
- `aiService.generations`: assistant generation metadata with timestamps
- `composer.composerData`: composer/session metadata

The export intentionally avoids Cursor auth tokens and unrelated UI state. If Cursor changes the internal schema, export the relevant key values as JSON and keep the original key names above.
