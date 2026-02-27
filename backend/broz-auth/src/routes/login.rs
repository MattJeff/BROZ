use axum::extract::State;
use axum::Json;
use diesel::prelude::*;
use serde::Deserialize;
use std::sync::Arc;

use broz_shared::errors::{AppError, AppResult, ErrorCode};
use broz_shared::types::auth::{TokenPair, UserRole};
use broz_shared::types::ApiResponse;

use crate::models::{Credential, NewRefreshToken};
use crate::schema::{credentials, refresh_tokens};
use crate::services::{auth_service, token_service};
use crate::AppState;

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
    pub device_fingerprint: Option<String>,
}

pub async fn login(
    State(state): State<Arc<AppState>>,
    Json(req): Json<LoginRequest>,
) -> AppResult<Json<ApiResponse<TokenPair>>> {
    let mut conn = state.db.get().map_err(|e| AppError::internal(e.to_string()))?;

    let credential: Credential = credentials::table
        .filter(credentials::email.eq(req.email.to_lowercase()))
        .first(&mut conn)
        .map_err(|_| AppError::new(ErrorCode::InvalidCredentials, "invalid email or password"))?;

    // Check ban
    if credential.is_banned {
        if let Some(ban_until) = credential.ban_until {
            if ban_until > chrono::Utc::now() {
                return Err(AppError::new(ErrorCode::UserBanned, format!("account banned until {}", ban_until.format("%Y-%m-%d %H:%M UTC"))));
            }
            // Ban expired, unban
            diesel::update(credentials::table.filter(credentials::id.eq(credential.id)))
                .set((credentials::is_banned.eq(false), credentials::ban_until.eq(None::<chrono::DateTime<chrono::Utc>>)))
                .execute(&mut conn)?;
        } else {
            return Err(AppError::new(ErrorCode::UserBanned, "account permanently banned"));
        }
    }

    let valid = auth_service::verify_password(&req.password, &credential.password_hash)?;
    if !valid {
        return Err(AppError::new(ErrorCode::InvalidCredentials, "invalid email or password"));
    }

    let role = credential.role.parse::<UserRole>().unwrap_or(UserRole::User);

    let (token_pair, refresh_hash) = token_service::create_token_pair(
        credential.id,
        role,
        &state.config.jwt_secret,
        state.config.jwt_access_ttl,
    )?;

    let new_rt = NewRefreshToken {
        credential_id: credential.id,
        token_hash: refresh_hash,
        device_fingerprint: req.device_fingerprint,
        expires_at: chrono::Utc::now() + chrono::Duration::seconds(state.config.jwt_refresh_ttl),
    };
    diesel::insert_into(refresh_tokens::table)
        .values(&new_rt)
        .execute(&mut conn)?;

    tracing::info!(user_id = %credential.id, "user logged in");

    Ok(Json(ApiResponse::ok(token_pair)))
}
