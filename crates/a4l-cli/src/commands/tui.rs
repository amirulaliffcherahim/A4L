use a4l_core::state::{CircuitBreakerStatus, StateStore, Task, TaskRole, TaskStatus, TodoStatus};
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::{Backend, CrosstermBackend},
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame, Terminal,
};
use std::io;
use std::path::Path;

pub fn run_tui(repo_root: &Path) -> anyhow::Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let res = run_app(&mut terminal, repo_root);

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

fn run_app<B: Backend>(terminal: &mut Terminal<B>, repo_root: &Path) -> io::Result<()> {
    let tasks_file = repo_root.join(".a4l").join("tasks.json");
    let store = StateStore::new(&tasks_file);

    let mut selected_idx = 0;

    loop {
        let tasks = store.load().unwrap_or_default();
        let queue_len = tasks.queue.len();

        if selected_idx >= queue_len && queue_len > 0 {
            selected_idx = queue_len - 1;
        }

        terminal.draw(|f| {
            render_ui(f, &tasks.queue, selected_idx);
        })?;

        if event::poll(std::time::Duration::from_millis(250))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') | KeyCode::Esc => return Ok(()),
                    KeyCode::Up => {
                        selected_idx = selected_idx.saturating_sub(1);
                    }
                    KeyCode::Down if queue_len > 0 && selected_idx + 1 < queue_len => {
                        selected_idx += 1;
                    }
                    KeyCode::Char('a') => {
                        // Approve active task if in Human role
                        if let Some(task) = tasks.queue.get(selected_idx) {
                            if task.state.role == TaskRole::Human {
                                let _ = a4l_core::guards::can_advance(
                                    task,
                                    TaskRole::Human,
                                    TaskStatus::Completed,
                                );
                                let _ = store.update_task(&task.id, |t| {
                                    t.state.status = TaskStatus::Completed;
                                    Ok(())
                                });
                            }
                        }
                    }
                    KeyCode::Char('s') => {
                        // Send back to Planner if in Human role
                        if let Some(task) = tasks.queue.get(selected_idx) {
                            if task.state.role == TaskRole::Human {
                                let _ = store.update_task(&task.id, |t| {
                                    t.state.role = TaskRole::Planner;
                                    t.state.status = TaskStatus::SentBack;
                                    t.state.attempt += 1;
                                    Ok(())
                                });
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
    }
}

pub fn render_ui(f: &mut Frame, tasks: &[Task], selected_idx: usize) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(10),
            Constraint::Length(3),
        ])
        .split(f.area());

    // Title Bar
    let title = Paragraph::new(" A4L — Agentic 4 All [Interactive Pipeline Dashboard]")
        .style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(title, chunks[0]);

    // Main Columns: Left = Task Queue, Right = Task Detail / Gates
    let body_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
        .split(chunks[1]);

    // Task List
    let items: Vec<ListItem> = tasks
        .iter()
        .enumerate()
        .map(|(i, t)| {
            let style = if i == selected_idx {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };
            let status_color = match t.state.status {
                TaskStatus::Completed => Color::Green,
                TaskStatus::InProgress => Color::Yellow,
                TaskStatus::AwaitingHitl => Color::Magenta,
                _ => Color::Gray,
            };
            let line = Line::from(vec![
                Span::styled(format!("[{}] ", t.id), Style::default().fg(Color::Cyan)),
                Span::styled(format!("{:<20} ", t.title), style),
                Span::styled(
                    format!("{:?}", t.state.status),
                    Style::default().fg(status_color),
                ),
            ]);
            ListItem::new(line)
        })
        .collect();

    let task_list =
        List::new(items).block(Block::default().title(" Task Queue ").borders(Borders::ALL));
    f.render_widget(task_list, body_chunks[0]);

    // Task Details & Gate Proofs
    if let Some(task) = tasks.get(selected_idx) {
        render_task_details(f, body_chunks[1], task);
    } else {
        let empty = Paragraph::new("No task selected or queue is empty.")
            .block(Block::default().title(" Detail ").borders(Borders::ALL));
        f.render_widget(empty, body_chunks[1]);
    }

    // Footer Keys
    let footer_text = Line::from(vec![
        Span::raw(" [↑/↓] Select Task | "),
        Span::styled("[a] Approve (HITL) | ", Style::default().fg(Color::Green)),
        Span::styled("[s] Send Back | ", Style::default().fg(Color::Red)),
        Span::raw("[q/Esc] Quit"),
    ]);
    let footer = Paragraph::new(footer_text).block(Block::default().borders(Borders::ALL));
    f.render_widget(footer, chunks[2]);
}

