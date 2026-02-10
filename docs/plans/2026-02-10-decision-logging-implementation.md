# Decision Logging Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add an append-only event log to capture decision history, with timeline visualization and automatic event logging from status-changing commands.

**Architecture:** Events stored in `.jjj/events.jsonl` as one JSON object per line. Commit messages include `jjj: {...}` suffix for rebuild capability. Existing commands (`solution accept`, `critique address`, etc.) automatically append events. New `jjj events` and `jjj timeline` commands for querying.

**Tech Stack:** Rust, serde_json, chrono, clap

---

### Task 1: Event Model

**Files:**
- Create: `src/models/event.rs`
- Modify: `src/models.rs` (add re-export)

**Step 1: Write the event model**

Create `src/models/event.rs`:

```rust
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Event types for decision logging
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EventType {
    // Problem events
    ProblemCreated,
    ProblemSolved,
    ProblemDissolved,
    ProblemReopened,

    // Solution events
    SolutionCreated,
    SolutionAccepted,
    SolutionRefuted,

    // Critique events
    CritiqueRaised,
    CritiqueAddressed,
    CritiqueDismissed,
    CritiqueValidated,

    // Milestone events
    MilestoneCreated,
    MilestoneCompleted,
}

impl std::fmt::Display for EventType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EventType::ProblemCreated => write!(f, "problem_created"),
            EventType::ProblemSolved => write!(f, "problem_solved"),
            EventType::ProblemDissolved => write!(f, "problem_dissolved"),
            EventType::ProblemReopened => write!(f, "problem_reopened"),
            EventType::SolutionCreated => write!(f, "solution_created"),
            EventType::SolutionAccepted => write!(f, "solution_accepted"),
            EventType::SolutionRefuted => write!(f, "solution_refuted"),
            EventType::CritiqueRaised => write!(f, "critique_raised"),
            EventType::CritiqueAddressed => write!(f, "critique_addressed"),
            EventType::CritiqueDismissed => write!(f, "critique_dismissed"),
            EventType::CritiqueValidated => write!(f, "critique_validated"),
            EventType::MilestoneCreated => write!(f, "milestone_created"),
            EventType::MilestoneCompleted => write!(f, "milestone_completed"),
        }
    }
}

/// A single event in the decision log
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    /// When the event occurred
    pub when: DateTime<Utc>,

    /// Type of event
    #[serde(rename = "type")]
    pub event_type: EventType,

    /// Primary entity ID (p1, s1, c1, m1)
    pub entity: String,

    /// Who triggered the event
    pub by: String,

    /// Human explanation of why (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rationale: Option<String>,

    /// Related entity IDs for linking
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub refs: Vec<String>,

    /// Additional context depending on event type
    #[serde(flatten)]
    pub extra: EventExtra,
}

/// Type-specific extra fields
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EventExtra {
    /// For critique_raised: target solution
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target: Option<String>,

    /// For critique_raised: severity
    #[serde(skip_serializing_if = "Option::is_none")]
    pub severity: Option<String>,

    /// For critique_raised: title
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,

    /// For solution_created: problem ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub problem: Option<String>,

    /// For solution_created: supersedes ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub supersedes: Option<String>,
}

impl Event {
    /// Create a new event with current timestamp
    pub fn new(event_type: EventType, entity: String, by: String) -> Self {
        Self {
            when: Utc::now(),
            event_type,
            entity,
            by,
            rationale: None,
            refs: Vec::new(),
            extra: EventExtra::default(),
        }
    }

    /// Add rationale
    pub fn with_rationale(mut self, rationale: impl Into<String>) -> Self {
        self.rationale = Some(rationale.into());
        self
    }

    /// Add refs
    pub fn with_refs(mut self, refs: Vec<String>) -> Self {
        self.refs = refs;
        self
    }

    /// Add extra fields
    pub fn with_extra(mut self, extra: EventExtra) -> Self {
        self.extra = extra;
        self
    }

    /// Serialize to JSON line (no trailing newline)
    pub fn to_json_line(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }

    /// Format for commit message suffix
    pub fn to_commit_suffix(&self) -> Result<String, serde_json::Error> {
        Ok(format!("jjj: {}", self.to_json_line()?))
    }
}
```

