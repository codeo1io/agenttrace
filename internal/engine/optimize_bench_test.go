// optimize_bench_test.go - Comprehensive performance benchmark tests
// Copyright 2026 agenttrace contributors. MIT License.

package engine

import (
	"os"
	"path/filepath"
	"runtime"
	"sync"
	"testing"
)

// BenchmarkDetectFormatOptimized tests optimized format detection
func BenchmarkDetectFormatOptimized(b *testing.B) {
	testFiles := getTestFiles(b)
	if len(testFiles) == 0 {
		b.Skip("No test files found")
	}

	for _, tf := range testFiles {
		b.Run(filepath.Base(tf), func(b *testing.B) {
			b.ResetTimer()
			for i := 0; i < b.N; i++ {
				DetectFormat(tf)
			}
		})
	}
}

// BenchmarkParseHermesJSONL tests Hermes JSONL parsing performance
func BenchmarkParseHermesJSONL(b *testing.B) {
	// 创建测试数据
	testData := createTestJSONL(100)
	b.ResetTimer()

	for i := 0; i < b.N; i++ {
		parseHermesJSONL(testData)
	}
}

// BenchmarkFastSplitLines tests fast line splitting performance
func BenchmarkFastSplitLines(b *testing.B) {
	testData := createTestJSONL(100)
	b.ResetTimer()

	for i := 0; i < b.N; i++ {
		fastSplitLines(testData)
	}
}

// BenchmarkStringSplit tests standard string split performance (comparison)
func BenchmarkStringSplit(b *testing.B) {
	testData := createTestJSONL(100)
	b.ResetTimer()

	for i := 0; i < b.N; i++ {
		_ = splitLines(testData)
	}
}

// splitLines standard split implementation (for comparison)
func splitLines(s string) []string {
	var lines []string
	start := 0
	for i := 0; i < len(s); i++ {
		if s[i] == '\n' {
			lines = append(lines, s[start:i])
			start = i + 1
		}
	}
	if start < len(s) {
		lines = append(lines, s[start:])
	}
	return lines
}

// BenchmarkParallelVsSequential compares parallel vs sequential processing
func BenchmarkParallelVsSequential(b *testing.B) {
	testFiles := getTestFiles(b)
	if len(testFiles) == 0 {
		b.Skip("No test files found")
	}

	b.Run("Sequential", func(b *testing.B) {
		for i := 0; i < b.N; i++ {
			for _, f := range testFiles {
				LoadSession(f)
			}
		}
	})

	b.Run("Parallel-2", func(b *testing.B) {
		for i := 0; i < b.N; i++ {
			loadSessionsWithWorkers(testFiles, 2)
		}
	})

	b.Run("Parallel-4", func(b *testing.B) {
		for i := 0; i < b.N; i++ {
			loadSessionsWithWorkers(testFiles, 4)
		}
	})

	b.Run("Parallel-8", func(b *testing.B) {
		for i := 0; i < b.N; i++ {
			loadSessionsWithWorkers(testFiles, 8)
		}
	})
}

// loadSessionsWithWorkers loads sessions using specified number of workers
func loadSessionsWithWorkers(paths []string, workers int) []Session {
	if len(paths) == 0 {
		return nil
	}
	if workers <= 0 {
		workers = 4
	}
	if len(paths) < workers {
		workers = len(paths)
	}

	type result struct {
		session Session
		ok      bool
	}

	results := make([]result, len(paths))
	jobs := make(chan int, len(paths))
	var wg sync.WaitGroup

	for w := 0; w < workers; w++ {
		wg.Add(1)
		go func() {
			defer wg.Done()
			for idx := range jobs {
				s, err := LoadSession(paths[idx])
				if err != nil {
					results[idx] = result{ok: false}
					continue
				}
				results[idx] = result{session: *s, ok: true}
			}
		}()
	}

	for i := range paths {
		jobs <- i
	}
	close(jobs)
	wg.Wait()

	var sessions []Session
	for _, r := range results {
		if r.ok {
			sessions = append(sessions, r.session)
		}
	}
	return sessions
}

// BenchmarkMemoryAllocation tests memory allocation optimization
func BenchmarkMemoryAllocation(b *testing.B) {
	b.Run("EventsSlice", func(b *testing.B) {
		for i := 0; i < b.N; i++ {
			events := NewEventsSlice(128)
			for j := 0; j < 100; j++ {
				events.Append(Event{
					Role:    "test",
					Content: "test content",
				})
			}
			_ = events.Events()
		}
	})

	b.Run("RegularSlice", func(b *testing.B) {
		for i := 0; i < b.N; i++ {
			events := make([]Event, 0, 128)
			for j := 0; j < 100; j++ {
				events = append(events, Event{
					Role:    "test",
					Content: "test content",
				})
			}
			_ = events
		}
	})
}

// BenchmarkBatchProcessor tests batch processor performance
func BenchmarkBatchProcessor(b *testing.B) {
	testFiles := getTestFiles(b)
	if len(testFiles) == 0 {
		b.Skip("No test files found")
	}

	b.Run("DirectLoad", func(b *testing.B) {
		for i := 0; i < b.N; i++ {
			var sessions []Session
			for _, f := range testFiles {
				s, _ := LoadSession(f)
				if s != nil {
					sessions = append(sessions, *s)
				}
			}
		}
	})

	b.Run("BatchProcessor", func(b *testing.B) {
		for i := 0; i < b.N; i++ {
			bp := NewBatchProcessor()
			bp.Process(testFiles, runtime.NumCPU())
		}
	})
}

// Helper functions

func getTestFiles(b *testing.B) []string {
	b.Helper()

	testFiles := []string{
		"../../testdata/claude-code-preamble.jsonl",
		"../../testdata/gemini-current-chat.json",
		"../../testdata/kimi-tool-args.json",
		"../../testdata/copilot-attrs-map.jsonl",
	}

	var existing []string
	for _, f := range testFiles {
		if _, err := os.Stat(f); err == nil {
			existing = append(existing, f)
		}
	}
	return existing
}

func createTestJSONL(lines int) string {
	s := ""
	for i := 0; i < lines; i++ {
		s += `{"role":"user","content":"test message ` + string(rune('0'+i%10)) + `","timestamp":"2026-01-01T00:00:00Z"}` + "\n"
	}
	return s
}
