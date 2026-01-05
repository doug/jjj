// TUI module for interactive board and review interfaces
// This is a stub for future implementation using ratatui

use crate::error::Result;

/// Launch the interactive Kanban board TUI
pub fn launch_board() -> Result<()> {
    println!("Interactive TUI not yet implemented.");
    println!("Use 'jjj board' for a simple text-based view.");
    Ok(())
}

/// Launch the interactive review TUI
pub fn launch_review(_change_id: &str) -> Result<()> {
    println!("Interactive review TUI not yet implemented.");
    println!("Use 'jjj review start <change-id>' for a simple text-based view.");
    Ok(())
}

// Future implementation will include:
// - Interactive Kanban board with drag-and-drop
// - Keyboard navigation
// - Inline diff viewer
// - Comment threads
// - Real-time collaboration indicators
