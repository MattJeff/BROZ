use axum::extract::{Multipart, State};
use axum::Json;
use diesel::prelude::*;
use serde::Serialize;
use std::sync::Arc;
use uuid::Uuid;

use broz_shared::errors::{AppError, AppResult, ErrorCode};
use broz_shared::types::auth::AuthUser;
use broz_shared::types::ApiResponse;

use crate::models::Profile;
use crate::schema::profiles;
use crate::AppState;

#[derive(Debug, Serialize)]
pub struct PhotoUploadResponse {
    pub photo_url: String,
}

pub async fn upload_photo(
    user: AuthUser,
    State(state): State<Arc<AppState>>,
    mut multipart: Multipart,
) -> AppResult<Json<ApiResponse<PhotoUploadResponse>>> {
    let mut conn = state.db.get().map_err(|e| AppError::internal(e.to_string()))?;

    // Get user profile
    let profile = profiles::table
        .filter(profiles::credential_id.eq(user.id))
        .first::<Profile>(&mut conn)
        .map_err(|_| AppError::new(ErrorCode::ProfileNotFound, "profile not found"))?;

    // Read the file from multipart
    let field = multipart
        .next_field()
        .await
        .map_err(|e| AppError::new(ErrorCode::PhotoUploadFailed, format!("failed to read multipart: {e}")))?
        .ok_or_else(|| AppError::new(ErrorCode::PhotoUploadFailed, "no file provided"))?;

    let content_type = field
        .content_type()
        .unwrap_or("application/octet-stream")
        .to_string();

    // Determine file extension from content type
    let ext = match content_type.as_str() {
        "image/jpeg" | "image/jpg" => "jpg",
        "image/png" => "png",
        "image/webp" => "webp",
        "image/gif" => "gif",
        _ => {
            return Err(AppError::new(
                ErrorCode::PhotoUploadFailed,
                "unsupported image format, accepted: jpeg, png, webp, gif",
            ));
        }
    };

    let data = field
        .bytes()
        .await
        .map_err(|e| AppError::new(ErrorCode::PhotoUploadFailed, format!("failed to read file data: {e}")))?;

    // Upload to MinIO
    let file_id = Uuid::now_v7();
    let key = format!("profiles/{}/{}.{}", profile.id, file_id, ext);

    let photo_url = state
        .minio
        .upload(&key, data.to_vec(), &content_type)
        .await
        .map_err(|e| AppError::new(ErrorCode::PhotoUploadFailed, e))?;

    // Update profile photo URL
    diesel::update(profiles::table.filter(profiles::id.eq(profile.id)))
        .set((
            profiles::profile_photo_url.eq(&photo_url),
            profiles::updated_at.eq(chrono::Utc::now()),
        ))
        .execute(&mut conn)?;

    tracing::info!(
        profile_id = %profile.id,
        photo_url = %photo_url,
        "profile photo uploaded"
    );

    Ok(Json(ApiResponse::ok(PhotoUploadResponse { photo_url })))
}
