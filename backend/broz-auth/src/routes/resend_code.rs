use axum::extract::State;
use axum::Json;
use diesel::prelude::*;
use std::sync::Arc;

use broz_shared::errors::{AppError, AppResult, ErrorCode};
use broz_shared::types::auth::AuthUser;
use broz_shared::types::ApiResponse;

use crate::models::NewEmailVerification;
use crate::schema::{credentials, email_verifications};
use crate::services::auth_service;
use crate::AppState;

pub async fn resend_code(
    user: AuthUser,
    State(state): State<Arc<AppState>>,
) -> AppResult<Json<ApiResponse<&'static str>>> {
    let mut conn = state.db.get().map_err(|e| AppError::internal(e.to_string()))?;

    let credential = credentials::table
        .filter(credentials::id.eq(user.id))
        .first::<crate::models::Credential>(&mut conn)
        .map_err(|_| AppError::new(ErrorCode::NotFound, "user not found"))?;

    if credential.email_verified {
        return Ok(Json(ApiResponse::ok("email already verified")));
    }

    // Rate limit: check Redis for 1 per minute
    let rate_key = format!("verify:rate:{}", credential.email);
    let allowed = state.redis.rate_limit_check(&rate_key, 1, 60).await
        .unwrap_or(true);

    if !allowed {
        return Err(AppError::new(ErrorCode::EmailRateLimited, "please wait before requesting a new code"));
    }

    let code = auth_service::generate_verification_code();
    let verification = NewEmailVerification {
        credential_id: credential.id,
        code: code.clone(),
        expires_at: chrono::Utc::now() + chrono::Duration::minutes(15),
    };
    diesel::insert_into(email_verifications::table)
        .values(&verification)
        .execute(&mut conn)?;

    if let Err(e) = state.email.send_verification_code(&credential.email, &code).await {
        tracing::error!(error = %e, "failed to send verification email");
    }

    Ok(Json(ApiResponse::ok("verification code sent")))
}
