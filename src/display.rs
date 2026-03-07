//! Display formatting utilities for entity IDs.

/// Minimum prefix length for truncated display.
const MIN_PREFIX_LEN: usize = 6;

/// Calculate unambiguous prefixes for a list of UUIDs.
///
/// Returns a Vec of (uuid, prefix) pairs where each prefix is the shortest
/// unambiguous prefix (minimum 6 chars).
pub(crate) fn truncated_prefixes(uuids: &[&str]) -> Vec<(String, String)> {
    uuids
        .iter()
        .map(|uuid| {
            let prefix = shortest_unambiguous_prefix(uuid, uuids);
            (uuid.to_string(), prefix)
        })
        .collect()
}

/// Find the shortest unambiguous prefix for a UUID within a set.
fn shortest_unambiguous_prefix(uuid: &str, all_uuids: &[&str]) -> String {
    // Remove hyphens for prefix calculation
    let normalized: String = uuid.chars().filter(|c| *c != '-').collect();
    let all_normalized: Vec<String> = all_uuids
        .iter()
        .map(|u| u.chars().filter(|c| *c != '-').collect())
        .collect();

    for len in MIN_PREFIX_LEN..=normalized.len() {
        let prefix = &normalized[..len];
        let matches = all_normalized
            .iter()
            .filter(|u| u.starts_with(prefix))
            .count();
        if matches == 1 {
            return prefix.to_lowercase();
        }
    }

    // Fallback to full UUID (normalized)
    normalized.to_lowercase()
}

/// Format an entity for mixed-type listings with type prefix.
pub(crate) fn format_with_type_prefix(entity_type: &str, prefix: &str) -> String {
    let type_char = match entity_type {
        "problem" => "p",
        "solution" => "s",
        "critique" => "c",
        "milestone" => "m",
        _ => "?",
    };
    format!("{}/{}", type_char, prefix)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_single_uuid_uses_min_prefix() {
        let uuids = vec!["01957d3e-a8b2-7def-8c3a-9f4e5d6c7b8a"];
        let result = truncated_prefixes(&uuids);
        assert_eq!(result[0].1.len(), MIN_PREFIX_LEN);
    }

    #[test]
    fn test_different_uuids_use_min_prefix() {
        let uuids = vec![
            "01957d3e-a8b2-7def-8c3a-9f4e5d6c7b8a",
            "02957d3e-b1c4-7abc-9d2e-3f4a5b6c7d8e",
        ];
        let result = truncated_prefixes(&uuids);
        assert_eq!(result[0].1.len(), MIN_PREFIX_LEN);
        assert_eq!(result[1].1.len(), MIN_PREFIX_LEN);
    }

    #[test]
    fn test_similar_uuids_extend_prefix() {
        let uuids = vec![
            "a3f8c2de-a8b2-7def-8c3a-9f4e5d6c7b8a",
            "a3f8c2df-b1c4-7abc-9d2e-3f4a5b6c7d8e",
        ];
        let result = truncated_prefixes(&uuids);
        // Both start with "a3f8c2d", need to extend to 8 chars
        assert!(
            result[0].1.len() > MIN_PREFIX_LEN,
            "Prefix should extend: {}",
            result[0].1
        );
        assert!(
            result[1].1.len() > MIN_PREFIX_LEN,
            "Prefix should extend: {}",
            result[1].1
        );
        assert_ne!(result[0].1, result[1].1);
    }

    #[test]
    fn test_format_with_type_prefix() {
        assert_eq!(format_with_type_prefix("problem", "a3f8c2"), "p/a3f8c2");
        assert_eq!(format_with_type_prefix("solution", "b7e2f9"), "s/b7e2f9");
        assert_eq!(format_with_type_prefix("critique", "c1d2e3"), "c/c1d2e3");
        assert_eq!(format_with_type_prefix("milestone", "d4e5f6"), "m/d4e5f6");
    }
}
