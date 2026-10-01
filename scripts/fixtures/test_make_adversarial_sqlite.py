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

Before this suite the generator had no test coverage: a silent edit to
its row classes or constants would change what the committed fixtures
reproduce with nothing in the tree noticing. These tests pin the
invariants the reproducers depend on — file set, row counts, adversarial
row classes, schema shape, determinism, byte-equality between the
committed databases and a fresh regeneration, and the ``__main__`` path.

Reconciliation note (integration 2026-10-01): rm-002 was implemented
twice in parallel campaigns — the 5-test suite landed via fe8b316
(campaign 4a20d61e) and the 7-test suite via 334b5a8 (run 30484632).
This file is the union: committed-fixture shape asserts and exact
stdout announcements come from the former; module-form regeneration,
schema/session-row shape, byte-determinism, byte-equality against the
committed reproducers, and the script-form test from the latter.
Two of the latter's assertions were made portable at integration:
regenerated-vs-committed (and script-vs-committed) comparisons match on
schema-plus-row dumps, not raw bytes — SQLite stamps the writing
library's version into the file header (offsets 96-99), so byte
equality with the committed files only holds on the exact sqlite that
produced them (3.53.1; this integration host runs 3.37.2). Byte-level
assertions are kept where both sides are produced by the same host's
sqlite: run-to-run determinism and script-form vs module-form.

Everything is hermetic: the committed databases are opened read-only,
and regeneration happens either by importing the generator with
``OUT_DIR`` redirected to a pytest ``tmp_path`` (module form) or on a
layout-preserving copy under ``tmp_path`` (script form — the generator
resolves its output from ``Path(__file__).parents[2]``), never inside
the repository. CI wiring for this test is intentionally out of scope
(outside rm-002's acceptance).

Run: pytest -q scripts/fixtures/
"""

import hashlib
import importlib.util
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
EXPECTED_FILES = {"overflow.db", "wrap.db"}


def load_generator():
    """Import the generator as a module without running it."""
    spec = importlib.util.spec_from_file_location("make_adversarial_sqlite", GENERATOR)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def generate(tmp_path: Path):
    """Run the generator (module form) with OUT_DIR redirected to tmp_path."""
    module = load_generator()
    module.OUT_DIR = tmp_path
    assert module.main() == 0
    return tmp_path


def regenerate(tmp_path: Path):
    """Copies the generator into tmp_path (layout-preserving) and runs it.

    Script form — exercises the real ``__main__`` path and the
    ``Path(__file__).parents[2]`` resolution. Returns (output_dir,
    completed_process) so tests can assert on both the artifacts and the
    generator's own announcements. Each test calls this itself — no
    cross-test ordering, and re-runs are idempotent because the
    generator unlinks existing databases first.
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


def rows(db_path: Path, table: str):
    with sqlite3.connect(db_path) as conn:
        return conn.execute(f"select * from {table}").fetchall()


def dump(db: Path):
    """Portable content fingerprint: schema SQL plus every table's rows.

    Byte-comparing database files is not portable: SQLite stores the
    writing library's version number in the file header (offsets
    96-99), so a regeneration on a host with a different sqlite never
    matches the committed bytes even when the content is identical.
    Comparing dumps pins the content contract instead.
    """
    conn = sqlite3.connect(f"file:{db}?mode=ro", uri=True)
    try:
        schema = [
            row[0]
            for row in conn.execute(
                "select sql from sqlite_master where sql is not null order by name"
            )
        ]
        tables = {
            row[0]: conn.execute(f"select * from {row[0]} order by 1").fetchall()
            for row in conn.execute(
                "select name from sqlite_master where type='table' order by name"
            )
        }
    finally:
        conn.close()
    return schema, tables


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


def test_creates_both_databases(tmp_path):
    out = generate(tmp_path)
    assert {p.name for p in out.iterdir()} == EXPECTED_FILES


def test_overflow_reproducer_shape(tmp_path):
    """overflow.db carries two i64::MAX inputs — the P5-1 overflow class."""
    assert_overflow_shape(generate(tmp_path) / "overflow.db")


def test_wrap_reproducer_shape(tmp_path):
    """wrap.db carries one u64::MAX input — the P5-2 wrap class."""
    assert_wrap_shape(generate(tmp_path) / "wrap.db")


def test_schema_and_session_row(tmp_path):
    """Both databases share the discovery-path schema and one session row."""
    for name in sorted(EXPECTED_FILES):
        db = generate(tmp_path) / name
        assert_tables_present(db)
        assert rows(db, "session") == [("s1", "adversarial", NOW_MS, NOW_MS)]
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
    """A fresh regeneration matches the committed reproducers' content.

    Guards drift: if the generator or the committed databases are edited
    without the other following, the committed reproducers no longer
    document what the generator claims to produce. Compared as schema +
    row dumps, not raw bytes (see ``dump``).
    """
    regenerated = generate(tmp_path)
    for name in sorted(EXPECTED_FILES):
        committed = COMMITTED_DIR / name
        assert committed.is_file(), f"committed fixture missing: {committed}"
        assert dump(regenerated / name) == dump(committed), (
            f"committed {name} differs from generator output — regenerate with"
            " python3 scripts/fixtures/make-adversarial-sqlite.py and review"
            " the diff"
        )


def test_committed_overflow_db_keeps_its_documented_shape():
    assert_tables_present(COMMITTED_DIR / "overflow.db")
    assert_overflow_shape(COMMITTED_DIR / "overflow.db")


def test_committed_wrap_db_keeps_its_documented_shape():
    assert_tables_present(COMMITTED_DIR / "wrap.db")
    assert_wrap_shape(COMMITTED_DIR / "wrap.db")


def test_generator_announces_its_outputs(tmp_path):
    _, proc = regenerate(tmp_path)
    expected = tmp_path / "testdata" / "generated" / "adversarial" / "sqlite"
    assert f"wrote {expected / 'overflow.db'}" in proc.stdout, proc.stdout
    assert f"wrote {expected / 'wrap.db'}" in proc.stdout, proc.stdout


def test_generator_runs_as_script(tmp_path):
    """The __main__ entrypoint exits 0 and the script form matches the
    module form byte-for-byte (same host sqlite) and the committed
    reproducers by content."""
    out_dir, proc = regenerate(tmp_path)
    assert proc.stdout.count("wrote ") == 2
    assert {p.name for p in out_dir.iterdir()} == EXPECTED_FILES
    module_out = generate(tmp_path / "module-form")
    for name in sorted(EXPECTED_FILES):
        assert (out_dir / name).read_bytes() == (module_out / name).read_bytes()
        assert dump(out_dir / name) == dump(COMMITTED_DIR / name)
