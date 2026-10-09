mod app;

pub use app::{run, run_with_language, run_with_sessions, run_with_sessions_with_language};

// rm-819: the TUI re-exports the ONE stdout sanitizer family from
// agenttrace-core instead of growing its own. Before this, every
// render site pushed transcript-derived strings (session names,
// statuses, model ids, report text) straight into the buffer, and the
// closest local helper (`terminal_safe_report`) ASCII-folded box
// characters but passed ESC/BEL/C1 through untouched — a poisoned
// journal could drive the terminal around the ratatui frame. The two
// helpers below are the same functions the CLI dispatches stdout
// through; cells that went through the line sanitizer still compose
// (U+FFFD is printable, sanitizing twice is a no-op).
pub use agenttrace_core::{sanitize_line_segment, sanitize_output_document};
