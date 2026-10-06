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

`agenttrace --clear-cache` removes exactly the four artifact classes above, and additionally sweeps superseded-version leftovers (`hermes-sqlite-v*.json`, `opencode-sqlite-v*.json`) that older builds wrote next to the session cache — the same parsed metrics under versioned names no current code reads. One further artifact is deliberately *not* cleared: `--preserve-history` writes derived session titles and metrics you asked to keep to `~/.local/share/agenttrace/history.json` (relocatable with `AGENTTRACE_HISTORY_DIR`). It is preserved user data, not a cache — delete that file to discard it.

**File modes and transient siblings.** Every session-derived artifact agenttrace writes — the snapshots, the statusline journal, the retained history, the `--output` report artifacts — is created with owner-only permissions (`0600` on Unix; umask can only tighten it further). This applies at creation: a file that already exists keeps the permissions it already had, so an older artifact created `0644` by a previous build stays `0644` until it is rewritten or removed with `--clear-cache`. Report artifacts are the exception on rewrite: `--output` paths are always staged through a fresh owner-only temp and renamed into place, so writing an existing report path re-tightens its mode too. Atomic rewrites happen through transient `<name>.tmp.<pid>.<seq>` siblings in the same directory — they carry the same payload as the file being replaced and exist only for the microseconds a rename takes. If the process crashes mid-write such a sibling can linger; it is reclaimed by the next cache load (and by `--clear-cache`-adjacent sweeps), not left forever. These transients are outside the four `--clear-cache` classes named above because they are write mechanics, not a retention decision.

agenttrace runs fully offline by default. Reports use a dated pricing snapshot bundled with the binary, and no report or test path contacts the network. Every network touch is opt-in, carries its own flag, and is announced on stderr at the moment it happens:

- `agenttrace --update-pricing` downloads public model pricing metadata from the LiteLLM community pricing source (`https://raw.githubusercontent.com/BerriAI/litellm/main/model_prices_and_context_window.json`) and caches it locally for later runs. It does not send your local session logs. Deleting the cached pricing catalog (for example via `--clear-cache`) makes reports fall back to the bundled snapshot until the next `--update-pricing`.
- `agenttrace --fetch upstream` runs `git fetch` against the configured `upstream` git remote (the repository URL comes from `git remote get-url upstream` on your machine) and makes one HTTPS request to `https://registry.npmjs.org/@zack78%2fagenttrace/latest` to read the published package version. Both touches are read-only metadata fetches and never upload your data.

If you run neither of those flags, agenttrace makes no network requests.
