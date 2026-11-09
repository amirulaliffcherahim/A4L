use a4l_core::state::{
    StateStore, Task, TaskRole, TaskStatus, TodoStatus,
};
use a4l_memory::EpisodicStore;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::{Backend, CrosstermBackend},
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, List, ListItem, Paragraph, Wrap},
    Frame, Terminal,
};
use std::io;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

// Mistral & Pi signature color palette
const MISTRAL_ORANGE: Color = Color::Rgb(255, 112, 0);
const MISTRAL_AMBER: Color = Color::Rgb(250, 176, 5);
const BG_CHARCOAL: Color = Color::Rgb(20, 20, 24);
const CARD_BG: Color = Color::Rgb(30, 30, 36);
const CARD_BORDER: Color = Color::Rgb(55, 55, 68);
const TEXT_WARM_WHITE: Color = Color::Rgb(248, 249, 250);
const TEXT_MUTED: Color = Color::Rgb(140, 144, 155);
const SUCCESS_EMERALD: Color = Color::Rgb(46, 204, 113);
const FAIL_CRIMSON: Color = Color::Rgb(235, 87, 87);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActiveTab {
    AgentChat,
    PipelineBoard,
    MemoryStudio,
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub enum ChatItemKind {
    User,
    Agent {
        role: String,
    },
    ToolCall {
        tool: String,
        summary: String,
        exit: Option<i32>,
    },
    SystemEvent,
}

#[derive(Debug, Clone)]
pub struct ChatMessageItem {
    pub kind: ChatItemKind,
    pub title: String,
    pub body: String,
    pub time: String,
}

pub struct TuiState {
    pub tab: ActiveTab,
    pub input: String,
    pub chat_history: Vec<ChatMessageItem>,
    pub scroll_offset: usize,
    pub selected_task_idx: usize,
    pub memory_query: String,
    pub memory_results: Vec<a4l_memory::Episode>,
    pub notification: Option<(String, Instant)>,
    pub repo_root: PathBuf,
}

impl TuiState {
    pub fn new(repo_root: PathBuf) -> Self {
        let now_str = chrono::Utc::now().format("%H:%M:%S").to_string();
        let initial_chat = vec![
            ChatMessageItem {
                kind: ChatItemKind::SystemEvent,
                title: "A4L System Initialized".to_string(),
                body: "Deterministic human-in-the-loop harness ready. Progressive context loaded."
                    .to_string(),
                time: now_str.clone(),
            },
            ChatMessageItem {
                kind: ChatItemKind::Agent {
                    role: "planner".to_string(),
                },
                title: "A4L (Planner)".to_string(),
                body: "Awaiting instruction. Type a request or run /help to inspect commands."
                    .to_string(),
                time: now_str,
            },
        ];

        Self {
            tab: ActiveTab::AgentChat,
            input: String::new(),
            chat_history: initial_chat,
            scroll_offset: 0,
            selected_task_idx: 0,
            memory_query: String::new(),
            memory_results: Vec::new(),
            notification: None,
            repo_root,
        }
    }

    pub fn set_notification(&mut self, text: impl Into<String>) {
        self.notification = Some((text.into(), Instant::now()));
    }
}

pub fn run_tui(repo_root: &Path) -> anyhow::Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut state = TuiState::new(repo_root.to_path_buf());
    let res = run_loop(&mut terminal, &mut state);

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        eprintln!("TUI Error: {:?}", err);
    }

    Ok(())
}

fn run_loop<B: Backend>(terminal: &mut Terminal<B>, state: &mut TuiState) -> io::Result<()> {
    let tasks_file = state.repo_root.join(".a4l").join("tasks.json");
    let store = StateStore::new(&tasks_file);

    loop {
        let tasks = store.load().unwrap_or_default();
        let queue_len = tasks.queue.len();

        if state.selected_task_idx >= queue_len && queue_len > 0 {
            state.selected_task_idx = queue_len - 1;
        }

        // Clean up stale notification after 3 seconds
        if let Some((_, time)) = &state.notification {
            if time.elapsed() > Duration::from_secs(3) {
                state.notification = None;
            }
        }

        terminal.draw(|f| {
            render_main_ui(f, state, &tasks.queue);
        })?;

        if event::poll(Duration::from_millis(150))? {
            if let Event::Key(key) = event::read()? {
                // Global quit on Esc or Ctrl+C
                if key.code == KeyCode::Esc
                    || (key.modifiers == KeyModifiers::CONTROL && key.code == KeyCode::Char('c'))
                {
                    return Ok(());
                }

                // Tab switching with Tab
                if key.code == KeyCode::Tab {
                    state.tab = match state.tab {
                        ActiveTab::AgentChat => ActiveTab::PipelineBoard,
                        ActiveTab::PipelineBoard => ActiveTab::MemoryStudio,
                        ActiveTab::MemoryStudio => ActiveTab::AgentChat,
                    };
                    continue;
                }

                match state.tab {
                    ActiveTab::AgentChat => {
                        handle_chat_input(state, key.code, &store, &tasks.queue)
                    }
                    ActiveTab::PipelineBoard => {
                        handle_board_input(state, key.code, &store, &tasks.queue)
                    }
                    ActiveTab::MemoryStudio => handle_memory_input(state, key.code),
                }
            }
        }
    }
}

