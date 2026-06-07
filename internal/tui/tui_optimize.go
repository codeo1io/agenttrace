// tui_optimize.go - TUI rendering optimizations
// Copyright 2026 agenttrace contributors. MIT License.

package tui

import (
	"strings"
	"sync"
)

// RenderCache caches rendered content.
type RenderCache struct {
	mu      sync.RWMutex
	cache   map[string]cacheEntry
	maxSize int
}

type cacheEntry struct {
	content string
	width   int
	height  int
}

// NewRenderCache creates a new render cache.
func NewRenderCache(maxSize int) *RenderCache {
	if maxSize <= 0 {
		maxSize = 100
	}
	return &RenderCache{
		cache:   make(map[string]cacheEntry, maxSize),
		maxSize: maxSize,
	}
}

// Get gets cached content.
func (rc *RenderCache) Get(key string, width, height int) (string, bool) {
	rc.mu.RLock()
	defer rc.mu.RUnlock()

	entry, ok := rc.cache[key]
	if !ok {
		return "", false
	}

	// Check size match
	if entry.width != width || entry.height != height {
		return "", false
	}

	return entry.content, true
}

// Set sets cached content.
func (rc *RenderCache) Set(key string, content string, width, height int) {
	rc.mu.Lock()
	defer rc.mu.Unlock()

	// Evict old entries if cache is full
	if len(rc.cache) >= rc.maxSize {
		rc.evictOldest()
	}

	rc.cache[key] = cacheEntry{
		content: content,
		width:   width,
		height:  height,
	}
}

// evictOldest evicts oldest entries.
func (rc *RenderCache) evictOldest() {
	// Simple implementation: clear half the cache
	count := 0
	for key := range rc.cache {
		delete(rc.cache, key)
		count++
		if count >= rc.maxSize/2 {
			break
		}
	}
}

// Clear clears the cache.
func (rc *RenderCache) Clear() {
	rc.mu.Lock()
	defer rc.mu.Unlock()
	rc.cache = make(map[string]cacheEntry, rc.maxSize)
}

// DirtyRegion tracks dirty regions.
type DirtyRegion struct {
	mu    sync.RWMutex
	dirty map[string]bool
}

// NewDirtyRegion creates a new dirty region tracker.
func NewDirtyRegion() *DirtyRegion {
	return &DirtyRegion{
		dirty: make(map[string]bool),
	}
}

// MarkDirty marks a region as dirty.
func (dr *DirtyRegion) MarkDirty(name string) {
	dr.mu.Lock()
	defer dr.mu.Unlock()
	dr.dirty[name] = true
}

// IsDirty checks if a region is dirty.
func (dr *DirtyRegion) IsDirty(name string) bool {
	dr.mu.RLock()
	defer dr.mu.RUnlock()
	return dr.dirty[name]
}

// ClearDirty clears dirty flag for a region.
func (dr *DirtyRegion) ClearDirty(name string) {
	dr.mu.Lock()
	defer dr.mu.Unlock()
	delete(dr.dirty, name)
}

// ClearAll clears all dirty flags.
func (dr *DirtyRegion) ClearAll() {
	dr.mu.Lock()
	defer dr.mu.Unlock()
	dr.dirty = make(map[string]bool)
}

// HasDirty checks if there are any dirty regions.
func (dr *DirtyRegion) HasDirty() bool {
	dr.mu.RLock()
	defer dr.mu.RUnlock()
	return len(dr.dirty) > 0
}

// VirtualList implements virtual scrolling for large lists.
type VirtualList struct {
	items      []string
	itemHeight int
	viewHeight int
	scrollY    int
	cache      *RenderCache
}

// NewVirtualList creates a new virtual list.
func NewVirtualList(itemHeight, viewHeight int) *VirtualList {
	return &VirtualList{
		itemHeight: itemHeight,
		viewHeight: viewHeight,
		cache:      NewRenderCache(50),
	}
}

// SetItems sets list items.
func (vl *VirtualList) SetItems(items []string) {
	vl.items = items
	vl.cache.Clear()
}

// ScrollTo scrolls to a position.
func (vl *VirtualList) ScrollTo(y int) {
	vl.scrollY = clamp(y, 0, vl.maxScroll())
}

// ScrollUp scrolls up.
func (vl *VirtualList) ScrollUp(lines int) {
	vl.ScrollTo(vl.scrollY - lines)
}

// ScrollDown scrolls down.
func (vl *VirtualList) ScrollDown(lines int) {
	vl.ScrollTo(vl.scrollY + lines)
}

// maxScroll returns maximum scroll position.
func (vl *VirtualList) maxScroll() int {
	totalHeight := len(vl.items) * vl.itemHeight
	if totalHeight <= vl.viewHeight {
		return 0
	}
	return totalHeight - vl.viewHeight
}

// VisibleRange returns the visible range.
func (vl *VirtualList) VisibleRange() (start, end int) {
	start = vl.scrollY / vl.itemHeight
	end = start + (vl.viewHeight / vl.itemHeight) + 1
	if end > len(vl.items) {
		end = len(vl.items)
	}
	return start, end
}

// Render renders the visible portion.
func (vl *VirtualList) Render() string {
	start, end := vl.VisibleRange()

	var buf strings.Builder
	for i := start; i < end; i++ {
		buf.WriteString(vl.items[i])
		buf.WriteString("\n")
	}

	return buf.String()
}

// clamp restricts value to range.
func clamp(value, min, max int) int {
	if value < min {
		return min
	}
	if value > max {
		return max
	}
	return value
}

// RenderOptimizer optimizes rendering.
type RenderOptimizer struct {
	cache       *RenderCache
	dirty       *DirtyRegion
	renderCount int
}

// NewRenderOptimizer creates a new render optimizer.
func NewRenderOptimizer() *RenderOptimizer {
	return &RenderOptimizer{
		cache: NewRenderCache(100),
		dirty: NewDirtyRegion(),
	}
}

// ShouldRender checks if rendering is needed.
func (ro *RenderOptimizer) ShouldRender(region string) bool {
	return ro.dirty.IsDirty(region)
}

// MarkRendered marks a region as rendered.
func (ro *RenderOptimizer) MarkRendered(region string) {
	ro.dirty.ClearDirty(region)
	ro.renderCount++
}

// GetCached gets cached render result.
func (ro *RenderOptimizer) GetCached(key string, width, height int) (string, bool) {
	return ro.cache.Get(key, width, height)
}

// SetCache sets cache.
func (ro *RenderOptimizer) SetCache(key string, content string, width, height int) {
	ro.cache.Set(key, content, width, height)
}

// GetRenderCount returns render count.
func (ro *RenderOptimizer) GetRenderCount() int {
	return ro.renderCount
}

// ResetRenderCount resets render count.
func (ro *RenderOptimizer) ResetRenderCount() {
	ro.renderCount = 0
}