**Step 2: Add re-export to models.rs**

Modify `src/models.rs` to add:

```rust
mod event;
pub use event::{Event, EventExtra, EventType};
```

**Step 3: Verify it compiles**

Run: `cargo build`
Expected: Compiles successfully

**Step 4: Commit**

```bash
git add src/models/event.rs src/models.rs
git commit -m "feat: add Event model for decision logging"
```

---

### Task 2: Event Storage in MetadataStore

**Files:**
- Modify: `src/storage.rs`

**Step 1: Add events file constant**

Add near the top of `src/storage.rs` after other constants:

```rust
const EVENTS_FILE: &str = "events.jsonl";
```

**Step 2: Add append_event method**

Add this method to `impl MetadataStore`:

```rust
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
```

**Step 3: Add list_events method**

Add this method to `impl MetadataStore`:

```rust
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
```

**Step 4: Add get_current_user helper**

Add this helper method:

```rust
/// Get the current user name from jj config
pub fn get_current_user(&self) -> Result<String> {
    self.jj_client.get_user_name()
}
```

**Step 5: Verify it compiles**

Run: `cargo build`
Expected: Compiles successfully

**Step 6: Commit**

```bash
git add src/storage.rs
git commit -m "feat: add event storage methods to MetadataStore"
```

---

### Task 3: Modify with_metadata for Structured Commits

**Files:**
- Modify: `src/storage.rs`

**Step 1: Add pending_event field to MetadataStore**

Modify the MetadataStore struct to track a pending event:

```rust
use std::cell::RefCell;

pub struct MetadataStore {
    meta_path: PathBuf,
    pub jj_client: JjClient,
    pub meta_client: JjClient,
    /// Event to append during commit
    pending_event: RefCell<Option<Event>>,
}
```

**Step 2: Update MetadataStore::new**

Initialize the pending_event field:

```rust
Ok(Self {
    meta_path,
    jj_client,
    meta_client,
    pending_event: RefCell::new(None),
})
```

**Step 3: Add set_pending_event method**

```rust
/// Set an event to be logged during the next commit
pub fn set_pending_event(&self, event: Event) {
    *self.pending_event.borrow_mut() = Some(event);
}
```

**Step 4: Modify commit_changes to include event**

Find the `commit_changes` method and modify it to:
1. Append pending event to events.jsonl
2. Include jjj: suffix in commit message

```rust
fn commit_changes(&self, message: &str) -> Result<()> {
    // Handle pending event
    let event_suffix = if let Some(event) = self.pending_event.borrow_mut().take() {
        self.append_event(&event)?;
        format!("\n\n{}", event.to_commit_suffix()?)
    } else {
        String::new()
    };

    let full_message = format!("{}{}", message, event_suffix);

    self.meta_client.execute(&["commit", "-m", &full_message])?;
    self.jj_client.set_bookmark(META_BOOKMARK, "@-")?;
    Ok(())
}
```

**Step 5: Verify it compiles**

Run: `cargo build`
Expected: Compiles successfully

**Step 6: Commit**

```bash
git add src/storage.rs
git commit -m "feat: add pending_event to include in commit messages"
```

---

### Task 4: Add --rationale Flag to Accept/Refute Commands

**Files:**
- Modify: `src/cli.rs`

**Step 1: Add rationale flag to Accept action**

Find `SolutionAction::Accept` and add the rationale flag:

