// sqlite_index.go - SQLite index optimization
// Copyright 2026 agenttrace contributors. MIT License.

package engine

import (
	"database/sql"
	"fmt"
)

// SQLiteIndexManager manages SQLite indexes.
type SQLiteIndexManager struct {
	db *sql.DB
}

// NewSQLiteIndexManager creates a new index manager.
func NewSQLiteIndexManager(db *sql.DB) *SQLiteIndexManager {
	return &SQLiteIndexManager{db: db}
}

// IndexDefinition defines an index.
type IndexDefinition struct {
	Name      string
	Table     string
	Columns   []string
	Unique    bool
	Condition string // WHERE condition for partial indexes
}

// GetHermesIndexes returns recommended indexes for Hermes database.
func GetHermesIndexes() []IndexDefinition {
	return []IndexDefinition{
		{
			Name:    "idx_sessions_started_at",
			Table:   "sessions",
			Columns: []string{"started_at"},
		},
		{
			Name:    "idx_sessions_ended_at",
			Table:   "sessions",
			Columns: []string{"ended_at"},
		},
		{
			Name:    "idx_sessions_model",
			Table:   "sessions",
			Columns: []string{"model"},
		},
		{
			Name:    "idx_messages_session_id",
			Table:   "messages",
			Columns: []string{"session_id"},
		},
		{
			Name:    "idx_messages_session_role",
			Table:   "messages",
			Columns: []string{"session_id", "role"},
		},
	}
}

// GetOpenCodeIndexes returns recommended indexes for OpenCode database.
func GetOpenCodeIndexes() []IndexDefinition {
	return []IndexDefinition{
		{
			Name:    "idx_session_time_created",
			Table:   "session",
			Columns: []string{"time_created"},
		},
		{
			Name:    "idx_session_time_updated",
			Table:   "session",
			Columns: []string{"time_updated"},
		},
		{
			Name:    "idx_message_session_id",
			Table:   "message",
			Columns: []string{"session_id"},
		},
		{
			Name:    "idx_part_session_id",
			Table:   "part",
			Columns: []string{"session_id"},
		},
	}
}

// EnsureIndexes ensures all indexes exist.
func (im *SQLiteIndexManager) EnsureIndexes(indexes []IndexDefinition) error {
	for _, idx := range indexes {
		if err := im.createIndexIfNotExists(idx); err != nil {
			return fmt.Errorf("failed to create index %s: %w", idx.Name, err)
		}
	}
	return nil
}

// createIndexIfNotExists creates an index if it doesn't exist.
func (im *SQLiteIndexManager) createIndexIfNotExists(idx IndexDefinition) error {
	// Check if index exists
	var exists bool
	query := `SELECT COUNT(*) > 0 FROM sqlite_master WHERE type='index' AND name=?`
	if err := im.db.QueryRow(query, idx.Name).Scan(&exists); err != nil {
		return err
	}
	if exists {
		return nil
	}

	// Build CREATE INDEX statement
	unique := ""
	if idx.Unique {
		unique = "UNIQUE "
	}

	columns := ""
	for i, col := range idx.Columns {
		if i > 0 {
			columns += ", "
		}
		columns += col
	}

	condition := ""
	if idx.Condition != "" {
		condition = " WHERE " + idx.Condition
	}

	sql := fmt.Sprintf("CREATE %sINDEX IF NOT EXISTS %s ON %s (%s)%s",
		unique, idx.Name, idx.Table, columns, condition)

	_, err := im.db.Exec(sql)
	return err
}

// AnalyzeAll analyzes all existing tables to update statistics.
func (im *SQLiteIndexManager) AnalyzeAll() error {
	// Query actual tables in the database
	rows, err := im.db.Query("SELECT name FROM sqlite_master WHERE type='table'")
	if err != nil {
		return err
	}
	defer rows.Close()

	var tables []string
	for rows.Next() {
		var name string
		if err := rows.Scan(&name); err != nil {
			continue
		}
		tables = append(tables, name)
	}

	// Only analyze existing tables
	for _, table := range tables {
		if _, err := im.db.Exec("ANALYZE " + table); err != nil {
			// Ignore errors for system tables
			continue
		}
	}
	return nil
}

// GetIndexStats returns index statistics.
func (im *SQLiteIndexManager) GetIndexStats() ([]IndexStat, error) {
	rows, err := im.db.Query(`
		SELECT 
			name,
			tbl_name,
			sql
		FROM sqlite_master 
		WHERE type='index' 
		ORDER BY tbl_name, name
	`)
	if err != nil {
		return nil, err
	}
	defer rows.Close()

	var stats []IndexStat
	for rows.Next() {
		var stat IndexStat
		if err := rows.Scan(&stat.Name, &stat.Table, &stat.SQL); err != nil {
			continue
		}
		stats = append(stats, stat)
	}
	return stats, nil
}

// IndexStat holds index statistics.
type IndexStat struct {
	Name  string
	Table string
	SQL   string
}

// OptimizeDatabase optimizes the database.
func (im *SQLiteIndexManager) OptimizeDatabase() error {
	// Analyze tables to update statistics
	if err := im.AnalyzeAll(); err != nil {
		return err
	}

	// Defragment
	if _, err := im.db.Exec("VACUUM"); err != nil {
		return err
	}

	return nil
}

// EnsureHermesIndexes ensures Hermes database indexes.
func EnsureHermesIndexes(db *sql.DB) error {
	im := NewSQLiteIndexManager(db)
	return im.EnsureIndexes(GetHermesIndexes())
}

// EnsureOpenCodeIndexes ensures OpenCode database indexes.
func EnsureOpenCodeIndexes(db *sql.DB) error {
	im := NewSQLiteIndexManager(db)
	return im.EnsureIndexes(GetOpenCodeIndexes())
}
