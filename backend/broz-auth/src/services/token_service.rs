use jsonwebtoken::{encode, EncodingKey, Header};
use rand::Rng;
use sha2::{Digest, Sha256};
use uuid::Uuid;

use broz_shared::errors::AppError;
use broz_shared::types::auth::{Claims, TokenPair, UserRole};

pub fn create_access_token(
    user_id: Uuid,
    role: UserRole,
    secret: &str,
    ttl_secs: i64,
) -> Result<String, AppError> {
    let claims = Claims::new(user_id, role, ttl_secs);
    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .map_err(|e| AppError::internal(format!("JWT encoding failed: {e}")))
}

pub fn create_refresh_token() -> String {
    let mut rng = rand::thread_rng();
    let bytes: [u8; 32] = rng.gen();
    hex::encode(bytes)
}

pub fn hash_token(token: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(token.as_bytes());
    hex::encode(hasher.finalize())
}

pub fn create_token_pair(
    user_id: Uuid,
    role: UserRole,
    secret: &str,
    access_ttl: i64,
) -> Result<(TokenPair, String), AppError> {
    let access_token = create_access_token(user_id, role, secret, access_ttl)?;
    let refresh_token = create_refresh_token();
    let refresh_hash = hash_token(&refresh_token);
    let pair = TokenPair::new(access_token, refresh_token, access_ttl);
    Ok((pair, refresh_hash))
}
