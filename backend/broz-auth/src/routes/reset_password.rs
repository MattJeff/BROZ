use axum::extract::State;
use axum::Json;
use diesel::prelude::*;
use serde::Deserialize;
use std::sync::Arc;

use broz_shared::errors::{AppError, AppResult, ErrorCode};
use broz_shared::types::ApiResponse;

use crate::schema::{credentials, password_resets};
use crate::services::auth_service;
use crate::AppState;

#[derive(Debug, Deserialize)]
pub struct ResetPasswordRequest {
    pub email: String,
    pub code: String,
    pub new_password: String,
}

pub async fn reset_password(
    State(state): State<Arc<AppState>>,
    Json(req): Json<ResetPasswordRequest>,
) -> AppResult<Json<ApiResponse<&'static str>>> {
    auth_service::validate_password(&req.new_password)?;

    let mut conn = state.db.get().map_err(|e| AppError::internal(e.to_string()))?;

    let credential = credentials::table
        .filter(credentials::email.eq(req.email.to_lowercase()))
        .first::<crate::models::Credential>(&mut conn)
        .map_err(|_| AppError::new(ErrorCode::ResetCodeInvalid, "invalid reset code"))?;

    let reset = password_resets::table
        .filter(password_resets::credential_id.eq(credential.id))
        .filter(password_resets::code.eq(&req.code))
        .filter(password_resets::used_at.is_null())
        .order(password_resets::created_at.desc())
        .first::<crate::models::PasswordReset>(&mut conn)
        .map_err(|_| AppError::new(ErrorCode::ResetCodeInvalid, "invalid reset code"))?;

    if reset.expires_at < chrono::Utc::now() {
        return Err(AppError::new(ErrorCode::ResetCodeExpired, "reset code expired"));
    }

    // Mark code as used
    diesel::update(password_resets::table.filter(password_resets::id.eq(reset.id)))
        .set(password_resets::used_at.eq(Some(chrono::Utc::now())))
        .execute(&mut conn)?;

    // Update password
    let new_hash = auth_service::hash_password(&req.new_password)?;
    diesel::update(credentials::table.filter(credentials::id.eq(credential.id)))
        .set((
            credentials::password_hash.eq(new_hash),
            credentials::updated_at.eq(chrono::Utc::now()),
        ))
        .execute(&mut conn)?;

    // Revoke all refresh tokens for this user
    use crate::schema::refresh_tokens;
    diesel::update(
        refresh_tokens::table
            .filter(refresh_tokens::credential_id.eq(credential.id))
            .filter(refresh_tokens::revoked_at.is_null()),
    )
    .set(refresh_tokens::revoked_at.eq(Some(chrono::Utc::now())))
    .execute(&mut conn)?;

    tracing::info!(user_id = %credential.id, "password reset");

    Ok(Json(ApiResponse::ok("password reset successful")))
}