```rust
/// Accept solution (requires no open critiques)
Accept {
    /// Solution ID (e.g., s1)
    solution_id: String,

    /// Force accept even with open critiques
    #[arg(long)]
    force: bool,

    /// Reason for accepting
    #[arg(long)]
    rationale: Option<String>,

    /// Skip rationale prompt
    #[arg(long)]
    no_rationale: bool,
},
```

**Step 2: Add rationale flag to Refute action**

Find `SolutionAction::Refute` and add:

```rust
/// Refute solution (criticism showed it won't work)
Refute {
    /// Solution ID (e.g., s1)
    solution_id: String,

    /// Reason for refuting
    #[arg(long)]
    rationale: Option<String>,

    /// Skip rationale prompt
    #[arg(long)]
    no_rationale: bool,
},
```

**Step 3: Verify it compiles**

Run: `cargo build`
Expected: Compiles successfully

**Step 4: Commit**

```bash
git add src/cli.rs
git commit -m "feat: add --rationale flag to accept/refute commands"
```

---

### Task 5: Update Accept/Refute Commands to Log Events

**Files:**
- Modify: `src/commands/solution.rs`

**Step 1: Update execute function signature**

Update the match arm for Accept:

```rust
SolutionAction::Accept { solution_id, force, rationale, no_rationale } =>
    accept_solution(solution_id, force, rationale, no_rationale),
```

And Refute:

```rust
SolutionAction::Refute { solution_id, rationale, no_rationale } =>
    refute_solution(solution_id, rationale, no_rationale),
```

**Step 2: Update accept_solution function**

```rust
fn accept_solution(solution_id: String, force: bool, rationale: Option<String>, no_rationale: bool) -> Result<()> {
    use crate::models::{Event, EventType};

    let jj_client = JjClient::new()?;
    let store = MetadataStore::new(jj_client)?;

    // ... existing critique checking code stays the same ...

    // Get rationale (prompt if not provided and not skipped)
    let rationale = if let Some(r) = rationale {
        Some(r)
    } else if no_rationale {
        None
    } else {
        print!("Rationale (optional, press Enter to skip): ");
        io::stdout().flush()?;
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let trimmed = input.trim();
        if trimmed.is_empty() { None } else { Some(trimmed.to_string()) }
    };

    // Build refs from addressed critiques
    let refs: Vec<String> = open_critiques.iter().map(|c| c.id.clone()).collect();

    // Create event
    let user = store.get_current_user()?;
    let mut event = Event::new(EventType::SolutionAccepted, solution_id.clone(), user);
    if let Some(r) = &rationale {
        event = event.with_rationale(r);
    }
    if !refs.is_empty() {
        event = event.with_refs(refs);
    }

    store.with_metadata(&format!("Accept solution {}", solution_id), || {
        // Set the event to be logged
        store.set_pending_event(event.clone());

        let mut solution = store.load_solution(&solution_id)?;
        if force {
            solution.force_accepted = true;
        }
        solution.accept();
        store.save_solution(&solution)?;

        // ... rest of existing code ...
        Ok(())
    })
}
```

**Step 3: Update refute_solution function**

```rust
fn refute_solution(solution_id: String, rationale: Option<String>, no_rationale: bool) -> Result<()> {
    use crate::models::{Event, EventType};

    let jj_client = JjClient::new()?;
    let store = MetadataStore::new(jj_client)?;

    // Get rationale
    let rationale = if let Some(r) = rationale {
        Some(r)
    } else if no_rationale {
        None
    } else {
        print!("Rationale (optional, press Enter to skip): ");
        io::stdout().flush()?;
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let trimmed = input.trim();
        if trimmed.is_empty() { None } else { Some(trimmed.to_string()) }
    };

    let user = store.get_current_user()?;
    let mut event = Event::new(EventType::SolutionRefuted, solution_id.clone(), user);
    if let Some(r) = &rationale {
        event = event.with_rationale(r);
    }

    store.with_metadata(&format!("Refute solution {}", solution_id), || {
        store.set_pending_event(event.clone());

        let mut solution = store.load_solution(&solution_id)?;
        solution.refute();
        store.save_solution(&solution)?;
        println!("Solution {} refuted", solution_id);
        Ok(())
    })
}
```