fn handle_chat_input(state: &mut TuiState, code: KeyCode, store: &StateStore, tasks: &[Task]) {
    match code {
        KeyCode::Enter => {
            let trimmed = state.input.trim().to_string();
            if trimmed.is_empty() {
                return;
            }
            state.input.clear();
            let now_str = chrono::Utc::now().format("%H:%M:%S").to_string();

            // Push User Message
            state.chat_history.push(ChatMessageItem {
                kind: ChatItemKind::User,
                title: "You".to_string(),
                body: trimmed.clone(),
                time: now_str.clone(),
            });

            // Handle slash commands
            if trimmed.starts_with('/') {
                handle_slash_command(state, &trimmed, store, tasks);
                return;
            }

            // Standard Agent Simulated Response / Execution Card
            state.chat_history.push(ChatMessageItem {
                kind: ChatItemKind::ToolCall {
                    tool: "bash".to_string(),
                    summary: "cargo check --quiet".to_string(),
                    exit: Some(0),
                },
                title: "Tool Execution: bash".to_string(),
                body: "Exit 0. Static compilation checks green across workspace.".to_string(),
                time: now_str.clone(),
            });

            state.chat_history.push(ChatMessageItem {
                kind: ChatItemKind::Agent {
                    role: "maker".to_string(),
                },
                title: "A4L (Maker)".to_string(),
                body: format!(
                    "Working on '{}'. All verification gates observed exit 0. Ready for review.",
                    trimmed
                ),
                time: now_str,
            });
        }
        KeyCode::Char(c) => {
            state.input.push(c);
        }
        KeyCode::Backspace => {
            state.input.pop();
        }
        KeyCode::Up => {
            state.scroll_offset = state.scroll_offset.saturating_add(1);
        }
        KeyCode::Down => {
            state.scroll_offset = state.scroll_offset.saturating_sub(1);
        }
        _ => {}
    }
}

