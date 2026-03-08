use clap::{Parser, Subcommand, ValueEnum};

#[derive(Parser)]
#[command(name = "jjj")]
#[command(author, version)]
#[command(about = "Distributed project management for Jujutsu repositories")]
#[command(long_about = "
jjj organises work as Problems → Solutions → Critiques, stored in an orphaned
'jjj' bookmark alongside your code. No central server, no database — everything
syncs with your normal 'jj git push'.

  Problems   What needs solving (the question)
  Solutions  Conjectures attached to jj change IDs (the approach)
  Critiques  Error-elimination feedback that blocks or endorses a solution
  Milestones Time-based goals grouping related problems")]
#[command(after_long_help = "TYPICAL WORKFLOW:
  jjj init                    Set up a new repository
  jjj problem new 'Bug: ...'  Define what needs solving
  jjj solution new 'Fix: ...' Propose your approach (creates a solution record)
  jjj solution attach <id>    Link your current jj change to the solution
  jjj solution submit <id>    Submit the solution for review
  jjj critique new <id> '...' Raise a critique against a solution
  jjj solution approve <id>   Approve the solution once critiques are resolved

Run 'jjj <command> --help' for detailed options.")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    // ── Quick start ────────────────────────────────────────────────────────
    /// Open the interactive project browser (recommended starting point)
    #[command(display_order = 0)]
    Ui,

    /// Show what needs your attention: open problems, pending reviews, critiques
    #[command(display_order = 1)]
    Status {
        /// Show all items regardless of limit
        #[arg(long)]
        all: bool,

        /// Show only your own authored work
        #[arg(long)]
        mine: bool,

        /// Show top N items (default: 5)
        #[arg(long)]
        limit: Option<usize>,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Print the highest-priority next action(s) (great for shell prompts)
    #[command(display_order = 2)]
    Next {
        /// Show top N items (default: 1; 0 means all)
        #[arg(long)]
        top: Option<usize>,

        /// Show only your own authored work
        #[arg(long)]
        mine: bool,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    // ── Core entities ──────────────────────────────────────────────────────
    /// Track problems to solve (the questions driving your work)
    #[command(display_order = 10)]
    Problem {
        #[command(subcommand)]
        action: ProblemAction,
    },

    /// Propose and approve solutions — conjectures attached to jj change IDs
    #[command(display_order = 11)]
    Solution {
        #[command(subcommand)]
        action: SolutionAction,
    },

    /// Add and resolve critiques that challenge or endorse a solution
    #[command(display_order = 12)]
    Critique {
        #[command(subcommand)]
        action: CritiqueAction,
    },

    /// Group problems into time-based milestones and track progress
    #[command(display_order = 13)]
    Milestone {
        #[command(subcommand)]
        action: MilestoneAction,
    },

    // ── Discover ───────────────────────────────────────────────────────────
    /// Search problems, solutions, and critiques by text or semantic similarity
    #[command(display_order = 30)]
    Search {
        /// Query string, or an entity reference like 'p/01957d' for similarity search
        query: String,

        /// Restrict to one entity type: problem, solution, critique, milestone, event
        #[arg(long, short = 't')]
        r#type: Option<String>,

        /// Use full-text search only (skip semantic/embedding features)
        #[arg(long)]
        text_only: bool,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Show the full history of a problem: solutions, critiques, and decisions
    #[command(display_order = 31)]
    Timeline {
        /// Problem ID or title
        problem_id: String,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Browse the structured event log (state changes, rationales, decisions)
    #[command(display_order = 32)]
    Events {
        #[command(subcommand)]
        action: Option<EventsAction>,

        /// Show events on or after this date (YYYY-MM-DD or YYYY-MM)
        #[arg(long)]
        from: Option<String>,

        /// Show events on or before this date
        #[arg(long)]
        to: Option<String>,

        /// Filter to events touching a specific problem
        #[arg(long)]
        problem: Option<String>,

        /// Filter to events touching a specific solution
        #[arg(long)]
        solution: Option<String>,

        /// Filter by event type (e.g. problem_created, solution_approved)
        #[arg(long, name = "type")]
        event_type: Option<String>,

        /// Full-text search in event rationales
        #[arg(long)]
        search: Option<String>,

        /// Filter events after this RFC3339 timestamp (e.g. 2025-01-01T00:00:00Z)
        #[arg(long)]
        since: Option<String>,

        /// Output as JSON
        #[arg(long)]
        json: bool,

        /// Maximum number of events to show (default: 20)
        #[arg(long, default_value = "20")]
        limit: usize,
    },

    // ── Collaborate ────────────────────────────────────────────────────────
    /// Pull jj changes and jjj metadata from a remote
    #[command(display_order = 40)]
    Fetch {
        /// Remote name (default: origin)
        #[arg(long, default_value = "origin")]
        remote: String,
    },

    /// Push jj changes and jjj metadata to a remote
    #[command(display_order = 41)]
    Push {
        /// Additional bookmarks to push alongside jjj
        bookmarks: Vec<String>,

        /// Remote name (default: origin)
        #[arg(long, default_value = "origin")]
        remote: String,

        /// Skip interactive confirmation prompts
        #[arg(long)]
        no_prompt: bool,

        /// Preview what would be pushed without pushing
        #[arg(long)]
        dry_run: bool,
    },

    /// Fetch metadata from remote then push local changes back (shorthand for fetch + push)
    #[command(display_order = 42)]
    Sync {
        /// Remote name (default: origin)
        #[arg(long, default_value = "origin")]
        remote: String,

        /// Skip interactive confirmation prompts
        #[arg(long)]
        no_prompt: bool,

        /// Preview what would happen without making any changes
        #[arg(long)]
        dry_run: bool,
    },

    /// Bridge jjj problems and solutions with GitHub Issues and Pull Requests
    #[command(display_order = 43)]
    Github {
        #[command(subcommand)]
        action: Option<GitHubSyncAction>,

        /// Preview actions without making any changes
        #[arg(long)]
        dry_run: bool,
    },

    /// List all tags in use with counts
    #[command(display_order = 33)]
    Tags {
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    // ── Setup & utilities ──────────────────────────────────────────────────
    /// Initialize jjj metadata in the current jj repository
    #[command(display_order = 50)]
    Init,

    /// Manage the local SQLite cache (full-text search index and embeddings)
    #[command(display_order = 51)]
    Db {
        #[command(subcommand)]
        action: DbAction,
    },

    /// Generate shell completions (bash, zsh, fish, etc.)
    #[command(display_order = 52)]
    Completion {
        /// Target shell
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
// Db Commands
// =============================================================================

#[derive(Subcommand)]
pub enum DbAction {
    /// Show local database status (entity counts, index health)
    Status,

    /// Rebuild the database from markdown files (re-indexes FTS and embeddings)
    Rebuild,
}

// =============================================================================
// Events Commands
// =============================================================================

#[derive(Subcommand)]
pub enum EventsAction {
    /// Verify event count by replaying the jjj commit history
    Rebuild,

    /// Check that event log states match current entity states
    Validate,
}

// =============================================================================
// Problem Commands
// =============================================================================

#[derive(Subcommand)]
pub enum ProblemAction {
    /// Create a new problem
    #[command(display_order = 0)]
    New {
        /// Problem title (the question to answer)
        title: String,

        /// Priority: critical (P0), high (P1), medium (P2), low (P3)
        #[arg(long, default_value = "medium")]
        priority: String,

        /// Parent problem ID — makes this a sub-problem
        #[arg(long)]
        parent: Option<String>,

        /// Milestone to assign this problem to
        #[arg(long)]
        milestone: Option<String>,

        /// Skip duplicate-detection checks
        #[arg(long, short = 'f')]
        force: bool,

        /// Initial context for the problem (why is this hard / background)
        #[arg(long)]
        context: Option<String>,

        /// Comma-separated tags (e.g., --tags backend,auth,size:L)
        #[arg(long, value_delimiter = ',')]
        tags: Vec<String>,
    },

    /// List problems with optional filters
    #[command(display_order = 1)]
    List {
        /// Filter by status: open, in_progress, solved, dissolved
        #[arg(long)]
        status: Option<String>,

        /// Show problems as a hierarchy tree
        #[arg(long)]
        tree: bool,

        /// Filter to problems in a specific milestone
        #[arg(long)]
        milestone: Option<String>,

        /// Filter by title keyword
        #[arg(long)]
        search: Option<String>,

        /// Filter by assignee
        #[arg(long)]
        assignee: Option<String>,

        /// Filter by tag (case-insensitive exact match)
        #[arg(long)]
        tag: Option<String>,

        /// Sort by: priority, status, created, title
        #[arg(long, default_value = "priority")]
        sort: String,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Show details for a problem
    #[command(display_order = 2)]
    Show {
        /// Problem ID, short prefix, or fuzzy title (e.g., "auth bug", 01957d)
        problem_id: String,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Edit a problem's title, status, or priority
    #[command(display_order = 3)]
    Edit {
        /// Problem ID, short prefix, or fuzzy title
        problem_id: String,

        /// New title
        #[arg(long)]
        title: Option<String>,

        /// New status: open, in_progress, solved, dissolved
        #[arg(long)]
        status: Option<String>,

        /// New priority: critical (P0), high (P1), medium (P2), low (P3)
        #[arg(long)]
        priority: Option<String>,

        /// New parent problem (re-parents this as a sub-problem)
        #[arg(long)]
        parent: Option<String>,

        /// Add a tag
        #[arg(long)]
        add_tag: Option<String>,

        /// Remove a tag
        #[arg(long)]
        remove_tag: Option<String>,
    },

    /// Show problems as a hierarchy tree
    #[command(display_order = 4)]
    Tree {
        /// Root problem to start from (default: all root problems)
        problem_id: Option<String>,
    },

    /// Mark a problem solved (requires an approved solution or all sub-problems solved)
    #[command(display_order = 5)]
    Solve {
        /// Problem ID, short prefix, or fuzzy title
        problem_id: String,

        /// Close the linked GitHub issue after solving
        #[arg(long)]
        github_close: bool,
    },

    /// Dissolve a problem — mark it as based on false premises, not truly a problem
    #[command(display_order = 6)]
    Dissolve {
        /// Problem ID, short prefix, or fuzzy title
        problem_id: String,

        /// Explanation of why the problem turned out to be misconceived
        #[arg(long)]
        reason: Option<String>,

        /// Close the linked GitHub issue after dissolving
        #[arg(long)]
        github_close: bool,
    },

    /// Assign a problem to yourself or someone else
    #[command(display_order = 7)]
    Assign {
        /// Problem ID, short prefix, or fuzzy title
        problem_id: String,

        /// Assignee (defaults to your jj identity)
        #[arg(long)]
        to: Option<String>,
    },

    /// Reopen a solved or dissolved problem (transition back to open)
    #[command(display_order = 8)]
    Reopen {
        /// Problem ID, short prefix, or fuzzy title
        problem_id: String,
    },

    /// Mark a problem as a duplicate of another — dissolves it with a back-reference
    #[command(display_order = 9)]
    Duplicate {
        /// The duplicate problem to dissolve (ID, prefix, or title)
        problem_id: String,

        /// The canonical problem this is a duplicate of (ID, prefix, or title)
        #[arg(long)]
        of: String,
    },

    /// Render problem hierarchy as ASCII DAG
    #[command(display_order = 10)]
    Graph {
        /// Filter to problems in a specific milestone
        #[arg(long)]
        milestone: Option<String>,

        /// Include solved and dissolved problems
        #[arg(long)]
        all: bool,
    },
}

// =============================================================================
// Solution Commands
// =============================================================================

#[derive(Subcommand)]
pub enum SolutionAction {
    /// Propose a new solution (conjecture) for a problem
    #[command(display_order = 0)]
    New {
        /// Solution title describing the approach
        title: String,

        /// Problem this solution addresses (prompts if omitted)
        #[arg(long)]
        problem: Option<String>,

        /// ID of an older solution that this one supersedes
        #[arg(long)]
        supersedes: Option<String>,

        /// Request review from specific people; use @name or name:severity
        #[arg(long, value_name = "REVIEWER")]
        reviewer: Vec<String>,

        /// Skip duplicate-detection checks
        #[arg(long, short = 'f')]
        force: bool,

        /// Comma-separated tags (e.g., --tags backend,refactor)
        #[arg(long, value_delimiter = ',')]
        tags: Vec<String>,
    },

    /// List solutions with optional filters
    #[command(display_order = 1)]
    List {
        /// Filter to solutions for a specific problem
        #[arg(long)]
        problem: Option<String>,

        /// Filter by status: proposed, submitted, approved, withdrawn
        #[arg(long)]
        status: Option<String>,

        /// Filter by assignee
        #[arg(long)]
        assignee: Option<String>,

        /// Filter by title keyword
        #[arg(long)]
        search: Option<String>,

        /// Filter by tag (case-insensitive exact match)
        #[arg(long)]
        tag: Option<String>,

        /// Sort by: status, created, title
        #[arg(long, default_value = "status")]
        sort: String,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Show details for a solution
    #[command(display_order = 2)]
    Show {
        /// Solution ID, short prefix, or fuzzy title (e.g., "pooling", 01958a)
        solution_id: String,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Edit a solution's title or status
    #[command(display_order = 3)]
    Edit {
        /// Solution ID, short prefix, or fuzzy title
        solution_id: String,

        /// New title
        #[arg(long)]
        title: Option<String>,

        /// New status: proposed, submitted, approved, withdrawn
        #[arg(long)]
        status: Option<String>,

        /// Add a tag
        #[arg(long)]
        add_tag: Option<String>,

        /// Remove a tag
        #[arg(long)]
        remove_tag: Option<String>,
    },

    /// Link the current jj change to a solution
    #[command(display_order = 4)]
    Attach {
        /// Solution ID, short prefix, or fuzzy title
        solution_id: String,

        /// Skip validation checks (change existence, duplicate attachment)
        #[arg(long)]
        force: bool,
    },

    /// Unlink a jj change from a solution
    #[command(display_order = 5)]
    Detach {
        /// Solution ID, short prefix, or fuzzy title
        solution_id: String,

        /// Change ID to detach (defaults to the current change)
        change_id: Option<String>,

        /// Skip safety checks (review state, last-change guard)
        #[arg(long)]
        force: bool,
    },

    /// Submit a solution for review — opens it for critique
    #[command(display_order = 6)]
    Submit {
        /// Solution ID, short prefix, or fuzzy title
        solution_id: String,
    },

    /// Approve a solution — resolve critiques, integrate code, solve the problem
    ///
    /// The solution must be submitted (`jjj solution submit`) with no open
    /// critiques. Use --force to override open critiques.
    #[command(display_order = 7)]
    Approve {
        /// Solution ID, short prefix, or fuzzy title (optional — defaults to current change)
        solution_id: Option<String>,

        /// Approve despite open critiques
        #[arg(long)]
        force: bool,

        /// Record why this solution was approved
        #[arg(long)]
        rationale: Option<String>,

        /// Skip the rationale prompt
        #[arg(long)]
        no_rationale: bool,
    },

    /// Withdraw a solution — pull it back from review
    #[command(display_order = 8)]
    Withdraw {
        /// Solution ID, short prefix, or fuzzy title
        solution_id: String,

        /// Record why this solution was withdrawn
        #[arg(long)]
        rationale: Option<String>,

        /// Skip the rationale prompt
        #[arg(long)]
        no_rationale: bool,
    },

    /// Assign a solution to yourself or someone else
    #[command(display_order = 9)]
    Assign {
        /// Solution ID, short prefix, or fuzzy title
        solution_id: String,

        /// Assignee (defaults to your jj identity)
        #[arg(long)]
        to: Option<String>,
    },

    /// Switch back to working on an existing solution
    #[command(display_order = 10)]
    Resume {
        /// Solution ID, short prefix, or fuzzy title
        solution_id: String,
    },

    /// Sign off as a reviewer — addresses your open review critique (LGTM)
    #[command(display_order = 11)]
    Lgtm {
        /// Solution ID, short prefix, or fuzzy title
        solution_id: String,
    },

    /// Leave a reply on a critique of this solution
    #[command(display_order = 12)]
    Comment {
        /// Solution ID, short prefix, or fuzzy title (defaults to active solution)
        solution_id: Option<String>,

        /// Critique to reply to — ID, prefix, or fuzzy title
        #[arg(long, short = 'c')]
        critique: Option<String>,

        /// Reply body (prompted interactively if not given)
        body: Option<String>,
    },

    /// Show jj diff for a solution's attached change IDs
    #[command(display_order = 13)]
    Diff {
        /// Solution ID, short prefix, or fuzzy title
        solution_id: String,
    },
}

// =============================================================================
// Critique Commands
// =============================================================================

#[derive(Subcommand)]
pub enum CritiqueAction {
    /// Raise a new critique against a solution
    #[command(display_order = 0)]
    New {
        /// Solution to critique (ID, short prefix, or fuzzy title)
        solution_id: String,

        /// Brief description of the critique
        title: String,

        /// Severity: low, medium, high, critical
        #[arg(long, default_value = "medium")]
        severity: String,

        /// Source file relevant to this critique
        #[arg(long)]
        file: Option<String>,

        /// Source line number
        #[arg(long)]
        line: Option<usize>,

        /// Assign a specific reviewer to address this critique
        #[arg(long)]
        reviewer: Option<String>,
    },

    /// List critiques with optional filters
    #[command(display_order = 1)]
    List {
        /// Filter to critiques on a specific solution
        #[arg(long)]
        solution: Option<String>,

        /// Filter by status: open, addressed, valid, dismissed
        #[arg(long)]
        status: Option<String>,

        /// Filter by assigned reviewer
        #[arg(long)]
        reviewer: Option<String>,

        /// Filter by author (substring match)
        #[arg(long)]
        author: Option<String>,

        /// Show only critiques authored by you
        #[arg(long)]
        mine: bool,

        /// Filter by title keyword
        #[arg(long)]
        search: Option<String>,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Show details for a critique
    #[command(display_order = 2)]
    Show {
        /// Critique ID, short prefix, or fuzzy title
        critique_id: String,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Edit a critique's title, severity, or status
    #[command(display_order = 3)]
    Edit {
        /// Critique ID, short prefix, or fuzzy title
        critique_id: String,

        /// New title
        #[arg(long)]
        title: Option<String>,

        /// New severity: low, medium, high, critical
        #[arg(long)]
        severity: Option<String>,

        /// New status: open, addressed, valid, dismissed
        #[arg(long)]
        status: Option<String>,
    },

    /// Mark a critique as addressed — the solution was updated to handle it
    #[command(display_order = 4)]
    Address {
        /// Critique ID, short prefix, or fuzzy title
        critique_id: String,
    },

    /// Validate a critique — confirm it is correct and the solution should be withdrawn
    #[command(display_order = 5)]
    Validate {
        /// Critique ID, short prefix, or fuzzy title
        critique_id: String,
    },

    /// Dismiss a critique — it is incorrect or no longer relevant
    #[command(display_order = 6)]
    Dismiss {
        /// Critique ID, short prefix, or fuzzy title
        critique_id: String,
    },

    /// Reply to a critique with a comment
    #[command(display_order = 7)]
    Reply {
        /// Critique ID, short prefix, or fuzzy title
        critique_id: String,

        /// Reply text
        body: String,
    },
}

// =============================================================================
// Milestone Commands
// =============================================================================

#[derive(Subcommand)]
pub enum MilestoneAction {
    /// Create a new milestone
    #[command(display_order = 0)]
    New {
        /// Milestone title
        title: String,

        /// Target completion date (YYYY-MM-DD)
        #[arg(long)]
        date: Option<String>,
    },

    /// List all milestones
    #[command(display_order = 1)]
    List {
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Show details for a milestone
    #[command(display_order = 2)]
    Show {
        /// Milestone ID, short prefix, or fuzzy title (e.g., "v1.0", 01960c)
        milestone_id: String,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Edit a milestone's title, date, or status
    #[command(display_order = 3)]
    Edit {
        /// Milestone ID, short prefix, or fuzzy title
        milestone_id: String,

        /// New title
        #[arg(long)]
        title: Option<String>,

        /// New target date (YYYY-MM-DD)
        #[arg(long)]
        date: Option<String>,

        /// New status: planning, active, completed, cancelled
        #[arg(long)]
        status: Option<String>,
    },

    /// Show milestone roadmap: problems and their solution progress
    #[command(display_order = 4)]
    Roadmap {
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Add a problem to a milestone
    #[command(display_order = 5)]
    AddProblem {
        /// Milestone ID, short prefix, or fuzzy title
        milestone_id: String,

        /// Problem ID, short prefix, or fuzzy title
        problem_id: String,
    },

    /// Remove a problem from a milestone
    #[command(display_order = 6)]
    RemoveProblem {
        /// Milestone ID, short prefix, or fuzzy title
        milestone_id: String,

        /// Problem ID, short prefix, or fuzzy title
        problem_id: String,
    },

    /// Assign a milestone to yourself or someone else
    #[command(display_order = 7)]
    Assign {
        /// Milestone ID, short prefix, or fuzzy title
        milestone_id: String,

        /// Assignee (defaults to your jj identity)
        #[arg(long)]
        to: Option<String>,
    },

    /// Show completion statistics for a milestone
    #[command(display_order = 8)]
    Status {
        /// Milestone ID, short prefix, or fuzzy title
        milestone_id: String,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
}

// =============================================================================
// GitHub Sync Commands
// =============================================================================

#[derive(Subcommand)]
pub enum GitHubSyncAction {
    /// Import a GitHub issue (or all unlinked issues) as jjj problems
    #[command(display_order = 0)]
    Import {
        /// Issue number to import (e.g., "123" or "#123")
        issue: Option<String>,

        /// Import every unlinked open issue
        #[arg(long)]
        all: bool,

        /// Filter by label when using --all
        #[arg(long)]
        label: Option<String>,
    },

    /// Create or update a GitHub PR for a solution
    #[command(display_order = 1)]
    Pr {
        /// Solution ID or title (defaults to the current change's solution)
        solution_id: Option<String>,

        /// Base branch for the PR
        #[arg(long, default_value = "main")]
        base: String,
    },

    /// Show sync status for all linked problems and solutions
    #[command(display_order = 2)]
    Status,

    /// Squash-merge the linked GitHub PR for a solution
    #[command(display_order = 3)]
    Merge {
        /// Solution ID or title
        solution_id: String,
    },

    /// Close the linked GitHub issue for a problem
    #[command(display_order = 4)]
    Close {
        /// Problem ID or title
        problem_id: String,
    },

    /// Reopen the linked GitHub issue for a problem
    #[command(display_order = 5)]
    Reopen {
        /// Problem ID or title
        problem_id: String,
    },

    /// Push local state back to GitHub: refresh PR bodies and sync issue open/closed state
    #[command(display_order = 6)]
    Push,
}
