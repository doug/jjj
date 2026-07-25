//! Display formatting utilities for entity IDs.

/// Minimum prefix length for truncated display.
const MIN_PREFIX_LEN: usize = 6;

/// Return a short prefix of an entity ID for display (first 6 hex chars).
pub(crate) fn short_id(id: &str) -> &str {
    &id[..MIN_PREFIX_LEN.min(id.len())]
}

/// Calculate unambiguous prefixes for a list of UUIDs.
///
/// Returns a Vec of (uuid, prefix) pairs where each prefix is the shortest
/// unambiguous prefix (minimum 6 chars).
///
/// O(n log n): a prefix is ambiguous only against the ids adjacent to it in
/// sorted order, so each id's required length is one past its longest common
/// prefix with its sorted neighbors. (The previous per-id scan over the whole
/// set was O(n²) and dominated `list` at 25K entities.)
pub(crate) fn truncated_prefixes(uuids: &[&str]) -> Vec<(String, String)> {
    let normalized: Vec<String> = uuids
        .iter()
        .map(|u| {
            u.chars()
                .filter(|c| *c != '-')
                .collect::<String>()
                .to_lowercase()
        })
        .collect();

    let mut order: Vec<usize> = (0..normalized.len()).collect();
    order.sort_by(|&a, &b| normalized[a].cmp(&normalized[b]));

    let mut required = vec![MIN_PREFIX_LEN; normalized.len()];
    for w in order.windows(2) {
        let (i, j) = (w[0], w[1]);
        let lcp = common_prefix_len(&normalized[i], &normalized[j]);
        // One char past the shared prefix disambiguates; a full-length match
        // (duplicate id) can't be disambiguated, so it falls back to the
        // whole normalized id.
        required[i] = required[i].max((lcp + 1).min(normalized[i].len()));
        required[j] = required[j].max((lcp + 1).min(normalized[j].len()));
    }

    uuids
        .iter()
        .zip(normalized.iter().zip(required))
        .map(|(uuid, (norm, req))| {
            let req = req.min(norm.len());
            (uuid.to_string(), norm[..req].to_string())
        })
        .collect()
}

/// Length of the common prefix of two ASCII (hex) strings, in bytes.
fn common_prefix_len(a: &str, b: &str) -> usize {
    a.bytes().zip(b.bytes()).take_while(|(x, y)| x == y).count()
}

/// Format an entity for mixed-type listings with type prefix.
pub(crate) fn format_with_type_prefix(entity_type: &str, prefix: &str) -> String {
    let type_char = crate::entity_type::EntityType::from_singular(entity_type)
        .map(|e| e.prefix())
        .unwrap_or('?');
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
    fn test_prefixes_unique_at_scale() {
        // Dense sequential ids (worst case for shared prefixes): every
        // returned prefix must still be unique and ≥ the minimum length.
        let ids: Vec<String> = (0..5000)
            .map(|i| format!("0195{:04x}-{:04x}-7def-8c3a-{:012x}", i, i % 0xffff, i))
            .collect();
        let refs: Vec<&str> = ids.iter().map(|s| s.as_str()).collect();
        let result = truncated_prefixes(&refs);
        let mut seen = std::collections::HashSet::new();
        for (_, prefix) in &result {
            assert!(prefix.len() >= MIN_PREFIX_LEN);
            assert!(seen.insert(prefix.clone()), "duplicate prefix {}", prefix);
        }
    }

    #[test]
    fn test_duplicate_ids_fall_back_to_full() {
        let uuids = vec![
            "a3f8c2de-a8b2-7def-8c3a-9f4e5d6c7b8a",
            "a3f8c2de-a8b2-7def-8c3a-9f4e5d6c7b8a",
        ];
        let result = truncated_prefixes(&uuids);
        assert_eq!(result[0].1, "a3f8c2dea8b27def8c3a9f4e5d6c7b8a");
        assert_eq!(result[1].1, result[0].1);
    }

    #[test]
    fn test_format_with_type_prefix() {
        assert_eq!(format_with_type_prefix("problem", "a3f8c2"), "p/a3f8c2");
        assert_eq!(format_with_type_prefix("solution", "b7e2f9"), "s/b7e2f9");
        assert_eq!(format_with_type_prefix("critique", "c1d2e3"), "c/c1d2e3");
        assert_eq!(format_with_type_prefix("milestone", "d4e5f6"), "m/d4e5f6");
    }
}
