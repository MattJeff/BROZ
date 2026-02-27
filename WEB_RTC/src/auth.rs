use std::collections::HashMap;
use std::sync::RwLock;
use std::time::{SystemTime, UNIX_EPOCH};

use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use rand::Rng;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::ApiError;

// ---------------------------------------------------------------------------
// API Keys
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKey {
    pub key: String,
    pub name: String,
    pub created_at: u64,
}

/// Generate an API key in the form `lr_` followed by 32 random hex characters.
pub fn generate_api_key() -> String {
    let mut rng = rand::thread_rng();
    let mut hex = String::with_capacity(32);
    for _ in 0..16 {
        let byte: u8 = rng.gen();
        hex.push_str(&format!("{:02x}", byte));
    }
    format!("lr_{}", hex)
}

// ---------------------------------------------------------------------------
// JWT Tokens
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenClaims {
    /// Peer ID (UUID).
    pub sub: String,
    pub room_id: String,
    /// One of "publish", "subscribe", or "call".
    pub role: String,
    /// Opaque key identifier (first 8 characters of the API key).
    pub key_id: String,
    /// Expiration (unix timestamp).
    pub exp: usize,
    /// Issued-at (unix timestamp).
    pub iat: usize,
}

/// Create a signed JWT for a new peer.
///
/// A fresh UUID is generated for the `sub` (peer_id) claim.
/// Only the first 8 characters of `api_key` are stored in the token
/// as `key_id` to avoid leaking the full key.
pub fn create_token(
    secret: &str,
    room_id: &str,
    role: &str,
    api_key: &str,
    ttl_secs: u64,
) -> Result<String, jsonwebtoken::errors::Error> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock before unix epoch")
        .as_secs();

    let claims = TokenClaims {
        sub: Uuid::new_v4().to_string(),
        room_id: room_id.to_string(),
        role: role.to_string(),
        key_id: api_key.chars().take(8).collect(),
        exp: (now + ttl_secs) as usize,
        iat: now as usize,
    };

    encode(
        &Header::default(), // HS256
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
}

/// Verify and decode a JWT, returning the inner claims.
pub fn verify_token(
    secret: &str,
    token: &str,
) -> Result<TokenClaims, jsonwebtoken::errors::Error> {
    let token_data = decode::<TokenClaims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &Validation::default(), // HS256 + exp validation
    )?;
    Ok(token_data.claims)
}

// ---------------------------------------------------------------------------
// Role validation
// ---------------------------------------------------------------------------

/// Returns `true` if the role is one of the allowed values.
pub fn validate_role(role: &str) -> bool {
    matches!(role, "publish" | "subscribe" | "call" | "conference")
}

// ---------------------------------------------------------------------------
// Axum helper -- API-key gate
// ---------------------------------------------------------------------------

/// Validate the `Authorization: Bearer lr_...` header against the known keys.
///
/// Returns the matching [`ApiKey`] or an [`ApiError`].
pub async fn require_api_key(
    headers: &axum::http::HeaderMap,
    api_keys: &RwLock<HashMap<String, ApiKey>>,
) -> Result<ApiKey, ApiError> {
    let auth = headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .ok_or_else(ApiError::auth_header_missing)?;

    let token = auth
        .strip_prefix("Bearer ")
        .ok_or_else(ApiError::auth_header_missing)?;

    let keys = api_keys.read().unwrap();
    keys.get(token).cloned().ok_or_else(ApiError::api_key_invalid)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn api_key_format() {
        let key = generate_api_key();
        assert!(key.starts_with("lr_"));
        // "lr_" (3 chars) + 32 hex chars = 35
        assert_eq!(key.len(), 35);
        assert!(key[3..].chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn roundtrip_token() {
        let secret = "test-secret";
        let token = create_token(secret, "room-1", "publish", "lr_abc", 3600).unwrap();
        let claims = verify_token(secret, &token).unwrap();

        assert_eq!(claims.room_id, "room-1");
        assert_eq!(claims.role, "publish");
        assert_eq!(claims.key_id, "lr_abc".chars().take(8).collect::<String>());
        assert!(!claims.sub.is_empty());
        assert!(claims.exp > claims.iat);
    }

    #[test]
    fn bad_secret_rejects() {
        let token = create_token("secret-a", "room-1", "call", "lr_x", 60).unwrap();
        assert!(verify_token("secret-b", &token).is_err());
    }

    #[test]
    fn validate_role_accepts_valid() {
        assert!(validate_role("publish"));
        assert!(validate_role("subscribe"));
        assert!(validate_role("call"));
    }

    #[test]
    fn validate_role_rejects_invalid() {
        assert!(!validate_role("admin"));
        assert!(!validate_role(""));
        assert!(!validate_role("PUBLISH"));
    }

    #[tokio::test]
    async fn require_api_key_success() {
        let api_keys: RwLock<HashMap<String, ApiKey>> = RwLock::new(HashMap::new());
        let key = generate_api_key();
        api_keys.write().unwrap().insert(
            key.clone(),
            ApiKey {
                key: key.clone(),
                name: "test".into(),
                created_at: 0,
            },
        );

        let mut headers = axum::http::HeaderMap::new();
        headers.insert(
            "authorization",
            format!("Bearer {}", key).parse().unwrap(),
        );

        let result = require_api_key(&headers, &api_keys).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().key, key);
    }

    #[tokio::test]
    async fn require_api_key_missing_header() {
        let api_keys: RwLock<HashMap<String, ApiKey>> = RwLock::new(HashMap::new());
        let headers = axum::http::HeaderMap::new();

        let result = require_api_key(&headers, &api_keys).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn require_api_key_unknown_key() {
        let api_keys: RwLock<HashMap<String, ApiKey>> = RwLock::new(HashMap::new());
        let mut headers = axum::http::HeaderMap::new();
        headers.insert(
            "authorization",
            "Bearer lr_does_not_exist".parse().unwrap(),
        );

        let result = require_api_key(&headers, &api_keys).await;
        assert!(result.is_err());
    }
}
