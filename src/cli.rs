use clap::{Parser, Subcommand, ValueEnum};

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

    /// Display the board (solutions by status)
    Board {
        /// Output in JSON format
        #[arg(long)]
        json: bool,
    },

    /// Manage problems (what needs to be solved)
    Problem {
        #[command(subcommand)]
        action: ProblemAction,
    },

    /// Manage solutions (conjectures to solve problems)
    Solution {
        #[command(subcommand)]
        action: SolutionAction,
    },

    /// Manage critiques (criticism of solutions)
    Critique {
        #[command(subcommand)]
        action: CritiqueAction,
    },

    /// Manage code reviews
    Review {
        #[command(subcommand)]
        action: ReviewAction,
    },

    /// Show dashboard with pending work
    Dashboard {
        /// Output in JSON format
        #[arg(long)]
        json: bool,
    },

    /// Resolve conflicts
    Resolve {
        /// ID to resolve
        id: String,

        /// Pick a specific version
        #[arg(long)]
        pick: Option<String>,
    },

    /// Manage milestones
    Milestone {
        #[command(subcommand)]
        action: MilestoneAction,
    },

    /// Manage tags
    Tag {
        #[command(subcommand)]
        action: TagAction,
    },

    /// Start working on a solution
    Start {
        /// Solution ID (to resume) or Title (to create new solution)
        arg: String,

        /// Problem this solution addresses (required for new solutions)
        #[arg(long)]
        problem: Option<String>,
    },

    /// Submit current changes (squash and complete solution)
    Submit {
        /// Force submit (bypass review check)
        #[arg(long)]
        force: bool,
    },

    /// Generate shell completions
    Completion {
        /// Shell to generate completions for
        shell: Shell,
    },
}

#[derive(ValueEnum, Clone, Debug)]
pub enum Shell {
    Bash,
    Zsh,
    Fish,
    PowerShell,
    Elvish,
}

// =============================================================================
// Problem Commands
// =============================================================================

#[derive(Subcommand)]
pub enum ProblemAction {
    /// Create a new problem
    New {
        /// Problem title
        title: String,

        /// Parent problem (for sub-problems)
        #[arg(long)]
        parent: Option<String>,

        /// Milestone to target
        #[arg(long)]
        milestone: Option<String>,

        /// Tags to apply
        #[arg(long)]
        tag: Vec<String>,
    },

    /// List all problems
    List {
        /// Filter by status
        #[arg(long)]
        status: Option<String>,

        /// Show as tree (hierarchical view)
        #[arg(long)]
        tree: bool,

        /// Filter by milestone
        #[arg(long)]
        milestone: Option<String>,

        /// Output in JSON format
        #[arg(long)]
        json: bool,
    },

    /// Show problem details
    Show {
        /// Problem ID (e.g., P-1)
        problem_id: String,

        /// Output in JSON format
        #[arg(long)]
        json: bool,
    },

    /// Edit problem details
    Edit {
        /// Problem ID (e.g., P-1)
        problem_id: String,

        /// New title
        #[arg(long)]
        title: Option<String>,

        /// New status (open, in_progress, solved, dissolved)
        #[arg(long)]
        status: Option<String>,

        /// Set parent problem
        #[arg(long)]
        parent: Option<String>,

        /// Add tags
        #[arg(long)]
        add_tag: Vec<String>,

        /// Remove tags
        #[arg(long)]
        remove_tag: Vec<String>,
    },

    /// Show problem hierarchy as tree
    Tree {
        /// Starting problem ID (defaults to all root problems)
        problem_id: Option<String>,
    },

    /// Mark problem as solved (requires accepted solution)
    Solve {
        /// Problem ID (e.g., P-1)
        problem_id: String,
    },

    /// Mark problem as dissolved (based on false premises)
    Dissolve {
        /// Problem ID (e.g., P-1)
        problem_id: String,
    },

    /// Assign a problem to a person
    Assign {
        /// Problem ID (e.g., P-1)
        problem_id: String,

        /// Assignee name (if not specified, assigns to self)
        #[arg(long)]
        to: Option<String>,
    },
}

// =============================================================================
// Solution Commands
// =============================================================================

#[derive(Subcommand)]
pub enum SolutionAction {
    /// Create a new solution (conjecture)
    New {
        /// Solution title
        title: String,

        /// Problem this solution addresses (required)
        #[arg(long)]
        problem: String,

        /// Tags to apply
        #[arg(long)]
        tag: Vec<String>,
    },

    /// List all solutions
    List {
        /// Filter by problem
        #[arg(long)]
        problem: Option<String>,

        /// Filter by status (proposed, testing, refuted, accepted)
        #[arg(long)]
        status: Option<String>,

        /// Output in JSON format
        #[arg(long)]
        json: bool,
    },

    /// Show solution details
    Show {
        /// Solution ID (e.g., S-1)
        solution_id: String,

        /// Output in JSON format
        #[arg(long)]
        json: bool,
    },

    /// Edit solution details
    Edit {
        /// Solution ID (e.g., S-1)
        solution_id: String,

        /// New title
        #[arg(long)]
        title: Option<String>,

        /// New status
        #[arg(long)]
        status: Option<String>,

        /// Add tags
        #[arg(long)]
        add_tag: Vec<String>,

        /// Remove tags
        #[arg(long)]
        remove_tag: Vec<String>,
    },

    /// Attach current jj change to solution
    Attach {
        /// Solution ID (e.g., S-1)
        solution_id: String,
    },