**Step 4: Verify it compiles**

Run: `cargo build`
Expected: Compiles successfully

**Step 5: Commit**

```bash
git add src/commands/solution.rs
git commit -m "feat: log events on solution accept/refute"
```

---

### Task 6: Add Events Command

**Files:**
- Modify: `src/cli.rs`
- Create: `src/commands/events.rs`
- Modify: `src/commands/mod.rs`

**Step 1: Add Events command to CLI**

Add to `Commands` enum in `src/cli.rs`:

```rust
/// Query the event log
Events {
    #[command(subcommand)]
    action: Option<EventsAction>,

    /// Filter by start date (YYYY-MM-DD or YYYY-MM)
    #[arg(long)]
    from: Option<String>,

    /// Filter by end date
    #[arg(long)]
    to: Option<String>,

    /// Filter by problem
    #[arg(long)]
    problem: Option<String>,

    /// Filter by solution
    #[arg(long)]
    solution: Option<String>,

    /// Filter by event type
    #[arg(long, name = "type")]
    event_type: Option<String>,

    /// Full-text search in rationales
    #[arg(long)]
    search: Option<String>,

    /// Output as JSON
    #[arg(long)]
    json: bool,

    /// Number of events to show (default: 20)
    #[arg(long, default_value = "20")]
    limit: usize,
},
```

Add the subcommand enum:

```rust
#[derive(Subcommand)]
pub enum EventsAction {
    /// Rebuild events.jsonl from commit history
    Rebuild,

    /// Validate event log against entity states
    Validate,
}
```

**Step 2: Create events command handler**

Create `src/commands/events.rs`:

```rust
use crate::cli::EventsAction;
use crate::error::Result;
use crate::jj::JjClient;
use crate::models::Event;
use crate::storage::MetadataStore;
use chrono::{NaiveDate, TimeZone, Utc};

pub fn execute(
    action: Option<EventsAction>,
    from: Option<String>,
    to: Option<String>,
    problem: Option<String>,
    solution: Option<String>,
    event_type: Option<String>,
    search: Option<String>,
    json: bool,
    limit: usize,
) -> Result<()> {
    match action {
        Some(EventsAction::Rebuild) => rebuild_events(),
        Some(EventsAction::Validate) => validate_events(),
        None => list_events(from, to, problem, solution, event_type, search, json, limit),
    }
}

fn list_events(
    from: Option<String>,
    to: Option<String>,
    problem: Option<String>,
    solution: Option<String>,
    event_type: Option<String>,
    search: Option<String>,
    json: bool,
    limit: usize,
) -> Result<()> {
    let jj_client = JjClient::new()?;
    let store = MetadataStore::new(jj_client)?;

    let mut events = store.list_events()?;

    // Parse date filters
    let from_date = from.as_ref().and_then(|s| parse_date_filter(s));
    let to_date = to.as_ref().and_then(|s| parse_date_filter(s));

    // Apply filters
    events.retain(|e| {
        // Date filters
        if let Some(ref fd) = from_date {
            if e.when < *fd {
                return false;
            }
        }
        if let Some(ref td) = to_date {
            if e.when > *td {
                return false;
            }
        }

        // Entity filters
        if let Some(ref p) = problem {
            if !e.entity.starts_with('p') || e.entity != *p {
                if !e.refs.contains(p) {
                    return false;
                }
            }
        }
        if let Some(ref s) = solution {
            if !e.entity.starts_with('s') || e.entity != *s {
                if !e.refs.contains(s) {
                    return false;
                }
            }
        }

        // Type filter
        if let Some(ref t) = event_type {
            if !e.event_type.to_string().contains(t) {
                return false;
            }
        }

        // Search filter
        if let Some(ref q) = search {
            let q_lower = q.to_lowercase();
            let matches = e.rationale.as_ref()
                .map(|r| r.to_lowercase().contains(&q_lower))
                .unwrap_or(false);
            if !matches {
                return false;
            }
        }

        true
    });

    // Reverse to show most recent first, then limit
    events.reverse();
    events.truncate(limit);

    if json {
        println!("{}", serde_json::to_string_pretty(&events)?);
        return Ok(());
    }

    if events.is_empty() {
        println!("No events found");
        return Ok(());
    }

    for event in &events {
        let date = event.when.format("%Y-%m-%d %H:%M");
        let rationale = event.rationale.as_ref()
            .map(|r| format!(" - {}", truncate(r, 50)))
            .unwrap_or_default();

        println!("{} {:20} {:8} {}{}",
            date,
            event.event_type.to_string(),
            event.entity,
            event.by,
            rationale
        );
    }

    Ok(())
}

fn parse_date_filter(s: &str) -> Option<chrono::DateTime<Utc>> {
    // Try YYYY-MM-DD
    if let Ok(date) = NaiveDate::parse_from_str(s, "%Y-%m-%d") {
        return Some(Utc.from_utc_datetime(&date.and_hms_opt(0, 0, 0).unwrap()));
    }
    // Try YYYY-MM (first of month)
    if let Ok(date) = NaiveDate::parse_from_str(&format!("{}-01", s), "%Y-%m-%d") {
        return Some(Utc.from_utc_datetime(&date.and_hms_opt(0, 0, 0).unwrap()));
    }
    None
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}...", &s[..max-3])
    }
}

fn rebuild_events() -> Result<()> {
    println!("Rebuild not yet implemented");
    // TODO: Parse commit history for jjj: lines
    Ok(())
}

fn validate_events() -> Result<()> {
    println!("Validate not yet implemented");
    // TODO: Cross-check events with entity states
    Ok(())
}
```