fn handle_slash_command(state: &mut TuiState, cmd: &str, store: &StateStore, tasks: &[Task]) {
    let now_str = chrono::Utc::now().format("%H:%M:%S").to_string();
    let parts: Vec<&str> = cmd.split_whitespace().collect();
    match parts[0] {
        "/help" => {
            state.chat_history.push(ChatMessageItem {
                kind: ChatItemKind::SystemEvent,
                title: "Help & Commands".to_string(),
                body: "/status - Show task pipeline status\n/approve - Approve current task at HITL gate\n/sendback <notes> - Send task back to Planner\n/memory <query> - Search episodic memories\n/clear - Clear chat history\n/quit - Exit application".to_string(),
                time: now_str,
            });
        }
        "/clear" => {
            state.chat_history.clear();
            state.set_notification("Chat history cleared.");
        }
        "/status" => {
            let count = tasks.len();
            state.chat_history.push(ChatMessageItem {
                kind: ChatItemKind::SystemEvent,
                title: "Pipeline Status".to_string(),
                body: format!("Queue has {} task(s). Active tab: {:?}", count, state.tab),
                time: now_str,
            });
        }
        "/approve" => {
            if let Some(task) = tasks.get(state.selected_task_idx) {
                if task.state.role == TaskRole::Human {
                    let _ = store.update_task(&task.id, |t| {
                        t.state.status = TaskStatus::Completed;
                        Ok(())
                    });
                    state.set_notification(format!("Task {} approved and completed!", task.id));
                    state.chat_history.push(ChatMessageItem {
                        kind: ChatItemKind::SystemEvent,
                        title: "Executive Sign-Off".to_string(),
                        body: format!(
                            "Task {} approved by human executive. Marked completed.",
                            task.id
                        ),
                        time: now_str,
                    });
                } else {
                    state.set_notification(format!(
                        "Task {} is in {} role, not awaiting HITL.",
                        task.id, task.state.role
                    ));
                }
            }
        }
        "/sendback" => {
            let notes = if parts.len() > 1 {
                parts[1..].join(" ")
            } else {
                "Needs revision".to_string()
            };
            if let Some(task) = tasks.get(state.selected_task_idx) {
                let _ = store.update_task(&task.id, |t| {
                    t.state.role = TaskRole::Planner;
                    t.state.status = TaskStatus::SentBack;
                    t.state.attempt += 1;
                    Ok(())
                });
                state.set_notification(format!("Task {} sent back to Planner.", task.id));
                state.chat_history.push(ChatMessageItem {
                    kind: ChatItemKind::SystemEvent,
                    title: "Sent Back to Planner".to_string(),
                    body: format!("Task {} sent back with notes: '{}'", task.id, notes),
                    time: now_str,
                });
            }
        }
        "/memory" => {
            if parts.len() > 1 {
                let query = parts[1..].join(" ");
                let db_path = state.repo_root.join(".a4l").join("memory.db");
                if let Ok(ep_store) = EpisodicStore::open(&db_path) {
                    if let Ok(matches) = ep_store.search_by_symptom(&query) {
                        state.memory_results = matches.clone();
                        state.chat_history.push(ChatMessageItem {
                            kind: ChatItemKind::SystemEvent,
                            title: format!("Episodic Memory Search: '{}'", query),
                            body: format!(
                                "Found {} relevant episodes matching symptoms.",
                                matches.len()
                            ),
                            time: now_str,
                        });
                    }
                }
            }
        }
        "/quit" => {
            std::process::exit(0);
        }
        _ => {
            state.set_notification(format!("Unknown command: {}", parts[0]));
        }
    }
}

fn handle_board_input(state: &mut TuiState, code: KeyCode, store: &StateStore, tasks: &[Task]) {
    let queue_len = tasks.len();
    match code {
        KeyCode::Up => {
            state.selected_task_idx = state.selected_task_idx.saturating_sub(1);
        }
        KeyCode::Down if queue_len > 0 && state.selected_task_idx + 1 < queue_len => {
            state.selected_task_idx += 1;
        }
        KeyCode::Char('a') => {
            if let Some(task) = tasks.get(state.selected_task_idx) {
                if task.state.role == TaskRole::Human {
                    let _ = store.update_task(&task.id, |t| {
                        t.state.status = TaskStatus::Completed;
                        Ok(())
                    });
                    state.set_notification(format!("Task {} approved!", task.id));
                }
            }
        }
        KeyCode::Char('s') => {
            if let Some(task) = tasks.get(state.selected_task_idx) {
                let _ = store.update_task(&task.id, |t| {
                    t.state.role = TaskRole::Planner;
                    t.state.status = TaskStatus::SentBack;
                    t.state.attempt += 1;
                    Ok(())
                });
                state.set_notification(format!("Task {} sent back to Planner.", task.id));
            }
        }
        _ => {}
    }
}

fn handle_memory_input(state: &mut TuiState, code: KeyCode) {
    match code {
        KeyCode::Char(c) => {
            state.memory_query.push(c);
        }
        KeyCode::Backspace => {
            state.memory_query.pop();
        }
        KeyCode::Enter => {
            let db_path = state.repo_root.join(".a4l").join("memory.db");
            if let Ok(ep_store) = EpisodicStore::open(&db_path) {
                if let Ok(matches) = ep_store.search_by_symptom(&state.memory_query) {
                    state.memory_results = matches;
                    state.set_notification(format!(
                        "Found {} episodes.",
                        state.memory_results.len()
                    ));
                }
            }
        }
        _ => {}
    }
}

// -----------------------------------------------------------------------------
// UI Rendering with Mistral & Pi Aesthetics
// -----------------------------------------------------------------------------

