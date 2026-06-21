use agenttrace_core::{
    average_health, canonical_sessions, clear_session_cache, collect_session_files,
    compute_overview, discover_session_dirs, load_sessions_from_dir, render_waste_report,
    report_compare, report_text, session_cache_path, total_tokens, Overview, Session, VERSION,
};
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Cell, Paragraph, Row, Table, TableState, Tabs, Wrap};
use ratatui::{DefaultTerminal, Frame};
use std::cmp::Ordering;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver};
use std::thread;
use std::time::Duration;

const POLL_INTERVAL: Duration = Duration::from_millis(120);
type LoadResult = anyhow::Result<(bool, Vec<Session>)>;

pub fn run(sessions_dir: &str) -> anyhow::Result<()> {
    let label = if sessions_dir.trim().is_empty() {
        "auto-discovery"
    } else {
        sessions_dir
    };
    run_with_app(App::new_loading(label, sessions_dir.to_string()))
}

pub fn run_with_sessions(sessions: Vec<Session>, label: &str) -> anyhow::Result<()> {
    run_with_app(App::new(sessions, label, None))
}

fn run_with_app(app: App) -> anyhow::Result<()> {
    let mut terminal = ratatui::init();
    let result = run_app(&mut terminal, app);
    ratatui::restore();
    result
}

