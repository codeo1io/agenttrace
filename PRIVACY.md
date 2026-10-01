# Privacy

agenttrace is local-first. It reads AI coding-agent session logs from paths you choose, computes metrics on your machine, and does not upload prompts, code, logs, reports, or telemetry to any hosted service.

Generated reports are written only to the output path you request with `-o`. Review reports before sharing them, because they can contain filenames, command names, model names, token counts, costs, and excerpts derived from local session logs.

## What agenttrace writes on disk

Parsing and reporting persist derived data on this machine. The cache artifacts live under `~/.cache/agenttrace` (`XDG_CACHE_HOME` relocates that root; the session cache, the SQLite snapshots, and the statusline journal additionally honor `AGENTTRACE_SESSION_CACHE_DIR`):

| Artifact | What it holds | Purge |
| --- | --- | --- |
| `sessions.json` | parsed metrics per session file: session names, file paths, working directories, paths of files touched, token/cost/health summaries | `agenttrace --clear-cache` |
| `hermes-sqlite.json`, `opencode-sqlite.json` | the same parsed metrics snapshotted from the Hermes and OpenCode SQLite session stores | `agenttrace --clear-cache` |
| `statusline.jsonl` | raw statusline payloads captured by the statusline hook: model names, token/cost summaries, rate-limit state (journal capped at 10 MiB; see the statusline section of the README) | `agenttrace --clear-cache` |
| `pricing.json` | public LiteLLM model pricing metadata — no personal data; re-downloadable with `agenttrace --update-pricing` | `agenttrace --clear-cache` |

`agenttrace --clear-cache` removes exactly the four artifact classes above, and additionally sweeps superseded-version leftovers (`hermes-sqlite-v*.json`, `opencode-sqlite-v*.json`) that older builds wrote next to the session cache — the same parsed metrics under versioned names no current code reads. Crash-orphaned atomic-write temps (`*.tmp.*` siblings of the artifacts above, left by an interrupted save or journal compaction) are likewise reclaimed automatically on the next agenttrace run. One further artifact is deliberately *not* cleared: `--preserve-history` writes derived session titles and metrics you asked to keep to `~/.local/share/agenttrace/history.json` (relocatable with `AGENTTRACE_HISTORY_DIR`). It is preserved user data, not a cache — delete that file to discard it.

agenttrace runs fully offline by default. Reports use a dated pricing snapshot bundled with the binary, and no report or test path contacts the network. The only exception is `agenttrace --update-pricing`, which downloads public model pricing metadata from the LiteLLM community pricing source and caches it locally for later runs. It does not send your local session logs. Deleting the cached pricing catalog (for example via `--clear-cache`) makes reports fall back to the bundled snapshot until the next `--update-pricing`.
