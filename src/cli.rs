use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "jjj")]
#[command(author, version, about = "Distributed project management and code review for Jujutsu", long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Initialize jjj in the current repository
    Init,

    /// Display the Kanban board
    Board,

    /// Manage tasks
    Task {
        #[command(subcommand)]
        action: TaskAction,
    },

    /// Manage code reviews
    Review {
        #[command(subcommand)]
        action: ReviewAction,
    },

    /// Show dashboard with pending reviews and tasks
    Dashboard,

    /// Resolve conflicts in tasks or reviews
    Resolve {
        /// Task or Change ID to resolve
        id: String,

        /// Pick a specific version (e.g., "Done", "Blocked")
        #[arg(long)]
        pick: Option<String>,
    },
}

#[derive(Subcommand)]
pub enum TaskAction {
    /// Create a new task
    New {
        /// Task title
        title: String,

        /// Feature this task belongs to (e.g., F-1)
        #[arg(long)]
        feature: String,

        /// Tags to apply (e.g., "backend", "frontend")
        #[arg(long)]
        tag: Vec<String>,

        /// Initial column (default: "TODO")
        #[arg(long)]
        column: Option<String>,
    },

    /// List all tasks
    List {
        /// Filter by column
        #[arg(long)]
        column: Option<String>,

        /// Filter by tag
        #[arg(long)]
        tag: Option<String>,
    },

    /// Show task details
    Show {
        /// Task ID (e.g., T-101)
        task_id: String,
    },

    /// Attach the current change to a task
    Attach {
        /// Task ID (e.g., T-101)
        task_id: String,
    },

    /// Detach a change from a task
    Detach {
        /// Task ID (e.g., T-101)
        task_id: String,

        /// Change ID (if not specified, uses current change)
        change_id: Option<String>,
    },

    /// Move a task to a different column
    Move {
        /// Task ID (e.g., T-101)
        task_id: String,

        /// Target column name
        column: String,
    },

    /// Edit task details
    Edit {
        /// Task ID (e.g., T-101)
        task_id: String,

        /// New title
        #[arg(long)]
        title: Option<String>,

        /// Add tags
        #[arg(long)]
        add_tag: Vec<String>,

        /// Remove tags
        #[arg(long)]
        remove_tag: Vec<String>,
    },

    /// Delete a task
    Delete {
        /// Task ID (e.g., T-101)
        task_id: String,
    },
}

#[derive(Subcommand)]
pub enum ReviewAction {
    /// Request a review for the current change
    Request {
        /// Reviewers (e.g., @alice, @bob)
        reviewers: Vec<String>,

        /// Include the entire stack
        #[arg(long)]
        stack: bool,
    },

    /// List pending reviews
    List {
        /// Show only reviews you requested
        #[arg(long)]
        mine: bool,

        /// Show only reviews requesting your input
        #[arg(long)]
        pending: bool,
    },

    /// Start reviewing a change
    Start {
        /// Change ID to review
        change_id: String,
    },

    /// Add a comment to a change
    Comment {
        /// Change ID
        change_id: String,

        /// File path
        #[arg(long)]
        file: Option<String>,

        /// Line number
        #[arg(long)]
        line: Option<usize>,

        /// Comment body
        #[arg(long)]
        body: String,
    },

    /// Show review status
    Status {
        /// Change ID (if not specified, uses current change)
        change_id: Option<String>,
    },

    /// Approve a change
    Approve {
        /// Change ID (if not specified, uses current change)
        change_id: Option<String>,
    },

    /// Request changes on a review
    RequestChanges {
        /// Change ID (if not specified, uses current change)
        change_id: Option<String>,

        /// Summary message
        #[arg(long)]
        message: String,
    },
}