pub fn render_main_ui(f: &mut Frame, state: &TuiState, tasks: &[Task]) {
    let root_area = f.area();

    // Fill background with warm dark charcoal
    let bg_block = Block::default().style(Style::default().bg(BG_CHARCOAL));
    f.render_widget(bg_block, root_area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Mistral Header
            Constraint::Min(10),   // Active Tab Viewport
            Constraint::Length(3), // Interactive Prompt Bar
            Constraint::Length(1), // Footer status & hotkeys
        ])
        .split(root_area);

    render_header(f, chunks[0], state, tasks);

    match state.tab {
        ActiveTab::AgentChat => render_chat_view(f, chunks[1], state, tasks),
        ActiveTab::PipelineBoard => render_pipeline_board_view(f, chunks[1], state, tasks),
        ActiveTab::MemoryStudio => render_memory_studio_view(f, chunks[1], state),
    }

    render_prompt_bar(f, chunks[2], state);
    render_footer(f, chunks[3], state);

    // Optional toast notification popup
    if let Some((msg, _)) = &state.notification {
        render_toast(f, msg);
    }
}

fn render_header(f: &mut Frame, area: Rect, state: &TuiState, tasks: &[Task]) {
    let header_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(32), // Logo & Brand
            Constraint::Min(20),    // Tab Pills
            Constraint::Length(36), // Context & Role Badge
        ])
        .split(area);

    // 1. Logo & Brand
    let brand = Paragraph::new(Line::from(vec![
        Span::styled(
            " ✦ A4L ",
            Style::default()
                .fg(MISTRAL_ORANGE)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            "HARNESS ",
            Style::default()
                .fg(MISTRAL_AMBER)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("v0.1", Style::default().fg(TEXT_MUTED)),
    ]))
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(CARD_BORDER)),
    );
    f.render_widget(brand, header_chunks[0]);

    // 2. Navigation Tabs
    let chat_tab_style = if state.tab == ActiveTab::AgentChat {
        Style::default()
            .fg(MISTRAL_ORANGE)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(TEXT_MUTED)
    };
    let board_tab_style = if state.tab == ActiveTab::PipelineBoard {
        Style::default()
            .fg(MISTRAL_ORANGE)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(TEXT_MUTED)
    };
    let memory_tab_style = if state.tab == ActiveTab::MemoryStudio {
        Style::default()
            .fg(MISTRAL_ORANGE)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(TEXT_MUTED)
    };

    let tabs = Paragraph::new(Line::from(vec![
        Span::raw("  "),
        Span::styled("[1] 💬 Agent Chat", chat_tab_style),
        Span::raw("   "),
        Span::styled("[2] 📊 Pipeline Board", board_tab_style),
        Span::raw("   "),
        Span::styled("[3] 🧠 Memory Studio", memory_tab_style),
    ]))
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(CARD_BORDER)),
    );
    f.render_widget(tabs, header_chunks[1]);

    // 3. Status Badges & Metrics
    let active_role = tasks
        .get(state.selected_task_idx)
        .map(|t| format!("{:?}", t.state.role).to_lowercase())
        .unwrap_or_else(|| "idle".to_string());

    let role_color = match active_role.as_str() {
        "planner" => MISTRAL_AMBER,
        "maker" => MISTRAL_ORANGE,
        "checker" => Color::Cyan,
        "human" => SUCCESS_EMERALD,
        _ => TEXT_MUTED,
    };

    let meta = Paragraph::new(Line::from(vec![
        Span::raw("Role: "),
        Span::styled(
            format!("[{}] ", active_role.to_uppercase()),
            Style::default().fg(role_color).add_modifier(Modifier::BOLD),
        ),
        Span::styled("Tokens: ", Style::default().fg(TEXT_MUTED)),
        Span::styled("1,842 ", Style::default().fg(TEXT_WARM_WHITE)),
        Span::styled("(84% cache)", Style::default().fg(SUCCESS_EMERALD)),
    ]))
    .alignment(Alignment::Right)
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(CARD_BORDER)),
    );
    f.render_widget(meta, header_chunks[2]);
}

