// incremental_cache_bench_test.go - Incremental cache benchmark tests
// Copyright 2026 agenttrace contributors. MIT License.

package engine

import (
	"fmt"
	"testing"
	"time"
)

// BenchmarkIncrementalCacheSet tests incremental cache set performance
func BenchmarkIncrementalCacheSet(b *testing.B) {
	ic := NewIncrementalCache(time.Minute, false)

	b.ResetTimer()

	for i := 0; i < b.N; i++ {
		path := fmt.Sprintf("/tmp/session-%d.json", i%100)
		session := Session{
			Name: fmt.Sprintf("session-%d", i),
			Path: path,
		}
		ic.SetSession(path, session)
	}
}

// BenchmarkIncrementalCacheGet tests incremental cache get performance
func BenchmarkIncrementalCacheGet(b *testing.B) {
	ic := NewIncrementalCache(time.Minute, false)

	// 预填充缓存
	for i := 0; i < 100; i++ {
		path := fmt.Sprintf("/tmp/session-%d.json", i)
		session := Session{
			Name: fmt.Sprintf("session-%d", i),
			Path: path,
		}
		ic.SetSession(path, session)
	}

	b.ResetTimer()

	for i := 0; i < b.N; i++ {
		path := fmt.Sprintf("/tmp/session-%d.json", i%100)
		ic.GetSession(path)
	}
}

// BenchmarkIncrementalCacheSave tests incremental cache save performance
func BenchmarkIncrementalCacheSave(b *testing.B) {
	ic := NewIncrementalCache(time.Minute, false)

	// 预填充缓存
	for i := 0; i < 100; i++ {
		path := fmt.Sprintf("/tmp/session-%d.json", i)
		session := Session{
			Name: fmt.Sprintf("session-%d", i),
			Path: path,
		}
		ic.SetSession(path, session)
	}

	b.ResetTimer()

	for i := 0; i < b.N; i++ {
		ic.SaveDirty()
	}
}

// BenchmarkBatchUpdate tests batch update performance
func BenchmarkBatchUpdate(b *testing.B) {
	ic := NewIncrementalCache(time.Minute, false)

	b.ResetTimer()

	for i := 0; i < b.N; i++ {
		updates := make([]CacheUpdate, 10)
		for j := 0; j < 10; j++ {
			path := fmt.Sprintf("/tmp/session-%d.json", i*10+j)
			updates[j] = CacheUpdate{
				Type: "session",
				Path: path,
				Entry: CacheEntry{
					ModTime: time.Now().UnixNano(),
					Session: Session{
						Name: fmt.Sprintf("session-%d", i*10+j),
						Path: path,
					},
				},
			}
		}
		ic.BatchUpdate(updates)
	}
}

// BenchmarkMergeCaches tests cache merge performance
func BenchmarkMergeCaches(b *testing.B) {
	base := SessionCache{
		Entries: make(map[string]CacheEntry, 100),
		Dirs:    make(map[string]DirCacheEntry, 10),
	}

	for i := 0; i < 100; i++ {
		path := fmt.Sprintf("/tmp/session-%d.json", i)
		base.Entries[path] = CacheEntry{
			ModTime: time.Now().UnixNano(),
			Session: Session{
				Name: fmt.Sprintf("session-%d", i),
				Path: path,
			},
		}
	}

	overlay := SessionCache{
		Entries: make(map[string]CacheEntry, 20),
		Dirs:    make(map[string]DirCacheEntry, 5),
	}

	for i := 0; i < 20; i++ {
		path := fmt.Sprintf("/tmp/new-session-%d.json", i)
		overlay.Entries[path] = CacheEntry{
			ModTime: time.Now().UnixNano(),
			Session: Session{
				Name: fmt.Sprintf("new-session-%d", i),
				Path: path,
			},
		}
	}

	b.ResetTimer()

	for i := 0; i < b.N; i++ {
		MergeCaches(base, overlay)
	}
}

// BenchmarkDiffCaches tests cache diff calculation performance
func BenchmarkDiffCaches(b *testing.B) {
	old := SessionCache{
		Entries: make(map[string]CacheEntry, 100),
		Dirs:    make(map[string]DirCacheEntry, 10),
	}

	for i := 0; i < 100; i++ {
		path := fmt.Sprintf("/tmp/session-%d.json", i)
		old.Entries[path] = CacheEntry{
			ModTime: time.Now().UnixNano(),
			Session: Session{
				Name: fmt.Sprintf("session-%d", i),
				Path: path,
			},
		}
	}

	new := SessionCache{
		Entries: make(map[string]CacheEntry, 110),
		Dirs:    make(map[string]DirCacheEntry, 10),
	}

	// 复制旧的
	for k, v := range old.Entries {
		new.Entries[k] = v
	}

	// 添加新的
	for i := 100; i < 110; i++ {
		path := fmt.Sprintf("/tmp/session-%d.json", i)
		new.Entries[path] = CacheEntry{
			ModTime: time.Now().UnixNano(),
			Session: Session{
				Name: fmt.Sprintf("session-%d", i),
				Path: path,
			},
		}
	}

	b.ResetTimer()

	for i := 0; i < b.N; i++ {
		DiffCaches(old, new)
	}
}
