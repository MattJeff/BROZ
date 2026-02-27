use axum::extract::State;
use axum::Json;
use diesel::prelude::*;
use serde::Deserialize;
use std::sync::Arc;

use broz_shared::errors::{AppError, AppResult};
use broz_shared::types::ApiResponse;

use crate::schema::refresh_tokens;
use crate::services::token_service;
use crate::AppState;

#[derive(Debug, Deserialize)]
pub struct LogoutRequest {
    pub refresh_token: String,
}

pub async fn logout(
    State(state): State<Arc<AppState>>,
    Json(req): Json<LogoutRequest>,
) -> AppResult<Json<ApiResponse<&'static str>>> {
    let token_hash = token_service::hash_token(&req.refresh_token);
    let mut conn = state.db.get().map_err(|e| AppError::internal(e.to_string()))?;

    diesel::update(
        refresh_tokens::table
            .filter(refresh_tokens::token_hash.eq(&token_hash))
            .filter(refresh_tokens::revoked_at.is_null()),
    )
    .set(refresh_tokens::revoked_at.eq(Some(chrono::Utc::now())))
    .execute(&mut conn)?;

    Ok(Json(ApiResponse::ok("logged out")))
}
