use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use rand::Rng;

use broz_shared::errors::{AppError, ErrorCode};

pub fn hash_password(password: &str) -> Result<String, AppError> {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    argon2
        .hash_password(password.as_bytes(), &salt)
        .map(|h| h.to_string())
        .map_err(|e| AppError::internal(format!("password hashing failed: {e}")))
}

pub fn verify_password(password: &str, hash: &str) -> Result<bool, AppError> {
    let parsed_hash = PasswordHash::new(hash)
        .map_err(|e| AppError::internal(format!("invalid password hash: {e}")))?;
    Ok(Argon2::default()
        .verify_password(password.as_bytes(), &parsed_hash)
        .is_ok())
}

pub fn validate_password(password: &str) -> Result<(), AppError> {
    if password.len() < 8 {
        return Err(AppError::new(ErrorCode::PasswordTooWeak, "password must be at least 8 characters"));
    }
    if !password.chars().any(|c| c.is_ascii_digit()) {
        return Err(AppError::new(ErrorCode::PasswordTooWeak, "password must contain at least one number"));
    }
    if !password.chars().any(|c| c.is_ascii_alphabetic()) {
        return Err(AppError::new(ErrorCode::PasswordTooWeak, "password must contain at least one letter"));
    }
    Ok(())
}

pub fn generate_verification_code() -> String {
    let mut rng = rand::thread_rng();
    format!("{:06}", rng.gen_range(0..1_000_000))
}
