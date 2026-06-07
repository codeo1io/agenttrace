// parallel.go - Concurrent session loading optimizations
// Copyright 2026 agenttrace contributors. MIT License.

package engine

import (
	"runtime"
	"sort"
	"sync"
)

// LoadSessionsParallel loads multiple session files concurrently.
// Uses worker pool pattern to avoid resource contention from excessive concurrency.
func LoadSessionsParallel(paths []string) []Session {
	if len(paths) == 0 {
		return nil
	}

	// Determine concurrency based on CPU cores and file count
	maxWorkers := runtime.NumCPU()
	if maxWorkers > 8 {
		maxWorkers = 8 // Cap max concurrency to avoid resource contention
	}
	if len(paths) < maxWorkers {
		maxWorkers = len(paths)
	}

	type result struct {
		session Session
		index   int
		ok      bool
	}

	results := make([]result, len(paths))
	jobs := make(chan int, len(paths))
	var wg sync.WaitGroup

	// Start workers
	for w := 0; w < maxWorkers; w++ {
		wg.Add(1)
		go func() {
			defer wg.Done()
			for idx := range jobs {
				path := paths[idx]
				s, err := LoadSession(path)
				if err != nil {
					results[idx] = result{index: idx, ok: false}
					continue
				}
				results[idx] = result{session: *s, index: idx, ok: true}
			}
		}()
	}

	// Dispatch jobs
	for i := range paths {
		jobs <- i
	}
	close(jobs)

	// Wait for completion
	wg.Wait()

	// Collect valid results
	var sessions []Session
	for _, r := range results {
		if r.ok {
			sessions = append(sessions, r.session)
		}
	}

	// Sort by timestamp
	sort.Slice(sessions, func(i, j int) bool {
		return sessions[i].Metrics.SessionStart > sessions[j].Metrics.SessionStart
	})

	return sessions
}

// FindSessionFilesParallel discovers session files concurrently.
// Significantly improves file discovery speed for multiple directories.
func FindSessionFilesParallel(dirs []string) []string {
	if len(dirs) == 0 {
		return nil
	}
	if len(dirs) == 1 {
		return FindSessionFiles(dirs[0])
	}

	maxWorkers := runtime.NumCPU()
	if maxWorkers > 4 {
		maxWorkers = 4
	}
	if len(dirs) < maxWorkers {
		maxWorkers = len(dirs)
	}

	type result struct {
		files []string
		index int
	}

	results := make([]result, len(dirs))
	jobs := make(chan int, len(dirs))
	var wg sync.WaitGroup

	// Start workers
	for w := 0; w < maxWorkers; w++ {
		wg.Add(1)
		go func() {
			defer wg.Done()
			for idx := range jobs {
				dir := dirs[idx]
				files := FindSessionFiles(dir)
				results[idx] = result{files: files, index: idx}
			}
		}()
	}

	// Dispatch jobs
	for i := range dirs {
		jobs <- i
	}
	close(jobs)

	// Wait for completion
	wg.Wait()

	// Collect all files
	var allFiles []string
	seen := make(map[string]bool)
	for _, r := range results {
		for _, f := range r.files {
			if !seen[f] {
				seen[f] = true
				allFiles = append(allFiles, f)
			}
		}
	}

	return allFiles
}