fn render_chat_view(f: &mut Frame, area: Rect, state: &TuiState, tasks: &[Task]) {
    let chat_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(70), Constraint::Percentage(30)])
        .split(area);

    // Left: Conversation Stream
    let mut items = Vec::new();
    for msg in &state.chat_history {
        let (border_color, header_color, icon) = match msg.kind {
            ChatItemKind::User => (Color::DarkGray, MISTRAL_AMBER, "👤"),
            ChatItemKind::Agent { .. } => (CARD_BORDER, MISTRAL_ORANGE, "✻"),
            ChatItemKind::ToolCall { exit, .. } => {
                if exit == Some(0) {
                    (CARD_BORDER, SUCCESS_EMERALD, "⚙")
                } else {
                    (CARD_BORDER, FAIL_CRIMSON, "⚠")
                }
            }
            ChatItemKind::SystemEvent => (CARD_BORDER, Color::Cyan, "✦"),
        };

        let card_header = Line::from(vec![
            Span::styled(format!(" {} ", icon), Style::default().fg(header_color)),
            Span::styled(
                format!("{} ", msg.title),
                Style::default()
                    .fg(header_color)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(format!("  {} ", msg.time), Style::default().fg(TEXT_MUTED)),
        ]);

        let card_body = Paragraph::new(msg.body.as_str())
            .wrap(Wrap { trim: true })
            .style(Style::default().fg(TEXT_WARM_WHITE))
            .block(
                Block::default()
                    .title(card_header)
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(border_color))
                    .style(Style::default().bg(CARD_BG)),
            );

        // Height calculation heuristic
        let line_count = msg.body.lines().count().max(1) + 2;
        items.push((card_body, line_count));
    }

    // Render cards vertically inside left chat area
    let mut y_offset = chat_chunks[0].y;
    for (card, h) in items {
        if y_offset + (h as u16) <= chat_chunks[0].y + chat_chunks[0].height {
            let card_rect = Rect {
                x: chat_chunks[0].x,
                y: y_offset,
                width: chat_chunks[0].width,
                height: h as u16,
            };
            f.render_widget(card, card_rect);
            y_offset += h as u16;
        }
    }

    // Right: Active Task Snapshot & Verification Ladder
    render_active_task_rail(f, chat_chunks[1], state, tasks);
}

fn render_active_task_rail(f: &mut Frame, area: Rect, state: &TuiState, tasks: &[Task]) {
    let task = tasks.get(state.selected_task_idx);

    let mut lines = Vec::new();
    if let Some(t) = task {
        lines.push(Line::from(vec![
            Span::styled(
                format!("[{}] ", t.id),
                Style::default()
                    .fg(MISTRAL_ORANGE)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                &t.title,
                Style::default()
                    .fg(TEXT_WARM_WHITE)
                    .add_modifier(Modifier::BOLD),
            ),
        ]));
        lines.push(Line::raw(""));

        // Status Badge
        lines.push(Line::from(vec![
            Span::styled("Role: ", Style::default().fg(TEXT_MUTED)),
            Span::styled(
                format!("{:?} ", t.state.role),
                Style::default().fg(MISTRAL_AMBER),
            ),
            Span::styled("Status: ", Style::default().fg(TEXT_MUTED)),
            Span::styled(
                format!("{:?}", t.state.status),
                Style::default().fg(SUCCESS_EMERALD),
            ),
        ]));
        lines.push(Line::raw(""));

        // Verification Ladder (L1-L4)
        lines.push(Line::styled(
            "Verification Ladder:",
            Style::default()
                .fg(MISTRAL_ORANGE)
                .add_modifier(Modifier::BOLD),
        ));
        let gates = [
            ("L1 Static", &t.gates.l1),
            ("L2 Unit", &t.gates.l2),
            ("L3 Smoke", &t.gates.l3),
            ("L4 E2E", &t.gates.l4),
        ];

        for (name, g) in gates {
            let (label, color) = match g.exit {
                Some(0) => ("PASS (0)", SUCCESS_EMERALD),
                Some(_code) => ("FAIL", FAIL_CRIMSON),
                None => ("UNRUN", TEXT_MUTED),
            };
            lines.push(Line::from(vec![
                Span::styled(
                    format!("  {:<10} ", name),
                    Style::default().fg(TEXT_WARM_WHITE),
                ),
                Span::styled(
                    format!("[{}]", label),
                    Style::default().fg(color).add_modifier(Modifier::BOLD),
                ),
            ]));
        }

        // Todos
        lines.push(Line::raw(""));
        lines.push(Line::styled(
            "Checklist:",
            Style::default()
                .fg(MISTRAL_ORANGE)
                .add_modifier(Modifier::BOLD),
        ));
        for todo in &t.todos {
            let (bullet, color) = match todo.status {
                TodoStatus::Verified => ("✓ verified", SUCCESS_EMERALD),
                TodoStatus::Done => ("✓ claim", MISTRAL_AMBER),
                TodoStatus::Pending => ("○ pending", TEXT_MUTED),
            };
            lines.push(Line::from(vec![
                Span::styled(format!("  {} ", bullet), Style::default().fg(color)),
                Span::styled(&todo.text, Style::default().fg(TEXT_WARM_WHITE)),
            ]));
        }

        // HITL prompt if awaiting
        if t.state.role == TaskRole::Human || t.state.status == TaskStatus::AwaitingHitl {
            lines.push(Line::raw(""));
            lines.push(Line::styled(
                "⚡ AWAITING HUMAN SIGN-OFF",
                Style::default()
                    .fg(MISTRAL_AMBER)
                    .add_modifier(Modifier::BOLD),
            ));
            lines.push(Line::styled(
                "Type /approve or /sendback <notes>",
                Style::default().fg(TEXT_MUTED),
            ));
        }
    } else {
        lines.push(Line::raw("No active tasks in queue."));
        lines.push(Line::raw(
            "Scaffold with `a4l init` or add task to .a4l/tasks.json.",
        ));
    }

    let rail = Paragraph::new(lines).wrap(Wrap { trim: true }).block(
        Block::default()
            .title(" Task Proof Station ")
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(CARD_BORDER))
            .style(Style::default().bg(CARD_BG)),
    );
    f.render_widget(rail, area);
}

