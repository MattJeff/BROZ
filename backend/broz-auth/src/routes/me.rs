use axum::extract::State;
use axum::Json;
use diesel::prelude::*;
use serde::Serialize;
use std::sync::Arc;

use broz_shared::errors::{AppError, AppResult, ErrorCode};
use broz_shared::types::auth::AuthUser;
use broz_shared::types::ApiResponse;

use crate::schema::credentials;
use crate::AppState;

#[derive(Debug, Serialize)]
pub struct MeResponse {
    pub id: uuid::Uuid,
    pub email: String,
    pub email_verified: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

pub async fn me(
    user: AuthUser,
    State(state): State<Arc<AppState>>,
) -> AppResult<Json<ApiResponse<MeResponse>>> {
    let mut conn = state.db.get().map_err(|e| AppError::internal(e.to_string()))?;

    let credential = credentials::table
        .filter(credentials::id.eq(user.id))
        .first::<crate::models::Credential>(&mut conn)
        .map_err(|_| AppError::new(ErrorCode::NotFound, "user not found"))?;

    Ok(Json(ApiResponse::ok(MeResponse {
        id: credential.id,
        email: credential.email,
        email_verified: credential.email_verified,
        created_at: credential.created_at,
    })))
}
