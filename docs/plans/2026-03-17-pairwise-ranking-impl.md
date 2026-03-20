# Pairwise Ranking Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use executing-plans to implement this plan task-by-task.

**Goal:** Add a Glicko-2 pairwise ranking system for problems within milestones, replacing static priority tiers.

**Architecture:** New `src/ranking/` module for Glicko-2 algorithm + comparison storage. New `src/commands/rank.rs` for CLI. Rankings stored as per-user JSONL files in the shadow graph (`rankings/{milestone_id}/{user}.jsonl`). Ratings computed on-the-fly from comparison history.

**Tech Stack:** Rust, serde/serde_json for JSONL, chrono for timestamps, crossterm for interactive CLI session, ratatui for TUI integration.

---

### Task 1: Glicko-2 Algorithm — Core Types and Constants

**Files:**
- Create: `src/ranking/mod.rs`
- Create: `src/ranking/glicko2.rs`
- Modify: `src/lib.rs:26` (add `pub mod ranking;`)

**Step 1: Write the failing test**

Add to `src/ranking/glicko2.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_rating() {
        let r = Rating::default();
        assert_eq!(r.mu, 1500.0);
        assert!((r.phi - 350.0).abs() < 0.01);
        assert!((r.sigma - 0.06).abs() < 0.001);
    }

    #[test]
    fn test_confidence_high() {
        let r = Rating { mu: 1600.0, phi: 30.0, sigma: 0.06 };
        assert_eq!(r.confidence(), Confidence::High);
    }

    #[test]
    fn test_confidence_low() {
        let r = Rating::default(); // phi=350 -> Low
        assert_eq!(r.confidence(), Confidence::Low);
    }
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test ranking::glicko2 2>&1 | head -20`
Expected: Compilation error — module doesn't exist

**Step 3: Write minimal implementation**

Create `src/ranking/mod.rs`:
```rust
pub mod glicko2;
```

Create `src/ranking/glicko2.rs`:
```rust
use serde::{Deserialize, Serialize};

/// Glicko-2 system constants
const TAU: f64 = 0.5; // System volatility constraint
const EPSILON: f64 = 0.000001; // Convergence tolerance
const SCALE: f64 = 173.7178; // Glicko-2 scaling factor (400 / ln(10))

/// A Glicko-2 rating triplet.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Rating {
    /// Rating (μ) — default 1500
    pub mu: f64,
    /// Rating deviation (φ) — default 350 (high uncertainty)
    pub phi: f64,
    /// Volatility (σ) — default 0.06
    pub sigma: f64,
}

impl Default for Rating {
    fn default() -> Self {
        Self {
            mu: 1500.0,
            phi: 350.0,
            sigma: 0.06,
        }
    }
}

/// Confidence level derived from rating deviation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Confidence {
    High,
    Medium,
    Low,
}

impl std::fmt::Display for Confidence {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Confidence::High => write!(f, "high"),
            Confidence::Medium => write!(f, "med"),
            Confidence::Low => write!(f, "low"),
        }
    }
}

impl Rating {
    /// Confidence level based on rating deviation.
    /// High: φ < 50, Medium: φ < 150, Low: φ >= 150
    pub fn confidence(&self) -> Confidence {
        if self.phi < 50.0 {
            Confidence::High
        } else if self.phi < 150.0 {
            Confidence::Medium
        } else {
            Confidence::Low
        }
    }
}
```

Add to `src/lib.rs` after the last `pub mod` line:
```rust
pub mod ranking;
```

**Step 4: Run test to verify it passes**

Run: `cargo test ranking::glicko2 --lib -- --nocapture 2>&1 | tail -10`
Expected: 3 tests PASS

**Step 5: Commit**

```bash
git add src/ranking/mod.rs src/ranking/glicko2.rs src/lib.rs
git commit -m "feat(ranking): add Glicko-2 core types and constants"
```

---

### Task 2: Glicko-2 Algorithm — Rating Update

**Files:**
- Modify: `src/ranking/glicko2.rs`

**Step 1: Write the failing test**

Add to the `tests` module in `src/ranking/glicko2.rs`:

```rust
    #[test]
    fn test_update_winner_rating_increases() {
        let mut winner = Rating::default();
        let mut loser = Rating::default();
        update_ratings(&mut winner, &mut loser, 1.0);
        assert!(winner.mu > 1500.0, "Winner rating should increase");
        assert!(loser.mu < 1500.0, "Loser rating should decrease");
    }

    #[test]
    fn test_update_symmetric() {
        let mut w = Rating::default();
        let mut l = Rating::default();
        update_ratings(&mut w, &mut l, 1.0);
        // Winner gain ≈ loser loss (symmetric when starting equal)
        let gain = w.mu - 1500.0;
        let loss = 1500.0 - l.mu;
        assert!((gain - loss).abs() < 1.0, "Gains and losses should be roughly symmetric");
    }

    #[test]
    fn test_update_reduces_deviation() {
        let mut w = Rating::default();
        let mut l = Rating::default();
        update_ratings(&mut w, &mut l, 1.0);
        assert!(w.phi < 350.0, "Winner deviation should decrease after comparison");
        assert!(l.phi < 350.0, "Loser deviation should decrease after comparison");
    }

    #[test]
    fn test_update_with_weight() {
        let mut w1 = Rating::default();
        let mut l1 = Rating::default();
        update_ratings(&mut w1, &mut l1, 1.0);

        let mut w2 = Rating::default();
        let mut l2 = Rating::default();
        update_ratings(&mut w2, &mut l2, 2.0);

        // Higher weight should produce larger rating change
        assert!((w2.mu - 1500.0) > (w1.mu - 1500.0));
    }
```

**Step 2: Run test to verify it fails**

Run: `cargo test ranking::glicko2 --lib 2>&1 | head -10`
Expected: FAIL — `update_ratings` not found

**Step 3: Write minimal implementation**

Add to `src/ranking/glicko2.rs` (above the `tests` module):

