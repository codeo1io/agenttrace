// sqlite_index_bench_test.go - SQLite index optimization benchmark tests
// Copyright 2026 agenttrace contributors. MIT License.

package engine

import (
	"database/sql"
	"fmt"
	"os"
	"path/filepath"
	"testing"

	_ "modernc.org/sqlite"
)

// BenchmarkSQLiteWithIndexes tests query performance with indexes
func BenchmarkSQLiteWithIndexes(b *testing.B) {
	// 创建临时数据库
	tmpDir := b.TempDir()
	dbPath := filepath.Join(tmpDir, "test.db")

	db, err := sql.Open("sqlite", dbPath)
	if err != nil {
		b.Fatal(err)
	}

	// 创建测试表
	createTestTables(db)

	// 插入测试数据
	insertTestData(db, 1000)

	db.Close()

	b.ResetTimer()

	for i := 0; i < b.N; i++ {
		db, err := openSQLiteWithIndexes(dbPath, EnsureHermesIndexes)
		if err != nil {
			b.Fatal(err)
		}

		// 执行查询
		rows, err := db.Query("SELECT id, model FROM sessions WHERE started_at > 0")
		if err != nil {
			b.Fatal(err)
		}
		rows.Close()

		db.Close()
	}
}

// BenchmarkSQLiteWithoutIndexes tests query performance without indexes
func BenchmarkSQLiteWithoutIndexes(b *testing.B) {
	// 创建临时数据库
	tmpDir := b.TempDir()
	dbPath := filepath.Join(tmpDir, "test.db")

	db, err := sql.Open("sqlite", dbPath)
	if err != nil {
		b.Fatal(err)
	}

	// 创建测试表（无索引）
	createTestTables(db)

	// 插入测试数据
	insertTestData(db, 1000)

	db.Close()

	b.ResetTimer()

	for i := 0; i < b.N; i++ {
		db, err := openSQLiteReadOnly(dbPath)
		if err != nil {
			b.Fatal(err)
		}

		// 执行查询
		rows, err := db.Query("SELECT id, model FROM sessions WHERE started_at > 0")
		if err != nil {
			b.Fatal(err)
		}
		rows.Close()

		db.Close()
	}
}

// createTestTables creates test tables
func createTestTables(db *sql.DB) {
	db.Exec(`
		CREATE TABLE IF NOT EXISTS sessions (
			id TEXT PRIMARY KEY,
			model TEXT,
			started_at REAL,
			ended_at REAL,
			message_count INTEGER,
			tool_call_count INTEGER,
			input_tokens INTEGER,
			output_tokens INTEGER,
			cache_read_tokens INTEGER,
			cache_write_tokens INTEGER
		)
	`)

	db.Exec(`
		CREATE TABLE IF NOT EXISTS messages (
			id INTEGER PRIMARY KEY AUTOINCREMENT,
			session_id TEXT,
			role TEXT,
			content TEXT,
			timestamp REAL
		)
	`)
}

// insertTestData inserts test data
func insertTestData(db *sql.DB, count int) {
	tx, _ := db.Begin()

	stmt, _ := tx.Prepare(`
		INSERT INTO sessions (id, model, started_at, ended_at, message_count, tool_call_count, 
			input_tokens, output_tokens, cache_read_tokens, cache_write_tokens)
		VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
	`)

	for i := 0; i < count; i++ {
		stmt.Exec(
			fmt.Sprintf("session-%d", i),
			"test-model",
			float64(1000000+i),
			float64(1000000+i+100),
			10,
			5,
			1000,
			500,
			200,
			100,
		)
	}

	stmt.Close()
	tx.Commit()
}

// BenchmarkIndexCreation tests index creation performance
func BenchmarkIndexCreation(b *testing.B) {
	tmpDir := b.TempDir()
	dbPath := filepath.Join(tmpDir, "test.db")

	for i := 0; i < b.N; i++ {
		db, err := sql.Open("sqlite", dbPath)
		if err != nil {
			b.Fatal(err)
		}

		createTestTables(db)
		insertTestData(db, 100)

		im := NewSQLiteIndexManager(db)
		im.EnsureIndexes(GetHermesIndexes())

		db.Close()

		os.Remove(dbPath)
	}
}