    /// Detach a change from solution
    Detach {
        /// Solution ID (e.g., S-1)
        solution_id: String,

        /// Change ID (if not specified, uses current change)
        change_id: Option<String>,
    },

    /// Move solution to testing status
    Test {
        /// Solution ID (e.g., S-1)
        solution_id: String,
    },

    /// Accept solution (requires no valid critiques)
    Accept {
        /// Solution ID (e.g., S-1)
        solution_id: String,
    },

    /// Refute solution (criticism showed it won't work)
    Refute {
        /// Solution ID (e.g., S-1)
        solution_id: String,
    },

    /// Assign a solution to a person
    Assign {
        /// Solution ID (e.g., S-1)
        solution_id: String,

        /// Assignee name (if not specified, assigns to self)
        #[arg(long)]
        to: Option<String>,
    },
}

// =============================================================================
// Critique Commands
// =============================================================================

#[derive(Subcommand)]
pub enum CritiqueAction {
    /// Add a critique to a solution
    New {
        /// Solution to critique (e.g., S-1)
        solution_id: String,

        /// Critique title
        title: String,

        /// Severity (low, medium, high, critical)
        #[arg(long, default_value = "medium")]
        severity: String,

        /// File path for code-level critique
        #[arg(long)]
        file: Option<String>,

        /// Line number for code-level critique
        #[arg(long)]
        line: Option<usize>,
    },

    /// List critiques
    List {
        /// Filter by solution
        #[arg(long)]
        solution: Option<String>,

        /// Filter by status (open, addressed, valid, dismissed)
        #[arg(long)]
        status: Option<String>,

        /// Output in JSON format
        #[arg(long)]
        json: bool,
    },

    /// Show critique details
    Show {
        /// Critique ID (e.g., CQ-1)
        critique_id: String,

        /// Output in JSON format
        #[arg(long)]
        json: bool,
    },

    /// Edit critique details
    Edit {
        /// Critique ID (e.g., CQ-1)
        critique_id: String,

        /// New title
        #[arg(long)]
        title: Option<String>,

        /// New severity
        #[arg(long)]
        severity: Option<String>,

        /// New status
        #[arg(long)]
        status: Option<String>,
    },

    /// Mark critique as addressed (solution was modified)
    Address {
        /// Critique ID (e.g., CQ-1)
        critique_id: String,
    },

    /// Validate critique (it's correct, solution should be refuted)
    Validate {
        /// Critique ID (e.g., CQ-1)
        critique_id: String,
    },

    /// Dismiss critique (incorrect or irrelevant)
    Dismiss {
        /// Critique ID (e.g., CQ-1)
        critique_id: String,
    },

    /// Reply to a critique
    Reply {
        /// Critique ID (e.g., CQ-1)
        critique_id: String,

        /// Reply body
        body: String,
    },
}

// =============================================================================
// Milestone Commands
// =============================================================================

#[derive(Subcommand)]
pub enum MilestoneAction {
    /// Create a new milestone
    New {
        /// Milestone title
        title: String,

        /// Target date (YYYY-MM-DD)
        #[arg(long)]
        date: Option<String>,

        /// Tags to apply
        #[arg(long)]
        tag: Vec<String>,
    },

    /// Edit milestone details
    Edit {
        /// Milestone ID (e.g., M-1)
        milestone_id: String,

        /// New title
        #[arg(long)]
        title: Option<String>,

        /// New target date
        #[arg(long)]
        date: Option<String>,

        /// New status (planning, active, completed, cancelled)
        #[arg(long)]
        status: Option<String>,

        /// Add tags
        #[arg(long)]
        add_tag: Vec<String>,

        /// Remove tags
        #[arg(long)]
        remove_tag: Vec<String>,
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

    /// Add a problem to milestone
    AddProblem {
        /// Milestone ID (e.g., M-1)
        milestone_id: String,

        /// Problem ID (e.g., P-1)
        problem_id: String,
    },

    /// Remove a problem from milestone
    RemoveProblem {
        /// Milestone ID (e.g., M-1)
        milestone_id: String,

        /// Problem ID (e.g., P-1)
        problem_id: String,
    },

    /// Show roadmap view (problems and solution progress)
    Roadmap {
        /// Output in JSON format
        #[arg(long)]
        json: bool,
    },

    /// Assign a milestone to a person
    Assign {
        /// Milestone ID (e.g., M-1)
        milestone_id: String,

        /// Assignee name (if not specified, assigns to self)
        #[arg(long)]
        to: Option<String>,
    },
}

// =============================================================================
// Review Commands
// =============================================================================

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

// =============================================================================
// Tag Commands
// =============================================================================

#[derive(Subcommand)]
pub enum TagAction {
    /// List all tags
    List {
        /// Output in JSON format
        #[arg(long)]
        json: bool,
    },

    /// Create a new tag
    New {
        /// Tag name
        name: String,

        /// Description
        #[arg(long)]
        desc: Option<String>,

        /// Color (hex or name)
        #[arg(long)]
        color: Option<String>,
    },

    /// Edit a tag
    Edit {
        /// Tag ID (e.g., tag-1)
        tag_id: String,

        /// New name
        #[arg(long)]
        name: Option<String>,

        /// New description
        #[arg(long)]
        desc: Option<String>,

        /// New color
        #[arg(long)]
        color: Option<String>,
    },

    /// Delete a tag
    Delete {
        /// Tag ID (e.g., tag-1)
        tag_id: String,
    },
}
