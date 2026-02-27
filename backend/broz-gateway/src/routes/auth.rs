use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use broz_shared::{ApiErrorResponse, UserRole};
use jsonwebtoken::{decode, Algorithm, DecodingKey, Validation};
use uuid::Uuid;

/// Authenticated user information extracted from the JWT.
#[derive(Debug, Clone)]
pub struct AuthInfo {
    pub user_id: Uuid,
    pub role: UserRole,
}

/// Extract and validate the JWT from request headers.
///
/// Returns `AuthInfo` on success, or an error `Response` mapped to the appropriate
/// BROZ error codes:
/// - E0004 (Unauthorized): missing or malformed Authorization header
/// - E0005 (Forbidden): token expired
/// - E0006 (RateLimited): (unused here, but reserved)
pub fn extract_auth_user(headers: &HeaderMap, jwt_secret: &str) -> Result<AuthInfo, Response> {
    let auth_header = headers
        .get("Authorization")
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| {
            (
                StatusCode::UNAUTHORIZED,
                Json(ApiErrorResponse::new("E0004", "missing authorization header")),
            )
                .into_response()
        })?;

    if !auth_header.starts_with("Bearer ") {
        return Err((
            StatusCode::UNAUTHORIZED,
            Json(ApiErrorResponse::new("E0004", "authorization header must use Bearer scheme")),
        )
            .into_response());
    }

    let token = &auth_header[7..];

    let mut validation = Validation::new(Algorithm::HS256);
    validation.validate_exp = true;

    let token_data = decode::<broz_shared::Claims>(
        token,
        &DecodingKey::from_secret(jwt_secret.as_bytes()),
        &validation,
    )
    .map_err(|e| match e.kind() {
        jsonwebtoken::errors::ErrorKind::ExpiredSignature => (
            StatusCode::UNAUTHORIZED,
            Json(ApiErrorResponse::new("E0005", "token has expired")),
        )
            .into_response(),
        _ => (
            StatusCode::UNAUTHORIZED,
            Json(ApiErrorResponse::new("E0006", format!("invalid token: {e}"))),
        )
            .into_response(),
    })?;

    Ok(AuthInfo {
        user_id: token_data.claims.sub,
        role: token_data.claims.role,
    })
}
