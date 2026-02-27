use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use axum::http::HeaderMap;
use jsonwebtoken::{decode, DecodingKey, Validation, Algorithm};

use crate::errors::{AppError, ErrorCode};
use crate::types::auth::{AuthUser, Claims, UserRole};

#[axum::async_trait]
impl<S> FromRequestParts<S> for AuthUser
where
    S: Send + Sync,
{
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let token = extract_bearer_token(&parts.headers)?;
        let claims = validate_jwt(&token)?;

        if claims.is_expired() {
            return Err(AppError::new(ErrorCode::TokenExpired, "token has expired"));
        }

        Ok(AuthUser::from(claims))
    }
}

fn extract_bearer_token(headers: &HeaderMap) -> Result<String, AppError> {
    let auth_header = headers
        .get("Authorization")
        .ok_or_else(|| AppError::new(ErrorCode::Unauthorized, "missing authorization header"))?
        .to_str()
        .map_err(|_| AppError::new(ErrorCode::Unauthorized, "invalid authorization header"))?;

    if !auth_header.starts_with("Bearer ") {
        return Err(AppError::new(ErrorCode::Unauthorized, "authorization header must use Bearer scheme"));
    }

    Ok(auth_header[7..].to_string())
}

fn validate_jwt(token: &str) -> Result<Claims, AppError> {
    let jwt_secret = std::env::var("JWT_SECRET")
        .unwrap_or_else(|_| "development-secret-change-in-production".to_string());

    let mut validation = Validation::new(Algorithm::HS256);
    validation.validate_exp = true;

    let token_data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(jwt_secret.as_bytes()),
        &validation,
    )
    .map_err(|e| match e.kind() {
        jsonwebtoken::errors::ErrorKind::ExpiredSignature => {
            AppError::new(ErrorCode::TokenExpired, "token has expired")
        }
        _ => AppError::new(ErrorCode::TokenInvalid, format!("invalid token: {e}")),
    })?;

    Ok(token_data.claims)
}

/// Optional auth extractor
pub struct OptionalAuthUser(pub Option<AuthUser>);

#[axum::async_trait]
impl<S> FromRequestParts<S> for OptionalAuthUser
where
    S: Send + Sync,
{
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        match AuthUser::from_request_parts(parts, state).await {
            Ok(user) => Ok(Self(Some(user))),
            Err(_) => Ok(Self(None)),
        }
    }
}

/// Require Admin role
pub struct AdminUser(pub AuthUser);

#[axum::async_trait]
impl<S> FromRequestParts<S> for AdminUser
where
    S: Send + Sync,
{
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let user = AuthUser::from_request_parts(parts, state).await?;
        if user.role != UserRole::Admin {
            return Err(AppError::new(ErrorCode::Forbidden, "admin access required"));
        }
        Ok(Self(user))
    }
}

/// Require Moderator or Admin role
pub struct ModeratorUser(pub AuthUser);

#[axum::async_trait]
impl<S> FromRequestParts<S> for ModeratorUser
where
    S: Send + Sync,
{
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let user = AuthUser::from_request_parts(parts, state).await?;
        if !matches!(user.role, UserRole::Moderator | UserRole::Admin) {
            return Err(AppError::new(ErrorCode::Forbidden, "moderator access required"));
        }
        Ok(Self(user))
    }
}
