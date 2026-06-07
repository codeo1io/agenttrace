// memory.go - Memory optimization utilities
// Copyright 2026 agenttrace contributors. MIT License.

package engine

import (
	"runtime"
	"sync"
)

// EventPool is an object pool for Event structs to reduce GC pressure.
var EventPool = sync.Pool{
	New: func() interface{} {
		return &Event{}
	},
}

// ToolCallPool is an object pool for ToolCall structs.
var ToolCallPool = sync.Pool{
	New: func() interface{} {
		return &ToolCall{}
	},
}

// GetEvent gets an Event from the pool.
func GetEvent() *Event {
	return EventPool.Get().(*Event)
}

// PutEvent returns an Event to the pool.
func PutEvent(e *Event) {
	// Reset fields
	e.Role = ""
	e.Content = ""
	e.Timestamp = ""
	e.Reasoning = ""
	e.Redacted = false
	e.CWD = ""
	e.ToolCalls = e.ToolCalls[:0]
	e.ToolCallID = ""
	e.IsError = false
	e.Usage = nil
	e.ModelUsed = ""
	e.SourceTool = ""
	EventPool.Put(e)
}

// GetToolCall gets a ToolCall from the pool.
func GetToolCall() *ToolCall {
	return ToolCallPool.Get().(*ToolCall)
}

// PutToolCall returns a ToolCall to the pool.
func PutToolCall(tc *ToolCall) {
	tc.ID = ""
	tc.Name = ""
	tc.Args = ""
	ToolCallPool.Put(tc)
}

// EventsSlice is a pre-allocated slice for events.
type EventsSlice struct {
	events []Event
}

// NewEventsSlice creates a new EventsSlice with pre-allocated capacity.
func NewEventsSlice(capacity int) *EventsSlice {
	if capacity <= 0 {
		capacity = 64
	}
	return &EventsSlice{
		events: make([]Event, 0, capacity),
	}
}

// Append adds an event to the slice.
func (es *EventsSlice) Append(e Event) {
	es.events = append(es.events, e)
}

// Events returns the underlying event slice.
func (es *EventsSlice) Events() []Event {
	return es.events
}

// Len returns the number of events.
func (es *EventsSlice) Len() int {
	return len(es.events)
}

// Compact returns a compacted copy of the events slice.
func (es *EventsSlice) Compact() []Event {
	if len(es.events) == cap(es.events) {
		return es.events
	}
	compacted := make([]Event, len(es.events))
	copy(compacted, es.events)
	return compacted
}

// MemoryStats holds memory statistics.
type MemoryStats struct {
	Alloc      uint64 // Current allocated memory
	TotalAlloc uint64 // Cumulative allocated memory
	Sys        uint64 // System memory
	NumGC      uint32 // GC count
}

// GetMemoryStats returns current memory statistics.
func GetMemoryStats() MemoryStats {
	var m runtime.MemStats
	runtime.ReadMemStats(&m)
	return MemoryStats{
		Alloc:      m.Alloc,
		TotalAlloc: m.TotalAlloc,
		Sys:        m.Sys,
		NumGC:      m.NumGC,
	}
}

// ForceGC forces garbage collection (for testing/debugging only).
func ForceGC() {
	runtime.GC()
}

// OptimizeMemory optimizes memory usage.
func OptimizeMemory() {
	// Trigger GC
	runtime.GC()
	// Release idle memory
	debug := false
	if debug {
		runtime.MemProfile(nil, true)
	}
}