fn run_app(terminal: &mut DefaultTerminal, mut app: App) -> anyhow::Result<()> {
    loop {
        app.poll_pending_load();
        terminal.draw(|frame| render(frame, &mut app))?;
        if event::poll(POLL_INTERVAL)? && app.handle_event(event::read()?)? {
            break;
        }
    }
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum View {
    Overview,
    List,
    Detail,
    Diagnostics,
    Diff,
    Help,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum InputMode {
    Normal,
    Search,
    Command,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SortKey {
    Recent,
    Health,
    Cost,
    Turns,
    Failures,
    Source,
    Name,
    Anomalies,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Language {
    En,
    Zh,
}

impl Language {
    fn toggle(self) -> Self {
        match self {
            Self::En => Self::Zh,
            Self::Zh => Self::En,
        }
    }
}

struct App {
    sessions: Vec<Session>,
    overview: Overview,
    source_label: String,
    reload_dir: Option<String>,
    view: View,
    mode: InputMode,
    filtered: Vec<usize>,
    selected: usize,
    table_state: TableState,
    query: String,
    health_filter: String,
    source_filter: String,
    model_filter: String,
    cost_filter: Option<(CostOp, f64)>,
    anomaly_filter: Option<String>,
    input: String,
    status: String,
    sort_key: SortKey,
    sort_desc: bool,
    scroll: u16,
    pending_load: Option<Receiver<LoadResult>>,
    load_state: LoadState,
    language: Language,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum CostOp {
    Gt,
    Gte,
    Lt,
    Lte,
    Eq,
}

#[derive(Debug, Clone, Default)]
struct LoadState {
    phase: LoadPhase,
    force: bool,
    source: String,
    discovered: usize,
    parsed: usize,
    cache_hits: usize,
    cache_state: String,
    sources: Vec<(String, usize)>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
enum LoadPhase {
    #[default]
    Idle,
    Discovering,
    Parsing,
    Ready,
    Failed,
}

impl App {
    fn new(sessions: Vec<Session>, source_label: &str, reload_dir: Option<String>) -> Self {
        let sessions = canonical_sessions(&sessions);
        let overview = compute_overview(&sessions);
        let mut app = Self {
            sessions,
            overview,
            source_label: source_label.to_string(),
            reload_dir,
            view: View::Overview,
            mode: InputMode::Normal,
            filtered: Vec::new(),
            selected: 0,
            table_state: TableState::default(),
            query: String::new(),
            health_filter: String::new(),
            source_filter: String::new(),
            model_filter: String::new(),
            cost_filter: None,
            anomaly_filter: None,
            input: String::new(),
            status: String::new(),
            sort_key: SortKey::Recent,
            sort_desc: true,
            scroll: 0,
            pending_load: None,
            load_state: LoadState::default(),
            language: Language::En,
        };
        app.refresh_filtered();
        app
    }

    fn new_loading(source_label: &str, reload_dir: String) -> Self {
        let mut app = Self::new(Vec::new(), source_label, Some(reload_dir));
        app.start_reload(false);
        app
    }

    fn handle_event(&mut self, event: Event) -> anyhow::Result<bool> {
        let Event::Key(key) = event else {
            return Ok(false);
        };
        if key.kind != KeyEventKind::Press {
            return Ok(false);
        }
        match self.mode {
            InputMode::Search => return Ok(self.handle_search_key(key)),
            InputMode::Command => return self.handle_command_key(key),
            InputMode::Normal => {}
        }
        self.handle_normal_key(key)
    }

    fn handle_search_key(&mut self, key: KeyEvent) -> bool {
        match key.code {
            KeyCode::Esc => {
                self.mode = InputMode::Normal;
                self.input.clear();
            }
            KeyCode::Enter => {
                self.query = self.input.trim().to_string();
                self.mode = InputMode::Normal;
                self.input.clear();
                self.status = if self.query.is_empty() {
                    self.t("filter cleared", "已清除筛选").to_string()
                } else {
                    format!("{}: {}", self.t("filter", "筛选"), self.query)
                };
                self.refresh_filtered();
                self.view = View::List;
            }
            KeyCode::Backspace => {
                self.input.pop();
            }
            KeyCode::Char(c) => self.input.push(c),
            _ => {}
        }
        false
    }

    fn handle_command_key(&mut self, key: KeyEvent) -> anyhow::Result<bool> {
        match key.code {
            KeyCode::Esc => {
                self.mode = InputMode::Normal;
                self.input.clear();
            }
            KeyCode::Enter => {
                let command = self.input.trim().to_string();
                self.input.clear();
                self.mode = InputMode::Normal;
                return self.run_command(&command);
            }
            KeyCode::Backspace => {
                self.input.pop();
            }
            KeyCode::Char(c) => self.input.push(c),
            _ => {}
        }
        Ok(false)
    }

    fn handle_normal_key(&mut self, key: KeyEvent) -> anyhow::Result<bool> {
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('r') {
            self.reload(true)?;
            return Ok(false);
        }
        match key.code {
            KeyCode::Char('q') | KeyCode::Char('Q') => return Ok(true),
            KeyCode::Char(':') => {
                self.mode = InputMode::Command;
                self.input.clear();
            }
            KeyCode::Char('/') => {
                self.mode = InputMode::Search;
                self.input.clear();
                self.view = View::List;
            }
            KeyCode::Char('?') => self.view = View::Help,
            KeyCode::Tab => self.next_view(),
            KeyCode::Char('0') => self.view = View::Overview,
            KeyCode::Char('1') => self.view = View::List,
            KeyCode::Char('2') => {
                if self.selected_session().is_some() {
                    self.view = View::Detail;
                    self.scroll = 0;
                }
            }
            KeyCode::Enter => {
                if self.view == View::Overview {
                    self.open_inspect_first();
                } else if self.selected_session().is_some() {
                    self.view = View::Detail;
                    self.scroll = 0;
                }
            }
            KeyCode::Char('3') | KeyCode::Char('w') => {
                if self.selected_session().is_some() {
                    self.view = View::Diagnostics;
                    self.scroll = 0;
                }
            }
            KeyCode::Char('4') | KeyCode::Char('d') => {
                self.view = View::Diff;
                self.scroll = 0;
            }
            KeyCode::Char('r') => self.reload(false)?,
            KeyCode::Char('f') => self.cycle_health_filter(),
            KeyCode::Char('s') => self.filter_selected_source(),
            KeyCode::Char('$') => self.filter_costly_sessions(),
            KeyCode::Char('!') => self.filter_critical_sessions(),
            KeyCode::Char('c') => self.set_sort(SortKey::Cost),
            KeyCode::Char('e') => self.set_sort(SortKey::Failures),
            KeyCode::Char('h') => self.set_sort(SortKey::Health),
            KeyCode::Char('n') => self.set_sort(SortKey::Name),
            KeyCode::Char('t') => self.set_sort(SortKey::Turns),
            KeyCode::Char('a') => self.set_sort(SortKey::Anomalies),
            KeyCode::Char('l') | KeyCode::Char('L') => self.toggle_language(),
            KeyCode::Esc => {
                if self.has_filters() {
                    self.clear_filters();
                    self.refresh_filtered();
                    self.status = self.t("filter cleared", "已清除筛选").to_string();
                } else {
                    self.view = View::Overview;
                }
            }
            KeyCode::Down | KeyCode::Char('j') => self.move_next(),
            KeyCode::Up | KeyCode::Char('k') => self.move_previous(),
            KeyCode::PageDown => self.scroll = self.scroll.saturating_add(8),
            KeyCode::PageUp => self.scroll = self.scroll.saturating_sub(8),
            _ => {}
        }
        Ok(false)
    }

    fn t(&self, en: &'static str, zh: &'static str) -> &'static str {
        text(self.language, en, zh)
    }

    fn toggle_language(&mut self) {
        self.language = self.language.toggle();
        self.status = match self.language {
            Language::En => "language: English".to_string(),
            Language::Zh => "语言：中文".to_string(),
        };
    }

    fn run_command(&mut self, command: &str) -> anyhow::Result<bool> {
        let command = command.trim();
        let lower = command.to_ascii_lowercase();
        match lower.as_str() {
            "" => {}
            "q" | "quit" | "exit" => return Ok(true),
            "0" | "overview" => self.view = View::Overview,
            "1" | "list" => self.view = View::List,
            "2" | "detail" => {
                if self.selected_session().is_some() {
                    self.view = View::Detail;
                    self.scroll = 0;
                }
            }
            "3" | "diagnostics" | "waste" => {
                if self.selected_session().is_some() {
                    self.view = View::Diagnostics;
                    self.scroll = 0;
                }
            }
            "4" | "diff" => self.view = View::Diff,
            "help" | "?" => self.view = View::Help,
            "first" | "inspect" => {
                self.select_inspect_item(0);
            }
            "clear" | "reset" => {
                self.clear_filters();
                self.refresh_filtered();
                self.view = View::List;
                self.status = self.t("filter cleared", "已清除筛选").to_string();
            }
            "reload" | "r" => self.reload(false)?,
            "critical" => {
                self.health_filter = "crit".to_string();
                self.refresh_filtered();
                self.view = View::List;
                self.status = self
                    .t("filter health: critical", "筛选健康度：严重")
                    .to_string();
            }
            "anomalies" | "anomaly" => {
                self.anomaly_filter = Some(String::new());
                self.refresh_filtered();
                self.view = View::List;
                self.status = self.t("filter anomalies", "筛选异常").to_string();
            }
            _ if lower.starts_with("first ") || lower.starts_with("inspect ") => {
                let fields = command.split_whitespace().collect::<Vec<_>>();
                if fields.len() == 2 {
                    match fields[1].parse::<usize>() {
                        Ok(rank) if rank > 0 => {
                            self.select_inspect_item(rank - 1);
                        }
                        _ => {
                            self.status = self
                                .t("usage: :inspect [1-5]", "用法：:inspect [1-5]")
                                .to_string()
                        }
                    }
                } else {
                    self.status = self
                        .t("usage: :inspect [1-5]", "用法：:inspect [1-5]")
                        .to_string();
                }
            }
            _ if lower.starts_with("search ") || lower.starts_with("filter ") => {
                let query = command
                    .split_once(' ')
                    .map(|(_, value)| value.trim())
                    .unwrap_or("");
                self.query = query.to_string();
                self.refresh_filtered();
                self.view = View::List;
                self.status = format!("{}: {}", self.t("filter", "筛选"), self.query);
            }
            _ if lower.starts_with("health ") => {
                let value = command_value(command);
                if parse_health_filter(&value).is_some() {
                    self.health_filter = value.to_ascii_lowercase();
                    self.refresh_filtered();
                    self.view = View::List;
                    self.status = format!(
                        "{}: {}",
                        self.t("filter health", "筛选健康度"),
                        self.health_filter
                    );
                } else {
                    self.status = self
                        .t(
                            "usage: :health good|warn|crit|<80|>=90",
                            "用法：:health good|warn|crit|<80|>=90",
                        )
                        .to_string();
                }
            }
            _ if lower.starts_with("source ") => {
                self.source_filter = command_value(command);
                self.refresh_filtered();
                self.view = View::List;
                self.status = format!(
                    "{}: {}",
                    self.t("filter source", "筛选来源"),
                    self.source_filter
                );
            }
            _ if lower.starts_with("model ") => {
                self.model_filter = command_value(command);
                self.refresh_filtered();
                self.view = View::List;
                self.status = format!(
                    "{}: {}",
                    self.t("filter model", "筛选模型"),
                    self.model_filter
                );
            }
            _ if lower.starts_with("cost ") => {
                let value = command_value(command);
                if let Some(filter) = parse_cost_filter(&value) {
                    self.cost_filter = Some(filter);
                    self.refresh_filtered();
                    self.view = View::List;
                    self.status = format!("{}: {}", self.t("filter cost", "筛选成本"), value);
                } else {
                    self.status = self
                        .t(
                            "usage: :cost >0.10|>=1|<0.05|=0",
                            "用法：:cost >0.10|>=1|<0.05|=0",
                        )
                        .to_string();
                }
            }
            _ if lower.starts_with("anomaly ") || lower.starts_with("anomalies ") => {
                let value = command_value(command).to_ascii_lowercase();
                self.anomaly_filter = Some(value.clone());
                self.refresh_filtered();
                self.view = View::List;
                self.status = if value.is_empty() {
                    self.t("filter anomalies", "筛选异常").to_string()
                } else {
                    format!("{}: {value}", self.t("filter anomaly", "筛选异常"))
                };
            }
            _ if lower.starts_with("top ") => match parse_sort_key(&command_value(command)) {
                Some(key) => self.set_sort_desc(key, true),
                None => {
                    self.status = self
                        .t(
                            "usage: :top cost|turns|failures|health|source|anomalies",
                            "用法：:top cost|turns|failures|health|source|anomalies",
                        )
                        .to_string()
                }
            },
            _ if lower.starts_with("sort ") => {
                let fields = command.split_whitespace().collect::<Vec<_>>();
                if fields.len() < 2 || fields.len() > 3 {
                    self.status = self
                        .t(
                            "usage: :sort health|cost|turns|failures|source|name|anomalies [asc|desc]",
                            "用法：:sort health|cost|turns|failures|source|name|anomalies [asc|desc]",
                        )
                        .to_string();
                } else if let Some(key) = parse_sort_key(fields[1]) {
                    let desc = if fields.len() == 3 {
                        match fields[2].to_ascii_lowercase().as_str() {
                            "asc" => false,
                            "desc" => true,
                            _ => {
                                self.status = format!(
                                    "{}: {}",
                                    self.t("unknown sort direction", "未知排序方向"),
                                    fields[2]
                                );
                                return Ok(false);
                            }
                        }
                    } else {
                        key != SortKey::Name
                    };
                    self.set_sort_desc(key, desc);
                } else {
                    self.status = self
                        .t(
                            "usage: :sort health|cost|turns|failures|source|name|anomalies [asc|desc]",
                            "用法：:sort health|cost|turns|failures|source|name|anomalies [asc|desc]",
                        )
                        .to_string();
                }
            }
            _ => {
                self.query = command.to_string();
                self.refresh_filtered();
                self.view = View::List;
                self.status = format!("{}: {}", self.t("filter", "筛选"), self.query);
            }
        }
        Ok(false)
    }

    fn reload(&mut self, force: bool) -> anyhow::Result<()> {
        self.start_reload(force);
        Ok(())
    }

    fn start_reload(&mut self, force: bool) {
        let Some(dir) = self.reload_dir.as_deref() else {
            self.status = self
                .t(
                    "reload unavailable for this session source",
                    "当前会话来源不支持重新加载",
                )
                .to_string();
            return;
        };
        let dir = dir.to_string();
        let discovered_files = collect_tui_session_files(&dir);
        let discovered = discovered_files.len();
        let cache_hits = if force {
            0
        } else {
            count_cache_hits(&discovered_files)
        };
        let cache_state = if force {
            "cache bypass".to_string()
        } else {
            cache_state_label()
        };
        let (tx, rx) = mpsc::channel();
        self.pending_load = Some(rx);
        self.load_state = LoadState {
            phase: LoadPhase::Discovering,
            force,
            source: self.source_label.clone(),
            discovered,
            parsed: 0,
            cache_hits,
            cache_state,
            sources: Vec::new(),
        };
        self.status = if force {
            format!(
                "{} {} {}",
                self.t("force reload: discovering", "强制重载：发现中"),
                format_count(discovered as i64),
                self.t("session files", "个会话文件")
            )
        } else {
            format!(
                "{} {} {}",
                self.t("loading: discovering", "加载中：发现中"),
                format_count(discovered as i64),
                self.t("session files", "个会话文件")
            )
        };
        thread::spawn(move || {
            let result = load_sessions_for_tui(&dir, force);
            let _ = tx.send(result);
        });
    }

    fn poll_pending_load(&mut self) {
        let Some(rx) = self.pending_load.take() else {
            return;
        };
        match rx.try_recv() {
            Ok(Ok((force, sessions))) => self.apply_loaded_sessions(sessions, force),
            Ok(Err(err)) => {
                self.load_state.phase = LoadPhase::Failed;
                self.status = format!("{}: {err}", self.t("reload failed", "重新加载失败"));
            }
            Err(mpsc::TryRecvError::Empty) => {
                self.load_state.phase = LoadPhase::Parsing;
                self.pending_load = Some(rx);
            }
            Err(mpsc::TryRecvError::Disconnected) => {
                self.load_state.phase = LoadPhase::Failed;
                self.status = self
                    .t(
                        "reload failed: loader disconnected",
                        "重新加载失败：加载器已断开",
                    )
                    .to_string();
            }
        }
    }

    fn apply_loaded_sessions(&mut self, sessions: Vec<Session>, force: bool) {
        self.sessions = canonical_sessions(&sessions);
        self.overview = compute_overview(&self.sessions);
        self.selected = 0;
        self.scroll = 0;
        self.refresh_filtered();
        self.load_state.phase = LoadPhase::Ready;
        self.load_state.force = force;
        self.load_state.parsed = self.sessions.len();
        self.load_state.sources = source_counts(&self.sessions);
        self.status = if force {
            format!(
                "{} {} {} {} {}",
                self.t("force reloaded", "已强制重载"),
                self.sessions.len(),
                self.t("sessions from", "个会话，来自"),
                format_count(self.load_state.discovered as i64),
                self.t("files", "个文件")
            )
        } else {
            format!(
                "{} {} {} {} {}",
                self.t("loaded", "已加载"),
                self.sessions.len(),
                self.t("sessions from", "个会话，来自"),
                format_count(self.load_state.discovered as i64),
                self.t("files", "个文件")
            )
        };
    }

    fn next_view(&mut self) {
        self.view = match self.view {
            View::Overview => View::List,
            View::List => {
                if self.selected_session().is_some() {
                    View::Detail
                } else {
                    View::Overview
                }
            }
            View::Detail => View::Diagnostics,
            View::Diagnostics => View::Diff,
            View::Diff | View::Help => View::Overview,
        };
        self.scroll = 0;
    }

    fn set_sort(&mut self, key: SortKey) {
        if self.sort_key == key {
            self.sort_desc = !self.sort_desc;
        } else {
            self.sort_key = key;
            self.sort_desc = key != SortKey::Name;
        }
        self.refresh_filtered();
        self.status = format!(
            "{} {}",
            self.t("sorted by", "排序："),
            sort_key_label(self.sort_key, self.language)
        );
        self.view = View::List;
    }

    fn set_sort_desc(&mut self, key: SortKey, desc: bool) {
        self.sort_key = key;
        self.sort_desc = desc;
        self.refresh_filtered();
        self.status = format!(
            "{} {} {}",
            self.t("sorted by", "排序："),
            sort_key_label(self.sort_key, self.language),
            if self.sort_desc {
                self.t("desc", "降序")
            } else {
                self.t("asc", "升序")
            }
        );
        self.view = View::List;
    }

    fn clear_filters(&mut self) {
        self.query.clear();
        self.health_filter.clear();
        self.source_filter.clear();
        self.model_filter.clear();
        self.cost_filter = None;
        self.anomaly_filter = None;
    }

    fn has_filters(&self) -> bool {
        !self.query.is_empty()
            || !self.health_filter.is_empty()
            || !self.source_filter.is_empty()
            || !self.model_filter.is_empty()
            || self.cost_filter.is_some()
            || self.anomaly_filter.is_some()
    }

    fn cycle_health_filter(&mut self) {
        self.health_filter = match self.health_filter.as_str() {
            "" => "crit".to_string(),
            "crit" => "warn".to_string(),
            "warn" => "good".to_string(),
            _ => String::new(),
        };
        self.refresh_filtered();
        self.view = View::List;
        self.status = if self.health_filter.is_empty() {
            self.t("quick health filter cleared", "已清除快捷健康度筛选")
                .to_string()
        } else {
            format!(
                "{}: {}",
                self.t("quick health filter", "快捷健康度筛选"),
                health_filter_label(&self.health_filter, self.language)
            )
        };
    }

    fn filter_selected_source(&mut self) {
        let Some(source) = self
            .selected_session()
            .map(|session| session.metrics.source_tool.clone())
        else {
            self.status = self
                .t("source filter unavailable", "来源筛选不可用")
                .to_string();
            return;
        };
        if source.is_empty() {
            self.status = self
                .t("source filter unavailable", "来源筛选不可用")
                .to_string();
            return;
        }
        self.source_filter = source;
        self.refresh_filtered();
        self.view = View::List;
        self.status = format!(
            "{}: {}",
            self.t("quick source filter", "快捷来源筛选"),
            display_source_label(&self.source_filter)
        );
    }

    fn filter_costly_sessions(&mut self) {
        self.cost_filter = Some((CostOp::Gt, 0.0));
        self.refresh_filtered();
        self.view = View::List;
        self.status = self
            .t("quick cost filter: >0", "快捷成本筛选：>0")
            .to_string();
    }

    fn filter_critical_sessions(&mut self) {
        self.health_filter = "crit".to_string();
        self.refresh_filtered();
        self.view = View::List;
        self.status = self.t("quick critical filter", "快捷严重筛选").to_string();
    }

    fn open_inspect_first(&mut self) {
        self.select_inspect_item(0);
    }

    fn select_inspect_item(&mut self, rank: usize) -> bool {
        let items = inspect_first_items(&self.sessions);
        let Some(item) = items.get(rank).cloned() else {
            self.status = if self.sessions.is_empty() {
                self.t("no sessions loaded", "尚未加载会话").to_string()
            } else {
                format!(
                    "{} {} {}",
                    self.t("inspect rank", "检查排名"),
                    rank + 1,
                    self.t("unavailable", "不可用")
                )
            };
            return false;
        };
        let Some(session) = self.sessions.get(item.index) else {
            self.status = self
                .t("inspect target unavailable", "检查目标不可用")
                .to_string();
            return false;
        };
        let session_name = session.name.clone();
        let target_view = inspect_target_view(item.label);

        self.clear_filters();
        self.refresh_filtered();
        let Some(position) = self.filtered.iter().position(|idx| *idx == item.index) else {
            self.status = self
                .t("inspect target hidden", "检查目标已隐藏")
                .to_string();
            return false;
        };
        self.selected = position;
        self.clamp_selection();
        self.view = target_view;
        self.scroll = 0;
        self.status = format!(
            "{} {} #{}: {}",
            self.t("inspect", "检查"),
            inspect_label(item.label, self.language),
            rank + 1,
            short(&session_name, 36)
        );
        true
    }

    fn refresh_filtered(&mut self) {
        let query = self.query.trim().to_ascii_lowercase();
        self.filtered = self
            .sessions
            .iter()
            .enumerate()
            .filter_map(|(idx, session)| {
                if self.session_visible(session, &query) {
                    Some(idx)
                } else {
                    None
                }
            })
            .collect();
        let sort_key = self.sort_key;
        let sort_desc = self.sort_desc;
        self.filtered.sort_by(|a, b| {
            compare_sessions(&self.sessions[*a], &self.sessions[*b], sort_key, sort_desc)
        });
        self.clamp_selection();
    }

    fn session_visible(&self, session: &Session, query: &str) -> bool {
        (query.is_empty() || session_matches(session, query))
            && matches_health_filter(session, &self.health_filter)
            && matches_source_filter(session, &self.source_filter)
            && matches_text_filter(&session.metrics.model_used, &self.model_filter)
            && matches_cost_filter(session, self.cost_filter)
            && matches_anomaly_filter(session, self.anomaly_filter.as_deref())
    }

    fn clamp_selection(&mut self) {
        if self.filtered.is_empty() {
            self.selected = 0;
            self.table_state.select(None);
            return;
        }
        if self.selected >= self.filtered.len() {
            self.selected = self.filtered.len() - 1;
        }
        self.table_state.select(Some(self.selected));
    }

    fn move_next(&mut self) {
        if self.view == View::Detail || self.view == View::Diagnostics || self.view == View::Diff {
            self.scroll = self.scroll.saturating_add(1);
            return;
        }
        if !self.filtered.is_empty() {
            self.selected = (self.selected + 1).min(self.filtered.len() - 1);
            self.clamp_selection();
        }
    }

    fn move_previous(&mut self) {
        if self.view == View::Detail || self.view == View::Diagnostics || self.view == View::Diff {
            self.scroll = self.scroll.saturating_sub(1);
            return;
        }
        self.selected = self.selected.saturating_sub(1);
        self.clamp_selection();
    }

    fn selected_session(&self) -> Option<&Session> {
        self.filtered
            .get(self.selected)
            .and_then(|idx| self.sessions.get(*idx))
    }

    fn visible_sessions(&self) -> Vec<Session> {
        self.filtered
            .iter()
            .filter_map(|idx| self.sessions.get(*idx).cloned())
            .collect()
    }
}

fn text(language: Language, en: &'static str, zh: &'static str) -> &'static str {
    match language {
        Language::En => en,
        Language::Zh => zh,
    }
}

fn sort_key_label(key: SortKey, language: Language) -> &'static str {
    match key {
        SortKey::Recent => text(language, "Recent", "最近"),
        SortKey::Health => text(language, "Health", "健康度"),
        SortKey::Cost => text(language, "Cost", "成本"),
        SortKey::Turns => text(language, "Turns", "轮次"),
        SortKey::Failures => text(language, "Failures", "失败"),
        SortKey::Source => text(language, "Source", "来源"),
        SortKey::Name => text(language, "Name", "名称"),
        SortKey::Anomalies => text(language, "Anomalies", "异常"),
    }
}

fn health_filter_label(filter: &str, language: Language) -> String {
    match filter {
        "good" | "healthy" => text(language, "good", "良好").to_string(),
        "warn" | "warning" => text(language, "warning", "警告").to_string(),
        "crit" | "critical" => text(language, "critical", "严重").to_string(),
        _ => filter.to_string(),
    }
}

fn inspect_label(label: &str, language: Language) -> &'static str {
    match label {
        "critical" => text(language, "critical", "严重"),
        "anomaly" => text(language, "anomaly", "异常"),
        "failures" => text(language, "failures", "失败"),
        "cost" => text(language, "cost", "成本"),
        "latency" => text(language, "latency", "延迟"),
        _ => text(language, "session", "会话"),
    }
}

fn load_sessions_for_tui(dir: &str, force: bool) -> anyhow::Result<(bool, Vec<Session>)> {
    if force {
        clear_session_cache()?;
    }
    let load_dir = if dir.trim().is_empty() {
        None
    } else {
        Some(std::path::Path::new(dir))
    };
    Ok((force, load_sessions_from_dir(load_dir)))
}

fn render(frame: &mut Frame<'_>, app: &mut App) {
    let area = frame.area();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(4),
            Constraint::Length(3),
            Constraint::Min(4),
            Constraint::Length(3),
        ])
        .split(area);

    render_header(frame, app, chunks[0]);
    render_tabs(frame, app, chunks[1]);
    match app.view {
        View::Overview => render_overview(frame, app, chunks[2]),
        View::List => render_list(frame, app, chunks[2]),
        View::Detail => render_report(
            frame,
            app,
            chunks[2],
            report_title(app, app.t("Detail", "详情")),
            detail_text(app),
        ),
        View::Diagnostics => render_report(
            frame,
            app,
            chunks[2],
            report_title(app, app.t("Diagnostics", "诊断")),
            diagnostics_text(app),
        ),
        View::Diff => render_report(frame, app, chunks[2], diff_title(app), diff_text(app)),
        View::Help => render_report(
            frame,
            app,
            chunks[2],
            app.t("Help", "帮助"),
            help_text(app.language),
        ),
    }
    render_footer(frame, app, chunks[3]);
}

