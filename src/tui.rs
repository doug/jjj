use crate::error::Result;
use crate::models::{Solution, SolutionStatus};
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::{Backend, CrosstermBackend},
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Tabs},
    Frame, Terminal,
};
use std::{io, time::Duration};

const STATUSES: [SolutionStatus; 4] = [
    SolutionStatus::Proposed,
    SolutionStatus::Testing,
    SolutionStatus::Accepted,
    SolutionStatus::Refuted,
];

struct App {
    solutions: Vec<Solution>,
    selected_status: usize,
    selected_solution: Option<usize>,
}

impl App {
    fn new(solutions: Vec<Solution>) -> Self {
        Self {
            solutions,
            selected_status: 0,
            selected_solution: None,
        }
    }

    fn current_status(&self) -> &SolutionStatus {
        &STATUSES[self.selected_status]
    }

    fn solutions_in_current_status(&self) -> Vec<&Solution> {
        self.solutions
            .iter()
            .filter(|s| &s.status == self.current_status())
            .collect()
    }
}

/// Launch the interactive Kanban board TUI
pub fn launch_board(solutions: Vec<Solution>) -> Result<()> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app state
    let app = App::new(solutions);

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
                        app.selected_status = (app.selected_status + 1) % STATUSES.len();
                        app.selected_solution = None;
                    }
                    KeyCode::Left => {
                        if app.selected_status > 0 {
                            app.selected_status -= 1;
                        } else {
                            app.selected_status = STATUSES.len() - 1;
                        }
                        app.selected_solution = None;
                    }
                    KeyCode::Down => {
                        let solution_count = app.solutions_in_current_status().len();
                        if solution_count > 0 {
                            let next = match app.selected_solution {
                                Some(i) => {
                                    if i >= solution_count - 1 {
                                        0
                                    } else {
                                        i + 1
                                    }
                                }
                                None => 0,
                            };
                            app.selected_solution = Some(next);
                        }
                    }
                    KeyCode::Up => {
                        let solution_count = app.solutions_in_current_status().len();
                        if solution_count > 0 {
                            let next = match app.selected_solution {
                                Some(i) => {
                                    if i == 0 {
                                        solution_count - 1
                                    } else {
                                        i - 1
                                    }
                                }
                                None => solution_count - 1,
                            };
                            app.selected_solution = Some(next);
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

    let titles: Vec<Line> = STATUSES
        .iter()
        .map(|s| {
            let name = format!("{}", s);
            let first = name.chars().next().unwrap().to_string();
            let rest = name.chars().skip(1).collect::<String>();
            Line::from(vec![
                Span::styled(first, Style::default().fg(Color::Yellow)),
                Span::styled(rest, Style::default().fg(Color::Green)),
            ])
        })
        .collect();

    let tabs = Tabs::new(titles)
        .block(Block::default().borders(Borders::ALL).title("Solutions"))
        .select(app.selected_status)
        .style(Style::default().fg(Color::Cyan))
        .highlight_style(
            Style::default()
                .add_modifier(Modifier::BOLD)
                .bg(Color::DarkGray),
        );

    f.render_widget(tabs, chunks[0]);

    // Render solutions for selected status
    let status_solutions = app.solutions_in_current_status();
    let status_name = format!("{}", app.current_status());

    let items: Vec<ListItem> = status_solutions
        .iter()
        .map(|s| {
            let content = vec![Line::from(Span::styled(
                format!("{} - {} ({})", s.id, s.title, s.problem_id),
                Style::default().fg(Color::White),
            ))];
            ListItem::new(content)
        })
        .collect();

    let solutions_list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(status_name.as_str()),
        )
        .highlight_style(
            Style::default()
                .add_modifier(Modifier::BOLD)
                .fg(Color::Yellow),
        )
        .highlight_symbol(">> ");

    let mut state = ratatui::widgets::ListState::default();
    state.select(app.selected_solution);

    f.render_stateful_widget(solutions_list, chunks[1], &mut state);
}
