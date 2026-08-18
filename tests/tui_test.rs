//! Drives the interactive TUI without a terminal.
//!
//! `App::run` is a thin loop: poll crossterm, hand the key to `App::handle_key`,
//! redraw. Tests skip the polling and call `handle_key` directly, rendering into
//! ratatui's `TestBackend` so the resulting screen can be asserted on as text.
//!
//! Why this file exists: the TUI is the tool's recommended entry point and its
//! largest module, and until now nothing verified it. Ordering semantics in
//! particular (nudge, fling, sized gaps, undo) changed twice in 2026 with no
//! automated check that the keys still do what the docs say.

mod test_helpers;

use std::path::Path;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use jjj::tui::App;
use ratatui::{backend::TestBackend, Terminal};
use test_helpers::{jj_available, run_jjj_success, setup_test_repo};

/// A driver bundling an app with the terminal it renders into.
struct Tui {
    app: App,
    terminal: Terminal<TestBackend>,
}

impl Tui {
    fn open(repo: &Path) -> Self {
        let app = App::open_at(repo.to_path_buf()).expect("open TUI against test repo");
        let terminal = Terminal::new(TestBackend::new(140, 40)).expect("test backend");
        Self { app, terminal }
    }

    /// Press a plain key.
    fn key(&mut self, code: KeyCode) -> &mut Self {
        self.app
            .handle_key(KeyEvent::from(code))
            .expect("handle key");
        self
    }

    /// Press a key with modifiers (Shift for nudges, Ctrl for undo).
    fn key_mod(&mut self, code: KeyCode, mods: KeyModifiers) -> &mut Self {
        self.app
            .handle_key(KeyEvent::new(code, mods))
            .expect("handle key");
        self
    }

    /// Type a sequence of plain character keys.
    fn typed(&mut self, text: &str) -> &mut Self {
        for ch in text.chars() {
            self.key(KeyCode::Char(ch));
        }
        self
    }

    /// Render a frame and return the screen as text, one line per row.
    fn screen(&mut self) -> String {
        self.terminal
            .draw(|f| jjj::tui::ui::draw(f, &self.app))
            .expect("draw");
        let buffer = self.terminal.backend().buffer();
        let area = *buffer.area();
        (0..area.height)
            .map(|y| {
                (0..area.width)
                    .map(|x| buffer[(x, y)].symbol())
                    .collect::<String>()
                    .trim_end()
                    .to_string()
            })
            .collect::<Vec<_>>()
            .join("\n")
    }
}

/// A repo with a milestone and three ranked problems — enough for the ordering
/// keys to have something to move.
fn repo_with_backlog() -> tempfile::TempDir {
    let repo = setup_test_repo();
    run_jjj_success(repo.path(), &["milestone", "new", "Q1"]);
    for title in ["Alpha", "Beta", "Gamma"] {
        run_jjj_success(
            repo.path(),
            &["problem", "new", title, "--milestone", "Q1", "--force"],
        );
    }
    repo
}

// =============================================================================
// Rendering
// =============================================================================

#[test]
fn renders_the_project_without_panicking() {
    if !jj_available() {
        return;
    }
    let repo = repo_with_backlog();
    let mut tui = Tui::open(repo.path());
    let screen = tui.screen();

    assert!(screen.contains("Alpha"), "backlog should render: {screen}");
    assert!(screen.contains("Beta"));
    assert!(screen.contains("Gamma"));
}

#[test]
fn renders_at_a_narrow_width_without_panicking() {
    if !jj_available() {
        return;
    }
    let repo = repo_with_backlog();
    let app = App::open_at(repo.path().to_path_buf()).expect("open");
    // Layout maths that assumes room for both panes is a classic source of
    // subtract-with-overflow panics; 40 columns is narrower than the split.
    let mut terminal = Terminal::new(TestBackend::new(40, 12)).expect("backend");
    terminal
        .draw(|f| jjj::tui::ui::draw(f, &app))
        .expect("narrow render must not panic");
}

#[test]
fn help_overlay_opens_and_any_key_closes_it() {
    if !jj_available() {
        return;
    }
    let repo = repo_with_backlog();
    let mut tui = Tui::open(repo.path());

    tui.key(KeyCode::Char('?'));
    let helped = tui.screen();
    assert!(
        helped.to_lowercase().contains("help") || helped.contains("Keys"),
        "'?' should open help: {helped}"
    );

    tui.key(KeyCode::Esc);
    let closed = tui.screen();
    assert!(closed.contains("Alpha"), "help should close: {closed}");
}

// =============================================================================
// Navigation
// =============================================================================

#[test]
fn tab_moves_focus_between_panes() {
    if !jj_available() {
        return;
    }
    let repo = repo_with_backlog();
    let mut tui = Tui::open(repo.path());

    let start = format!("{:?}", tui.app.ui.focused_pane);
    tui.key(KeyCode::Tab);
    let after = format!("{:?}", tui.app.ui.focused_pane);
    assert_ne!(start, after, "Tab should change the focused pane");

    tui.key(KeyCode::Tab);
    assert_eq!(
        start,
        format!("{:?}", tui.app.ui.focused_pane),
        "Tab should cycle back"
    );
}

