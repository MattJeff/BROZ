use axum::extract::{Query, State};
use axum::Json;
use chrono::NaiveDate;
use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use broz_shared::errors::{AppError, AppResult, ErrorCode};
use broz_shared::types::auth::AuthUser;
use broz_shared::types::ApiResponse;

use crate::events::publisher;
use crate::models::{Profile, UpdateProfile};
use crate::schema::profiles;
use crate::AppState;

// --- GET /me ---

pub async fn get_profile(
    user: AuthUser,
    State(state): State<Arc<AppState>>,
) -> AppResult<Json<ApiResponse<Profile>>> {
    let mut conn = state.db.get().map_err(|e| AppError::internal(e.to_string()))?;

    let profile = profiles::table
        .filter(profiles::credential_id.eq(user.id))
        .first::<Profile>(&mut conn)
        .map_err(|_| AppError::new(ErrorCode::ProfileNotFound, "profile not found"))?;

    Ok(Json(ApiResponse::ok(profile)))
}

// --- PATCH /me ---

pub async fn update_profile(
    user: AuthUser,
    State(state): State<Arc<AppState>>,
    Json(payload): Json<UpdateProfile>,
) -> AppResult<Json<ApiResponse<Profile>>> {
    let mut conn = state.db.get().map_err(|e| AppError::internal(e.to_string()))?;

    let profile = profiles::table
        .filter(profiles::credential_id.eq(user.id))
        .first::<Profile>(&mut conn)
        .map_err(|_| AppError::new(ErrorCode::ProfileNotFound, "profile not found"))?;

    let updated = diesel::update(profiles::table.filter(profiles::id.eq(profile.id)))
        .set((
            &payload,
            profiles::updated_at.eq(chrono::Utc::now()),
        ))
        .get_result::<Profile>(&mut conn)?;

    publisher::publish_profile_updated(&state.rabbitmq, updated.id, updated.credential_id).await;

    Ok(Json(ApiResponse::ok(updated)))
}

// --- POST /onboarding ---

#[derive(Debug, Deserialize)]
pub struct OnboardingRequest {
    pub display_name: String,
    pub birth_date: String,
    pub bio: Option<String>,
    pub kinks: Vec<String>,
    pub country: String,
}

pub async fn complete_onboarding(
    user: AuthUser,
    State(state): State<Arc<AppState>>,
    Json(req): Json<OnboardingRequest>,
) -> AppResult<Json<ApiResponse<Profile>>> {
    // Validate display_name: 3-20 chars, alphanumeric + underscore
    if req.display_name.len() < 3 || req.display_name.len() > 20 {
        return Err(AppError::new(
            ErrorCode::InvalidDisplayName,
            "display name must be between 3 and 20 characters",
        ));
    }
    if !req.display_name.chars().all(|c| c.is_alphanumeric() || c == '_') {
        return Err(AppError::new(
            ErrorCode::InvalidDisplayName,
            "display name can only contain letters, numbers, and underscores",
        ));
    }

    // Parse birth_date
    let birth_date = NaiveDate::parse_from_str(&req.birth_date, "%Y-%m-%d")
        .map_err(|_| AppError::new(ErrorCode::ValidationError, "invalid birth_date format, expected YYYY-MM-DD"))?;

    let mut conn = state.db.get().map_err(|e| AppError::internal(e.to_string()))?;

    // Check display_name uniqueness
    let name_taken: bool = profiles::table
        .filter(profiles::display_name.eq(&req.display_name))
        .count()
        .get_result::<i64>(&mut conn)
        .map(|c| c > 0)
        .unwrap_or(false);

    if name_taken {
        return Err(AppError::new(ErrorCode::DisplayNameTaken, "display name is already taken"));
    }

    let profile = profiles::table
        .filter(profiles::credential_id.eq(user.id))
        .first::<Profile>(&mut conn)
        .map_err(|_| AppError::new(ErrorCode::ProfileNotFound, "profile not found"))?;

    let kinks_json = serde_json::to_value(&req.kinks)
        .map_err(|e| AppError::internal(e.to_string()))?;

    let updated = diesel::update(profiles::table.filter(profiles::id.eq(profile.id)))
        .set((
            profiles::display_name.eq(&req.display_name),
            profiles::birth_date.eq(birth_date),
            profiles::bio.eq(&req.bio),
            profiles::kinks.eq(&kinks_json),
            profiles::country.eq(&req.country),
            profiles::onboarding_complete.eq(true),
            profiles::updated_at.eq(chrono::Utc::now()),
        ))
        .get_result::<Profile>(&mut conn)?;

    publisher::publish_onboarding_completed(&state.rabbitmq, user.id, &req.display_name).await;

    tracing::info!(
        credential_id = %user.id,
        display_name = %req.display_name,
        "onboarding completed"
    );

    Ok(Json(ApiResponse::ok(updated)))
}

// --- GET /profile/:id --- (public profile by credential_id or profile_id)

pub async fn get_public_profile(
    _user: AuthUser,
    State(state): State<Arc<AppState>>,
    axum::extract::Path(id): axum::extract::Path<uuid::Uuid>,
) -> AppResult<Json<ApiResponse<Profile>>> {
    let mut conn = state.db.get().map_err(|e| AppError::internal(e.to_string()))?;

    // Try credential_id first, then profile id
    let profile = profiles::table
        .filter(profiles::credential_id.eq(id))
        .first::<Profile>(&mut conn)
        .or_else(|_| {
            profiles::table
                .filter(profiles::id.eq(id))
                .first::<Profile>(&mut conn)
        })
        .map_err(|_| AppError::new(ErrorCode::ProfileNotFound, "profile not found"))?;

    Ok(Json(ApiResponse::ok(profile)))
}

// --- GET /check-pseudo ---

#[derive(Debug, Deserialize)]
pub struct CheckNameQuery {
    pub name: String,
}

#[derive(Debug, Serialize)]
pub struct CheckNameResponse {
    pub available: bool,
}

pub async fn check_display_name(
    State(state): State<Arc<AppState>>,
    Query(query): Query<CheckNameQuery>,
) -> AppResult<Json<ApiResponse<CheckNameResponse>>> {
    let mut conn = state.db.get().map_err(|e| AppError::internal(e.to_string()))?;

    let taken: bool = profiles::table
        .filter(profiles::display_name.eq(&query.name))
        .count()
        .get_result::<i64>(&mut conn)
        .map(|c| c > 0)
        .unwrap_or(false);

    Ok(Json(ApiResponse::ok(CheckNameResponse { available: !taken })))
}
