//! The five jjj entity kinds and their canonical string / table / prefix
//! mappings.
//!
//! These mappings were previously hand-matched in several layers (display,
//! search, db cache, TUI), which let them drift. Centralizing them here makes
//! [`EntityType`] the single source of truth and lets low-level modules use it
//! without depending on the TUI (where the enum used to live).

use serde::{Deserialize, Serialize};

/// A jjj entity kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EntityType {
    Problem,
    Solution,
    Critique,
    Milestone,
    Finding,
}

impl EntityType {
    /// All variants, for iteration.
    pub const ALL: [EntityType; 5] = [
        EntityType::Problem,
        EntityType::Solution,
        EntityType::Critique,
        EntityType::Milestone,
        EntityType::Finding,
    ];

    /// Lowercase singular name (`"problem"`). Matches the serde encoding.
    pub fn as_str(self) -> &'static str {
        match self {
            EntityType::Problem => "problem",
            EntityType::Solution => "solution",
            EntityType::Critique => "critique",
            EntityType::Milestone => "milestone",
            EntityType::Finding => "finding",
        }
    }

    /// SQLite table / metadata directory name (`"problems"`).
    pub fn table(self) -> &'static str {
        match self {
            EntityType::Problem => "problems",
            EntityType::Solution => "solutions",
            EntityType::Critique => "critiques",
            EntityType::Milestone => "milestones",
            EntityType::Finding => "findings",
        }
    }

    /// Single-character listing prefix (`'p'`).
    pub fn prefix(self) -> char {
        match self {
            EntityType::Problem => 'p',
            EntityType::Solution => 's',
            EntityType::Critique => 'c',
            EntityType::Milestone => 'm',
            EntityType::Finding => 'f',
        }
    }

    /// Parse from the singular lowercase name. Returns `None` for anything else.
    pub fn from_singular(s: &str) -> Option<EntityType> {
        match s {
            "problem" => Some(EntityType::Problem),
            "solution" => Some(EntityType::Solution),
            "critique" => Some(EntityType::Critique),
            "milestone" => Some(EntityType::Milestone),
            "finding" => Some(EntityType::Finding),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mappings_round_trip() {
        for et in EntityType::ALL {
            assert_eq!(EntityType::from_singular(et.as_str()), Some(et));
            // table is the pluralized singular
            assert!(et.table().starts_with(et.as_str()));
        }
        assert_eq!(EntityType::Problem.prefix(), 'p');
        assert_eq!(EntityType::Milestone.table(), "milestones");
        assert_eq!(EntityType::from_singular("nope"), None);
    }
}
