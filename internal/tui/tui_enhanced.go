// tui_enhanced.go - Enhanced TUI features
// Adds mouse support, preview panel, action menu, and Vim navigation
// Inspired by gh-dash, lazygit, k9s, btop

package tui

import (
	"fmt"
	"strings"

	tea "github.com/charmbracelet/bubbletea"
	"github.com/charmbracelet/lipgloss"
	"github.com/luoyuctl/agenttrace/internal/engine"
)

// ── Mouse Support ──

// EnableMouse returns tea commands to enable mouse tracking
func EnableMouse() tea.Cmd {
	return tea.EnableMouseAllMotion
}

// HandleMouseEvent processes mouse events
func (m *Model) HandleMouseEvent(msg tea.MouseMsg) (tea.Model, tea.Cmd) {
	switch msg.Type {
	case tea.MouseWheelUp:
		if m.view == viewList {
			m.table.MoveUp(1)
		} else if m.view == viewDetail || m.view == viewDiagnostics {
			m.viewport.LineUp(1)
		}
	case tea.MouseWheelDown:
		if m.view == viewList {
			m.table.MoveDown(1)
		} else if m.view == viewDetail || m.view == viewDiagnostics {
			m.viewport.LineDown(1)
		}
	case tea.MouseLeft:
		m.handleMouseClick(msg.X, msg.Y)
	}
	return m, nil
}

// handleMouseClick processes mouse click events
func (m *Model) handleMouseClick(x, y int) {
	// Tab bar click detection
	if y == 1 {
		tabWidth := m.width / 5
		switch x / tabWidth {
		case 0:
			m.view = viewOverview
		case 1:
			m.view = viewList
		case 2:
			if len(m.filteredIndices) > 0 {
				m.view = viewDetail
				m.openDetail()
			}
		case 3:
			m.openDiagnostics()
		case 4:
			m.openDiff()
		}
	}
}

// ── Preview Panel ──

// RenderSessionPreview renders a preview panel for the selected session
func (m Model) RenderSessionPreview(width int) string {
	if len(m.filteredIndices) == 0 {
		return m.renderEmptyPreview(width)
	}

	idx := m.table.Cursor()
	if idx >= len(m.filteredIndices) {
		return m.renderEmptyPreview(width)
	}

	sessionIdx := m.filteredIndices[idx]
	if sessionIdx >= len(m.sessions) {
		return m.renderEmptyPreview(width)
	}

	s := m.sessions[sessionIdx]
	return m.renderPreviewContent(s, width)
}

// renderEmptyPreview renders an empty preview panel
func (m Model) renderEmptyPreview(width int) string {
	style := lipgloss.NewStyle().
		Width(width).
		Height(20).
		Border(lipgloss.RoundedBorder()).
		BorderForeground(lipgloss.Color("240")).
		Padding(1, 2)

	content := lipgloss.Place(
		width-4, 18,
		lipgloss.Center, lipgloss.Center,
		dimStyle.Render("Select a session to preview"),
	)
	return style.Render(content)
}

// renderPreviewContent renders the preview content for a session
func (m Model) renderPreviewContent(s engine.Session, width int) string {
	borderColor := healthColorPreview(s.Health).GetForeground()
	style := lipgloss.NewStyle().
		Width(width).
		Border(lipgloss.RoundedBorder()).
		BorderForeground(borderColor).
		Padding(1, 2)

	innerW := width - 6

	// Header
	header := boldStyle.Render(truncate(s.Name, innerW))

	// Health bar
	healthBar := m.renderMiniHealthBar(s.Health, innerW)

	// Metrics
	metrics := m.renderPreviewMetrics(s, innerW)

	// Anomalies
	anomalies := m.renderPreviewAnomalies(s, innerW)

	// Recent tools
	tools := m.renderPreviewTools(s, innerW)

	content := lipgloss.JoinVertical(lipgloss.Left,
		header,
		"",
		healthBar,
		"",
		metrics,
		"",
		anomalies,
		"",
		tools,
	)

	return style.Render(content)
}

// renderMiniHealthBar renders a compact health bar
func (m Model) renderMiniHealthBar(health int, width int) string {
	barWidth := width - 10
	filled := int(float64(barWidth) * float64(health) / 100.0)
	if filled > barWidth {
		filled = barWidth
	}

	bar := strings.Repeat("█", filled) + strings.Repeat("░", barWidth-filled)
	color := healthColor(health)

	return fmt.Sprintf("Health: %s %d%%",
		color.Render(bar),
		health,
	)
}

// renderPreviewMetrics renders key metrics for preview
func (m Model) renderPreviewMetrics(s engine.Session, width int) string {
	metrics := []string{
		fmt.Sprintf("Model: %s", cyanStyle.Render(s.Metrics.ModelUsed)),
		fmt.Sprintf("Source: %s", cyanStyle.Render(s.Metrics.SourceTool)),
		fmt.Sprintf("Tokens: %s", compactInt(s.Metrics.TokensInput+s.Metrics.TokensOutput)),
		fmt.Sprintf("Cost: %s", money2(s.Metrics.CostEstimated)),
		fmt.Sprintf("Turns: %d", s.Metrics.AssistantTurns),
		fmt.Sprintf("Tools: %d/%d", s.Metrics.ToolCallsOK, s.Metrics.ToolCallsTotal),
	}

	var lines []string
	for _, m := range metrics {
		lines = append(lines, truncate(m, width))
	}

	return strings.Join(lines, "\n")
}

