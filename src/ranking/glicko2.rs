use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// System volatility constraint (τ). Controls how much volatility can change
/// per rating period. Lower values make the system more conservative.
pub const TAU: f64 = 0.5;

/// Convergence tolerance for the Illinois algorithm.
pub const EPSILON: f64 = 0.000001;

/// Scaling factor that converts between Glicko (mu ~ 1500) and Glicko-2
/// (mu ~ 0) scales: `SCALE = 400 / ln(10) ≈ 173.7178`.
pub const SCALE: f64 = 173.7178;

// ---------------------------------------------------------------------------
// Core types
// ---------------------------------------------------------------------------

/// A Glicko-2 rating on the *Glicko-1* scale (mu centred around 1500).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Rating {
    /// Rating value (Glicko-1 scale, default 1500.0).
    pub mu: f64,
    /// Rating deviation (Glicko-1 scale, default 350.0).
    pub phi: f64,
    /// Rating volatility (default 0.06).
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

/// Qualitative confidence band derived from the rating deviation (phi).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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
            Confidence::Medium => write!(f, "medium"),
            Confidence::Low => write!(f, "low"),
        }
    }
}

impl Rating {
    /// Returns a qualitative confidence level based on the rating deviation.
    ///
    /// - `phi < 50`  => High (very confident)
    /// - `phi < 150` => Medium
    /// - otherwise   => Low (uncertain)
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

// ---------------------------------------------------------------------------
// Comparison types (for batch computation)
// ---------------------------------------------------------------------------

/// A recorded pairwise comparison with a timestamp.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Comparison {
    pub winner: String,
    pub loser: String,
    pub ts: DateTime<Utc>,
}

/// A pairwise comparison with an associated weight.
///
/// `weight` scales the comparison's impact on ratings:
/// - 1.0 = normal
/// - 2.0 = double impact (e.g. owner/expert opinion)
#[derive(Debug, Clone)]
pub struct WeightedComparison {
    pub winner: String,
    pub loser: String,
    pub weight: f64,
}

// ---------------------------------------------------------------------------
// Glicko-2 internals
// ---------------------------------------------------------------------------

/// Convert from Glicko-1 scale to Glicko-2 scale.
fn to_glicko2(mu: f64, phi: f64) -> (f64, f64) {
    ((mu - 1500.0) / SCALE, phi / SCALE)
}

/// Convert from Glicko-2 scale back to Glicko-1 scale.
fn from_glicko2(mu2: f64, phi2: f64) -> (f64, f64) {
    (mu2 * SCALE + 1500.0, phi2 * SCALE)
}

/// The g(φ) function from the Glicko-2 algorithm.
///
/// g(φ) = 1 / sqrt(1 + 3φ²/π²)
fn g(phi: f64) -> f64 {
    1.0 / (1.0 + 3.0 * phi * phi / (std::f64::consts::PI * std::f64::consts::PI)).sqrt()
}

/// Expected score E(μ, μ_j, φ_j).
///
/// E = 1 / (1 + exp(-g(φ_j)(μ - μ_j)))
fn expected(mu: f64, mu_j: f64, phi_j: f64) -> f64 {
    1.0 / (1.0 + (-g(phi_j) * (mu - mu_j)).exp())
}

/// Compute new volatility (σ') using the Illinois algorithm (Glicko-2 Step 5).
///
/// Finds the root of `f(x) = 0` where:
///   f(x) = (e^x (Δ² - φ² - v - e^x)) / (2(φ² + v + e^x)²) - (x - ln(σ²)) / τ²
fn new_volatility(sigma: f64, phi: f64, v: f64, delta: f64) -> f64 {
    let a = (sigma * sigma).ln();
    let tau_sq = TAU * TAU;
    let phi_sq = phi * phi;

    // The function f(x) we need to find the root of.
    let f = |x: f64| -> f64 {
        let ex = x.exp();
        let d2 = delta * delta;
        let denom = phi_sq + v + ex;
        (ex * (d2 - phi_sq - v - ex)) / (2.0 * denom * denom) - (x - a) / tau_sq
    };

    // Step 5a: Set initial bracket [a_val, b_val].
    let mut a_val = a;
    let mut b_val = if delta * delta > phi_sq + v {
        (delta * delta - phi_sq - v).ln()
    } else {
        // Find k such that f(a - k*τ) is positive.
        let mut k = 1.0_f64;
        while f(a - k * TAU) < 0.0 {
            k += 1.0;
            if k > 100.0 {
                break;
            }
        }
        a - k * TAU
    };

    // Step 5b-d: Illinois algorithm for root finding.
    let mut f_a = f(a_val);
    let mut f_b = f(b_val);

    while (b_val - a_val).abs() > EPSILON {
        if (f_b - f_a).abs() < 1e-15 {
            break;
        }
        let c = a_val + (a_val - b_val) * f_a / (f_b - f_a);
        let f_c = f(c);

        if f_c * f_b <= 0.0 {
            a_val = b_val;
            f_a = f_b;
        } else {
            f_a /= 2.0;
        }

        b_val = c;
        f_b = f_c;
    }

    (a_val / 2.0).exp()
}