```rust
/// Convert from Glicko to Glicko-2 scale
fn to_glicko2(mu: f64, phi: f64) -> (f64, f64) {
    ((mu - 1500.0) / SCALE, phi / SCALE)
}

/// Convert from Glicko-2 back to Glicko scale
fn from_glicko2(mu2: f64, phi2: f64) -> (f64, f64) {
    (mu2 * SCALE + 1500.0, phi2 * SCALE)
}

/// The g(φ) function from Glicko-2
fn g(phi: f64) -> f64 {
    1.0 / (1.0 + 3.0 * phi * phi / (std::f64::consts::PI * std::f64::consts::PI)).sqrt()
}

/// Expected score E(μ, μ_j, φ_j)
fn expected(mu: f64, mu_j: f64, phi_j: f64) -> f64 {
    1.0 / (1.0 + (-g(phi_j) * (mu - mu_j)).exp())
}

/// Compute new volatility using the Illinois algorithm (Glicko-2 Step 5).
fn new_volatility(sigma: f64, phi: f64, v: f64, delta: f64) -> f64 {
    let a = (sigma * sigma).ln();
    let delta_sq = delta * delta;
    let phi_sq = phi * phi;

    let f = |x: f64| -> f64 {
        let ex = x.exp();
        let d = phi_sq + v + ex;
        (ex * (delta_sq - phi_sq - v - ex)) / (2.0 * d * d) - (x - a) / (TAU * TAU)
    };

    // Step 5.2: Initial bounds
    let mut big_a = a;
    let mut big_b = if delta_sq > phi_sq + v {
        (delta_sq - phi_sq - v).ln()
    } else {
        let mut k = 1.0_f64;
        while f(a - k * TAU) < 0.0 {
            k += 1.0;
        }
        a - k * TAU
    };

    // Step 5.3: Illinois algorithm iteration
    let mut fa = f(big_a);
    let mut fb = f(big_b);
    while (big_b - big_a).abs() > EPSILON {
        let big_c = big_a + (big_a - big_b) * fa / (fb - fa);
        let fc = f(big_c);
        if fc * fb <= 0.0 {
            big_a = big_b;
            fa = fb;
        } else {
            fa /= 2.0;
        }
        big_b = big_c;
        fb = fc;
    }

    (big_a / 2.0).exp()
}

/// Update two ratings after a comparison where `winner` beat `loser`.
///
/// `weight` scales the comparison's impact (1.0 = normal, 2.0 = double weight).
/// Implements the full Glicko-2 update for both players.
pub fn update_ratings(winner: &mut Rating, loser: &mut Rating, weight: f64) {
    // Convert to Glicko-2 scale
    let (mu_w, phi_w) = to_glicko2(winner.mu, winner.phi);
    let (mu_l, phi_l) = to_glicko2(loser.mu, loser.phi);

    // Update winner (score = 1.0)
    let (new_mu_w, new_phi_w, new_sigma_w) =
        update_single(mu_w, phi_w, winner.sigma, mu_l, phi_l, 1.0, weight);
    // Update loser (score = 0.0)
    let (new_mu_l, new_phi_l, new_sigma_l) =
        update_single(mu_l, phi_l, loser.sigma, mu_w, phi_w, 0.0, weight);

    // Convert back
    let (mu_w_out, phi_w_out) = from_glicko2(new_mu_w, new_phi_w);
    let (mu_l_out, phi_l_out) = from_glicko2(new_mu_l, new_phi_l);

    winner.mu = mu_w_out;
    winner.phi = phi_w_out;
    winner.sigma = new_sigma_w;

    loser.mu = mu_l_out;
    loser.phi = phi_l_out;
    loser.sigma = new_sigma_l;
}

/// Single-player Glicko-2 update against one opponent.
fn update_single(
    mu: f64,
    phi: f64,
    sigma: f64,
    mu_j: f64,
    phi_j: f64,
    score: f64,
    weight: f64,
) -> (f64, f64, f64) {
    let g_j = g(phi_j);
    let e_j = expected(mu, mu_j, phi_j);

    // Step 3: Estimated variance
    let v = weight / (g_j * g_j * e_j * (1.0 - e_j));

    // Step 4: Estimated improvement
    let delta = v * g_j * (score - e_j) * weight;

    // Step 5: New volatility
    let sigma_new = new_volatility(sigma, phi, v, delta);

    // Step 6: Update phi to pre-rating period value
    let phi_star = (phi * phi + sigma_new * sigma_new).sqrt();

    // Step 7: Update rating and deviation
    let phi_new = 1.0 / (1.0 / (phi_star * phi_star) + 1.0 / v).sqrt();
    let mu_new = mu + phi_new * phi_new * g_j * (score - e_j) * weight;

    (mu_new, phi_new, sigma_new)
}
```

**Step 4: Run test to verify it passes**

Run: `cargo test ranking::glicko2 --lib -- --nocapture 2>&1 | tail -15`
Expected: 7 tests PASS (3 from Task 1 + 4 new)

**Step 5: Commit**

```bash
git add src/ranking/glicko2.rs
git commit -m "feat(ranking): implement Glicko-2 rating update algorithm"
```

---

### Task 3: Glicko-2 Algorithm — Batch Compute Ratings

**Files:**
- Modify: `src/ranking/glicko2.rs`

**Step 1: Write the failing test**

```rust
    #[test]
    fn test_compute_ratings_empty() {
        let ratings = compute_ratings(&[], &HashMap::new());
        assert!(ratings.is_empty());
    }

    #[test]
    fn test_compute_ratings_basic() {
        use std::collections::HashMap;
        let comparisons = vec![
            Comparison { winner: "A".into(), loser: "B".into(), ts: Utc::now(), },
            Comparison { winner: "A".into(), loser: "C".into(), ts: Utc::now(), },
            Comparison { winner: "B".into(), loser: "C".into(), ts: Utc::now(), },
        ];
        let ratings = compute_ratings(&comparisons, &HashMap::new());
        assert!(ratings["A"].mu > ratings["B"].mu);
        assert!(ratings["B"].mu > ratings["C"].mu);
    }

    #[test]
    fn test_compute_ratings_weighted_user() {
        use std::collections::HashMap;
        // Owner says B > A, regular user says A > B
        let comparisons = vec![
            Comparison { winner: "B".into(), loser: "A".into(), ts: Utc::now() },
            Comparison { winner: "A".into(), loser: "B".into(), ts: Utc::now() },
        ];
        // First comparison from owner (weight 2), second from regular (weight 1)
        let mut weights = HashMap::new();
        weights.insert(0usize, 2.0_f64);
        // Actually, we need to rethink: weights are per-user, not per-comparison.
        // Let's use a different API: compute_ratings takes comparisons with user info.
    }
```

Wait — I need to rethink the API. Comparisons need user attribution for weighting. Let me revise.

**Step 1 (revised): Write the failing test**

```rust
    #[test]
    fn test_compute_ratings_empty() {
        let ratings = compute_ratings(&[]);
        assert!(ratings.is_empty());
    }

    #[test]
    fn test_compute_ratings_total_ordering() {
        let comparisons = vec![
            WeightedComparison { winner: "A".into(), loser: "B".into(), weight: 1.0 },
            WeightedComparison { winner: "A".into(), loser: "C".into(), weight: 1.0 },
            WeightedComparison { winner: "B".into(), loser: "C".into(), weight: 1.0 },
        ];
        let ratings = compute_ratings(&comparisons);
        assert!(ratings["A"].mu > ratings["B"].mu);
        assert!(ratings["B"].mu > ratings["C"].mu);
    }

    #[test]
    fn test_compute_ratings_owner_weight_wins() {
        // Owner (weight=2) says B > A; regular (weight=1) says A > B
        let comparisons = vec![
            WeightedComparison { winner: "B".into(), loser: "A".into(), weight: 2.0 },
            WeightedComparison { winner: "A".into(), loser: "B".into(), weight: 1.0 },
        ];
        let ratings = compute_ratings(&comparisons);
        assert!(ratings["B"].mu > ratings["A"].mu, "Owner's heavier weight should dominate");
    }
```