fn render_header(frame: &mut Frame<'_>, app: &App, area: Rect) {
    let source = if area.width >= 96 {
        format!(
            "{}={}  {}={}",
            app.t("source", "来源"),
            short(&display_source_label(&app.source_label), 28),
            app.t("sessions", "会话"),
            app.sessions.len()
        )
    } else {
        format!(
            "{}={}  n={}",
            app.t("src", "来源"),
            short(&display_source_label(&app.source_label), 16),
            app.sessions.len()
        )
    };
    let text = vec![
        Line::from(vec![
            Span::styled(
                format!("AGENTTRACE v{}", VERSION),
                Style::default()
                    .fg(Color::LightGreen)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("  "),
            Span::raw(source),
            Span::raw(format!("  {}=", app.t("next", "下一步"))),
            Span::styled(
                next_action(app),
                Style::default()
                    .fg(priority_color(app))
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::raw(format!("{} ", app.t("health", "健康度"))),
            Span::styled(
                format!("{:.1}", average_health(&app.sessions)),
                Style::default()
                    .fg(health_color(average_health(&app.sessions) as i32))
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(format!(
                "  {}={} {}={} {}={}  {}=${:.4}  {}={}  {}",
                app.t("ok", "良好"),
                app.overview.healthy,
                app.t("warn", "警告"),
                app.overview.warning,
                app.t("crit", "严重"),
                app.overview.critical,
                app.t("cost", "成本"),
                app.overview.total_cost,
                app.t("tokens", "令牌"),
                format_count(total_tokens_all(&app.sessions)),
                load_summary_line(app)
            )),
        ]),
    ];
    frame.render_widget(
        Paragraph::new(text).block(Block::default().borders(Borders::ALL)),
        area,
    );
}

fn render_tabs(frame: &mut Frame<'_>, app: &App, area: Rect) {
    let selected = match app.view {
        View::Overview => 0,
        View::List => 1,
        View::Detail => 2,
        View::Diagnostics => 3,
        View::Diff => 4,
        View::Help => 5,
    };
    let tabs = Tabs::new([
        format!("0 {}", app.t("Overview", "概览")),
        format!("1 {}", app.t("List", "列表")),
        format!("2 {}", app.t("Detail", "详情")),
        format!("3 {}", app.t("Diagnostics", "诊断")),
        format!("4 {}", app.t("Diff", "对比")),
        format!("? {}", app.t("Help", "帮助")),
    ])
    .select(selected)
    .style(Style::default().fg(Color::Gray))
    .highlight_style(
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    )
    .block(Block::default().borders(Borders::ALL));
    frame.render_widget(tabs, area);
}

fn render_overview(frame: &mut Frame<'_>, app: &App, area: Rect) {
    if area.width < 96 {
        render_overview_compact(frame, app, area);
        return;
    }

    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(44), Constraint::Percentage(56)])
        .split(area);

    let left_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(6),
            Constraint::Length(6),
            Constraint::Min(4),
        ])
        .split(chunks[0]);
    render_scoreboard(frame, app, left_chunks[0]);
    render_driver_summary(frame, app, left_chunks[1]);
    render_loading_status(frame, app, left_chunks[2]);

    let right_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(12), Constraint::Min(4)])
        .split(chunks[1]);
    render_inspect_first(frame, app, right_chunks[0]);
    render_recent_sessions(frame, app, right_chunks[1]);
}

fn render_overview_compact(frame: &mut Frame<'_>, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(6),
            Constraint::Length(10),
            Constraint::Min(4),
        ])
        .split(area);
    render_scoreboard(frame, app, chunks[0]);
    render_inspect_first(frame, app, chunks[1]);
    render_recent_sessions(frame, app, chunks[2]);
}

fn render_scoreboard(frame: &mut Frame<'_>, app: &App, area: Rect) {
    let lines = vec![
        Line::from(vec![
            Span::raw(format!("{} ", app.t("health", "健康度"))),
            Span::styled(
                format!("{:.1}", average_health(&app.sessions)),
                Style::default()
                    .fg(health_color(average_health(&app.sessions) as i32))
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(format!(
                "  {} {}  {} {}  {} {}",
                app.t("sessions", "会话"),
                app.overview.total_sessions,
                app.t("critical", "严重"),
                app.overview.critical,
                app.t("warning", "警告"),
                app.overview.warning
            )),
        ]),
        Line::from(format!(
            "{} ${:.4}  {} {}  {} {}  p95 {}",
            app.t("cost", "成本"),
            app.overview.total_cost,
            app.t("tokens", "令牌"),
            format_count(total_tokens_all(&app.sessions)),
            app.t("elapsed", "耗时"),
            format_duration(total_duration(&app.sessions)),
            format_duration(p95_gap(&app.sessions))
        )),
        Line::from(format!("{}: {}", app.t("next", "下一步"), next_action(app))),
        Line::from(top_model_line(app)),
    ];
    frame.render_widget(
        Paragraph::new(lines)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(app.t("Scoreboard", "记分板")),
            )
            .wrap(Wrap { trim: true }),
        area,
    );
}

fn render_inspect_first(frame: &mut Frame<'_>, app: &App, area: Rect) {
    frame.render_widget(
        Paragraph::new(inspect_first_lines(app, area.width))
            .block(Block::default().borders(Borders::ALL).title(app.t(
                "Inspect First - Enter opens #1, :inspect N jumps",
                "优先检查 - Enter 打开 #1，:inspect N 跳转",
            )))
            .wrap(Wrap { trim: true }),
        area,
    );
}

fn render_recent_sessions(frame: &mut Frame<'_>, app: &App, area: Rect) {
    let name_width = if area.width >= 92 { 28 } else { 20 };
    let mut lines = Vec::new();
    for session in app
        .visible_sessions()
        .iter()
        .take(recent_limit(area.height))
    {
        lines.push(Line::from(vec![
            Span::styled(
                format!("{:>3} ", session.health),
                Style::default().fg(health_color(session.health)),
            ),
            Span::raw(format!(
                "{:<name_width$} ${:<8.4} {:<14} {}",
                short(&session.name, name_width),
                session.metrics.cost_estimated,
                short(&display_session_source(session), 14),
                short(&triage_reason(session, app.language), 24),
                name_width = name_width
            )),
        ]));
    }
    if lines.is_empty() {
        lines.push(Line::from(app.t("no sessions visible", "没有可见会话")));
    }
    frame.render_widget(
        Paragraph::new(lines)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(app.t("Recent Sessions", "最近会话")),
            )
            .wrap(Wrap { trim: true }),
        area,
    );
}

fn render_list(frame: &mut Frame<'_>, app: &mut App, area: Rect) {
    if app.sessions.is_empty() {
        frame.render_widget(
            Paragraph::new(app.t(
                "No sessions loaded yet. Wait for loading or press r to reload.",
                "尚未加载会话。等待加载完成，或按 r 重新加载。",
            ))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(app.t("Sessions", "会话")),
            ),
            area,
        );
        return;
    }
    let active_filters = active_filter_summary(app);
    if app.filtered.is_empty() && !active_filters.is_empty() {
        let text = vec![
            Line::from(app.t(
                "No visible sessions match the active filters.",
                "没有会话匹配当前筛选。",
            )),
            Line::from(format!(
                "{}: {}",
                app.t("Active filters", "当前筛选"),
                active_filters
            )),
            Line::from(app.t(
                "Press Esc or run :clear to show all sessions.",
                "按 Esc 或运行 :clear 显示全部会话。",
            )),
        ];
        frame.render_widget(
            Paragraph::new(text)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title(app.t("Sessions - 0 visible", "会话 - 0 个可见")),
                )
                .wrap(Wrap { trim: true }),
            area,
        );
        return;
    }

    if area.width < 96 || area.height < 24 {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(3), Constraint::Min(4)])
            .split(area);
        render_list_status(frame, app, chunks[0], &active_filters);
        render_session_table(frame, app, chunks[1], &active_filters, true);
        return;
    }

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(6),
            Constraint::Length(5),
            Constraint::Min(4),
        ])
        .split(area);
    render_list_status(frame, app, chunks[0], &active_filters);

    let top = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(48), Constraint::Percentage(52)])
        .split(chunks[1]);
    render_driver_summary(frame, app, top[0]);
    render_selected_summary(frame, app, top[1]);

    render_loading_status(frame, app, chunks[2]);
    render_session_table(frame, app, chunks[3], &active_filters, false);
}

fn render_session_table(
    frame: &mut Frame<'_>,
    app: &mut App,
    area: Rect,
    active_filters: &str,
    compact: bool,
) {
    let rows = app.filtered.iter().filter_map(|idx| {
        let session = app.sessions.get(*idx)?;
        let metrics = &session.metrics;
        let success_rate = tool_success_rate(session);
        if compact {
            Some(Row::new(vec![
                Cell::from(short(&session.name, 18)),
                Cell::from(session.health.to_string())
                    .style(Style::default().fg(health_color(session.health))),
                Cell::from(format!("${:.3}", metrics.cost_estimated)),
                Cell::from(metrics.tool_calls_fail.to_string()),
                Cell::from(short(&triage_reason(session, app.language), 16)),
            ]))
        } else {
            Some(
                Row::new(vec![
                    Cell::from(short(&session.name, 22)),
                    Cell::from(health_label(session.health, app.language))
                        .style(Style::default().fg(health_color(session.health))),
                    Cell::from(short(&display_session_source(session), 14)),
                    Cell::from(short(&metrics.model_used, 14)),
                    Cell::from(format!("${:.4}", metrics.cost_estimated)),
                    Cell::from(format_count(total_tokens(session))),
                    Cell::from(format!("{success_rate:.0}%")),
                    Cell::from(metrics.tool_calls_fail.to_string()),
                    Cell::from(session.anomalies.len().to_string()),
                    Cell::from(short(&triage_reason(session, app.language), 20)),
                ])
                .style(session_row_style(session)),
            )
        }
    });
    let title = session_table_title(app, active_filters);
    let table = if compact {
        Table::new(
            rows,
            [
                Constraint::Length(18),
                Constraint::Length(6),
                Constraint::Length(8),
                Constraint::Length(5),
                Constraint::Min(12),
            ],
        )
        .header(
            Row::new([
                app.t("session", "会话"),
                app.t("score", "评分"),
                app.t("cost", "成本"),
                app.t("fail", "失败"),
                app.t("reason", "原因"),
            ])
            .style(
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
        )
        .block(Block::default().borders(Borders::ALL).title(title))
        .row_highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("> ")
    } else {
        Table::new(
            rows,
            [
                Constraint::Length(22),
                Constraint::Length(8),
                Constraint::Length(14),
                Constraint::Length(14),
                Constraint::Length(10),
                Constraint::Length(10),
                Constraint::Length(6),
                Constraint::Length(6),
                Constraint::Length(5),
                Constraint::Min(16),
            ],
        )
        .header(
            Row::new([
                app.t("session", "会话"),
                app.t("health", "健康"),
                app.t("source", "来源"),
                app.t("model", "模型"),
                app.t("cost", "成本"),
                app.t("tokens", "令牌"),
                "ok%",
                app.t("fail", "失败"),
                app.t("anom", "异常"),
                app.t("reason", "原因"),
            ])
            .style(
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
        )
        .block(Block::default().borders(Borders::ALL).title(title))
        .row_highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("> ")
    };
    frame.render_stateful_widget(table, area, &mut app.table_state);
}

fn render_list_status(frame: &mut Frame<'_>, app: &App, area: Rect, active_filters: &str) {
    let filter = if active_filters.is_empty() {
        app.t("none", "无").to_string()
    } else {
        active_filters.to_string()
    };
    let hint = if active_filters.is_empty() {
        app.t(
            "Enter detail | 3 diagnostics | 4 diff",
            "Enter 详情 | 3 诊断 | 4 对比",
        )
    } else {
        app.t("Esc/:clear resets filters", "Esc/:clear 重置筛选")
    };
    let text = format!(
        "{}/{} {}  {}: {}  {}: {} {}  {}",
        app.filtered.len(),
        app.sessions.len(),
        app.t("visible", "可见"),
        app.t("filters", "筛选"),
        filter,
        app.t("sort", "排序"),
        sort_key_label(app.sort_key, app.language),
        if app.sort_desc {
            app.t("desc", "降序")
        } else {
            app.t("asc", "升序")
        },
        hint
    );
    frame.render_widget(
        Paragraph::new(text)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(app.t("List Status", "列表状态")),
            )
            .wrap(Wrap { trim: true }),
        area,
    );
}

fn render_loading_status(frame: &mut Frame<'_>, app: &App, area: Rect) {
    frame.render_widget(
        Paragraph::new(loading_status_lines(app))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(app.t("Loading Status", "加载状态")),
            )
            .wrap(Wrap { trim: true }),
        area,
    );
}

fn render_driver_summary(frame: &mut Frame<'_>, app: &App, area: Rect) {
    let visible = app.visible_sessions();
    let total = visible.len();
    let text = vec![
        Line::from(format!(
            "{}: {} {}",
            app.t("Visible", "可见"),
            format_count(total as i64),
            app.t("sessions", "会话")
        )),
        Line::from(driver_summary_line(
            app.t("Source", "来源"),
            top_driver(&visible, driver_source),
            total,
        )),
        Line::from(driver_summary_line(
            app.t("Model", "模型"),
            top_driver(&visible, driver_model),
            total,
        )),
        Line::from(driver_summary_line(
            app.t("Anomaly", "异常"),
            top_anomaly_driver(&visible),
            total,
        )),
    ];
    frame.render_widget(
        Paragraph::new(text)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(app.t("Driver Summary", "驱动摘要")),
            )
            .wrap(Wrap { trim: true }),
        area,
    );
}

fn render_selected_summary(frame: &mut Frame<'_>, app: &App, area: Rect) {
    let text = if let Some(session) = app.selected_session() {
        vec![
            Line::from(format!(
                "{}: {}  {}={}  ok={:.0}%  {}={}  {}={}  {}={}  {}=${:.4}",
                app.t("selected", "选中"),
                short(&session.name, 24),
                app.t("reason", "原因"),
                short(&triage_reason(session, app.language), 22),
                tool_success_rate(session),
                app.t("fail", "失败"),
                session.metrics.tool_calls_fail,
                app.t("anom", "异常"),
                session.anomalies.len(),
                app.t("health", "健康度"),
                session.health,
                app.t("cost", "成本"),
                session.metrics.cost_estimated
            )),
            Line::from(format!(
                "{}={}  {}={}  {}={}  {}={}  p95 {}={}",
                app.t("source", "来源"),
                short(&display_session_source(session), 18),
                app.t("model", "模型"),
                short(&driver_model(session), 24),
                app.t("tokens", "令牌"),
                format_count(total_tokens(session)),
                app.t("elapsed", "耗时"),
                format_duration(session.metrics.duration_sec),
                app.t("latency", "延迟"),
                format_duration(session_p95_gap(session))
            )),
            Line::from(format!(
                "{}={}",
                app.t("action", "动作"),
                selected_next_action(session, app.language)
            )),
        ]
    } else {
        vec![Line::from(app.t("selected: none", "未选中会话"))]
    };
    frame.render_widget(
        Paragraph::new(text)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(app.t("Selected Triage", "选中分诊")),
            )
            .wrap(Wrap { trim: true }),
        area,
    );
}

fn render_report(
    frame: &mut Frame<'_>,
    app: &App,
    area: Rect,
    title: impl Into<String>,
    text: String,
) {
    let text = terminal_safe_report(&text);
    frame.render_widget(
        Paragraph::new(text)
            .block(Block::default().borders(Borders::ALL).title(title.into()))
            .scroll((app.scroll, 0))
            .wrap(Wrap { trim: false }),
        area,
    );
}