// renderPreviewAnomalies renders anomaly list for preview
func (m Model) renderPreviewAnomalies(s engine.Session, width int) string {
	if len(s.Anomalies) == 0 {
		return dimStyle.Render("No anomalies detected")
	}

	title := boldStyle.Render("Anomalies:")
	var items []string
	for i, a := range s.Anomalies {
		if i >= 3 {
			items = append(items, dimStyle.Render(fmt.Sprintf("  ...and %d more", len(s.Anomalies)-3)))
			break
		}
		emoji := "🔴"
		if a.Severity == "medium" {
			emoji = "🟡"
		} else if a.Severity == "low" {
			emoji = "🟢"
		}
		items = append(items, fmt.Sprintf("  %s %s", emoji, truncate(a.Type, width-6)))
	}

	return title + "\n" + strings.Join(items, "\n")
}

// renderPreviewTools renders recent tool calls for preview
func (m Model) renderPreviewTools(s engine.Session, width int) string {
	if len(s.Metrics.ToolUsage) == 0 {
		return dimStyle.Render("No tool calls")
	}

	title := boldStyle.Render("Top Tools:")
	var items []string
	count := 0
	for tool, usage := range s.Metrics.ToolUsage {
		if count >= 3 {
			break
		}
		items = append(items, fmt.Sprintf("  • %s (%d)", truncate(tool, width-10), usage))
		count++
	}

	return title + "\n" + strings.Join(items, "\n")
}

// ── Action Menu ──

// ActionMenu represents a quick action menu
type ActionMenu struct {
	items   []ActionItem
	cursor  int
	visible bool
}

// ActionItem represents a single action in the menu
type ActionItem struct {
	Key    string
	Label  string
	Desc   string
	Action string
}

// ShowActionMenu shows the quick action menu
func (m *Model) ShowActionMenu() {
	m.actionMenu = ActionMenu{
		items: []ActionItem{
			{Key: "r", Label: "Reload", Desc: "Reload all sessions", Action: "reload"},
			{Key: "e", Label: "Export", Desc: "Export to JSON", Action: "export"},
			{Key: "c", Label: "Compare", Desc: "Compare sessions", Action: "compare"},
			{Key: "o", Label: "Open", Desc: "Open in editor", Action: "open"},
			{Key: "d", Label: "Diff", Desc: "Show diff view", Action: "diff"},
			{Key: "w", Label: "Waste", Desc: "Show waste analysis", Action: "waste"},
			{Key: "q", Label: "Quit", Desc: "Exit application", Action: "quit"},
		},
		cursor:  0,
		visible: true,
	}
}

// RenderActionMenu renders the quick action menu
func (m Model) RenderActionMenu() string {
	if !m.actionMenu.visible {
		return ""
	}

	width := 40
	style := lipgloss.NewStyle().
		Width(width).
		Border(lipgloss.RoundedBorder()).
		BorderForeground(lipgloss.Color("39")).
		Padding(1, 2)

	title := boldStyle.Render("Quick Actions")
	var items []string
	for i, item := range m.actionMenu.items {
		cursor := " "
		if i == m.actionMenu.cursor {
			cursor = "▸"
		}
		key := cyanStyle.Render(fmt.Sprintf("[%s]", item.Key))
		label := truncate(item.Label, 12)
		desc := dimStyle.Render(truncate(item.Desc, width-25))

		line := fmt.Sprintf(" %s %s %-12s %s", cursor, key, label, desc)
		items = append(items, line)
	}

	help := dimStyle.Render("\n ↑/↓ navigate · enter select · esc close")
	content := title + "\n" + strings.Join(items, "\n") + help

	return style.Render(content)
}

// ── Vim Navigation Enhancements ──

// HandleVimNavigation handles enhanced Vim-style navigation
func (m *Model) HandleVimNavigation(key string) bool {
	switch key {
	case "ctrl+d": // Half page down
		if m.view == viewList {
			m.table.MoveDown(m.table.Height() / 2)
		} else {
			m.viewport.LineDown(m.viewport.Height / 2)
		}
		return true

	case "ctrl+u": // Half page up
		if m.view == viewList {
			m.table.MoveUp(m.table.Height() / 2)
		} else {
			m.viewport.LineUp(m.viewport.Height / 2)
		}
		return true

	case "G": // Go to bottom
		if m.view == viewList {
			rows := m.table.Rows()
			if len(rows) > 0 {
				m.table.SetCursor(len(rows) - 1)
			}
		} else {
			m.viewport.GotoBottom()
		}
		return true

	case "H": // Go to top of visible
		if m.view == viewList {
			m.table.SetCursor(0)
		}
		return true

	case "M": // Go to middle
		if m.view == viewList {
			rows := m.table.Rows()
			if len(rows) > 0 {
				m.table.SetCursor(len(rows) / 2)
			}
		}
		return true

	case "L": // Go to bottom of visible
		if m.view == viewList {
			rows := m.table.Rows()
			if len(rows) > 0 {
				m.table.SetCursor(len(rows) - 1)
			}
		}
		return true

	case "z": // Center on cursor
		if m.view == viewDetail || m.view == viewDiagnostics {
			m.viewport.SetYOffset(0)
		}
		return true
	}

	return false
}

