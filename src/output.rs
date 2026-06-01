//! Terminal-output gating for shared code paths.
//!
//! Several layers (automation, storage warnings, domain hooks) emit
//! informational or warning text to stdout/stderr. These layers are shared
//! between the CLI and the TUI. When the TUI owns the alternate screen, a
//! raw `println!`/`eprintln!` scrolls the alt-screen and leaves the footer /
//! status bar visibly "stacked".
//!
//! Callers route their messages through [`notify`] and [`warn`], which are
//! suppressed while the TUI is active (set via [`set_tui_active`]). The TUI
//! surfaces user-facing feedback through its own flash mechanism instead.

use std::sync::atomic::{AtomicBool, Ordering};

static TUI_ACTIVE: AtomicBool = AtomicBool::new(false);

/// Mark whether the interactive TUI currently owns the terminal.
///
/// While active, [`notify`] and [`warn`] become no-ops so shared code paths
/// don't corrupt the alternate screen.
pub fn set_tui_active(active: bool) {
    TUI_ACTIVE.store(active, Ordering::Relaxed);
}

/// Whether the TUI currently owns the terminal.
pub fn tui_active() -> bool {
    TUI_ACTIVE.load(Ordering::Relaxed)
}

/// Print an informational message to stdout, unless the TUI is active.
pub fn notify(msg: &str) {
    if !tui_active() {
        println!("{}", msg);
    }
}

/// Print a warning to stderr (prefixed with `Warning:`), unless the TUI is active.
pub fn warn(msg: &str) {
    if !tui_active() {
        eprintln!("Warning: {}", msg);
    }
}