fn render_footer(frame: &mut Frame<'_>, app: &App, area: Rect) {
    let prompt = match app.mode {
        InputMode::Search => format!("/ {}", app.input),
        InputMode::Command => format!(": {}", app.input),
        InputMode::Normal => {
            let base = if area.width >= 118 {
                app.t(
                    "q quit | tab view | j/k select | enter inspect/detail | / search | f/s/$/! filters | h/c/t/e/a/n sort | r/^R reload | : cmd | ? help | l language",
                    "q 退出 | tab 视图 | j/k 选择 | enter 检查/详情 | / 搜索 | f/s/$/! 筛选 | h/c/t/e/a/n 排序 | r/^R 重载 | : 命令 | ? 帮助 | l 语言",
                )
            } else if area.width >= 84 {
                app.t(
                    "q quit | tab view | j/k move | enter inspect/detail | / search | ! critical | $ cost | ? help | l language",
                    "q 退出 | tab 视图 | j/k 移动 | enter 检查/详情 | / 搜索 | ! 严重 | $ 成本 | ? 帮助 | l 语言",
                )
            } else {
                app.t(
                    "q quit | tab view | enter inspect | / search | ! critical | ? help | l language",
                    "q 退出 | tab 视图 | enter 检查 | / 搜索 | ! 严重 | ? 帮助 | l 语言",
                )
            };
            if app.status.is_empty() {
                base.to_string()
            } else {
                format!("{} | {base}", short(&app.status, status_width(area.width)))
            }
        }
    };
    frame.render_widget(
        Paragraph::new(prompt).block(Block::default().borders(Borders::ALL)),
        area,
    );
}

fn report_title(app: &App, base: &str) -> String {
    let Some(session) = app.selected_session() else {
        return base.to_string();
    };
    format!(
        "{} - {} {}={}",
        base,
        short(&session.name, 18),
        app.t("reason", "原因"),
        short(&triage_reason(session, app.language), 18)
    )
}

fn diff_title(app: &App) -> String {
    let active_filters = active_filter_summary(app);
    if active_filters.is_empty() {
        format!(
            "{} - {} {} - {} {} {}",
            app.t("Diff", "对比"),
            app.filtered.len(),
            app.t("visible", "可见"),
            app.t("sort", "排序"),
            sort_key_label(app.sort_key, app.language),
            if app.sort_desc {
                app.t("desc", "降序")
            } else {
                app.t("asc", "升序")
            }
        )
    } else {
        format!(
            "{} - {} {} - {} {} - {} {} {}",
            app.t("Diff", "对比"),
            app.filtered.len(),
            app.t("visible", "可见"),
            app.t("filter", "筛选"),
            active_filters,
            app.t("sort", "排序"),
            sort_key_label(app.sort_key, app.language),
            if app.sort_desc {
                app.t("desc", "降序")
            } else {
                app.t("asc", "升序")
            }
        )
    }
}

fn detail_text(app: &App) -> String {
    app.selected_session()
        .map(|session| {
            report_with_context(
                detail_native_text(session, app.language),
                app.t("Raw report", "原始报告"),
                report_text(session),
            )
        })
        .unwrap_or_else(|| app.t("No selected session.", "未选中会话。").to_string())
}

fn diagnostics_text(app: &App) -> String {
    app.selected_session()
        .map(|session| {
            report_with_context(
                diagnostics_native_text(session, app.language),
                app.t("Raw diagnostics", "原始诊断"),
                render_waste_report(session),
            )
        })
        .unwrap_or_else(|| app.t("No selected session.", "未选中会话。").to_string())
}

fn report_with_context(summary: String, raw_title: &str, report: String) -> String {
    format!(
        "{}\n\n{}\n{}\n{}",
        summary,
        raw_title,
        "-".repeat(raw_title.len()),
        report
    )
}

fn report_context_line(session: &Session, language: Language) -> String {
    format!(
        "{}: {}={} {}={} {}=${:.4} {}={} {}={} {}={}",
        text(language, "Context", "上下文"),
        text(language, "reason", "原因"),
        triage_reason(session, language),
        text(language, "health", "健康度"),
        session.health,
        text(language, "cost", "成本"),
        session.metrics.cost_estimated,
        text(language, "fail", "失败"),
        session.metrics.tool_calls_fail,
        text(language, "anom", "异常"),
        session.anomalies.len(),
        text(language, "source", "来源"),
        display_session_source(session)
    )
}

fn detail_native_text(session: &Session, language: Language) -> String {
    let metrics = &session.metrics;
    let mut lines = vec![
        text(language, "Session Summary", "会话摘要").to_string(),
        "---------------".to_string(),
        report_context_line(session, language),
        format!("{}: {}", text(language, "Name", "名称"), session.name),
        format!("{}: {}", text(language, "Path", "路径"), session.path),
        format!(
            "{}: {}",
            text(language, "CWD", "工作目录"),
            if session.cwd.is_empty() {
                text(language, "unknown", "未知")
            } else {
                &session.cwd
            }
        ),
        format!(
            "{}: {}={} {}={}",
            text(language, "Driver", "驱动"),
            text(language, "source", "来源"),
            display_session_source(session),
            text(language, "model", "模型"),
            driver_model(session)
        ),
        format!(
            "{}: {}={} {}={} {}={} p95_gap={}",
            text(language, "Timeline", "时间线"),
            text(language, "start", "开始"),
            empty_as_unknown(&metrics.session_start),
            text(language, "end", "结束"),
            empty_as_unknown(&metrics.session_end),
            text(language, "elapsed", "耗时"),
            format_duration(metrics.duration_sec),
            format_duration(session_p95_gap(session))
        ),
        format!(
            "{}: {}={} user={} assistant={} tool_results={}",
            text(language, "Turns", "轮次"),
            text(language, "events", "事件"),
            format_count(metrics.events_total as i64),
            format_count(metrics.user_messages as i64),
            format_count(metrics.assistant_turns as i64),
            format_count(metrics.tool_results as i64)
        ),
        format!(
            "{}: {}={} {}={} {}={:.0}%",
            text(language, "Tools", "工具"),
            text(language, "total", "总数"),
            format_count(metrics.tool_calls_total as i64),
            text(language, "failed", "失败"),
            format_count(metrics.tool_calls_fail as i64),
            text(language, "success", "成功率"),
            tool_success_rate(session)
        ),
        format!(
            "{}: input={} output={} cache_write={} cache_read={} {}={}",
            text(language, "Tokens", "令牌"),
            format_count(metrics.tokens_input),
            format_count(metrics.tokens_output),
            format_count(metrics.tokens_cache_w),
            format_count(metrics.tokens_cache_r),
            text(language, "total", "总数"),
            format_count(total_tokens(session))
        ),
        format!(
            "{}: ${:.4}",
            text(language, "Cost", "成本"),
            metrics.cost_estimated
        ),
        String::new(),
        text(language, "Next Action", "下一步动作").to_string(),
        "-----------".to_string(),
        format!("- {}", selected_next_action(session, language)),
    ];
    lines.extend(signal_lines(session, language));
    lines.push(String::new());
    lines.extend(anomaly_lines(session, 4, language));
    lines.join("\n")
}

fn diagnostics_native_text(session: &Session, language: Language) -> String {
    let metrics = &session.metrics;
    let mut lines = vec![
        text(language, "Problem", "问题").to_string(),
        "-------".to_string(),
        report_context_line(session, language),
        String::new(),
        text(language, "Evidence", "证据").to_string(),
        "--------".to_string(),
        format!(
            "{}={} {}={}",
            text(language, "health", "健康度"),
            session.health,
            text(language, "reason", "原因"),
            triage_reason(session, language)
        ),
        format!(
            "{}={} {}={}",
            text(language, "source", "来源"),
            display_session_source(session),
            text(language, "model", "模型"),
            driver_model(session)
        ),
        format!(
            "{}={} p95_gap={} {}={} {}={}",
            text(language, "duration", "时长"),
            format_duration(metrics.duration_sec),
            format_duration(session_p95_gap(session)),
            text(language, "failures", "失败"),
            format_count(metrics.tool_calls_fail as i64),
            text(language, "anomalies", "异常"),
            format_count(session.anomalies.len() as i64)
        ),
        format!(
            "{}=${:.4} {}={} cache_read_share={} tool_success={:.0}%",
            text(language, "cost", "成本"),
            metrics.cost_estimated,
            text(language, "tokens", "令牌"),
            format_count(total_tokens(session)),
            token_share(metrics.tokens_cache_r, total_tokens(session)),
            tool_success_rate(session)
        ),
        String::new(),
        text(language, "Next", "下一步").to_string(),
        "----".to_string(),
    ];
    lines.extend(
        diagnostic_actions(session, language)
            .into_iter()
            .take(4)
            .map(|line| format!("- {line}")),
    );
    lines.push(String::new());
    lines.push(text(language, "Raw Signals", "原始信号").to_string());
    lines.push("-----------".to_string());
    lines.extend(signal_lines(session, language).into_iter().take(5));
    lines.extend(anomaly_lines(session, 6, language));
    lines.join("\n")
}

fn diagnostic_actions(session: &Session, language: Language) -> Vec<String> {
    let mut actions = Vec::new();
    actions.push(selected_next_action(session, language));
    if session.health < 50 {
        actions.push(
            text(
                language,
                "check failed tool calls and high-severity anomalies before cost tuning",
                "先检查失败工具调用和高严重度异常，再看成本优化",
            )
            .to_string(),
        );
    }
    if session.metrics.tool_calls_fail > 0 {
        actions.push(
            text(
                language,
                "filter by failed tools or inspect raw diagnostics for tool errors",
                "按失败工具筛选，或查看原始诊断里的工具错误",
            )
            .to_string(),
        );
    }
    if !session.anomalies.is_empty() {
        actions.push(
            text(
                language,
                "review anomaly details and compare against nearby healthy sessions",
                "查看异常详情，并和相近的健康会话对比",
            )
            .to_string(),
        );
    }
    if session.metrics.cost_estimated >= 1.0 {
        actions.push(
            text(
                language,
                "open Diff to compare cost drivers against cheaper sessions",
                "打开对比视图，和更低成本会话比较成本来源",
            )
            .to_string(),
        );
    }
    if actions.len() == 1 {
        actions.push(
            text(
                language,
                "use Raw report when you need the full narrative",
                "需要完整叙事时查看原始报告",
            )
            .to_string(),
        );
    }
    actions
}

fn signal_lines(session: &Session, language: Language) -> Vec<String> {
    let metrics = &session.metrics;
    let mut lines = Vec::new();
    if let Some(line) = top_usage_line(
        text(language, "Top tool", "最高频工具"),
        &metrics.tool_usage,
        42,
    ) {
        lines.push(line);
    }
    if let Some(line) = top_usage_line(
        text(language, "Top file", "最高频文件"),
        &metrics.file_usage,
        42,
    ) {
        lines.push(line);
    }
    if let Some(line) = top_usage_line(
        text(language, "Top arg", "最高频参数"),
        &metrics.tool_arg_usage,
        42,
    ) {
        lines.push(line);
    }
    if let Some(line) = top_usage_line(
        text(language, "Authority", "权限"),
        &metrics.tool_authority,
        42,
    ) {
        lines.push(line);
    }
    if !metrics.highest_authority.is_empty() {
        lines.push(format!(
            "{}: {}",
            text(language, "Highest authority", "最高权限"),
            metrics.highest_authority
        ));
    }
    if metrics.reasoning_blocks > 0 {
        lines.push(format!(
            "{}: {}={} chars={} redacted={}",
            text(language, "Reasoning", "推理"),
            text(language, "blocks", "块"),
            format_count(metrics.reasoning_blocks as i64),
            format_count(metrics.reasoning_chars as i64),
            format_count(metrics.reasoning_redact as i64)
        ));
    }
    if lines.is_empty() {
        lines.push(
            text(
                language,
                "Signals: no tool/file hotspots recorded",
                "信号：未记录工具/文件热点",
            )
            .to_string(),
        );
    }
    lines
}

fn top_usage_line(label: &str, usage: &BTreeMap<String, usize>, max_name: usize) -> Option<String> {
    usage
        .iter()
        .max_by(|(left_name, left_count), (right_name, right_count)| {
            left_count
                .cmp(right_count)
                .then_with(|| right_name.cmp(left_name))
        })
        .map(|(name, count)| {
            format!(
                "{label}: {} ({})",
                short(name, max_name),
                format_count(*count as i64)
            )
        })
}

fn anomaly_lines(session: &Session, limit: usize, language: Language) -> Vec<String> {
    let mut lines = vec![
        text(language, "Anomalies", "异常").to_string(),
        "---------".to_string(),
    ];
    if session.anomalies.is_empty() {
        lines.push(format!("- {}", text(language, "none", "无")));
        return lines;
    }
    for anomaly in session.anomalies.iter().take(limit) {
        lines.push(format!(
            "- {} {}: {}",
            empty_as_unknown(&anomaly.severity),
            empty_as_unknown(&anomaly.kind),
            empty_as_unknown(&anomaly.detail)
        ));
    }
    if session.anomalies.len() > limit {
        lines.push(format!(
            "- ... {} {}",
            format_count((session.anomalies.len() - limit) as i64),
            text(language, "more", "更多")
        ));
    }
    lines
}

fn token_share(part: i64, total: i64) -> String {
    if total <= 0 || part <= 0 {
        return "0%".to_string();
    }
    format!("{:.0}%", (part as f64 / total as f64) * 100.0)
}

fn empty_as_unknown(value: &str) -> &str {
    if value.is_empty() {
        "unknown"
    } else {
        value
    }
}

fn diff_text(app: &App) -> String {
    let sessions = app.visible_sessions();
    let context = diff_context_line(app, sessions.len());
    if sessions.len() < 2 {
        let filters = active_filter_summary(app);
        let filter_hint = if filters.is_empty() {
            format!(
                "{}: {}",
                app.t("Active filters", "当前筛选"),
                app.t("none", "无")
            )
        } else {
            format!("{}: {filters}", app.t("Active filters", "当前筛选"))
        };
        return format!(
            "{context}\n\n{}\n{filter_hint}\n{}",
            app.t(
                "Need at least two visible sessions for diff.",
                "至少需要两个可见会话才能对比。"
            ),
            app.t(
                "Press Esc or run :clear/:reset to broaden the comparison set.",
                "按 Esc 或运行 :clear/:reset 扩大对比范围。"
            )
        );
    }
    format!("{context}\n\n{}", report_compare(&sessions, "default"))
}

fn diff_context_line(app: &App, visible_count: usize) -> String {
    let filters = active_filter_summary(app);
    let filter_text = if filters.is_empty() {
        app.t("none", "无").to_string()
    } else {
        filters
    };
    let top_source = top_driver(&app.visible_sessions(), driver_source)
        .map(|item| format!("{}:{}", item.label, item.sessions))
        .unwrap_or_else(|| app.t("none", "无").to_string());
    format!(
        "{}: {}={} {}={} {}={} {} top_source={}",
        app.t("Context", "上下文"),
        app.t("visible", "可见"),
        visible_count,
        app.t("filter", "筛选"),
        filter_text,
        app.t("sort", "排序"),
        sort_key_label(app.sort_key, app.language),
        if app.sort_desc {
            app.t("desc", "降序")
        } else {
            app.t("asc", "升序")
        },
        top_source
    )
}