**Step 2: Run test to verify it fails**

Run: `cargo test ranking::glicko2 --lib 2>&1 | head -10`
Expected: FAIL — `WeightedComparison` and `compute_ratings` not found

**Step 3: Write minimal implementation**

Add to `src/ranking/glicko2.rs`:

```rust
use std::collections::HashMap;
use chrono::{DateTime, Utc};

/// A single pairwise comparison as stored on disk.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Comparison {
    pub winner: String,
    pub loser: String,
    pub ts: DateTime<Utc>,
}

/// A comparison with its weight applied (after resolving user → weight mapping).
#[derive(Debug, Clone)]
pub struct WeightedComparison {
    pub winner: String,
    pub loser: String,
    pub weight: f64,
}

/// Compute Glicko-2 ratings from a sequence of weighted comparisons.
///
/// All items start at the default rating (1500, φ=350, σ=0.06).
/// Comparisons are processed in order; each updates both winner and loser.
/// Returns a map of item ID → final Rating.
pub fn compute_ratings(comparisons: &[WeightedComparison]) -> HashMap<String, Rating> {
    let mut ratings: HashMap<String, Rating> = HashMap::new();

    for cmp in comparisons {
        // Ensure both items exist with default ratings
        ratings.entry(cmp.winner.clone()).or_default();
        ratings.entry(cmp.loser.clone()).or_default();

        // Clone, update, write back (avoid double borrow)
        let mut winner_rating = ratings[&cmp.winner].clone();
        let mut loser_rating = ratings[&cmp.loser].clone();
        update_ratings(&mut winner_rating, &mut loser_rating, cmp.weight);
        ratings.insert(cmp.winner.clone(), winner_rating);
        ratings.insert(cmp.loser.clone(), loser_rating);
    }

    ratings
}

/// Produce a sorted ranking from computed ratings (highest first).
pub fn sorted_ranking(ratings: &HashMap<String, Rating>) -> Vec<(String, Rating)> {
    let mut items: Vec<_> = ratings
        .iter()
        .map(|(id, r)| (id.clone(), r.clone()))
        .collect();
    items.sort_by(|a, b| b.1.mu.partial_cmp(&a.1.mu).unwrap_or(std::cmp::Ordering::Equal));
    items
}
```

**Step 4: Run test to verify it passes**

Run: `cargo test ranking::glicko2 --lib -- --nocapture 2>&1 | tail -15`
Expected: All tests PASS

**Step 5: Commit**

```bash
git add src/ranking/glicko2.rs
git commit -m "feat(ranking): add batch rating computation and sorted ranking"
```

---

### Task 4: Comparison Storage — Read/Write JSONL

**Files:**
- Create: `src/ranking/store.rs`
- Modify: `src/ranking/mod.rs` (add `pub mod store;`)
- Modify: `src/storage/mod.rs` (add `RANKINGS_DIR` constant and ensure dir creation in `init()`)

**Step 1: Write the failing test**

Add to `src/ranking/store.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_append_and_load_comparisons() {
        let dir = TempDir::new().unwrap();
        let milestone_id = "m-123";
        let user = "alice";

        let cmp = Comparison {
            winner: "P-1".into(),
            loser: "P-2".into(),
            ts: Utc::now(),
        };

        append_comparison(dir.path(), milestone_id, user, &cmp).unwrap();
        append_comparison(dir.path(), milestone_id, user, &Comparison {
            winner: "P-3".into(),
            loser: "P-1".into(),
            ts: Utc::now(),
        }).unwrap();

        let loaded = load_comparisons(dir.path(), milestone_id).unwrap();
        assert_eq!(loaded.len(), 2);
        assert_eq!(loaded[0].winner, "P-1");
        assert_eq!(loaded[1].winner, "P-3");
    }

    #[test]
    fn test_load_comparisons_empty() {
        let dir = TempDir::new().unwrap();
        let loaded = load_comparisons(dir.path(), "nonexistent").unwrap();
        assert!(loaded.is_empty());
    }

    #[test]
    fn test_load_comparisons_multiple_users() {
        let dir = TempDir::new().unwrap();
        let milestone_id = "m-123";

        append_comparison(dir.path(), milestone_id, "alice", &Comparison {
            winner: "A".into(), loser: "B".into(), ts: Utc::now(),
        }).unwrap();
        append_comparison(dir.path(), milestone_id, "bob", &Comparison {
            winner: "B".into(), loser: "A".into(), ts: Utc::now(),
        }).unwrap();

        let loaded = load_comparisons(dir.path(), milestone_id).unwrap();
        assert_eq!(loaded.len(), 2);
    }

    #[test]
    fn test_load_attributed_comparisons() {
        let dir = TempDir::new().unwrap();
        let milestone_id = "m-123";

        append_comparison(dir.path(), milestone_id, "alice", &Comparison {
            winner: "A".into(), loser: "B".into(), ts: Utc::now(),
        }).unwrap();
        append_comparison(dir.path(), milestone_id, "bob", &Comparison {
            winner: "B".into(), loser: "A".into(), ts: Utc::now(),
        }).unwrap();

        let loaded = load_attributed_comparisons(dir.path(), milestone_id).unwrap();
        assert_eq!(loaded.len(), 2);
        assert_eq!(loaded[0].1, "alice");
        assert_eq!(loaded[1].1, "bob");
    }
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test ranking::store --lib 2>&1 | head -10`
Expected: FAIL — module doesn't exist

**Step 3: Write minimal implementation**

Create `src/ranking/store.rs`:

```rust
use crate::ranking::glicko2::Comparison;
use std::fs;
use std::io::Write;
use std::path::Path;
use chrono::Utc;

const RANKINGS_DIR: &str = "rankings";

/// Append a comparison to the user's JSONL file for a milestone.
///
/// Creates directories as needed. File path:
/// `{base}/rankings/{milestone_id}/{user_slug}.jsonl`
pub fn append_comparison(
    base: &Path,
    milestone_id: &str,
    user: &str,
    comparison: &Comparison,
) -> crate::error::Result<()> {
    let dir = base.join(RANKINGS_DIR).join(milestone_id);
    fs::create_dir_all(&dir)?;

    let file_path = dir.join(format!("{}.jsonl", sanitize_user(user)));
    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&file_path)?;

    let line = serde_json::to_string(comparison)?;
    writeln!(file, "{}", line)?;

    Ok(())
}

/// Load all comparisons for a milestone from all user files.
///
/// Returns comparisons in chronological order (sorted by timestamp).
pub fn load_comparisons(
    base: &Path,
    milestone_id: &str,
) -> crate::error::Result<Vec<Comparison>> {
    let attributed = load_attributed_comparisons(base, milestone_id)?;
    Ok(attributed.into_iter().map(|(c, _)| c).collect())
}

/// Load all comparisons with their user attribution.
///
/// Returns `(Comparison, user_slug)` pairs sorted by timestamp.
pub fn load_attributed_comparisons(
    base: &Path,
    milestone_id: &str,
) -> crate::error::Result<Vec<(Comparison, String)>> {
    let dir = base.join(RANKINGS_DIR).join(milestone_id);
    if !dir.exists() {
        return Ok(Vec::new());
    }

    let mut all: Vec<(Comparison, String)> = Vec::new();

    for entry in fs::read_dir(&dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("jsonl") {
            continue;
        }
        let user_slug = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown")
            .to_string();

        let content = fs::read_to_string(&path)?;
        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            let cmp: Comparison = serde_json::from_str(line)?;
            all.push((cmp, user_slug.clone()));
        }
    }

    all.sort_by(|a, b| a.0.ts.cmp(&b.0.ts));
    Ok(all)
}

/// Sanitize a user identity string for use as a filename.
///
/// "Alice Smith <alice@example.com>" → "alice-smith"
fn sanitize_user(user: &str) -> String {
    // Take just the name part (before <email>)
    let name = user.split('<').next().unwrap_or(user).trim();
    name.to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .trim_matches('-')
        .to_string()
}
```

