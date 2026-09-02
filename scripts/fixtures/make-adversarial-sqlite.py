#!/usr/bin/env python3
"""Generate the adversarial SQLite fixtures committed under
testdata/generated/adversarial/sqlite/.

These are the cycle-2 committed reproducers for the assessment pass-5
findings (P5-1/P5-2) against crates/agenttrace-core/src/sqlite_sessions.rs:

  overflow.db — two assistant messages whose `tokens.input` is i64::MAX
    each; the per-session accumulator `agg.input_tokens += input`
    overflows in debug builds (exit 101) and wraps in release.

  wrap.db — one assistant message whose `tokens.input` is u64::MAX; the
    local `number_as_i64` helper casts `n as i64` and reports -1.

The loading tests copy these databases into a temporary
$HOME/.local/share/opencode/opencode.db and load them through the normal
discovery path, so the fixture files themselves need no session rows
beyond what each reproducer requires.

Usage: python3 scripts/fixtures/make-adversarial-sqlite.py
"""

import json
import sqlite3
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[2]
OUT_DIR = REPO_ROOT / "testdata" / "generated" / "adversarial" / "sqlite"

NOW_MS = 1770000000000
MODEL = "claude-sonnet-4-5"


def make_db(path: Path, messages: list[dict]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    if path.exists():
        path.unlink()
    conn = sqlite3.connect(path)
    try:
        conn.execute(
            "create table session ("
            "id text primary key, title text, time_created integer, time_updated integer)"
        )
        conn.execute(
            "create table message (id text primary key, session_id text, data text)"
        )
        conn.execute(
            "create table part (session_id text, data text)"
        )
        for index, message in enumerate(messages):
            conn.execute(
                "insert into message values (?,?,?)",
                (f"m{index}", "s1", json.dumps(message)),
            )
        conn.execute(
            "insert into session values ('s1','adversarial',?,?)", (NOW_MS, NOW_MS)
        )
        conn.commit()
    finally:
        conn.close()


def main() -> int:
    overflow_messages = [
        # Two legal i64::MAX inputs: `agg.input_tokens += input` overflows.
        {
            "role": "assistant",
            "modelID": MODEL,
            "tokens": {
                "input": 9223372036854775807,
                "output": 1,
                "cache": {"read": 0, "write": 0},
            },
        },
        {
            "role": "assistant",
            "modelID": MODEL,
            "tokens": {
                "input": 9223372036854775807,
                "output": 1,
                "cache": {"read": 0, "write": 0},
            },
        },
    ]
    wrap_messages = [
        # u64::MAX input: `n as i64` wraps to -1 (P5-2 reproduced -1 output).
        {
            "role": "assistant",
            "modelID": MODEL,
            "tokens": {
                "input": 18446744073709551615,
                "output": 1,
                "cache": {"read": 0, "write": 0},
            },
        },
    ]
    make_db(OUT_DIR / "overflow.db", overflow_messages)
    make_db(OUT_DIR / "wrap.db", wrap_messages)
    print(f"wrote {OUT_DIR / 'overflow.db'}")
    print(f"wrote {OUT_DIR / 'wrap.db'}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
