use std::sync::Arc;
use axum::extract::{Query, State};
use axum::Json;
use chrono::{NaiveDate, Utc};
use diesel::prelude::*;
use serde::{Deserialize, Serialize};

use broz_shared::errors::{AppError, AppResult};
use broz_shared::middleware::AdminUser;
use broz_shared::types::pagination::{PaginationParams, Paginated};

use crate::AppState;
use crate::models::{AnalyticsEvent, DailyStat};
use crate::schema::{analytics_events, daily_stats};

// --- Overview ---

#[derive(Debug, Serialize)]
pub struct StatsOverview {
    pub dau: i64,
    pub wau: i64,
    pub mau: i64,
    pub registrations_today: i64,
    pub matches_today: i64,
}

/// GET /stats/overview
/// Returns today's aggregated metrics from the daily_stats table.
/// Requires AdminUser.
pub async fn get_overview(
    _admin: AdminUser,
    State(state): State<Arc<AppState>>,
) -> AppResult<Json<broz_shared::types::api::ApiResponse<StatsOverview>>> {
    let mut conn = state.db.get()
        .map_err(|e| AppError::internal(format!("db pool error: {e}")))?;

    let today = Utc::now().date_naive();

    let stats: Vec<DailyStat> = daily_stats::table
        .filter(daily_stats::date.eq(today))
        .load(&mut conn)?;

    let get_metric = |name: &str| -> i64 {
        stats.iter()
            .find(|s| s.metric == name)
            .map(|s| s.value)
            .unwrap_or(0)
    };

    let overview = StatsOverview {
        dau: get_metric("dau"),
        wau: get_metric("wau"),
        mau: get_metric("mau"),
        registrations_today: get_metric("registrations_today"),
        matches_today: get_metric("matches_today"),
    };

    Ok(Json(broz_shared::types::api::ApiResponse::ok(overview)))
}

// --- Daily Stats by Date Range ---

#[derive(Debug, Deserialize)]
pub struct DateRangeQuery {
    /// Start date in YYYY-MM-DD format
    pub from: String,
    /// End date in YYYY-MM-DD format
    pub to: String,
}

/// GET /stats/daily?from=2025-01-01&to=2025-01-31
/// Returns all daily_stats rows within the given date range.
/// Requires AdminUser.
pub async fn get_daily_stats(
    _admin: AdminUser,
    State(state): State<Arc<AppState>>,
    Query(query): Query<DateRangeQuery>,
) -> AppResult<Json<broz_shared::types::api::ApiResponse<Vec<DailyStat>>>> {
    let from = NaiveDate::parse_from_str(&query.from, "%Y-%m-%d")
        .map_err(|_| AppError::bad_request("invalid 'from' date format, expected YYYY-MM-DD"))?;
    let to = NaiveDate::parse_from_str(&query.to, "%Y-%m-%d")
        .map_err(|_| AppError::bad_request("invalid 'to' date format, expected YYYY-MM-DD"))?;

    if from > to {
        return Err(AppError::bad_request("'from' date must be before or equal to 'to' date"));
    }

    let mut conn = state.db.get()
        .map_err(|e| AppError::internal(format!("db pool error: {e}")))?;

    let stats: Vec<DailyStat> = daily_stats::table
        .filter(daily_stats::date.ge(from))
        .filter(daily_stats::date.le(to))
        .order(daily_stats::date.asc())
        .load(&mut conn)?;

    Ok(Json(broz_shared::types::api::ApiResponse::ok(stats)))
}

// --- Paginated Events ---

/// GET /stats/events?page=1&per_page=20
/// Returns a paginated list of recent analytics events.
/// Requires AdminUser.
pub async fn get_events(
    _admin: AdminUser,
    State(state): State<Arc<AppState>>,
    Query(params): Query<PaginationParams>,
) -> AppResult<Json<broz_shared::types::api::ApiResponse<Paginated<AnalyticsEvent>>>> {
    let mut conn = state.db.get()
        .map_err(|e| AppError::internal(format!("db pool error: {e}")))?;

    let total: i64 = analytics_events::table
        .count()
        .get_result(&mut conn)?;

    let events: Vec<AnalyticsEvent> = analytics_events::table
        .order(analytics_events::created_at.desc())
        .offset(params.offset() as i64)
        .limit(params.limit() as i64)
        .load(&mut conn)?;

    let paginated = Paginated::new(events, total as u64, &params);

    Ok(Json(broz_shared::types::api::ApiResponse::ok(paginated)))
}
