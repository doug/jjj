//! One query grammar for every entity kind.
//!
//! jjj grew a filter flag at a time, per command, and they drifted: `--tag` and
//! `--sort` exist on `problem list` and `solution list` but not on `critique` or
//! `finding`; `--author` exists on `critique` and `finding` but those two use it
//! where the others use `--assignee`; and `milestone list` takes no filters at
//! all. Answering "open problems tagged perf, most recently edited first" took
//! several calls and some jq.
//!
//! One query string covers all of it instead — `status:open sort:edit`. One
//! grammar, one place to learn it, and it spans kinds that have no equivalent
//! flags today.
//!
//! # Grammar
//!
//! Whitespace-separated terms. A term is either `key:value` or a bare word.
//!
//! | Term | Meaning |
//! |---|---|
//! | `type:problem` | restrict to a kind (repeatable, or comma-separated) |
//! | `status:open` | match status, case-insensitive |
//! | `tag:perf` | carries this tag |
//! | `author:ana` | authored by (substring, `@` optional) |
//! | `assignee:ana` | assigned to (substring) |
//! | `mine` | shorthand for the current actor |
//! | `problem:<ref>` | belongs to this problem |
//! | `sort:edit` | order by clock, `created`, `title`, or `updated` |
//! | `limit:20` | cap the result count |
//! | bare word | case-insensitive substring of the title |
//!
//! Repeating a key ORs its values (`status:open status:in_progress`); different
//! keys AND together. That is the behaviour people expect from this shape of
//! query and it keeps the common case short.
//!
//! `sort:edit` sorts by the Lamport clock, which is why it means something: it is
//! a causal order rather than a wall-clock one, so "most recently edited" does
//! not depend on whose machine had the fastest clock.

use std::collections::BTreeSet;

/// What to order results by.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SortKey {
    /// Lamport clock — causal edit order. See the module docs.
    Edit,
    /// Creation time.
    #[default]
    Created,
    /// Title, lexicographic.
    Title,
    /// Wall-clock `updated_at`. Present because it is what a reader sees in the
    /// file; `Edit` is the one to trust when clocks disagree.
    Updated,
}

impl std::str::FromStr for SortKey {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_ascii_lowercase().as_str() {
            "edit" => Ok(SortKey::Edit),
            "created" => Ok(SortKey::Created),
            "title" => Ok(SortKey::Title),
            "updated" => Ok(SortKey::Updated),
            other => Err(format!(
                "unknown sort key '{other}' — use edit, created, title or updated"
            )),
        }
    }
}

/// A parsed query.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Query {
    /// Kinds to include; empty means every kind.
    pub kinds: BTreeSet<String>,
    /// Statuses to match; empty means any.
    pub statuses: BTreeSet<String>,
    /// Tags that must be present (any of).
    pub tags: BTreeSet<String>,
    /// Author substrings (any of).
    pub authors: BTreeSet<String>,
    /// Assignee substrings (any of).
    pub assignees: BTreeSet<String>,
    /// Parent problem references (any of).
    pub problems: BTreeSet<String>,
    /// Bare words, all of which must appear in the title.
    pub words: Vec<String>,
    /// Order to return results in.
    pub sort: SortKey,
    /// Reverse the sort.
    pub reverse: bool,
    /// Maximum results, if capped.
    pub limit: Option<usize>,
    /// Whether `mine` was given — resolved to the current actor by the caller,
    /// since the query layer has no business reading identity.
    pub mine: bool,
}

