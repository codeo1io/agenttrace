// batch.go - Batch processing optimizations
// Copyright 2026 agenttrace contributors. MIT License.

package engine

import (
	"sort"
	"sync"
)

// BatchProcessor processes sessions in batches for optimized bulk operations.
type BatchProcessor struct {
	mu       sync.RWMutex
	sessions []Session
	errors   []error
}

// NewBatchProcessor creates a new batch processor.
func NewBatchProcessor() *BatchProcessor {
	return &BatchProcessor{
		sessions: make([]Session, 0, 256),
		errors:   make([]error, 0, 16),
	}
}

// Process processes sessions in batches.
func (bp *BatchProcessor) Process(paths []string, workers int) []Session {
	if len(paths) == 0 {
		return nil
	}

	if workers <= 0 {
		workers = 4
	}

	jobs := make(chan string, len(paths))
	results := make(chan struct {
		session Session
		err     error
	}, len(paths))

	// Start workers
	var wg sync.WaitGroup
	for i := 0; i < workers; i++ {
		wg.Add(1)
		go func() {
			defer wg.Done()
			for path := range jobs {
				s, err := LoadSession(path)
				if err != nil {
					results <- struct {
						session Session
						err     error
					}{err: err}
					continue
				}
				results <- struct {
					session Session
					err     error
				}{session: *s}
			}
		}()
	}

	// Dispatch jobs
	go func() {
		for _, path := range paths {
			jobs <- path
		}
		close(jobs)
	}()

	// Collect results
	go func() {
		wg.Wait()
		close(results)
	}()

	// Process results
	for r := range results {
		if r.err != nil {
			bp.errors = append(bp.errors, r.err)
			continue
		}
		bp.sessions = append(bp.sessions, r.session)
	}

	// Sort by timestamp
	sort.Slice(bp.sessions, func(i, j int) bool {
		return bp.sessions[i].Metrics.SessionStart > bp.sessions[j].Metrics.SessionStart
	})

	return bp.sessions
}

// Stats holds processing statistics.
type Stats struct {
	Total   int
	Success int
	Failed  int
	Errors  []error
}

// ProcessWithStats processes sessions with statistics tracking.
func ProcessWithStats(paths []string, workers int) ([]Session, Stats) {
	bp := NewBatchProcessor()
	sessions := bp.Process(paths, workers)

	stats := Stats{
		Total:   len(paths),
		Success: len(sessions),
		Failed:  len(bp.errors),
		Errors:  bp.errors,
	}

	return sessions, stats
}

// FilterByHealth filters sessions by minimum health score.
func FilterByHealth(sessions []Session, minHealth int) []Session {
	var filtered []Session
	for _, s := range sessions {
		if s.Health >= minHealth {
			filtered = append(filtered, s)
		}
	}
	return filtered
}

// FilterBySource filters sessions by source tool.
func FilterBySource(sessions []Session, source string) []Session {
	var filtered []Session
	for _, s := range sessions {
		if s.Metrics.SourceTool == source {
			filtered = append(filtered, s)
		}
	}
	return filtered
}

// FilterByModel filters sessions by model.
func FilterByModel(sessions []Session, model string) []Session {
	var filtered []Session
	for _, s := range sessions {
		if s.Metrics.ModelUsed == model {
			filtered = append(filtered, s)
		}
	}
	return filtered
}

// AggregateByAgent aggregates statistics by agent source.
func AggregateByAgent(sessions []Session) map[string]AgentOverview {
	agents := make(map[string]AgentOverview)
	for _, s := range sessions {
		agent := s.Metrics.SourceTool
		ao := agents[agent]
		ao.Sessions++
		ao.Cost += s.Metrics.CostEstimated
		agents[agent] = ao
	}
	return agents
}

// AggregateByModel aggregates statistics by model.
func AggregateByModel(sessions []Session) map[string]ModelOverview {
	models := make(map[string]ModelOverview)
	for _, s := range sessions {
		model := s.Metrics.ModelUsed
		if model == "" {
			model = "unknown"
		}
		mo := models[model]
		mo.Sessions++
		mo.Cost += s.Metrics.CostEstimated
		models[model] = mo
	}
	return models
}
