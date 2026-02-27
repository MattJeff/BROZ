use axum::extract::{Path, State};
use axum::Json;
use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

use broz_shared::errors::{AppError, AppResult, ErrorCode};
use broz_shared::types::auth::AuthUser;
use broz_shared::types::ApiResponse;

use crate::events::publisher;
use crate::models::{Like, NewLike, Profile};
use crate::schema::{likes, profiles};
use crate::AppState;

#[derive(Debug, Deserialize)]
pub struct SendLikeRequest {
    pub liked_id: Uuid,
    pub match_session_id: Option<Uuid>,
}

pub async fn send_like(
    user: AuthUser,
    State(state): State<Arc<AppState>>,
    Json(req): Json<SendLikeRequest>,
) -> AppResult<Json<ApiResponse<Like>>> {
    let mut conn = state.db.get().map_err(|e| AppError::internal(e.to_string()))?;

    // Get liker profile
    let liker = profiles::table
        .filter(profiles::credential_id.eq(user.id))
        .first::<Profile>(&mut conn)
        .map_err(|_| AppError::new(ErrorCode::ProfileNotFound, "profile not found"))?;

    // Check liked profile exists (liked_id can be either profile id or credential_id)
    let liked = profiles::table
        .filter(
            profiles::id.eq(req.liked_id)
                .or(profiles::credential_id.eq(req.liked_id)),
        )
        .first::<Profile>(&mut conn)
        .map_err(|_| AppError::new(ErrorCode::ProfileNotFound, "liked profile not found"))?;

    // Check if already liked (one like per user pair, lifetime)
    let already_liked = likes::table
        .filter(likes::liker_id.eq(liker.id))
        .filter(likes::liked_id.eq(liked.id))
        .first::<Like>(&mut conn)
        .optional()?;

    if let Some(existing) = already_liked {
        return Ok(Json(ApiResponse::ok(existing)));
    }

    // Create like using the profile id (not credential_id)
    let new_like = NewLike {
        liker_id: liker.id,
        liked_id: liked.id,
        match_session_id: req.match_session_id,
    };

    let like = diesel::insert_into(likes::table)
        .values(&new_like)
        .get_result::<Like>(&mut conn)?;

    // Increment total_likes on liked profile
    diesel::update(profiles::table.filter(profiles::id.eq(liked.id)))
        .set(profiles::total_likes.eq(profiles::total_likes + 1))
        .execute(&mut conn)?;

    let display_name = liker.display_name.as_deref().unwrap_or("unknown");
    publisher::publish_like_sent(
        &state.rabbitmq,
        liker.id,
        req.liked_id,
        display_name,
        req.match_session_id,
    )
    .await;

    Ok(Json(ApiResponse::ok(like)))
}

#[derive(Debug, Serialize)]
pub struct LikeCheckResponse {
    pub already_liked: bool,
}

/// GET /likes/check/:target_id - check if current user already liked target
pub async fn check_like(
    user: AuthUser,
    State(state): State<Arc<AppState>>,
    Path(target_id): Path<Uuid>,
) -> AppResult<Json<ApiResponse<LikeCheckResponse>>> {
    let mut conn = state.db.get().map_err(|e| AppError::internal(e.to_string()))?;

    let liker = profiles::table
        .filter(profiles::credential_id.eq(user.id))
        .first::<Profile>(&mut conn)
        .map_err(|_| AppError::new(ErrorCode::ProfileNotFound, "profile not found"))?;

    // target_id can be either profile id or credential_id
    let liked = profiles::table
        .filter(
            profiles::id.eq(target_id)
                .or(profiles::credential_id.eq(target_id)),
        )
        .first::<Profile>(&mut conn)
        .optional()?;

    let already_liked = if let Some(liked_profile) = liked {
        likes::table
            .filter(likes::liker_id.eq(liker.id))
            .filter(likes::liked_id.eq(liked_profile.id))
            .first::<Like>(&mut conn)
            .optional()?
            .is_some()
    } else {
        false
    };

    Ok(Json(ApiResponse::ok(LikeCheckResponse { already_liked })))
}
