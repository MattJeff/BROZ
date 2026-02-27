use broz_shared::clients::redis::RedisClient;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

const HISTORY_PREFIX: &str = "matching:history";
const HISTORY_TTL: u64 = 604800; // 7 days
const SESSION_LIKES_PREFIX: &str = "matching:session_likes";
const SESSION_FOLLOW_PREFIX: &str = "matching:session_follow";
const SESSION_MSGS_PREFIX: &str = "matching:session_msgs";
const SESSION_TTL: u64 = 3600; // 1h safety net

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PairHistory {
    pub times_matched: u32,
    pub last_matched_at: i64, // timestamp millis
    pub total_duration_secs: u32,
    pub likes: u8,
    pub follows: bool,
    pub messages: u32,
    pub skips: u32,
}

fn pair_key(a: &Uuid, b: &Uuid) -> String {
    let (lo, hi) = if a < b { (a, b) } else { (b, a) };
    format!("{HISTORY_PREFIX}:{lo}:{hi}")
}

pub async fn get_pair_history(redis: &RedisClient, a: &Uuid, b: &Uuid) -> PairHistory {
    let key = pair_key(a, b);
    match redis.get(&key).await {
        Ok(Some(data)) => serde_json::from_str(&data).unwrap_or_default(),
        _ => PairHistory::default(),
    }
}

pub async fn save_pair_history(redis: &RedisClient, a: &Uuid, b: &Uuid, history: &PairHistory) {
    let key = pair_key(a, b);
    if let Ok(data) = serde_json::to_string(history) {
        let _ = redis.set(&key, &data, HISTORY_TTL).await;
    }
}

/// Called when a match session ends. Reads session counters, updates PairHistory, cleans up.
pub async fn record_match_end(
    redis: &RedisClient,
    a: &Uuid,
    b: &Uuid,
    duration_secs: u32,
    match_id: &Uuid,
) {
    let likes_key = format!("{SESSION_LIKES_PREFIX}:{match_id}");
    let follow_key = format!("{SESSION_FOLLOW_PREFIX}:{match_id}");
    let msgs_key = format!("{SESSION_MSGS_PREFIX}:{match_id}");

    // Read session counters
    let session_likes: u8 = redis
        .get(&likes_key)
        .await
        .ok()
        .flatten()
        .and_then(|v| v.parse::<u8>().ok())
        .unwrap_or(0);

    let session_follow: bool = redis
        .get(&follow_key)
        .await
        .ok()
        .flatten()
        .map(|v| v == "1")
        .unwrap_or(false);

    let session_msgs: u32 = redis
        .get(&msgs_key)
        .await
        .ok()
        .flatten()
        .and_then(|v| v.parse::<u32>().ok())
        .unwrap_or(0);

    // Load existing history
    let mut history = get_pair_history(redis, a, b).await;

    // Update
    history.times_matched += 1;
    history.last_matched_at = chrono::Utc::now().timestamp_millis();
    history.total_duration_secs = history.total_duration_secs.saturating_add(duration_secs);
    history.likes = history.likes.saturating_add(session_likes);
    history.follows = history.follows || session_follow;
    history.messages = history.messages.saturating_add(session_msgs);
    if duration_secs < 15 {
        history.skips = history.skips.saturating_add(1);
    }

    // Save
    save_pair_history(redis, a, b, &history).await;

    // Cleanup session keys
    let _ = redis.del(&likes_key).await;
    let _ = redis.del(&follow_key).await;
    let _ = redis.del(&msgs_key).await;
}

/// Batch fetch pair histories for one user against multiple candidates.
/// Returns a Vec<PairHistory> in the same order as `candidate_ids`.
pub async fn get_pair_histories_batch(
    redis: &RedisClient,
    user_id: &Uuid,
    candidate_ids: &[Uuid],
) -> Vec<PairHistory> {
    if candidate_ids.is_empty() {
        return vec![];
    }
    let keys: Vec<String> = candidate_ids
        .iter()
        .map(|cid| pair_key(user_id, cid))
        .collect();
    match redis.mget(&keys).await {
        Ok(values) => values
            .into_iter()
            .map(|v| {
                v.and_then(|data| serde_json::from_str(&data).ok())
                    .unwrap_or_default()
            })
            .collect(),
        Err(e) => {
            tracing::error!(error = %e, "failed to batch fetch pair histories");
            vec![PairHistory::default(); candidate_ids.len()]
        }
    }
}

// -- Session counter increments (fire-and-forget, use Redis INCR for atomicity) --

pub async fn increment_session_likes(redis: &RedisClient, match_id: &Uuid) {
    let key = format!("{SESSION_LIKES_PREFIX}:{match_id}");
    let _ = redis.incr(&key).await;
    let _ = redis.expire(&key, SESSION_TTL as i64).await;
}

pub async fn set_session_follow(redis: &RedisClient, match_id: &Uuid) {
    let key = format!("{SESSION_FOLLOW_PREFIX}:{match_id}");
    let _ = redis.set(&key, "1", SESSION_TTL).await;
}

pub async fn increment_session_msgs(redis: &RedisClient, match_id: &Uuid) {
    let key = format!("{SESSION_MSGS_PREFIX}:{match_id}");
    let _ = redis.incr(&key).await;
    let _ = redis.expire(&key, SESSION_TTL as i64).await;
}