fn help_text(language: Language) -> String {
    match language {
        Language::En => [
            "Triage workflow",
            "  Start on Overview. Inspect First ranks the sessions most worth opening.",
            "  enter on Overview opens the top Inspect First item.",
            "  l switches language between English and Chinese.",
            "  ! critical sessions, $ costly sessions, f cycles health filters",
            "  enter outside Overview opens detail, 3 opens diagnostics for the selected session",
            "",
            "Navigation",
            "  0 overview, 1 list, 2 detail, 3 diagnostics, 4 diff, tab next view",
            "  j/k or arrows move selection; page up/down scroll detail panels",
            "",
            "Filters and sorting",
            "  / text, Esc clear, s selected source, h health sort, c cost sort",
            "  t turns, e failures, n name, a anomalies",
            "",
            "Command mode",
            "  :overview, :list, :detail, :diagnostics, :diff, :inspect [rank], :search <text>, :clear/:reset, :reload, :quit",
            "  :health good|warn|crit|<80, :source <name>, :model <name>, :cost >0.10",
            "  :anomaly [type], :critical, :top cost|failures|source, :sort <field> [asc|desc]",
            "",
            "Automation",
            "  agenttrace --overview -f json",
            "  agenttrace --overview -f html -o agenttrace-overview.html",
        ]
        .join("\n"),
        Language::Zh => [
            "分诊流程",
            "  默认从概览开始。优先检查会把最值得打开的会话排在前面。",
            "  在概览按 enter 会打开优先检查的第一项。",
            "  按 l 在英文和中文之间切换。",
            "  ! 筛严重会话，$ 筛高成本会话，f 循环健康度筛选。",
            "  在其它视图按 enter 打开详情，按 3 打开选中会话的诊断。",
            "",
            "导航",
            "  0 概览，1 列表，2 详情，3 诊断，4 对比，tab 下一个视图。",
            "  j/k 或方向键移动选择；page up/down 滚动详情面板。",
            "",
            "筛选和排序",
            "  / 文本，Esc 清除，s 选中来源，h 健康度排序，c 成本排序。",
            "  t 轮次，e 失败，n 名称，a 异常。",
            "",
            "命令模式",
            "  :overview, :list, :detail, :diagnostics, :diff, :inspect [rank], :search <text>, :clear/:reset, :reload, :quit",
            "  :health good|warn|crit|<80, :source <name>, :model <name>, :cost >0.10",
            "  :anomaly [type], :critical, :top cost|failures|source, :sort <field> [asc|desc]",
            "",
            "自动化",
            "  agenttrace --overview -f json",
            "  agenttrace --overview -f html -o agenttrace-overview.html",
        ]
        .join("\n"),
    }
}

fn collect_tui_session_files(dir: &str) -> Vec<PathBuf> {
    if dir.trim().is_empty() {
        return discover_session_dirs()
            .iter()
            .flat_map(|dir| collect_session_files(dir))
            .collect();
    }
    collect_session_files(Path::new(dir))
}

fn count_cache_hits(files: &[PathBuf]) -> usize {
    let Ok(raw) = std::fs::read_to_string(session_cache_path()) else {
        return 0;
    };
    files
        .iter()
        .filter(|path| raw.contains(&path.to_string_lossy().to_string()))
        .count()
}

fn cache_state_label() -> String {
    match session_cache_path().metadata() {
        Ok(metadata) if metadata.len() > 0 => "cache warm".to_string(),
        Ok(_) => "cache empty".to_string(),
        Err(_) => "cache empty".to_string(),
    }
}

fn source_counts(sessions: &[Session]) -> Vec<(String, usize)> {
    let mut counts: BTreeMap<String, usize> = BTreeMap::new();
    for session in sessions {
        *counts.entry(driver_source(session)).or_default() += 1;
    }
    let mut items = counts.into_iter().collect::<Vec<_>>();
    items.sort_by(|left, right| right.1.cmp(&left.1).then_with(|| left.0.cmp(&right.0)));
    items
}

fn loading_status_lines(app: &App) -> Vec<Line<'static>> {
    let state = &app.load_state;
    let mode = if state.force {
        app.t("force reload", "强制重载")
    } else {
        app.t("normal load", "正常加载")
    };
    let parsed = if state.phase == LoadPhase::Ready {
        state.parsed
    } else {
        state.parsed.min(app.sessions.len())
    };
    let source_text = if state.sources.is_empty() {
        format!("{}={}", app.t("sources", "来源"), app.t("none", "无"))
    } else {
        format!(
            "{}={}",
            app.t("sources", "来源"),
            state
                .sources
                .iter()
                .take(4)
                .map(|(source, count)| format!("{}:{count}", short(source, 18)))
                .collect::<Vec<_>>()
                .join(",")
        )
    };
    vec![
        Line::from(format!(
            "{} - {} {} {}",
            load_phase_label(state.phase, app.language),
            mode,
            app.t("from", "来自"),
            short(&display_source_label(&state.source), 36)
        )),
        Line::from(format!(
            "{} {}/{} {}, {} {}, {}",
            app.t("loaded", "已加载"),
            format_count(parsed as i64),
            format_count(state.discovered as i64),
            app.t("files", "个文件"),
            format_count(state.cache_hits as i64),
            app.t("cache hits", "缓存命中"),
            state.cache_state
        )),
        Line::from(source_text),
    ]
}

fn load_summary_line(app: &App) -> String {
    let state = &app.load_state;
    match state.phase {
        LoadPhase::Idle => app.t("idle", "空闲").to_string(),
        LoadPhase::Discovering => format!(
            "{} {} {}",
            app.t("discovering", "发现中"),
            format_count(state.discovered as i64),
            app.t("files", "个文件")
        ),
        LoadPhase::Parsing => format!(
            "{} {} {}, {} {}",
            app.t("loading", "加载中"),
            format_count(state.discovered as i64),
            app.t("files", "个文件"),
            format_count(state.cache_hits as i64),
            app.t("cache hits", "缓存命中")
        ),
        LoadPhase::Ready => {
            let source = state
                .sources
                .first()
                .map(|(source, count)| {
                    format!(
                        "{}:{}",
                        display_source_label(source),
                        format_count(*count as i64)
                    )
                })
                .unwrap_or_else(|| app.t("none", "无").to_string());
            format!(
                "{} {} {}, {} {}, {source}",
                app.t("loaded", "已加载"),
                format_count(state.parsed as i64),
                app.t("sessions", "个会话"),
                format_count(state.cache_hits as i64),
                app.t("cache hits", "缓存命中")
            )
        }
        LoadPhase::Failed => app.t("load failed", "加载失败").to_string(),
    }
}

fn load_phase_label(phase: LoadPhase, language: Language) -> &'static str {
    match phase {
        LoadPhase::Idle => text(language, "Idle", "空闲"),
        LoadPhase::Discovering => text(language, "Discovering", "发现中"),
        LoadPhase::Parsing => text(language, "Loading", "加载中"),
        LoadPhase::Ready => text(language, "Ready", "就绪"),
        LoadPhase::Failed => text(language, "Failed", "失败"),
    }
}

fn top_group(
    groups: &std::collections::BTreeMap<String, agenttrace_core::GroupOverview>,
) -> Option<(&String, &agenttrace_core::GroupOverview)> {
    groups
        .iter()
        .max_by(|(left_name, left), (right_name, right)| {
            left.sessions
                .cmp(&right.sessions)
                .then_with(|| cmp_f64(left.cost, right.cost))
                .then_with(|| right_name.cmp(left_name))
        })
}

fn top_model_line(app: &App) -> String {
    if let Some((model, group)) = top_group(&app.overview.by_model) {
        format!(
            "{} {}  {} {}  {} ${:.4}",
            app.t("top model", "最高频模型"),
            short(model, 24),
            app.t("sessions", "会话"),
            group.sessions,
            app.t("cost", "成本"),
            group.cost
        )
    } else {
        format!(
            "{} {}",
            app.t("top model", "最高频模型"),
            app.t("none", "无")
        )
    }
}

fn health_color(health: i32) -> Color {
    if health >= 80 {
        Color::Gray
    } else if health >= 50 {
        Color::Yellow
    } else {
        Color::LightRed
    }
}

fn session_row_style(session: &Session) -> Style {
    if session.health < 50 {
        Style::default().fg(Color::LightRed)
    } else if session.metrics.tool_calls_fail > 0 || !session.anomalies.is_empty() {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default().fg(Color::Gray)
    }
}

fn priority_color(app: &App) -> Color {
    if app.overview.critical > 0 {
        Color::LightRed
    } else if app.overview.warning > 0 {
        Color::Yellow
    } else {
        Color::LightGreen
    }
}

fn health_label(health: i32, language: Language) -> String {
    if health >= 80 {
        format!("{health} {}", text(language, "ok", "良好"))
    } else if health >= 50 {
        format!("{health} {}", text(language, "warn", "警告"))
    } else {
        format!("{health} {}", text(language, "crit", "严重"))
    }
}

fn session_table_title(app: &App, active_filters: &str) -> String {
    if active_filters.is_empty() {
        format!(
            "{} - {} {} - {} {} {}",
            app.t("Sessions", "会话"),
            app.filtered.len(),
            app.t("visible", "可见"),
            app.t("sort", "排序"),
            sort_key_label(app.sort_key, app.language),
            if app.sort_desc {
                app.t("desc", "降序")
            } else {
                app.t("asc", "升序")
            }
        )
    } else {
        format!(
            "{} - {} {} - {} {} - {} {} {}",
            app.t("Sessions", "会话"),
            app.filtered.len(),
            app.t("visible", "可见"),
            app.t("filters", "筛选"),
            active_filters,
            app.t("sort", "排序"),
            sort_key_label(app.sort_key, app.language),
            if app.sort_desc {
                app.t("desc", "降序")
            } else {
                app.t("asc", "升序")
            }
        )
    }
}

fn recent_limit(height: u16) -> usize {
    height.saturating_sub(2).max(1) as usize
}

#[derive(Debug, Clone, Default, PartialEq)]
struct DriverItem {
    label: String,
    sessions: usize,
    failures: usize,
    tokens: i64,
    cost: f64,
}

#[derive(Debug, Clone, PartialEq)]
struct InspectFirstItem {
    label: &'static str,
    index: usize,
}

fn top_driver(sessions: &[Session], label: fn(&Session) -> String) -> Option<DriverItem> {
    let mut groups: BTreeMap<String, DriverItem> = BTreeMap::new();
    for session in sessions {
        let label = label(session);
        let entry = groups.entry(label.clone()).or_insert_with(|| DriverItem {
            label,
            ..DriverItem::default()
        });
        entry.sessions += 1;
        entry.failures += session.metrics.tool_calls_fail;
        entry.tokens += total_tokens(session);
        entry.cost += session.metrics.cost_estimated;
    }
    groups.into_values().max_by(compare_driver_items)
}

fn top_anomaly_driver(sessions: &[Session]) -> Option<DriverItem> {
    let mut groups: BTreeMap<String, DriverItem> = BTreeMap::new();
    for session in sessions {
        let mut seen = BTreeMap::new();
        for anomaly in &session.anomalies {
            seen.insert(anomaly.kind.clone(), ());
        }
        for label in seen.keys() {
            let entry = groups.entry(label.clone()).or_insert_with(|| DriverItem {
                label: label.clone(),
                ..DriverItem::default()
            });
            entry.sessions += 1;
            entry.failures += session.metrics.tool_calls_fail;
            entry.tokens += total_tokens(session);
            entry.cost += session.metrics.cost_estimated;
        }
    }
    groups.into_values().max_by(compare_driver_items)
}

fn compare_driver_items(left: &DriverItem, right: &DriverItem) -> Ordering {
    left.sessions
        .cmp(&right.sessions)
        .then_with(|| left.failures.cmp(&right.failures))
        .then_with(|| cmp_f64(left.cost, right.cost))
        .then_with(|| right.label.cmp(&left.label))
}

fn inspect_first_lines(app: &App, width: u16) -> Vec<Line<'static>> {
    let mut lines = vec![Line::from(app.t(
        "rank  target                  open          why",
        "排名  目标                    打开          原因",
    ))];
    let items = inspect_first_items(&app.sessions);
    if items.is_empty() {
        lines.push(Line::from(
            app.t("no priority sessions", "没有需要优先检查的会话"),
        ));
        return lines;
    }
    let name_width = if width >= 70 { 24 } else { 18 };
    for (rank, item) in items.into_iter().take(4).enumerate() {
        let Some(session) = app.sessions.get(item.index) else {
            continue;
        };
        lines.push(Line::from(vec![
            Span::styled(
                format!("{:<5}", rank + 1),
                Style::default()
                    .fg(if rank == 0 { Color::Cyan } else { Color::Gray })
                    .add_modifier(if rank == 0 {
                        Modifier::BOLD
                    } else {
                        Modifier::empty()
                    }),
            ),
            Span::raw(format!(
                "{:<name_width$} ",
                short(&session.name, name_width),
                name_width = name_width
            )),
            Span::styled(
                format!("{:<12}", inspect_open_label(item.label, app.language)),
                Style::default().fg(inspect_label_color(item.label)),
            ),
            Span::raw(short(&triage_reason(session, app.language), 28)),
        ]));
        lines.push(Line::from(vec![
            Span::raw(format!("      {}=", app.t("health", "健康度"))),
            Span::styled(
                format!("{:<3} ", session.health),
                Style::default().fg(health_color(session.health)),
            ),
            Span::raw(format!(
                "${:<8.4} {}: {}",
                session.metrics.cost_estimated,
                app.t("action", "动作"),
                short(
                    &selected_next_action(session, app.language),
                    width.saturating_sub(28).max(24) as usize
                )
            )),
        ]));
    }
    lines
}

fn inspect_first_items(sessions: &[Session]) -> Vec<InspectFirstItem> {
    let mut items = Vec::new();
    let mut seen = BTreeMap::new();
    push_inspect_item(
        &mut items,
        &mut seen,
        "critical",
        inspect_by_where(
            sessions,
            |session| session.health < 50,
            |left, right| {
                right
                    .health
                    .cmp(&left.health)
                    .then_with(|| {
                        cmp_f64(left.metrics.cost_estimated, right.metrics.cost_estimated)
                    })
                    .then_with(|| total_tokens(left).cmp(&total_tokens(right)))
            },
        ),
    );
    push_inspect_item(
        &mut items,
        &mut seen,
        "anomaly",
        inspect_by_where(
            sessions,
            |session| !session.anomalies.is_empty(),
            |left, right| {
                left.anomalies
                    .len()
                    .cmp(&right.anomalies.len())
                    .then_with(|| right.health.cmp(&left.health))
                    .then_with(|| {
                        cmp_f64(left.metrics.cost_estimated, right.metrics.cost_estimated)
                    })
            },
        ),
    );
    push_inspect_item(
        &mut items,
        &mut seen,
        "failures",
        inspect_by_where(
            sessions,
            |session| session.metrics.tool_calls_fail > 0,
            |left, right| {
                left.metrics
                    .tool_calls_fail
                    .cmp(&right.metrics.tool_calls_fail)
                    .then_with(|| right.health.cmp(&left.health))
                    .then_with(|| {
                        cmp_f64(left.metrics.cost_estimated, right.metrics.cost_estimated)
                    })
            },
        ),
    );
    push_inspect_item(
        &mut items,
        &mut seen,
        "cost",
        inspect_by(sessions, |left, right| {
            cmp_f64(left.metrics.cost_estimated, right.metrics.cost_estimated)
                .then_with(|| total_tokens(left).cmp(&total_tokens(right)))
        }),
    );
    push_inspect_item(
        &mut items,
        &mut seen,
        "latency",
        inspect_by_where(
            sessions,
            |session| session.metrics.duration_sec > 0.0 || session_p95_gap(session) > 0.0,
            |left, right| {
                cmp_f64(left.metrics.duration_sec, right.metrics.duration_sec)
                    .then_with(|| cmp_f64(session_p95_gap(left), session_p95_gap(right)))
                    .then_with(|| {
                        cmp_f64(left.metrics.cost_estimated, right.metrics.cost_estimated)
                    })
            },
        ),
    );
    items
}

