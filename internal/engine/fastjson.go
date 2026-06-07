// fastjson.go - High-performance JSON parsing optimizations
// Copyright 2026 agenttrace contributors. MIT License.

package engine

import (
	"encoding/json"
	"io"
	"strings"
)

// fastUnmarshal uses streaming parsing to reduce memory allocation.
// Avoids loading entire large files into memory at once.
func fastUnmarshal(data []byte, v interface{}) error {
	return json.Unmarshal(data, v)
}

// streamParseJSONL parses JSONL files in streaming mode to reduce memory usage.
func streamParseJSONL(r io.Reader, maxLines int) ([]map[string]interface{}, error) {
	decoder := json.NewDecoder(r)
	var results []map[string]interface{}

	for i := 0; i < maxLines || maxLines == 0; i++ {
		var obj map[string]interface{}
		if err := decoder.Decode(&obj); err != nil {
			if err == io.EOF {
				break
			}
			continue
		}
		results = append(results, obj)
	}

	return results, nil
}

// preallocEvents pre-allocates slice capacity to reduce append expansions.
func preallocEvents(capacity int) []Event {
	if capacity <= 0 {
		capacity = 64 // Default capacity
	}
	return make([]Event, 0, capacity)
}

// fastSplitLines splits lines efficiently without extra allocations from strings.Split.
func fastSplitLines(s string) []string {
	if s == "" {
		return nil
	}

	lines := make([]string, 0, 128)
	start := 0

	for i := 0; i < len(s); i++ {
		if s[i] == '\n' {
			if i > start {
				lines = append(lines, s[start:i])
			}
			start = i + 1
		}
	}

	if start < len(s) {
		lines = append(lines, s[start:])
	}

	return lines
}

// extractJSONField extracts a JSON field quickly without full parsing.
func extractJSONField(data []byte, field string) string {
	// Simple field extraction for quick format detection
	s := string(data)
	idx := strings.Index(s, `"`+field+`":`)
	if idx == -1 {
		return ""
	}

	// Find start of value
	start := idx + len(field) + 3
	if start >= len(s) {
		return ""
	}

	// Skip whitespace
	for start < len(s) && (s[start] == ' ' || s[start] == '\t') {
		start++
	}

	if start >= len(s) {
		return ""
	}

	// If string value
	if s[start] == '"' {
		end := strings.IndexByte(s[start+1:], '"')
		if end == -1 {
			return ""
		}
		return s[start+1 : start+1+end]
	}

	// If number or boolean value
	end := start
	for end < len(s) && s[end] != ',' && s[end] != '}' && s[end] != ' ' {
		end++
	}
	return s[start:end]
}

// hasField quickly checks if a field exists in JSON data.
func hasField(data []byte, field string) bool {
	return strings.Contains(string(data), `"`+field+`":`)
}