fn render_task_details(f: &mut Frame, area: Rect, task: &Task) {
    let mut lines = Vec::new();

    lines.push(Line::from(vec![
        Span::styled("Task: ", Style::default().add_modifier(Modifier::BOLD)),
        Span::styled(
            format!("{} - {}", task.id, task.title),
            Style::default().fg(Color::Cyan),
        ),
    ]));
    lines.push(Line::from(vec![
        Span::styled("Role: ", Style::default().add_modifier(Modifier::BOLD)),
        Span::styled(
            format!("{}  ", task.state.role),
            Style::default().fg(Color::Yellow),
        ),
        Span::styled("Status: ", Style::default().add_modifier(Modifier::BOLD)),
        Span::styled(
            format!("{:?}  ", task.state.status),
            Style::default().fg(Color::Green),
        ),
        Span::styled(
            format!("Attempt: {}", task.state.attempt),
            Style::default().fg(Color::Gray),
        ),
    ]));

    lines.push(Line::raw(""));
    lines.push(Line::styled(
        "Verification Gates (L1-L4):",
        Style::default().add_modifier(Modifier::BOLD),
    ));

    let gates = [
        ("L1 Static", &task.gates.l1),
        ("L2 Unit", &task.gates.l2),
        ("L3 Smoke", &task.gates.l3),
        ("L4 E2E", &task.gates.l4),
    ];

    for (name, gate) in gates {
        let status_span = match gate.exit {
            Some(0) => Span::styled(
                " PASS (0) ",
                Style::default().bg(Color::Green).fg(Color::Black),
            ),
            Some(code) => Span::styled(
                format!(" FAIL ({}) ", code),
                Style::default().bg(Color::Red).fg(Color::White),
            ),
            None => Span::styled(
                " UNRUN ",
                Style::default().bg(Color::DarkGray).fg(Color::White),
            ),
        };
        lines.push(Line::from(vec![
            Span::raw(format!("  {:<12} ", name)),
            status_span,
            Span::raw(format!("  cmd: {}", gate.cmd)),
        ]));
    }

    lines.push(Line::raw(""));
    lines.push(Line::styled(
        "TODO Items:",
        Style::default().add_modifier(Modifier::BOLD),
    ));
    for todo in &task.todos {
        let check = match todo.status {
            TodoStatus::Verified => {
                Span::styled("[✓ verified] ", Style::default().fg(Color::Green))
            }
            TodoStatus::Done => Span::styled("[✓ claim]    ", Style::default().fg(Color::Yellow)),
            TodoStatus::Pending => Span::styled("[ ]          ", Style::default().fg(Color::Gray)),
        };
        lines.push(Line::from(vec![
            Span::raw("  "),
            check,
            Span::raw(&todo.text),
        ]));
    }

    if task.circuit_breaker.status == CircuitBreakerStatus::Tripped {
        lines.push(Line::raw(""));
        lines.push(Line::from(vec![
            Span::styled(
                "⚡ CIRCUIT BREAKER TRIPPED: ",
                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("{:?}", task.circuit_breaker.reason),
                Style::default().fg(Color::Yellow),
            ),
        ]));
    }

    let detail = Paragraph::new(lines).block(
        Block::default()
            .title(" Task Proof & Gates ")
            .borders(Borders::ALL),
    );
    f.render_widget(detail, area);
}