Update `src/ranking/mod.rs`:
```rust
pub mod glicko2;
pub mod store;
```

**Step 4: Run test to verify it passes**

Run: `cargo test ranking::store --lib -- --nocapture 2>&1 | tail -15`
Expected: All 4 tests PASS

**Step 5: Commit**

```bash
git add src/ranking/store.rs src/ranking/mod.rs
git commit -m "feat(ranking): add JSONL comparison storage (per-user, per-milestone)"
```

---

### Task 5: Matchup Suggestion Algorithm

**Files:**
- Create: `src/ranking/matchups.rs`
- Modify: `src/ranking/mod.rs` (add `pub mod matchups;`)

**Step 1: Write the failing test**

Add to `src/ranking/matchups.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::ranking::glicko2::Rating;

    #[test]
    fn test_suggest_prefers_high_uncertainty() {
        let mut ratings = HashMap::new();
        ratings.insert("A".into(), Rating { mu: 1500.0, phi: 30.0, sigma: 0.06 });  // certain
        ratings.insert("B".into(), Rating { mu: 1500.0, phi: 350.0, sigma: 0.06 }); // uncertain
        ratings.insert("C".into(), Rating { mu: 1500.0, phi: 350.0, sigma: 0.06 }); // uncertain

        let pairs = suggest_matchups(&ratings, &[], 1);
        assert_eq!(pairs.len(), 1);
        // Should pair the two uncertain items, not the certain one
        let pair = &pairs[0];
        assert!(pair.0 != "A" || pair.1 != "A");
    }

    #[test]
    fn test_suggest_avoids_recent_pairs() {
        let mut ratings = HashMap::new();
        ratings.insert("A".into(), Rating::default());
        ratings.insert("B".into(), Rating::default());
        ratings.insert("C".into(), Rating::default());

        let recent = vec![("A".to_string(), "B".to_string())];
        let pairs = suggest_matchups(&ratings, &recent, 2);

        // Should not suggest A vs B again
        for (a, b) in &pairs {
            assert!(!(a == "A" && b == "B") && !(a == "B" && b == "A"));
        }
    }

    #[test]
    fn test_suggest_returns_requested_count() {
        let mut ratings = HashMap::new();
        for i in 0..6 {
            ratings.insert(format!("P-{}", i), Rating::default());
        }
        let pairs = suggest_matchups(&ratings, &[], 5);
        assert_eq!(pairs.len(), 5);
    }

    #[test]
    fn test_suggest_with_fewer_items_than_requested() {
        let mut ratings = HashMap::new();
        ratings.insert("A".into(), Rating::default());
        ratings.insert("B".into(), Rating::default());
        // Only 1 possible pair
        let pairs = suggest_matchups(&ratings, &[], 5);
        assert_eq!(pairs.len(), 1);
    }
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test ranking::matchups --lib 2>&1 | head -10`
Expected: FAIL — module doesn't exist

**Step 3: Write minimal implementation**

Create `src/ranking/matchups.rs`:

```rust
use crate::ranking::glicko2::Rating;
use std::collections::{HashMap, HashSet};

/// Suggest optimal matchup pairs for a ranking session.
///
/// Prioritizes pairs where:
/// 1. At least one item has high uncertainty (large φ)
/// 2. Items have similar ratings (most informative for Glicko-2)
/// 3. The pair hasn't been compared recently
///
/// Returns up to `count` pairs of (item_id_a, item_id_b).
pub fn suggest_matchups(
    ratings: &HashMap<String, Rating>,
    recent_pairs: &[(String, String)],
    count: usize,
) -> Vec<(String, String)> {
    let ids: Vec<&String> = ratings.keys().collect();
    if ids.len() < 2 {
        return Vec::new();
    }

    let recent_set: HashSet<(&str, &str)> = recent_pairs
        .iter()
        .flat_map(|(a, b)| vec![(a.as_str(), b.as_str()), (b.as_str(), a.as_str())])
        .collect();

    // Score all possible pairs
    let mut candidates: Vec<(f64, String, String)> = Vec::new();
    for i in 0..ids.len() {
        for j in (i + 1)..ids.len() {
            let a = ids[i];
            let b = ids[j];

            if recent_set.contains(&(a.as_str(), b.as_str())) {
                continue;
            }

            let ra = &ratings[a];
            let rb = &ratings[b];

            // Information gain heuristic:
            // - Higher uncertainty (phi) = more to learn
            // - Closer ratings = more informative comparison
            let uncertainty = ra.phi + rb.phi;
            let closeness = 1.0 / (1.0 + (ra.mu - rb.mu).abs() / 100.0);
            let score = uncertainty * closeness;

            candidates.push((score, a.clone(), b.clone()));
        }
    }

    // Sort by score descending (highest information gain first)
    candidates.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

    candidates
        .into_iter()
        .take(count)
        .map(|(_, a, b)| (a, b))
        .collect()
}
```

Update `src/ranking/mod.rs`:
```rust
pub mod glicko2;
pub mod matchups;
pub mod store;
```

**Step 4: Run test to verify it passes**

Run: `cargo test ranking::matchups --lib -- --nocapture 2>&1 | tail -15`
Expected: All 4 tests PASS

**Step 5: Commit**

```bash
git add src/ranking/matchups.rs src/ranking/mod.rs
git commit -m "feat(ranking): add uncertainty-driven matchup suggestion algorithm"
```

---

### Task 6: CLI — `jjj rank` Subcommand Definition

**Files:**
- Modify: `src/cli.rs` (add `Rank` variant to `Commands` enum and `RankAction` enum)
- Create: `src/commands/rank.rs`
- Modify: `src/commands/mod.rs` (add module and dispatch)

**Step 1: Add the CLI definition**

Add to `src/cli.rs` — new `RankAction` enum after the `MilestoneAction` enum:

