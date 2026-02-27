use axum::extract::{Query, State};
use axum::Json;
use diesel::prelude::*;
use serde::Deserialize;
use std::sync::Arc;

use broz_shared::errors::{AppError, AppResult};
use broz_shared::types::auth::AuthUser;
use broz_shared::types::ApiResponse;

use crate::models::Profile;
use crate::schema::profiles;
use crate::AppState;

#[derive(Deserialize)]
pub struct SearchParams {
    q: String,
    #[serde(default = "default_limit")]
    limit: i64,
}

fn default_limit() -> i64 {
    20
}

/// GET /search?q=<query>&limit=20
pub async fn search_users(
    user: AuthUser,
    State(state): State<Arc<AppState>>,
    Query(params): Query<SearchParams>,
) -> AppResult<Json<ApiResponse<Vec<Profile>>>> {
    let query = params.q.trim().to_string();
    if query.is_empty() {
        return Ok(Json(ApiResponse::ok(vec![])));
    }

    let limit = params.limit.clamp(1, 50);
    let pattern = format!("%{}%", query);

    let mut conn = state.db.get().map_err(|e| AppError::internal(e.to_string()))?;

    let mut results = profiles::table
        .filter(profiles::display_name.ilike(&pattern))
        .filter(profiles::credential_id.ne(user.id))
        .filter(profiles::onboarding_complete.eq(true))
        .limit(limit)
        .load::<Profile>(&mut conn)
        .map_err(|e| AppError::internal(e.to_string()))?;

    // Enrich is_online from Redis (source of truth for presence)
    for p in &mut results {
        let key = format!("online:{}", p.credential_id);
        if let Ok(true) = state.redis.exists(&key).await {
            p.is_online = true;
        }
    }

    Ok(Json(ApiResponse::ok(results)))
}
