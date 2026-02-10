//! UUID7 generation and utilities for entity IDs.

use uuid::Uuid;

/// Generate a new UUID7 identifier.
///
/// UUID7 is time-ordered, so IDs sort chronologically.
pub fn generate_id() -> String {
    Uuid::now_v7().to_string()
}

/// Check if a string is a valid UUID.
pub fn is_uuid(s: &str) -> bool {
    Uuid::parse_str(s).is_ok()
}

/// Check if a string looks like a hex prefix (for prefix matching).
/// Must be 6+ hex characters.
pub fn is_hex_prefix(s: &str) -> bool {
    s.len() >= 6 && s.chars().all(|c| c.is_ascii_hexdigit())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_id_is_valid_uuid() {
        let id = generate_id();
        assert!(is_uuid(&id), "Generated ID should be valid UUID: {}", id);
    }

    #[test]
    fn test_generate_id_is_unique() {
        let id1 = generate_id();
        let id2 = generate_id();
        assert_ne!(id1, id2, "Generated IDs should be unique");
    }

    #[test]
    fn test_generate_id_is_time_ordered() {
        let id1 = generate_id();
        std::thread::sleep(std::time::Duration::from_millis(1));
        let id2 = generate_id();
        assert!(id1 < id2, "UUIDs should sort chronologically: {} vs {}", id1, id2);
    }

    #[test]
    fn test_is_uuid() {
        assert!(is_uuid("01957d3e-a8b2-7def-8c3a-9f4e5d6c7b8a"));
        assert!(is_uuid("550e8400-e29b-41d4-a716-446655440000"));
        assert!(!is_uuid("p1"));
        assert!(!is_uuid("not-a-uuid"));
        assert!(!is_uuid(""));
    }

    #[test]
    fn test_is_hex_prefix() {
        assert!(is_hex_prefix("a3f8c2"));
        assert!(is_hex_prefix("01957d3e"));
        assert!(is_hex_prefix("ABCDEF"));
        assert!(!is_hex_prefix("a3f8c")); // too short
        assert!(!is_hex_prefix("auth")); // not hex
        assert!(!is_hex_prefix("p1"));
    }
}