```rust
// =============================================================================
// Rank Commands
// =============================================================================

#[derive(Subcommand)]
pub enum RankAction {
    /// Start a guided ranking session — compare problems in pairs
    #[command(display_order = 0)]
    Session {
        /// Milestone to rank problems for (ID, prefix, or title)
        milestone: Option<String>,

        /// Number of matchups to present (default: 7)
        #[arg(long, default_value = "7")]
        count: usize,
    },

    /// Show the computed ranking for a milestone
    #[command(display_order = 1)]
    Show {
        /// Milestone ID, prefix, or title
        milestone: Option<String>,

        /// Show per-user ranking breakdown
        #[arg(long)]
        by_user: bool,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Show comparison history for a milestone
    #[command(display_order = 2)]
    History {
        /// Milestone ID, prefix, or title
        milestone: Option<String>,

        /// Maximum entries to show (default: 20)
        #[arg(long, default_value = "20")]
        limit: usize,
    },
}
```

Add the `Rank` variant to the `Commands` enum (after Milestone):

```rust
    /// Rank problems by importance using pairwise comparisons (Glicko-2)
    #[command(display_order = 14)]
    Rank {
        #[command(subcommand)]
        action: RankAction,
    },
```

**Step 2: Create the command file stub**

Create `src/commands/rank.rs`:

```rust
use crate::cli::RankAction;
use crate::context::CommandContext;
use crate::error::Result;

pub fn execute(ctx: &CommandContext, action: RankAction) -> Result<()> {
    match action {
        RankAction::Session { milestone, count } => session(ctx, milestone, count),
        RankAction::Show { milestone, by_user, json } => show(ctx, milestone, by_user, json),
        RankAction::History { milestone, limit } => history(ctx, milestone, limit),
    }
}

fn session(_ctx: &CommandContext, _milestone: Option<String>, _count: usize) -> Result<()> {
    eprintln!("rank session not yet implemented");
    Ok(())
}

fn show(_ctx: &CommandContext, _milestone: Option<String>, _by_user: bool, _json: bool) -> Result<()> {
    eprintln!("rank show not yet implemented");
    Ok(())
}

fn history(_ctx: &CommandContext, _milestone: Option<String>, _limit: usize) -> Result<()> {
    eprintln!("rank history not yet implemented");
    Ok(())
}
```

**Step 3: Wire up dispatch**

Add `pub mod rank;` to `src/commands/mod.rs`.

Add dispatch in `execute_with_context()`:

```rust
        Commands::Rank { action } => rank::execute(ctx, action),
```

**Step 4: Build to verify it compiles**

Run: `cargo build 2>&1 | tail -5`
Expected: Compiles without errors

**Step 5: Verify CLI help**

Run: `cargo run -- rank --help 2>&1`
Expected: Shows `session`, `show`, `history` subcommands

**Step 6: Commit**

```bash
git add src/cli.rs src/commands/rank.rs src/commands/mod.rs
git commit -m "feat(ranking): add jjj rank CLI subcommand skeleton"
```

---

### Task 7: CLI — `jjj rank show` Implementation

**Files:**
- Modify: `src/commands/rank.rs`

**Step 1: Write the `show` function**

Replace the `show` stub in `src/commands/rank.rs`:

```rust
use crate::ranking::glicko2::{compute_ratings, sorted_ranking, WeightedComparison};
use crate::ranking::store::load_attributed_comparisons;

fn show(ctx: &CommandContext, milestone: Option<String>, by_user: bool, json: bool) -> Result<()> {
    let milestone_id = resolve_milestone_for_rank(ctx, milestone)?;
    let milestone_obj = ctx.store.load_milestone(&milestone_id)?;

    let attributed = load_attributed_comparisons(ctx.store.meta_path(), &milestone_id)?;
    if attributed.is_empty() {
        println!("No rankings yet for milestone \"{}\". Run `jjj rank session` to start.", milestone_obj.title);
        return Ok(());
    }

    // Determine owner weight
    let owner = milestone_obj.assignee.as_deref().unwrap_or("");
    let owner_weight = 2.0_f64;

    // Build weighted comparisons
    let weighted: Vec<WeightedComparison> = attributed
        .iter()
        .map(|(cmp, user)| {
            let weight = if user_matches_owner(user, owner) { owner_weight } else { 1.0 };
            WeightedComparison {
                winner: cmp.winner.clone(),
                loser: cmp.loser.clone(),
                weight,
            }
        })
        .collect();

    let user_count = {
        let mut users: std::collections::HashSet<&str> = std::collections::HashSet::new();
        for (_, u) in &attributed { users.insert(u); }
        users.len()
    };

    let ratings = compute_ratings(&weighted);
    let ranking = sorted_ranking(&ratings);

    if json {
        let json_out: Vec<serde_json::Value> = ranking
            .iter()
            .enumerate()
            .map(|(i, (id, r))| {
                serde_json::json!({
                    "rank": i + 1,
                    "problem_id": id,
                    "rating": (r.mu * 10.0).round() / 10.0,
                    "confidence": r.confidence().to_string(),
                    "deviation": (r.phi * 10.0).round() / 10.0,
                })
            })
            .collect();
        println!("{}", serde_json::to_string_pretty(&json_out)?);
        return Ok(());
    }

    // Load problem titles for display
    let problems = ctx.store.list_problems()?;
    let title_map: std::collections::HashMap<&str, &str> = problems
        .iter()
        .map(|p| (p.id.as_str(), p.title.as_str()))
        .collect();

    println!(
        "Ranking for \"{}\" ({} comparisons from {} user{})\n",
        milestone_obj.title,
        attributed.len(),
        user_count,
        if user_count == 1 { "" } else { "s" },
    );
    println!(" {:<3} {:<40} {:>6}   {:<4}   {:>5}", "#", "Problem", "Rating", "Conf", "Cmps");
    println!(" {}", "─".repeat(68));

    for (i, (id, rating)) in ranking.iter().enumerate() {
        let short_id = &id[..6.min(id.len())];
        let title = title_map.get(id.as_str()).unwrap_or(&short_id);
        let truncated = crate::utils::truncate(title, 36);
        let cmp_count = attributed.iter().filter(|(c, _)| c.winner == *id || c.loser == *id).count();

        println!(
            " {:<3} {:<40} {:>6.0}   {:<4}   {:>5}",
            i + 1,
            format!("{} ({})", truncated, short_id),
            rating.mu,
            rating.confidence(),
            cmp_count,
        );
    }

    Ok(())
}

/// Resolve a milestone from user input, or pick the first active milestone.
fn resolve_milestone_for_rank(
    ctx: &CommandContext,
    milestone: Option<String>,
) -> Result<String> {
    match milestone {
        Some(m) => ctx.resolve_milestone(&m),
        None => {
            let milestones = ctx.store.list_milestones()?;
            let active = milestones.iter().find(|m| m.is_active());
            match active {
                Some(m) => Ok(m.id.clone()),
                None => Err(crate::error::JjjError::Validation(
                    "No active milestone found. Specify one with: jjj rank show <milestone>".into(),
                )),
            }
        }
    }
}

/// Check if a sanitized user slug matches the milestone owner identity.
fn user_matches_owner(user_slug: &str, owner: &str) -> bool {
    if owner.is_empty() {
        return false;
    }
    let owner_slug = owner
        .split('<').next().unwrap_or(owner)
        .trim()
        .to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect::<String>();
    let owner_slug = owner_slug.trim_matches('-');
    user_slug == owner_slug
}
```