**Step 3: Add to commands/mod.rs**

Add the module and dispatch:

```rust
pub mod events;

// In execute function:
Commands::Events { action, from, to, problem, solution, event_type, search, json, limit } =>
    events::execute(action, from, to, problem, solution, event_type, search, json, limit),
```

**Step 4: Verify it compiles**

Run: `cargo build`
Expected: Compiles successfully

**Step 5: Commit**

```bash
git add src/cli.rs src/commands/events.rs src/commands/mod.rs
git commit -m "feat: add jjj events command for querying event log"
```

---

### Task 7: Add Timeline Command

**Files:**
- Modify: `src/cli.rs`
- Create: `src/commands/timeline.rs`
- Modify: `src/commands/mod.rs`

**Step 1: Add Timeline command to CLI**

Add to `Commands` enum:

```rust
/// Show timeline for a problem
Timeline {
    /// Problem ID to show timeline for
    problem_id: String,

    /// Output as JSON
    #[arg(long)]
    json: bool,
},
```

**Step 2: Create timeline command handler**

Create `src/commands/timeline.rs`:

```rust
use crate::error::Result;
use crate::jj::JjClient;
use crate::models::{Event, EventType};
use crate::storage::MetadataStore;

pub fn execute(problem_id: String, json: bool) -> Result<()> {
    let jj_client = JjClient::new()?;
    let store = MetadataStore::new(jj_client)?;

    // Load problem for title
    let problem = store.load_problem(&problem_id)?;

    // Get all events related to this problem
    let all_events = store.list_events()?;

    // Find related entity IDs (solutions and critiques for this problem)
    let solutions = store.get_solutions_for_problem(&problem_id)?;
    let solution_ids: Vec<String> = solutions.iter().map(|s| s.id.clone()).collect();

    let mut critique_ids: Vec<String> = Vec::new();
    for s in &solutions {
        let critiques = store.get_critiques_for_solution(&s.id)?;
        for c in critiques {
            critique_ids.push(c.id.clone());
        }
    }

    // Filter events related to this problem
    let mut events: Vec<&Event> = all_events.iter().filter(|e| {
        // Direct match
        if e.entity == problem_id {
            return true;
        }
        // Solution for this problem
        if solution_ids.contains(&e.entity) {
            return true;
        }
        // Critique on a solution for this problem
        if critique_ids.contains(&e.entity) {
            return true;
        }
        // Referenced in refs
        if e.refs.contains(&problem_id) {
            return true;
        }
        false
    }).collect();

    // Sort by timestamp
    events.sort_by_key(|e| e.when);

    if json {
        println!("{}", serde_json::to_string_pretty(&events)?);
        return Ok(());
    }

    // Print header
    println!("{}: {}", problem_id, problem.title);
    println!("{}", "━".repeat(50));
    println!();

    for event in &events {
        let date = event.when.format("%Y-%m-%d");
        let desc = format_event_description(event);
        let by = &event.by;

        println!("{:<12} {:<40} {}", date, desc, by);

        if let Some(ref rationale) = event.rationale {
            // Indent rationale
            for line in rationale.lines() {
                println!("             \"{}\"", line);
            }
        }
    }

    Ok(())
}

fn format_event_description(event: &Event) -> String {
    match event.event_type {
        EventType::ProblemCreated => "problem created".to_string(),
        EventType::ProblemSolved => "problem solved".to_string(),
        EventType::ProblemDissolved => "problem dissolved".to_string(),
        EventType::ProblemReopened => "problem reopened".to_string(),
        EventType::SolutionCreated => {
            let supersedes = event.extra.supersedes.as_ref()
                .map(|s| format!(" (supersedes {})", s))
                .unwrap_or_default();
            format!("{} proposed{}", event.entity, supersedes)
        }
        EventType::SolutionAccepted => format!("{} accepted", event.entity),
        EventType::SolutionRefuted => format!("{} refuted", event.entity),
        EventType::CritiqueRaised => {
            let title = event.extra.title.as_ref()
                .map(|t| format!(": \"{}\"", truncate(t, 25)))
                .unwrap_or_default();
            format!("{} raised{}", event.entity, title)
        }
        EventType::CritiqueAddressed => format!("{} addressed", event.entity),
        EventType::CritiqueDismissed => format!("{} dismissed", event.entity),
        EventType::CritiqueValidated => format!("{} validated", event.entity),
        EventType::MilestoneCreated => format!("{} created", event.entity),
        EventType::MilestoneCompleted => format!("{} completed", event.entity),
    }
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}...", &s[..max-3])
    }
}
```