fn inspect_by(sessions: &[Session], compare: fn(&Session, &Session) -> Ordering) -> Option<usize> {
    inspect_by_where(sessions, |_| true, compare)
}

fn inspect_by_where(
    sessions: &[Session],
    include: fn(&Session) -> bool,
    compare: fn(&Session, &Session) -> Ordering,
) -> Option<usize> {
    sessions
        .iter()
        .enumerate()
        .filter(|(_, session)| include(session))
        .max_by(|(left_idx, left), (right_idx, right)| {
            compare(left, right).then_with(|| right_idx.cmp(left_idx))
        })
        .map(|(idx, _)| idx)
}

fn push_inspect_item(
    items: &mut Vec<InspectFirstItem>,
    seen: &mut BTreeMap<usize, ()>,
    label: &'static str,
    index: Option<usize>,
) {
    let Some(index) = index else {
        return;
    };
    if seen.insert(index, ()).is_some() {
        return;
    }
    items.push(InspectFirstItem { label, index });
}

fn inspect_target_view(label: &str) -> View {
    match label {
        "cost" => View::Detail,
        _ => View::Diagnostics,
    }
}

fn inspect_open_label(label: &str, language: Language) -> &'static str {
    match inspect_target_view(label) {
        View::Detail => text(language, "Detail", "详情"),
        View::Diagnostics => text(language, "Diagnostics", "诊断"),
        _ => text(language, "Open", "打开"),
    }
}

fn inspect_label_color(label: &str) -> Color {
    match label {
        "critical" | "failures" => Color::LightRed,
        "anomaly" | "latency" => Color::Yellow,
        "cost" => Color::LightMagenta,
        _ => Color::Gray,
    }
}

fn driver_source(session: &Session) -> String {
    if session.metrics.source_tool.is_empty() {
        "unknown".to_string()
    } else {
        display_source_label(&session.metrics.source_tool)
    }
}

fn display_session_source(session: &Session) -> String {
    driver_source(session)
}

fn display_source_label(source: &str) -> String {
    let source = source.trim();
    if source.is_empty() || source == "auto-discovery" {
        return "auto discovery".to_string();
    }
    if source == "pi" || source.ends_with("/.pi/agent/sessions") {
        return "Pi sessions".to_string();
    }
    if source == "oh_my_pi" || source.ends_with("/.omp/agent/sessions") {
        return "Oh My Pi sessions".to_string();
    }
    if source == "claude_code" || source.ends_with("/.claude/projects") {
        return "Claude Code".to_string();
    }
    if source == "codex_cli" || source.contains("/.codex/") {
        return "Codex".to_string();
    }
    if source == "hermes_db" || source.ends_with("/.hermes/state.db") {
        return "Hermes DB".to_string();
    }
    if source == "opencode_db" || source.ends_with("/opencode.db") {
        return "OpenCode DB".to_string();
    }
    if source.contains('/') {
        return source
            .rsplit('/')
            .find(|part| !part.is_empty())
            .unwrap_or(source)
            .to_string();
    }
    source.to_string()
}

fn driver_model(session: &Session) -> String {
    if session.metrics.model_used.is_empty() {
        "unknown".to_string()
    } else {
        session.metrics.model_used.clone()
    }
}

fn driver_summary_line(label: &str, item: Option<DriverItem>, total_sessions: usize) -> String {
    let Some(item) = item else {
        return format!("{label:<7} none");
    };
    let pct = (item.sessions * 100)
        .checked_div(total_sessions)
        .unwrap_or(0);
    format!(
        "{label:<7} {}  {}/{} {}%  fail{}  {}",
        short(&item.label, 18),
        item.sessions,
        total_sessions,
        pct,
        item.failures,
        format_compact_cost(item.cost)
    )
}

fn format_compact_cost(cost: f64) -> String {
    if !cost.is_finite() {
        return "$0.0000".to_string();
    }
    let abs = cost.abs();
    if abs >= 1_000_000.0 {
        format!("${:.1}M", cost / 1_000_000.0)
    } else if abs >= 1_000.0 {
        format!("${:.1}K", cost / 1_000.0)
    } else if abs >= 10.0 {
        format!("${cost:.2}")
    } else {
        format!("${cost:.4}")
    }
}

fn total_tokens_all(sessions: &[Session]) -> i64 {
    sessions.iter().map(total_tokens).sum()
}

fn total_duration(sessions: &[Session]) -> f64 {
    sessions
        .iter()
        .map(|session| session.metrics.duration_sec)
        .sum()
}

fn p95_gap(sessions: &[Session]) -> f64 {
    let mut gaps: Vec<f64> = sessions
        .iter()
        .flat_map(|session| session.metrics.gaps_sec.iter().copied())
        .filter(|value| value.is_finite() && *value > 0.0)
        .collect();
    if gaps.is_empty() {
        return 0.0;
    }
    gaps.sort_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Equal));
    let index = ((gaps.len() as f64) * 0.95) as usize;
    gaps[index.min(gaps.len() - 1)]
}

fn session_p95_gap(session: &Session) -> f64 {
    let mut gaps = session
        .metrics
        .gaps_sec
        .iter()
        .copied()
        .filter(|value| value.is_finite() && *value > 0.0)
        .collect::<Vec<_>>();
    if gaps.is_empty() {
        return 0.0;
    }
    gaps.sort_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Equal));
    let index = ((gaps.len() as f64) * 0.95) as usize;
    gaps[index.min(gaps.len() - 1)]
}

fn tool_success_rate(session: &Session) -> f64 {
    let total = session.metrics.tool_calls_total;
    if total == 0 {
        return 100.0;
    }
    let ok = total.saturating_sub(session.metrics.tool_calls_fail);
    ok as f64 / total as f64 * 100.0
}

fn triage_reason(session: &Session, language: Language) -> String {
    if session.health < 50 {
        return text(language, "critical health", "健康度严重").to_string();
    }
    if let Some(anomaly) = session
        .anomalies
        .iter()
        .find(|anomaly| anomaly.severity == "high")
        .or_else(|| session.anomalies.first())
    {
        return format!("{} {}", anomaly.kind, text(language, "anomaly", "异常"));
    }
    if session.metrics.tool_calls_fail > 0 {
        return format!(
            "{} {}",
            session.metrics.tool_calls_fail,
            text(language, "failed tools", "个失败工具")
        );
    }
    if session.metrics.cost_estimated >= 1.0 {
        return text(language, "high cost", "高成本").to_string();
    }
    text(language, "healthy", "健康").to_string()
}

fn selected_next_action(session: &Session, language: Language) -> String {
    if session.health < 50 {
        return text(
            language,
            "open diagnostics for critical health",
            "打开诊断查看严重健康问题",
        )
        .to_string();
    }
    if let Some(anomaly) = session
        .anomalies
        .iter()
        .find(|anomaly| anomaly.severity == "high")
        .or_else(|| session.anomalies.first())
    {
        return match language {
            Language::En => format!("inspect {} anomaly in diagnostics", anomaly.kind),
            Language::Zh => format!("在诊断中检查 {} 异常", anomaly.kind),
        };
    }
    if session.metrics.tool_calls_fail > 0 {
        return text(
            language,
            "inspect failed tool results",
            "检查失败的工具结果",
        )
        .to_string();
    }
    if session.metrics.cost_estimated >= 1.0 {
        return text(
            language,
            "compare cost drivers in diff",
            "在对比视图比较成本来源",
        )
        .to_string();
    }
    text(
        language,
        "open detail for full report",
        "打开详情查看完整报告",
    )
    .to_string()
}

fn next_action(app: &App) -> String {
    if app.sessions.is_empty() {
        if app.pending_load.is_some() {
            return app.t("wait for loader", "等待加载完成").to_string();
        }
        return app.t("load sessions", "加载会话").to_string();
    }
    if app.overview.critical > 0 {
        return app.t("open critical sessions", "打开严重会话").to_string();
    }
    if app
        .sessions
        .iter()
        .any(|session| !session.anomalies.is_empty())
    {
        return app.t("review anomalies", "查看异常").to_string();
    }
    if app
        .sessions
        .iter()
        .any(|session| session.metrics.tool_calls_fail > 0)
    {
        return app.t("inspect failed tools", "检查失败工具").to_string();
    }
    app.t("watch cost and latency", "关注成本和延迟")
        .to_string()
}

fn format_count(value: i64) -> String {
    let sign = if value < 0 { "-" } else { "" };
    let digits = value.abs().to_string();
    let mut out = String::new();
    for (idx, ch) in digits.chars().rev().enumerate() {
        if idx > 0 && idx % 3 == 0 {
            out.push(',');
        }
        out.push(ch);
    }
    format!("{sign}{}", out.chars().rev().collect::<String>())
}

fn format_duration(seconds: f64) -> String {
    if !seconds.is_finite() || seconds <= 0.0 {
        return "0s".to_string();
    }
    if seconds < 60.0 {
        return format!("{seconds:.0}s");
    }
    if seconds < 3600.0 {
        return format!("{:.1}m", seconds / 60.0);
    }
    if seconds < 86_400.0 {
        return format!("{:.1}h", seconds / 3600.0);
    }
    format!("{:.1}d", seconds / 86_400.0)
}

fn session_matches(session: &Session, query: &str) -> bool {
    contains(&session.name, query)
        || contains(&session.path, query)
        || contains(&session.cwd, query)
        || contains(&session.metrics.source_tool, query)
        || contains(&display_session_source(session), query)
        || contains(&session.metrics.model_used, query)
        || session
            .metrics
            .tool_usage
            .keys()
            .any(|tool| contains(tool, query))
        || session
            .metrics
            .file_usage
            .keys()
            .any(|file| contains(file, query))
        || session
            .anomalies
            .iter()
            .any(|anomaly| contains(&anomaly.kind, query) || contains(&anomaly.detail, query))
}

fn matches_text_filter(value: &str, filter: &str) -> bool {
    let filter = filter.trim().to_ascii_lowercase();
    filter.is_empty() || contains(value, &filter)
}

fn matches_source_filter(session: &Session, filter: &str) -> bool {
    let filter = filter.trim().to_ascii_lowercase();
    filter.is_empty()
        || contains(&session.metrics.source_tool, &filter)
        || contains(&display_session_source(session), &filter)
}

fn matches_health_filter(session: &Session, filter: &str) -> bool {
    let filter = filter.trim().to_ascii_lowercase();
    if filter.is_empty() {
        return true;
    }
    match filter.as_str() {
        "good" | "healthy" => session.health >= 80,
        "warn" | "warning" => (50..80).contains(&session.health),
        "crit" | "critical" => session.health < 50,
        _ => parse_numeric_i32_filter(&filter)
            .map(|(op, value)| compare_i32(session.health, op, value))
            .unwrap_or(false),
    }
}

fn parse_health_filter(filter: &str) -> Option<()> {
    let filter = filter.trim().to_ascii_lowercase();
    match filter.as_str() {
        "good" | "healthy" | "warn" | "warning" | "crit" | "critical" => Some(()),
        _ => parse_numeric_i32_filter(&filter).map(|_| ()),
    }
}

fn matches_cost_filter(session: &Session, filter: Option<(CostOp, f64)>) -> bool {
    let Some((op, value)) = filter else {
        return true;
    };
    compare_f64(session.metrics.cost_estimated, op, value)
}

fn matches_anomaly_filter(session: &Session, filter: Option<&str>) -> bool {
    let Some(filter) = filter else {
        return true;
    };
    let filter = filter.trim().to_ascii_lowercase();
    if filter.is_empty() {
        return !session.anomalies.is_empty();
    }
    session
        .anomalies
        .iter()
        .any(|anomaly| contains(&anomaly.kind, &filter) || contains(&anomaly.detail, &filter))
}

fn parse_numeric_i32_filter(filter: &str) -> Option<(CostOp, i32)> {
    let (op, value) = parse_operator_value(filter)?;
    value.parse::<i32>().ok().map(|value| (op, value))
}

fn parse_cost_filter(filter: &str) -> Option<(CostOp, f64)> {
    let (op, value) = parse_operator_value(filter.trim())?;
    value.parse::<f64>().ok().map(|value| (op, value))
}

fn parse_operator_value(filter: &str) -> Option<(CostOp, &str)> {
    let filter = filter.trim();
    for (prefix, op) in [
        (">=", CostOp::Gte),
        ("<=", CostOp::Lte),
        (">", CostOp::Gt),
        ("<", CostOp::Lt),
        ("=", CostOp::Eq),
    ] {
        if let Some(value) = filter.strip_prefix(prefix) {
            return Some((op, value.trim()));
        }
    }
    filter.parse::<f64>().ok().map(|_| (CostOp::Gte, filter))
}

fn compare_i32(left: i32, op: CostOp, right: i32) -> bool {
    match op {
        CostOp::Gt => left > right,
        CostOp::Gte => left >= right,
        CostOp::Lt => left < right,
        CostOp::Lte => left <= right,
        CostOp::Eq => left == right,
    }
}

fn compare_f64(left: f64, op: CostOp, right: f64) -> bool {
    match op {
        CostOp::Gt => left > right,
        CostOp::Gte => left >= right,
        CostOp::Lt => left < right,
        CostOp::Lte => left <= right,
        CostOp::Eq => (left - right).abs() < f64::EPSILON,
    }
}

fn command_value(command: &str) -> String {
    command
        .split_once(char::is_whitespace)
        .map(|(_, value)| value.trim().to_string())
        .unwrap_or_default()
}

fn parse_sort_key(value: &str) -> Option<SortKey> {
    match value.trim().to_ascii_lowercase().as_str() {
        "recent" | "time" => Some(SortKey::Recent),
        "health" => Some(SortKey::Health),
        "cost" => Some(SortKey::Cost),
        "turn" | "turns" => Some(SortKey::Turns),
        "fail" | "fails" | "failure" | "failures" | "errors" => Some(SortKey::Failures),
        "source" | "agent" => Some(SortKey::Source),
        "name" | "session" => Some(SortKey::Name),
        "anom" | "anomaly" | "anomalies" => Some(SortKey::Anomalies),
        _ => None,
    }
}

