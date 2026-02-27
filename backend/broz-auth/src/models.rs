use chrono::{DateTime, Utc};
use diesel::prelude::*;
use serde::Serialize;
use uuid::Uuid;

use crate::schema::{credentials, email_verifications, oauth_accounts, password_resets, refresh_tokens};

// --- Credentials ---

#[derive(Debug, Queryable, Identifiable, Serialize)]
#[diesel(table_name = credentials)]
pub struct Credential {
    pub id: Uuid,
    pub email: String,
    #[serde(skip_serializing)]
    pub password_hash: String,
    pub email_verified: bool,
    pub is_banned: bool,
    pub ban_until: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub role: String,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = credentials)]
pub struct NewCredential {
    pub email: String,
    pub password_hash: String,
}

// --- OAuth Accounts ---

#[derive(Debug, Queryable, Identifiable, Serialize)]
#[diesel(table_name = oauth_accounts)]
pub struct OAuthAccount {
    pub id: Uuid,
    pub credential_id: Uuid,
    pub provider: String,
    pub provider_uid: String,
    pub access_token_enc: Option<String>,
    pub refresh_token_enc: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = oauth_accounts)]
pub struct NewOAuthAccount {
    pub credential_id: Uuid,
    pub provider: String,
    pub provider_uid: String,
    pub access_token_enc: Option<String>,
    pub refresh_token_enc: Option<String>,
}

// --- Email Verifications ---

#[derive(Debug, Queryable, Identifiable, Serialize)]
#[diesel(table_name = email_verifications)]
pub struct EmailVerification {
    pub id: Uuid,
    pub credential_id: Uuid,
    pub code: String,
    pub expires_at: DateTime<Utc>,
    pub used_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = email_verifications)]
pub struct NewEmailVerification {
    pub credential_id: Uuid,
    pub code: String,
    pub expires_at: DateTime<Utc>,
}

// --- Password Resets ---

#[derive(Debug, Queryable, Identifiable, Serialize)]
#[diesel(table_name = password_resets)]
pub struct PasswordReset {
    pub id: Uuid,
    pub credential_id: Uuid,
    pub code: String,
    pub expires_at: DateTime<Utc>,
    pub used_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = password_resets)]
pub struct NewPasswordReset {
    pub credential_id: Uuid,
    pub code: String,
    pub expires_at: DateTime<Utc>,
}

// --- Refresh Tokens ---

#[derive(Debug, Queryable, Identifiable, Serialize)]
#[diesel(table_name = refresh_tokens)]
pub struct RefreshToken {
    pub id: Uuid,
    pub credential_id: Uuid,
    pub token_hash: String,
    pub device_fingerprint: Option<String>,
    pub expires_at: DateTime<Utc>,
    pub revoked_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = refresh_tokens)]
pub struct NewRefreshToken {
    pub credential_id: Uuid,
    pub token_hash: String,
    pub device_fingerprint: Option<String>,
    pub expires_at: DateTime<Utc>,
}