**Step 2: Build and test manually**

Run: `cargo build 2>&1 | tail -5`
Expected: Compiles

**Step 3: Commit**

```bash
git add src/commands/rank.rs
git commit -m "feat(ranking): implement jjj rank show with Glicko-2 ratings display"
```

---

### Task 8: CLI — `jjj rank session` Interactive Flow

**Files:**
- Modify: `src/commands/rank.rs`

**Step 1: Implement the guided session**

Replace the `session` stub:

```rust
use crossterm::event::{self, Event, KeyCode, KeyEvent};
use crossterm::terminal;
use std::io::Write;

fn session(ctx: &CommandContext, milestone: Option<String>, count: usize) -> Result<()> {
    let milestone_id = resolve_milestone_for_rank(ctx, milestone)?;
    let milestone_obj = ctx.store.load_milestone(&milestone_id)?;

    // Get open problems in this milestone
    let problems = ctx.store.list_problems()?;
    let open_problems: Vec<_> = problems
        .iter()
        .filter(|p| {
            p.milestone_id.as_deref() == Some(milestone_id.as_str()) && p.is_open()
        })
        .collect();

    if open_problems.len() < 2 {
        println!("Need at least 2 open problems in milestone \"{}\" to rank.", milestone_obj.title);
        return Ok(());
    }

    // Get current user
    let user = ctx.jj().user_identity()?;

    // Load existing comparisons to compute current ratings
    let attributed = load_attributed_comparisons(ctx.store.meta_path(), &milestone_id)?;
    let owner = milestone_obj.assignee.as_deref().unwrap_or("");
    let owner_weight = 2.0;

    let weighted: Vec<WeightedComparison> = attributed
        .iter()
        .map(|(cmp, u)| WeightedComparison {
            winner: cmp.winner.clone(),
            loser: cmp.loser.clone(),
            weight: if user_matches_owner(u, owner) { owner_weight } else { 1.0 },
        })
        .collect();

    let mut ratings = compute_ratings(&weighted);

    // Ensure all open problems have ratings
    for p in &open_problems {
        ratings.entry(p.id.clone()).or_default();
    }

    // Filter ratings to only open problems
    let open_ids: std::collections::HashSet<&str> = open_problems.iter().map(|p| p.id.as_str()).collect();
    ratings.retain(|k, _| open_ids.contains(k.as_str()));

    // Get recent pairs this user has compared
    let user_slug = crate::ranking::store::sanitize_user(&user);
    let recent_pairs: Vec<(String, String)> = attributed
        .iter()
        .filter(|(_, u)| u == &user_slug)
        .map(|(c, _)| (c.winner.clone(), c.loser.clone()))
        .collect();

    // Suggest matchups
    let matchups = crate::ranking::matchups::suggest_matchups(&ratings, &recent_pairs, count);

    if matchups.is_empty() {
        println!("No more matchups to suggest. All pairs have been compared.");
        return Ok(());
    }

    let title_map: std::collections::HashMap<&str, &str> = open_problems
        .iter()
        .map(|p| (p.id.as_str(), p.title.as_str()))
        .collect();

    println!(
        "Ranking problems in milestone \"{}\" ({} open problems)\n",
        milestone_obj.title,
        open_problems.len(),
    );

    let mut completed = 0;

    // Enable raw mode for single-keypress input
    terminal::enable_raw_mode()?;

    for (a, b) in &matchups {
        let title_a = title_map.get(a.as_str()).unwrap_or(&a.as_str());
        let title_b = title_map.get(b.as_str()).unwrap_or(&b.as_str());
        let short_a = &a[..6.min(a.len())];
        let short_b = &b[..6.min(b.len())];

        // Print to raw terminal
        let mut stdout = std::io::stdout();
        write!(stdout, "\r\nWhich is more important to tackle?\r\n\r\n")?;
        write!(stdout, "  [A] {} ({})\r\n", crate::utils::truncate(title_a, 50), short_a)?;
        write!(stdout, "  [B] {} ({})\r\n", crate::utils::truncate(title_b, 50), short_b)?;
        write!(stdout, "\r\n  Press A, B, or S to skip (Q to quit): ")?;
        stdout.flush()?;

        loop {
            if let Event::Key(KeyEvent { code, .. }) = event::read()? {
                match code {
                    KeyCode::Char('a') | KeyCode::Char('A') => {
                        write!(stdout, "A\r\n")?;
                        let cmp = crate::ranking::glicko2::Comparison {
                            winner: a.clone(),
                            loser: b.clone(),
                            ts: chrono::Utc::now(),
                        };
                        // Save via with_metadata for atomic commit
                        ctx.store.with_metadata(
                            &format!("Rank: {} over {}", short_a, short_b),
                            || {
                                crate::ranking::store::append_comparison(
                                    ctx.store.meta_path(),
                                    &milestone_id,
                                    &user,
                                    &cmp,
                                )
                            },
                        )?;
                        completed += 1;
                        break;
                    }
                    KeyCode::Char('b') | KeyCode::Char('B') => {
                        write!(stdout, "B\r\n")?;
                        let cmp = crate::ranking::glicko2::Comparison {
                            winner: b.clone(),
                            loser: a.clone(),
                            ts: chrono::Utc::now(),
                        };
                        ctx.store.with_metadata(
                            &format!("Rank: {} over {}", short_b, short_a),
                            || {
                                crate::ranking::store::append_comparison(
                                    ctx.store.meta_path(),
                                    &milestone_id,
                                    &user,
                                    &cmp,
                                )
                            },
                        )?;
                        completed += 1;
                        break;
                    }
                    KeyCode::Char('s') | KeyCode::Char('S') => {
                        write!(stdout, "skip\r\n")?;
                        break;
                    }
                    KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => {
                        write!(stdout, "quit\r\n")?;
                        terminal::disable_raw_mode()?;
                        println!("\nRecorded {} comparison(s).", completed);
                        return Ok(());
                    }
                    _ => {}
                }
            }
        }
    }

    terminal::disable_raw_mode()?;
    println!("\nDone! {} comparison(s) recorded.", completed);

    Ok(())
}
```

**Important:** The `sanitize_user` function in `src/ranking/store.rs` needs to be made `pub` (it was `fn` before). Change `fn sanitize_user` to `pub fn sanitize_user`.

**Step 2: Build to verify compilation**

Run: `cargo build 2>&1 | tail -5`
Expected: Compiles

**Step 3: Commit**

```bash
git add src/commands/rank.rs src/ranking/store.rs
git commit -m "feat(ranking): implement interactive jjj rank session with guided matchups"
```

---

### Task 9: CLI — `jjj rank history` Implementation

**Files:**
- Modify: `src/commands/rank.rs`

**Step 1: Implement history**