fn active_filter_summary(app: &App) -> String {
    let mut filters = Vec::new();
    if !app.query.is_empty() {
        filters.push(format!("text={}", app.query));
    }
    if !app.health_filter.is_empty() {
        filters.push(format!("health={}", app.health_filter));
    }
    if !app.source_filter.is_empty() {
        filters.push(format!("source={}", app.source_filter));
    }
    if !app.model_filter.is_empty() {
        filters.push(format!("model={}", app.model_filter));
    }
    if let Some((op, value)) = app.cost_filter {
        filters.push(format!("cost{}{}", cost_op_label(op), value));
    }
    if let Some(value) = &app.anomaly_filter {
        if value.is_empty() {
            filters.push("anomaly=any".to_string());
        } else {
            filters.push(format!("anomaly={value}"));
        }
    }
    filters.join(",")
}

fn cost_op_label(op: CostOp) -> &'static str {
    match op {
        CostOp::Gt => ">",
        CostOp::Gte => ">=",
        CostOp::Lt => "<",
        CostOp::Lte => "<=",
        CostOp::Eq => "=",
    }
}

fn compare_sessions(a: &Session, b: &Session, key: SortKey, desc: bool) -> Ordering {
    let ord = match key {
        SortKey::Recent => a
            .metrics
            .session_start
            .cmp(&b.metrics.session_start)
            .then_with(|| a.name.cmp(&b.name)),
        SortKey::Health => a.health.cmp(&b.health).then_with(|| a.name.cmp(&b.name)),
        SortKey::Cost => cmp_f64(a.metrics.cost_estimated, b.metrics.cost_estimated)
            .then_with(|| a.name.cmp(&b.name)),
        SortKey::Turns => a
            .metrics
            .assistant_turns
            .cmp(&b.metrics.assistant_turns)
            .then_with(|| a.name.cmp(&b.name)),
        SortKey::Failures => a
            .metrics
            .tool_calls_fail
            .cmp(&b.metrics.tool_calls_fail)
            .then_with(|| a.name.cmp(&b.name)),
        SortKey::Source => a
            .metrics
            .source_tool
            .cmp(&b.metrics.source_tool)
            .then_with(|| a.name.cmp(&b.name)),
        SortKey::Name => a.name.cmp(&b.name),
        SortKey::Anomalies => a
            .anomalies
            .len()
            .cmp(&b.anomalies.len())
            .then_with(|| a.name.cmp(&b.name)),
    };
    if desc {
        ord.reverse()
    } else {
        ord
    }
}

fn cmp_f64(a: f64, b: f64) -> Ordering {
    a.partial_cmp(&b).unwrap_or(Ordering::Equal)
}

fn contains(value: &str, query: &str) -> bool {
    value.to_ascii_lowercase().contains(query)
}

fn status_width(width: u16) -> usize {
    if width >= 118 {
        44
    } else if width >= 84 {
        28
    } else {
        18
    }
}

fn short(value: &str, max: usize) -> String {
    if value.chars().count() <= max {
        return value.to_string();
    }
    if max <= 3 {
        return value.chars().take(max).collect();
    }
    let mut out = value.chars().take(max - 3).collect::<String>();
    out.push_str("...");
    out
}

