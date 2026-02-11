//! Entity resolution from user input to UUID.
//!
//! Resolution priority:
//! 1. Exact UUID match
//! 2. Prefix match (hex string starting with input)
//! 3. Fuzzy title search via SQLite FTS

use crate::id::{is_hex_prefix, is_uuid};

/// Result of resolving user input to entities.
#[derive(Debug, Clone)]
pub enum ResolveResult {
    /// Exactly one entity matched
    Single(String),
    /// Multiple entities matched - need disambiguation
    Multiple(Vec<ResolveMatch>),
    /// No entities matched
    None,
}

/// A matched entity with its ID and title.
#[derive(Debug, Clone)]
pub struct ResolveMatch {
    pub id: String,
    pub title: String,
}

/// Resolve user input to entity ID(s).
///
/// Takes a list of (id, title) pairs representing available entities.
pub fn resolve(input: &str, entities: &[(String, String)]) -> ResolveResult {
    // 1. Exact UUID match
    if is_uuid(input) {
        if entities.iter().any(|(id, _)| id == input) {
            return ResolveResult::Single(input.to_string());
        }
        return ResolveResult::None;
    }

    // 2. Prefix match (if input looks like hex)
    if is_hex_prefix(input) {
        let normalized_input: String = input.chars().filter(|c| *c != '-').collect();
        let matches: Vec<_> = entities
            .iter()
            .filter(|(id, _)| {
                let normalized_id: String = id.chars().filter(|c| *c != '-').collect();
                normalized_id
                    .to_lowercase()
                    .starts_with(&normalized_input.to_lowercase())
            })
            .map(|(id, title)| ResolveMatch {
                id: id.clone(),
                title: title.clone(),
            })
            .collect();

        return match matches.len() {
            0 => ResolveResult::None,
            1 => ResolveResult::Single(matches[0].id.clone()),
            _ => ResolveResult::Multiple(matches),
        };
    }

    // 3. Fuzzy title search (simple contains for now, FTS in actual use)
    let input_lower = input.to_lowercase();
    let matches: Vec<_> = entities
        .iter()
        .filter(|(_, title)| title.to_lowercase().contains(&input_lower))
        .map(|(id, title)| ResolveMatch {
            id: id.clone(),
            title: title.clone(),
        })
        .collect();

    match matches.len() {
        0 => ResolveResult::None,
        1 => ResolveResult::Single(matches[0].id.clone()),
        _ => ResolveResult::Multiple(matches),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_entities() -> Vec<(String, String)> {
        vec![
            (
                "01957d3e-a8b2-7def-8c3a-9f4e5d6c7b8a".to_string(),
                "Fix auth timeout bug".to_string(),
            ),
            (
                "01957d3e-b1c4-7abc-9d2e-3f4a5b6c7d8e".to_string(),
                "Auth token refresh fails".to_string(),
            ),
            (
                "02957d3e-c2d5-7fed-ae4b-5c6d7e8f9a0b".to_string(),
                "Database connection pooling".to_string(),
            ),
        ]
    }

    #[test]
    fn test_resolve_exact_uuid() {
        let entities = test_entities();
        match resolve("01957d3e-a8b2-7def-8c3a-9f4e5d6c7b8a", &entities) {
            ResolveResult::Single(id) => assert_eq!(id, "01957d3e-a8b2-7def-8c3a-9f4e5d6c7b8a"),
            other => panic!("Expected Single, got {:?}", other),
        }
    }

    #[test]
    fn test_resolve_uuid_not_found() {
        let entities = test_entities();
        match resolve("99999999-9999-9999-9999-999999999999", &entities) {
            ResolveResult::None => {}
            other => panic!("Expected None, got {:?}", other),
        }
    }

    #[test]
    fn test_resolve_prefix_unique() {
        let entities = test_entities();
        // "02957d" only matches the third entity
        match resolve("02957d", &entities) {
            ResolveResult::Single(id) => assert_eq!(id, "02957d3e-c2d5-7fed-ae4b-5c6d7e8f9a0b"),
            other => panic!("Expected Single, got {:?}", other),
        }
    }

    #[test]
    fn test_resolve_prefix_ambiguous() {
        let entities = test_entities();
        // "01957d" matches two entities
        match resolve("01957d", &entities) {
            ResolveResult::Multiple(matches) => {
                assert_eq!(matches.len(), 2);
            }
            other => panic!("Expected Multiple, got {:?}", other),
        }
    }

    #[test]
    fn test_resolve_title_unique() {
        let entities = test_entities();
        match resolve("database", &entities) {
            ResolveResult::Single(id) => assert_eq!(id, "02957d3e-c2d5-7fed-ae4b-5c6d7e8f9a0b"),
            other => panic!("Expected Single, got {:?}", other),
        }
    }

    #[test]
    fn test_resolve_title_ambiguous() {
        let entities = test_entities();
        // "auth" matches two entities
        match resolve("auth", &entities) {
            ResolveResult::Multiple(matches) => {
                assert_eq!(matches.len(), 2);
            }
            other => panic!("Expected Multiple, got {:?}", other),
        }
    }

    #[test]
    fn test_resolve_title_not_found() {
        let entities = test_entities();
        match resolve("nonexistent", &entities) {
            ResolveResult::None => {}
            other => panic!("Expected None, got {:?}", other),
        }
    }
}