impl Query {
    /// Parse a query string.
    ///
    /// Unknown keys are an error rather than being treated as free text. A
    /// misspelled `statuss:open` that silently became a title search would
    /// return a confidently wrong empty result, and a query language whose typos
    /// are indistinguishable from valid input is worse than flags.
    pub fn parse(input: &str) -> Result<Self, String> {
        let mut q = Query::default();
        let mut saw_sort = false;

        for term in input.split_whitespace() {
            if term.is_empty() {
                continue;
            }
            // `mine` is the one bare keyword; everything else bare is free text.
            if term.eq_ignore_ascii_case("mine") {
                q.mine = true;
                continue;
            }
            let Some((key, value)) = term.split_once(':') else {
                q.words.push(term.to_ascii_lowercase());
                continue;
            };
            let value = value.trim();
            if value.is_empty() {
                return Err(format!("'{key}:' has no value"));
            }
            // Comma-separated values are the same as repeating the key.
            let values = || value.split(',').filter(|v| !v.trim().is_empty());

            match key.to_ascii_lowercase().as_str() {
                "type" | "kind" => {
                    for v in values() {
                        let singular = normalize_kind(v.trim())?;
                        q.kinds.insert(singular);
                    }
                }
                "status" => {
                    for v in values() {
                        q.statuses.insert(v.trim().to_ascii_lowercase());
                    }
                }
                "tag" | "label" => {
                    for v in values() {
                        q.tags.insert(v.trim().to_ascii_lowercase());
                    }
                }
                "author" | "by" => {
                    for v in values() {
                        q.authors
                            .insert(v.trim().trim_start_matches('@').to_ascii_lowercase());
                    }
                }
                "assignee" | "owner" => {
                    for v in values() {
                        q.assignees
                            .insert(v.trim().trim_start_matches('@').to_ascii_lowercase());
                    }
                }
                "problem" => {
                    for v in values() {
                        q.problems.insert(v.trim().to_string());
                    }
                }
                "sort" => {
                    if saw_sort {
                        return Err("sort given more than once".to_string());
                    }
                    saw_sort = true;
                    let raw = value.trim();
                    // A leading `-` reverses, the convention every other CLI uses.
                    let (raw, reverse) = match raw.strip_prefix('-') {
                        Some(rest) => (rest, true),
                        None => (raw, false),
                    };
                    q.sort = raw.parse()?;
                    q.reverse = reverse;
                }
                "limit" => {
                    q.limit = Some(
                        value
                            .trim()
                            .parse::<usize>()
                            .map_err(|_| format!("limit must be a number, got '{value}'"))?,
                    );
                }
                other => {
                    return Err(format!(
                        "unknown term '{other}:' — valid keys are type, status, tag, author, \
                         assignee, problem, sort, limit (plus the bare word `mine`)"
                    ))
                }
            }
        }
        Ok(q)
    }

    /// Whether this kind should be included.
    pub fn wants_kind(&self, singular: &str) -> bool {
        self.kinds.is_empty() || self.kinds.contains(singular)
    }

    /// Whether a status passes the filter.
    pub fn wants_status(&self, status: &str) -> bool {
        self.statuses.is_empty() || self.statuses.contains(&status.to_ascii_lowercase())
    }

    /// Whether the tag set passes. Any matching tag is enough.
    pub fn wants_tags(&self, tags: &[String]) -> bool {
        self.tags.is_empty()
            || tags
                .iter()
                .any(|t| self.tags.contains(&t.to_ascii_lowercase()))
    }

    /// Whether a title contains every bare word.
    pub fn wants_title(&self, title: &str) -> bool {
        let lower = title.to_ascii_lowercase();
        self.words.iter().all(|w| lower.contains(w))
    }

    /// Whether an optional person field matches any of `set` (substring).
    ///
    /// An empty filter passes. A filter against an absent field fails — asking
    /// for `author:ana` should not return entities with no author.
    pub fn wants_person(set: &BTreeSet<String>, value: Option<&str>) -> bool {
        if set.is_empty() {
            return true;
        }
        match value {
            None => false,
            Some(v) => {
                let v = v.trim_start_matches('@').to_ascii_lowercase();
                set.iter().any(|want| v.contains(want))
            }
        }
    }
}

