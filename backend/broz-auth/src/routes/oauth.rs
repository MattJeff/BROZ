use axum::extract::State;
use axum::Json;
use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use broz_shared::errors::{AppError, AppResult, ErrorCode};
use broz_shared::types::auth::{TokenPair, UserRole};
use broz_shared::types::ApiResponse;

use crate::models::{NewCredential, NewOAuthAccount, NewRefreshToken};
use crate::schema::{credentials, oauth_accounts, refresh_tokens};
use crate::services::token_service;
use crate::AppState;

#[derive(Debug, Deserialize)]
pub struct GoogleOAuthRequest {
    pub code: String,
}

#[derive(Debug, Deserialize)]
struct GoogleTokenResponse {
    access_token: String,
    #[allow(dead_code)]
    id_token: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GoogleUserInfo {
    #[serde(alias = "sub")]
    id: String,
    email: String,
    #[allow(dead_code)]
    name: Option<String>,
    #[allow(dead_code)]
    picture: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct OAuthResponse {
    #[serde(flatten)]
    pub tokens: TokenPair,
    pub is_new_user: bool,
}

pub async fn google_oauth(
    State(state): State<Arc<AppState>>,
    Json(req): Json<GoogleOAuthRequest>,
) -> AppResult<Json<ApiResponse<OAuthResponse>>> {
    // Exchange code for token
    let client = reqwest::Client::new();
    let token_response = client
        .post("https://oauth2.googleapis.com/token")
        .form(&[
            ("code", req.code.as_str()),
            ("client_id", &state.config.google_client_id),
            ("client_secret", &state.config.google_client_secret),
            ("redirect_uri", &state.config.google_redirect_uri),
            ("grant_type", "authorization_code"),
        ])
        .send()
        .await
        .map_err(|e| AppError::new(ErrorCode::OAuthError, format!("google token exchange failed: {e}")))?;

    if !token_response.status().is_success() {
        let body = token_response.text().await.unwrap_or_default();
        return Err(AppError::new(ErrorCode::OAuthError, format!("google token error: {body}")));
    }

    let google_token: GoogleTokenResponse = token_response.json().await
        .map_err(|e| AppError::new(ErrorCode::OAuthError, format!("invalid token response: {e}")))?;

    // Fetch user info
    let user_info_response = client
        .get("https://www.googleapis.com/oauth2/v3/userinfo")
        .bearer_auth(&google_token.access_token)
        .send()
        .await
        .map_err(|e| AppError::new(ErrorCode::OAuthError, format!("google userinfo failed: {e}")))?;

    let google_user: GoogleUserInfo = user_info_response.json().await
        .map_err(|e| AppError::new(ErrorCode::OAuthError, format!("invalid userinfo response: {e}")))?;

    let mut conn = state.db.get().map_err(|e| AppError::internal(e.to_string()))?;

    // Check if OAuth account already exists
    let existing_oauth = oauth_accounts::table
        .filter(oauth_accounts::provider.eq("google"))
        .filter(oauth_accounts::provider_uid.eq(&google_user.id))
        .first::<crate::models::OAuthAccount>(&mut conn)
        .optional()
        .map_err(|e| AppError::internal(e.to_string()))?;

    let (credential_id, is_new_user) = if let Some(oauth_account) = existing_oauth {
        (oauth_account.credential_id, false)
    } else {
        // Check if email exists
        let existing_cred = credentials::table
            .filter(credentials::email.eq(google_user.email.to_lowercase()))
            .first::<crate::models::Credential>(&mut conn)
            .optional()
            .map_err(|e| AppError::internal(e.to_string()))?;

        let cred_id = if let Some(ref cred) = existing_cred {
            cred.id
        } else {
            // Create new credential
            let new_cred = NewCredential {
                email: google_user.email.to_lowercase(),
                password_hash: "oauth_no_password".to_string(),
            };
            let cred: crate::models::Credential = diesel::insert_into(credentials::table)
                .values(&new_cred)
                .get_result(&mut conn)?;

            // Mark email as verified (Google verified it)
            diesel::update(credentials::table.filter(credentials::id.eq(cred.id)))
                .set(credentials::email_verified.eq(true))
                .execute(&mut conn)?;

            // Publish registration event
            crate::events::publisher::publish_user_registered(&state.rabbitmq, cred.id, &cred.email).await;

            cred.id
        };

        // Create OAuth account link
        let new_oauth = NewOAuthAccount {
            credential_id: cred_id,
            provider: "google".to_string(),
            provider_uid: google_user.id,
            access_token_enc: Some(google_token.access_token),
            refresh_token_enc: None,
        };
        diesel::insert_into(oauth_accounts::table)
            .values(&new_oauth)
            .execute(&mut conn)?;

        (cred_id, existing_cred.is_none())
    };

    // Create token pair
    let (token_pair, refresh_hash) = token_service::create_token_pair(
        credential_id,
        UserRole::User,
        &state.config.jwt_secret,
        state.config.jwt_access_ttl,
    )?;

    let new_rt = NewRefreshToken {
        credential_id,
        token_hash: refresh_hash,
        device_fingerprint: None,
        expires_at: chrono::Utc::now() + chrono::Duration::seconds(state.config.jwt_refresh_ttl),
    };
    diesel::insert_into(refresh_tokens::table)
        .values(&new_rt)
        .execute(&mut conn)?;

    tracing::info!(user_id = %credential_id, is_new = is_new_user, "google oauth login");

    Ok(Json(ApiResponse::ok(OAuthResponse {
        tokens: token_pair,
        is_new_user,
    })))
}
