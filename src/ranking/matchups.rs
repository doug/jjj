use std::collections::{HashMap, HashSet};

use crate::ranking::glicko2::Rating;

/// Suggests optimal matchup pairs for a ranking session.
///
/// Pairs are scored by an information-gain heuristic that favours comparisons
/// between items with high rating uncertainty (phi) and similar ratings (mu).
///
/// - Pairs that appear in `recent_pairs` (in either order) are excluded.
/// - Returns at most `count` pairs, sorted by descending information gain.
pub fn suggest_matchups(
    ratings: &HashMap<String, Rating>,
    recent_pairs: &[(String, String)],
    count: usize,
) -> Vec<(String, String)> {
    if ratings.len() < 2 {
        return Vec::new();
    }

    // Build a set of recently-compared pairs (both orderings).
    let mut excluded: HashSet<(&str, &str)> = HashSet::new();
    for (a, b) in recent_pairs {
        excluded.insert((a.as_str(), b.as_str()));
        excluded.insert((b.as_str(), a.as_str()));
    }

    // Collect keys so we can iterate pairs.
    let keys: Vec<&String> = ratings.keys().collect();

    // Generate all unique pairs, score them, and filter out recent ones.
    let mut scored: Vec<(f64, &String, &String)> = Vec::new();
    for i in 0..keys.len() {
        for j in (i + 1)..keys.len() {
            let a = keys[i];
            let b = keys[j];

            if excluded.contains(&(a.as_str(), b.as_str())) {
                continue;
            }

            let ra = &ratings[a];
            let rb = &ratings[b];

            let uncertainty = ra.phi + rb.phi;
            let closeness = 1.0 / (1.0 + (ra.mu - rb.mu).abs() / 100.0);
            let score = uncertainty * closeness;

            scored.push((score, a, b));
        }
    }

    // Sort by score descending.
    scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

    scored
        .into_iter()
        .take(count)
        .map(|(_, a, b)| (a.clone(), b.clone()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Items with high uncertainty (phi=350) should be preferred over items
    /// with low uncertainty (phi=30), even if mu is the same.
    #[test]
    fn test_suggest_prefers_high_uncertainty() {
        let mut ratings = HashMap::new();
        // High uncertainty pair
        ratings.insert(
            "A".to_string(),
            Rating {
                mu: 1500.0,
                phi: 350.0,
                sigma: 0.06,
            },
        );
        ratings.insert(
            "B".to_string(),
            Rating {
                mu: 1500.0,
                phi: 350.0,
                sigma: 0.06,
            },
        );
        // Low uncertainty pair
        ratings.insert(
            "C".to_string(),
            Rating {
                mu: 1500.0,
                phi: 30.0,
                sigma: 0.06,
            },
        );
        ratings.insert(
            "D".to_string(),
            Rating {
                mu: 1500.0,
                phi: 30.0,
                sigma: 0.06,
            },
        );

        let results = suggest_matchups(&ratings, &[], 1);
        assert_eq!(results.len(), 1);
        let (ref a, ref b) = results[0];

        // The top pair must be the high-uncertainty one: {A, B}.
        let pair: HashSet<&str> = [a.as_str(), b.as_str()].into_iter().collect();
        assert!(
            pair.contains("A") && pair.contains("B"),
            "Expected high-uncertainty pair (A, B), got ({a}, {b})"
        );
    }

    /// Pairs listed in `recent_pairs` (in either ordering) must not appear.
    #[test]
    fn test_suggest_avoids_recent_pairs() {
        let mut ratings = HashMap::new();
        ratings.insert("A".to_string(), Rating::default());
        ratings.insert("B".to_string(), Rating::default());
        ratings.insert("C".to_string(), Rating::default());

        // Exclude A-B (given as B-A to test order independence).
        let recent = vec![("B".to_string(), "A".to_string())];
        let results = suggest_matchups(&ratings, &recent, 10);

        for (a, b) in &results {
            let pair: HashSet<&str> = [a.as_str(), b.as_str()].into_iter().collect();
            assert!(
                !(pair.contains("A") && pair.contains("B")),
                "Pair (A, B) should have been excluded but appeared as ({a}, {b})"
            );
        }
        // We should still get the remaining pairs.
        assert!(!results.is_empty());
    }

    /// With 6 items (15 possible pairs), requesting 5 should yield exactly 5.
    #[test]
    fn test_suggest_returns_requested_count() {
        let mut ratings = HashMap::new();
        for name in ["A", "B", "C", "D", "E", "F"] {
            ratings.insert(name.to_string(), Rating::default());
        }

        let results = suggest_matchups(&ratings, &[], 5);
        assert_eq!(results.len(), 5, "Expected 5 pairs, got {}", results.len());
    }

    /// With only 2 items (1 possible pair), requesting 5 should yield 1.
    #[test]
    fn test_suggest_with_fewer_items_than_requested() {
        let mut ratings = HashMap::new();
        ratings.insert("X".to_string(), Rating::default());
        ratings.insert("Y".to_string(), Rating::default());

        let results = suggest_matchups(&ratings, &[], 5);
        assert_eq!(
            results.len(),
            1,
            "Expected 1 pair from 2 items, got {}",
            results.len()
        );
    }
}
