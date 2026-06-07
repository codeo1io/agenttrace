// incremental_cache.go - Incremental cache updates
// Copyright 2026 agenttrace contributors. MIT License.

package engine

import (
	"encoding/json"
	"os"
	"sync"
	"time"
)

// IncrementalCache manages incremental cache updates.
type IncrementalCache struct {
	mu           sync.RWMutex
	cache        SessionCache
	dirtyEntries map[string]bool
	dirtyDirs    map[string]bool
	lastSave     time.Time
	saveInterval time.Duration
	autoSave     bool
}

// NewIncrementalCache creates a new incremental cache manager.
func NewIncrementalCache(saveInterval time.Duration, autoSave bool) *IncrementalCache {
	return &IncrementalCache{
		cache:        LoadSessionCache(),
		dirtyEntries: make(map[string]bool),
		dirtyDirs:    make(map[string]bool),
		saveInterval: saveInterval,
		autoSave:     autoSave,
	}
}

// GetSession gets a session (read-only).
func (ic *IncrementalCache) GetSession(path string) (Session, bool) {
	ic.mu.RLock()
	defer ic.mu.RUnlock()
	return CachedSession(path, ic.cache)
}

// SetSession sets a session (marks as dirty).
func (ic *IncrementalCache) SetSession(path string, session Session) {
	ic.mu.Lock()
	defer ic.mu.Unlock()

	entry := CacheEntry{
		ModTime: time.Now().UnixNano(),
		Size:    0,
		Session: session,
	}

	if ic.cache.Entries == nil {
		ic.cache.Entries = make(map[string]CacheEntry)
	}
	ic.cache.Entries[path] = entry
	ic.dirtyEntries[path] = true

	// Check auto-save
	if ic.autoSave {
		ic.checkAutoSave()
	}
}

// SetDirCache sets directory cache (marks as dirty).
func (ic *IncrementalCache) SetDirCache(dir string, entry DirCacheEntry) {
	ic.mu.Lock()
	defer ic.mu.Unlock()

	if ic.cache.Dirs == nil {
		ic.cache.Dirs = make(map[string]DirCacheEntry)
	}
	ic.cache.Dirs[dir] = entry
	ic.dirtyDirs[dir] = true

	// Check auto-save
	if ic.autoSave {
		ic.checkAutoSave()
	}
}

// GetDirCache gets directory cache.
func (ic *IncrementalCache) GetDirCache(dir string) (DirCacheEntry, bool) {
	ic.mu.RLock()
	defer ic.mu.RUnlock()

	entry, ok := ic.cache.Dirs[dir]
	return entry, ok
}

// HasDirtyEntries checks if there are dirty entries.
func (ic *IncrementalCache) HasDirtyEntries() bool {
	ic.mu.RLock()
	defer ic.mu.RUnlock()
	return len(ic.dirtyEntries) > 0 || len(ic.dirtyDirs) > 0
}

// DirtyCount returns the number of dirty entries.
func (ic *IncrementalCache) DirtyCount() int {
	ic.mu.RLock()
	defer ic.mu.RUnlock()
	return len(ic.dirtyEntries) + len(ic.dirtyDirs)
}

// SaveDirty saves dirty entries to disk.
func (ic *IncrementalCache) SaveDirty() error {
	ic.mu.Lock()
	defer ic.mu.Unlock()

	if len(ic.dirtyEntries) == 0 && len(ic.dirtyDirs) == 0 {
		return nil
	}

	// Save to disk
	err := SaveSessionCache(ic.cache)
	if err != nil {
		return err
	}

	// Clear dirty flags
	ic.dirtyEntries = make(map[string]bool)
	ic.dirtyDirs = make(map[string]bool)
	ic.lastSave = time.Now()

	return nil
}

// SaveAll saves all cache to disk.
func (ic *IncrementalCache) SaveAll() error {
	ic.mu.Lock()
	defer ic.mu.Unlock()

	err := SaveSessionCache(ic.cache)
	if err != nil {
		return err
	}

	ic.dirtyEntries = make(map[string]bool)
	ic.dirtyDirs = make(map[string]bool)
	ic.lastSave = time.Now()

	return nil
}

// checkAutoSave checks if auto-save is needed.
func (ic *IncrementalCache) checkAutoSave() {
	if time.Since(ic.lastSave) >= ic.saveInterval {
		go func() {
			ic.SaveDirty()
		}()
	}
}

// GetCache returns the underlying cache.
func (ic *IncrementalCache) GetCache() SessionCache {
	ic.mu.RLock()
	defer ic.mu.RUnlock()
	return ic.cache
}

// SetCache sets the underlying cache.
func (ic *IncrementalCache) SetCache(cache SessionCache) {
	ic.mu.Lock()
	defer ic.mu.Unlock()
	ic.cache = cache
}

// Clear clears the cache.
func (ic *IncrementalCache) Clear() {
	ic.mu.Lock()
	defer ic.mu.Unlock()

	ic.cache = emptySessionCache()
	ic.dirtyEntries = make(map[string]bool)
	ic.dirtyDirs = make(map[string]bool)
}

