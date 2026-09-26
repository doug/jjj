/// `skip_serializing_if` helper: omit a zero clock so existing files and
/// entities that predate Lamport ordering keep byte-identical frontmatter.
pub(crate) fn is_zero(n: &u64) -> bool {
    *n == 0
}

mod config;
mod critique;
mod event;
mod finding;
mod milestone;
mod problem;
mod solution;

pub use config::{
    AutomationAction, AutomationConfig, AutomationRule, GitHubConfig, ProjectConfig, SyncConfig,
};
pub use critique::{Critique, CritiqueSeverity, CritiqueStatus, Reply};
pub use event::{Event, EventExtra, EventType};
pub use finding::{Finding, FindingStatus};
pub use milestone::{Milestone, MilestoneStatus};
pub use problem::{Confidence, Priority, Problem, ProblemStatus};
pub use solution::{Solution, SolutionStatus};