#[test]
fn j_and_k_move_the_tree_selection() {
    if !jj_available() {
        return;
    }
    let repo = repo_with_backlog();
    let mut tui = Tui::open(repo.path());

    let start = tui.app.ui.tree_index;
    tui.key(KeyCode::Char('j'));
    assert_eq!(tui.app.ui.tree_index, start + 1, "j moves down");

    tui.key(KeyCode::Char('k'));
    assert_eq!(tui.app.ui.tree_index, start, "k moves back up");
}

#[test]
fn selection_never_walks_off_the_top_of_the_tree() {
    if !jj_available() {
        return;
    }
    let repo = repo_with_backlog();
    let mut tui = Tui::open(repo.path());

    for _ in 0..20 {
        tui.key(KeyCode::Char('k'));
    }
    assert_eq!(
        tui.app.ui.tree_index, 0,
        "k at the top must clamp, not wrap"
    );
    tui.screen();
}

#[test]
fn detail_scroll_is_clamped_rather_than_blanking_the_pane() {
    if !jj_available() {
        return;
    }
    let repo = repo_with_backlog();
    let mut tui = Tui::open(repo.path());

    // Focus the detail pane, then over-scroll well past any content.
    tui.key(KeyCode::Tab);
    for _ in 0..200 {
        tui.key(KeyCode::Char('j'));
    }
    let screen = tui.screen();

    assert!(
        screen.chars().any(|c| !c.is_whitespace()),
        "over-scrolling blanked the detail pane"
    );
}

// =============================================================================
// Ordering: nudge, fling, gaps, undo
// =============================================================================

/// Titles in the milestone's personal ordering, top first.
///
/// A personal ordering is materialized lazily — the TUI only writes one once you
/// act on it — so before the first nudge this falls back to the order the
/// problems would be given by default. That is exactly what the user sees, and
/// it lets a test compare before and after without priming a file first.
fn ordering_titles(tui: &Tui) -> Vec<String> {
    let title_of = |id: &String| -> String {
        tui.app
            .data
            .problems
            .iter()
            .find(|p| &p.id == id)
            .map(|p| p.title.clone())
            .unwrap_or_else(|| id.clone())
    };

    match tui.app.ui.personal_orderings.values().next() {
        Some(ordering) => ordering.order.iter().map(title_of).collect(),
        None => tui
            .app
            .data
            .problems
            .iter()
            .filter(|p| p.milestone_id.is_some())
            .map(|p| p.title.clone())
            .collect(),
    }
}

/// Move the tree cursor onto the named problem.
fn select_problem(tui: &mut Tui, title: &str) {
    for _ in 0..40 {
        if tui.app.selected_title() == Some(title) {
            return;
        }
        tui.key(KeyCode::Char('j'));
    }
    panic!("never reached {title} in the tree");
}

#[test]
fn shift_k_nudges_the_selected_item_up_one_slot() {
    if !jj_available() {
        return;
    }
    let repo = repo_with_backlog();
    let mut tui = Tui::open(repo.path());

    let before = ordering_titles(&tui);
    let second = before[1].clone();
    select_problem(&mut tui, &second);

    tui.key_mod(KeyCode::Char('K'), KeyModifiers::SHIFT);

    let after = ordering_titles(&tui);
    assert_eq!(
        after[0], second,
        "Shift+K should move the selection up one slot: {before:?} -> {after:?}"
    );
    assert_eq!(
        after.len(),
        before.len(),
        "a nudge must not add or drop items"
    );
}

#[test]
fn shift_j_nudges_the_selected_item_down_one_slot() {
    if !jj_available() {
        return;
    }
    let repo = repo_with_backlog();
    let mut tui = Tui::open(repo.path());

    let before = ordering_titles(&tui);
    let first = before[0].clone();
    select_problem(&mut tui, &first);

    tui.key_mod(KeyCode::Char('J'), KeyModifiers::SHIFT);

    let after = ordering_titles(&tui);
    assert_eq!(
        after[1], first,
        "Shift+J should move the selection down one slot: {before:?} -> {after:?}"
    );
}

#[test]
fn ctrl_z_undoes_the_last_ordering_change() {
    if !jj_available() {
        return;
    }
    let repo = repo_with_backlog();
    let mut tui = Tui::open(repo.path());

    let before = ordering_titles(&tui);
    let second = before[1].clone();
    select_problem(&mut tui, &second);

    tui.key_mod(KeyCode::Char('K'), KeyModifiers::SHIFT);
    assert_ne!(ordering_titles(&tui), before, "precondition: order changed");

    tui.key_mod(KeyCode::Char('z'), KeyModifiers::CONTROL);
    assert_eq!(
        ordering_titles(&tui),
        before,
        "Ctrl+Z should restore the previous order"
    );
}