/// Perform a single-player Glicko-2 update against one opponent.
///
/// Returns `(mu', phi', sigma')` on the Glicko-1 scale.
///
/// - `score`: 1.0 for win, 0.0 for loss
/// - `weight`: scales the comparison impact (1.0 = normal)
fn update_single(
    mu: f64,
    phi: f64,
    sigma: f64,
    mu_j: f64,
    phi_j: f64,
    score: f64,
    weight: f64,
) -> (f64, f64, f64) {
    // Convert to Glicko-2 scale.
    let (mu2, phi2) = to_glicko2(mu, phi);
    let (mu_j2, phi_j2) = to_glicko2(mu_j, phi_j);

    // Step 3: Compute the estimated variance v and estimated improvement Δ.
    let g_val = g(phi_j2);
    let e_val = expected(mu2, mu_j2, phi_j2);
    let e_clamped = e_val.clamp(1e-10, 1.0 - 1e-10);

    // Apply weight: scale the "number of games" effectively.
    let v = 1.0 / (weight * g_val * g_val * e_clamped * (1.0 - e_clamped));
    let delta = v * weight * g_val * (score - e_val);

    // Step 4-5: Update volatility.
    let sigma_prime = new_volatility(sigma, phi2, v, delta);

    // Step 6: Pre-rating period phi*.
    let phi_star = (phi2 * phi2 + sigma_prime * sigma_prime).sqrt();

    // Step 7: Update rating and RD.
    let phi_prime = 1.0 / (1.0 / (phi_star * phi_star) + weight / v).sqrt();
    let mu_prime = mu2 + phi_prime * phi_prime * weight * g_val * (score - e_val);

    // Convert back.
    let (mu_out, phi_out) = from_glicko2(mu_prime, phi_prime);
    (mu_out, phi_out, sigma_prime)
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Update the ratings of both players after a single comparison.
///
/// `weight` scales the comparison's impact (1.0 = normal, 2.0 = double).
pub fn update_ratings(winner: &mut Rating, loser: &mut Rating, weight: f64) {
    debug_assert!(weight > 0.0, "weight must be positive, got {}", weight);

    // Snapshot current values so each update uses pre-update opponent values.
    let w_mu = winner.mu;
    let w_phi = winner.phi;
    let w_sigma = winner.sigma;
    let l_mu = loser.mu;
    let l_phi = loser.phi;
    let l_sigma = loser.sigma;

    let (wmu, wphi, wsig) = update_single(w_mu, w_phi, w_sigma, l_mu, l_phi, 1.0, weight);
    let (lmu, lphi, lsig) = update_single(l_mu, l_phi, l_sigma, w_mu, w_phi, 0.0, weight);

    winner.mu = wmu;
    winner.phi = wphi;
    winner.sigma = wsig;

    loser.mu = lmu;
    loser.phi = lphi;
    loser.sigma = lsig;
}

/// Process a sequence of weighted comparisons and return ratings for every
/// participant.
///
/// Each comparison updates both winner and loser ratings sequentially.
/// Participants not previously seen start with `Rating::default()`.
pub fn compute_ratings(comparisons: &[WeightedComparison]) -> HashMap<String, Rating> {
    let mut ratings: HashMap<String, Rating> = HashMap::new();

    for cmp in comparisons {
        // Ensure both participants exist.
        ratings.entry(cmp.winner.clone()).or_default();
        ratings.entry(cmp.loser.clone()).or_default();

        // Clone out the current ratings so we can mutate them.
        let mut winner_rating = ratings[&cmp.winner].clone();
        let mut loser_rating = ratings[&cmp.loser].clone();

        update_ratings(&mut winner_rating, &mut loser_rating, cmp.weight);

        ratings.insert(cmp.winner.clone(), winner_rating);
        ratings.insert(cmp.loser.clone(), loser_rating);
    }

    ratings
}

/// Return participants sorted by descending rating (highest first).
pub fn sorted_ranking(ratings: &HashMap<String, Rating>) -> Vec<(String, Rating)> {
    let mut ranked: Vec<(String, Rating)> = ratings
        .iter()
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();
    ranked.sort_by(|a, b| b.1.mu.partial_cmp(&a.1.mu).unwrap_or(std::cmp::Ordering::Equal));
    ranked
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_rating() {
        let r = Rating::default();
        assert!((r.mu - 1500.0).abs() < f64::EPSILON);
        assert!((r.phi - 350.0).abs() < f64::EPSILON);
        assert!((r.sigma - 0.06).abs() < f64::EPSILON);
    }

    #[test]
    fn test_confidence_high() {
        let r = Rating {
            mu: 1500.0,
            phi: 30.0,
            sigma: 0.06,
        };
        assert_eq!(r.confidence(), Confidence::High);
    }

    #[test]
    fn test_confidence_low() {
        let r = Rating {
            mu: 1500.0,
            phi: 350.0,
            sigma: 0.06,
        };
        assert_eq!(r.confidence(), Confidence::Low);
    }

    #[test]
    fn test_update_winner_rating_increases() {
        let mut winner = Rating::default();
        let mut loser = Rating::default();
        update_ratings(&mut winner, &mut loser, 1.0);
        assert!(
            winner.mu > 1500.0,
            "Winner mu should increase: {}",
            winner.mu
        );
        assert!(loser.mu < 1500.0, "Loser mu should decrease: {}", loser.mu);
    }

    #[test]
    fn test_update_symmetric() {
        let mut winner = Rating::default();
        let mut loser = Rating::default();
        update_ratings(&mut winner, &mut loser, 1.0);

        let gain = winner.mu - 1500.0;
        let loss = 1500.0 - loser.mu;
        assert!(
            (gain - loss).abs() < 1.0,
            "Gain ({gain}) and loss ({loss}) should be approximately equal"
        );
    }

    #[test]
    fn test_update_reduces_deviation() {
        let mut winner = Rating::default();
        let mut loser = Rating::default();
        let initial_phi = winner.phi;
        update_ratings(&mut winner, &mut loser, 1.0);
        assert!(
            winner.phi < initial_phi,
            "Winner phi should decrease: {} < {}",
            winner.phi,
            initial_phi
        );
        assert!(
            loser.phi < initial_phi,
            "Loser phi should decrease: {} < {}",
            loser.phi,
            initial_phi
        );
    }

    #[test]
    fn test_update_with_weight() {
        let mut w1 = Rating::default();
        let mut l1 = Rating::default();
        update_ratings(&mut w1, &mut l1, 1.0);
        let change_1 = w1.mu - 1500.0;

        let mut w2 = Rating::default();
        let mut l2 = Rating::default();
        update_ratings(&mut w2, &mut l2, 2.0);
        let change_2 = w2.mu - 1500.0;

        assert!(
            change_2 > change_1,
            "Weight 2.0 change ({change_2}) should exceed weight 1.0 change ({change_1})"
        );
    }

    #[test]
    fn test_compute_ratings_empty() {
        let ratings = compute_ratings(&[]);
        assert!(ratings.is_empty());
    }

    #[test]
    fn test_compute_ratings_total_ordering() {
        // A beats B, A beats C, B beats C => A > B > C
        let comparisons = vec![
            WeightedComparison {
                winner: "A".to_string(),
                loser: "B".to_string(),
                weight: 1.0,
            },
            WeightedComparison {
                winner: "A".to_string(),
                loser: "C".to_string(),
                weight: 1.0,
            },
            WeightedComparison {
                winner: "B".to_string(),
                loser: "C".to_string(),
                weight: 1.0,
            },
        ];
        let ratings = compute_ratings(&comparisons);
        assert!(
            ratings["A"].mu > ratings["B"].mu,
            "A ({}) should rank above B ({})",
            ratings["A"].mu,
            ratings["B"].mu
        );
        assert!(
            ratings["B"].mu > ratings["C"].mu,
            "B ({}) should rank above C ({})",
            ratings["B"].mu,
            ratings["C"].mu
        );
    }

    #[test]
    fn test_compute_ratings_owner_weight_wins() {
        // Regular user (weight 1) says A > B.
        // Owner (weight 2) says B > A.
        // Owner's opinion should dominate => B > A.
        let comparisons = vec![
            WeightedComparison {
                winner: "A".to_string(),
                loser: "B".to_string(),
                weight: 1.0,
            },
            WeightedComparison {
                winner: "B".to_string(),
                loser: "A".to_string(),
                weight: 2.0,
            },
        ];
        let ratings = compute_ratings(&comparisons);
        assert!(
            ratings["B"].mu > ratings["A"].mu,
            "B ({}) should rank above A ({}) because owner weight dominates",
            ratings["B"].mu,
            ratings["A"].mu
        );
    }
}
