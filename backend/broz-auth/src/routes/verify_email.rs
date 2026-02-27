use axum::extract::State;
use axum::Json;
use diesel::prelude::*;
use serde::Deserialize;
use std::sync::Arc;

use broz_shared::errors::{AppError, AppResult, ErrorCode};
use broz_shared::types::auth::AuthUser;
use broz_shared::types::ApiResponse;

use crate::schema::{credentials, email_verifications};
use crate::AppState;

#[derive(Debug, Deserialize)]
pub struct VerifyEmailRequest {
    pub code: String,
}

pub async fn verify_email(
    user: AuthUser,
    State(state): State<Arc<AppState>>,
    Json(req): Json<VerifyEmailRequest>,
) -> AppResult<Json<ApiResponse<&'static str>>> {
    let mut conn = state.db.get().map_err(|e| AppError::internal(e.to_string()))?;

    // Find the latest unused verification code for this user
    let verification = email_verifications::table
        .filter(email_verifications::credential_id.eq(user.id))
        .filter(email_verifications::used_at.is_null())
        .filter(email_verifications::code.eq(&req.code))
        .order(email_verifications::created_at.desc())
        .first::<crate::models::EmailVerification>(&mut conn)
        .map_err(|_| AppError::new(ErrorCode::VerificationCodeInvalid, "invalid verification code"))?;

    if verification.expires_at < chrono::Utc::now() {
        return Err(AppError::new(ErrorCode::VerificationCodeExpired, "verification code expired"));
    }

    // Mark code as used
    diesel::update(email_verifications::table.filter(email_verifications::id.eq(verification.id)))
        .set(email_verifications::used_at.eq(Some(chrono::Utc::now())))
        .execute(&mut conn)?;

    // Set email_verified = true
    diesel::update(credentials::table.filter(credentials::id.eq(user.id)))
        .set(credentials::email_verified.eq(true))
        .execute(&mut conn)?;

    tracing::info!(user_id = %user.id, "email verified");

    Ok(Json(ApiResponse::ok("email verified")))
}
