// parallel_bench_test.go - Parallel loading benchmark tests
// Copyright 2026 agenttrace contributors. MIT License.

package engine

import (
	"os"
	"path/filepath"
	"testing"
)

// BenchmarkLoadSessionSingle tests single session loading performance
func BenchmarkLoadSessionSingle(b *testing.B) {
	// 使用测试数据中的文件
	testFiles := []string{
		"../../testdata/claude-code-preamble.jsonl",
		"../../testdata/gemini-current-chat.json",
		"../../testdata/kimi-tool-args.json",
	}

	for _, tf := range testFiles {
		if _, err := os.Stat(tf); os.IsNotExist(err) {
			continue
		}

		b.Run(filepath.Base(tf), func(b *testing.B) {
			for i := 0; i < b.N; i++ {
				LoadSession(tf)
			}
		})
	}
}

// BenchmarkLoadSessionsParallel tests parallel loading performance
func BenchmarkLoadSessionsParallel(b *testing.B) {
	testFiles := []string{
		"../../testdata/claude-code-preamble.jsonl",
		"../../testdata/gemini-current-chat.json",
		"../../testdata/kimi-tool-args.json",
	}

	// 检查文件是否存在
	var existingFiles []string
	for _, f := range testFiles {
		if _, err := os.Stat(f); err == nil {
			existingFiles = append(existingFiles, f)
		}
	}

	if len(existingFiles) == 0 {
		b.Skip("No test files found")
	}

	b.Run("Sequential", func(b *testing.B) {
		for i := 0; i < b.N; i++ {
			for _, f := range existingFiles {
				LoadSession(f)
			}
		}
	})

	b.Run("Parallel", func(b *testing.B) {
		for i := 0; i < b.N; i++ {
			LoadSessionsParallel(existingFiles)
		}
	})
}

// BenchmarkDetectFormat tests format detection performance
func BenchmarkDetectFormat(b *testing.B) {
	testFiles := []string{
		"../../testdata/claude-code-preamble.jsonl",
		"../../testdata/gemini-current-chat.json",
		"../../testdata/kimi-tool-args.json",
	}

	for _, tf := range testFiles {
		if _, err := os.Stat(tf); os.IsNotExist(err) {
			continue
		}

		b.Run(filepath.Base(tf), func(b *testing.B) {
			for i := 0; i < b.N; i++ {
				DetectFormat(tf)
			}
		})
	}
}