Replace the `history` stub:

```rust
fn history(ctx: &CommandContext, milestone: Option<String>, limit: usize) -> Result<()> {
    let milestone_id = resolve_milestone_for_rank(ctx, milestone)?;
    let milestone_obj = ctx.store.load_milestone(&milestone_id)?;

    let attributed = load_attributed_comparisons(ctx.store.meta_path(), &milestone_id)?;
    if attributed.is_empty() {
        println!("No ranking history for milestone \"{}\".", milestone_obj.title);
        return Ok(());
    }

    let problems = ctx.store.list_problems()?;
    let title_map: std::collections::HashMap<&str, &str> = problems
        .iter()
        .map(|p| (p.id.as_str(), p.title.as_str()))
        .collect();

    println!("Ranking history for \"{}\" ({} total comparisons)\n", milestone_obj.title, attributed.len());
    println!(" {:<20} {:<8} {:<30} {:<30}", "Timestamp", "User", "Winner", "Loser");
    println!(" {}", "─".repeat(90));

    // Show most recent first, up to limit
    for (cmp, user) in attributed.iter().rev().take(limit) {
        let short_w = &cmp.winner[..6.min(cmp.winner.len())];
        let short_l = &cmp.loser[..6.min(cmp.loser.len())];
        let title_w = title_map.get(cmp.winner.as_str()).unwrap_or(&short_w);
        let title_l = title_map.get(cmp.loser.as_str()).unwrap_or(&short_l);

        println!(
            " {:<20} {:<8} {:<30} {:<30}",
            cmp.ts.format("%Y-%m-%d %H:%M"),
            crate::utils::truncate(user, 8),
            format!("{} ({})", crate::utils::truncate(title_w, 20), short_w),
            format!("{} ({})", crate::utils::truncate(title_l, 20), short_l),
        );
    }

    Ok(())
}
```

**Step 2: Build to verify**

Run: `cargo build 2>&1 | tail -5`
Expected: Compiles

**Step 3: Commit**

```bash
git add src/commands/rank.rs
git commit -m "feat(ranking): implement jjj rank history command"
```

---

### Task 10: Integrate Rankings into `jjj next` / `build_next_actions`

**Files:**
- Modify: `src/tui/next_actions.rs`

**Step 1: Write the failing test**

Add to the `tests` module in `src/tui/next_actions.rs`:

```rust
    #[test]
    fn test_todo_sorting_with_rankings() {
        use std::collections::HashMap;
        use crate::ranking::glicko2::Rating;

        let problems = vec![
            make_problem("P-1", "Low-rated problem"),
            make_problem("P-2", "High-rated problem"),
            make_problem("P-3", "Mid-rated problem"),
        ];

        let mut rankings = HashMap::new();
        rankings.insert("P-1".to_string(), Rating { mu: 1400.0, phi: 50.0, sigma: 0.06 });
        rankings.insert("P-2".to_string(), Rating { mu: 1700.0, phi: 50.0, sigma: 0.06 });
        rankings.insert("P-3".to_string(), Rating { mu: 1550.0, phi: 50.0, sigma: 0.06 });

        let actions = build_next_actions_ranked(&problems, &[], &[], "alice", Some(&rankings));

        assert_eq!(actions.len(), 3);
        // Should be sorted by rating descending: P-2, P-3, P-1
        assert_eq!(actions[0].entity_id, "P-2");
        assert_eq!(actions[1].entity_id, "P-3");
        assert_eq!(actions[2].entity_id, "P-1");
    }

    #[test]
    fn test_todo_sorting_without_rankings_uses_priority() {
        let problems = vec![
            make_problem_with_priority("P-1", "Low", Priority::Low),
            make_problem_with_priority("P-2", "Critical", Priority::Critical),
        ];

        let actions = build_next_actions_ranked(&problems, &[], &[], "alice", None);

        assert_eq!(actions[0].entity_id, "P-2"); // Critical first
        assert_eq!(actions[1].entity_id, "P-1");
    }
```

**Step 2: Run test to verify it fails**

Run: `cargo test next_actions --lib 2>&1 | head -10`
Expected: FAIL — `build_next_actions_ranked` not found

**Step 3: Implement**

Add to `src/tui/next_actions.rs` — a new function alongside the existing `build_next_actions`:

```rust
use crate::ranking::glicko2::Rating;
use std::collections::HashMap;

/// Build next actions with optional Glicko-2 ranking for sorting.
///
/// When `rankings` is provided, TODO items are sorted by Glicko-2 rating
/// instead of static priority. Other categories (BLOCKED, READY, REVIEW)
/// continue to use priority-based sorting.
pub fn build_next_actions_ranked(
    problems: &[Problem],
    solutions: &[Solution],
    critiques: &[Critique],
    user: &str,
    rankings: Option<&HashMap<String, Rating>>,
) -> Vec<NextAction> {
    let mut items = build_next_actions(problems, solutions, critiques, user);

    if let Some(ratings) = rankings {
        // Re-sort TODO items by Glicko-2 rating (descending)
        items.sort_by(|a, b| {
            let cat_cmp = a.category.sort_order().cmp(&b.category.sort_order());
            if cat_cmp != std::cmp::Ordering::Equal {
                return cat_cmp;
            }

            // Within TODO category, sort by Glicko-2 rating if available
            if a.category == Category::Todo && b.category == Category::Todo {
                let ra = ratings.get(&a.entity_id).map(|r| r.mu).unwrap_or(1500.0);
                let rb = ratings.get(&b.entity_id).map(|r| r.mu).unwrap_or(1500.0);
                return rb.partial_cmp(&ra).unwrap_or(std::cmp::Ordering::Equal);
            }

            // Non-TODO categories: keep priority-based sorting
            b.priority.cmp(&a.priority)
        });
    }

    items
}
```

**Step 4: Run test to verify it passes**

Run: `cargo test next_actions --lib -- --nocapture 2>&1 | tail -15`
Expected: All tests PASS (existing + 2 new)

**Step 5: Commit**

```bash
git add src/tui/next_actions.rs
git commit -m "feat(ranking): integrate Glicko-2 ratings into next action sorting"
```

---

### Task 11: Storage Layer — Initialize Rankings Directory

**Files:**
- Modify: `src/storage/mod.rs`

**Step 1: Add RANKINGS_DIR constant and ensure creation in init()**

Add constant alongside the others in `src/storage/mod.rs`:

```rust
pub(super) const RANKINGS_DIR: &str = "rankings";
```

Add `rankings` dir creation in the `init()` method, after the milestones dir:

```rust
        fs::create_dir_all(self.meta_path.join(RANKINGS_DIR))?;
```

**Step 2: Build**

Run: `cargo build 2>&1 | tail -5`
Expected: Compiles

**Step 3: Commit**

```bash
git add src/storage/mod.rs
git commit -m "feat(ranking): add rankings directory to metadata store initialization"
```

---

### Task 12: TUI — Ranking Display in Problem Lists

This task adds Glicko-2 rating display to the TUI's project tree. When a milestone is selected, problems show their rank position and confidence.