// CacheStats holds cache statistics.
type CacheStats struct {
	TotalEntries int
	DirtyEntries int
	TotalDirs    int
	DirtyDirs    int
	LastSave     time.Time
	SaveInterval time.Duration
}

// GetStats returns cache statistics.
func (ic *IncrementalCache) GetStats() CacheStats {
	ic.mu.RLock()
	defer ic.mu.RUnlock()

	return CacheStats{
		TotalEntries: len(ic.cache.Entries),
		DirtyEntries: len(ic.dirtyEntries),
		TotalDirs:    len(ic.cache.Dirs),
		DirtyDirs:    len(ic.dirtyDirs),
		LastSave:     ic.lastSave,
		SaveInterval: ic.saveInterval,
	}
}

// BatchUpdate performs batch updates.
func (ic *IncrementalCache) BatchUpdate(updates []CacheUpdate) error {
	ic.mu.Lock()
	defer ic.mu.Unlock()

	for _, update := range updates {
		switch update.Type {
		case "session":
			if ic.cache.Entries == nil {
				ic.cache.Entries = make(map[string]CacheEntry)
			}
			ic.cache.Entries[update.Path] = update.Entry
			ic.dirtyEntries[update.Path] = true

		case "dir":
			if ic.cache.Dirs == nil {
				ic.cache.Dirs = make(map[string]DirCacheEntry)
			}
			ic.cache.Dirs[update.Path] = update.DirEntry
			ic.dirtyDirs[update.Path] = true
		}
	}

	// Batch save
	if ic.autoSave && len(updates) > 0 {
		go func() {
			ic.SaveDirty()
		}()
	}

	return nil
}

// CacheUpdate represents a cache update.
type CacheUpdate struct {
	Type     string // "session" or "dir"
	Path     string
	Entry    CacheEntry
	DirEntry DirCacheEntry
}

// MergeCaches merges two caches.
func MergeCaches(base, overlay SessionCache) SessionCache {
	merged := SessionCache{
		Entries: make(map[string]CacheEntry, len(base.Entries)+len(overlay.Entries)),
		Dirs:    make(map[string]DirCacheEntry, len(base.Dirs)+len(overlay.Dirs)),
	}

	// Copy base cache
	for k, v := range base.Entries {
		merged.Entries[k] = v
	}
	for k, v := range base.Dirs {
		merged.Dirs[k] = v
	}

	// Overlay (overlay takes precedence)
	for k, v := range overlay.Entries {
		merged.Entries[k] = v
	}
	for k, v := range overlay.Dirs {
		merged.Dirs[k] = v
	}

	return merged
}

// DiffCaches calculates the difference between two caches.
func DiffCaches(old, new SessionCache) CacheDiff {
	diff := CacheDiff{
		AddedSessions:   make([]string, 0),
		RemovedSessions: make([]string, 0),
		UpdatedSessions: make([]string, 0),
		AddedDirs:       make([]string, 0),
		RemovedDirs:     make([]string, 0),
		UpdatedDirs:     make([]string, 0),
	}

	// Check session differences
	for path := range new.Entries {
		if _, ok := old.Entries[path]; !ok {
			diff.AddedSessions = append(diff.AddedSessions, path)
		}
	}
	for path := range old.Entries {
		if _, ok := new.Entries[path]; !ok {
			diff.RemovedSessions = append(diff.RemovedSessions, path)
		}
	}

	// Check directory differences
	for path := range new.Dirs {
		if _, ok := old.Dirs[path]; !ok {
			diff.AddedDirs = append(diff.AddedDirs, path)
		}
	}
	for path := range old.Dirs {
		if _, ok := new.Dirs[path]; !ok {
			diff.RemovedDirs = append(diff.RemovedDirs, path)
		}
	}

	return diff
}

// CacheDiff represents cache differences.
type CacheDiff struct {
	AddedSessions   []string
	RemovedSessions []string
	UpdatedSessions []string
	AddedDirs       []string
	RemovedDirs     []string
	UpdatedDirs     []string
}

// HasChanges checks if there are any changes.
func (cd CacheDiff) HasChanges() bool {
	return len(cd.AddedSessions) > 0 ||
		len(cd.RemovedSessions) > 0 ||
		len(cd.UpdatedSessions) > 0 ||
		len(cd.AddedDirs) > 0 ||
		len(cd.RemovedDirs) > 0 ||
		len(cd.UpdatedDirs) > 0
}

// SerializeCache serializes cache to JSON.
func SerializeCache(cache SessionCache) ([]byte, error) {
	return json.Marshal(cache)
}

// DeserializeCache deserializes cache from JSON.
func DeserializeCache(data []byte) (SessionCache, error) {
	var cache SessionCache
	err := json.Unmarshal(data, &cache)
	return cache, err
}

// SaveCacheToFile saves cache to a file.
func SaveCacheToFile(cache SessionCache, path string) error {
	data, err := SerializeCache(cache)
	if err != nil {
		return err
	}
	return os.WriteFile(path, data, 0644)
}

// LoadCacheFromFile loads cache from a file.
func LoadCacheFromFile(path string) (SessionCache, error) {
	data, err := os.ReadFile(path)
	if err != nil {
		return emptySessionCache(), err
	}
	return DeserializeCache(data)
}
