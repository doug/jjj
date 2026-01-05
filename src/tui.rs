use crate::error::Result;
use crate::models::Task;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::{Backend, CrosstermBackend},
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Span, Line},
    widgets::{Block, Borders, List, ListItem, Tabs},
    Frame, Terminal,
};
use std::{io, time::Duration};

struct App {
    tasks: Vec<Task>,
    columns: Vec<String>,
    selected_column: usize,
    selected_task: Option<usize>,
}

impl App {
    fn new(tasks: Vec<Task>, columns: Vec<String>) -> Self {
        Self {
            tasks,
            columns,
            selected_column: 0,
            selected_task: None,
        }
    }
}

/// Launch the interactive Kanban board TUI
pub fn launch_board(tasks: Vec<Task>, columns: Vec<String>) -> Result<()> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app state
    let app = App::new(tasks, columns);

    // Run app
    let res = run_app(&mut terminal, app);

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("{:?}", err);
    }

    Ok(())
}

fn run_app<B: Backend>(terminal: &mut Terminal<B>, mut app: App) -> io::Result<()> {
    loop {
        terminal.draw(|f| ui(f, &mut app))?;

        if crossterm::event::poll(Duration::from_millis(250))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') => return Ok(()),
                    KeyCode::Right => {
                        app.selected_column = (app.selected_column + 1) % app.columns.len();
                        app.selected_task = None; // Reset task selection when changing column
                    }
                    KeyCode::Left => {
                        if app.selected_column > 0 {
                            app.selected_column -= 1;
                        } else {
                            app.selected_column = app.columns.len() - 1;
                        }
                        app.selected_task = None;
                    }
                    KeyCode::Down => {
                        let current_col = &app.columns[app.selected_column];
                        let task_count = app.tasks.iter().filter(|t| &t.column == current_col).count();
                        if task_count > 0 {
                            let next = match app.selected_task {
                                Some(i) => {
                                    if i >= task_count - 1 {
                                        0
                                    } else {
                                        i + 1
                                    }
                                }
                                None => 0,
                            };
                            app.selected_task = Some(next);
                        }
                    }
                    KeyCode::Up => {
                        let current_col = &app.columns[app.selected_column];
                        let task_count = app.tasks.iter().filter(|t| &t.column == current_col).count();
                        if task_count > 0 {
                            let next = match app.selected_task {
                                Some(i) => {
                                    if i == 0 {
                                        task_count - 1
                                    } else {
                                        i - 1
                                    }
                                }
                                None => task_count - 1,
                            };
                            app.selected_task = Some(next);
                        }
                    }
                    _ => {}
                }
            }
        }
    }
}

fn ui(f: &mut Frame, app: &mut App) {
    let size = f.area();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(0)].as_ref())
        .split(size);

    let titles: Vec<Line> = app
        .columns
        .iter()
        .map(|t| {
            let (first, rest) = t.split_at(1);
            Line::from(vec![
                Span::styled(first, Style::default().fg(Color::Yellow)),
                Span::styled(rest, Style::default().fg(Color::Green)),
            ])
        })
        .collect();

    let tabs = Tabs::new(titles)
        .block(Block::default().borders(Borders::ALL).title("Board"))
        .select(app.selected_column)
        .style(Style::default().fg(Color::Cyan))
        .highlight_style(Style::default().add_modifier(Modifier::BOLD).bg(Color::DarkGray));

    f.render_widget(tabs, chunks[0]);

    // Render tasks for selected column
    let current_col = &app.columns[app.selected_column];
    let column_tasks: Vec<_> = app.tasks.iter().filter(|t| &t.column == current_col).collect();

    let items: Vec<ListItem> = column_tasks
        .iter()
        .map(|t| {
            let content = vec![Line::from(Span::styled(
                format!("{} - {}", t.id, t.title),
                Style::default().fg(Color::White),
            ))];
            ListItem::new(content)
        })
        .collect();

    let tasks_list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title(current_col.as_str()))
        .highlight_style(Style::default().add_modifier(Modifier::BOLD).fg(Color::Yellow))
        .highlight_symbol(">> ");

    let mut state = ratatui::widgets::ListState::default();
    state.select(app.selected_task);

    f.render_stateful_widget(tasks_list, chunks[1], &mut state);
}

