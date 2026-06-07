// tui_optimize_bench_test.go - TUI 优化基准测试
// Copyright 2026 agenttrace contributors. MIT License.

package tui

import (
	"fmt"
	"testing"
)

// BenchmarkRenderCache 渲染缓存性能
func BenchmarkRenderCache(b *testing.B) {
	cache := NewRenderCache(100)

	b.ResetTimer()

	for i := 0; i < b.N; i++ {
		key := fmt.Sprintf("key-%d", i%50)
		content := fmt.Sprintf("content-%d", i)

		// 设置缓存
		cache.Set(key, content, 80, 24)

		// 获取缓存
		cache.Get(key, 80, 24)
	}
}

// BenchmarkRenderCacheMiss 缓存未命中性能
func BenchmarkRenderCacheMiss(b *testing.B) {
	cache := NewRenderCache(100)

	// 预填充缓存
	for i := 0; i < 50; i++ {
		cache.Set(fmt.Sprintf("key-%d", i), "content", 80, 24)
	}

	b.ResetTimer()

	for i := 0; i < b.N; i++ {
		key := fmt.Sprintf("miss-%d", i)
		cache.Get(key, 80, 24)
	}
}

// BenchmarkDirtyRegion 脏区域检测性能
func BenchmarkDirtyRegion(b *testing.B) {
	dr := NewDirtyRegion()

	b.ResetTimer()

	for i := 0; i < b.N; i++ {
		name := fmt.Sprintf("region-%d", i%10)

		// 标记脏
		dr.MarkDirty(name)

		// 检查脏
		dr.IsDirty(name)

		// 清除脏
		dr.ClearDirty(name)
	}
}

// BenchmarkVirtualList 虚拟列表性能
func BenchmarkVirtualList(b *testing.B) {
	vl := NewVirtualList(1, 24)

	// 创建测试数据
	items := make([]string, 10000)
	for i := range items {
		items[i] = fmt.Sprintf("Item %d", i)
	}
	vl.SetItems(items)

	b.ResetTimer()

	for i := 0; i < b.N; i++ {
		// 滚动
		vl.ScrollTo(i % 9976)

		// 渲染
		vl.Render()
	}
}

// BenchmarkVirtualListScroll 虚拟列表滚动性能
func BenchmarkVirtualListScroll(b *testing.B) {
	vl := NewVirtualList(1, 24)

	// 创建测试数据
	items := make([]string, 10000)
	for i := range items {
		items[i] = fmt.Sprintf("Item %d", i)
	}
	vl.SetItems(items)

	b.ResetTimer()

	for i := 0; i < b.N; i++ {
		if i%2 == 0 {
			vl.ScrollDown(1)
		} else {
			vl.ScrollUp(1)
		}
	}
}

// BenchmarkRenderOptimizer 渲染优化器性能
func BenchmarkRenderOptimizer(b *testing.B) {
	ro := NewRenderOptimizer()

	b.ResetTimer()

	for i := 0; i < b.N; i++ {
		region := fmt.Sprintf("region-%d", i%5)

		// 检查是否需要渲染
		if ro.ShouldRender(region) {
			// 模拟渲染
			content := fmt.Sprintf("rendered-%d", i)
			ro.SetCache(region, content, 80, 24)
			ro.MarkRendered(region)
		}

		// 标记脏
		ro.dirty.MarkDirty(region)
	}
}
