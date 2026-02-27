use broz_shared::clients::redis::RedisClient;
use uuid::Uuid;

use super::algorithm::QueueUser;

const QUEUE_KEY: &str = "matching:queue";
const COOLDOWN_PREFIX: &str = "matching:cooldown";
const PAIR_PREFIX: &str = "matching:pair";
const USER_MATCH_PREFIX: &str = "matching:user_match";

pub async fn add_to_queue(redis: &RedisClient, user: &QueueUser) -> Result<(), String> {
    let score = chrono::Utc::now().timestamp_millis() as f64;
    let data = serde_json::to_string(user).map_err(|e| e.to_string())?;
    redis
        .zadd(QUEUE_KEY, &data, score)
        .await
        .map_err(|e| e.to_string())
}

pub async fn remove_from_queue(redis: &RedisClient, user_id: &Uuid) -> Result<bool, String> {
    let members = redis
        .zrange(QUEUE_KEY, 0, -1)
        .await
        .map_err(|e| e.to_string())?;
    for member in members {
        if let Ok(u) = serde_json::from_str::<QueueUser>(&member) {
            if u.user_id == *user_id {
                redis
                    .zrem(QUEUE_KEY, &member)
                    .await
                    .map_err(|e| e.to_string())?;
                return Ok(true);
            }
        }
    }
    Ok(false)
}

pub async fn get_queue_users(redis: &RedisClient) -> Result<Vec<QueueUser>, String> {
    let members = redis
        .zrange(QUEUE_KEY, 0, -1)
        .await
        .map_err(|e| e.to_string())?;
    let mut users = Vec::new();
    for m in members {
        if let Ok(u) = serde_json::from_str::<QueueUser>(&m) {
            users.push(u);
        }
    }
    Ok(users)
}

pub async fn get_queue_size(redis: &RedisClient) -> u64 {
    redis.zcard(QUEUE_KEY).await.unwrap_or(0)
}

pub async fn is_in_queue(redis: &RedisClient, user_id: &Uuid) -> bool {
    let members = redis.zrange(QUEUE_KEY, 0, -1).await.unwrap_or_default();
    members.iter().any(|m| {
        serde_json::from_str::<QueueUser>(m)
            .map(|u| u.user_id == *user_id)
            .unwrap_or(false)
    })
}

pub async fn has_cooldown(redis: &RedisClient, user_a: &Uuid, user_b: &Uuid) -> bool {
    let (a, b) = if user_a < user_b {
        (user_a, user_b)
    } else {
        (user_b, user_a)
    };
    let key = format!("{COOLDOWN_PREFIX}:{a}:{b}");
    redis.exists(&key).await.unwrap_or(false)
}

/// Batch check cooldowns for one user against multiple candidates.
/// Returns a Vec<bool> in the same order as `candidate_ids`.
pub async fn has_cooldowns_batch(
    redis: &RedisClient,
    user_id: &Uuid,
    candidate_ids: &[Uuid],
) -> Vec<bool> {
    if candidate_ids.is_empty() {
        return vec![];
    }
    let keys: Vec<String> = candidate_ids
        .iter()
        .map(|cid| {
            let (a, b) = if user_id < cid { (user_id, cid) } else { (cid, user_id) };
            format!("{COOLDOWN_PREFIX}:{a}:{b}")
        })
        .collect();
    redis.exists_multi(&keys).await.unwrap_or_else(|e| {
        tracing::error!(error = %e, "failed to batch check cooldowns");
        vec![false; candidate_ids.len()]
    })
}

pub async fn set_cooldown(redis: &RedisClient, user_a: &Uuid, user_b: &Uuid) {
    let (a, b) = if user_a < user_b {
        (user_a, user_b)
    } else {
        (user_b, user_a)
    };
    let key = format!("{COOLDOWN_PREFIX}:{a}:{b}");
    let _ = redis.set(&key, "1", 5).await;
}

pub async fn set_active_pair(
    redis: &RedisClient,
    match_id: &Uuid,
    user_a: &Uuid,
    user_b: &Uuid,
) {
    let key = format!("{PAIR_PREFIX}:{match_id}");
    let val = serde_json::json!({"user_a": user_a, "user_b": user_b}).to_string();
    let _ = redis.set(&key, &val, 3600).await;

    // Also store the reverse mapping: user -> match_id for quick lookup
    let key_a = format!("{USER_MATCH_PREFIX}:{user_a}");
    let key_b = format!("{USER_MATCH_PREFIX}:{user_b}");
    let _ = redis.set(&key_a, &match_id.to_string(), 3600).await;
    let _ = redis.set(&key_b, &match_id.to_string(), 3600).await;
}

pub async fn remove_active_pair(redis: &RedisClient, match_id: &Uuid) {
    // Get pair info before removing
    let key = format!("{PAIR_PREFIX}:{match_id}");
    if let Ok(Some(val)) = redis.get(&key).await {
        if let Ok(pair) = serde_json::from_str::<serde_json::Value>(&val) {
            if let (Some(a), Some(b)) = (
                pair.get("user_a").and_then(|v| v.as_str()),
                pair.get("user_b").and_then(|v| v.as_str()),
            ) {
                let _ = redis.del(&format!("{USER_MATCH_PREFIX}:{a}")).await;
                let _ = redis.del(&format!("{USER_MATCH_PREFIX}:{b}")).await;
            }
        }
    }
    let _ = redis.del(&key).await;
}

pub async fn get_user_active_match(redis: &RedisClient, user_id: &Uuid) -> Option<Uuid> {
    let key = format!("{USER_MATCH_PREFIX}:{user_id}");
    if let Ok(Some(val)) = redis.get(&key).await {
        val.parse::<Uuid>().ok()
    } else {
        None
    }
}

pub async fn get_active_pair(
    redis: &RedisClient,
    match_id: &Uuid,
) -> Option<(Uuid, Uuid)> {
    let key = format!("{PAIR_PREFIX}:{match_id}");
    if let Ok(Some(val)) = redis.get(&key).await {
        if let Ok(pair) = serde_json::from_str::<serde_json::Value>(&val) {
            let user_a = pair
                .get("user_a")
                .and_then(|v| v.as_str())
                .and_then(|s| s.parse::<Uuid>().ok());
            let user_b = pair
                .get("user_b")
                .and_then(|v| v.as_str())
                .and_then(|s| s.parse::<Uuid>().ok());
            if let (Some(a), Some(b)) = (user_a, user_b) {
                return Some((a, b));
            }
        }
    }
    None
}

/// Get the partner of a user in a given match
pub async fn get_partner(redis: &RedisClient, match_id: &Uuid, user_id: &Uuid) -> Option<Uuid> {
    if let Some((a, b)) = get_active_pair(redis, match_id).await {
        if a == *user_id {
            Some(b)
        } else if b == *user_id {
            Some(a)
        } else {
            None
        }
    } else {
        None
    }
}
