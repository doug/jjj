use super::{MetadataStore, EVENTS_FILE};
use crate::error::Result;
use crate::models::Event;

impl MetadataStore {
    /// Set an event to be logged during the next commit
    pub fn set_pending_event(&self, event: Event) {
        *self.pending_event.borrow_mut() = Some(event);
    }

    /// Append an event to the event log
    pub fn append_event(&self, event: &Event) -> Result<()> {
        self.ensure_meta_checkout()?;

        let events_path = self.meta_path.join(EVENTS_FILE);

        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&events_path)?;

        use std::io::Write;
        writeln!(file, "{}", event.to_json_line()?)?;

        Ok(())
    }

    /// Load all events from the event log
    pub fn list_events(&self) -> Result<Vec<Event>> {
        self.ensure_meta_checkout()?;

        let events_path = self.meta_path.join(EVENTS_FILE);

        if !events_path.exists() {
            return Ok(Vec::new());
        }

        let content = std::fs::read_to_string(&events_path)?;
        let mut events = Vec::new();

        for line in content.lines() {
            if line.trim().is_empty() {
                continue;
            }
            let event: Event = serde_json::from_str(line)?;
            events.push(event);
        }

        Ok(events)
    }

    /// Overwrite the entire event log with the given events
    pub fn write_all_events(&self, events: &[Event]) -> Result<()> {
        self.ensure_meta_checkout()?;

        let events_path = self.meta_path.join(EVENTS_FILE);
        let mut content = String::new();

        for event in events {
            use std::fmt::Write;
            writeln!(content, "{}", event.to_json_line()?).unwrap();
        }

        std::fs::write(events_path, content)?;
        Ok(())
    }

    /// Get the current user name from jj config
    pub fn get_current_user(&self) -> Result<String> {
        self.jj_client.user_name()
    }

}
