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
    Board {
        /// Output in JSON format
        #[arg(long)]
        json: bool,
    },

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
    Dashboard {
        /// Output in JSON format
        #[arg(long)]
        json: bool,
    },

    /// Resolve conflicts in tasks or reviews
    Resolve {
        /// Task or Change ID to resolve
        id: String,

        /// Pick a specific version (e.g., "Done", "Blocked")
        #[arg(long)]
        pick: Option<String>,
    },

    /// Manage milestones
    Milestone {
        #[command(subcommand)]
        action: MilestoneAction,
    },

    /// Manage features
    Feature {
        #[command(subcommand)]
        action: FeatureAction,
    },

    /// Manage bugs
    Bug {
        #[command(subcommand)]
        action: BugAction,
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

        /// Output in JSON format
        #[arg(long)]
        json: bool,
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

        /// Output in JSON format
        #[arg(long)]
        json: bool,
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

#[derive(Subcommand)]
pub enum MilestoneAction {
    /// Create a new milestone
    New {
        /// Milestone title
        title: String,

        /// Target date (YYYY-MM-DD)
        #[arg(long)]
        date: Option<String>,

        /// Description
        #[arg(long)]
        description: Option<String>,
    },

    /// List all milestones
    List {
        /// Output in JSON format
        #[arg(long)]
        json: bool,
    },

    /// Show milestone details
    Show {
        /// Milestone ID (e.g., M-1)
        milestone_id: String,

        /// Output in JSON format
        #[arg(long)]
        json: bool,
    },

    /// Add a feature to a milestone
    AddFeature {
        /// Milestone ID (e.g., M-1)
        milestone_id: String,

        /// Feature ID (e.g., F-1)
        feature_id: String,
    },

    /// Add a bug to a milestone
    AddBug {
        /// Milestone ID (e.g., M-1)
        milestone_id: String,

        /// Bug ID (e.g., B-1)
        bug_id: String,
    },

    /// Show roadmap view
    Roadmap {
        /// Output in JSON format
        #[arg(long)]
        json: bool,
    },
}

#[derive(Subcommand)]
pub enum FeatureAction {
    /// Create a new feature
    New {
        /// Feature title
        title: String,

        /// Milestone this feature belongs to (e.g., M-1)
        #[arg(long)]
        milestone: Option<String>,

        /// Priority (low, medium, high, critical)
        #[arg(long)]
        priority: Option<String>,

        /// Description
        #[arg(long)]
        description: Option<String>,
    },

    /// List all features
    List {
        /// Filter by milestone
        #[arg(long)]
        milestone: Option<String>,

        /// Filter by status
        #[arg(long)]
        status: Option<String>,

        /// Output in JSON format
        #[arg(long)]
        json: bool,
    },

    /// Show feature details
    Show {
        /// Feature ID (e.g., F-1)
        feature_id: String,

        /// Output in JSON format
        #[arg(long)]
        json: bool,
    },

    /// Show feature board view
    Board {
        /// Feature ID (e.g., F-1)
        feature_id: Option<String>,

        /// Output in JSON format
        #[arg(long)]
        json: bool,
    },

    /// Show feature progress
    Progress {
        /// Feature ID (e.g., F-1)
        feature_id: String,
    },

    /// Move feature to different status
    Move {
        /// Feature ID (e.g., F-1)
        feature_id: String,

        /// Target status (backlog, inprogress, review, done, blocked)
        status: String,
    },
}

#[derive(Subcommand)]
pub enum BugAction {
    /// Report a new bug
    New {
        /// Bug title
        title: String,

        /// Severity (low, medium, high, critical)
        #[arg(long)]
        severity: Option<String>,

        /// Description
        #[arg(long)]
        description: Option<String>,

        /// Reproduction steps
        #[arg(long)]
        repro: Option<String>,
    },

    /// List all bugs
    List {
        /// Filter by severity
        #[arg(long)]
        severity: Option<String>,

        /// Filter by status
        #[arg(long)]
        status: Option<String>,

        /// Show only open bugs
        #[arg(long)]
        open: bool,

        /// Output in JSON format
        #[arg(long)]
        json: bool,
    },

    /// Show bug details
    Show {
        /// Bug ID (e.g., B-1)
        bug_id: String,

        /// Output in JSON format
        #[arg(long)]
        json: bool,
    },

    /// Link bug to feature or milestone
    Link {
        /// Bug ID (e.g., B-1)
        bug_id: String,

        /// Feature to link to (e.g., F-1)
        #[arg(long)]
        feature: Option<String>,

        /// Milestone to link to (e.g., M-1)
        #[arg(long)]
        milestone: Option<String>,
    },

    /// Update bug status
    Status {
        /// Bug ID (e.g., B-1)
        bug_id: String,

        /// New status (new, confirmed, inprogress, fixed, closed, wontfix, duplicate)
        status: String,
    },

    /// Show bug triage view
    Triage {
        /// Output in JSON format
        #[arg(long)]
        json: bool,
    },
}
