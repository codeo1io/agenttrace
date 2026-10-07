# Cursor Import

Cursor stores local chat and composer state in SQLite `state.vscdb` files under the VS Code-style workspace storage directory. `agenttrace` keeps the main binary local-first and reads exported JSON instead of opening Cursor workspace databases directly.

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

## Cost estimates for cursor sessions (rm-566)

Cursor journals carry **no token accounting** — no usage events, no
provider-reported input/output tokens. Every dollar agenttrace
attributes to a cursor session is therefore priced from cursor-local
heuristics (model pricing applied to locally-derived turn/token
proxies), not measured provider usage.

All overview surfaces mark such rows: `By agent` rows show `(est.)` in
text/markdown, the JSON `by_agent` entries and `recent_sessions` rows
carry `"estimated": true`, and the HTML table annotates the cost cell.
The marker means "these dollars are an estimate with no token basis",
not "the estimate is rough by a few percent".

The gap is not small. A codeburn investigation of real usage (#1637,
Oct 2026) measured one month of Cursor API-usage-based billing at
**$703.77** where the local cursor data implied **$15.67** — a 45x
divergence, consistent with Cursor's local records under-counting what
the provider actually billed (fast-path/agent-mode requests largely do
not appear in local token fields). Treat cursor cost rows as a lower
bound; do not use them for chargeback, budget enforcement, or
upstream/downstream comparisons against token-accounted sources
(claude_code, codex_cli, gemini_cli, …).

Importing Cursor's web-exported usage CSV (which carries billed
dollars per request) to reconcile the two is tracked as a separate,
optional follow-up; the `(est.)` marker does not depend on it.

Review fix 25d9da7b extends the marker to every overview dimension:
model, provider, task-type, and project rows in text / markdown / html,
the `estimated` column in the CSV `by_model` / `by_provider` /
`by_task_type` tables (present only when a cursor-priced row is in
that table), the TUI top-model line, and `"estimated": true` on those
groups in `-f json`. Clean corpora stay byte-identical.
