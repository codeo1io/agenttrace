# Statusline capture

Two data sources agenttrace users repeatedly ask for never appear in session
transcripts:

- subscription limit pressure (the `5h` / `7d` usage windows and their reset
  times), and
- Claude Code's upstream prompt-cache analytics (hit ratio, misses, and miss
  causes such as `tools_changed`).

Both are only observable from the **statusline payload** Claude Code feeds the
`statusLine` command on every prompt. `agenttrace statusline` captures that
payload to a local journal so `--statusline-report`, the TUI Efficiency panel,
and `--doctor` can surface them.

## Setup

Register the command once in Claude Code's `settings.json`:

```json
{
  "statusLine": {
    "type": "command",
    "command": "agenttrace statusline"
  }
}
```

The command must be on `PATH` for the host (the [install
script](../../README.md#install) puts it on `~/.local/bin` or `/usr/local/bin`).

## Host contract

Claude Code invokes the command with the payload as a single line of JSON on
stdin and expects **exactly one line on stdout**. `agenttrace statusline`:

- renders `name | ctx N% | 5h/7d N% until HH:MM | cache N% | $cost`,
- exits `0` for **any** input — valid payload, malformed JSON, or empty stdin,
- exits `0` for any **stdout condition** too — a closed pipe or full disk is
  reported to stderr at most, never a panic (the line is written with the
  write error ignored),
- sanitizes payload strings (session/model names) by replacing control
  characters (C0, DEL, C1) with `U+FFFD`, so a hostile session title can
  neither break the one-line contract nor inject escape sequences into the
  host terminal,
- writes diagnostics to stderr only (never stdout, never a non-zero exit), so
  the host UI cannot be broken by a capture bug,
- bounds stdin at 1 MiB so a pathological host cannot wedge the prompt.

On bad or empty input the rendered line is simply `agenttrace`.

## Journal and retention

Every invocation appends one JSON line — `{"captured_at": <unix seconds>,
"payload": <the host payload verbatim>}` — to:

- `$AGENTTRACE_SESSION_CACHE_DIR/statusline.jsonl` when that variable is set
  (the same directory as the session cache), otherwise
- `<user cache dir>/agenttrace/statusline.jsonl` (typically
  `~/.cache/agenttrace/statusline.jsonl`).

Retention is bounded at 10 MiB: past that, oldest **whole** lines are dropped
until the journal fits half the bound, through a temp-file rename so a crash
mid-compaction cannot truncate it. Torn or malformed lines are skipped on read.

## Schema (what agenttrace reads)

The journal is schema-faithful to the documented Claude Code statusline
payload. agenttrace reads these fields when present and ignores the rest:

| Field | Meaning |
| --- | --- |
| `session_id`, `session_name` | session identity for per-session grouping |
| `model.display_name` | rendered in the status line |
| `workspace.current_dir` | optional, for correlation |
| `context_window.used_percentage` | context pressure |
| `cost.total_cost_usd` | session cost to date |
| `rate_limits.five_hour.{used_percentage,resets_at}` | the 5-hour window |
| `rate_limits.seven_day.{used_percentage,resets_at}` | the 7-day window |
| `prompt_cache.{hit_ratio,misses,last_miss_cause,miss_causes}` | cache analytics |

Report semantics worth knowing:

- Captures are **deduplicated by exact payload**: identical consecutive
  payloads (e.g. a redrawn prompt) count once. Reports say so
  (`N captures (M after dedup)`).
- A **limit crossing** is evidenced only by observations on both sides of a
  `resets_at` boundary (usage observed before, and the first observation
  after). A window whose reset passed outside the journal, or a bare
  `resets_at` with no before-side, is not claimed as a crossing.
- `hit_ratio` is a `0..1` ratio; rendered percentages multiply by 100.

## Reviewing captures

```bash
agenttrace --statusline-report          # text
agenttrace --statusline-report -f json  # machine-readable insights
agenttrace --doctor                     # "Statusline capture:" health line
```

The TUI Efficiency panel shows the same insights under "Subscription limits"
when a journal exists.

## Honesty notes

- agenttrace **requires** the statusline command to be configured; nothing is
  captured otherwise, and `--doctor` says so.
- The fixtures in `crates/agenttrace-core/src/statusline.rs` tests follow the
  documented payload contract; they are not recordings of a real host.