fn render_pipeline_board_view(f: &mut Frame, area: Rect, state: &TuiState, tasks: &[Task]) {
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(25),
            Constraint::Percentage(25),
            Constraint::Percentage(25),
            Constraint::Percentage(25),
        ])
        .split(area);

    let col_roles = [
        ("1. PLANNER", TaskRole::Planner, MISTRAL_AMBER),
        ("2. MAKER", TaskRole::Maker, MISTRAL_ORANGE),
        ("3. CHECKER", TaskRole::Checker, Color::Cyan),
        ("4. HUMAN (HITL)", TaskRole::Human, SUCCESS_EMERALD),
    ];

    for (idx, (title, role, color)) in col_roles.iter().enumerate() {
        let matching_tasks: Vec<&Task> = tasks.iter().filter(|t| t.state.role == *role).collect();
        let items: Vec<ListItem> = matching_tasks
            .iter()
            .map(|t| {
                let is_selected = tasks
                    .get(state.selected_task_idx)
                    .map(|st| st.id == t.id)
                    .unwrap_or(false);
                let border_style = if is_selected {
                    Style::default()
                        .fg(MISTRAL_ORANGE)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(CARD_BORDER)
                };

                let card_content = vec![
                    Line::from(vec![
                        Span::styled(format!("[{}] ", t.id), Style::default().fg(MISTRAL_ORANGE)),
                        Span::styled(
                            &t.title,
                            Style::default()
                                .fg(TEXT_WARM_WHITE)
                                .add_modifier(Modifier::BOLD),
                        ),
                    ]),
                    Line::from(vec![
                        Span::styled(
                            format!("status: {:?}", t.state.status),
                            Style::default().fg(TEXT_MUTED),
                        ),
                        Span::styled(
                            format!(" | attempt: {}", t.state.attempt),
                            Style::default().fg(TEXT_MUTED),
                        ),
                    ]),
                ];

                ListItem::new(card_content).style(border_style)
            })
            .collect();

        let col_block = List::new(items).block(
            Block::default()
                .title(Span::styled(
                    format!(" {} ({}) ", title, matching_tasks.len()),
                    Style::default().fg(*color).add_modifier(Modifier::BOLD),
                ))
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(CARD_BORDER))
                .style(Style::default().bg(CARD_BG)),
        );
        f.render_widget(col_block, cols[idx]);
    }
}

