// stream_parser_bench_test.go - Streaming parser benchmark tests
// Copyright 2026 agenttrace contributors. MIT License.

package engine

import (
	"bytes"
	"encoding/json"
	"os"
	"strings"
	"testing"
)

// BenchmarkStreamParser tests streaming parser performance
func BenchmarkStreamParser(b *testing.B) {
	// 创建测试数据
	data := createTestJSONLData(1000)

	b.ResetTimer()

	for i := 0; i < b.N; i++ {
		reader := strings.NewReader(data)
		parser := NewStreamParser(reader, 0)

		count := 0
		for parser.scanner.Scan() {
			line := strings.TrimSpace(parser.scanner.Text())
			if line == "" {
				continue
			}
			var obj map[string]interface{}
			json.Unmarshal([]byte(line), &obj)
			count++
		}
	}
}

// BenchmarkStandardParser tests standard parser performance
func BenchmarkStandardParser(b *testing.B) {
	// 创建测试数据
	data := createTestJSONLData(1000)

	b.ResetTimer()

	for i := 0; i < b.N; i++ {
		lines := strings.Split(data, "\n")
		count := 0
		for _, line := range lines {
			line = strings.TrimSpace(line)
			if line == "" {
				continue
			}
			var obj map[string]interface{}
			json.Unmarshal([]byte(line), &obj)
			count++
		}
	}
}

// BenchmarkStreamEventParser tests streaming event parser performance
func BenchmarkStreamEventParser(b *testing.B) {
	// 创建测试数据
	data := createTestJSONLData(1000)

	b.ResetTimer()

	for i := 0; i < b.N; i++ {
		reader := strings.NewReader(data)
		parser := NewStreamEventParser(reader, 0)

		count := 0
		parser.ParseEvents(func(ev Event) error {
			count++
			return nil
		})
	}
}

// BenchmarkParseJSONLStream tests streaming JSONL parsing performance
func BenchmarkParseJSONLStream(b *testing.B) {
	// 创建测试数据
	data := createTestJSONLData(1000)

	b.ResetTimer()

	for i := 0; i < b.N; i++ {
		reader := strings.NewReader(data)
		count := 0
		ParseJSONLStream(reader, func(obj map[string]interface{}) error {
			count++
			return nil
		})
	}
}

// BenchmarkParseJSONLStreamBatch tests batch streaming parsing performance
func BenchmarkParseJSONLStreamBatch(b *testing.B) {
	// 创建测试数据
	data := createTestJSONLData(1000)

	b.ResetTimer()

	for i := 0; i < b.N; i++ {
		reader := strings.NewReader(data)
		count := 0
		ParseJSONLStreamBatch(reader, 100, func(batch []map[string]interface{}) error {
			count += len(batch)
			return nil
		})
	}
}

// BenchmarkParseLargeJSONL tests large file parsing performance
func BenchmarkParseLargeJSONL(b *testing.B) {
	// 创建临时文件
	tmpDir := b.TempDir()
	filePath := tmpDir + "/test.jsonl"

	// 创建测试数据
	data := createTestJSONLData(5000)
	writeTestFile(filePath, data)

	b.ResetTimer()

	for i := 0; i < b.N; i++ {
		events, _ := ParseLargeJSONL(filePath, 1000)
		_ = events
	}
}

// createTestJSONLData creates test JSONL data
func createTestJSONLData(lines int) string {
	var buf bytes.Buffer

	for i := 0; i < lines; i++ {
		obj := map[string]interface{}{
			"role":      "user",
			"content":   "test message",
			"timestamp": "2026-01-01T00:00:00Z",
		}
		data, _ := json.Marshal(obj)
		buf.Write(data)
		buf.WriteString("\n")
	}

	return buf.String()
}

// writeTestFile writes test file
func writeTestFile(path string, data string) {
	os.WriteFile(path, []byte(data), 0644)
}