/// Map a user-supplied kind word to jjj's singular name.
///
/// Accepts the plural and the single-letter prefix, because those are what
/// people type and what jjj's own listings print (`p/`, `s/`, `c/`).
fn normalize_kind(word: &str) -> Result<String, String> {
    let w = word.to_ascii_lowercase();
    let singular = match w.as_str() {
        "problem" | "problems" | "p" => "problem",
        "solution" | "solutions" | "s" => "solution",
        "critique" | "critiques" | "c" => "critique",
        "milestone" | "milestones" | "m" => "milestone",
        "finding" | "findings" | "f" => "finding",
        other => {
            return Err(format!(
                "unknown type '{other}' — use problem, solution, critique, milestone or finding"
            ))
        }
    };
    Ok(singular.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn q(s: &str) -> Query {
        Query::parse(s).expect("should parse")
    }

    #[test]
    fn an_empty_query_matches_everything() {
        let q = q("");
        assert!(q.wants_kind("problem"));
        assert!(q.wants_status("anything"));
        assert!(q.wants_tags(&[]));
        assert!(q.wants_title("any title"));
        assert_eq!(q.limit, None);
    }

    #[test]
    fn keys_and_bare_words_combine() {
        let parsed = q("status:open tag:perf decode slow");
        assert!(parsed.statuses.contains("open"));
        assert!(parsed.tags.contains("perf"));
        assert_eq!(parsed.words, vec!["decode", "slow"]);
        assert!(parsed.wants_title("Decode is slow"));
        assert!(
            !parsed.wants_title("Decode is fine"),
            "all words must match"
        );
    }

    #[test]
    fn repeating_a_key_ors_its_values() {
        // Different keys AND; the same key ORs. "open or in_progress" is the
        // common case and should stay short.
        let parsed = q("status:open status:in_progress");
        assert!(parsed.wants_status("open"));
        assert!(parsed.wants_status("in_progress"));
        assert!(!parsed.wants_status("solved"));

        // Commas are the same thing.
        let comma = q("status:open,in_progress");
        assert_eq!(comma.statuses, parsed.statuses);
    }

    #[test]
    fn kinds_accept_plural_and_prefix_forms() {
        for form in ["type:problem", "type:problems", "type:p", "kind:P"] {
            assert!(q(form).wants_kind("problem"), "{form} did not match");
        }
        let multi = q("type:solution,critique");
        assert!(multi.wants_kind("solution") && multi.wants_kind("critique"));
        assert!(!multi.wants_kind("problem"));
    }

    #[test]
    fn sort_defaults_and_reverses() {
        assert_eq!(q("").sort, SortKey::Created);
        assert!(!q("").reverse);
        assert_eq!(q("sort:edit").sort, SortKey::Edit);
        let rev = q("sort:-title");
        assert_eq!(rev.sort, SortKey::Title);
        assert!(rev.reverse);
    }

    #[test]
    fn people_matching_is_substring_and_at_optional() {
        let parsed = q("author:@ana");
        assert!(parsed.authors.contains("ana"));
        assert!(Query::wants_person(&parsed.authors, Some("ana-1")));
        assert!(Query::wants_person(&parsed.authors, Some("@ana")));
        assert!(!Query::wants_person(&parsed.authors, Some("bo")));
    }

    #[test]
    fn a_person_filter_excludes_entities_with_nobody() {
        // Asking for author:ana must not return unattributed entities.
        let parsed = q("author:ana");
        assert!(!Query::wants_person(&parsed.authors, None));
        // And no filter passes them through.
        assert!(Query::wants_person(&Query::default().authors, None));
    }

    #[test]
    fn mine_is_a_bare_keyword_not_free_text() {
        let parsed = q("mine status:open");
        assert!(parsed.mine);
        assert!(
            parsed.words.is_empty(),
            "`mine` leaked into the title search: {:?}",
            parsed.words
        );
    }

    #[test]
    fn a_misspelled_key_is_an_error_not_a_title_search() {
        // The failure this guards against: `statuss:open` silently becoming a
        // title search returns a confidently wrong empty list, and a query
        // language whose typos look like valid input is worse than flags.
        let err = Query::parse("statuss:open").expect_err("should reject");
        assert!(err.contains("unknown term"), "unhelpful error: {err}");
        assert!(err.contains("status"), "should name the valid keys: {err}");
    }

    #[test]
    fn malformed_terms_are_rejected_with_reasons() {
        for (input, expect) in [
            ("status:", "no value"),
            ("limit:many", "must be a number"),
            ("sort:sideways", "unknown sort key"),
            ("sort:edit sort:created", "more than once"),
            ("type:bug", "unknown type"),
        ] {
            let err = Query::parse(input).expect_err(input);
            assert!(
                err.contains(expect),
                "{input}: expected an error mentioning {expect:?}, got {err:?}"
            );
        }
    }

    #[test]
    fn tags_and_status_are_case_insensitive() {
        let parsed = q("status:OPEN tag:Perf");
        assert!(parsed.wants_status("open"));
        assert!(parsed.wants_tags(&["PERF".to_string()]));
    }
}