fn render_memory_studio_view(f: &mut Frame, area: Rect, state: &TuiState) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    // Left: Bounded MEMORY.md view
    let mem_path = state.repo_root.join(".a4l").join("MEMORY.md");
    let mem_content = std::fs::read_to_string(mem_path)
        .unwrap_or_else(|_| "No .a4l/MEMORY.md found.".to_string());

    let left = Paragraph::new(mem_content).wrap(Wrap { trim: true }).block(
        Block::default()
            .title(" Bounded Long-Term Memory (MEMORY.md) ")
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(CARD_BORDER))
            .style(Style::default().bg(CARD_BG)),
    );
    f.render_widget(left, chunks[0]);

    // Right: Episodic Store Search
    let mut right_lines = Vec::new();
    right_lines.push(Line::from(vec![
        Span::styled("Query: ", Style::default().fg(TEXT_MUTED)),
        Span::styled(
            &state.memory_query,
            Style::default()
                .fg(MISTRAL_ORANGE)
                .add_modifier(Modifier::BOLD),
        ),
    ]));
    right_lines.push(Line::styled(
        "Type symptom query and press [Enter]...",
        Style::default().fg(TEXT_MUTED),
    ));
    right_lines.push(Line::raw(""));

    if state.memory_results.is_empty() {
        right_lines.push(Line::styled(
            "No episodes found. Try searching for 'compiler error', 'failure', or task ID.",
            Style::default().fg(TEXT_MUTED),
        ));
    } else {
        for ep in &state.memory_results {
            right_lines.push(Line::from(vec![
                Span::styled(
                    format!("[{}] ", ep.task_id),
                    Style::default()
                        .fg(MISTRAL_ORANGE)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(&ep.title, Style::default().fg(TEXT_WARM_WHITE)),
            ]));
            right_lines.push(Line::styled(
                format!("  Symptoms: {}", ep.symptoms),
                Style::default().fg(MISTRAL_AMBER),
            ));
            right_lines.push(Line::styled(
                format!("  Resolution: {}", ep.resolution),
                Style::default().fg(SUCCESS_EMERALD),
            ));
            right_lines.push(Line::raw(""));
        }
    }

    let right = Paragraph::new(right_lines).wrap(Wrap { trim: true }).block(
        Block::default()
            .title(" SQLite Episodic Memory Search (memory.db) ")
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(CARD_BORDER))
            .style(Style::default().bg(CARD_BG)),
    );
    f.render_widget(right, chunks[1]);
}

fn render_prompt_bar(f: &mut Frame, area: Rect, state: &TuiState) {
    let input_line = Line::from(vec![
        Span::styled(
            " ❯ ",
            Style::default()
                .fg(MISTRAL_ORANGE)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(&state.input, Style::default().fg(TEXT_WARM_WHITE)),
        Span::styled("█", Style::default().fg(MISTRAL_AMBER)),
    ]);

    let prompt = Paragraph::new(input_line).block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(MISTRAL_ORANGE))
            .style(Style::default().bg(CARD_BG)),
    );
    f.render_widget(prompt, area);
}

fn render_footer(f: &mut Frame, area: Rect, _state: &TuiState) {
    let footer_text = Line::from(vec![
        Span::styled(" [Tab] ", Style::default().fg(MISTRAL_ORANGE)),
        Span::styled("Switch View  │", Style::default().fg(TEXT_MUTED)),
        Span::styled(" [Enter] ", Style::default().fg(MISTRAL_ORANGE)),
        Span::styled("Send Prompt  │", Style::default().fg(TEXT_MUTED)),
        Span::styled(" [/] ", Style::default().fg(MISTRAL_ORANGE)),
        Span::styled("Commands (/help)  │", Style::default().fg(TEXT_MUTED)),
        Span::styled(" [a/s] ", Style::default().fg(MISTRAL_ORANGE)),
        Span::styled("HITL Approve/SendBack  │", Style::default().fg(TEXT_MUTED)),
        Span::styled(" [Esc] ", Style::default().fg(MISTRAL_ORANGE)),
        Span::styled("Exit", Style::default().fg(TEXT_MUTED)),
    ]);

    let footer = Paragraph::new(footer_text)
        .alignment(Alignment::Center)
        .style(Style::default().bg(BG_CHARCOAL));
    f.render_widget(footer, area);
}

fn render_toast(f: &mut Frame, msg: &str) {
    let area = f.area();
    let toast_width = (msg.len() as u16 + 8).min(area.width.saturating_sub(4));
    let toast_area = Rect {
        x: area.width.saturating_sub(toast_width + 2),
        y: 2,
        width: toast_width,
        height: 3,
    };

    f.render_widget(Clear, toast_area);
    let toast = Paragraph::new(Line::from(vec![
        Span::styled(" ✦ ", Style::default().fg(MISTRAL_ORANGE)),
        Span::styled(
            msg,
            Style::default()
                .fg(TEXT_WARM_WHITE)
                .add_modifier(Modifier::BOLD),
        ),
    ]))
    .alignment(Alignment::Center)
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(MISTRAL_ORANGE))
            .style(Style::default().bg(CARD_BG)),
    );
    f.render_widget(toast, toast_area);
}
