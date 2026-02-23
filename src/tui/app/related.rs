use super::App;
use std::time::Duration;

impl App {
    /// Schedule a debounced load of related items for the currently selected entity
    pub fn load_related_for_selected(&mut self) {
        self.ui.related_selected = 0;
        // Drop any in-flight request for a previous selection
        self.ui.related_rx = None;

        // Get current selected entity info
        let (entity_type, entity_id) = match self.get_selected_entity_info() {
            Some(info) => info,
            None => {
                self.ui.related_items.clear();
                self.ui.related_pending_load = None;
                return;
            }
        };

        // Check cache first
        let cache_key = (entity_type.clone(), entity_id.clone());
        if let Some(cached) = self.ui.related_cache.get(&cache_key) {
            self.ui.related_items = cached.clone();
            self.ui.related_pending_load = None;
            return;
        }

        // Clear stale results and schedule debounced background load
        self.ui.related_items.clear();
        self.ui.related_pending_load =
            Some((entity_type, entity_id, std::time::Instant::now()));
    }

    /// Poll for completed background related-items load, or spawn one when debounce expires.
    pub(super) fn check_pending_related_load(&mut self) {
        // Collect results from an in-flight background load
        let received = if let Some((ref et, ref eid, ref rx)) = self.ui.related_rx {
            match rx.try_recv() {
                Ok(results) => Some((et.clone(), eid.clone(), results)),
                Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                    // Thread finished without sending (db unavailable etc.) — clear receiver
                    Some((et.clone(), eid.clone(), Vec::new()))
                }
                Err(std::sync::mpsc::TryRecvError::Empty) => None, // still loading
            }
        } else {
            None
        };

        if let Some((et, eid, results)) = received {
            self.ui.related_rx = None;
            // Only apply if the selection hasn't changed since the request was made
            if self.get_selected_entity_info() == Some((et.clone(), eid.clone())) {
                let items: Vec<_> = results.into_iter().filter(|r| r.similarity > 0.5).collect();
                self.ui.related_cache.insert((et, eid), items.clone());
                self.ui.related_items = items;
            }
            return;
        }

        // Check if debounce has expired to start a new background load
        let (entity_type, entity_id) = match &self.ui.related_pending_load {
            Some((et, eid, requested_at))
                if requested_at.elapsed() >= Duration::from_millis(300) =>
            {
                (et.clone(), eid.clone())
            }
            _ => return,
        };

        self.ui.related_pending_load = None;

        // Verify selection hasn't changed
        if self.get_selected_entity_info() != Some((entity_type.clone(), entity_id.clone())) {
            return;
        }

        // Spawn background thread to run the similarity query
        if let Some(ref db_path) = self.db_path {
            let db_path = db_path.clone();
            let et = entity_type.clone();
            let eid = entity_id.clone();
            let (tx, rx) = std::sync::mpsc::channel();

            std::thread::spawn(move || {
                use crate::db::{search::find_similar, Database};
                if let Ok(db) = Database::open(&db_path) {
                    if let Ok(results) = find_similar(db.conn(), &et, &eid, None, 5) {
                        let _ = tx.send(results);
                    }
                }
            });

            self.ui.related_rx = Some((entity_type, entity_id, rx));
        }
    }
}
