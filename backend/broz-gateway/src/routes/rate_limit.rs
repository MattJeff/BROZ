use axum::http::StatusCode;
use broz_shared::UserRole;
use chrono::Utc;
use redis::AsyncCommands;
use uuid::Uuid;

use crate::config::AppConfig;

/// Check per-minute and per-hour rate limits for a user.
///
/// Keys:
/// - Per-minute: `rl:{user_id}:min:{YYYYMMDDHHMM}`
/// - Per-hour:   `rl:{user_id}:hr:{YYYYMMDDHH}`
///
/// `UserRole::User` gets the free-tier limits; `Moderator`/`Admin` get premium limits.
///
/// Returns `Err(StatusCode::TOO_MANY_REQUESTS)` if any limit is exceeded.
pub async fn check_rate_limit(
    redis: &tokio::sync::Mutex<redis::aio::ConnectionManager>,
    user_id: Uuid,
    role: UserRole,
    config: &AppConfig,
) -> Result<(), StatusCode> {
    let now = Utc::now();
    let minute_key = format!("rl:{}:min:{}", user_id, now.format("%Y%m%d%H%M"));
    let hour_key = format!("rl:{}:hr:{}", user_id, now.format("%Y%m%d%H"));

    let (rpm_limit, rph_limit) = match role {
        UserRole::User => (config.free_rpm, config.free_rph),
        UserRole::Moderator | UserRole::Admin => (config.premium_rpm, config.premium_rph),
    };

    let mut conn = redis.lock().await;

    // Per-minute check
    let minute_count: u64 = conn.incr(&minute_key, 1u64).await.map_err(|e| {
        tracing::error!(error = %e, "redis incr failed for minute key");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;
    if minute_count == 1 {
        let _: () = conn.expire(&minute_key, 60).await.map_err(|e| {
            tracing::error!(error = %e, "redis expire failed for minute key");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
    }
    if minute_count > rpm_limit {
        return Err(StatusCode::TOO_MANY_REQUESTS);
    }

    // Per-hour check
    let hour_count: u64 = conn.incr(&hour_key, 1u64).await.map_err(|e| {
        tracing::error!(error = %e, "redis incr failed for hour key");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;
    if hour_count == 1 {
        let _: () = conn.expire(&hour_key, 3600).await.map_err(|e| {
            tracing::error!(error = %e, "redis expire failed for hour key");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
    }
    if hour_count > rph_limit {
        return Err(StatusCode::TOO_MANY_REQUESTS);
    }

    Ok(())
}
