use serde::{Deserialize, Serialize};

use super::history::PairHistory;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueUser {
    pub user_id: uuid::Uuid,
    pub display_name: String,
    pub bio: Option<String>,
    pub age: i32,
    pub country: Option<String>,
    pub kinks: Vec<String>,
    pub profile_photo_url: Option<String>,
    pub filters: MatchFilters,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    #[serde(default = "default_joined_at")]
    pub joined_at: i64, // timestamp millis UTC
}

fn default_joined_at() -> i64 {
    chrono::Utc::now().timestamp_millis()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchFilters {
    pub country: Option<String>,
    pub age_min: Option<i32>,
    pub age_max: Option<i32>,
    pub kinks: Vec<String>,
}

pub struct MatchScore {
    pub score: f64,
    pub passes_filters: bool,
}

// -- Weights v3 — liquidity-first, Flingster-speed --
// Age is NEVER blocking, just a strong scoring signal.
// Country is the only hard filter (and only briefly).
const W_COUNTRY: f64 = 0.25;
const W_AGE: f64 = 0.25;
const W_KINKS: f64 = 0.20;
const W_HISTORY: f64 = 0.15;
const W_FRESHNESS: f64 = 0.10;
const W_DISTANCE: f64 = 0.05;

// -- Match phases — instant matching, millisecond-scale --
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MatchPhase {
    Strict,    // 0-500ms  : country hard, best score
    Normal,    // 500ms-1s : country soft
    Relaxed,   // 1-3s     : everything soft
    Desperate, // 3s+      : match anyone alive
}

impl MatchPhase {
    pub fn from_wait_ms(wait_ms: i64) -> Self {
        match wait_ms {
            0..=499 => Self::Strict,
            500..=999 => Self::Normal,
            1000..=2999 => Self::Relaxed,
            _ => Self::Desperate,
        }
    }

    pub fn min_match_score(self) -> f64 {
        match self {
            Self::Strict => 0.10,
            Self::Normal => 0.05,
            Self::Relaxed => 0.02,
            Self::Desperate => 0.0,
        }
    }
}

/// Calculate the FSRS-inspired pair modifier from PairHistory.
fn pair_modifier(history: &PairHistory) -> f64 {
    if history.times_matched == 0 {
        return 1.05; // novelty bonus for never-seen pairs
    }

    let stability = (history.likes as f64) * 2.0
        + if history.follows { 3.0 } else { 0.0 }
        + (history.messages as f64) * 0.1
        + if history.total_duration_secs > 120 { 1.5 } else { 0.0 }
        - (history.skips as f64) * 2.0;

    let now_ms = chrono::Utc::now().timestamp_millis();
    let elapsed_hours = ((now_ms - history.last_matched_at) as f64) / 3_600_000.0;
    let denom = 9.0 * stability.abs().max(1.0);
    let retrievability = 1.0 / (1.0 + elapsed_hours / denom);

    if stability > 0.0 {
        0.85 + 0.20 * (1.0 - retrievability)
    } else {
        0.30 + 0.70 * (1.0 - retrievability)
    }
}

/// Calculate freshness bonus — users waiting longer get a slight boost.
fn freshness_score(candidate_wait_ms: i64) -> f64 {
    // Normalize: 0ms → 0.5, 3000ms → 1.0 (capped)
    (0.5 + (candidate_wait_ms as f64) / 6000.0).min(1.0)
}

/// Haversine distance in km between two lat/lng points.
pub fn haversine_km(lat1: f64, lng1: f64, lat2: f64, lng2: f64) -> f64 {
    const R: f64 = 6371.0; // Earth radius in km
    let d_lat = (lat2 - lat1).to_radians();
    let d_lng = (lng2 - lng1).to_radians();
    let a = (d_lat / 2.0).sin().powi(2)
        + lat1.to_radians().cos() * lat2.to_radians().cos() * (d_lng / 2.0).sin().powi(2);
    let c = 2.0 * a.sqrt().asin();
    R * c
}

/// Distance score: closer users score higher.
/// 0 km → 1.0, 100 km → ~0.5, 500+ km → ~0.1.
fn distance_score(user_a: &QueueUser, user_b: &QueueUser) -> f64 {
    match (user_a.latitude, user_a.longitude, user_b.latitude, user_b.longitude) {
        (Some(lat1), Some(lng1), Some(lat2), Some(lng2)) => {
            let km = haversine_km(lat1, lng1, lat2, lng2);
            // Exponential decay: e^(-km/200), clamped to [0.05, 1.0]
            (-km / 200.0).exp().max(0.05)
        }
        _ => 0.5, // No geolocation → neutral score
    }
}

/// Age proximity as a soft score (NEVER blocking).
/// Same age = 1.0, 10y apart ≈ 0.5, 40y+ apart → 0.05.
fn age_score(age_a: i32, age_b: i32, filters_a: &MatchFilters, filters_b: &MatchFilters) -> f64 {
    let diff = (age_a - age_b).abs() as f64;
    let base = (1.0 - diff / 42.0).max(0.05);

    // Penalty if outside the other user's preferred range (but still not blocking)
    let mut penalty = 1.0;
    if let Some(min) = filters_b.age_min {
        if age_a < min {
            penalty *= 0.5;
        }
    }
    if let Some(max) = filters_b.age_max {
        if age_a > max {
            penalty *= 0.5;
        }
    }
    if let Some(min) = filters_a.age_min {
        if age_b < min {
            penalty *= 0.5;
        }
    }
    if let Some(max) = filters_a.age_max {
        if age_b > max {
            penalty *= 0.5;
        }
    }

    base * penalty
}

pub fn calculate_score(
    user_a: &QueueUser,
    user_b: &QueueUser,
    phase_a: MatchPhase,
    phase_b: MatchPhase,
    history: &PairHistory,
) -> MatchScore {
    // Use the more lenient phase (higher wait = more relaxed)
    let phase = if (phase_a as u8) >= (phase_b as u8) {
        phase_a
    } else {
        phase_b
    };

    // Check mutual filters with relaxation (only country can block, never age)
    let a_passes = passes_filters(user_a, &user_b.filters, phase);
    let b_passes = passes_filters(user_b, &user_a.filters, phase);

    if !a_passes || !b_passes {
        return MatchScore {
            score: 0.0,
            passes_filters: false,
        };
    }

    // Kinks overlap
    let kinks_overlap = if user_a.kinks.is_empty() && user_b.kinks.is_empty() {
        0.5
    } else {
        let intersection = user_a
            .kinks
            .iter()
            .filter(|k| user_b.kinks.contains(k))
            .count();
        let max_len = user_a.kinks.len().max(user_b.kinks.len()).max(1);
        intersection as f64 / max_len as f64
    };

    // Country match (score component)
    let country_match = match (&user_a.country, &user_b.country) {
        (Some(a), Some(b)) if a == b => 1.0,
        (None, _) | (_, None) => 0.7,
        _ => 0.3,
    };

    // Age proximity (soft score with penalty, never blocking)
    let age_prox = age_score(user_a.age, user_b.age, &user_a.filters, &user_b.filters);

    // Pair modifier (FSRS-inspired)
    let pair_mod = pair_modifier(history);

    // Freshness: average of both users' wait times (in ms)
    let now_ms = chrono::Utc::now().timestamp_millis();
    let wait_a = (now_ms - user_a.joined_at).max(0);
    let wait_b = (now_ms - user_b.joined_at).max(0);
    let freshness = freshness_score((wait_a + wait_b) / 2);

    let distance_factor = distance_score(user_a, user_b);

    let score = W_COUNTRY * country_match
        + W_AGE * age_prox
        + W_KINKS * kinks_overlap
        + W_HISTORY * pair_mod
        + W_FRESHNESS * freshness
        + W_DISTANCE * distance_factor;

    MatchScore {
        score,
        passes_filters: true,
    }
}

/// Only country can hard-block, and only in the first 2 seconds.
/// Age is NEVER blocking. Kinks are NEVER blocking.
fn passes_filters(user: &QueueUser, filters: &MatchFilters, phase: MatchPhase) -> bool {
    // Country: hard filter in Strict only (first 2s), soft after
    if phase == MatchPhase::Strict {
        if let Some(ref country) = filters.country {
            if let Some(ref user_country) = user.country {
                if country != user_country {
                    return false;
                }
            }
        }
    }

    // Everything else is scoring-only, never blocking
    true
}
