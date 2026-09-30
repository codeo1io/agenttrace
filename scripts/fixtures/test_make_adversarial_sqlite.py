"""Pytest suite for scripts/fixtures/make-adversarial-sqlite.py (rm-002).

The generator produces the committed adversarial SQLite reproducers for
the assessment pass-5 findings against
crates/agenttrace-core/src/sqlite_sessions.rs:

  - overflow.db — two assistant messages with tokens.input = i64::MAX
    each, so the per-session accumulator `agg.input_tokens += input`
    overflows in debug builds (P5-1);
  - wrap.db — one assistant message with tokens.input = u64::MAX, which
    the `n as i64` helper wraps to -1 (P5-2).

Before this suite the generator had no test coverage: a silent edit to
its row classes or constants would change what the committed fixtures
reproduce with nothing in the tree noticing. These tests pin the
invariants the reproducers depend on — file set, row counts, adversarial
row classes, schema shape, determinism, and byte-equality between the
committed databases and a fresh regeneration.

Run from the repository root: pytest -q scripts/fixtures/

The suite never writes into the tree: OUT_DIR is redirected to a
pytest tmp_path and the committed databases are only read.
"""

import hashlib
import importlib.util
import json
import sqlite3
from pathlib import Path

import pytest

HERE = Path(__file__).resolve().parent
GENERATOR = HERE / "make-adversarial-sqlite.py"
COMMITTED_DIR = HERE.parents[1] / "testdata" / "generated" / "adversarial" / "sqlite"

I64_MAX = 9223372036854775807
U64_MAX = 18446744073709551615

EXPECTED_FILES = {"overflow.db", "wrap.db"}


def load_generator():
    """Import the generator as a module without running it."""
    spec = importlib.util.spec_from_file_location("make_adversarial_sqlite", GENERATOR)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def generate(tmp_path):
    """Run the generator with OUT_DIR redirected to tmp_path."""
    module = load_generator()
    module.OUT_DIR = tmp_path
    assert module.main() == 0
    return tmp_path


def rows(db_path, table):
    with sqlite3.connect(db_path) as conn:
        return conn.execute(f"select * from {table}").fetchall()


def message_payloads(db_path):
    """Return the parsed JSON payloads of the message rows, in row order."""
    with sqlite3.connect(db_path) as conn:
        raw = conn.execute("select data from message order by id").fetchall()
    return [json.loads(row[0]) for row in raw]


def test_creates_both_databases(tmp_path):
    out = generate(tmp_path)
    assert {p.name for p in out.iterdir()} == EXPECTED_FILES


def test_overflow_reproducer_shape(tmp_path):
    """overflow.db carries two i64::MAX inputs — the P5-1 overflow class."""
    payloads = message_payloads(generate(tmp_path) / "overflow.db")
    assert len(payloads) == 2
    for payload in payloads:
        assert payload["role"] == "assistant"
        assert payload["tokens"]["input"] == I64_MAX
        assert payload["tokens"]["output"] == 1


def test_wrap_reproducer_shape(tmp_path):
    """wrap.db carries one u64::MAX input — the P5-2 wrap class."""
    payloads = message_payloads(generate(tmp_path) / "wrap.db")
    assert len(payloads) == 1
    payload = payloads[0]
    assert payload["role"] == "assistant"
    assert payload["tokens"]["input"] == U64_MAX


def test_schema_and_session_row(tmp_path):
    """Both databases share the discovery-path schema and one session row."""
    for name in sorted(EXPECTED_FILES):
        db = generate(tmp_path) / name
        with sqlite3.connect(db) as conn:
            tables = {
                name_[0]
                for name_ in conn.execute(
                    "select name from sqlite_master where type='table'"
                ).fetchall()
            }
        assert {"session", "message", "part"} <= tables
        session_rows = rows(db, "session")
        assert session_rows == [("s1", "adversarial", 1770000000000, 1770000000000)]
        assert rows(db, "part") == []


def test_regeneration_is_deterministic(tmp_path):
    """Two runs produce byte-identical databases (fixed NOW_MS, no RNG)."""
    first = generate(tmp_path / "run1")
    second = generate(tmp_path / "run2")
    for name in sorted(EXPECTED_FILES):
        left = hashlib.sha256((first / name).read_bytes()).hexdigest()
        right = hashlib.sha256((second / name).read_bytes()).hexdigest()
        assert left == right, f"{name} is not deterministic across runs"


def test_committed_fixtures_match_generator(tmp_path):
    """A fresh regeneration is byte-identical to the committed reproducers.

    Guards drift: if the generator or the committed databases are edited
    without the other following, the committed reproducers no longer
    document what the generator claims to produce.
    """
    regenerated = generate(tmp_path)
    for name in sorted(EXPECTED_FILES):
        committed = COMMITTED_DIR / name
        assert committed.is_file(), f"committed fixture missing: {committed}"
        assert (regenerated / name).read_bytes() == committed.read_bytes(), (
            f"committed {name} differs from generator output — regenerate with"
            " python3 scripts/fixtures/make-adversarial-sqlite.py and review"
            " the diff"
        )


def test_generator_runs_as_script(tmp_path):
    """The __main__ entrypoint exits 0 and writes the expected fixtures."""
    import subprocess
    import sys

    layout = tmp_path / "layout"
    out_dir = layout / "testdata" / "generated" / "adversarial" / "sqlite"
    (layout / "scripts" / "fixtures").mkdir(parents=True)
    out_dir.mkdir(parents=True)
    (layout / "scripts" / "fixtures" / "make-adversarial-sqlite.py").write_text(
        GENERATOR.read_text(), encoding="utf-8"
    )
    proc = subprocess.run(
        [sys.executable, str(layout / "scripts" / "fixtures" / "make-adversarial-sqlite.py")],
        capture_output=True,
        text=True,
        check=False,
    )
    assert proc.returncode == 0, proc.stderr
    assert proc.stdout.count("wrote ") == 2
    assert {p.name for p in out_dir.iterdir()} == EXPECTED_FILES
    # The script form matches the module form: both equal the committed bytes.
    for name in sorted(EXPECTED_FILES):
        assert (out_dir / name).read_bytes() == (COMMITTED_DIR / name).read_bytes()
