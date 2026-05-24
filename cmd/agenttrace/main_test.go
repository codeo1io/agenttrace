package main

import (
	"os"
	"path/filepath"
	"testing"
)

func TestHasPostPricingAction(t *testing.T) {
	tests := []struct {
		name       string
		path       string
		listModels bool
		testMatch  bool
		doctor     bool
		latest     bool
		compare    bool
		overview   bool
		waste      bool
		search     string
		want       bool
	}{
		{name: "update pricing alone exits", want: false},
		{name: "list models continues", listModels: true, want: true},
		{name: "test match continues", testMatch: true, want: true},
		{name: "doctor continues", doctor: true, want: true},
		{name: "overview continues", overview: true, want: true},
		{name: "latest continues", latest: true, want: true},
		{name: "compare continues", compare: true, want: true},
		{name: "waste continues", waste: true, want: true},
		{name: "search continues", search: "billing", want: true},
		{name: "blank search exits", search: "   ", want: false},
		{name: "path continues", path: "session.jsonl", want: true},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			got := hasPostPricingAction(tt.path, tt.listModels, tt.testMatch, tt.doctor, tt.latest, tt.compare, tt.overview, tt.waste, tt.search)
			if got != tt.want {
				t.Fatalf("hasPostPricingAction() = %v, want %v", got, tt.want)
			}
		})
	}
}

func TestLoadSessionsForSearchIncludesNestedDirs(t *testing.T) {
	dir := t.TempDir()
	nested := filepath.Join(dir, "2026", "05", "25")
	if err := os.MkdirAll(nested, 0755); err != nil {
		t.Fatal(err)
	}
	path := filepath.Join(nested, "session.jsonl")
	raw := `{"timestamp":"2026-05-25T00:00:00Z","type":"session_meta","payload":{"id":"search-nested","cwd":"/tmp/search-nested","model":"gpt-5.4"}}` + "\n" +
		`{"timestamp":"2026-05-25T00:00:01Z","type":"response_item","payload":{"type":"message","role":"user","content":[{"type":"input_text","text":"hello"}]}}` + "\n"
	if err := os.WriteFile(path, []byte(raw), 0644); err != nil {
		t.Fatal(err)
	}
	sessions := loadSessionsForSearch(dir)
	if len(sessions) != 1 {
		t.Fatalf("expected nested session to be loaded, got %d", len(sessions))
	}
	if sessions[0].Path != path {
		t.Fatalf("loaded wrong path: %s", sessions[0].Path)
	}
}
