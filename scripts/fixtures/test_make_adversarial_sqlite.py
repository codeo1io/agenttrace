"""Pytest coverage for the adversarial SQLite fixture generator (rm-002).

The generator (``make-adversarial-sqlite.py``) produces the two committed
reproducers under ``testdata/generated/adversarial/sqlite/`` that the
core loading tests use:

- ``overflow.db`` — two assistant messages whose ``tokens.input`` is
  ``i64::MAX`` each, so the per-session accumulator
  ``agg.input_tokens += input`` overflows in debug builds and wraps in
  release (pass-5 P5-1);
- ``wrap.db`` — one assistant message whose ``tokens.input`` is
  ``u64::MAX``, so the ``number_as_i64`` helper's ``n as i64`` cast
  wraps to ``-1`` (pass-5 P5-2).

The adversarial values ride inside the JSON ``data`` column of the
``message`` table, never as bound SQL integers — sqlite cannot hold
them — so these tests assert on the parsed JSON payload.

Everything is hermetic: the committed databases are opened read-only,
and regeneration happens on a layout-preserving copy under ``tmp_path``
(the generator resolves its output from ``Path(__file__).parents[2]``),
never inside the repository. CI wiring for this test is intentionally
out of scope for cycle 1 (rm-002).

Run: python3 -m pytest scripts/fixtures -q
"""

import json
import shutil
import sqlite3
import subprocess
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
GENERATOR = HERE / "make-adversarial-sqlite.py"
# HERE is scripts/fixtures; the repo root is two levels up. (The
# generator itself resolves parents[2] from its own file path.)
COMMITTED_DIR = HERE.parents[1] / "testdata" / "generated" / "adversarial" / "sqlite"

I64_MAX = 9223372036854775807
U64_MAX = 18446744073709551615
MODEL = "claude-sonnet-4-5"
NOW_MS = 1770000000000


def read_fixture(db: Path):
    """Returns (message_rows, session_rows) parsed from a fixture db."""
    conn = sqlite3.connect(f"file:{db}?mode=ro", uri=True)
    try:
        messages = conn.execute(
            "select id, session_id, data from message order by id"
        ).fetchall()
        sessions = conn.execute(
            "select id, title, time_created, time_updated from session"
        ).fetchall()
    finally:
        conn.close()
    return messages, sessions


def assert_tables_present(db: Path) -> None:
    conn = sqlite3.connect(f"file:{db}?mode=ro", uri=True)
    try:
        tables = {
            row[0]
            for row in conn.execute("select name from sqlite_master where type='table'")
        }
    finally:
        conn.close()
    assert {"session", "message", "part"} <= tables


def assert_overflow_shape(db: Path) -> None:
    """overflow.db: exactly two assistant rows at i64::MAX input tokens."""
    messages, sessions = read_fixture(db)
    assert len(messages) == 2, f"expected 2 assistant rows, got {len(messages)}"
    payloads = [json.loads(data) for _, _, data in messages]
    assert all(entry["role"] == "assistant" for entry in payloads)
    assert all(entry["modelID"] == MODEL for entry in payloads)
    assert [entry["tokens"]["input"] for entry in payloads] == [I64_MAX, I64_MAX]
    # The reproducer needs no other token fields to be exotic.
    assert all(entry["tokens"]["output"] == 1 for entry in payloads)
    assert sessions == [("s1", "adversarial", NOW_MS, NOW_MS)]


def assert_wrap_shape(db: Path) -> None:
    """wrap.db: exactly one assistant row at u64::MAX input tokens."""
    messages, sessions = read_fixture(db)
    assert len(messages) == 1, f"expected 1 assistant row, got {len(messages)}"
    entry = json.loads(messages[0][2])
    assert entry["role"] == "assistant"
    assert entry["modelID"] == MODEL
    # The JSON text carries the full u64 value; it is the Rust-side
    # `n as i64` cast that wraps to -1, so the fixture must keep every
    # digit exactly as written.
    assert entry["tokens"]["input"] == U64_MAX
    assert entry["tokens"]["output"] == 1
    assert sessions == [("s1", "adversarial", NOW_MS, NOW_MS)]


def test_committed_overflow_db_keeps_its_documented_shape():
    assert_tables_present(COMMITTED_DIR / "overflow.db")
    assert_overflow_shape(COMMITTED_DIR / "overflow.db")


def test_committed_wrap_db_keeps_its_documented_shape():
    assert_tables_present(COMMITTED_DIR / "wrap.db")
    assert_wrap_shape(COMMITTED_DIR / "wrap.db")


def regenerate(tmp_path: Path):
    """Copies the generator into tmp_path (layout-preserving) and runs it.

    Returns (output_dir, completed_process) so tests can assert on both
    the artifacts and the generator's own announcements. Each test calls
    this itself — no cross-test ordering, and re-runs are idempotent
    because the generator unlinks existing databases first.
    """
    copied_script = tmp_path / "scripts" / "fixtures" / "make-adversarial-sqlite.py"
    copied_script.parent.mkdir(parents=True, exist_ok=True)
    shutil.copy2(GENERATOR, copied_script)
    proc = subprocess.run(
        [sys.executable, str(copied_script)],
        capture_output=True,
        text=True,
        cwd=tmp_path,
    )
    assert proc.returncode == 0, f"generator failed: {proc.stderr}"
    return tmp_path / "testdata" / "generated" / "adversarial" / "sqlite", proc


def test_generator_reproduces_both_adversarial_databases(tmp_path):
    out_dir, _ = regenerate(tmp_path)
    for name in ("overflow.db", "wrap.db"):
        assert (out_dir / name).is_file(), f"generator did not write {name}"
        assert_tables_present(out_dir / name)
    assert_overflow_shape(out_dir / "overflow.db")
    assert_wrap_shape(out_dir / "wrap.db")


def test_generator_output_matches_the_committed_payloads(tmp_path):
    out_dir, _ = regenerate(tmp_path)
    for name in ("overflow.db", "wrap.db"):
        generated_messages, generated_sessions = read_fixture(out_dir / name)
        committed_messages, committed_sessions = read_fixture(COMMITTED_DIR / name)
        assert generated_messages == committed_messages, name
        assert generated_sessions == committed_sessions, name


def test_generator_announces_its_outputs(tmp_path):
    _, proc = regenerate(tmp_path)
    expected = tmp_path / "testdata" / "generated" / "adversarial" / "sqlite"
    assert f"wrote {expected / 'overflow.db'}" in proc.stdout, proc.stdout
    assert f"wrote {expected / 'wrap.db'}" in proc.stdout, proc.stdout
