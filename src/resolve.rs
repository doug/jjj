//! Entity resolution from user input to UUID.
//!
//! Resolution priority:
//! 1. Exact UUID match
//! 2. Prefix match (hex string of ≥6 hex digits starting with input)
//! 3. Fuzzy title match (case-insensitive substring; not SQLite FTS)
//!
//! An empty or whitespace-only reference resolves to **nothing**, never to a
//! match. The fuzzy step is a substring test, and every title contains the empty
//! string — so `""` matched every entity, and in a repository holding exactly one
//! it matched that one *successfully and silently*. Since the usual source of an
//! empty argument is an unset shell variable (`jjj solution approve "$SID"`),
//! that turned a scripting slip into approving, dissolving or attaching to an
//! arbitrary entity. Found when a swarm agent called `solution attach` with no
//! argument twenty times in an hour.

use crate::id::{is_hex_prefix, is_uuid};

/// Result of resolving user input to entities.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResolveResult {
    /// Exactly one entity matched
    Single(String),
    /// Multiple entities matched - need disambiguation
    Multiple(Vec<ResolveMatch>),
    /// No entities matched
    None,
}

/// A matched entity with its ID and title.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolveMatch {
    pub id: String,
    pub title: String,
}

/// Resolve user input to entity ID(s).
///
/// Takes a list of (id, title) pairs representing available entities.
pub fn resolve(input: &str, entities: &[(String, String)]) -> ResolveResult {
    // 0. An empty reference identifies nothing. See the module docs: the fuzzy
    //    step would otherwise match every entity, and silently succeed whenever
    //    exactly one existed.
    if input.trim().is_empty() {
        return ResolveResult::None;
    }

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

    // 3. Fuzzy title match: case-insensitive substring over titles.
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

/// Parse an entity reference like "p/01957d" or "s/abc123".
///
/// Returns (entity_type, id_prefix) if valid, None otherwise.
pub fn parse_entity_reference(input: &str) -> Option<(&str, &str)> {
    // Split off the first character. Using `chars()` (not `split_at(1)`)
    // avoids panicking when the input begins with a multibyte UTF-8 character
    // — e.g. a fuzzy title query like "é bug" coming from `jjj search`.
    let mut chars = input.chars();
    let type_char = chars.next()?;
    let rest = chars.as_str();
    if !rest.starts_with('/') {
        return None;
    }

    let id = &rest[1..];
    if id.is_empty() {
        return None;
    }

    let entity_type = match type_char {
        'p' => "problem",
        's' => "solution",
        'c' => "critique",
        'm' => "milestone",
        _ => return None,
    };

    Some((entity_type, id))
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

    #[test]
    fn test_parse_entity_reference_valid() {
        assert_eq!(
            parse_entity_reference("p/01957d"),
            Some(("problem", "01957d"))
        );
        assert_eq!(
            parse_entity_reference("s/abc123"),
            Some(("solution", "abc123"))
        );
        assert_eq!(parse_entity_reference("c/xyz"), Some(("critique", "xyz")));
        assert_eq!(parse_entity_reference("m/123"), Some(("milestone", "123")));
    }

    #[test]
    fn test_parse_entity_reference_invalid() {
        assert_eq!(parse_entity_reference("p/"), None);
        assert_eq!(parse_entity_reference("x/123"), None);
        assert_eq!(parse_entity_reference("problem"), None);
        assert_eq!(parse_entity_reference("p123"), None);
        assert_eq!(parse_entity_reference(""), None);
        assert_eq!(parse_entity_reference("p"), None);
    }

    #[test]
    fn test_parse_entity_reference_multibyte_does_not_panic() {
        // Multibyte leading characters (e.g. a fuzzy title from `jjj search`)
        // must return None, not panic on a non-char-boundary split.
        assert_eq!(parse_entity_reference("é bug"), None);
        assert_eq!(parse_entity_reference("日本語"), None);
        assert_eq!(parse_entity_reference("🦀/x"), None);
        assert_eq!(parse_entity_reference("café"), None);
    }
}

#[cfg(test)]
mod empty_reference_tests {
    use super::*;

    fn entities() -> Vec<(String, String)> {
        vec![(
            "01900000-0000-7000-8000-000000000001".to_string(),
            "The only solution".to_string(),
        )]
    }

    /// An empty reference must never resolve, however few entities exist.
    ///
    /// The fuzzy step is a substring test and every title contains "", so a
    /// repository holding one entity resolved `""` to it — silently and
    /// successfully. An unset shell variable would then approve, dissolve or
    /// attach to an arbitrary entity.
    #[test]
    fn an_empty_reference_matches_nothing() {
        assert_eq!(resolve("", &entities()), ResolveResult::None);
        assert_eq!(resolve("   ", &entities()), ResolveResult::None);
        assert_eq!(resolve("\t\n", &entities()), ResolveResult::None);
    }

    #[test]
    fn an_empty_reference_matches_nothing_even_with_many_entities() {
        let many: Vec<_> = (1..=5)
            .map(|i| {
                (
                    format!("0190000{i}-0000-7000-8000-00000000000{i}"),
                    format!("Solution {i}"),
                )
            })
            .collect();
        assert_eq!(resolve("", &many), ResolveResult::None);
    }

    #[test]
    fn a_real_reference_still_resolves() {
        // The guard must not break ordinary use.
        let e = entities();
        assert_eq!(
            resolve("only solution", &e),
            ResolveResult::Single(e[0].0.clone())
        );
        assert_eq!(resolve(&e[0].0, &e), ResolveResult::Single(e[0].0.clone()));
    }
}
