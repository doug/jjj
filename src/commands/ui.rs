use crate::error::Result;
use crate::tui;

pub fn execute() -> Result<()> {
    tui::launch()
}