#[test]
fn p_cycles_the_gap_below_the_selected_item() {
    if !jj_available() {
        return;
    }
    let repo = repo_with_backlog();
    let mut tui = Tui::open(repo.path());

    let titles = ordering_titles(&tui);
    let first = titles[0].clone();
    select_problem(&mut tui, &first);

    let id = tui
        .app
        .data
        .problems
        .iter()
        .find(|p| p.title == first)
        .map(|p| p.id.clone())
        .expect("problem id");

    let gap_of = |tui: &Tui| -> Option<String> {
        tui.app
            .ui
            .personal_orderings
            .values()
            .next()
            .and_then(|o| o.gaps.get(&id))
            .map(|g| format!("{g:?}"))
    };

    assert_eq!(gap_of(&tui), None, "no gap is authored to begin with");

    // none -> S -> M -> L -> XL -> none. Each step must be distinct, and the
    // fifth must return to unset — a cycle that silently stuck at XL would look
    // fine on screen but make the gap unclearable.
    let mut seen = Vec::new();
    for _ in 0..4 {
        tui.key(KeyCode::Char('p'));
        seen.push(gap_of(&tui).expect("a gap should be set"));
    }
    assert_eq!(
        seen,
        vec!["S", "M", "L", "XL"],
        "gap cycle should step through every size in order"
    );

    tui.key(KeyCode::Char('p'));
    assert_eq!(gap_of(&tui), None, "the cycle must return to no gap");
}

#[test]
fn r_toggles_between_personal_and_global_ordering() {
    if !jj_available() {
        return;
    }
    let repo = repo_with_backlog();
    let mut tui = Tui::open(repo.path());

    let start = tui.app.ui.show_personal_ordering;
    tui.key(KeyCode::Char('r'));
    assert_ne!(
        tui.app.ui.show_personal_ordering, start,
        "'r' should switch the ordering view"
    );
    tui.screen();

    tui.key(KeyCode::Char('r'));
    assert_eq!(
        tui.app.ui.show_personal_ordering, start,
        "'r' should toggle"
    );
}

#[test]
fn ordering_changes_are_persisted_for_the_cli_to_read() {
    if !jj_available() {
        return;
    }
    let repo = repo_with_backlog();
    let mut tui = Tui::open(repo.path());

    let before = ordering_titles(&tui);
    let second = before[1].clone();
    select_problem(&mut tui, &second);
    tui.key_mod(KeyCode::Char('K'), KeyModifiers::SHIFT);
    tui.key(KeyCode::Char('p'));

    // The TUI and `jjj rank show` must agree — they are the two halves of the
    // ranking feature, and nothing else checks they read the same file.
    let shown = run_jjj_success(repo.path(), &["rank", "show", "Q1"]);
    assert!(
        shown.contains(&second),
        "the ordering authored in the TUI should reach `rank show`: {shown}"
    );
    assert!(
        !shown.contains("No rankings yet"),
        "the TUI wrote no ordering file: {shown}"
    );
}

// =============================================================================
// Selection and quitting
// =============================================================================

#[test]
fn space_toggles_multi_selection() {
    if !jj_available() {
        return;
    }
    let repo = repo_with_backlog();
    let mut tui = Tui::open(repo.path());
    select_problem(&mut tui, "Alpha");

    tui.key(KeyCode::Char(' '));
    assert_eq!(tui.app.ui.selected_ids.len(), 1, "Space should select");

    tui.key(KeyCode::Char(' '));
    assert!(
        tui.app.ui.selected_ids.is_empty(),
        "Space again should deselect"
    );
}

#[test]
fn q_and_ctrl_c_both_quit() {
    if !jj_available() {
        return;
    }
    let repo = repo_with_backlog();

    let mut tui = Tui::open(repo.path());
    tui.key(KeyCode::Char('q'));
    assert!(tui.app.should_quit, "q should quit");

    let mut tui = Tui::open(repo.path());
    tui.key_mod(KeyCode::Char('c'), KeyModifiers::CONTROL);
    assert!(tui.app.should_quit, "Ctrl+C should quit from any mode");
}

#[test]
fn ctrl_c_quits_even_while_typing_in_a_prompt() {
    if !jj_available() {
        return;
    }
    let repo = repo_with_backlog();
    let mut tui = Tui::open(repo.path());

    // Open the new-entity prompt and type into it.
    tui.key(KeyCode::Char('n')).typed("half a title");
    assert!(
        !tui.app.should_quit,
        "typing must not quit while in input mode"
    );

    tui.key_mod(KeyCode::Char('c'), KeyModifiers::CONTROL);
    assert!(
        tui.app.should_quit,
        "Ctrl+C is the escape hatch and must work from input mode too"
    );
}
