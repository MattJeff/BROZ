use axum::extract::State;
use axum::Json;
use diesel::prelude::*;
use serde::Deserialize;
use std::sync::Arc;
use validator::Validate;

use broz_shared::errors::{AppError, AppResult, ErrorCode};
use broz_shared::types::auth::{TokenPair, UserRole};
use broz_shared::types::ApiResponse;

use crate::models::{NewCredential, NewEmailVerification};
use crate::schema::{credentials, email_verifications};
use crate::services::{auth_service, token_service};
use crate::AppState;

#[derive(Debug, Deserialize, Validate)]
pub struct RegisterRequest {
    #[validate(email(message = "invalid email format"))]
    pub email: String,
    pub password: String,
}

pub async fn register(
    State(state): State<Arc<AppState>>,
    Json(req): Json<RegisterRequest>,
) -> AppResult<Json<ApiResponse<TokenPair>>> {
    req.validate()
        .map_err(|e| AppError::new(ErrorCode::ValidationError, e.to_string()))?;

    auth_service::validate_password(&req.password)?;

    let password_hash = auth_service::hash_password(&req.password)?;
    let mut conn = state.db.get().map_err(|e| AppError::internal(e.to_string()))?;

    // Check if email already exists
    let exists: bool = credentials::table
        .filter(credentials::email.eq(&req.email.to_lowercase()))
        .count()
        .get_result::<i64>(&mut conn)
        .map(|c| c > 0)
        .unwrap_or(false);

    if exists {
        return Err(AppError::new(ErrorCode::EmailAlreadyExists, "email already registered"));
    }

    let new_cred = NewCredential {
        email: req.email.to_lowercase(),
        password_hash,
    };

    let credential: crate::models::Credential = diesel::insert_into(credentials::table)
        .values(&new_cred)
        .get_result(&mut conn)?;

    // Generate verification code
    let code = auth_service::generate_verification_code();
    let verification = NewEmailVerification {
        credential_id: credential.id,
        code: code.clone(),
        expires_at: chrono::Utc::now() + chrono::Duration::minutes(15),
    };
    diesel::insert_into(email_verifications::table)
        .values(&verification)
        .execute(&mut conn)?;

    // Send verification email
    if let Err(e) = state.email.send_verification_code(&credential.email, &code).await {
        tracing::error!(error = %e, "failed to send verification email");
    }

    // Create token pair
    let (token_pair, refresh_hash) = token_service::create_token_pair(
        credential.id,
        UserRole::User,
        &state.config.jwt_secret,
        state.config.jwt_access_ttl,
    )?;

    // Store refresh token
    use crate::models::NewRefreshToken;
    use crate::schema::refresh_tokens;

    let new_rt = NewRefreshToken {
        credential_id: credential.id,
        token_hash: refresh_hash,
        device_fingerprint: None,
        expires_at: chrono::Utc::now() + chrono::Duration::seconds(state.config.jwt_refresh_ttl),
    };
    diesel::insert_into(refresh_tokens::table)
        .values(&new_rt)
        .execute(&mut conn)?;

    // Publish registration event
    crate::events::publisher::publish_user_registered(&state.rabbitmq, credential.id, &credential.email).await;

    tracing::info!(user_id = %credential.id, email = %credential.email, "user registered");

    Ok(Json(ApiResponse::ok(token_pair)))
}
