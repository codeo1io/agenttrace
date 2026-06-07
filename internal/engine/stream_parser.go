// stream_parser.go - Streaming JSON parser
// Copyright 2026 agenttrace contributors. MIT License.

package engine

import (
	"bufio"
	"encoding/json"
	"io"
	"os"
	"strings"
)

// StreamParser parses JSONL files in streaming mode.
type StreamParser struct {
	reader    io.Reader
	scanner   *bufio.Scanner
	buffer    []byte
	maxBuffer int
}

// NewStreamParser creates a new streaming parser.
func NewStreamParser(reader io.Reader, maxBuffer int) *StreamParser {
	if maxBuffer <= 0 {
		maxBuffer = 64 * 1024 // 64KB default buffer
	}

	scanner := bufio.NewScanner(reader)
	scanner.Buffer(make([]byte, maxBuffer), maxBuffer)

	return &StreamParser{
		reader:    reader,
		scanner:   scanner,
		maxBuffer: maxBuffer,
	}
}

// ParseJSONLStream parses JSONL files in streaming mode.
func ParseJSONLStream(reader io.Reader, callback func(map[string]interface{}) error) error {
	parser := NewStreamParser(reader, 0)

	for parser.scanner.Scan() {
		line := strings.TrimSpace(parser.scanner.Text())
		if line == "" || strings.HasPrefix(line, "#") {
			continue
		}

		var obj map[string]interface{}
		if err := json.Unmarshal([]byte(line), &obj); err != nil {
			continue
		}

		if err := callback(obj); err != nil {
			return err
		}
	}

	return parser.scanner.Err()
}

// ParseJSONLStreamBatch parses JSONL files in streaming batches.
func ParseJSONLStreamBatch(reader io.Reader, batchSize int, callback func([]map[string]interface{}) error) error {
	parser := NewStreamParser(reader, 0)

	batch := make([]map[string]interface{}, 0, batchSize)

	for parser.scanner.Scan() {
		line := strings.TrimSpace(parser.scanner.Text())
		if line == "" || strings.HasPrefix(line, "#") {
			continue
		}

		var obj map[string]interface{}
		if err := json.Unmarshal([]byte(line), &obj); err != nil {
			continue
		}

		batch = append(batch, obj)

		if len(batch) >= batchSize {
			if err := callback(batch); err != nil {
				return err
			}
			batch = batch[:0] // Reset slice, keep capacity
		}
	}

	// Process last batch
	if len(batch) > 0 {
		if err := callback(batch); err != nil {
			return err
		}
	}

	return parser.scanner.Err()
}

// ParseJSONLStreamWithLimit parses JSONL files with a limit.
func ParseJSONLStreamWithLimit(reader io.Reader, limit int, callback func(map[string]interface{}) error) error {
	parser := NewStreamParser(reader, 0)
	count := 0

	for parser.scanner.Scan() {
		if limit > 0 && count >= limit {
			break
		}

		line := strings.TrimSpace(parser.scanner.Text())
		if line == "" || strings.HasPrefix(line, "#") {
			continue
		}

		var obj map[string]interface{}
		if err := json.Unmarshal([]byte(line), &obj); err != nil {
			continue
		}

		if err := callback(obj); err != nil {
			return err
		}

		count++
	}

	return parser.scanner.Err()
}

// StreamEventParser parses events in streaming mode.
type StreamEventParser struct {
	reader    io.Reader
	maxBuffer int
}

// NewStreamEventParser creates a new streaming event parser.
func NewStreamEventParser(reader io.Reader, maxBuffer int) *StreamEventParser {
	if maxBuffer <= 0 {
		maxBuffer = 64 * 1024
	}

	return &StreamEventParser{
		reader:    reader,
		maxBuffer: maxBuffer,
	}
}

// ParseEvents parses events in streaming mode.
func (sep *StreamEventParser) ParseEvents(callback func(Event) error) error {
	parser := NewStreamParser(sep.reader, sep.maxBuffer)

	for parser.scanner.Scan() {
		line := strings.TrimSpace(parser.scanner.Text())
		if line == "" {
			continue
		}

		var ev Event
		if err := json.Unmarshal([]byte(line), &ev); err != nil {
			continue
		}

		if err := callback(ev); err != nil {
			return err
		}
	}

	return parser.scanner.Err()
}

// ParseEventsWithFilter parses events with filtering.
func (sep *StreamEventParser) ParseEventsWithFilter(filter func(Event) bool, callback func(Event) error) error {
	parser := NewStreamParser(sep.reader, sep.maxBuffer)

	for parser.scanner.Scan() {
		line := strings.TrimSpace(parser.scanner.Text())
		if line == "" {
			continue
		}

		var ev Event
		if err := json.Unmarshal([]byte(line), &ev); err != nil {
			continue
		}

		if filter(ev) {
			if err := callback(ev); err != nil {
				return err
			}
		}
	}

	return parser.scanner.Err()
}

// StreamStats holds streaming parsing statistics.
type StreamStats struct {
	LinesRead    int
	LinesParsed  int
	LinesSkipped int
	Errors       int
}

// ParseJSONLWithStats parses JSONL with statistics tracking.
func ParseJSONLWithStats(reader io.Reader, callback func(map[string]interface{}) error) (StreamStats, error) {
	parser := NewStreamParser(reader, 0)
	stats := StreamStats{}

	for parser.scanner.Scan() {
		stats.LinesRead++

		line := strings.TrimSpace(parser.scanner.Text())
		if line == "" || strings.HasPrefix(line, "#") {
			stats.LinesSkipped++
			continue
		}

		var obj map[string]interface{}
		if err := json.Unmarshal([]byte(line), &obj); err != nil {
			stats.Errors++
			continue
		}

		stats.LinesParsed++

		if err := callback(obj); err != nil {
			return stats, err
		}
	}

	return stats, parser.scanner.Err()
}

// ParseLargeJSONL parses large JSONL files with low memory usage.
func ParseLargeJSONL(path string, maxEvents int) ([]Event, error) {
	file, err := os.Open(path)
	if err != nil {
		return nil, err
	}
	defer file.Close()

	events := make([]Event, 0, min(maxEvents, 1024))
	count := 0

	parser := NewStreamEventParser(file, 0)
	err = parser.ParseEvents(func(ev Event) error {
		if maxEvents > 0 && count >= maxEvents {
			return io.EOF // Stop parsing
		}
		events = append(events, ev)
		count++
		return nil
	})

	if err == io.EOF {
		err = nil
	}

	return events, err
}

// min returns the minimum of two integers.
func min(a, b int) int {
	if a < b {
		return a
	}
	return b
}
