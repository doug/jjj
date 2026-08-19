//! Comparing actor identities across the formats jjj has written.
//!
//! The actor is resolved by [`MetadataStore::get_current_user`]
//! (`JJJ_USER` → pod → jj `user.name`), which yields a bare name like `alice`
//! or `pod-7`. But assignment and critique authorship historically stored the
//! full jj identity instead — `Alice <alice@example.com>` — so a repository in
//! use before 0.5.1 holds both shapes, and a strict `==` would make `--mine`
//! and assignee filters silently miss the user's own work after an upgrade.
//!
//! [`actor_matches`] is the one place that knows this, so the rest of the code
//! can compare identities without each call site inventing its own rule (there
//! were three: `==`, `contains`, and a bidirectional `contains`).
//!
//! [`MetadataStore::get_current_user`]: crate::storage::MetadataStore::get_current_user

/// The bare name part of an identity: `Alice <a@e.com>` → `Alice`.
fn name_part(identity: &str) -> &str {
    match identity.split_once('<') {
        Some((name, _)) => name.trim(),
        None => identity.trim(),
    }
}

/// Whether `stored` refers to the same actor as `actor`.
///
/// Matches when the two are equal, or when they agree once both are reduced to
/// their name part — so `alice`, `alice <alice@example.com>` and
/// `alice <other@example.com>` are all the same actor.
///
/// Deliberately **not** a substring test: `contains` made `bo` match `bob` and
/// every actor match the empty string, which silently widened `--mine` into
/// "everything".
pub fn actor_matches(stored: &str, actor: &str) -> bool {
    let stored = stored.trim();
    let actor = actor.trim();
    if stored.is_empty() || actor.is_empty() {
        return false;
    }
    if stored.eq_ignore_ascii_case(actor) {
        return true;
    }
    name_part(stored).eq_ignore_ascii_case(name_part(actor))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_names_match() {
        assert!(actor_matches("alice", "alice"));
        assert!(!actor_matches("alice", "bob"));
    }

    #[test]
    fn a_bare_name_matches_the_full_jj_identity() {
        // The upgrade case: assignee written by 0.4.x, actor resolved by 0.5.1.
        assert!(actor_matches("Alice <alice@example.com>", "Alice"));
        assert!(actor_matches("Alice", "Alice <alice@example.com>"));
    }

    #[test]
    fn the_email_is_not_what_identifies_someone() {
        // Same person, different machine config.
        assert!(actor_matches(
            "Alice <alice@work.example>",
            "Alice <alice@home.example>"
        ));
    }

    #[test]
    fn different_people_never_match() {
        assert!(!actor_matches(
            "Alice <a@example.com>",
            "Bob <b@example.com>"
        ));
        assert!(!actor_matches("agent-a", "agent-b"));
    }

    #[test]
    fn a_prefix_is_not_a_match() {
        // The old `contains` test made these match, quietly folding two agents
        // into one.
        assert!(!actor_matches("bob", "bo"));
        assert!(!actor_matches("agent-1", "agent-10"));
    }

    #[test]
    fn empty_matches_nothing() {
        // `"".contains(x)` logic previously made an unset identity match
        // everything, turning `--mine` into a full listing.
        assert!(!actor_matches("", "alice"));
        assert!(!actor_matches("alice", ""));
        assert!(!actor_matches("", ""));
    }

    #[test]
    fn case_differences_are_the_same_actor() {
        assert!(actor_matches("Alice", "alice"));
    }
}
