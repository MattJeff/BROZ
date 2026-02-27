use axum::extract::State;
use axum::Json;
use diesel::prelude::*;
use serde::Deserialize;
use std::sync::Arc;

use broz_shared::errors::{AppError, AppResult, ErrorCode};
use broz_shared::types::auth::{TokenPair, UserRole};
use broz_shared::types::ApiResponse;

use crate::models::{Credential, RefreshToken, NewRefreshToken};
use crate::schema::{credentials, refresh_tokens};
use crate::services::token_service;
use crate::AppState;

#[derive(Debug, Deserialize)]
pub struct RefreshRequest {
    pub refresh_token: String,
}

pub async fn refresh_token(
    State(state): State<Arc<AppState>>,
    Json(req): Json<RefreshRequest>,
) -> AppResult<Json<ApiResponse<TokenPair>>> {
    let token_hash = token_service::hash_token(&req.refresh_token);
    let mut conn = state.db.get().map_err(|e| AppError::internal(e.to_string()))?;

    let stored: RefreshToken = refresh_tokens::table
        .filter(refresh_tokens::token_hash.eq(&token_hash))
        .filter(refresh_tokens::revoked_at.is_null())
        .first(&mut conn)
        .map_err(|_| AppError::new(ErrorCode::TokenInvalid, "invalid refresh token"))?;

    if stored.expires_at < chrono::Utc::now() {
        return Err(AppError::new(ErrorCode::TokenExpired, "refresh token expired"));
    }

    // Revoke old token
    diesel::update(refresh_tokens::table.filter(refresh_tokens::id.eq(stored.id)))
        .set(refresh_tokens::revoked_at.eq(Some(chrono::Utc::now())))
        .execute(&mut conn)?;

    // Fetch credential to get role
    let credential: Credential = credentials::table
        .find(stored.credential_id)
        .first(&mut conn)
        .map_err(|_| AppError::new(ErrorCode::InvalidCredentials, "credential not found"))?;
    let role = credential.role.parse::<UserRole>().unwrap_or(UserRole::User);

    // Issue new token pair
    let (token_pair, new_hash) = token_service::create_token_pair(
        stored.credential_id,
        role,
        &state.config.jwt_secret,
        state.config.jwt_access_ttl,
    )?;

    let new_rt = NewRefreshToken {
        credential_id: stored.credential_id,
        token_hash: new_hash,
        device_fingerprint: stored.device_fingerprint,
        expires_at: chrono::Utc::now() + chrono::Duration::seconds(state.config.jwt_refresh_ttl),
    };
    diesel::insert_into(refresh_tokens::table)
        .values(&new_rt)
        .execute(&mut conn)?;

    Ok(Json(ApiResponse::ok(token_pair)))
}