// ── Enhanced Status Bar ──

// RenderEnhancedStatusBar renders an enhanced status bar with more info
func (m Model) RenderEnhancedStatusBar(width int) string {
	// Left side: view info
	leftItems := []string{
		m.viewName(),
		fmt.Sprintf("%d/%d", len(m.filteredIndices), len(m.sessions)),
	}

	if m.sortBy != "" {
		dir := "↑"
		if m.sortDesc {
			dir = "↓"
		}
		leftItems = append(leftItems, fmt.Sprintf("sort:%s%s", m.sortBy, dir))
	}

	if m.hasAnyFilter() {
		leftItems = append(leftItems, "filter:active")
	}

	// Right side: quick stats
	rightItems := []string{}
	if len(m.sessions) > 0 {
		totalCost := 0.0
		totalTokens := 0
		for _, s := range m.sessions {
			totalCost += s.Metrics.CostEstimated
			totalTokens += s.Metrics.TokensInput + s.Metrics.TokensOutput
		}
		rightItems = append(rightItems,
			fmt.Sprintf("💰 $%.2f", totalCost),
			fmt.Sprintf("📊 %d%%", int(m.aggStats.AvgHealth)),
		)
	}

	left := statusStyle.Render(strings.Join(leftItems, " · "))
	right := dimStyle.Render(strings.Join(rightItems, " · "))

	gap := width - lipgloss.Width(left) - lipgloss.Width(right) - 4
	if gap < 0 {
		gap = 0
	}

	return lipgloss.JoinHorizontal(lipgloss.Center,
		left,
		strings.Repeat(" ", gap),
		right,
	)
}

// ── Split View (List + Preview) ──

// RenderSplitListView renders list with preview panel on the right
func (m Model) RenderSplitListView() string {
	contentW := m.frameBodyWidth()

	// Need at least 160 columns for split view to avoid compressing table columns
	if contentW < 160 {
		return m.renderListView()
	}

	// 40% list, 60% preview
	listW := contentW * 40 / 100
	previewW := contentW - listW - 2

	// Render list
	listContent := m.renderListTable(listW)

	// Render preview
	previewContent := m.RenderSessionPreview(previewW)

	return lipgloss.JoinHorizontal(lipgloss.Top,
		listContent,
		" ",
		previewContent,
	)
}

// renderListTable renders just the table portion
func (m Model) renderListTable(width int) string {
	tableView := m.table
	tableView.SetWidth(width)
	tableView.SetHeight(m.listTableHeight(0))
	return tableView.View()
}

// ── Skeleton Loading Screen ──

// RenderSkeletonLoading renders a skeleton loading screen
func (m Model) RenderSkeletonLoading() string {
	width := m.width
	if width <= 0 {
		width = 80
	}

	// Clamp minimum width to avoid panic
	if width < 20 {
		width = 20
	}

	innerW := width - 4
	if innerW < 4 {
		innerW = 4
	}

	// Hero skeleton
	hero := lipgloss.NewStyle().
		Width(innerW).
		Height(3).
		Border(lipgloss.RoundedBorder()).
		BorderForeground(lipgloss.Color("240")).
		Render(dimStyle.Render("  Loading agenttrace..."))

	// Metrics skeleton - clamp repeat count
	metricsRepeat := innerW / 5
	if metricsRepeat < 1 {
		metricsRepeat = 1
	}
	metrics := lipgloss.NewStyle().
		Width(innerW).
		Height(3).
		Border(lipgloss.RoundedBorder()).
		BorderForeground(lipgloss.Color("240")).
		Render("  " + strings.Repeat("░░░░ ", metricsRepeat))

	// Table skeleton - clamp row width
	tableRowW := innerW - 4
	if tableRowW < 1 {
		tableRowW = 1
	}
	tableRows := make([]string, 10)
	for i := range tableRows {
		tableRows[i] = "  " + strings.Repeat("░", tableRowW)
	}
	table := lipgloss.NewStyle().
		Width(innerW).
		Height(12).
		Border(lipgloss.RoundedBorder()).
		BorderForeground(lipgloss.Color("240")).
		Render(strings.Join(tableRows, "\n"))

	return lipgloss.JoinVertical(lipgloss.Left,
		hero,
		"",
		metrics,
		"",
		table,
	)
}

// ── Helper Functions ──

// healthColorPreview returns the appropriate color for a health score (preview version)
func healthColorPreview(health int) lipgloss.Style {
	switch {
	case health >= 80:
		return greenStyle
	case health >= 50:
		return yellowStyle
	default:
		return redStyle
	}
}
