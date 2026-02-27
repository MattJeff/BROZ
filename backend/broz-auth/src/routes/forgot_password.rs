use axum::extract::State;
use axum::Json;
use diesel::prelude::*;
use serde::Deserialize;
use std::sync::Arc;

use broz_shared::errors::{AppError, AppResult, ErrorCode};
use broz_shared::types::ApiResponse;

use crate::models::NewPasswordReset;
use crate::schema::{credentials, password_resets};
use crate::services::auth_service;
use crate::AppState;

#[derive(Debug, Deserialize)]
pub struct ForgotPasswordRequest {
    pub email: String,
}

pub async fn forgot_password(
    State(state): State<Arc<AppState>>,
    Json(req): Json<ForgotPasswordRequest>,
) -> AppResult<Json<ApiResponse<&'static str>>> {
    let mut conn = state.db.get().map_err(|e| AppError::internal(e.to_string()))?;

    // Rate limit
    let rate_key = format!("verify:rate:{}", req.email.to_lowercase());
    let allowed = state.redis.rate_limit_check(&rate_key, 1, 60).await.unwrap_or(true);
    if !allowed {
        return Err(AppError::new(ErrorCode::EmailRateLimited, "please wait before requesting a new code"));
    }

    // Find credential (don't reveal if email exists)
    let credential = credentials::table
        .filter(credentials::email.eq(req.email.to_lowercase()))
        .first::<crate::models::Credential>(&mut conn);

    if let Ok(cred) = credential {
        let code = auth_service::generate_verification_code();
        let reset = NewPasswordReset {
            credential_id: cred.id,
            code: code.clone(),
            expires_at: chrono::Utc::now() + chrono::Duration::minutes(15),
        };
        diesel::insert_into(password_resets::table)
            .values(&reset)
            .execute(&mut conn)?;

        if let Err(e) = state.email.send_password_reset_code(&cred.email, &code).await {
            tracing::error!(error = %e, "failed to send reset email");
        }
    }

    // Always return success to prevent email enumeration
    Ok(Json(ApiResponse::ok("if the email exists, a reset code has been sent")))
}