fn terminal_safe_report(text: &str) -> String {
    text.chars()
        .map(|ch| match ch {
            '\n' | '\t' => ch,
            '━' | '─' | '═' | '—' | '–' => '-',
            '│' | '┃' => '|',
            '┌' | '┐' | '└' | '┘' | '┬' | '┴' | '├' | '┤' | '┼' => '+',
            ch if ch.is_ascii() => ch,
            _ => ' ',
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use agenttrace_core::{Anomaly, Metrics};
    use crossterm::event::KeyModifiers;
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;
    use std::fs;

    #[test]
    fn filters_sessions_by_metadata_and_tool_usage() {
        let mut app = App::new(
            vec![
                session("billing", "claude_code", "claude-sonnet-4", 70, 0.02, "rg"),
                session("docs", "codex_cli", "gpt-5", 95, 0.01, "read_file"),
            ],
            "test",
            None,
        );

        app.query = "billing".to_string();
        app.refresh_filtered();
        assert_eq!(app.filtered.len(), 1);
        assert_eq!(
            app.selected_session().map(|s| s.name.as_str()),
            Some("billing")
        );

        app.query = "read_file".to_string();
        app.refresh_filtered();
        assert_eq!(
            app.selected_session().map(|s| s.name.as_str()),
            Some("docs")
        );

        app.query = "Claude Code".to_string();
        app.refresh_filtered();
        assert_eq!(
            app.selected_session().map(|s| s.name.as_str()),
            Some("billing")
        );
    }

    #[test]
    fn commands_switch_views_and_apply_search() {
        let mut app = App::new(
            vec![session("billing", "claude_code", "m", 80, 0.0, "rg")],
            "test",
            None,
        );

        assert!(!app.run_command("search billing").unwrap());
        assert_eq!(app.view, View::List);
        assert_eq!(app.filtered.len(), 1);

        assert!(!app.run_command("reset").unwrap());
        assert_eq!(app.view, View::List);
        assert_eq!(app.status, "filter cleared");
        assert_eq!(app.filtered.len(), 1);
        assert!(app.query.is_empty());

        assert!(!app.run_command("diagnostics").unwrap());
        assert_eq!(app.view, View::Diagnostics);

        assert!(app.run_command("quit").unwrap());
    }

    #[test]
    fn inspect_command_selects_ranked_session_and_clears_filters() {
        let mut anomalous = session("anomalous", "pi", "m", 70, 0.05, "rg");
        anomalous.anomalies.push(Anomaly {
            kind: "latency".to_string(),
            severity: "high".to_string(),
            detail: "p95 gap".to_string(),
        });
        let mut critical = session("critical", "claude_code", "m", 40, 0.20, "bash");
        critical.metrics.tool_calls_fail = 2;
        critical.metrics.tool_calls_total = 3;
        let mut app = App::new(
            vec![
                critical,
                anomalous,
                session("docs", "codex_cli", "gpt-5", 95, 0.01, "read_file"),
            ],
            "test",
            None,
        );

        app.query = "docs".to_string();
        app.refresh_filtered();
        assert_eq!(
            app.selected_session().map(|session| session.name.as_str()),
            Some("docs")
        );

        assert!(!app.run_command("inspect 2").unwrap());
        assert!(app.query.is_empty());
        assert_eq!(app.view, View::Diagnostics);
        assert_eq!(
            app.selected_session().map(|session| session.name.as_str()),
            Some("anomalous")
        );
        assert!(app.status.contains("inspect anomaly #2"));
    }

    #[test]
    fn overview_enter_opens_first_inspect_item() {
        let mut app = App::new(
            vec![
                session("healthy", "codex_cli", "gpt-5", 95, 0.01, "read_file"),
                session("critical", "claude_code", "m", 35, 0.20, "bash"),
            ],
            "test",
            None,
        );
        app.view = View::Overview;

        app.handle_normal_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE))
            .expect("overview enter");

        assert_eq!(app.view, View::Diagnostics);
        assert_eq!(
            app.selected_session().map(|session| session.name.as_str()),
            Some("critical")
        );
        assert!(app.status.contains("inspect critical #1"));
    }

    #[test]
    fn language_defaults_to_english_and_l_toggles_chinese() {
        let mut app = App::new(
            vec![session("critical", "claude_code", "m", 35, 0.20, "bash")],
            "test",
            None,
        );
        assert_eq!(app.language, Language::En);
        assert!(help_text(app.language).contains("Triage workflow"));

        app.handle_normal_key(KeyEvent::new(KeyCode::Char('l'), KeyModifiers::NONE))
            .expect("toggle language");
        assert_eq!(app.language, Language::Zh);
        assert_eq!(app.status, "语言：中文");
        assert!(help_text(app.language).contains("分诊流程"));

        let backend = TestBackend::new(100, 24);
        let mut terminal = Terminal::new(backend).expect("test terminal");
        terminal
            .draw(|frame| render(frame, &mut app))
            .expect("render zh overview");
        let overview = format!("{:?}", terminal.backend().buffer());
        assert!(overview.contains("概览"));
        assert!(overview.contains("优先检查"));
        assert!(overview.contains("健康度严重"));
        assert!(overview.contains("打开诊断查看严重健康问题"));

        app.handle_normal_key(KeyEvent::new(KeyCode::Char('l'), KeyModifiers::NONE))
            .expect("toggle language back");
        assert_eq!(app.language, Language::En);
        assert_eq!(app.status, "language: English");
    }

    #[test]
    fn unknown_commands_fall_back_to_text_filter_like_go_tui() {
        let mut app = App::new(
            vec![
                session("billing", "claude_code", "m", 80, 0.0, "rg"),
                session("docs", "codex_cli", "m", 95, 0.0, "read_file"),
            ],
            "test",
            None,
        );

        assert!(!app.run_command("billing").unwrap());
        assert_eq!(app.view, View::List);
        assert_eq!(app.query, "billing");
        assert_eq!(app.status, "filter: billing");
        assert_eq!(app.filtered.len(), 1);
        assert_eq!(
            app.selected_session().map(|session| session.name.as_str()),
            Some("billing")
        );
    }

    #[test]
    fn commands_apply_go_style_triage_filters() {
        let mut costly = session("costly", "claude_code", "claude-sonnet-4", 45, 1.20, "rg");
        costly.anomalies.push(Anomaly {
            kind: "latency".to_string(),
            severity: "high".to_string(),
            detail: "p95 gap".to_string(),
        });
        let mut app = App::new(
            vec![
                costly,
                session("docs", "codex_cli", "gpt-5", 95, 0.01, "read_file"),
                session("mid", "pi", "gpt-5-mini", 70, 0.08, "grep"),
            ],
            "test",
            None,
        );

        assert!(!app.run_command("health crit").unwrap());
        assert_eq!(app.filtered.len(), 1);
        assert_eq!(
            app.selected_session().map(|s| s.name.as_str()),
            Some("costly")
        );

        assert!(!app.run_command("clear").unwrap());
        assert!(!app.run_command("source codex").unwrap());
        assert_eq!(
            app.selected_session().map(|s| s.name.as_str()),
            Some("docs")
        );

        assert!(!app.run_command("source Claude Code").unwrap());
        assert_eq!(
            app.selected_session().map(|s| s.name.as_str()),
            Some("costly")
        );

        assert!(!app.run_command("model mini").unwrap());
        assert_eq!(app.filtered.len(), 0);

        assert!(!app.run_command("clear").unwrap());
        assert!(!app.run_command("cost >0.10").unwrap());
        assert_eq!(
            app.selected_session().map(|s| s.name.as_str()),
            Some("costly")
        );

        assert!(!app.run_command("anomaly latency").unwrap());
        assert_eq!(
            app.selected_session().map(|s| s.name.as_str()),
            Some("costly")
        );

        assert!(!app.run_command("critical").unwrap());
        assert_eq!(app.health_filter, "crit");
    }

    #[test]
    fn commands_apply_explicit_sort_direction_and_top_alias() {
        let mut app = App::new(
            vec![
                session("cheap", "pi", "m", 90, 0.01, "rg"),
                session("expensive", "codex_cli", "m", 90, 0.30, "rg"),
            ],
            "test",
            None,
        );

        assert!(!app.run_command("sort cost asc").unwrap());
        assert_eq!(
            app.selected_session().map(|s| s.name.as_str()),
            Some("cheap")
        );
        assert!(!app.run_command("top cost").unwrap());
        assert_eq!(
            app.selected_session().map(|s| s.name.as_str()),
            Some("expensive")
        );
        assert_eq!(app.sort_key, SortKey::Cost);
        assert!(app.sort_desc);

        assert!(!app.run_command("sort source asc").unwrap());
        assert_eq!(
            app.selected_session().map(|s| s.name.as_str()),
            Some("expensive")
        );
        assert_eq!(app.sort_key, SortKey::Source);
        assert!(!app.sort_desc);
    }

    #[test]
    fn quick_filter_keys_match_go_keymap_semantics() {
        let mut app = App::new(
            vec![
                session("healthy", "codex_cli", "m", 95, 0.00, "read_file"),
                session("warning", "pi", "m", 70, 0.02, "rg"),
                session("critical", "claude_code", "m", 40, 0.50, "bash"),
            ],
            "test",
            None,
        );

        app.handle_normal_key(KeyEvent::new(KeyCode::Char('f'), KeyModifiers::NONE))
            .expect("health filter key");
        assert_eq!(app.health_filter, "crit");
        assert_eq!(
            app.selected_session().map(|s| s.name.as_str()),
            Some("critical")
        );

        app.handle_normal_key(KeyEvent::new(KeyCode::Char('f'), KeyModifiers::NONE))
            .expect("health filter key");
        assert_eq!(app.health_filter, "warn");
        assert_eq!(
            app.selected_session().map(|s| s.name.as_str()),
            Some("warning")
        );

        app.handle_normal_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE))
            .expect("clear filters");
        let selected_source = app
            .selected_session()
            .map(|session| session.metrics.source_tool.clone())
            .expect("selected session source");
        app.handle_normal_key(KeyEvent::new(KeyCode::Char('s'), KeyModifiers::NONE))
            .expect("source filter key");
        assert_eq!(app.source_filter, selected_source);

        app.handle_normal_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE))
            .expect("clear filters");
        app.handle_normal_key(KeyEvent::new(KeyCode::Char('$'), KeyModifiers::NONE))
            .expect("cost filter key");
        assert_eq!(app.filtered.len(), 2);
        assert!(app.status.contains("quick cost filter"));

        app.handle_normal_key(KeyEvent::new(KeyCode::Char('!'), KeyModifiers::NONE))
            .expect("critical filter key");
        assert_eq!(app.health_filter, "crit");
        assert_eq!(
            app.selected_session().map(|s| s.name.as_str()),
            Some("critical")
        );
    }

    #[test]
    fn sort_cost_descending_then_toggle() {
        let mut app = App::new(
            vec![
                session("cheap", "codex_cli", "m", 90, 0.01, "rg"),
                session("expensive", "codex_cli", "m", 90, 0.30, "rg"),
            ],
            "test",
            None,
        );

        app.set_sort(SortKey::Cost);
        assert_eq!(
            app.selected_session().map(|s| s.name.as_str()),
            Some("expensive")
        );
        app.set_sort(SortKey::Cost);
        assert_eq!(
            app.selected_session().map(|s| s.name.as_str()),
            Some("cheap")
        );
    }

    #[test]
    fn overview_inspect_first_prioritizes_distinct_triage_entries() {
        let mut critical = session("critical", "codex_cli", "m", 40, 0.20, "bash");
        critical.metrics.tokens_input = 200;
        let mut anomalous = session("anomalous", "pi", "m", 70, 0.05, "rg");
        anomalous.anomalies.push(Anomaly {
            kind: "latency".to_string(),
            severity: "medium".to_string(),
            detail: "p95 gap".to_string(),
        });
        let mut failed = session("failed", "claude_code", "m", 85, 0.03, "read_file");
        failed.metrics.tool_calls_fail = 3;
        failed.metrics.tool_calls_total = 4;
        let mut costly = session("costly", "codex_cli", "m", 95, 1.40, "rg");
        costly.metrics.tokens_input = 3_000;
        let mut slow = session("slow", "codex_cli", "m", 95, 0.02, "rg");
        slow.metrics.duration_sec = 700.0;
        slow.metrics.gaps_sec = vec![5.0, 120.0, 240.0];
        let mut app = App::new(
            vec![critical, anomalous, failed, costly, slow],
            "testdata",
            None,
        );

        let items = inspect_first_items(&app.sessions);
        assert_eq!(items[0].label, "critical");
        assert_eq!(app.sessions[items[0].index].name, "critical");
        assert!(items.iter().any(|item| item.label == "anomaly"));
        assert!(items.iter().any(|item| item.label == "failures"));
        assert!(items.iter().any(|item| item.label == "cost"));
        assert!(items.iter().any(|item| item.label == "latency"));

        let backend = TestBackend::new(120, 42);
        let mut terminal = Terminal::new(backend).expect("test terminal");
        terminal
            .draw(|frame| render(frame, &mut app))
            .expect("render overview");
        let overview = format!("{:?}", terminal.backend().buffer());
        assert!(overview.contains("Inspect First"));
        assert!(overview.contains("critical"));
        assert!(overview.contains("critical health"));
        assert!(overview.contains("action: open diagnostics for critical"));

        app.view = View::Overview;
        app.handle_normal_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE))
            .expect("open top inspect item");
        assert_eq!(app.view, View::Diagnostics);
        assert_eq!(
            app.selected_session().map(|session| session.name.as_str()),
            Some("critical")
        );
    }

    #[test]
    fn renders_overview_and_list_with_test_backend() {
        let mut billing = session("billing", "claude_code", "claude-sonnet-4", 80, 0.02, "rg");
        billing.anomalies.push(Anomaly {
            kind: "latency".to_string(),
            severity: "medium".to_string(),
            detail: "p95 gap".to_string(),
        });
        let mut app = App::new(
            vec![
                billing,
                session("docs", "codex_cli", "gpt-5", 95, 0.01, "read_file"),
            ],
            "testdata",
            None,
        );
        let backend = TestBackend::new(100, 38);
        let mut terminal = Terminal::new(backend).expect("test terminal");

        terminal
            .draw(|frame| render(frame, &mut app))
            .expect("render overview");
        let overview = format!("{:?}", terminal.backend().buffer());
        assert!(overview.contains("AGENTTRACE"));
        assert!(overview.contains("idle"));
        assert!(overview.contains("Scoreboard"));
        assert!(overview.contains("Loading Status"));
        assert!(overview.contains("normal load"));
        assert!(overview.contains("0 cache hits"));
        assert!(overview.contains("tokens"));
        assert!(overview.contains("health"));
        assert!(overview.contains("p95"));
        assert!(overview.contains("gpt-5") || overview.contains("claude-sonnet"));
        assert!(overview.contains("Driver Summary"));
        assert!(overview.contains("latency"));
        assert!(overview.contains("Inspect First"));
        assert!(overview.contains("Recent Sessions"));
        assert!(overview.contains("latency anomaly"));

        app.view = View::List;
        terminal
            .draw(|frame| render(frame, &mut app))
            .expect("render list");
        let list = format!("{:?}", terminal.backend().buffer());
        assert!(list.contains("Driver Summary"));
        assert!(list.contains("List Status"));
        assert!(list.contains("2/2 visible"));
        assert!(list.contains("filters: none"));
        assert!(list.contains("Enter detail"));
        assert!(list.contains("Loading Status"));
        assert!(list.contains("Idle"));
        assert!(list.contains("Source"));
        assert!(list.contains("Claude Code"));
        assert!(list.contains("Model"));
        assert!(list.contains("fail0"));
        assert!(list.contains("Selected Triage"));
        assert!(list.contains("selected: billing"));
        assert!(list.contains("ok=100%"));
        assert!(list.contains("reason=latency anomaly"));
        assert!(list.contains("p95 latency"));
        assert!(list.contains("Sessions"));
        assert!(list.contains("sort Recent desc"));
        assert!(list.contains("ok%"));
        assert!(list.contains("anom"));
        assert!(list.contains("reason"));
        assert!(list.contains("billing"));
        assert!(!list.contains("claude_code"));
        assert!(list.contains("q quit"));
        assert!(list.contains("! critical"));
        assert!(list.contains("? help"));

        app.view = View::Detail;
        terminal
            .draw(|frame| render(frame, &mut app))
            .expect("render detail");
        let detail = format!("{:?}", terminal.backend().buffer());
        assert!(detail.contains("Detail - billing"));
        assert!(detail.contains("reason=latency anomaly"));
        assert!(detail.contains("Session Summary"));
        assert!(detail.contains("Context:"));
        assert!(detail.contains("health=80"));
        assert!(detail.contains("cost=$0.0200"));
        assert!(detail.contains("fail=0"));
        assert!(detail.contains("anom=1"));
        assert!(detail.contains("source=Claude Code"));
        assert!(detail.contains("Next Action"));
        assert!(detail.contains("Raw report"));

        app.view = View::Diagnostics;
        terminal
            .draw(|frame| render(frame, &mut app))
            .expect("render diagnostics");
        let diagnostics = format!("{:?}", terminal.backend().buffer());
        assert!(diagnostics.contains("Diagnostics - billing"));
        assert!(diagnostics.contains("reason=latency anomaly"));
        assert!(diagnostics.contains("Problem"));
        assert!(diagnostics.contains("Next"));
        assert!(diagnostics.contains("Evidence"));
        assert!(diagnostics.contains("Raw Signals"));
        assert!(diagnostics.contains("Context:"));
        assert!(diagnostics.contains("health=80"));
        assert!(diagnostics.contains("cost=$0.0200"));
        assert!(diagnostics.contains("fail=0"));
        assert!(diagnostics.contains("anom=1"));
        assert!(diagnostics.contains("source=Claude Code"));
        assert!(diagnostics_text(&app).contains("Raw diagnostics"));

        app.view = View::Diff;
        terminal
            .draw(|frame| render(frame, &mut app))
            .expect("render diff");
        let diff = format!("{:?}", terminal.backend().buffer());
        assert!(diff.contains("Diff - 2 visible"));
        assert!(diff.contains("sort Recent desc"));
        assert!(diff.contains("Context:"));
        assert!(diff.contains("visible=2"));
        assert!(diff.contains("filter=none"));
        assert!(diff.contains("top_source=Claude Code:1"));

        app.view = View::Help;
        terminal
            .draw(|frame| render(frame, &mut app))
            .expect("render help");
        let help = format!("{:?}", terminal.backend().buffer());
        assert!(help.contains("Triage workflow"));
        assert!(help.contains("enter on Overview"));
        assert!(help.contains("f cycles health"));
        assert!(help.contains("s selected source"));
        assert!(help.contains("$ costly sessions"));
        assert!(help.contains("! critical"));
        assert!(help.contains(":inspect [rank]"));
        assert!(help.contains(":health good|warn|crit|<80"));
        assert!(help.contains(":source <name>"));
        assert!(help.contains(":clear/:reset"));
        assert!(help.contains(":sort <field> [asc|desc]"));
    }

    #[test]
    fn diff_empty_state_explains_active_filters() {
        let mut app = App::new(
            vec![session(
                "billing",
                "claude_code",
                "claude-sonnet-4",
                80,
                0.02,
                "rg",
            )],
            "testdata",
            None,
        );
        app.model_filter = "definitely-no-match".to_string();
        app.refresh_filtered();
        app.view = View::Diff;

        let backend = TestBackend::new(100, 18);
        let mut terminal = Terminal::new(backend).expect("test terminal");
        terminal
            .draw(|frame| render(frame, &mut app))
            .expect("render empty diff");
        let diff = format!("{:?}", terminal.backend().buffer());
        assert!(diff.contains("Context:"));
        assert!(diff.contains("visible=0"));
        assert!(diff.contains("filter=model=definitely-no-match"));
        assert!(diff.contains("Need at least two visible sessions for diff."));
        assert!(diff.contains("Active filters: model=definitely-no-match"));
        assert!(diff.contains("Press Esc or run :clear/:reset"));
    }

    #[test]
    fn renders_no_visible_sessions_state_for_empty_filter_result() {
        let mut app = App::new(
            vec![session(
                "billing",
                "claude_code",
                "claude-sonnet-4",
                80,
                0.02,
                "rg",
            )],
            "testdata",
            None,
        );
        app.model_filter = "definitely-no-match".to_string();
        app.refresh_filtered();
        app.view = View::List;

        let backend = TestBackend::new(100, 18);
        let mut terminal = Terminal::new(backend).expect("test terminal");
        terminal
            .draw(|frame| render(frame, &mut app))
            .expect("render empty filter list");
        let list = format!("{:?}", terminal.backend().buffer());
        assert!(list.contains("No visible sessions match the active filters."));
        assert!(list.contains("Active filters: model=definitely-no-match"));
        assert!(list.contains("Press Esc or run :clear"));
    }

    #[test]
    fn ctrl_r_force_reload_clears_session_cache_before_loading() {
        let root = std::env::temp_dir().join(format!(
            "agenttrace-rust-tui-force-reload-{}",
            std::process::id()
        ));
        let sessions_dir = root.join("sessions");
        let cache_dir = root.join("cache");
        fs::create_dir_all(&sessions_dir).expect("create sessions dir");
        fs::create_dir_all(&cache_dir).expect("create cache dir");
        let session_path = sessions_dir.join("session.jsonl");
        fs::write(
            &session_path,
            r#"{"role":"user","content":"fresh","timestamp":"2026-05-02T10:00:00Z","SourceTool":"generic"}
{"role":"assistant","content":"fresh answer","timestamp":"2026-05-02T10:00:01Z","SourceTool":"generic"}
"#,
        )
        .expect("write session");
        let metadata = fs::metadata(&session_path).expect("session metadata");
        let cache_path = cache_dir.join("sessions.json");
        fs::write(
            &cache_path,
            format!(
                r#"{{"schema_version":11,"entries":{{"{}":{{"mod_time":{},"size":{},"session":{{"Name":"cached","Path":"{}","Metrics":{{"SourceTool":"hermes_jsonl","ModelUsed":"cached-model","SessionStart":"2026-05-02T09:00:00Z","ToolArgUsage":{{}}}},"Health":91,"ToolWarnings":[]}}}}}}}}"#,
                session_path.to_string_lossy(),
                file_mod_time_nanos_for_test(&metadata),
                metadata.len(),
                session_path.to_string_lossy()
            ),
        )
        .expect("write cache");

        with_session_cache_dir_for_test(&cache_dir, || {
            let mut app = App::new(
                Vec::new(),
                "test",
                Some(sessions_dir.to_string_lossy().to_string()),
            );
            app.reload(false).expect("normal reload");
            assert_eq!(app.load_state.phase, LoadPhase::Discovering);
            assert_eq!(app.load_state.discovered, 1);
            assert_eq!(app.load_state.cache_hits, 1);
            assert!(app.status.contains("discovering 1 session files"));
            let loading = loading_status_lines(&app)
                .iter()
                .map(|line| format!("{line:?}"))
                .collect::<Vec<_>>()
                .join("\n");
            assert!(loading.contains("Discovering"));
            assert!(loading.contains("normal load"));
            assert!(loading.contains("loaded 0/1 files"));
            assert!(loading.contains("1 cache hits"));
            wait_for_pending_load(&mut app);
            assert_eq!(app.sessions.len(), 1);
            assert_eq!(app.sessions[0].name, "cached");
            assert_eq!(app.load_state.phase, LoadPhase::Ready);
            assert_eq!(app.load_state.parsed, 1);
            assert!(load_summary_line(&app).contains("loaded 1 sessions"));
            assert!(load_summary_line(&app).contains("1 cache hits"));
            assert!(load_summary_line(&app).contains("hermes_jsonl:1"));
            assert!(cache_path.is_file());

            app.handle_normal_key(KeyEvent::new(KeyCode::Char('r'), KeyModifiers::CONTROL))
                .expect("force reload");
            assert!(app
                .status
                .contains("force reload: discovering 1 session files"));
            assert_eq!(app.load_state.phase, LoadPhase::Discovering);
            assert_eq!(app.load_state.cache_hits, 0);
            let force_loading = loading_status_lines(&app)
                .iter()
                .map(|line| format!("{line:?}"))
                .collect::<Vec<_>>()
                .join("\n");
            assert!(force_loading.contains("force reload"));
            assert!(force_loading.contains("0 cache hits"));
            assert!(force_loading.contains("cache bypass"));
            wait_for_pending_load(&mut app);
            assert_eq!(app.sessions.len(), 1);
            assert_eq!(app.sessions[0].name, "session");
            let refreshed_cache = fs::read_to_string(&cache_path).expect("read refreshed cache");
            assert!(refreshed_cache.contains(r#""Name":"session""#));
            assert!(!refreshed_cache.contains(r#""Name":"cached""#));
            assert!(app.status.starts_with("force reloaded 1 sessions"));
        });

        let _ = fs::remove_dir_all(root);
    }

    fn wait_for_pending_load(app: &mut App) {
        for _ in 0..50 {
            app.poll_pending_load();
            if app.pending_load.is_none() {
                return;
            }
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
        panic!("pending TUI load did not finish");
    }

    fn session(
        name: &str,
        source: &str,
        model: &str,
        health: i32,
        cost: f64,
        tool: &str,
    ) -> Session {
        let mut metrics = Metrics {
            source_tool: source.to_string(),
            model_used: model.to_string(),
            session_start: format!("2026-05-02T10:00:0{}Z", name.len() % 10),
            assistant_turns: 2,
            tool_calls_total: 1,
            tool_calls_fail: if health < 80 { 1 } else { 0 },
            cost_estimated: cost,
            tokens_input: 10,
            tokens_output: 5,
            duration_sec: 125.0,
            gaps_sec: vec![2.0, 12.0, 40.0],
            ..Metrics::default()
        };
        metrics.tool_usage.insert(tool.to_string(), 1);
        Session {
            name: name.to_string(),
            path: format!("/tmp/{name}.jsonl"),
            cwd: "/tmp".to_string(),
            metrics,
            anomalies: Vec::new(),
            health,
            tool_warnings: Vec::new(),
        }
    }

    fn with_session_cache_dir_for_test(cache_dir: &std::path::Path, run: impl FnOnce()) {
        let previous = std::env::var_os("AGENTTRACE_SESSION_CACHE_DIR");
        std::env::set_var("AGENTTRACE_SESSION_CACHE_DIR", cache_dir);
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(run));
        match previous {
            Some(value) => std::env::set_var("AGENTTRACE_SESSION_CACHE_DIR", value),
            None => std::env::remove_var("AGENTTRACE_SESSION_CACHE_DIR"),
        }
        if let Err(payload) = result {
            std::panic::resume_unwind(payload);
        }
    }

    #[cfg(unix)]
    fn file_mod_time_nanos_for_test(metadata: &fs::Metadata) -> i64 {
        use std::os::unix::fs::MetadataExt;
        metadata.mtime() * 1_000_000_000 + metadata.mtime_nsec()
    }

    #[cfg(not(unix))]
    fn file_mod_time_nanos_for_test(metadata: &fs::Metadata) -> i64 {
        metadata
            .modified()
            .ok()
            .and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|duration| duration.as_nanos().min(i64::MAX as u128) as i64)
            .unwrap_or(0)
    }
}