**Files:**
- Modify: `src/tui/tree.rs` (add rank info to `TreeNode::Problem`)
- Modify: `src/tui/ui.rs` (render rank indicator)
- Modify: `src/tui/app/mod.rs` (load rankings into `ProjectData`)

**Step 1: Add rank field to TreeNode::Problem**

In `src/tui/tree.rs`, add an optional rank field to the `Problem` variant of `TreeNode`:

```rust
    Problem {
        id: String,
        title: String,
        status: ProblemStatus,
        priority: Priority,
        assignee: Option<String>,
        expanded: bool,
        rank: Option<(usize, String)>,  // (position, confidence)
    },
```

**Step 2: Update tree building to populate rank**

In the function that builds the flat tree, look up each problem's rank from the ranking data (passed as parameter or stored in `ProjectData`).

Add a `rankings` field to `ProjectData`:

```rust
pub struct ProjectData {
    pub milestones: Vec<Milestone>,
    pub problems: Vec<Problem>,
    pub solutions: Vec<Solution>,
    pub critiques: Vec<Critique>,
    pub rankings: HashMap<String, (usize, String)>,  // problem_id → (rank, confidence)
}
```

**Step 3: Render rank in ui.rs**

When rendering a Problem tree node, if `rank` is `Some((pos, conf))`, show `#N` before the title:

```rust
// In draw_project_tree, when rendering Problem nodes:
if let Some((pos, _conf)) = &rank {
    format!("#{} {}", pos, title)
} else {
    title.clone()
}
```

**Step 4: Load rankings in `refresh_data()`**

In `App::refresh_data()` (or equivalent data loading), compute rankings from the comparison files if the active milestone has comparisons.

**Step 5: Build and verify**

Run: `cargo build 2>&1 | tail -5`
Expected: Compiles

**Step 6: Commit**

```bash
git add src/tui/tree.rs src/tui/ui.rs src/tui/app/mod.rs
git commit -m "feat(ranking): display Glicko-2 rank position in TUI problem tree"
```

---

### Task 13: TUI — Quick Rank Mode (press `r`)

**Files:**
- Modify: `src/tui/app/mod.rs` (add `Ranking` input mode and `r` key handler)
- Modify: `src/tui/ui.rs` (render ranking matchup in detail pane)
- Modify: `src/tui/detail.rs` (add `RankingMatchup` variant to `DetailContent`)

**Step 1: Add InputMode variant**

Add to `InputMode` enum:

```rust
    Ranking {
        milestone_id: String,
        matchups: Vec<(String, String)>,
        current: usize,
        completed: usize,
    },
```

**Step 2: Add `r` key handler in `handle_normal_key()`**

When user presses `r` and a milestone is selected:
1. Load rankings and compute matchups
2. Enter `Ranking` mode with the matchup queue

**Step 3: Add ranking key handler**

In `handle_key()`, when mode is `Ranking`:
- `a` → record comparison (first wins), advance to next matchup
- `b` → record comparison (second wins), advance
- `s` → skip this matchup
- `q`/`Esc` → exit ranking mode

**Step 4: Render matchup in detail pane**

Add `DetailContent::RankingMatchup` variant that shows the two problems side by side with key hints.

**Step 5: Build and test**

Run: `cargo build 2>&1 | tail -5`
Expected: Compiles

**Step 6: Commit**

```bash
git add src/tui/app/mod.rs src/tui/ui.rs src/tui/detail.rs
git commit -m "feat(ranking): add TUI quick rank mode with r key"
```

---

### Task 14: Integration Test — Full Ranking Workflow

**Files:**
- Create: `tests/ranking_workflow.rs`

**Step 1: Write the integration test**

```rust
use jjj::ranking::glicko2::{compute_ratings, Comparison, WeightedComparison, Rating};
use jjj::ranking::store::{append_comparison, load_comparisons, load_attributed_comparisons};
use jjj::ranking::matchups::suggest_matchups;
use chrono::Utc;
use tempfile::TempDir;
use std::collections::HashMap;

#[test]
fn test_full_ranking_workflow() {
    let dir = TempDir::new().unwrap();
    let milestone_id = "m-test";

    // 1. Record comparisons from two users
    let cmps_alice = vec![
        ("P-1", "P-2"),
        ("P-1", "P-3"),
        ("P-2", "P-3"),
    ];
    for (w, l) in &cmps_alice {
        append_comparison(dir.path(), milestone_id, "Alice <alice@test.com>", &Comparison {
            winner: w.to_string(),
            loser: l.to_string(),
            ts: Utc::now(),
        }).unwrap();
    }

    append_comparison(dir.path(), milestone_id, "Bob <bob@test.com>", &Comparison {
        winner: "P-3".into(),
        loser: "P-1".into(),
        ts: Utc::now(),
    }).unwrap();

    // 2. Load and verify
    let all = load_comparisons(dir.path(), milestone_id).unwrap();
    assert_eq!(all.len(), 4);

    let attributed = load_attributed_comparisons(dir.path(), milestone_id).unwrap();
    assert_eq!(attributed.len(), 4);

    // 3. Compute ratings (Alice is owner, weight=2)
    let weighted: Vec<WeightedComparison> = attributed
        .iter()
        .map(|(c, user)| WeightedComparison {
            winner: c.winner.clone(),
            loser: c.loser.clone(),
            weight: if user == "alice" { 2.0 } else { 1.0 },
        })
        .collect();
    let ratings = compute_ratings(&weighted);

    // P-1 should be highest (Alice ranked it first with 2x weight)
    assert!(ratings["P-1"].mu > ratings["P-3"].mu);

    // 4. Suggest next matchups
    let recent: Vec<(String, String)> = attributed
        .iter()
        .filter(|(_, u)| u == "bob")
        .map(|(c, _)| (c.winner.clone(), c.loser.clone()))
        .collect();
    let matchups = suggest_matchups(&ratings, &recent, 3);
    assert!(!matchups.is_empty());

    // Bob's only comparison was P-3 > P-1, so P-2 should appear in suggestions
    let has_p2 = matchups.iter().any(|(a, b)| a == "P-2" || b == "P-2");
    assert!(has_p2, "Should suggest matchups involving the least-compared item");
}
```

**Step 2: Run test**

Run: `cargo test ranking_workflow 2>&1 | tail -15`
Expected: PASS

**Step 3: Commit**

```bash
git add tests/ranking_workflow.rs
git commit -m "test: add integration test for full ranking workflow"
```

---

### Task 15: Final — Cargo Clippy, Format, and Full Test Suite

**Step 1: Format**

Run: `cargo fmt`

**Step 2: Clippy**

Run: `cargo clippy -- -D warnings 2>&1 | tail -20`
Expected: No warnings. Fix any issues.

**Step 3: Full test suite**

Run: `cargo test 2>&1 | tail -20`
Expected: All tests pass.

**Step 4: Commit any fixes**

```bash
git add -A
git commit -m "chore: fix clippy warnings and formatting for ranking feature"
```