**Step 3: Add to commands/mod.rs**

```rust
pub mod timeline;

// In execute function:
Commands::Timeline { problem_id, json } => timeline::execute(problem_id, json),
```

**Step 4: Verify it compiles**

Run: `cargo build`
Expected: Compiles successfully

**Step 5: Commit**

```bash
git add src/cli.rs src/commands/timeline.rs src/commands/mod.rs
git commit -m "feat: add jjj timeline command for problem history"
```

---

### Task 8: Add Event Logging to Other Commands

**Files:**
- Modify: `src/commands/problem.rs`
- Modify: `src/commands/critique.rs`
- Modify: `src/commands/solution.rs` (for solution created)

**Step 1: Add event logging to problem commands**

In `src/commands/problem.rs`, add events to:
- `create_problem` → `ProblemCreated`
- `solve_problem` → `ProblemSolved`
- `dissolve_problem` → `ProblemDissolved`

**Step 2: Add event logging to critique commands**

In `src/commands/critique.rs`, add events to:
- `create_critique` → `CritiqueRaised` (with target, severity, title in extra)
- `address_critique` → `CritiqueAddressed`
- `dismiss_critique` → `CritiqueDismissed`
- `validate_critique` → `CritiqueValidated`

**Step 3: Add event logging to solution new**

