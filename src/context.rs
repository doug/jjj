//! Command execution context providing shared access to storage and JJ client.

use crate::error::Result;
use crate::jj::JjClient;
use crate::storage::MetadataStore;

/// Shared context for all command execution.
pub struct CommandContext {
    pub store: MetadataStore,
}

impl CommandContext {
    pub fn new() -> Result<Self> {
        let jj_client = JjClient::new()?;
        let store = MetadataStore::new(jj_client)?;
        Ok(Self { store })
    }

    pub fn jj(&self) -> &JjClient {
        &self.store.jj_client
    }

    /// Resolve a problem ID from user input.
    pub fn resolve_problem(&self, input: &str) -> Result<String> {
        use crate::picker::pick_one;
        use crate::resolve::{resolve, ResolveResult};

        let problems = self.store.list_problems()?;
        let entities: Vec<(String, String)> = problems
            .iter()
            .map(|p| (p.id.clone(), p.title.clone()))
            .collect();

        match resolve(input, &entities) {
            ResolveResult::Single(id) => Ok(id),
            ResolveResult::Multiple(matches) => pick_one(&matches, "problem"),
            ResolveResult::None => Err(crate::error::JjjError::ProblemNotFound(input.to_string())),
        }
    }

    /// Resolve a solution ID from user input.
    pub fn resolve_solution(&self, input: &str) -> Result<String> {
        use crate::picker::pick_one;
        use crate::resolve::{resolve, ResolveResult};

        let solutions = self.store.list_solutions()?;
        let entities: Vec<(String, String)> = solutions
            .iter()
            .map(|s| (s.id.clone(), s.title.clone()))
            .collect();

        match resolve(input, &entities) {
            ResolveResult::Single(id) => Ok(id),
            ResolveResult::Multiple(matches) => pick_one(&matches, "solution"),
            ResolveResult::None => Err(crate::error::JjjError::SolutionNotFound(input.to_string())),
        }
    }

    /// Resolve a critique ID from user input.
    pub fn resolve_critique(&self, input: &str) -> Result<String> {
        use crate::picker::pick_one;
        use crate::resolve::{resolve, ResolveResult};

        let critiques = self.store.list_critiques()?;
        let entities: Vec<(String, String)> = critiques
            .iter()
            .map(|c| (c.id.clone(), c.title.clone()))
            .collect();

        match resolve(input, &entities) {
            ResolveResult::Single(id) => Ok(id),
            ResolveResult::Multiple(matches) => pick_one(&matches, "critique"),
            ResolveResult::None => Err(crate::error::JjjError::CritiqueNotFound(input.to_string())),
        }
    }

    /// Resolve a milestone ID from user input.
    pub fn resolve_milestone(&self, input: &str) -> Result<String> {
        use crate::picker::pick_one;
        use crate::resolve::{resolve, ResolveResult};

        let milestones = self.store.list_milestones()?;
        let entities: Vec<(String, String)> = milestones
            .iter()
            .map(|m| (m.id.clone(), m.title.clone()))
            .collect();

        match resolve(input, &entities) {
            ResolveResult::Single(id) => Ok(id),
            ResolveResult::Multiple(matches) => pick_one(&matches, "milestone"),
            ResolveResult::None => {
                Err(crate::error::JjjError::MilestoneNotFound(input.to_string()))
            }
        }
    }
}