In `src/commands/solution.rs`, add event to:
- `new_solution` → `SolutionCreated` (with problem, supersedes in extra)

**Step 4: Verify it compiles**

Run: `cargo build`
Expected: Compiles successfully

**Step 5: Commit**

```bash
git add src/commands/problem.rs src/commands/critique.rs src/commands/solution.rs
git commit -m "feat: add event logging to all status-changing commands"
```

---

### Task 9: Integration Test

**Files:**
- Modify: `tests/workflow_test.rs`

**Step 1: Add event logging test**

Add a new test:

```rust
#[test]
fn test_events_logged_on_status_changes() {
    if which::which("jj").is_err() { return; }
    let temp_dir = setup_test_repo();
    let dir = temp_dir.path();

    // Create and accept a solution
    run_jjj(dir, &["solution", "new", "Test Solution", "--problem", "p1"]);
    run_jjj(dir, &["solution", "accept", "s1", "--rationale", "Tests pass", "--no-rationale"]);

    // Check events
    let output = run_jjj(dir, &["events", "--json"]);
    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("solution_created"), "Missing solution_created event");
    assert!(stdout.contains("solution_accepted"), "Missing solution_accepted event");
    assert!(stdout.contains("Tests pass"), "Missing rationale in event");
}
```

**Step 2: Run test**

Run: `cargo test test_events_logged`
Expected: Test passes

**Step 3: Commit**

```bash
git add tests/workflow_test.rs
git commit -m "test: add integration test for event logging"
```

---

### Task 10: Update Documentation

**Files:**
- Modify: `docs/reference/cli-workflow.md`
- Modify: `CLAUDE.md`

**Step 1: Add events command documentation**

Add section to `docs/reference/cli-workflow.md`:

```markdown
## `jjj events`

Query the decision event log.

```
jjj events [OPTIONS]
```

| Flag | Type | Description |
|------|------|-------------|
| `--from` | string | Filter by start date (YYYY-MM-DD or YYYY-MM) |
| `--to` | string | Filter by end date |
| `--problem` | string | Filter by problem ID |
| `--solution` | string | Filter by solution ID |
| `--type` | string | Filter by event type |
| `--search` | string | Full-text search in rationales |
| `--json` | bool | Output as JSON |
| `--limit` | integer | Number of events (default: 20) |

```bash
jjj events
jjj events --from 2024-01 --to 2024-06
jjj events --problem p1
jjj events --type solution_accepted
jjj events --search "cache"
```

### `jjj events rebuild`

Rebuild events.jsonl from commit history.

### `jjj events validate`

Validate event log against entity states.

## `jjj timeline`

Show timeline for a problem and all related entities.

```
jjj timeline <problem_id> [OPTIONS]
```

| Flag | Type | Description |
|------|------|-------------|
| `--json` | bool | Output as JSON |

```bash
jjj timeline p1
```
```

**Step 2: Update CLAUDE.md**

Add to the commands section:

```markdown
### Events and Timeline
```bash
jjj events                     # Recent events
jjj events --problem p1        # Events for a problem
jjj timeline p1                # Full timeline visualization
```
```

**Step 3: Commit**

```bash
git add docs/reference/cli-workflow.md CLAUDE.md
git commit -m "docs: add events and timeline command documentation"
```

---

## Summary

After completing all tasks, you will have:

1. **Event model** - `src/models/event.rs` with EventType enum and Event struct
2. **Event storage** - Methods in MetadataStore for append/list events
3. **Automatic logging** - All status-changing commands log events
4. **Structured commits** - Commit messages include `jjj: {...}` suffix
5. **Query commands** - `jjj events` with filters and `jjj timeline` for visualization
6. **Documentation** - Updated CLI reference and CLAUDE.md

The event log enables teams to understand decision history over 2+ years of complex work.
